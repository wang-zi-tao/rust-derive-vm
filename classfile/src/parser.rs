use crate::constants::Constant;
use failure::{Fallible};
use std::{convert::TryInto, slice::Iter};
#[derive(Fail, Debug)]
#[fail(display = "Unexcepted EOF!")]
pub struct FormatError;
#[derive(Clone)]
pub struct Parser<'p> {
    iter: Iter<'p, u8>,
}
impl<'p> Parser<'p> {
    pub fn from_slice(s: &'p [u8]) -> Self {
        Self::from_iter(s.iter())
    }

    pub fn from_iter(iter: Iter<'p, u8>) -> Self {
        Self { iter }
    }

    pub fn next_byte(&mut self) -> Fallible<[u8; 1]> {
        let slice = self.iter.as_slice();
        self.iter = slice.get(1..).ok_or(FormatError)?.iter();
        slice[0..1].try_into().or(Err(FormatError.into()))
    }

    pub fn next_2byte(&mut self) -> Fallible<[u8; 2]> {
        let slice = self.iter.as_slice();
        self.iter = slice.get(2..).ok_or(FormatError)?.iter();
        slice[0..2].try_into().or(Err(FormatError.into()))
    }

    pub fn next_4byte(&mut self) -> Fallible<[u8; 4]> {
        let slice = self.iter.as_slice();
        self.iter = slice.get(4..).ok_or(FormatError)?.iter();
        slice[0..4].try_into().or(Err(FormatError.into()))
    }

    pub fn next_8byte(&mut self) -> Fallible<[u8; 8]> {
        let slice = self.iter.as_slice();
        self.iter = slice.get(8..).ok_or(FormatError)?.iter();
        slice[0..8].try_into().or(Err(FormatError.into()))
    }

    pub fn next_vec_u8(&mut self, length: usize) -> Fallible<Vec<u8>> {
        let slice = self.iter.as_slice();
        self.iter = slice.get(length..).ok_or(FormatError)?.iter();
        slice[0..length].try_into().or(Err(FormatError.into()))
    }

    pub fn next_u8(&mut self) -> Fallible<u8> {
        self.next_byte().map(u8::from_be_bytes)
    }

    pub fn next_u16(&mut self) -> Fallible<u16> {
        self.next_2byte().map(u16::from_be_bytes)
    }

    pub fn next_u32(&mut self) -> Fallible<u32> {
        self.next_4byte().map(u32::from_be_bytes)
    }

    pub fn next_u64(&mut self) -> Fallible<u64> {
        self.next_8byte().map(u64::from_be_bytes)
    }

    pub fn next_i16(&mut self) -> Fallible<i16> {
        self.next_2byte().map(i16::from_be_bytes)
    }

    pub fn next_i32(&mut self) -> Fallible<i32> {
        self.next_4byte().map(i32::from_be_bytes)
    }

    pub fn next_i64(&mut self) -> Fallible<i64> {
        self.next_8byte().map(i64::from_be_bytes)
    }

    pub fn next_f32(&mut self) -> Fallible<f32> {
        self.next_4byte().map(f32::from_be_bytes)
    }

    pub fn next_f64(&mut self) -> Fallible<f64> {
        self.next_8byte().map(f64::from_be_bytes)
    }

    pub fn next_constant_index(&mut self, constant_pool: &Vec<Constant>) -> Fallible<Constant> {
        constant_pool
            .get(self.next_u16()? as usize)
            .cloned()
            .ok_or(FormatError.into())
    }

    pub fn next_constant_index_oprional(
        &mut self,
        constant_pool: &Vec<Constant>,
    ) -> Fallible<Option<Constant>> {
        match self.next_u16()? as usize {
            0 => Ok(None),
            index => constant_pool
                .get(index as usize)
                .cloned()
                .map(Some)
                .ok_or(FormatError.into()),
        }
    }

    pub fn take(&mut self, n: usize) -> Fallible<Parser<'p>> {
        let slice = &self.iter.as_slice()[0..n];
        self.iter = self.iter.as_slice().get(n..).ok_or(FormatError)?.iter();
        Ok(Parser::new(slice))
    }

    pub fn new(slice: &'p [u8]) -> Self {
        Self { iter: slice.iter() }
    }

    pub fn len(&self) -> usize {
        self.iter.as_slice().len()
    }

    pub fn as_slice(&self) -> &[u8] {
        self.iter.as_slice()
    }
}
impl<'a> Iterator for Parser<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_u8().ok()
    }
}
