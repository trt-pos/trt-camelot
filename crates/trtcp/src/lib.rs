#![allow(dead_code)]

mod error;
mod request;
mod response;
mod call;

pub use error::Error;
pub use request::Request;
pub use request::Action;
pub use request::ActionType;

pub use response::Response;
pub use response::Status;
pub use response::StatusType;

pub use call::Call;

use std::str;
use getset::Getters;

#[derive(Getters, Debug)]
pub struct Version {
    #[get = "pub"]
    major: u16,
    #[get = "pub"]
    patch: u16,
}

impl Version {
    pub fn new(major: u16, patch: u16) -> Self {
        Version { major, patch }
    }
    
    pub fn actual() -> Self {
        Version { major: 1, patch: 0 }
    }
}

impl TryFrom<&[u8]> for Version {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let (major_bytes, patch_bytes) = value.split_at(size_of::<u16>());

        let major = u16::from_be_bytes(major_bytes.try_into().map_err(|_| Error::InvalidHead)?);

        let patch = u16::from_be_bytes(patch_bytes.try_into().map_err(|_| Error::InvalidHead)?);

        Ok(Version { major, patch })
    }
}

impl From<Version> for Vec<u8> {
    fn from(value: Version) -> Self {
        let mut version = Vec::new();

        version.extend_from_slice(&value.major.to_be_bytes());
        version.extend_from_slice(&value.patch.to_be_bytes());

        version
    }
}

#[derive(Getters, Debug)]
pub struct Head<'r> {
    #[get = "pub"]
    version: Version,
    caller: &'r str,
}

impl Head<'_> {
    pub fn new(version: Version, caller: &str) -> Head {
        Head { version, caller }
    }
    
    pub fn caller(&self) -> &str {
        self.caller
    }
}

impl<'a> TryFrom<&'a [u8]> for Head<'a> {
    type Error = Error;

    fn try_from(head: &'a [u8]) -> Result<Self, Self::Error> {
        let (version_bytes, rest) = head.split_at(size_of::<Version>());
        let version = version_bytes.try_into()?;

        let caller = str::from_utf8(rest).map_err(|_| Error::InvalidHead)?;

        Ok(Head { version, caller })
    }
}

impl From<Head<'_>> for Vec<u8> {

    fn from(head: Head<'_>) -> Self {
        let mut result = Vec::new();

        let version_bytes: Vec<u8> = head.version.into();
        result.extend(version_bytes);

        let caller_bytes = head.caller.as_bytes();
        result.extend_from_slice(caller_bytes);

        result
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_head_into_bytes() {
        let head = Head {
            version: Version { major: 1, patch: 2 },
            caller: "345",
        };

        let bytes: Vec<u8> = head.try_into().unwrap();

        assert_eq!(
            bytes,
            vec![
                0, 1, // major (1)
                0, 2, // patch (2)
                51, 52, 53, // caller ("345")
            ]
        );
    }

    #[test]
    fn test_bytes_into_head() {
        let head: &[u8] = &[
            0, 1, // major (1)
            0, 2, // patch (2)
            51, 52, 53, // caller ("345")
        ];

        let head: Head = head.try_into().unwrap();

        assert_eq!(head.version.major, 1);
        assert_eq!(head.version.patch, 2);
        assert_eq!(head.caller, "345");
    }

    #[test]
    fn test_version_into_bytes() {
        let version = Version { major: 1, patch: 2 };

        let bytes: Vec<u8> = version.try_into().unwrap();

        assert_eq!(
            bytes,
            vec![
                0, 1, // major (1)
                0, 2, // patch (2)
            ]
        );
    }

    #[test]
    fn test_bytes_into_version() {
        let version: &[u8] = &[
            0, 1, // major (1)
            0, 2, // patch (2)
        ];

        let version: Version = version.try_into().unwrap();

        assert_eq!(version.major, 1);
        assert_eq!(version.patch, 2);
    }
}
