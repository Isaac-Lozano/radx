extern crate byteorder;
extern crate radx;

use std::env;
use std::fs::File;
use std::str::FromStr;
use std::io::{BufReader, BufWriter};

use byteorder::{BigEndian, WriteBytesExt};

fn main() {
    let mut args = env::args().skip(1);
    let filename = args.next().unwrap();

    let f = BufReader::new(File::open(filename).unwrap());
    let mut adx = radx::from_reader(f, true).unwrap();

    println!("channels: {}", adx.channels());
    println!("Sample rate: {}", adx.sample_rate());

    let mut file = BufWriter::new(File::create("output.i16be").unwrap());

    if let Some(num_samples_str) = args.next() {
        let num_samples = usize::from_str(&num_samples_str).unwrap();
        for _ in 0..num_samples {
            let sample = adx.next_sample().unwrap();
            file.write_i16::<BigEndian>(sample[0]).unwrap();
            file.write_i16::<BigEndian>(sample[1]).unwrap();
        }
    }
    else {
        for sample in adx {
            file.write_i16::<BigEndian>(sample[0]).unwrap();
            file.write_i16::<BigEndian>(sample[1]).unwrap();
        }
    }
}
