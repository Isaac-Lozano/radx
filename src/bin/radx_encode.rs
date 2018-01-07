extern crate radx;

extern crate getopts;
extern crate hound;

use std::env;
use std::io::{Read, BufReader, BufWriter};
use std::fs::File;
use std::process;

use radx::{AdxSpec, LoopInfo};
use radx::encoder::standard_encoder::StandardEncoder;

use getopts::Options;

use hound::{WavReader, Error as WavError, Result as WavResult};

fn main() {
	let mut args = env::args();
	let prog_name = args.next().unwrap();
    let options: Vec<_> = args.collect();
	
	// Create options
	let mut opts = Options::new();
	opts.optopt("s", "start", "Loop start sample (defaults to song start)", "START");
	opts.optopt("e", "end", "Loop end sample (defaults to song end)", "END");
	opts.optflag("n", "no-loop", "Don't loop the song");
    opts.optflag("h", "help", "print this help menu");
	
	// Parse options
	let matches = match opts.parse(&options) {
		Ok(matches) => matches,
		Err(err) => { barf(&err.to_string()) }
	};
	
	// Print help message if we have to
	if matches.opt_present("h") {
		help(&prog_name, opts);
	}

	// Get start and end samples
	let start_sample = matches
		.opt_str("s")
		.and_then(|start_str| { start_str.parse::<u32>().ok() })
		.unwrap_or(0);
	
	let end_sample_opt = matches
		.opt_str("e")
		.and_then(|end_str| { end_str.parse::<u32>().ok() });

	// Get input and output files
	let mut free_iter = matches.free.iter();
	let filename;
	if let Some(f) = free_iter.next() {
		filename = f;
	}
	else {
		help(&prog_name, opts);
	}
	let output_filename = free_iter
		.next()
		.map(|s| s.clone())
		.unwrap_or({
			let mut filename_clone = filename.clone();
			filename_clone.push_str(".adx");
			filename_clone
		});

	// Open input and output files
    let input = BufReader::new(File::open(filename).unwrap_or_else(|_| barf("Could not open input file.")));
    let output = BufWriter::new(File::create(&output_filename).unwrap_or_else(|_| barf("Could not open output file.")));

	// Read samples
    println!("Reading Samples.");
    let (samples, sample_rate) = read_samples(input).unwrap_or_else(|_| barf("Could not read samples from input."));

	// Make adx spec
    let spec = if matches.opt_present("n") {
		AdxSpec {
            channels: 2,
            sample_rate: sample_rate,
            loop_info: None,
        }
	}
	else {
        AdxSpec {
            channels: 2,
            sample_rate: sample_rate,
            loop_info: Some(
                LoopInfo {
                    start_sample: start_sample,
                    end_sample: end_sample_opt.unwrap_or(samples.len() as u32),
                }
            )
        }
    };

	// Make encoder from spec
    let mut encoder = StandardEncoder::new(output, spec).unwrap_or_else(|_| barf("Could not make encoder"));

	// Encode data
    println!("Encoding data.");
    encoder.encode_data(samples).unwrap_or_else(|_| barf("Could not encode data."));
    encoder.finish().unwrap_or_else(|_| barf("Could not finish writing adx file."));
}

fn barf(message: &str) -> ! {
	println!("Error: {}", message);
	process::exit(1);
}

fn help(prog_name: &str, opts: Options) -> ! {
    let brief = format!("Usage: {} [options] INPUT [OUTPUT]", prog_name);
    print!("{}", opts.usage(&brief));
	process::exit(0);
}

fn read_samples<R>(reader: R) -> WavResult<(Vec<Vec<i16>>, u32)>
    where R: Read
{
    let mut reader = WavReader::new(reader)?;
	let spec = reader.spec();
	if spec.channels == 1 {
		let mut samples = reader.samples::<i16>();
		let mut sample_vec = Vec::new();
		while let Some(sample_res) = samples.next() {
			let sample = sample_res?;
			sample_vec.push(vec![sample, sample]);
		}
		Ok((sample_vec, spec.sample_rate))
	}
	else if spec.channels == 2 {
		let mut samples = reader.samples::<i16>();
		let mut sample_vec = Vec::new();
		while let Some(sample1_res) = samples.next() {
			let sample1 = sample1_res?;
			let sample2 = if let Some(sample_res) = samples.next() {
				sample_res?
			}
			else {
				sample1
			};
			sample_vec.push(vec![sample1, sample2]);
		}
		Ok((sample_vec, spec.sample_rate))
	}
	else {
		Err(WavError::Unsupported)
	}
}
