use std::io::{self, Write};

use byteorder::{BigEndian, WriteBytesExt};

pub trait AdxWriter
{
    fn write_u8(&mut self, val: u8) -> io::Result<()>;
    fn write_u16(&mut self, val: u16) -> io::Result<()>;
    fn write_u32(&mut self, val: u32) -> io::Result<()>;
}

impl<W> AdxWriter for W
    where W: Write
{
    fn write_u8(&mut self, val: u8) -> io::Result<()> {
        WriteBytesExt::write_u8(self, val)
    }

    fn write_u16(&mut self, val: u16) -> io::Result<()> {
        WriteBytesExt::write_u16::<BigEndian>(self, val)
    }

    fn write_u32(&mut self, val: u32) -> io::Result<()> {
        WriteBytesExt::write_u32::<BigEndian>(self, val)
    }
}
