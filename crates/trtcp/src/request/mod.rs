use crate::{Head};

pub struct Request<'r> {
    head: Head<'r>,
    action: Action<'r>,
    body: &'r str,
}

impl<'r> TryFrom<&'r [u8]> for Request<'r> {
    type Error = crate::Error;
    fn try_from(request: &'r [u8]) -> Result<Self, Self::Error> {
        let split_request = request.split(|&x| x == 0x1F).collect::<Vec<&[u8]>>();

        if split_request.len() != 3 {
            return Err(crate::Error::InvalidRequest);
        }

        let head = split_request[0].try_into()?;
        let action = split_request[1].try_into()?;
        
        let body = split_request[2];
        let body = std::str::from_utf8(body).map_err(|_| crate::Error::InvalidBody)?;

        Ok(Request { head, action, body })
    }
}

pub struct Action<'r> {
    r#type: i32,
    module: &'r str,
    id: &'r str,
}

impl<'r> TryFrom<&'r [u8]> for Action<'r> {
    type Error = crate::Error;
    fn try_from(action: &'r [u8]) -> Result<Self, Self::Error> {
        let (int_bytes, rest) = action.split_at(size_of::<i32>());
        let r#type = i32::from_be_bytes(
            int_bytes
                .try_into()
                .map_err(|_| crate::Error::InvalidAction)?,
        );

        let namespace = std::str::from_utf8(rest).map_err(|_| crate::Error::InvalidAction)?;

        let action_id_separator = namespace.find(':').ok_or(crate::Error::InvalidAction)?;

        let module = &namespace[..action_id_separator];
        let id = &namespace[action_id_separator + 1..];

        Ok(Action {
            r#type,
            module,
            id,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_request_try_form() {
        let request: &[u8] = &[
            0, 0, 0, 1, // major (1)
            0, 0, 0, 2, // patch (2)
            51, 52, 53, // caller ("345")
            0x1F, // separator
            0, 0, 0, 6, // type
            0x6e, 115, 0x3a, 105, 100, // namespace ("ns:id")
            0x1F, // separator
            104, 101, 108, 108, 111, // body ("hello")
        ];
        
        let request = Request::try_from(request).unwrap();

        let head = request.head;
        assert_eq!(head.version.major, 1);
        assert_eq!(head.version.patch, 2);
        assert_eq!(head.caller, "345");
        
        let action = request.action;
        assert_eq!(action.r#type, 6);
        assert_eq!(action.module, "ns");
        assert_eq!(action.id, "id");
        
        assert_eq!(request.body, "hello");
    }

    #[test]
    fn test_parse_action() {

        assert_eq!(0x3a, b':');

        let action: &[u8] = &[
            0, 0, 0, 6, // type (6)
            0x6e, 115, 0x3a, 105, 100, // namespace ("ns:id")
        ];
        
        let action: Action = action.try_into().unwrap();

        assert_eq!(action.r#type, 6);
        assert_eq!(action.module, "ns");
        assert_eq!(action.id, "id");
    }
}
