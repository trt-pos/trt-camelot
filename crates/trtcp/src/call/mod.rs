use crate::Getters;
use crate::Head;
use std::str::Utf8Error;

#[derive(Getters)]
pub struct Call<'c> {
    head: Head<'c>,
    body: &'c [u8]
}

impl Call<'_> {
    pub fn new<'c>(head: Head<'c>, body: &'c [u8]) -> Call<'c> {
        Call { head, body }
    }
    
    pub fn body_as_str(&self) -> Result<&str, Utf8Error> {
        std::str::from_utf8(self.body)
    }
}

impl<'c> TryFrom<&'c [u8]> for Call<'c> {
    type Error = crate::Error;
    fn try_from(request: &'c [u8]) -> Result<Self, Self::Error> {
        let split_request = request.split(|&x| x == 0x1F).collect::<Vec<&[u8]>>();

        if split_request.len() != 2 {
            return Err(crate::Error::InvalidCall);
        }

        let head = split_request[0].try_into()?;

        let body = split_request[1];

        Ok(Call { head, body })
    }
}

impl From<Call<'_>> for Vec<u8> {
    fn from(call: Call) -> Self {
        let mut result = vec![];

        let head_bytes: Vec<u8> = call.head.into();
        result.extend(head_bytes);

        result.push(0x1F);

        result.extend_from_slice(call.body);

        result
    }
}