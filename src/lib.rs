extern crate byteorder;
#[macro_use]
extern crate lazy_static;

pub mod adx_header;
mod adx_reader;
mod adx_writer;
pub mod decoder;
pub mod encoder;
pub mod error;

use std::io::{Read, Seek};
use std::f64;

use adx_header::{AdxHeader, AdxEncoding};
use decoder::{Decoder, StandardDecoder, AhxDecoder};
use error::RadxResult;

#[derive(Clone,Copy,Debug)]
pub struct LoopInfo {
    pub start_sample: u32,
    pub end_sample: u32,
}

#[derive(Clone,Copy,Debug)]
pub struct AdxSpec {
    pub channels: u32,
    pub sample_rate: u32,
    pub loop_info: Option<LoopInfo>,
}

type Sample = Vec<i16>;

pub fn from_reader<R>(mut reader: R, looping: bool) -> RadxResult<Box<Decoder>>
    where R: Seek + Read + 'static
{
    let header = AdxHeader::read_header(&mut reader)?;
    match header.encoding {
        AdxEncoding::Standard =>
            Ok(Box::new(StandardDecoder::from_header(header, reader, looping))),
        AdxEncoding::Ahx =>
            Ok(Box::new(AhxDecoder::from_header(header, reader))),
        _ => unimplemented!(),
    }
}

/// Returns 12-bit fixed-point coefficients.
fn gen_coeffs(highpass_frequency: u32, sample_rate: u32) -> (i32, i32) {
    let highpass_samples = highpass_frequency as f64 / sample_rate as f64;
    let a = f64::consts::SQRT_2 - (2.0 * f64::consts::PI * highpass_samples).cos();
    let b = f64::consts::SQRT_2 - 1.0;
    let c = (a - ((a + b) * (a - b)).sqrt()) / b;

    let coeff1 = c * 2.0;
    let coeff2 = -(c * c);

    // 4096 = 1**12
    (((coeff1 * 4096.0) + 0.5) as i32, ((coeff2 * 4096.0) + 0.5) as i32)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
