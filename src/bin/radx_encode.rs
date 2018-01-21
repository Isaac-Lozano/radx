extern crate radx;

extern crate getopts;
extern crate hound;

use std::error::Error;
use std::env;
use std::io::{Read, BufReader, BufWriter};
use std::fs::File;
use std::process;

use radx::{AdxSpec, LoopInfo};
use radx::encoder::standard_encoder::StandardEncoder;
use radx::encoder::ahx_encoder::AhxEncoder;

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
    opts.optflag("a", "ahx", "Use ahx encoding (cannot loop)");
    opts.optflag("h", "help", "Print this help menu");

    // Parse options
    let matches = unwrap_or_barf(opts.parse(&options), "Could not parse options");

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
    let input = BufReader::new(unwrap_or_barf(File::open(filename), "Could not open input file."));
    let output = BufWriter::new(unwrap_or_barf(File::create(&output_filename), "Could not open output file."));

    // Change based on encoding
    if matches.opt_present("a") {
        // Read samples
        println!("Reading Samples.");
        let (samples, sample_rate) = unwrap_or_barf(read_samples_ahx(input), "Could not read samples from input.");

        if sample_rate != 22050 {
            barf("ahx encoding requires a sample rate of 22050.");
        }

        // Make encoder
        let mut encoder = unwrap_or_barf(AhxEncoder::new(output), "Could not make encoder.");

        // Encode data
        println!("Encoding data.");
        unwrap_or_barf(encoder.encode_data(samples), "Could not encode data.");
        unwrap_or_barf(encoder.finalize(), "Could not finish writing adx file.");
    }
    else {
        // Read samples
        println!("Reading Samples.");
        let (samples, sample_rate) = unwrap_or_barf(read_samples(input), "Could not read samples from input.");

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
        let mut encoder = unwrap_or_barf(StandardEncoder::new(output, spec), "Could not make encoder.");

        // Encode data
        println!("Encoding data.");
        unwrap_or_barf(encoder.encode_data(samples), "Could not encode data.");
        unwrap_or_barf(encoder.finish(), "Could not finish writing adx file.");
    }
}

fn barf(message: &str) -> ! {
    println!("Error: {}", message);
    process::exit(1);
}

fn unwrap_or_barf<T, E>(result: Result<T, E>, err_desc: &str) -> T
    where E: Error
{
    result.unwrap_or_else(|err| {
        let err_string = format!("{}: {}", err_desc, err);
        barf(&err_string);
    })
}

fn help(prog_name: &str, opts: Options) -> ! {
    let brief = format!("Usage: {} [options] INPUT [OUTPUT]", prog_name);
	println!("radx_encode {}", env!("CARGO_PKG_VERSION"));
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

fn read_samples_ahx<R>(reader: R) -> WavResult<(Vec<i16>, u32)>
    where R: Read
{
    let mut reader = WavReader::new(reader)?;
    let spec = reader.spec();
    if spec.channels == 1 {
        let samples: WavResult<_> = reader.samples::<i16>().collect();
        Ok((samples?, spec.sample_rate))
    }
    else {
        barf("ahx encoding requires 1 channel (mono)");
    }
}
