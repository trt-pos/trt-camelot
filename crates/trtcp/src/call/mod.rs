use crate::Head;
use crate::Getters;

#[derive(Getters)]
pub struct Call<'c> {
    head: Head<'c>,
    body: [u8]
}