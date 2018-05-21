use std::io::{Write, Seek, SeekFrom};
use std::ops::Index;

use adx_header::{AdxHeader, AdxEncoding, AdxVersion};
use error::RadxResult;

lazy_static! {
    static ref N: [[i64; 32]; 64] = {
        let mut n = [[0; 32]; 64];
        for i in 0..64 {
            for j in 0..32 {
                // 2 << FRAC_BITS == 268435456
                n[i][j] = ((((16 + i) * ((j << 1) + 1)) as f32 * 0.0490873852123405).cos() * 268435456.0) as i64
            }
        }
        n
    };
}

const ENWINDOW: [i64; 512] = [
     0x000000,-0x000080,-0x000080,-0x000080,-0x000080,-0x000080,-0x000080,-0x000100,-0x000100,-0x000100,-0x000100,-0x000180,-0x000180,-0x000200,-0x000200,-0x000280,
    -0x000280,-0x000300,-0x000380,-0x000380,-0x000400,-0x000480,-0x000500,-0x000580,-0x000680,-0x000700,-0x000800,-0x000880,-0x000980,-0x000A80,-0x000C00,-0x000D00,
    -0x000E80,-0x000F80,-0x001180,-0x001300,-0x001480,-0x001680,-0x001880,-0x001A80,-0x001D00,-0x001F80,-0x002200,-0x002480,-0x002780,-0x002A80,-0x002D80,-0x003080,
    -0x003400,-0x003780,-0x003A80,-0x003E80,-0x004200,-0x004580,-0x004980,-0x004D00,-0x005080,-0x005480,-0x005800,-0x005B80,-0x005F00,-0x006200,-0x006500,-0x006800,
     0x006A80, 0x006D00, 0x006F00, 0x007080, 0x007180, 0x007200, 0x007200, 0x007180, 0x007000, 0x006E80, 0x006B80, 0x006800, 0x006400, 0x005E80, 0x005880, 0x005180,
     0x004900, 0x003F80, 0x003500, 0x002980, 0x001C80, 0x000E80,-0x000100,-0x001200,-0x002400,-0x003780,-0x004C80,-0x006280,-0x007A00,-0x009300,-0x00AD80,-0x00C880,
    -0x00E580,-0x010380,-0x012280,-0x014280,-0x016380,-0x018580,-0x01A800,-0x01CB80,-0x01EF80,-0x021400,-0x023880,-0x025D00,-0x028180,-0x02A600,-0x02CA00,-0x02ED00,
    -0x030F80,-0x033100,-0x035100,-0x036F80,-0x038C80,-0x03A700,-0x03BF80,-0x03D500,-0x03E880,-0x03F800,-0x040480,-0x040D80,-0x041280,-0x041380,-0x041000,-0x040780,
     0x03FA80, 0x03E800, 0x03D000, 0x03B280, 0x038F00, 0x036580, 0x033600, 0x02FF80, 0x02C300, 0x028000, 0x023580, 0x01E500, 0x018D00, 0x012E80, 0x00C900, 0x005C80,
    -0x001680,-0x009000,-0x011080,-0x019700,-0x022380,-0x02B600,-0x034E00,-0x03EB00,-0x048D00,-0x053380,-0x05DE00,-0x068B80,-0x073C80,-0x07EF80,-0x08A480,-0x095A00,
    -0x0A1080,-0x0AC680,-0x0B7B80,-0x0C2E80,-0x0CDE80,-0x0D8B80,-0x0E3380,-0x0ED680,-0x0F7300,-0x100880,-0x109580,-0x111980,-0x119300,-0x120180,-0x126400,-0x12B880,
    -0x12FF80,-0x133700,-0x135E00,-0x137380,-0x137700,-0x136780,-0x134380,-0x130B00,-0x12BC00,-0x125680,-0x11D980,-0x114400,-0x109600,-0x0FCE00,-0x0EEC00,-0x0DEF00,
     0x0CD700, 0x0BA380, 0x0A5400, 0x08E880, 0x076000, 0x05BB80, 0x03FA80, 0x021D00, 0x002300,-0x01F300,-0x042500,-0x067200,-0x08DA80,-0x0B5D00,-0x0DF900,-0x10AE00,
    -0x137B80,-0x165F80,-0x195A00,-0x1C6A00,-0x1F8D80,-0x22C380,-0x260B00,-0x296280,-0x2CC880,-0x303B00,-0x33B900,-0x374080,-0x3AD000,-0x3E6580,-0x41FF80,-0x459C00,
    -0x493880,-0x4CD400,-0x506C00,-0x53FF00,-0x578A80,-0x5B0C80,-0x5E8300,-0x61EC80,-0x654680,-0x688F00,-0x6BC500,-0x6EE500,-0x71EE80,-0x74DF00,-0x77B480,-0x7A6E00,
    -0x7D0980,-0x7F8500,-0x81DF00,-0x841680,-0x862A00,-0x881780,-0x89DF00,-0x8B7E00,-0x8CF480,-0x8E4180,-0x8F6380,-0x905A00,-0x912480,-0x91C300,-0x923400,-0x927800,
     0x928F00, 0x927800, 0x923400, 0x91C300, 0x912480, 0x905A00, 0x8F6380, 0x8E4180, 0x8CF480, 0x8B7E00, 0x89DF00, 0x881780, 0x862A00, 0x841680, 0x81DF00, 0x7F8500,
     0x7D0980, 0x7A6E00, 0x77B480, 0x74DF00, 0x71EE80, 0x6EE500, 0x6BC500, 0x688F00, 0x654680, 0x61EC80, 0x5E8300, 0x5B0C80, 0x578A80, 0x53FF00, 0x506C00, 0x4CD400,
     0x493880, 0x459C00, 0x41FF80, 0x3E6580, 0x3AD000, 0x374080, 0x33B900, 0x303B00, 0x2CC880, 0x296280, 0x260B00, 0x22C380, 0x1F8D80, 0x1C6A00, 0x195A00, 0x165F80,
     0x137B80, 0x10AE00, 0x0DF900, 0x0B5D00, 0x08DA80, 0x067200, 0x042500, 0x01F300,-0x002300,-0x021D00,-0x03FA80,-0x05BB80,-0x076000,-0x08E880,-0x0A5400,-0x0BA380,
     0x0CD700, 0x0DEF00, 0x0EEC00, 0x0FCE00, 0x109600, 0x114400, 0x11D980, 0x125680, 0x12BC00, 0x130B00, 0x134380, 0x136780, 0x137700, 0x137380, 0x135E00, 0x133700,
     0x12FF80, 0x12B880, 0x126400, 0x120180, 0x119300, 0x111980, 0x109580, 0x100880, 0x0F7300, 0x0ED680, 0x0E3380, 0x0D8B80, 0x0CDE80, 0x0C2E80, 0x0B7B80, 0x0AC680,
     0x0A1080, 0x095A00, 0x08A480, 0x07EF80, 0x073C80, 0x068B80, 0x05DE00, 0x053380, 0x048D00, 0x03EB00, 0x034E00, 0x02B600, 0x022380, 0x019700, 0x011080, 0x009000,
     0x001680,-0x005C80,-0x00C900,-0x012E80,-0x018D00,-0x01E500,-0x023580,-0x028000,-0x02C300,-0x02FF80,-0x033600,-0x036580,-0x038F00,-0x03B280,-0x03D000,-0x03E800,
     0x03FA80, 0x040780, 0x041000, 0x041380, 0x041280, 0x040D80, 0x040480, 0x03F800, 0x03E880, 0x03D500, 0x03BF80, 0x03A700, 0x038C80, 0x036F80, 0x035100, 0x033100,
     0x030F80, 0x02ED00, 0x02CA00, 0x02A600, 0x028180, 0x025D00, 0x023880, 0x021400, 0x01EF80, 0x01CB80, 0x01A800, 0x018580, 0x016380, 0x014280, 0x012280, 0x010380,
     0x00E580, 0x00C880, 0x00AD80, 0x009300, 0x007A00, 0x006280, 0x004C80, 0x003780, 0x002400, 0x001200, 0x000100,-0x000E80,-0x001C80,-0x002980,-0x003500,-0x003F80,
    -0x004900,-0x005180,-0x005880,-0x005E80,-0x006400,-0x006800,-0x006B80,-0x006E80,-0x007000,-0x007180,-0x007200,-0x007200,-0x007180,-0x007080,-0x006F00,-0x006D00,
     0x006A80, 0x006800, 0x006500, 0x006200, 0x005F00, 0x005B80, 0x005800, 0x005480, 0x005080, 0x004D00, 0x004980, 0x004580, 0x004200, 0x003E80, 0x003A80, 0x003780,
     0x003400, 0x003080, 0x002D80, 0x002A80, 0x002780, 0x002480, 0x002200, 0x001F80, 0x001D00, 0x001A80, 0x001880, 0x001680, 0x001480, 0x001300, 0x001180, 0x000F80,
     0x000E80, 0x000D00, 0x000C00, 0x000A80, 0x000980, 0x000880, 0x000800, 0x000700, 0x000680, 0x000580, 0x000500, 0x000480, 0x000400, 0x000380, 0x000380, 0x000300,
     0x000280, 0x000280, 0x000200, 0x000200, 0x000180, 0x000180, 0x000100, 0x000100, 0x000100, 0x000100, 0x000080, 0x000080, 0x000080, 0x000080, 0x000080, 0x000080,
];


