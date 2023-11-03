#![forbid(unsafe_code)]

use std::{collections::HashMap, convert::TryFrom, io::BufRead, ops::AddAssign};

use anyhow::{bail, Result};

use crate::bit_reader::{BitReader, BitSequence};
use crate::huffman_coding::LitLenToken::{EndOfBlock, Length, Literal};
use crate::huffman_coding::TreeCodeToken::{CopyPrev, RepeatZero};

////////////////////////////////////////////////////////////////////////////////

fn litlen_static_code_lengths() -> Vec<u8> {
    [vec![8u8; 144], vec![9u8; 112], vec![7u8; 24], vec![8u8; 8]].concat()
}

const DIST_STATIC_CODE_LENGTHS: &[u8] = &[5u8; 32];

pub fn static_litlen_distance_trees(
) -> Result<(HuffmanCoding<LitLenToken>, HuffmanCoding<DistanceToken>)> {
    Ok((
        HuffmanCoding::<LitLenToken>::from_lengths(litlen_static_code_lengths().as_slice())?,
        HuffmanCoding::<DistanceToken>::from_lengths(DIST_STATIC_CODE_LENGTHS)?,
    ))
}

const LEN_CODE_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

pub fn decode_litlen_distance_trees<T: BufRead>(
    bit_reader: &mut BitReader<T>,
) -> Result<(HuffmanCoding<LitLenToken>, HuffmanCoding<DistanceToken>)> {
    // See RFC 1951, section 3.2.7.
    let hlit = bit_reader.read_bits(5)?.bits() as usize + 257;
    let hdist = bit_reader.read_bits(5)?.bits() as usize + 1;
    let hclen = bit_reader.read_bits(4)?.bits() as usize + 4;

    let mut code_lengths = [0u8; 19];
    for idx in 0..hclen {
        code_lengths[LEN_CODE_ORDER[idx]] = bit_reader.read_bits(3)?.bits() as u8;
    }

    let lengths_coding = HuffmanCoding::<TreeCodeToken>::from_lengths(&code_lengths)?;

    let mut lengths = vec![0u8; hlit + hdist];

    let mut idx = 0;
    while idx < lengths.len() {
        match lengths_coding.read_symbol(bit_reader)? {
            TreeCodeToken::Length(len) => {
                lengths[idx] = len;
                idx += 1;
            }
            CopyPrev => {
                let n_copies = bit_reader.read_bits(2)?.bits() as usize + 3;

                if n_copies > 0 && idx == 0 {
                    bail!("\"copy previous\" length token appeared as first length")
                }

                if idx + n_copies > lengths.len() {
                    bail!("\"copy previous\" would go out of range of the alphabet lengths")
                }

                let to_copy = lengths[idx - 1];
                for curr in lengths.iter_mut().skip(idx).take(n_copies) {
                    *curr = to_copy;
                }

                idx += n_copies;
            }
            RepeatZero { base, extra_bits } => {
                let n_zeros = (base + bit_reader.read_bits(extra_bits)?.bits()) as usize;

                if idx + n_zeros > lengths.len() {
                    bail!("\"repeat zero\" would go out of range of the alphabet lengths");
                }

                idx += n_zeros;
            }
        }
    }

    Ok((
        HuffmanCoding::<LitLenToken>::from_lengths(&lengths[..hlit])?,
        HuffmanCoding::<DistanceToken>::from_lengths(&lengths[hlit..])?,
    ))
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug)]
pub enum TreeCodeToken {
    Length(u8),
    CopyPrev,
    RepeatZero { base: u16, extra_bits: u8 },
}

impl TryFrom<HuffmanCodeWord> for TreeCodeToken {
    type Error = anyhow::Error;

    fn try_from(value: HuffmanCodeWord) -> Result<Self> {
        // See RFC 1951, section 3.2.7.
        match value.0 {
            0..=15 => Ok(TreeCodeToken::Length(value.0 as u8)),
            16 => Ok(CopyPrev),
            17 => Ok(RepeatZero {
                base: 3,
                extra_bits: 3,
            }),
            18 => Ok(RepeatZero {
                base: 11,
                extra_bits: 7,
            }),
            _ => bail!("invalid code for code length alphabet"),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug)]
pub enum LitLenToken {
    Literal(u8),
    EndOfBlock,
    Length { base: u16, extra_bits: u8 },
}

impl TryFrom<HuffmanCodeWord> for LitLenToken {
    type Error = anyhow::Error;

    fn try_from(value: HuffmanCodeWord) -> Result<Self> {
        // See RFC 1951, section 3.2.5.
        match value.0 {
            0..=255 => Ok(Literal(value.0 as u8)),
            256 => Ok(EndOfBlock),
            257..=264 => Ok(Length {
                base: value.0 - 254,
                extra_bits: 0,
            }),
            265..=284 => {
                let used = value.0 - 261;
                let extra_bits = used as u8 / 4;
                let n_in_block = used % 4;
                let block_step = 1 << extra_bits;
                let base_of_block = (block_step << 2) + 3;

                Ok(Length {
                    base: base_of_block + n_in_block * block_step,
                    extra_bits,
                })
            }
            285 => Ok(Length {
                base: 258,
                extra_bits: 0,
            }),
            _ => bail!("invalid code for literal/length alphabet"),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug)]
pub struct DistanceToken {
    pub base: u16,
    pub extra_bits: u8,
}

impl TryFrom<HuffmanCodeWord> for DistanceToken {
    type Error = anyhow::Error;

