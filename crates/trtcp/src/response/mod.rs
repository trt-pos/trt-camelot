use crate::{Head, SEPARATOR_BYTE};
use getset::Getters;

const START_BYTE: u8 = 0x01;

#[derive(Getters, Debug)]
pub struct Response<'r> {
    #[get = "pub"]
    head: Head<'r>,
    #[get = "pub"]
    status: Status,
    #[get = "pub"]
    body: &'r [u8],
}

impl Response<'_> {
    pub fn new<'a, T: Into<&'a [u8]>>(head: Head<'a>, status: Status, body: T) -> Response<'a> {
        Response {
            head,
            status,
            body: body.into(),
        }
    }

    pub fn body_as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.body)
    }
}

impl<'r> TryFrom<&'r [u8]> for Response<'r> {
    type Error = crate::Error;

    fn try_from(response: &'r [u8]) -> Result<Self, Self::Error> {
        let (start_byte, response) = response.split_at(size_of::<u8>());

        if start_byte[0] != START_BYTE {
            return Err(crate::Error::InvalidRequest);
        }
        
        let (length_bytes, response) = response.split_at(size_of::<u32>());
        
        let length = u32::from_be_bytes(
            length_bytes.try_into().map_err(|_| crate::Error::InvalidHead)?,
        ) as usize;
        
        if length != response.len() { 
            return Err(crate::Error::InvalidResponse);
        }
        
        let split_response = response.split(|&x| x == SEPARATOR_BYTE).collect::<Vec<&[u8]>>();

        if split_response.len() != 3 {
            return Err(crate::Error::InvalidResponse);
        }

        let head = split_response[0].try_into()?;
        let status = split_response[1].try_into()?;
        let body = split_response[2];

        Ok(Response { head, status, body })
    }
}

impl<'r> From<Response<'r>> for Vec<u8> {
    fn from(response: Response<'r>) -> Self {
        let mut result = vec![];

        let head_bytes: Vec<u8> = response.head.into();
        result.extend(head_bytes);

        result.push(0x1F);

        let status_bytes: Vec<u8> = response.status.into();
        result.extend(status_bytes);

        result.push(0x1F);

        result.extend_from_slice(response.body);
        
        let length = (result.len() as u32).to_be_bytes();
        let msg_type = START_BYTE.to_be_bytes();
        
        let mut final_result = vec![];
        final_result.extend_from_slice(&msg_type);
        final_result.extend_from_slice(&length);
        final_result.extend_from_slice(&result);
        
        final_result
    }
}

#[derive(Getters, Debug)]
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

impl From<Status> for Vec<u8> {
    fn from(status: Status) -> Self {
        let mut result = vec![];

        let r#type: i8 = status.r#type.into();
        result.push(r#type as u8);

        result
    }
}

/// 0 -> OK
/// < 0 -> Unrecoverable error
/// > 0 -> Recoverable error
#[derive(PartialEq, Debug, Clone)]
pub enum StatusType {
    OK, // 0
    // Errors
    GenericError,        // -1
    NeedConnection,      // -2
    InternalServerError, // -3
    // Warnings
    AlreadyConnected,   // 1
    InvalidRequest,     // 2
    EventNotFound,      // 3
    ListenerNotFound,   // 4
    EventAlreadyExists, // 5
    AlreadySubscribed, // 6
}

impl TryFrom<i8> for StatusType {
    type Error = crate::Error;

    fn try_from(code: i8) -> Result<Self, crate::Error> {
        match code {
            0 => Ok(StatusType::OK),
            -1 => Ok(StatusType::GenericError),
            -2 => Ok(StatusType::NeedConnection),
            -3 => Ok(StatusType::InternalServerError),
            1 => Ok(StatusType::AlreadyConnected),
            2 => Ok(StatusType::InvalidRequest),
            3 => Ok(StatusType::EventNotFound),
            4 => Ok(StatusType::ListenerNotFound),
            5 => Ok(StatusType::EventAlreadyExists),
            6 => Ok(StatusType::AlreadySubscribed),
            _ => Err(crate::Error::InvalidStatus),
        }
    }
}

impl From<StatusType> for i8 {
    fn from(status: StatusType) -> Self {
        match status {
            StatusType::OK => 0i8,
            StatusType::GenericError => -1,
            StatusType::NeedConnection => -2,
            StatusType::InternalServerError => -3,
            StatusType::AlreadyConnected => 1,
            StatusType::InvalidRequest => 2,
            StatusType::EventNotFound => 3,
            StatusType::ListenerNotFound => 4,
            StatusType::EventAlreadyExists => 5,
            StatusType::AlreadySubscribed => 6,
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
            body: "345".as_bytes(),
        };

        let bytes: Vec<u8> = response.into();

        let response: Response = bytes.as_slice().try_into().unwrap();

        assert_eq!(response.head.version.major, 1);
        assert_eq!(response.head.version.patch, 2);
        assert_eq!(response.head.caller, "345");
        assert_eq!(response.status.r#type, StatusType::GenericError);
        assert_eq!(response.body, "345".as_bytes());
    }

    #[test]
    fn test_bytes_into_response() {
        let response: &[u8] = &[
            START_BYTE,
            0, 0, 0, 13, // length (13)
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
        assert_eq!(response.body, "345".as_bytes());
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
            body: "345".as_bytes(),
        };

        let bytes: Vec<u8> = response.into();

        assert_eq!(
            bytes,
            vec![
                START_BYTE,
                0, 0, 0, 13, // length (13)
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
        let bytes: Vec<u8> = status.into();

        assert_eq!(bytes, vec![0]);
    }
}