const SF_TABLE: [i64; 63] = [
    0x20000000,
    0x1965fea5,
    0x1428a2fa,
    0x10000000,
    0x0cb2ff53,
    0x0a14517d,
    0x08000000,
    0x06597fa9,
    0x050a28be,
    0x04000000,
    0x032cbfd5,
    0x0285145f,
    0x02000000,
    0x01965fea,
    0x01428a30,
    0x01000000,
    0x00cb2ff5,
    0x00a14518,
    0x00800000,
    0x006597fb,
    0x0050a28c,
    0x00400000,
    0x0032cbfd,
    0x00285146,
    0x00200000,
    0x001965ff,
    0x001428a3,
    0x00100000,
    0x000cb2ff,
    0x000a1451,
    0x00080000,
    0x00065980,
    0x00050a29,
    0x00040000,
    0x00032cc0,
    0x00028514,
    0x00020000,
    0x00019660,
    0x0001428a,
    0x00010000,
    0x0000cb30,
    0x0000a145,
    0x00008000,
    0x00006598,
    0x000050a3,
    0x00004000,
    0x000032cc,
    0x00002851,
    0x00002000,
    0x00001966,
    0x00001429,
    0x00001000,
    0x00000cb3,
    0x00000a14,
    0x00000800,
    0x00000659,
    0x0000050a,
    0x00000400,
    0x0000032d,
    0x00000285,
    0x00000200,
    0x00000196,
    0x00000143,
];

