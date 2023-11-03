#![forbid(unsafe_code)]

use byteorder::ReadBytesExt;
use std::io::{self, BufRead};

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BitSequence {
    bits: u16,
    len: u8,
}

impl BitSequence {
    pub fn new(bits: u16, len: u8) -> Self {
        assert!(len <= 16);

        Self {
            // zero unused bits
            bits: bits
                .overflowing_shl(16 - len as u32)
                .0
                .overflowing_shr(16 - len as u32)
                .0,
            len,
        }
    }

    pub fn bits(&self) -> u16 {
        self.bits
    }

    pub fn len(&self) -> u8 {
        self.len
    }

    pub fn drop_low(&mut self, k: u8) -> u16 {
        assert!(k <= self.len);

        let to_ret = self.bits & (1u16.overflowing_shl(k as u32).0 - 1);
        self.bits = self.bits.overflowing_shr(k as u32).0;
        self.len -= k;

        to_ret
    }

    pub fn concat(self, other: Self) -> Self {
        assert!(self.len + other.len <= 16);

        BitSequence {
            bits: (self.bits.overflowing_shl(other.len as u32).0) | other.bits,
            len: self.len + other.len,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct BitReader<T> {
    stream: T,
    leftover: BitSequence,
}

impl<T: BufRead> BitReader<T> {
    pub fn new(stream: T) -> Self {
        Self {
            stream,
            leftover: BitSequence::new(0, 0),
        }
    }

    pub fn read_bits(&mut self, len: u8) -> io::Result<BitSequence> {
        assert!(len <= 16);

        if self.leftover.len() >= len {
            return Ok(BitSequence::new(self.leftover.drop_low(len), len));
        }

        let mut to_ret = self.leftover.bits();
        let mut read_len = self.leftover.len();

        let mut next_byte = self.stream.read_u8()?;

        if len - read_len > 8 {
            to_ret += (next_byte as u16).overflowing_shl(read_len as u32).0;
            read_len += 8;

            next_byte = self.stream.read_u8()?;
        }

        self.leftover = BitSequence::new(next_byte as u16, 8);
        to_ret += self
            .leftover
            .drop_low(len - read_len)
            .overflowing_shl(read_len as u32)
            .0;
        Ok(BitSequence::new(to_ret, len))
    }

    /// Discard all the unread bits in the current byte and return a mutable reference
    /// to the underlying reader.
    pub fn borrow_reader_from_boundary(&mut self) -> &mut T {
        self.leftover = BitSequence::new(0, 0);
        &mut self.stream
    }

    pub fn inner(self) -> T {
        self.stream
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::ReadBytesExt;

    #[test]
    fn read_bits() -> io::Result<()> {
        let data: &[u8] = &[0b01100011, 0b11011011, 0b10101111];
        let mut reader = BitReader::new(data);
        assert_eq!(reader.read_bits(1)?, BitSequence::new(0b1, 1));
        assert_eq!(reader.read_bits(2)?, BitSequence::new(0b01, 2));
        assert_eq!(reader.read_bits(3)?, BitSequence::new(0b100, 3));
        assert_eq!(reader.read_bits(4)?, BitSequence::new(0b1101, 4));
        assert_eq!(reader.read_bits(5)?, BitSequence::new(0b10110, 5));
        assert_eq!(reader.read_bits(8)?, BitSequence::new(0b01011111, 8));
        assert_eq!(
            reader.read_bits(2).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
        Ok(())
    }

    #[test]
    fn borrow_reader_from_boundary() -> io::Result<()> {
        let data: &[u8] = &[0b01100011, 0b11011011, 0b10101111];
        let mut reader = BitReader::new(data);
        assert_eq!(reader.read_bits(3)?, BitSequence::new(0b011, 3));
        assert_eq!(reader.borrow_reader_from_boundary().read_u8()?, 0b11011011);
        assert_eq!(reader.read_bits(8)?, BitSequence::new(0b10101111, 8));
        Ok(())
    }
}
