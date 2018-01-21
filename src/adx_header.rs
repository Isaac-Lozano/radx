use std::io::{Read, Write, Seek, SeekFrom};

use adx_reader::AdxReader;
use adx_writer::AdxWriter;
use error::{RadxResult, RadxError};

const ADX_MAGIC: u16 = 0x8000;
// TODO: Make function to pub this
pub(crate) const ADX_HEADER_LEN: usize = 0x0032;

#[derive(Clone,Copy,Debug)]
pub struct AdxVersion3LoopInfo {
    pub alignment_samples: u16,
    pub enabled_short: u16,
    pub enabled_int: u32,
    pub begin_sample: u32,
    pub begin_byte: u32,
    pub end_sample: u32,
    pub end_byte: u32,
}

#[derive(Clone,Copy,Debug)]
pub enum AdxVersion {
    Version3(Option<AdxVersion3LoopInfo>),
    Version4,
    /// Version 4 without looping support
    Version5,
    /// Seen in SA2B voice afs
    Version6,
}

impl From<AdxVersion> for u8 {
    fn from(val: AdxVersion) -> u8 {
        match val {
            AdxVersion::Version3(_) => 0x03,
            AdxVersion::Version4 => 0x04,
            AdxVersion::Version5 => 0x05,
            AdxVersion::Version6 => 0x06,
        }
    }
}

#[derive(Clone,Copy,Debug)]
pub enum AdxEncoding {
    Preset,
    Standard,
    Exponential,
    Ahx,
}

impl AdxEncoding {
    fn from_u8(val: u8) -> RadxResult<AdxEncoding> {
        match val {
            0x02 => Ok(AdxEncoding::Preset),
            0x03 => Ok(AdxEncoding::Standard),
            0x04 => Ok(AdxEncoding::Exponential),
            0x10 | 0x11 => Ok(AdxEncoding::Ahx),
            _ => Err(RadxError::BadAdxHeader("bad encoding value")),
        }
    }
}

impl From<AdxEncoding> for u8 {
    fn from(val: AdxEncoding) -> u8 {
        match val {
            AdxEncoding::Preset => 0x02,
            AdxEncoding::Standard => 0x03,
            AdxEncoding::Exponential => 0x04,
            AdxEncoding::Ahx => 0x10,
        }
    }
}

#[derive(Clone,Debug)]
pub struct AdxHeader {
//    pub data_offset: u16,
    pub encoding: AdxEncoding,
    pub block_size: u8,
    pub sample_bitdepth: u8,
    pub channel_count: u8,
    pub sample_rate: u32,
    pub total_samples: u32,
    pub highpass_frequency: u16,
    pub version: AdxVersion,
    pub flags: u8,
}

impl AdxHeader {
    pub fn read_header<S>(mut inner: S) -> RadxResult<AdxHeader>
        where S: Read + Seek
    {
        let magic = inner.read_u16()?;
        if magic != ADX_MAGIC {
            return Err(RadxError::BadAdxHeader("bad adx magic value"));
        }

        let data_offset = inner.read_u16()?;
        let encoding = AdxEncoding::from_u8(inner.read_u8()?)?;
        let block_size = inner.read_u8()?;
        let sample_bitdepth = inner.read_u8()?;
        let channel_count = inner.read_u8()?;
        let sample_rate = inner.read_u32()?;
        let total_samples = inner.read_u32()?;
        let highpass_frequency = inner.read_u16()?;
        let version_byte = inner.read_u8()?;
        let flags = inner.read_u8()?;
        let version = match version_byte {
            0x03 => {
                let loop_info = if data_offset >= 40 { 
                    let alignment_samples = inner.read_u16()?;
                    let enabled_short = inner.read_u16()?;
                    let enabled_int = inner.read_u32()?;
                    let begin_sample = inner.read_u32()?;
                    let begin_byte = inner.read_u32()?;
                    let end_sample = inner.read_u32()?;
                    let end_byte = inner.read_u32()?;
                    Some(
                        AdxVersion3LoopInfo {
                            alignment_samples: alignment_samples,
                            enabled_short: enabled_short,
                            enabled_int: enabled_int,
                            begin_sample: begin_sample,
                            begin_byte: begin_byte,
                            end_sample: end_sample,
                            end_byte: end_byte,
                        }
                    )
                }
                else {
                    None
                };
                AdxVersion::Version3(loop_info)
            }
            0x04 => AdxVersion::Version4,
            0x05 => AdxVersion::Version5,
            0x06 => AdxVersion::Version6,
            _ => return Err(RadxError::BadAdxHeader("bad adx version value")),
        };

        inner.seek(SeekFrom::Start(data_offset as u64 - 2))?;

        let mut copyright_buffer = [0u8; 6];
        inner.read_exact(&mut copyright_buffer)?;
        if &copyright_buffer != b"(c)CRI" {
            return Err(RadxError::BadAdxHeader("bad copyright string"));
        }

        Ok(AdxHeader {
//            data_offset: data_offset,
            encoding: encoding,
            block_size: block_size,
            sample_bitdepth: sample_bitdepth,
            channel_count: channel_count,
            sample_rate: sample_rate,
            total_samples: total_samples,
            highpass_frequency: highpass_frequency,
            version: version,
            flags: flags,
        })
    }

    pub fn to_writer<W>(&self, mut writer: W, header_size: usize) -> RadxResult<()>
        where W: Write
    {
        writer.write_u16(ADX_MAGIC)?;
        // Leave room for header stuff.
        writer.write_u16(header_size as u16 - 0x04)?;
        writer.write_u8(self.encoding.into())?;
        writer.write_u8(self.block_size)?;
        writer.write_u8(self.sample_bitdepth)?;
        writer.write_u8(self.channel_count)?;
        writer.write_u32(self.sample_rate)?;
        writer.write_u32(self.total_samples)?;
        writer.write_u16(self.highpass_frequency)?;
        writer.write_u8(self.version.into())?;
        writer.write_u8(self.flags)?;
        match self.version {
            AdxVersion::Version3(Some(ref loop_info)) => {
                writer.write_u16(loop_info.alignment_samples)?;
                writer.write_u16(loop_info.enabled_short)?;
                writer.write_u32(loop_info.enabled_int)?;
                writer.write_u32(loop_info.begin_sample)?;
                writer.write_u32(loop_info.begin_byte)?;
                writer.write_u32(loop_info.end_sample)?;
                writer.write_u32(loop_info.end_byte)?;
                for _ in 0..(header_size - 0x2c - 0x06) {
                    writer.write_u8(0)?;
                }
            }
            _ => {
                for _ in 0..(header_size - 0x14 - 0x06) {
                    writer.write_u8(0)?;
                }
            }
        }
        writer.write_all(b"(c)CRI")?;
        Ok(())
    }
}
