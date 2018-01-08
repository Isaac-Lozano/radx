extern crate radx;

extern crate getopts;
extern crate hound;

use std::env;
use std::fs::File;
use std::io::{Read, Seek, BufReader, BufWriter};
use std::process;

use radx::adx_header::AdxHeader;

use getopts::Options;

use hound::{WavWriter, WavSpec, SampleFormat};

fn main() {
    let mut args = env::args();
    let prog_name = args.next().unwrap();
    let options: Vec<_> = args.collect();

    // Create options
    let mut opts = Options::new();
    opts.optopt("l", "loop", "Loop N times", "N");
    opts.optflag("i", "info", "Print adx header info");
    opts.optflag("h", "help", "Print this help menu");

    // Parse options
    let matches = match opts.parse(&options) {
        Ok(matches) => matches,
        Err(err) => { barf(&err.to_string()) }
    };

    // Print help message if we have to
    if matches.opt_present("h") {
        help(&prog_name, opts);
    }

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
            filename_clone.push_str(".wav");
            filename_clone
        });

    // Determine number of loops to read
    let loops_opt = matches
        .opt_str("l")
        .and_then(|start_str| { start_str.parse::<u32>().ok() });

    // Open adx file and make reader/print header
    let adx_file = BufReader::new(File::open(filename).unwrap_or_else(|_| barf("Could not open adx file.")));
	if matches.opt_present("i") {
		print_info(adx_file);
	}
    let mut adx = radx::from_reader(adx_file, loops_opt.is_some()).unwrap_or_else(|_| barf("Could not make adx reader."));

    // Print adx info
    println!("ADX info:");
    println!("    channels: {}", adx.channels());
    println!("    Sample rate: {}", adx.sample_rate());
    if let Some(loop_info) = adx.loop_info() {
        println!("    Loop start sample: {}", loop_info.start_sample);
        println!("    Loop end sample: {}", loop_info.end_sample);
    }
    else {
        println!("    Non-looping ADX");
    }

    // Make wav spec
    let spec = WavSpec {
        channels: adx.channels() as u16,
        sample_rate: adx.sample_rate(),
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    // Open wav writer
    let wav_file = BufWriter::new(File::create(output_filename).unwrap_or_else(|_| barf("Could not open output file.")));
    let mut wav_writer = WavWriter::new(wav_file, spec).unwrap_or_else(|_| barf("Could not make wav writer."));

    // Read depending on number of loops
    println!("Decoding and writing wav.");
    if let Some(loops) = loops_opt {
        if let Some(loop_info) = adx.loop_info() {
            let samples_to_read = loop_info.start_sample + loops * (loop_info.end_sample - loop_info.start_sample);
            for _ in 0..samples_to_read {
                let sample = adx.next_sample().unwrap();
                for channel_sample in sample {
                    wav_writer.write_sample(channel_sample).unwrap_or_else(|_| barf("Problem writing wav samples."));
                }
            }
        }
        else {
            barf("File is not a looping ADX. Do not use \"-l\".");
        }
    }
    else {
        for sample in adx {
            for channel_sample in sample {
                wav_writer.write_sample(channel_sample).unwrap_or_else(|_| barf("Problem writing wav samples."));
            }
        }
    };

    // Finish writing to the wav
    wav_writer.finalize().unwrap_or_else(|_| barf("Could not finalize writing wav file."));
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

fn print_info<R>(reader: R) -> !
    where R: Read + Seek
{
    let header = AdxHeader::read_header(reader).unwrap_or_else(|_| barf("Could not read adx header."));
	println!("{:#?}", header);
	process::exit(0);
}