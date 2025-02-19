#![allow(dead_code)]

mod request;
mod response;
mod error;

use std::str;
pub use request::Request;
pub use request::Action;
pub use error::Error;

pub struct Version {
    pub major: i32,
    pub patch: i32,
}

pub struct Head<'r> {
    pub version: Version,
    pub caller: &'r str,
}

impl<'a> TryFrom<&'a [u8]> for Head<'a> {
    type Error = Error;

    fn try_from(head: &'a [u8]) -> Result<Self, Self::Error> {
        let (int_bytes, rest) = head.split_at(size_of::<i32>());
        let major = i32::from_be_bytes(
            int_bytes
                .try_into()
                .map_err(|_| Error::InvalidHead)?,
        );

        let (int_bytes, rest) = rest.split_at(size_of::<i32>());
        let patch = i32::from_be_bytes(
            int_bytes
                .try_into()
                .map_err(|_| Error::InvalidHead)?,
        );

        let caller = str::from_utf8(rest).map_err(|_| Error::InvalidHead)?;

        Ok(Head {
            version: Version { major, patch },
            caller,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::Head;

    #[test]
    fn test_parse_head() {
        let head: &[u8] = &[
            0, 0, 0, 1, // major (1)
            0, 0, 0, 2, // patch (2)
            51, 52, 53, // caller ("345")
        ];
        
        let head: Head = head.try_into().unwrap();

        assert_eq!(head.version.major, 1);
        assert_eq!(head.version.patch, 2);
        assert_eq!(head.caller, "345");
    }
}