use std::{
    ffi::{OsStr, OsString},
    fmt,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

use super::version::{Compatibility, StreamlinkVersion, classify_version, parse_version_output};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamlinkExecutable {
    pub program: OsString,
    pub prefix_arguments: Vec<OsString>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiscoverySource {
    UserSelected,
    Path,
    PythonModule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamlinkDetection {
    pub executable: StreamlinkExecutable,
    pub source: DiscoverySource,
    pub version: StreamlinkVersion,
    pub compatibility: Compatibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamlinkStatus {
    pub source: DiscoverySource,
    pub version: StreamlinkVersion,
    pub compatibility: Compatibility,
}

impl From<StreamlinkDetection> for StreamlinkStatus {
    fn from(detection: StreamlinkDetection) -> Self {
        Self {
            source: detection.source,
            version: detection.version,
            compatibility: detection.compatibility,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectionError {
    Missing,
    TimedOut,
    MalformedVersion,
    ExecutionFailed(String),
}

impl fmt::Display for DetectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => formatter.write_str("Streamlink was not found"),
            Self::TimedOut => formatter.write_str("Streamlink version check timed out"),
            Self::MalformedVersion => {
                formatter.write_str("Streamlink returned malformed version output")
            }
            Self::ExecutionFailed(message) => {
                write!(formatter, "Streamlink version check failed: {message}")
            }
        }
    }
}

impl std::error::Error for DetectionError {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProbeOutput {
    stdout: String,
    stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProbeFailure {
    NotFound,
    TimedOut,
    Failed(String),
}

trait Probe {
    fn version(
        &mut self,
        program: &OsStr,
        prefix_arguments: &[OsString],
        timeout: Duration,
    ) -> Result<ProbeOutput, ProbeFailure>;
}

pub fn detect_streamlink(
    selected: Option<PathBuf>,
    timeout: Duration,
) -> Result<StreamlinkDetection, DetectionError> {
    detect_with_probe(selected, timeout, &mut SystemProbe)
}

fn detect_with_probe(
    selected: Option<PathBuf>,
    timeout: Duration,
    probe: &mut impl Probe,
) -> Result<StreamlinkDetection, DetectionError> {
    let mut candidates = Vec::new();
    if let Some(path) = selected {
        candidates.push((
            StreamlinkExecutable {
                program: path.into_os_string(),
                prefix_arguments: Vec::new(),
            },
            DiscoverySource::UserSelected,
        ));
    }
    candidates.push((
        StreamlinkExecutable {
            program: "streamlink".into(),
            prefix_arguments: Vec::new(),
        },
        DiscoverySource::Path,
    ));
    for python in ["python3", "python"] {
        candidates.push((
            StreamlinkExecutable {
                program: python.into(),
                prefix_arguments: vec!["-m".into(), "streamlink".into()],
            },
            DiscoverySource::PythonModule,
        ));
    }

    for (executable, source) in candidates {
        let output = match probe.version(&executable.program, &executable.prefix_arguments, timeout)
        {
            Ok(output) => output,
            Err(ProbeFailure::NotFound) => continue,
            Err(ProbeFailure::TimedOut) => return Err(DetectionError::TimedOut),
            Err(ProbeFailure::Failed(message)) => {
                if source == DiscoverySource::PythonModule {
                    continue;
                }
                return Err(DetectionError::ExecutionFailed(message));
            }
        };
        let combined = if output.stdout.trim().is_empty() {
            output.stderr.trim()
        } else {
            output.stdout.trim()
        };
        let version =
            parse_version_output(combined).map_err(|_| DetectionError::MalformedVersion)?;
        return Ok(StreamlinkDetection {
            executable,
            source,
            version,
            compatibility: classify_version(version),
        });
    }

    Err(DetectionError::Missing)
}

struct SystemProbe;

impl Probe for SystemProbe {
    fn version(
        &mut self,
        program: &OsStr,
        prefix_arguments: &[OsString],
        timeout: Duration,
    ) -> Result<ProbeOutput, ProbeFailure> {
        let mut command = Command::new(program);
        command
            .args(prefix_arguments)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        super::process::configure_background_process(&mut command);
        let mut child = command.spawn().map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                ProbeFailure::NotFound
            } else {
                ProbeFailure::Failed(error.to_string())
            }
        })?;
        let started = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => {
                    let output = child
                        .wait_with_output()
                        .map_err(|error| ProbeFailure::Failed(error.to_string()))?;
                    if !output.status.success() {
                        return Err(ProbeFailure::Failed(
                            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
                        ));
                    }
                    return Ok(ProbeOutput {
                        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                    });
                }
                Ok(None) if started.elapsed() < timeout => {
                    thread::sleep(Duration::from_millis(10));
                }
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(ProbeFailure::TimedOut);
                }
                Err(error) => return Err(ProbeFailure::Failed(error.to_string())),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, ffi::OsString, path::PathBuf, time::Duration};

    use super::{
        DetectionError, DiscoverySource, Probe, ProbeFailure, ProbeOutput, StreamlinkDetection,
        StreamlinkExecutable, StreamlinkStatus, detect_with_probe,
    };
    use crate::streamlink::version::{Compatibility, StreamlinkVersion};

    #[derive(Default)]
    struct FakeProbe {
        responses: HashMap<OsString, Result<ProbeOutput, ProbeFailure>>,
        calls: Vec<OsString>,
    }

    impl Probe for FakeProbe {
        fn version(
            &mut self,
            program: &std::ffi::OsStr,
            _prefix_arguments: &[OsString],
            _timeout: Duration,
        ) -> Result<ProbeOutput, ProbeFailure> {
            self.calls.push(program.to_owned());
            self.responses
                .get(program)
                .cloned()
                .unwrap_or(Err(ProbeFailure::NotFound))
        }
    }

    fn output(version: &str) -> Result<ProbeOutput, ProbeFailure> {
        Ok(ProbeOutput {
            stdout: format!("streamlink {version}"),
            stderr: String::new(),
        })
    }

    #[test]
    fn selected_executable_precedes_path_and_python_fallback() {
        let selected = PathBuf::from("/chosen/streamlink");
        let mut probe = FakeProbe::default();
        probe
            .responses
            .insert(selected.clone().into(), output("8.4.0"));
        probe.responses.insert("streamlink".into(), output("8.0.0"));

        let detection =
            detect_with_probe(Some(selected.clone()), Duration::from_secs(1), &mut probe).unwrap();

        assert_eq!(detection.source, DiscoverySource::UserSelected);
        assert_eq!(detection.executable.program, selected.into_os_string());
        assert_eq!(detection.version, StreamlinkVersion::new(8, 4, 0));
        assert_eq!(detection.compatibility, Compatibility::Supported);
        assert_eq!(probe.calls, vec![OsString::from("/chosen/streamlink")]);
    }

    #[test]
    fn path_precedes_python_module_fallback() {
        let mut probe = FakeProbe::default();
        probe.responses.insert("streamlink".into(), output("8.4.0"));
        probe.responses.insert("python3".into(), output("8.4.0"));

        let detection = detect_with_probe(None, Duration::from_secs(1), &mut probe).unwrap();

        assert_eq!(detection.source, DiscoverySource::Path);
        assert!(detection.executable.prefix_arguments.is_empty());
        assert_eq!(probe.calls, vec![OsString::from("streamlink")]);
    }

    #[test]
    fn falls_back_to_python_module_and_reports_missing() {
        let mut probe = FakeProbe::default();
        probe.responses.insert("python3".into(), output("8.0.0"));

        let detection = detect_with_probe(None, Duration::from_secs(1), &mut probe).unwrap();
        assert_eq!(detection.source, DiscoverySource::PythonModule);
        assert_eq!(
            detection.executable.prefix_arguments,
            vec![OsString::from("-m"), OsString::from("streamlink")]
        );

        let mut missing = FakeProbe::default();
        assert_eq!(
            detect_with_probe(None, Duration::from_secs(1), &mut missing).unwrap_err(),
            DetectionError::Missing
        );
    }

    #[test]
    fn timeout_and_malformed_output_are_actionable() {
        let mut timeout = FakeProbe::default();
        timeout
            .responses
            .insert("streamlink".into(), Err(ProbeFailure::TimedOut));
        assert_eq!(
            detect_with_probe(None, Duration::from_millis(5), &mut timeout).unwrap_err(),
            DetectionError::TimedOut
        );

        let mut malformed = FakeProbe::default();
        malformed.responses.insert(
            "streamlink".into(),
            Ok(ProbeOutput {
                stdout: "not a semantic version".into(),
                stderr: String::new(),
            }),
        );
        assert_eq!(
            detect_with_probe(None, Duration::from_secs(1), &mut malformed).unwrap_err(),
            DetectionError::MalformedVersion
        );
    }

    #[test]
    fn status_exposes_detection_without_the_executable_path() {
        let detection = StreamlinkDetection {
            executable: StreamlinkExecutable {
                program: OsString::from("/private/configured/streamlink"),
                prefix_arguments: Vec::new(),
            },
            source: DiscoverySource::UserSelected,
            version: StreamlinkVersion::new(8, 4, 0),
            compatibility: Compatibility::Supported,
        };

        let serialized = serde_json::to_value(StreamlinkStatus::from(detection)).unwrap();

        assert_eq!(serialized["source"], "userSelected");
        assert_eq!(serialized["version"]["major"], 8);
        assert_eq!(serialized["compatibility"], "supported");
        assert!(!serialized.to_string().contains("/private/configured"));
        assert!(serialized.get("executable").is_none());
    }
}
