use std::io::{self, Write, Seek, SeekFrom};
use std::iter;
use std::i16;

use {Sample, AdxSpec, gen_coeffs};
use adx_header::{AdxHeader, AdxEncoding, AdxVersion, AdxVersion3LoopInfo, ADX_HEADER_LEN};
use adx_writer::AdxWriter;

const HIGHPASS_FREQ: u32 = 0x01F4;

#[derive(Clone,Copy,Debug)]
struct Prev<T> {
    first: T,
    second: T,
}

#[derive(Clone,Copy,Debug)]
struct Block {
    prev: Prev<i16>,
    orig_prev: Prev<i16>,
    min: i32,
    max: i32,
    samples: [i16; 32],
    size: usize,
}

impl Block {
    fn new() -> Block {
        Block {
            prev: Prev {
                first: 0,
                second: 0,
            },
            orig_prev: Prev {
                first: 0,
                second: 0,
            },
            min: 0,
            max: 0,
            samples: [0; 32],
            size: 0,
        }
    }

    fn from_prev(other: &Block) -> Block {
        Block {
            prev: other.prev,
            orig_prev: other.prev,
            min: 0,
            max: 0,
            samples: [0; 32],
            size: 0,
        }
    }

    fn push(&mut self, sample: i16, coeffs: (i32, i32)) {
        let delta = (((sample as i32) << 12) - coeffs.0 * self.prev.first as i32 -
                     coeffs.1 * self.prev.second as i32) >> 12;
        if delta < self.min {
            self.min = delta;
        } else if delta > self.max {
            self.max = delta;
        }

        self.samples[self.size] = sample;
        self.size += 1;

        self.prev.second = self.prev.first;
        self.prev.first = sample;
    }
	
	fn is_empty(&self) -> bool {
		self.size == 0
	}

    fn is_full(&self) -> bool {
        self.size == 32
    }

    fn to_writer<W>(&mut self, mut writer: W, coeffs: (i32, i32)) -> io::Result<()>
        where W: Write
    {
        if self.min == 0 && self.max == 0 {
            for _ in 0..18 {
                writer.write_u8(0)?;
            }
            return Ok(());
        }

        let mut scale = if self.max / 7 > self.min / -8 {
            self.max / 7
        } else {
            self.min / -8
        };

        if scale == 0 {
            scale = 1;
        }

        self.prev = self.orig_prev;

        writer.write_u16(scale as u16)?;
        for byte_idx in 0..self.samples.len() / 2 {
            let sample1 = self.samples[byte_idx * 2];
            let sample2 = self.samples[byte_idx * 2 + 1];
            let upper_nibble = self.get_nibble(sample1, scale, coeffs);
            let lower_nibble = self.get_nibble(sample2, scale, coeffs);
            let byte = (upper_nibble << 4) | (lower_nibble & 0xF);
            writer.write_u8(byte)?;
        }
        Ok(())
    }

    fn get_nibble(&mut self, sample: i16, scale: i32, coeffs: (i32, i32)) -> u8 {
        let delta = (((sample as i32) << 12) - coeffs.0 * self.prev.first as i32 -
                     coeffs.1 * self.prev.second as i32) >> 12;

        // Rounded div
        let unclipped = if delta > 0 {
            (delta + (scale >> 1)) / scale
        } else {
            (delta - (scale >> 1)) / scale
        };

        // Clip
        let nibble = if unclipped >= 7 {
            7
        } else if unclipped <= -8 {
            -8
        } else {
            unclipped
        };

        let simulated_unclipped_sample =
            (((nibble) << 12) * scale + coeffs.0 * self.prev.first as i32 +
             coeffs.1 * self.prev.second as i32) >> 12;
        // Clamp sample
        let simulated_sample = if simulated_unclipped_sample >= i16::MAX as i32 {
            i16::MAX
        } else if simulated_unclipped_sample <= i16::MIN as i32 {
            i16::MIN
        } else {
            simulated_unclipped_sample as i16
        };

        self.prev.second = self.prev.first;
        self.prev.first = simulated_sample;

        nibble as u8
    }
}

#[derive(Clone,Debug)]
struct Frame {
    blocks: Vec<Block>,
}

impl Frame {
    fn new(channels: usize) -> Frame {
        Frame { blocks: iter::repeat(Block::new()).take(channels).collect() }
    }

    fn from_prev(other: &Frame) -> Frame {
        let mut blocks = Vec::new();

        for block in other.blocks.iter() {
            blocks.push(Block::from_prev(block));
        }

        Frame { blocks: blocks }
    }

    fn push(&mut self, sample: Sample, coeffs: (i32, i32)) {
        for (channel, block) in self.blocks.iter_mut().enumerate() {
            block.push(sample[channel], coeffs);
        }
    }
	
	fn is_empty(&self) -> bool {
		self.blocks[0].is_empty()
	}

    fn is_full(&self) -> bool {
        self.blocks[0].is_full()
    }