    fn try_from(value: HuffmanCodeWord) -> Result<Self> {
        // See RFC 1951, section 3.2.5.
        match value.0 {
            0..=3 => Ok(Self {
                base: value.0 + 1,
                extra_bits: 0,
            }),
            4..=29 => {
                let extra_bits = value.0 as u8 / 2 - 1;
                let n_in_block = value.0 % 2;
                let block_step = 1 << extra_bits;
                let base_of_block = (block_step << 1) + 1;

                Ok(Self {
                    base: base_of_block + n_in_block * block_step,
                    extra_bits,
                })
            }
            _ => bail!("invalid code for distance alphabet"),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct HuffmanCodeWord(pub u16);

pub struct HuffmanCoding<T> {
    map: HashMap<BitSequence, T>,
}

impl<T> HuffmanCoding<T>
where
    T: Copy + TryFrom<HuffmanCodeWord, Error = anyhow::Error>,
{
    pub fn new(map: HashMap<BitSequence, T>) -> Self {
        Self { map }
    }

    pub fn decode_symbol(&self, seq: BitSequence) -> Option<T> {
        self.map.get(&seq).copied()
    }

    pub fn read_symbol<U: BufRead>(&self, bit_reader: &mut BitReader<U>) -> Result<T> {
        let mut seq = BitSequence::new(0, 0);
        loop {
            seq = seq.concat(bit_reader.read_bits(1)?);
            match self.decode_symbol(seq) {
                None => continue,
                Some(value) => break Ok(value),
            }
        }
    }

    pub fn from_lengths(code_lengths: &[u8]) -> Result<Self> {
        // See RFC 1951, section 3.2.2.

        // Primary check
        if code_lengths.is_empty() {
            return Ok(Self::new(HashMap::new()));
        }

        // Step 1
        let bl_count = code_lengths.iter().filter(|&&len| len != 0).fold(
            HashMap::<u8, u16>::new(),
            |mut acc, &len| {
                acc.entry(len).or_default().add_assign(1);
                acc
            },
        );

        // Step 2
        let max_len = *code_lengths.iter().max().unwrap();
        let mut next_code = vec![0u16; max_len as usize + 1];

        let mut code = 0u16;
        for n_bits in 1..=max_len {
            code = (code + bl_count.get(&(n_bits - 1)).copied().unwrap_or(0)) << 1;
            next_code[n_bits as usize] = code;
        }

        // Step 3
        let mut map = HashMap::new();
        for (value, &length) in code_lengths.iter().enumerate() {
            if length == 0 {
                continue;
            }

            map.insert(
                BitSequence::new(next_code[length as usize], length),
                T::try_from(HuffmanCodeWord(value as u16))?,
            );
            next_code[length as usize] += 1;
        }

        Ok(Self::new(map))
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Value(u16);

    impl TryFrom<HuffmanCodeWord> for Value {
        type Error = anyhow::Error;

        fn try_from(x: HuffmanCodeWord) -> Result<Self> {
            Ok(Self(x.0))
        }
    }

    #[test]
    fn from_lengths() -> Result<()> {
        let code = HuffmanCoding::<Value>::from_lengths(&[2, 3, 4, 3, 3, 4, 2])?;

        assert_eq!(
            code.decode_symbol(BitSequence::new(0b00, 2)),
            Some(Value(0)),
        );
        assert_eq!(
            code.decode_symbol(BitSequence::new(0b100, 3)),
            Some(Value(1)),
        );
        assert_eq!(
            code.decode_symbol(BitSequence::new(0b1110, 4)),
            Some(Value(2)),
        );
        assert_eq!(
            code.decode_symbol(BitSequence::new(0b101, 3)),
            Some(Value(3)),
        );
        assert_eq!(
            code.decode_symbol(BitSequence::new(0b110, 3)),
            Some(Value(4)),
        );
        assert_eq!(
            code.decode_symbol(BitSequence::new(0b1111, 4)),
            Some(Value(5)),
        );
        assert_eq!(
            code.decode_symbol(BitSequence::new(0b01, 2)),
            Some(Value(6)),
        );

        assert_eq!(code.decode_symbol(BitSequence::new(0b0, 1)), None);
        assert_eq!(code.decode_symbol(BitSequence::new(0b10, 2)), None);
        assert_eq!(code.decode_symbol(BitSequence::new(0b111, 3)), None);

        Ok(())
    }

    #[test]
    fn read_symbol() -> Result<()> {
        let code = HuffmanCoding::<Value>::from_lengths(&[2, 3, 4, 3, 3, 4, 2])?;
        let mut data: &[u8] = &[0b10111001, 0b11001010, 0b11101101];
        let mut reader = BitReader::new(&mut data);

        assert_eq!(code.read_symbol(&mut reader)?, Value(1));
        assert_eq!(code.read_symbol(&mut reader)?, Value(2));
        assert_eq!(code.read_symbol(&mut reader)?, Value(3));
        assert_eq!(code.read_symbol(&mut reader)?, Value(6));
        assert_eq!(code.read_symbol(&mut reader)?, Value(0));
        assert_eq!(code.read_symbol(&mut reader)?, Value(2));
        assert_eq!(code.read_symbol(&mut reader)?, Value(4));
        assert!(code.read_symbol(&mut reader).is_err());

        Ok(())
    }

    #[test]
    fn from_lengths_with_zeros() -> Result<()> {
        let lengths = [3, 4, 5, 5, 0, 0, 6, 6, 4, 0, 6, 0, 7];
        let code = HuffmanCoding::<Value>::from_lengths(&lengths)?;
        let mut data: &[u8] = &[
            0b00100000, 0b00100001, 0b00010101, 0b10010101, 0b00110101, 0b00011101,
        ];
        let mut reader = BitReader::new(&mut data);

        assert_eq!(code.read_symbol(&mut reader)?, Value(0));
        assert_eq!(code.read_symbol(&mut reader)?, Value(1));
        assert_eq!(code.read_symbol(&mut reader)?, Value(2));
        assert_eq!(code.read_symbol(&mut reader)?, Value(3));
        assert_eq!(code.read_symbol(&mut reader)?, Value(6));
        assert_eq!(code.read_symbol(&mut reader)?, Value(7));
        assert_eq!(code.read_symbol(&mut reader)?, Value(8));
        assert_eq!(code.read_symbol(&mut reader)?, Value(10));
        assert_eq!(code.read_symbol(&mut reader)?, Value(12));
        assert!(code.read_symbol(&mut reader).is_err());

        Ok(())
    }

    #[test]
    fn from_lengths_additional() -> Result<()> {
        let lengths = [
            9, 10, 10, 8, 8, 8, 5, 6, 4, 5, 4, 5, 4, 5, 4, 4, 5, 4, 4, 5, 4, 5, 4, 5, 5, 5, 4, 6, 6,
        ];
        let code = HuffmanCoding::<Value>::from_lengths(&lengths)?;
        let mut data: &[u8] = &[
            0b11111000, 0b10111100, 0b01010001, 0b11111111, 0b00110101, 0b11111001, 0b11011111,
            0b11100001, 0b01110111, 0b10011111, 0b10111111, 0b00110100, 0b10111010, 0b11111111,
            0b11111101, 0b10010100, 0b11001110, 0b01000011, 0b11100111, 0b00000010,
        ];
        let mut reader = BitReader::new(&mut data);

        assert_eq!(code.read_symbol(&mut reader)?, Value(10));
        assert_eq!(code.read_symbol(&mut reader)?, Value(7));
        assert_eq!(code.read_symbol(&mut reader)?, Value(27));
        assert_eq!(code.read_symbol(&mut reader)?, Value(22));
        assert_eq!(code.read_symbol(&mut reader)?, Value(9));
        assert_eq!(code.read_symbol(&mut reader)?, Value(0));
        assert_eq!(code.read_symbol(&mut reader)?, Value(11));
        assert_eq!(code.read_symbol(&mut reader)?, Value(15));
        assert_eq!(code.read_symbol(&mut reader)?, Value(2));
        assert_eq!(code.read_symbol(&mut reader)?, Value(20));
        assert_eq!(code.read_symbol(&mut reader)?, Value(8));
        assert_eq!(code.read_symbol(&mut reader)?, Value(4));
        assert_eq!(code.read_symbol(&mut reader)?, Value(23));
        assert_eq!(code.read_symbol(&mut reader)?, Value(24));
        assert_eq!(code.read_symbol(&mut reader)?, Value(5));
        assert_eq!(code.read_symbol(&mut reader)?, Value(26));
        assert_eq!(code.read_symbol(&mut reader)?, Value(18));
        assert_eq!(code.read_symbol(&mut reader)?, Value(12));
        assert_eq!(code.read_symbol(&mut reader)?, Value(25));
        assert_eq!(code.read_symbol(&mut reader)?, Value(1));
        assert_eq!(code.read_symbol(&mut reader)?, Value(3));
        assert_eq!(code.read_symbol(&mut reader)?, Value(6));
        assert_eq!(code.read_symbol(&mut reader)?, Value(13));
        assert_eq!(code.read_symbol(&mut reader)?, Value(14));
        assert_eq!(code.read_symbol(&mut reader)?, Value(16));
        assert_eq!(code.read_symbol(&mut reader)?, Value(17));
        assert_eq!(code.read_symbol(&mut reader)?, Value(19));
        assert_eq!(code.read_symbol(&mut reader)?, Value(21));

        Ok(())
    }
}
