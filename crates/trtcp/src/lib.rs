#![allow(dead_code)]

use std::str;

mod error;

pub struct Version {
    pub major: i32,
    pub patch: i32,
}

pub struct Head<'r> {
    pub version: Version,
    pub caller: &'r str,
}

pub struct Action<'r> {
    r#type: i32,
    module: &'r str,
    id: &'r str,
}

pub struct Request<'r> {
    head: Head<'r>,
    action: Action<'r>,
    body: &'r str,
}

pub fn parse_request(request: &[u8]) -> Result<Request, error::Error> {
    let split_request = request.split(|&x| x == 0x1F).collect::<Vec<&[u8]>>();

    if split_request.len() != 3 {
        return Err(error::Error::InvalidRequest);
    }

    let head = split_request[0];
    let action = split_request[1];
    let body = split_request[2];

    let head = parse_head(head)?;
    let action = parse_action(action)?;
    let body = str::from_utf8(body).map_err(|_| error::Error::InvalidBody)?;

    Ok(Request { head, action, body })
}

fn parse_head(head: &[u8]) -> Result<Head, error::Error> {
    let major = i32::from_be_bytes(
        head[0..4]
            .try_into()
            .map_err(|_| error::Error::InvalidHead)?,
    );
    let patch = i32::from_be_bytes(
        head[4..8]
            .try_into()
            .map_err(|_| error::Error::InvalidHead)?,
    );
    let caller = str::from_utf8(&head[8..]).map_err(|_| error::Error::InvalidHead)?;

    Ok(Head {
        version: Version { major, patch },
        caller,
    })
}

fn parse_action(action: &[u8]) -> Result<Action, error::Error> {
    let r#type = i32::from_be_bytes(
        action[0..4]
            .try_into()
            .map_err(|_| error::Error::InvalidAction)?,
    );
    let namespace = str::from_utf8(&action[4..]).map_err(|_| error::Error::InvalidAction)?;

    let action_id_separator = namespace.find(':').ok_or(error::Error::InvalidAction)?;

    let module = &namespace[..action_id_separator];
    let id = &namespace[action_id_separator + 1..];

    Ok(Action {
        r#type,
        module,
        id,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_head() {
        let head = [
            0, 0, 0, 1, // major (1)
            0, 0, 0, 2, // patch (2)
            51, 52, 53, // caller ("345")
        ];
        let head = parse_head(&head).unwrap();

        assert_eq!(head.version.major, 1);
        assert_eq!(head.version.patch, 2);
        assert_eq!(head.caller, "345");
    }

    #[test]
    fn test_parse_action() {
        
        assert_eq!(0x3a, b':');
        
        let action = [
            0, 0, 0, 6, // type (6)
            0x6e, 115, 0x3a, 105, 100, // namespace ("ns:id")
        ];
        let action = parse_action(&action).unwrap();

        assert_eq!(action.r#type, 6);
        assert_eq!(action.module, "ns");
        assert_eq!(action.id, "id");
    }
}