// Inverse scalefactor table
const ISF_TABLE: [i64; 63] = [
    0x00000008000000,
    0x0000000A14517C,
    0x0000000CB2FF52,
    0x00000010000000,
    0x0000001428A2F8,
    0x0000001965FEA4,
    0x00000020000000,
    0x000000285145F5,
    0x00000032CBFD4E,
    0x00000040000000,
    0x00000050A28BDD,
    0x0000006597FA9C,
    0x00000080000000,
    0x000000A14517ED,
    0x000000CB2FF4E8,
    0x00000100000000,
    0x000001428A2FDB,
    0x000001965FE9D1,
    0x00000200000000,
    0x00000285145C8A,
    0x0000032CBFD3A3,
    0x00000400000000,
    0x0000050A28C5C7,
    0x000006597FA747,
    0x00000800000000,
    0x00000A145158C2,
    0x00000CB2FF4E8E,
    0x00001000000000,
    0x00001428A37CB4,
    0x00001965FFDFA8,
    0x00002000000000,
    0x0000285143CCA8,
    0x000032CBFAB527,
    0x00004000000000,
    0x000050A2879951,
    0x000065980992F3,
    0x00008000000000,
    0x0000A1450F32A2,
    0x0000CB301325E7,
    0x00010000000000,
    0x0001428A1E6544,
    0x00019660264BCF,
    0x00020000000000,
    0x000285143CCA88,
    0x00032CBB427564,
    0x00040000000000,
    0x00050A28799510,
    0x0006598AAD93B4,
    0x00080000000000,
    0x000A1450F32A20,
    0x000CB2C4B983B2,
    0x00100000000000,
    0x001428A1E65441,
    0x001966CC01966C,
    0x00200000000000,
    0x00285470CC2B7B,
    0x0032CD98032CD9,
    0x00400000000000,
    0x00509C2E9A4AF1,
    0x00659B300659B3,
    0x00800000000000,
    0x00A16B312EA8FC,
    0x00CAE5D85F1BBD,
];