    fn to_writer<W>(&mut self, mut writer: W, coeffs: (i32, i32)) -> io::Result<()>
        where W: Write
    {
        for block in self.blocks.iter_mut() {
            block.to_writer(&mut writer, coeffs)?;
        }
        Ok(())
    }
}

#[derive(Clone,Debug)]
pub struct StandardEncoder<W> {
    inner: W,
    spec: AdxSpec,
	header_size: usize,
    alignment_samples: usize,
    coeffs: (i32, i32),
    samples_encoded: usize,
    current_frame: Frame,
}

impl<W> StandardEncoder<W>
    where W: Write + Seek
{
    pub fn new(mut writer: W, mut spec: AdxSpec) -> io::Result<StandardEncoder<W>> {
        let alignment_samples = spec.loop_info
            .as_mut()
            .map(|li| {
                let alignment_samples = (32 - (li.start_sample % 32)) % 32;
                li.start_sample += alignment_samples;
                li.end_sample += alignment_samples;
                alignment_samples as usize
            })
            .unwrap_or(0);
		
		let header_size = spec.loop_info
			.map(|li| {
				let bytes_till_loop_start = Self::sample_to_byte(li.start_sample, spec.channels);
				let mut fs_blocks = bytes_till_loop_start / 0x800;
				if bytes_till_loop_start % 0x800 > 0x800 - ADX_HEADER_LEN {
					fs_blocks += 1;
				}
				fs_blocks += 1;
				fs_blocks * 0x800 - bytes_till_loop_start
			})
			.unwrap_or(ADX_HEADER_LEN);

        writer.seek(SeekFrom::Start(header_size as u64))?;
			
        let mut encoder = StandardEncoder {
            inner: writer,
            spec: spec,
			header_size: header_size,
            alignment_samples: alignment_samples,
            coeffs: gen_coeffs(HIGHPASS_FREQ, spec.sample_rate),
            samples_encoded: 0,
            current_frame: Frame::new(spec.channels as usize),
        };
        encoder.encode_data(iter::repeat(iter::repeat(0)
                    .take(spec.channels as usize)
                    .collect::<Sample>())
                .take(alignment_samples))?;
        Ok(encoder)
    }

    pub fn encode_data<I>(&mut self, samples: I) -> io::Result<()>
        where I: IntoIterator<Item = Sample>
    {
        for sample in samples {
            self.current_frame.push(sample, self.coeffs);
			self.samples_encoded += 1;
            if self.current_frame.is_full() {
                self.current_frame.to_writer(&mut self.inner, self.coeffs)?;
                let new_frame = Frame::from_prev(&self.current_frame);
                self.current_frame = new_frame;
            }
        }
        Ok(())
    }

    pub fn finish(mut self) -> io::Result<()> {
		if !self.current_frame.is_empty() {
			self.current_frame.to_writer(&mut self.inner, self.coeffs)?;
		}
        self.inner.write_u16(0x8001)?;
        self.inner.write_u16(0x000e)?;
        for _ in 0..14 {
            self.inner.write_u8(0x00)?;
        }
        self.inner.seek(SeekFrom::Start(0))?;

        let loop_info = self.spec.loop_info.map(|li| {
            AdxVersion3LoopInfo {
                alignment_samples: self.alignment_samples as u16,
                enabled_short: 1,
                enabled_int: 1,
                begin_sample: li.start_sample,
                begin_byte: (Self::sample_to_byte(li.start_sample, self.spec.channels) + self.header_size) as u32,
                end_sample: li.end_sample,
                end_byte: (Self::sample_to_byte(li.end_sample, self.spec.channels) + self.header_size) as u32,
            }
        });

        let header = AdxHeader {
            encoding: AdxEncoding::Standard,
            block_size: 18,
            sample_bitdepth: 4,
            channel_count: self.spec.channels as u8,
            sample_rate: self.spec.sample_rate,
            total_samples: self.samples_encoded as u32,
            highpass_frequency: HIGHPASS_FREQ as u16,
            version: AdxVersion::Version3(loop_info),
            flags: 0,
        };
        header.to_writer(self.inner, self.header_size)?;
        Ok(())
    }
	
	fn sample_to_byte(start_sample: u32, channels: u32) -> usize {
		// (li.start_sample / 8) * 9 + ADX_HEADER_LEN as u32
		let mut frames = start_sample / 32;
		if start_sample % 32 != 0 {
			frames += 1;
		}
		(frames * 18 * channels) as usize
	}
}

#[cfg(test)]
mod tests {
    use super::Block;
    use gen_coeffs;

    #[test]
    fn test_block_write() {
        let coeffs = gen_coeffs(500, 32000);
        let mut buf = Vec::new();
        let mut block = Block::new();
        for _ in 0..32 {
            block.push(100, coeffs);
        }
        block.to_writer(&mut buf).unwrap();
        println!("{:#?}", block);
        block = Block::from_prev(&block);
        for _ in 0..32 {
            block.push(1, coeffs);
        }
        block.to_writer(&mut buf).unwrap();
        println!("{:#?}", block);
        println!("{:?}", buf);
        assert!(false);
    }
}
