use std::fmt;

use serde::{Deserialize, Serialize};

pub const MINIMUM_SUPPORTED_VERSION: StreamlinkVersion = StreamlinkVersion::new(8, 0, 0);
pub const CURRENT_VERIFIED_MAJOR: u64 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamlinkVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl StreamlinkVersion {
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl fmt::Display for StreamlinkVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Compatibility {
    TooOld,
    Supported,
    NewerUnverified,
}

pub fn parse_version_output(output: &str) -> Result<StreamlinkVersion, String> {
    let version = output
        .split_whitespace()
        .find(|part| {
            part.chars()
                .next()
                .is_some_and(|character| character.is_ascii_digit())
        })
        .ok_or_else(|| "Streamlink did not return a semantic version".to_owned())?;
    let core = version.split(['-', '+']).next().unwrap_or(version);
    let mut parts = core.split('.');
    let major = parse_part(parts.next(), output)?;
    let minor = parse_part(parts.next(), output)?;
    let patch = parse_part(parts.next(), output)?;
    if parts.next().is_some() {
        return Err(format!("malformed Streamlink version output: {output:?}"));
    }

    Ok(StreamlinkVersion::new(major, minor, patch))
}

fn parse_part(part: Option<&str>, output: &str) -> Result<u64, String> {
    part.and_then(|value| value.parse().ok())
        .ok_or_else(|| format!("malformed Streamlink version output: {output:?}"))
}

pub fn classify_version(version: StreamlinkVersion) -> Compatibility {
    if version < MINIMUM_SUPPORTED_VERSION {
        Compatibility::TooOld
    } else if version.major > CURRENT_VERIFIED_MAJOR {
        Compatibility::NewerUnverified
    } else {
        Compatibility::Supported
    }
}

#[cfg(test)]
mod tests {
    use super::{Compatibility, StreamlinkVersion, classify_version, parse_version_output};

    #[test]
    fn parses_streamlink_semantic_versions() {
        assert_eq!(
            parse_version_output(include_str!(
                "../../tests/fixtures/streamlink-version-8.4.txt"
            ))
            .unwrap(),
            StreamlinkVersion::new(8, 4, 0)
        );
        assert!(
            parse_version_output(include_str!(
                "../../tests/fixtures/streamlink-version-malformed.txt"
            ))
            .is_err()
        );
    }

    #[test]
    fn classifies_supported_version_range() {
        assert_eq!(
            classify_version(StreamlinkVersion::new(7, 6, 0)),
            Compatibility::TooOld
        );
        assert_eq!(
            classify_version(
                parse_version_output(include_str!(
                    "../../tests/fixtures/streamlink-version-8.0.txt"
                ))
                .unwrap()
            ),
            Compatibility::Supported
        );
        assert_eq!(
            classify_version(StreamlinkVersion::new(8, 4, 0)),
            Compatibility::Supported
        );
        assert_eq!(
            classify_version(StreamlinkVersion::new(9, 0, 0)),
            Compatibility::NewerUnverified
        );
    }
}