#[derive(Clone,Copy,Debug)]
struct GroupSpec {
    nlevels: i32,
    group_bits: u32,
}

#[derive(Clone,Copy,Debug)]
struct QuantSpec {
    a: i64,
    b: i64,
    num_bits: u32,
    group_spec: Option<GroupSpec>,
}

const QUANT_TABLE: [QuantSpec; 30] = [
    QuantSpec{ a: 0x0F800000, b: -0x00800000, num_bits: 5, group_spec: None, },
    QuantSpec{ a: 0x0F800000, b: -0x00800000, num_bits: 5, group_spec: None, },
    QuantSpec{ a: 0x0F800000, b: -0x00800000, num_bits: 5, group_spec: None, },
    QuantSpec{ a: 0x0F800000, b: -0x00800000, num_bits: 5, group_spec: None, },
    QuantSpec{ a: 0x0F000000, b: -0x01000000, num_bits: 4, group_spec: None, },
    QuantSpec{ a: 0x0F000000, b: -0x01000000, num_bits: 4, group_spec: None, },
    QuantSpec{ a: 0x09000000, b: -0x07000000, num_bits: 4, group_spec: Some(GroupSpec{ nlevels: 9, group_bits: 10, }), },
    QuantSpec{ a: 0x09000000, b: -0x07000000, num_bits: 4, group_spec: Some(GroupSpec{ nlevels: 9, group_bits: 10, }), },
    QuantSpec{ a: 0x09000000, b: -0x07000000, num_bits: 4, group_spec: Some(GroupSpec{ nlevels: 9, group_bits: 10, }), },
    QuantSpec{ a: 0x09000000, b: -0x07000000, num_bits: 4, group_spec: Some(GroupSpec{ nlevels: 9, group_bits: 10, }), },
    QuantSpec{ a: 0x09000000, b: -0x07000000, num_bits: 4, group_spec: Some(GroupSpec{ nlevels: 9, group_bits: 10, }), },
    QuantSpec{ a: 0x09000000, b: -0x07000000, num_bits: 4, group_spec: Some(GroupSpec{ nlevels: 9, group_bits: 10, }), },
    QuantSpec{ a: 0x0C000000, b: -0x04000000, num_bits: 2, group_spec: Some(GroupSpec{ nlevels: 3, group_bits: 5, }), },
    QuantSpec{ a: 0x0C000000, b: -0x04000000, num_bits: 2, group_spec: Some(GroupSpec{ nlevels: 3, group_bits: 5, }), },
    QuantSpec{ a: 0x0C000000, b: -0x04000000, num_bits: 2, group_spec: Some(GroupSpec{ nlevels: 3, group_bits: 5, }), },
    QuantSpec{ a: 0x0C000000, b: -0x04000000, num_bits: 2, group_spec: Some(GroupSpec{ nlevels: 3, group_bits: 5, }), },
    QuantSpec{ a: 0x0C000000, b: -0x04000000, num_bits: 2, group_spec: Some(GroupSpec{ nlevels: 3, group_bits: 5, }), },
    QuantSpec{ a: 0x0C000000, b: -0x04000000, num_bits: 2, group_spec: Some(GroupSpec{ nlevels: 3, group_bits: 5, }), },
    QuantSpec{ a: 0x0C000000, b: -0x04000000, num_bits: 2, group_spec: Some(GroupSpec{ nlevels: 3, group_bits: 5, }), },
    QuantSpec{ a: 0x0C000000, b: -0x04000000, num_bits: 2, group_spec: Some(GroupSpec{ nlevels: 3, group_bits: 5, }), },
    QuantSpec{ a: 0x0C000000, b: -0x04000000, num_bits: 2, group_spec: Some(GroupSpec{ nlevels: 3, group_bits: 5, }), },
    QuantSpec{ a: 0x0C000000, b: -0x04000000, num_bits: 2, group_spec: Some(GroupSpec{ nlevels: 3, group_bits: 5, }), },
    QuantSpec{ a: 0x0C000000, b: -0x04000000, num_bits: 2, group_spec: Some(GroupSpec{ nlevels: 3, group_bits: 5, }), },
    QuantSpec{ a: 0x0C000000, b: -0x04000000, num_bits: 2, group_spec: Some(GroupSpec{ nlevels: 3, group_bits: 5, }), },
    QuantSpec{ a: 0x0C000000, b: -0x04000000, num_bits: 2, group_spec: Some(GroupSpec{ nlevels: 3, group_bits: 5, }), },
    QuantSpec{ a: 0x0C000000, b: -0x04000000, num_bits: 2, group_spec: Some(GroupSpec{ nlevels: 3, group_bits: 5, }), },
    QuantSpec{ a: 0x0C000000, b: -0x04000000, num_bits: 2, group_spec: Some(GroupSpec{ nlevels: 3, group_bits: 5, }), },
    QuantSpec{ a: 0x0C000000, b: -0x04000000, num_bits: 2, group_spec: Some(GroupSpec{ nlevels: 3, group_bits: 5, }), },
    QuantSpec{ a: 0x0C000000, b: -0x04000000, num_bits: 2, group_spec: Some(GroupSpec{ nlevels: 3, group_bits: 5, }), },
    QuantSpec{ a: 0x0C000000, b: -0x04000000, num_bits: 2, group_spec: Some(GroupSpec{ nlevels: 3, group_bits: 5, }), },
];

