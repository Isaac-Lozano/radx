use std::io::{self, Read};

use byteorder::{BigEndian, ReadBytesExt};

pub trait AdxReader
{
    fn read_u8(&mut self) -> io::Result<u8>;
    fn read_u16(&mut self) -> io::Result<u16>;
    fn read_u32(&mut self) -> io::Result<u32>;
}

impl<R> AdxReader for R
    where R: Read
{
    fn read_u8(&mut self) -> io::Result<u8> {
        ReadBytesExt::read_u8(self)
    }

    fn read_u16(&mut self) -> io::Result<u16> {
        ReadBytesExt::read_u16::<BigEndian>(self)
    }

    fn read_u32(&mut self) -> io::Result<u32> {
        ReadBytesExt::read_u32::<BigEndian>(self)
    }
}
