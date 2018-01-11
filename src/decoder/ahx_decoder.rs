use adx_header::AdxHeader;
use decoder::Decoder;
use ::{Sample, LoopInfo};

use std::cmp;
use std::i16;
use std::io::{self, Read};
use std::num::Wrapping;

const FRAC_BITS: u32 = 28;

const BIT_ALLOC_TABLE: [u32; 30] = [
    4, 4, 4, 4,
    3, 3, 3, 3, 3, 3, 3,
    2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2
];

#[derive(Clone,Copy,Debug)]
struct QuantizeSpec {
    nlevels: i64,
    group: u32,
    bits: u32,
    c: i64,
    d: i64,
}

const QUANT_TABLE_LOW: [QuantizeSpec; 16] = [
    QuantizeSpec{nlevels: 3,     group: 2, bits:5,  c: 0x15555555, d: 0x08000000},
    QuantizeSpec{nlevels: 5,     group: 4, bits:7,  c: 0x1999999a, d: 0x08000000},
    QuantizeSpec{nlevels: 7,     group: 0, bits:3,  c: 0x12492492, d: 0x04000000},
    QuantizeSpec{nlevels: 9,     group: 4, bits:10, c: 0x1c71c71c, d: 0x08000000},
    QuantizeSpec{nlevels: 15,    group: 0, bits:4,  c: 0x11111111, d: 0x02000000},
    QuantizeSpec{nlevels: 31,    group: 0, bits:5,  c: 0x10842108, d: 0x01000000},
    QuantizeSpec{nlevels: 63,    group: 0, bits:6,  c: 0x10410410, d: 0x00800000},
    QuantizeSpec{nlevels: 127,   group: 0, bits:7,  c: 0x10204081, d: 0x00400000},
    QuantizeSpec{nlevels: 255,   group: 0, bits:8,  c: 0x10101010, d: 0x00200000},
    QuantizeSpec{nlevels: 511,   group: 0, bits:9,  c: 0x10080402, d: 0x00100000},
    QuantizeSpec{nlevels: 1023,  group: 0, bits:10, c: 0x10040100, d: 0x00080000},
    QuantizeSpec{nlevels: 2047,  group: 0, bits:11, c: 0x10020040, d: 0x00040000},
    QuantizeSpec{nlevels: 4095,  group: 0, bits:12, c: 0x10010010, d: 0x00020000},
    QuantizeSpec{nlevels: 8191,  group: 0, bits:13, c: 0x10008004, d: 0x00010000},
    QuantizeSpec{nlevels: 16383, group: 0, bits:14, c: 0x10004001, d: 0x00008000},
    QuantizeSpec{nlevels: 32767, group: 0, bits:15, c: 0x10002000, d: 0x00004000},
];