struct Window {
    window: [i16; 512],
    window_idx: usize,
}

impl Window {
    fn new() -> Window {
        Window {
            window: [0; 512],
            window_idx: 0,
        }
    }

    fn add_samples(&mut self, samples: &[i16]) {
        for idx in 0..32 {
            self.window[self.window_idx + idx] = samples[idx];
        }

        self.window_idx += 32;
        self.window_idx %= 512;
    }

    fn polyphase(&self) -> [i64; 32] {
        let mut polyphased = [0; 32];

        // Precompute Y since it doesn't rely on subband
        let mut y = [0; 64];
        for i in 0..64 {
            for j in 0..8 {
                // Window the sample
                // (15b * 28b) >> 15 = 28b
                y[i] += (self[i + 64 * j] as i64 * ENWINDOW[i + 64 * j]) >> 15;
            }
        }

        // Now do polyphase filter
        for sb in 0..32 {
            for i in 0..64 {
                polyphased[sb] += (N[i][sb] * y[i]) >> 28;
            }
        }

        polyphased
    }
}

impl Index<usize> for Window {
    type Output = i16;

    fn index(&self, index: usize) -> &Self::Output {
        &self.window[(self.window_idx + index) % 512]
    }
}

struct BitWriter<W> {
    inner: W,
    byte: u8,
    bit: u32,
}

