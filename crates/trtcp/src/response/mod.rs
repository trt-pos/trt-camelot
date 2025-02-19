use crate::Head;

pub struct Response<'r> {
    head: Head<'r>,
    status: Status,
    body: &'r str,
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

pub struct Status {
    r#type: StatusType,
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

#[derive(PartialEq, Debug)]
pub enum StatusType {
    OK,    // 0
    Error, // -1
}

impl TryFrom<i8> for StatusType {
    type Error = crate::Error;

    fn try_from(code: i8) -> Result<Self, crate::Error> {
        match code {
            0 => Ok(StatusType::OK),
            -1 => Ok(StatusType::Error),
            _ => Err(crate::Error::InvalidStatus),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_response() {
        let response: &[u8] = &[
            0, 0, 0, 1, // major (1)
            0, 0, 0, 2, // patch (2)
            51, 52, 53, // caller ("345")
            0x1F, // separator
            0, 0, 0, 0, // code (1)
            0x1F, // separator
            51, 52, 53, // body ("345")
        ];

        let response = Response::try_from(response).unwrap();
        
        assert_eq!(response.head.version.major, 1);
        assert_eq!(response.head.version.patch, 2);
        assert_eq!(response.head.caller, "345");
        assert_eq!(response.status.r#type, StatusType::OK);
        assert_eq!(response.body, "345");
    }
    
    #[test]
    fn test_parse_status() {
        let status: &[u8] = &[0, 0, 0, 0];
        let status: Status = status.try_into().unwrap();
        
        assert_eq!(status.r#type, StatusType::OK);
    }
}
