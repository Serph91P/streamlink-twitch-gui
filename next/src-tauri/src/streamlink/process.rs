use std::{
    ffi::OsString,
    fmt,
    io::{BufRead, BufReader, Read},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

use super::{arguments::BuiltArguments, discovery::StreamlinkExecutable};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ProcessStatus {
    Running { pid: u32 },
    Exited { code: Option<i32>, success: bool },
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticSource {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticLine {
    pub source: DiagnosticSource,
    pub message: String,
}

#[derive(Debug)]
pub struct ProcessError(std::io::Error);

impl fmt::Display for ProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Streamlink process error: {}", self.0)
    }
}

impl std::error::Error for ProcessError {}

impl From<std::io::Error> for ProcessError {
    fn from(error: std::io::Error) -> Self {
        Self(error)
    }
}

pub(crate) struct CapturedProcess {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

pub(crate) enum CaptureError {
    Process(ProcessError),
    TimedOut,
}

impl From<std::io::Error> for CaptureError {
    fn from(error: std::io::Error) -> Self {
        Self::Process(ProcessError(error))
    }
}

pub(crate) fn capture_command(
    executable: &StreamlinkExecutable,
    arguments: &[OsString],
    timeout: Duration,
) -> Result<CapturedProcess, CaptureError> {
    let mut command = Command::new(&executable.program);
    command
        .args(&executable.prefix_arguments)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_process_tree(&mut command);
    let mut child = command.spawn()?;
    let stdout = child.stdout.take().expect("piped stdout must be available");
    let stderr = child.stderr.take().expect("piped stderr must be available");
    let stdout_reader = thread::spawn(move || read_all(stdout));
    let stderr_reader = thread::spawn(move || read_all(stderr));
    let started = Instant::now();

    let exit = loop {
        if let Some(exit) = child.try_wait()? {
            break exit;
        }
        if started.elapsed() >= timeout {
            terminate_process_tree(&mut child).map_err(CaptureError::Process)?;
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(CaptureError::TimedOut);
        }
        thread::sleep(Duration::from_millis(10));
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| std::io::Error::other("stdout reader panicked"))??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| std::io::Error::other("stderr reader panicked"))??;
    Ok(CapturedProcess {
        stdout,
        stderr,
        success: exit.success(),
    })
}

fn read_all(mut stream: impl Read) -> std::io::Result<String> {
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub struct PlaybackProcess {
    child: Child,
    status: ProcessStatus,
    diagnostics: Receiver<DiagnosticLine>,
    readers: Vec<JoinHandle<()>>,
}

pub fn launch_playback(
    executable: &StreamlinkExecutable,
    arguments: BuiltArguments,
) -> Result<PlaybackProcess, ProcessError> {
    let mut command = Command::new(&executable.program);
    command
        .args(&executable.prefix_arguments)
        .args(&arguments.execution)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_process_tree(&mut command);

    let mut child = command.spawn()?;
    let pid = child.id();
    let stdout = child.stdout.take().expect("piped stdout must be available");
    let stderr = child.stderr.take().expect("piped stderr must be available");
    let redactions = sensitive_values(&arguments);
    let (sender, diagnostics) = mpsc::channel();
    let readers = vec![
        read_diagnostics(
            stdout,
            DiagnosticSource::Stdout,
            redactions.clone(),
            sender.clone(),
        ),
        read_diagnostics(stderr, DiagnosticSource::Stderr, redactions, sender),
    ];

    Ok(PlaybackProcess {
        child,
        status: ProcessStatus::Running { pid },
        diagnostics,
        readers,
    })
}

impl PlaybackProcess {
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    pub fn status(&mut self) -> Result<ProcessStatus, ProcessError> {
        if !matches!(self.status, ProcessStatus::Running { .. }) {
            return Ok(self.status.clone());
        }
        if let Some(exit) = self.child.try_wait()? {
            self.status = ProcessStatus::Exited {
                code: exit.code(),
                success: exit.success(),
            };
            self.finish_readers();
        }
        Ok(self.status.clone())
    }

    pub fn wait_timeout(&mut self, timeout: Duration) -> Result<ProcessStatus, ProcessError> {
        let started = Instant::now();
        loop {
            let status = self.status()?;
            if !matches!(status, ProcessStatus::Running { .. }) || started.elapsed() >= timeout {
                return Ok(status);
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn cancel(&mut self) -> Result<ProcessStatus, ProcessError> {
        if !matches!(self.status, ProcessStatus::Running { .. }) {
            return Ok(self.status.clone());
        }
        terminate_process_tree(&mut self.child)?;
        self.status = ProcessStatus::Cancelled;
        self.finish_readers();
        Ok(self.status.clone())
    }

    pub fn diagnostics(&self) -> Vec<DiagnosticLine> {
        self.diagnostics.try_iter().collect()
    }

    fn finish_readers(&mut self) {
        for reader in self.readers.drain(..) {
            let _ = reader.join();
        }
    }
}

impl Drop for PlaybackProcess {
    fn drop(&mut self) {
        if matches!(self.status, ProcessStatus::Running { .. }) {
            let _ = terminate_process_tree(&mut self.child);
            self.status = ProcessStatus::Cancelled;
            self.finish_readers();
        }
    }
}

fn read_diagnostics(
    stream: impl Read + Send + 'static,
    source: DiagnosticSource,
    redactions: Vec<String>,
    sender: mpsc::Sender<DiagnosticLine>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
            let message = redactions
                .iter()
                .fold(line, |message, value| message.replace(value, "<redacted>"));
            if sender.send(DiagnosticLine { source, message }).is_err() {
                break;
            }
        }
    })
}

fn sensitive_values(arguments: &BuiltArguments) -> Vec<String> {
    let mut values = Vec::new();
    for pair in arguments.execution.windows(2) {
        if pair[0] == "--twitch-api-header" {
            let value = pair[1].to_string_lossy().into_owned();
            if let Some(token) = value.split_whitespace().last() {
                values.push(token.to_owned());
            }
            values.push(value);
        }
    }
    values.sort_by_key(|value| std::cmp::Reverse(value.len()));
    values.dedup();
    values
}

#[cfg(unix)]
fn configure_process_tree(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    command.process_group(0);
}

#[cfg(windows)]
fn configure_process_tree(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    command.creation_flags(CREATE_NEW_PROCESS_GROUP);
}

#[cfg(not(any(unix, windows)))]
fn configure_process_tree(_command: &mut Command) {}

#[cfg(unix)]
fn terminate_process_tree(child: &mut Child) -> Result<(), ProcessError> {
    let process_group = -(child.id().cast_signed());
    // The child starts a new process group, so signaling it also reaches its player descendants.
    unsafe {
        libc::kill(process_group, libc::SIGTERM);
    }
    let deadline = Instant::now() + Duration::from_millis(500);
    while Instant::now() < deadline {
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(10));
    }
    unsafe {
        libc::kill(process_group, libc::SIGKILL);
    }
    child.wait()?;
    Ok(())
}

#[cfg(windows)]
fn terminate_process_tree(child: &mut Child) -> Result<(), ProcessError> {
    let status = Command::new("taskkill")
        .args(["/PID", &child.id().to_string(), "/T", "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if !status.success() && child.try_wait()?.is_none() {
        child.kill()?;
    }
    child.wait()?;
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn terminate_process_tree(child: &mut Child) -> Result<(), ProcessError> {
    child.kill()?;
    child.wait()?;
    Ok(())
}
