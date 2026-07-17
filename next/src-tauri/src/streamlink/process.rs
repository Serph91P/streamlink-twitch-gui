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

#[derive(Debug)]
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

    pub fn accept(mut self, timeout: Duration) -> Result<Self, PlaybackLaunchError> {
        let status = self
            .wait_timeout(timeout)
            .map_err(PlaybackLaunchError::Process)?;
        if matches!(status, ProcessStatus::Running { .. }) {
            return Ok(self);
        }

        let summary = match status {
            ProcessStatus::Exited {
                code: Some(code), ..
            } => format!(
                "Streamlink exited before playback started (exit code {code}). Check the configured player and Streamlink settings."
            ),
            ProcessStatus::Exited { code: None, .. } | ProcessStatus::Cancelled => {
                "Streamlink exited before playback started. Check the configured player and Streamlink settings."
                    .to_owned()
            }
            ProcessStatus::Running { .. } => unreachable!(),
        };
        let diagnostics = self
            .diagnostics()
            .into_iter()
            .take(8)
            .map(|line| line.message)
            .collect();
        Err(PlaybackLaunchError::Exited {
            summary,
            diagnostics,
        })
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

#[derive(Debug)]
pub enum PlaybackLaunchError {
    Process(ProcessError),
    Exited {
        summary: String,
        diagnostics: Vec<String>,
    },
}

impl fmt::Display for PlaybackLaunchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Process(error) => error.fmt(formatter),
            Self::Exited {
                summary,
                diagnostics,
            } => {
                write!(formatter, "{summary}")?;
                for diagnostic in diagnostics {
                    write!(formatter, "\n{diagnostic}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for PlaybackLaunchError {}

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
        for line in BufReader::new(stream)
            .lines()
            .map_while(Result::ok)
            .take(40)
        {
            let message = sanitize_diagnostic(line, &redactions);
            if sender.send(DiagnosticLine { source, message }).is_err() {
                break;
            }
        }
    })
}

fn sanitize_diagnostic(line: String, redactions: &[String]) -> String {
    let message = redactions
        .iter()
        .fold(line, |message, value| message.replace(value, "<redacted>"));
    let lower = message.to_ascii_lowercase();
    if [
        "authorization",
        "bearer",
        "oauth",
        "cookie",
        "access_token",
        "refresh_token",
        "device_code",
        "client_secret",
        "client-integrity",
        "token=",
        "token:",
        "signature=",
        "sig=",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        return "<redacted sensitive diagnostic>".to_owned();
    }

    let mut message = redact_urls(message);
    if message.chars().count() > 512 {
        message = message.chars().take(509).collect::<String>();
        message.push_str("...");
    }
    message
}

fn redact_urls(mut message: String) -> String {
    loop {
        let start = [message.find("https://"), message.find("http://")]
            .into_iter()
            .flatten()
            .min();
        let Some(start) = start else {
            return message;
        };
        let end = message[start..]
            .find(|character: char| character.is_whitespace())
            .map_or(message.len(), |length| start + length);
        message.replace_range(start..end, "<redacted-url>");
    }
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

#[cfg(any(test, windows))]
const fn windows_creation_flags(new_process_group: bool) -> u32 {
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    CREATE_NO_WINDOW
        | if new_process_group {
            CREATE_NEW_PROCESS_GROUP
        } else {
            0
        }
}

#[cfg(windows)]
pub(crate) fn configure_background_process(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    command.creation_flags(windows_creation_flags(false));
}

#[cfg(not(windows))]
pub(crate) fn configure_background_process(_command: &mut Command) {}

#[cfg(windows)]
fn configure_process_tree(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    command.creation_flags(windows_creation_flags(true));
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

#[cfg(test)]
mod tests {
    #[test]
    fn windows_background_process_flags_preserve_process_tree_semantics() {
        assert_eq!(super::windows_creation_flags(true), 0x0800_0200);
        assert_eq!(super::windows_creation_flags(false), 0x0800_0000);
    }
}

#[cfg(windows)]
fn terminate_process_tree(child: &mut Child) -> Result<(), ProcessError> {
    let mut command = Command::new("taskkill");
    command
        .args(["/PID", &child.id().to_string(), "/T", "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    configure_background_process(&mut command);
    let status = command.status()?;
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
