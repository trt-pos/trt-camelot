use crate::Head;
use getset::Getters;

#[derive(Getters)]
pub struct Response<'r> {
    #[get = "pub"]
    head: Head<'r>,
    #[get = "pub"]
    status: Status,
    #[get = "pub"]
    body: &'r str,
}

impl Response<'_> {
    pub fn new<'a>(head: Head<'a>, status: Status, body: &'a str) -> Response<'a> {
        Response { head, status, body }
    }
    
}

impl<'r> TryFrom<&'r [u8]> for Response<'r> {
    type Error = crate::Error;

    fn try_from(response: &'r [u8]) -> Result<Self, Self::Error> {
        let split_response = response.split(|&x| x == 0x1F).collect::<Vec<&[u8]>>();

        if split_response.len() != 3 {
            return Err(crate::Error::InvalidResponse);
        }

        let head = split_response[0].try_into()?;
        let status = split_response[1].try_into()?;
        let body = split_response[2];

        let body = std::str::from_utf8(body).map_err(|_| crate::Error::InvalidBody)?;

        Ok(Response { head, status, body })
    }
}

impl<'r> TryFrom<Response<'r>> for Vec<u8> {
    type Error = crate::Error;

    fn try_from(response: Response<'r>) -> Result<Self, Self::Error> {
        let mut result = vec![];

        let head_bytes: Vec<u8> = response.head.try_into()?;
        result.extend(head_bytes);

        result.push(0x1F);

        let status_bytes: Vec<u8> = response.status.try_into()?;
        result.extend(status_bytes);

        result.push(0x1F);

        result.extend_from_slice(response.body.as_bytes());

        Ok(result)
    }
}

#[derive(Getters)]
pub struct Status {
    #[get = "pub"]
    r#type: StatusType,
}

impl Status {
    pub fn new(r#type: StatusType) -> Self {
        Status { r#type }
    }
}

impl TryFrom<&[u8]> for Status {
    type Error = crate::Error;

    fn try_from(status: &[u8]) -> Result<Self, Self::Error> {
        let (int_bytes, _) = status.split_at(size_of::<i8>());
        let r#type: StatusType = i8::from_be_bytes(
            int_bytes
                .try_into()
                .map_err(|_| crate::Error::InvalidStatus)?,
        )
        .try_into()?;

        Ok(Status { r#type })
    }
}

impl TryFrom<Status> for Vec<u8> {
    type Error = crate::Error;

    fn try_from(status: Status) -> Result<Self, Self::Error> {
        let mut result = vec![];

        let r#type: i8 = status.r#type.try_into()?;
        result.push(r#type as u8);

        Ok(result)
    }
}

/// 0 -> OK
/// < 0 -> Unrecoverable error
/// > 0 -> Recoverable error
#[derive(PartialEq, Debug, Clone)]
pub enum StatusType {
    OK,    // 0
    GenericError, // -1
    NeedConnection, // -2
    AlreadyConnected, // 1
    InvalidRequest, // 2
}

impl TryFrom<i8> for StatusType {
    type Error = crate::Error;

    fn try_from(code: i8) -> Result<Self, crate::Error> {
        match code {
            0 => Ok(StatusType::OK),
            -1 => Ok(StatusType::GenericError),
            -2 => Ok(StatusType::NeedConnection),
            1 => Ok(StatusType::AlreadyConnected),
            2 => Ok(StatusType::InvalidRequest),
            _ => Err(crate::Error::InvalidStatus),
        }
    }
}

impl TryFrom<StatusType> for i8 {
    type Error = crate::Error;

    fn try_from(status: StatusType) -> Result<Self, crate::Error> {
        match status {
            StatusType::OK => Ok(0i8),
            StatusType::GenericError => Ok(-1),
            StatusType::NeedConnection => Ok(-2),
            StatusType::AlreadyConnected => Ok(1),
            StatusType::InvalidRequest => Ok(2),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Version;

    #[test]
    fn test_response_into_response() {
        let response = Response {
            head: Head {
                version: Version { major: 1, patch: 2 },
                caller: "345",
            },
            status: Status {
                r#type: StatusType::GenericError,
            },
            body: "345",
        };

        let bytes: Vec<u8> = response.try_into().unwrap();

        let response: Response = bytes.as_slice().try_into().unwrap();

        assert_eq!(response.head.version.major, 1);
        assert_eq!(response.head.version.patch, 2);
        assert_eq!(response.head.caller, "345");
        assert_eq!(response.status.r#type, StatusType::GenericError);
        assert_eq!(response.body, "345");
    }

    #[test]
    fn test_bytes_into_response() {
        let response: &[u8] = &[
            0, 1, // major (1)
            0, 2, // patch (2)
            51, 52, 53,   // caller ("345")
            0x1F, // separator
            0,    // code (0)
            0x1F, // separator
            51, 52, 53, // body ("345")
        ];

        let response: Response = response.try_into().unwrap();

        assert_eq!(response.head.version.major, 1);
        assert_eq!(response.head.version.patch, 2);
        assert_eq!(response.head.caller, "345");
        assert_eq!(response.status.r#type, StatusType::OK);
        assert_eq!(response.body, "345");
    }

    #[test]
    fn test_response_into_bytes() {
        let response = Response {
            head: Head {
                version: Version { major: 1, patch: 2 },
                caller: "345",
            },
            status: Status {
                r#type: StatusType::OK,
            },
            body: "345",
        };

        let bytes: Vec<u8> = response.try_into().unwrap();

        assert_eq!(
            bytes,
            vec![
                0, 1, // major (1)
                0, 2, // patch (2)
                51, 52, 53,   // caller ("345")
                0x1F, // separator
                0,    // code (0)
                0x1F, // separator
                51, 52, 53, // body ("345")
            ]
        );
    }

    #[test]
    fn test_bytes_into_status() {
        let status: &[u8] = &[0, 0];
        let status: Status = status.try_into().unwrap();

        assert_eq!(status.r#type, StatusType::OK);
    }

    #[test]
    fn test_status_into_bytes() {
        let status = Status {
            r#type: StatusType::OK,
        };
        let bytes: Vec<u8> = status.try_into().unwrap();

        assert_eq!(bytes, vec![0]);
    }
}