impl<W> BitWriter<W>
    where W: Write
{
    fn new(inner: W) -> BitWriter<W> {
        BitWriter {
            inner: inner,
            byte: 0,
            bit: 0,
        }
    }

    fn reset(&mut self) {
        if self.bit != 0 {
            self.bit = 8;
        }
    }

    fn write_bit(&mut self, bit: u32) -> RadxResult<()> {
        if self.bit == 8 {
            let buf = [self.byte];
            self.inner.write_all(&buf)?;
            self.byte = 0;
            self.bit = 0;
        }
        self.byte |= (bit as u8) << (7 - self.bit);
        self.bit += 1;

        Ok(())
    }

    fn write(&mut self, num: u32, mut bits: u32) -> RadxResult<()> {
        while bits > 0 {
            bits -= 1;
            self.write_bit((num >> bits) & 1)?;
        }

        Ok(())
    }

    fn inner(mut self) -> RadxResult<W> {
        if self.bit != 0 {
            let buf = [self.byte];
            self.inner.write_all(&buf)?;
        }
        Ok(self.inner)
    }
}

pub struct AhxEncoder<S> {
    inner: BitWriter<S>,
    window: Window,
    samples_encoded: u32,
    buffer: [i16; 1152],
    buffer_idx: usize,
}

impl<S> AhxEncoder<S>
    where S: Write + Seek
{
    pub fn new(mut inner: S) -> RadxResult<AhxEncoder<S>> {
        inner.seek(SeekFrom::Start(0x24))?;
        Ok(AhxEncoder {
            inner: BitWriter::new(inner),
            window: Window::new(),
            samples_encoded: 0,
            buffer: [0; 1152],
            buffer_idx: 0,
        })
    }

    fn encode_frame(&mut self) -> RadxResult<()> {
        self.inner.reset();

        // Write frame header
        self.inner.write(0xFFF5E0C0, 32)?;

        // Write bit allocations
        for _ in 0..4 {
            self.inner.write(6, 4)?;
        }
        for _ in 0..2 {
            self.inner.write(4, 3)?;
        }
        for _ in 0..5 {
            self.inner.write(3, 3)?;
        }
        self.inner.write(3, 2)?;
        for _ in 0..18 {
            self.inner.write(1, 2)?;
        }

        // 1 scfsi per subband
        let mut scfsi = [0; 30];

        // 3 parts with 30 subbands
        let mut scalefactors = [[0; 30]; 3];

        // 3 parts with 4 granules with 32 subbands with 3 samples
        let mut polyphased_samples = [[[[0; 3]; 32]; 4]; 3];

        let mut sample_idx = 0;
        for part in 0..3 {
            // Read in samples
            for gr in 0..4 {
                for s in 0..3 {
                    self.window.add_samples(&self.buffer[sample_idx..sample_idx + 32]);
                    let polyphased = self.window.polyphase();
                    sample_idx += 32;

                    // Would be better if there was a nicer way to put these samples in place
                    for sb in 0..32 {
                        polyphased_samples[part][gr][sb][s] = polyphased[sb];
                    }
                }
            }

            // Analyze samples for scalefactors
            for sb in 0..30 {
                let mut max_sample = 0;
                for gr in 0..4 {
                    for s in 0..3 {
                        if polyphased_samples[part][gr][sb][s].abs() > max_sample {
                            max_sample = polyphased_samples[part][gr][sb][s].abs();
                        }
                    }
                }

                // Find best scalefactor
                let mut sf_index = 0;
                for i in 0..63 {
                    sf_index = 62 - i;
                    if max_sample < SF_TABLE[sf_index] {
                        break;
                    }
                }

                scalefactors[part][sb] = sf_index;
            }
        }

        // Analyze scfsi info
        for sb in 0..30 {
            if scalefactors[0][sb] == scalefactors[1][sb] {
                if scalefactors[1][sb] == scalefactors[2][sb] {
                    // All scalefactors the same
                    scfsi[sb] = 2;
                }
                else {
                    // First two same, last different
                    scfsi[sb] = 1;
                }
            }
            else {
                if scalefactors[1][sb] == scalefactors[2][sb] {
                    // Last two same, first different
                    scfsi[sb] = 3;
                }
                else {
                    // None same
                    scfsi[sb] = 0;
                }
            }
        }

        // Write scfsi information
        for sb in 0..30 {
            self.inner.write(scfsi[sb], 2)?;
        }

        // Write scalefactor information
        for sb in 0..30 {
            match scfsi[sb] {
                0 => {
                    // None the same, write all three scalefactors
                    self.inner.write(scalefactors[0][sb] as u32, 6)?;
                    self.inner.write(scalefactors[1][sb] as u32, 6)?;
                    self.inner.write(scalefactors[2][sb] as u32, 6)?;
                }
                1 => {
                    // First two same, last one different, write first and last
                    self.inner.write(scalefactors[0][sb] as u32, 6)?;
                    self.inner.write(scalefactors[2][sb] as u32, 6)?;
                }
                2 => {
                    // All scalefactors the same, write the first
                    self.inner.write(scalefactors[0][sb] as u32, 6)?;
                }
                3 => {
                    // Last two the same, first one different, write first and last
                    self.inner.write(scalefactors[0][sb] as u32, 6)?;
                    self.inner.write(scalefactors[2][sb] as u32, 6)?;
                }
                _ => unreachable!(),
            }
        }

        // Write sample data
        for part in 0..3 {
            for gr in 0..4 {
                for sb in 0..30 {
                    let quant = QUANT_TABLE[sb];

                    let mut quantized_samples = [0; 3];
                    for s in 0..3 {
                        let scaled = (polyphased_samples[part][gr][sb][s] * ISF_TABLE[scalefactors[part][sb]]) >> 28;
                        let transformed = ((scaled * quant.a) >> 28) + quant.b;
                        let quantized = transformed >> (28 - (quant.num_bits - 1));
                        let formatted = (quantized & ((1 << quant.num_bits) - 1)) ^ (1 << (quant.num_bits - 1));
                        quantized_samples[s] = formatted;
                    }

                    if let Some(group_spec) = quant.group_spec {
                        let grouped = quantized_samples[0] +
                            quantized_samples[1] * group_spec.nlevels as i64 +
                            quantized_samples[2] * group_spec.nlevels as i64 * group_spec.nlevels as i64;
                        self.inner.write(grouped as u32, group_spec.group_bits)?;
                    }
                    else {
                        for s in 0..3 {
                            self.inner.write(quantized_samples[s] as u32, quant.num_bits)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn encode_data<I>(&mut self, samples: I) -> RadxResult<()>
        where I: IntoIterator<Item = i16>
    {
        for sample in samples {
            self.buffer[self.buffer_idx] = sample;
            self.buffer_idx += 1;

            if self.buffer_idx == 1152 {
                self.encode_frame()?;
                self.buffer_idx = 0;
            }
            self.samples_encoded += 1;
        }

        Ok(())
    }

    pub fn finalize(mut self) -> RadxResult<()> {
        if self.buffer_idx != 0 {
            for idx in self.buffer_idx..1152 {
                self.buffer[idx] = 0;
            }
            self.encode_frame()?;
        }

        let mut inner = self.inner.inner()?;
        inner.write_all(b"\x00\x80\x01\x00\x0cAHXE(c)CRI\x00\x00")?;

        let header = AdxHeader {
            encoding: AdxEncoding::Ahx,
            block_size: 0,
            sample_bitdepth: 0,
            channel_count: 1,
            sample_rate: 22050,
            total_samples: self.samples_encoded,
            highpass_frequency: 0,
            version: AdxVersion::Version6,
            flags: 0,
        };
        inner.seek(SeekFrom::Start(0))?;
        header.to_writer(inner, 0x24)?;

        Ok(())
    }
}
