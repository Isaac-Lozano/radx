pub mod standard_decoder;
pub mod ahx_decoder;

pub(crate) use self::standard_decoder::StandardDecoder;
pub(crate) use self::ahx_decoder::AhxDecoder;

use {Sample, LoopInfo};

pub trait Decoder {
    fn channels(&self) -> u32;
    fn sample_rate(&self) -> u32;
	fn loop_info(&self) -> Option<LoopInfo>;
    fn next_sample(&mut self) -> Option<Sample>;
}

impl Iterator for Decoder {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_sample()
    }
}
