use crate::Head;
use getset::Getters;

#[derive(Getters)]
pub struct Request<'r> {
    #[get = "pub"]
    head: Head<'r>,
    #[get = "pub"]
    action: Action<'r>,
    #[get = "pub"]
    body: &'r [u8],
}

impl Request<'_> {
    pub fn new<'r, T: Into<&'r [u8]>>(head: Head<'r>, action: Action<'r>, body: T) -> Request<'r> {
        Request { head, action, body: body.into() }
    }
    
    pub fn body_as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.body)
    }
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

        Ok(Request { head, action, body })
    }
}

impl From<Request<'_>> for Vec<u8> {
    fn from(request: Request) -> Self {
        let mut result = vec![];

        let head_bytes: Vec<u8> = request.head.into();
        result.extend(head_bytes);

        result.push(0x1F);

        let action_bytes: Vec<u8> = request.action.into();
        result.extend(action_bytes);

        result.push(0x1F);

        result.extend_from_slice(request.body);

        result
    }
}

#[derive(Getters)]
pub struct Action<'r> {
    #[get = "pub"]
    r#type: ActionType,
    #[get = "pub"]
    module: &'r str,
    #[get = "pub"]
    id: &'r str,
}

impl Action<'_> {
    pub fn new<'r>(r#type: ActionType, module: &'r str, id: &'r str) -> Action<'r> {
        Action { r#type, module, id }
    }
}

impl<'r> TryFrom<&'r [u8]> for Action<'r> {
    type Error = crate::Error;
    fn try_from(action: &'r [u8]) -> Result<Self, Self::Error> {
        let (type_bytes, rest) = action.split_at(size_of::<u8>());
        let r#type = type_bytes.try_into()?;

        let namespace = std::str::from_utf8(rest).map_err(|_| crate::Error::InvalidAction)?;

        let action_id_separator = namespace.find(':').ok_or(crate::Error::InvalidAction)?;

        let module = &namespace[..action_id_separator];
        let id = &namespace[action_id_separator + 1..];

        Ok(Action { r#type, module, id })
    }
}

impl From<Action<'_>> for Vec<u8> {
    fn from(action: Action) -> Self {
        let mut result = vec![];

        let action_type_bytes: Vec<u8> = action.r#type.into();
        result.extend(action_type_bytes);
        result.extend_from_slice(action.module.as_bytes());
        result.push(b':');
        result.extend_from_slice(action.id.as_bytes());

        result
    }
}

#[derive(PartialEq, Debug)]
pub enum ActionType {
    Connect,
    Listen,
    Call,
    Leave,
    Create,
}

impl TryFrom<&[u8]> for ActionType {
    type Error = crate::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match value {
            [0] => Ok(ActionType::Connect),
            [1] => Ok(ActionType::Listen),
            [2] => Ok(ActionType::Call),
            [3] => Ok(ActionType::Create),
            [4] => Ok(ActionType::Leave),
            _ => Err(crate::Error::InvalidActionType),
        }
    }
}

impl From<ActionType> for Vec<u8> {
    fn from(value: ActionType) -> Self {
        match value {
            ActionType::Connect => vec![0],
            ActionType::Listen => vec![1],
            ActionType::Call => vec![2],
            ActionType::Create => vec![3],
            ActionType::Leave => vec![4],
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_req_into_req() {
        let request = Request {
            head: Head {
                version: crate::Version { major: 1, patch: 2 },
                caller: "345",
            },
            action: Action {
                r#type: ActionType::Listen,
                module: "ns",
                id: "id",
            },
            body: "hello".as_bytes(),
        };

        let bytes: Vec<u8> = request.try_into().unwrap();

        let request = Request::try_from(&bytes[..]).unwrap();

        let head = request.head;
        assert_eq!(head.version.major, 1);
        assert_eq!(head.version.patch, 2);
        assert_eq!(head.caller, "345");

        let action = request.action;
        assert_eq!(action.r#type, ActionType::Listen);
        assert_eq!(action.module, "ns");
        assert_eq!(action.id, "id");

        assert_eq!(request.body, "hello".as_bytes());
    }

    #[test]
    fn test_bytes_into_request() {
        let request: &[u8] = &[
            0, 1, // major (1)
            0, 2, // patch (2)
            51, 52, 53,   // caller ("345")
            0x1F, // separator
            2,    // type Call
            0x6e, 115, 0x3a, 105, 100,  // namespace ("ns:id")
            0x1F, // separator
            104, 101, 108, 108, 111, // body ("hello")
        ];

        let request = Request::try_from(request).unwrap();

        let head = request.head;
        assert_eq!(head.version.major, 1);
        assert_eq!(head.version.patch, 2);
        assert_eq!(head.caller, "345");

        let action = request.action;
        assert_eq!(action.r#type, ActionType::Call);
        assert_eq!(action.module, "ns");
        assert_eq!(action.id, "id");

        assert_eq!(request.body, "hello".as_bytes());
    }

    #[test]
    fn test_request_into_bytes() {
        let request = Request {
            head: Head {
                version: crate::Version { major: 1, patch: 2 },
                caller: "345",
            },
            action: Action {
                r#type: ActionType::Leave,
                module: "ns",
                id: "id",
            },
            body: "hello".as_bytes(),
        };

        let bytes: Vec<u8> = request.try_into().unwrap();

        assert_eq!(
            bytes,
            vec![
                0, 1, // major (1)
                0, 2, // patch (2)
                51, 52, 53,   // caller ("345")
                0x1F, // separator
                4,    // type Transaction
                0x6e, 115, 0x3a, 105, 100,  // namespace ("ns:id")
                0x1F, // separator
                104, 101, 108, 108, 111, // body ("hello")
            ]
        );
    }

    #[test]
    fn test_bytes_into_action() {
        let action: &[u8] = &[
            2, // type Call
            0x6e, 115, 0x3a, 105, 100, // namespace ("ns:id")
        ];

        let action: Action = action.try_into().unwrap();

        assert_eq!(action.r#type, ActionType::Call);
        assert_eq!(action.module, "ns");
        assert_eq!(action.id, "id");
    }

    #[test]
    fn test_action_into_bytes() {
        let action = Action {
            r#type: ActionType::Connect,
            module: "ns",
            id: "id",
        };

        let bytes: Vec<u8> = action.try_into().unwrap();

        assert_eq!(
            bytes,
            vec![
                0, // type Query
                0x6e, 115, 0x3a, 105, 100, // namespace ("ns:id")
            ]
        );
    }
}
