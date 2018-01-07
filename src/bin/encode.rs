extern crate byteorder;
extern crate radx;

use std::env;
use std::io::{self, Read, BufReader, BufWriter};
use std::fs::File;
use std::str::FromStr;

use byteorder::{BigEndian, ReadBytesExt};

use radx::{AdxSpec, LoopInfo};
use radx::encoder::standard_encoder::StandardEncoder;

fn main() {
    let mut args = env::args().skip(1);
    let filename = args.next().unwrap();
	let output_filename = args.next().unwrap();

    let mut input = BufReader::new(File::open(filename).unwrap());
    let output = BufWriter::new(File::create(output_filename).unwrap());

    let spec = if let Some((start_sample_str, end_sample_str)) = args.next().and_then(|start| args.next().map(|end| (start, end))) {
        let start_sample = u32::from_str(&start_sample_str).unwrap();
        let end_sample = u32::from_str(&end_sample_str).unwrap();
        AdxSpec {
            channels: 2,
            sample_rate: 32000,
            loop_info: Some(
                LoopInfo {
                    start_sample: start_sample,
                    end_sample: end_sample,
                }
            )
        }
    }
    else {
        AdxSpec {
            channels: 2,
            sample_rate: 32000,
            loop_info: None,
        }
    };

    let mut encoder = StandardEncoder::new(output, spec).unwrap();
    println!("{:#?}", encoder);

    println!("Reading Samples.");
    let mut samples = Vec::new();
    while let Ok((sample1, sample2)) = read_sample(&mut input) {
        let sample = vec![sample1, sample2];
        samples.push(sample);
    }

    println!("Encoding data.");
    encoder.encode_data(samples).unwrap();
    encoder.finish().unwrap();
}

fn read_sample<R>(mut reader: R) -> io::Result<(i16, i16)>
    where R: Read
{
    let s1 = reader.read_i16::<BigEndian>()?;
    let s2 = reader.read_i16::<BigEndian>()?;
    Ok((s1, s2))
}