const QUANT_TABLE_HIGH: [QuantizeSpec; 16] = [
    QuantizeSpec{nlevels: 3,     group: 2, bits: 5,  c: 0x15555555, d: 0x08000000},
    QuantizeSpec{nlevels: 5,     group: 4, bits: 7,  c: 0x1999999a, d: 0x08000000},
    QuantizeSpec{nlevels: 9,     group: 4, bits: 10, c: 0x1c71c71c, d: 0x08000000},
    QuantizeSpec{nlevels: 15,    group: 0, bits: 4,  c: 0x11111111, d: 0x02000000},
    QuantizeSpec{nlevels: 31,    group: 0, bits: 5,  c: 0x10842108, d: 0x01000000},
    QuantizeSpec{nlevels: 63,    group: 0, bits: 6,  c: 0x10410410, d: 0x00800000},
    QuantizeSpec{nlevels: 127,   group: 0, bits: 7,  c: 0x10204081, d: 0x00400000},
    QuantizeSpec{nlevels: 255,   group: 0, bits: 8,  c: 0x10101010, d: 0x00200000},
    QuantizeSpec{nlevels: 511,   group: 0, bits: 9,  c: 0x10080402, d: 0x00100000},
    QuantizeSpec{nlevels: 1023,  group: 0, bits: 10, c: 0x10040100, d: 0x00080000},
    QuantizeSpec{nlevels: 2047,  group: 0, bits: 11, c: 0x10020040, d: 0x00040000},
    QuantizeSpec{nlevels: 4095,  group: 0, bits: 12, c: 0x10010010, d: 0x00020000},
    QuantizeSpec{nlevels: 8191,  group: 0, bits: 13, c: 0x10008004, d: 0x00010000},
    QuantizeSpec{nlevels: 16383, group: 0, bits: 14, c: 0x10004001, d: 0x00008000},
    QuantizeSpec{nlevels: 32767, group: 0, bits: 15, c: 0x10002000, d: 0x00004000},
    QuantizeSpec{nlevels: 65535, group: 0, bits: 16, c: 0x10001000, d: 0x00002000},
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

const D: [i64; 512] = [
     0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,-0x00001000,
    -0x00001000,-0x00001000,-0x00001000,-0x00002000,-0x00002000,-0x00003000,-0x00003000,-0x00004000,
    -0x00004000,-0x00005000,-0x00006000,-0x00006000,-0x00007000,-0x00008000,-0x00009000,-0x0000A000,
    -0x0000C000,-0x0000D000,-0x0000F000,-0x00010000,-0x00012000,-0x00014000,-0x00017000,-0x00019000,
    -0x0001C000,-0x0001E000,-0x00022000,-0x00025000,-0x00028000,-0x0002C000,-0x00030000,-0x00034000,
    -0x00039000,-0x0003E000,-0x00043000,-0x00048000,-0x0004E000,-0x00054000,-0x0005A000,-0x00060000,
    -0x00067000,-0x0006E000,-0x00074000,-0x0007C000,-0x00083000,-0x0008A000,-0x00092000,-0x00099000,
    -0x000A0000,-0x000A8000,-0x000AF000,-0x000B6000,-0x000BD000,-0x000C3000,-0x000C9000,-0x000CF000,
     0x000D5000, 0x000DA000, 0x000DE000, 0x000E1000, 0x000E3000, 0x000E4000, 0x000E4000, 0x000E3000,
     0x000E0000, 0x000DD000, 0x000D7000, 0x000D0000, 0x000C8000, 0x000BD000, 0x000B1000, 0x000A3000,
     0x00092000, 0x0007F000, 0x0006A000, 0x00053000, 0x00039000, 0x0001D000,-0x00001000,-0x00023000,
    -0x00047000,-0x0006E000,-0x00098000,-0x000C4000,-0x000F3000,-0x00125000,-0x0015A000,-0x00190000,
    -0x001CA000,-0x00206000,-0x00244000,-0x00284000,-0x002C6000,-0x0030A000,-0x0034F000,-0x00396000,
    -0x003DE000,-0x00427000,-0x00470000,-0x004B9000,-0x00502000,-0x0054B000,-0x00593000,-0x005D9000,
    -0x0061E000,-0x00661000,-0x006A1000,-0x006DE000,-0x00718000,-0x0074D000,-0x0077E000,-0x007A9000,
    -0x007D0000,-0x007EF000,-0x00808000,-0x0081A000,-0x00824000,-0x00826000,-0x0081F000,-0x0080E000,
     0x007F5000, 0x007D0000, 0x007A0000, 0x00765000, 0x0071E000, 0x006CB000, 0x0066C000, 0x005FF000,
     0x00586000, 0x00500000, 0x0046B000, 0x003CA000, 0x0031A000, 0x0025D000, 0x00192000, 0x000B9000,
    -0x0002C000,-0x0011F000,-0x00220000,-0x0032D000,-0x00446000,-0x0056B000,-0x0069B000,-0x007D5000,
    -0x00919000,-0x00A66000,-0x00BBB000,-0x00D16000,-0x00E78000,-0x00FDE000,-0x01148000,-0x012B3000,
    -0x01420000,-0x0158C000,-0x016F6000,-0x0185C000,-0x019BC000,-0x01B16000,-0x01C66000,-0x01DAC000,
    -0x01EE5000,-0x02010000,-0x0212A000,-0x02232000,-0x02325000,-0x02402000,-0x024C7000,-0x02570000,
    -0x025FE000,-0x0266D000,-0x026BB000,-0x026E6000,-0x026ED000,-0x026CE000,-0x02686000,-0x02615000,
    -0x02577000,-0x024AC000,-0x023B2000,-0x02287000,-0x0212B000,-0x01F9B000,-0x01DD7000,-0x01BDD000,
     0x019AE000, 0x01747000, 0x014A8000, 0x011D1000, 0x00EC0000, 0x00B77000, 0x007F5000, 0x0043A000,
     0x00046000,-0x003E5000,-0x00849000,-0x00CE3000,-0x011B4000,-0x016B9000,-0x01BF1000,-0x0215B000,
    -0x026F6000,-0x02CBE000,-0x032B3000,-0x038D3000,-0x03F1A000,-0x04586000,-0x04C15000,-0x052C4000,
    -0x05990000,-0x06075000,-0x06771000,-0x06E80000,-0x0759F000,-0x07CCA000,-0x083FE000,-0x08B37000,
    -0x09270000,-0x099A7000,-0x0A0D7000,-0x0A7FD000,-0x0AF14000,-0x0B618000,-0x0BD05000,-0x0C3D8000,
    -0x0CA8C000,-0x0D11D000,-0x0D789000,-0x0DDC9000,-0x0E3DC000,-0x0E9BD000,-0x0EF68000,-0x0F4DB000,
    -0x0FA12000,-0x0FF09000,-0x103BD000,-0x1082C000,-0x10C53000,-0x1102E000,-0x113BD000,-0x116FB000,
    -0x119E8000,-0x11C82000,-0x11EC6000,-0x120B3000,-0x12248000,-0x12385000,-0x12467000,-0x124EF000,
     0x1251E000, 0x124F0000, 0x12468000, 0x12386000, 0x12249000, 0x120B4000, 0x11EC7000, 0x11C83000,
     0x119E9000, 0x116FC000, 0x113BE000, 0x1102F000, 0x10C54000, 0x1082D000, 0x103BE000, 0x0FF0A000,
     0x0FA13000, 0x0F4DC000, 0x0EF69000, 0x0E9BE000, 0x0E3DD000, 0x0DDCA000, 0x0D78A000, 0x0D11E000,
     0x0CA8D000, 0x0C3D9000, 0x0BD06000, 0x0B619000, 0x0AF15000, 0x0A7FE000, 0x0A0D8000, 0x099A8000,
     0x09271000, 0x08B38000, 0x083FF000, 0x07CCB000, 0x075A0000, 0x06E81000, 0x06772000, 0x06076000,
     0x05991000, 0x052C5000, 0x04C16000, 0x04587000, 0x03F1B000, 0x038D4000, 0x032B4000, 0x02CBF000,
     0x026F7000, 0x0215C000, 0x01BF2000, 0x016BA000, 0x011B5000, 0x00CE4000, 0x0084A000, 0x003E6000,
    -0x00045000,-0x00439000,-0x007F4000,-0x00B76000,-0x00EBF000,-0x011D0000,-0x014A7000,-0x01746000,
     0x019AE000, 0x01BDE000, 0x01DD8000, 0x01F9C000, 0x0212C000, 0x02288000, 0x023B3000, 0x024AD000,
     0x02578000, 0x02616000, 0x02687000, 0x026CF000, 0x026EE000, 0x026E7000, 0x026BC000, 0x0266E000,
     0x025FF000, 0x02571000, 0x024C8000, 0x02403000, 0x02326000, 0x02233000, 0x0212B000, 0x02011000,
     0x01EE6000, 0x01DAD000, 0x01C67000, 0x01B17000, 0x019BD000, 0x0185D000, 0x016F7000, 0x0158D000,
     0x01421000, 0x012B4000, 0x01149000, 0x00FDF000, 0x00E79000, 0x00D17000, 0x00BBC000, 0x00A67000,
     0x0091A000, 0x007D6000, 0x0069C000, 0x0056C000, 0x00447000, 0x0032E000, 0x00221000, 0x00120000,
     0x0002D000,-0x000B8000,-0x00191000,-0x0025C000,-0x00319000,-0x003C9000,-0x0046A000,-0x004FF000,
    -0x00585000,-0x005FE000,-0x0066B000,-0x006CA000,-0x0071D000,-0x00764000,-0x0079F000,-0x007CF000,
     0x007F5000, 0x0080F000, 0x00820000, 0x00827000, 0x00825000, 0x0081B000, 0x00809000, 0x007F0000,
     0x007D1000, 0x007AA000, 0x0077F000, 0x0074E000, 0x00719000, 0x006DF000, 0x006A2000, 0x00662000,
     0x0061F000, 0x005DA000, 0x00594000, 0x0054C000, 0x00503000, 0x004BA000, 0x00471000, 0x00428000,
     0x003DF000, 0x00397000, 0x00350000, 0x0030B000, 0x002C7000, 0x00285000, 0x00245000, 0x00207000,
     0x001CB000, 0x00191000, 0x0015B000, 0x00126000, 0x000F4000, 0x000C5000, 0x00099000, 0x0006F000,
     0x00048000, 0x00024000, 0x00002000,-0x0001C000,-0x00038000,-0x00052000,-0x00069000,-0x0007E000,
    -0x00091000,-0x000A2000,-0x000B0000,-0x000BC000,-0x000C7000,-0x000CF000,-0x000D6000,-0x000DC000,
    -0x000DF000,-0x000E2000,-0x000E3000,-0x000E3000,-0x000E2000,-0x000E0000,-0x000DD000,-0x000D9000,
     0x000D5000, 0x000D0000, 0x000CA000, 0x000C4000, 0x000BE000, 0x000B7000, 0x000B0000, 0x000A9000,
     0x000A1000, 0x0009A000, 0x00093000, 0x0008B000, 0x00084000, 0x0007D000, 0x00075000, 0x0006F000,
     0x00068000, 0x00061000, 0x0005B000, 0x00055000, 0x0004F000, 0x00049000, 0x00044000, 0x0003F000,
     0x0003A000, 0x00035000, 0x00031000, 0x0002D000, 0x00029000, 0x00026000, 0x00023000, 0x0001F000,
     0x0001D000, 0x0001A000, 0x00018000, 0x00015000, 0x00013000, 0x00011000, 0x00010000, 0x0000E000,
     0x0000D000, 0x0000B000, 0x0000A000, 0x00009000, 0x00008000, 0x00007000, 0x00007000, 0x00006000,
     0x00005000, 0x00005000, 0x00004000, 0x00004000, 0x00003000, 0x00003000, 0x00002000, 0x00002000,
     0x00002000, 0x00002000, 0x00001000, 0x00001000, 0x00001000, 0x00001000, 0x00001000, 0x00001000,
];

pub struct AhxDecoder<R> {
    inner: BitReader<R>,
    header: AdxHeader,
    v_off: usize,
    u: [i64; 512],
    v: [i64; 1024],
    buffer: [i16; 1152],
    buffer_idx: usize,
}

impl<R> AhxDecoder<R>
    where R: Read
{
    pub fn from_header(header: AdxHeader, inner: R) -> AhxDecoder<R> {
        AhxDecoder {
            inner: BitReader::new(inner),
            header: header,
            v_off: 0,
            u: [0; 512],
            v: [0; 1024],
            buffer: [0; 1152],
            buffer_idx: 1152,
        }
    }

    fn read_frame(&mut self) -> io::Result<Option<[i16; 1152]>> {
        self.inner.reset();
        // let _sync = self.inner.read(11)?;
        // let _version = self.inner.read(2)?;
        // let _layer = self.inner.read(2)?;
        // let _protection = self.inner.read(1)?;
        // let _bitrate = self.inner.read(4)?;
        // let _sampling = self.inner.read(2)?;
        // let _padding = self.inner.read(1)?;
        // let _private = self.inner.read(1)?;
        // let _channel = self.inner.read(2)?;
        // let _mode = self.inner.read(2)?;
        // let _copyright = self.inner.read(1)?;
        // let _original = self.inner.read(1)?;
        // let _emphasis = self.inner.read(2)?;
        let frame_header = self.inner.read(32)?;

        if frame_header == 0x00800100 {
            return Ok(None);
        }
        else if frame_header != 0xfff5e0c0 {
            return Err(io::Error::new(io::ErrorKind::Other, "Incorrect file format."));
        }

        let mut allocations = [0; 30];
        for sb in 0..30 {
            allocations[sb] = self.inner.read(BIT_ALLOC_TABLE[sb])?;
        }

        let mut scfsi = [0; 30];
        for sb in 0..30 {
            scfsi[sb] = self.inner.read(2)?;
        }

        let mut scalefactors = [[0; 3]; 30];
        for sb in 0..30 {
            if allocations[sb] != 0 {
                match scfsi[sb] {
                    0 => {
                        scalefactors[sb][0] = self.inner.read(6)?;
                        scalefactors[sb][1] = self.inner.read(6)?;
                        scalefactors[sb][2] = self.inner.read(6)?;
                    }
                    1 => {
                        let tmp = self.inner.read(6)?;
                        scalefactors[sb][0] = tmp;
                        scalefactors[sb][1] = tmp;
                        scalefactors[sb][2] = self.inner.read(6)?;
                    }
                    2 => {
                        let tmp = self.inner.read(6)?;
                        scalefactors[sb][0] = tmp;
                        scalefactors[sb][1] = tmp;
                        scalefactors[sb][2] = tmp;
                    }
                    3 => {
                        scalefactors[sb][0] = self.inner.read(6)?;
                        let tmp = self.inner.read(6)?;
                        scalefactors[sb][1] = tmp;
                        scalefactors[sb][2] = tmp;
                    }
                    _ => unreachable!(),
                }
            }
        }

        let mut pcm = [0; 1152];

        for part in 0..3 {
            for gr in 0..4 {
                let mut sb_samples = [[0; 3]; 32];

                for sb in 0..30 {
                    if allocations[sb] != 0 {
                        let quant;
                        if sb < 4 {
                            quant = QUANT_TABLE_LOW[allocations[sb] as usize - 1];
                        }
                        else {
                            quant = QUANT_TABLE_HIGH[allocations[sb] as usize - 1];
                        }

                        let samples = self.read_samples(quant)?;

                        for idx in 0..3 {
                            sb_samples[sb][idx] = (Wrapping(samples[idx]) * Wrapping(SF_TABLE[scalefactors[sb][part] as usize])).0 >> FRAC_BITS;
                        }
                    }
                }

                // Synthesis
                for idx in 0..3 {
                    let table_idx = (Wrapping(self.v_off) - Wrapping(64)).0 % 1024;
                    self.v_off = table_idx;

                    // Matrixing
                    for i in 0..64 {
                        let mut sum = 0;
                        for j in 0..32 {
                            sum += (Wrapping(N[i][j]) * Wrapping(sb_samples[j][idx])).0 >> FRAC_BITS;
                        }

                        self.v[table_idx + i] = sum;

                        for i in 0..8 {
                            for sb in 0..32 {
                                self.u[(i * 64) + sb] = self.v[(table_idx + (i * 128) + sb) % 1024];
                                self.u[(i * 64) + sb + 32] = self.v[(table_idx + (i * 128) + sb + 96) % 1024];
                            }
                        }

                        for i in 0..512 {
                            self.u[i] = (Wrapping(self.u[i]) * Wrapping(D[i])).0 >> FRAC_BITS;
                        }

                        for sb in 0..32 {
                            let mut sum = 0;
                            for i in 0..16 {
                                sum -= self.u[i * 32 + sb];
                            }

                            sum >>= FRAC_BITS - 15;

                            if sum > i16::MAX as i64 {
                                sum = i16::MAX as i64;
                            }
                            else if sum < i16::MIN as i64 {
                                sum = i16::MIN as i64;
                            }

                            pcm[part * 384 + gr * 96 + idx * 32 + sb] = sum as i16;
                        }
                    }
                }
            }
        }

        Ok(Some(pcm))
    }

    fn read_samples(&mut self, quant: QuantizeSpec) -> io::Result<[i64; 3]> {
        let mut samples = [0; 3];
        let num_bits;

        if quant.group != 0 {
            num_bits = quant.group;
            let mut grouped = self.inner.read(quant.bits)? as i64;

            for idx in 0..3 {
                samples[idx] = grouped % quant.nlevels;
                grouped /= quant.nlevels;
            }
        }
        else {
            num_bits = quant.bits;

            for idx in 0..3 {
                samples[idx] = self.inner.read(num_bits)? as i64;
            }
        }

        for idx in 0..3 {
            let mut requantized = samples[idx] ^ (1 << (num_bits - 1));
            requantized |= -(requantized & (1 << (num_bits - 1)));

            requantized <<= FRAC_BITS - (num_bits - 1);

            samples[idx] = (Wrapping(requantized + quant.d) * Wrapping(quant.c)).0 >> FRAC_BITS;
        }

        Ok(samples)
    }
}

impl<R> Decoder for AhxDecoder<R>
    where R: Read
{
    fn channels(&self) -> u32 {
        // Always mono
        1
    }

    fn sample_rate(&self) -> u32 {
        22050
    }

	fn loop_info(&self) -> Option<LoopInfo> {
        None
    }

    fn next_sample(&mut self) -> Option<Sample> {
        if self.buffer_idx == 1152 {
            if let Some(pcm) = self.read_frame().ok().and_then(|s| s) {
                self.buffer = pcm;
                self.buffer_idx = 0;
            }
            else {
                return None;
            }
        }

        let sample = self.buffer[self.buffer_idx];
        self.buffer_idx += 1;
        Some(vec![sample])
    }
}

pub struct BitReader<R> {
    inner: R,
    buffer: u8,
    bits_left: u32,
}

impl<R> BitReader<R>
    where R: Read
{
    pub fn new(inner: R) -> BitReader<R> {
        BitReader {
            inner: inner,
            buffer: 0,
            bits_left: 0,
        }
    }

    pub fn reset(&mut self) {
        self.bits_left = 0;
    }

    fn read_from_buffer(&mut self, bits: u32) -> u32 {
        assert!(bits <= 8);

        let result = self.buffer >> (8 - bits);
        self.buffer = self.buffer.checked_shl(bits).unwrap_or(0);
        self.bits_left -= bits;
        result as u32
    }

    pub fn read(&mut self, mut bits: u32) -> io::Result<u32> {
        assert!(bits <= 32);

        let mut result = 0;

        while bits != 0 {
            if self.bits_left == 0 {
                let mut buf_array = [0];
                self.inner.read_exact(&mut buf_array)?;
                self.buffer = buf_array[0];
                self.bits_left = 8;
            }
            let bits_to_read = cmp::min(bits, self.bits_left);
            let data = self.read_from_buffer(bits_to_read);
            result = (result << bits_to_read) | data;
            bits -= bits_to_read;
        }

        Ok(result)
    }
}
