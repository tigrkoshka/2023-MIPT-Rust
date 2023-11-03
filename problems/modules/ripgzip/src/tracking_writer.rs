#![forbid(unsafe_code)]

use std::cmp::min;
use std::collections::VecDeque;
use std::io::{self, Write};

use log::*;

use anyhow::{bail, Result};
use crc::{Crc, Digest, CRC_32_ISO_HDLC};

////////////////////////////////////////////////////////////////////////////////

const HISTORY_SIZE: usize = 32768;
const CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

pub(crate) struct History {
    data: VecDeque<Vec<u8>>,
    size: usize,
}

impl History {
    pub(crate) fn new() -> Self {
        Self {
            data: VecDeque::new(),
            size: 0,
        }
    }

    fn normalize(&mut self) {
        while self.size >= HISTORY_SIZE + self.data[0].len() {
            self.size -= self.data.pop_front().unwrap().len();
        }

        // Upon exit from while the following inequalities are true:
        //
        // self.size >= HISTORY_SIZE
        // self.data[0].len() > self.size - HISTORY_SIZE

        if self.size > HISTORY_SIZE {
            self.data[0] = self.data[0]
                .iter()
                .skip(self.size - HISTORY_SIZE)
                .copied()
                .collect();

            self.size = HISTORY_SIZE
        }
    }

    pub(crate) fn write(&mut self, buf: &[u8]) {
        self.data.push_back(buf.to_vec());
        self.size += buf.len();
        self.normalize();
    }

    pub(crate) fn get(&self, dist: usize, len: usize) -> Result<Vec<u8>> {
        log!(
            Level::Debug,
            "fetching data to write: dist = {}, len = {}",
            dist,
            len,
        );

        if dist > HISTORY_SIZE {
            bail!(
                "the requested distance ({}) is greater than the maximum history size ({})",
                dist,
                HISTORY_SIZE,
            );
        }

        if dist > self.size {
            bail!(
                "the requested distance ({}) is greater than the number of written bytes ({})",
                dist,
                self.size,
            );
        }

        if dist < len {
            bail!("the specified length is greater than the specified distance");
        }

        let mut left_dist = dist;
        let mut left_len = len;
        let mut chunk_idx = self.data.len() - 1;

        while left_dist > self.data[chunk_idx].len() {
            left_dist -= self.data[chunk_idx].len();
            chunk_idx -= 1;
        }

        if left_len <= left_dist {
            return Ok(self.data[chunk_idx]
                .iter()
                .rev()
                .take(left_dist)
                .rev()
                .take(left_len)
                .copied()
                .collect::<Vec<u8>>());
        }

        let mut to_ret = vec![];
        to_ret.extend(
            self.data[chunk_idx]
                .iter()
                .rev()
                .take(left_dist)
                .rev()
                .copied(),
        );

        left_len -= left_dist;
        chunk_idx += 1;

        // strictly > to avoid index out of bounds when we aim to fully read the last chunk,
        // if the comparison was >=, we would perform one more iteration, increase chunk_idx
        // (which would make it out of bounds), and then try to access self.data[chunk_idx]
        // (we could, of course, add a check left_len > 0, which would resolve this case,
        // but it gives a slight overhead on each iteration)
        while left_len > self.data[chunk_idx].len() {
            // pass an immutable reference to avoid moving or emptying
            to_ret.extend(&self.data[chunk_idx]);

            left_len -= self.data[chunk_idx].len();
            chunk_idx += 1;
        }

        to_ret.extend(self.data[chunk_idx].iter().take(left_len));

        Ok(to_ret)
    }
}

pub struct TrackingWriter<T> {
    inner: T,
    history: History,
    byte_count: usize,
    digest: Digest<'static, u32>,
}

impl<T: Write> Write for TrackingWriter<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.history.write(buf);
        let written = self.inner.write(buf)?;

        self.byte_count += written;
        self.digest.update(&buf[..written]);

        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<T: Write> TrackingWriter<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            history: History::new(),
            byte_count: 0,
            digest: CRC.digest(),
        }
    }

    /// Write a sequence of `len` bytes written `dist` bytes ago.
    pub fn write_previous(&mut self, dist: usize, len: usize) -> Result<()> {
        log!(
            Level::Debug,
            "writing previous data: dist = {}, len = {}",
            dist,
            len,
        );

        let to_write_vec = self.history.get(dist, min(len, dist))?;
        let to_write = to_write_vec.as_slice();

        log!(
            Level::Debug,
            "fetched data to write: size = {}",
            to_write.len(),
        );

        let mut left_len = len;
        while left_len >= to_write.len() {
            let curr_written = self.write(to_write)?;
            left_len -= curr_written;

            if curr_written != to_write.len() {
                bail!(
                    "written only {} bytes instead of the requested {}",
                    len - left_len,
                    len
                )
            }
        }

        if left_len != 0 {
            let curr_written = self.write(&to_write[..left_len])?;
            left_len -= curr_written;
            if left_len != 0 {
                bail!(
                    "written only {} bytes instead of the requested {}",
                    len - left_len,
                    len
                )
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn byte_count(&self) -> usize {
        self.byte_count
    }

    #[allow(dead_code)]
    pub fn crc32(self) -> u32 {
        self.digest.finalize()
    }

    pub fn finalize(self) -> (u32, usize, T) {
        let inner = self.inner;
        let digest = self.digest;
        (digest.finalize(), self.byte_count, inner)
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::WriteBytesExt;

    #[test]
    fn write() -> Result<()> {
        let mut buf: &mut [u8] = &mut [0u8; 10];
        let mut writer = TrackingWriter::new(&mut buf);

        assert_eq!(writer.write(&[1, 2, 3, 4])?, 4);
        assert_eq!(writer.byte_count(), 4);

        assert_eq!(writer.write(&[4, 8, 15, 16, 23])?, 5);
        assert_eq!(writer.byte_count(), 9);

        assert_eq!(writer.write(&[0, 0, 123])?, 1);
        assert_eq!(writer.byte_count(), 10);

        assert_eq!(writer.write(&[42, 124, 234, 27])?, 0);
        assert_eq!(writer.byte_count(), 10);
        assert_eq!(writer.crc32(), 2992191065);

        Ok(())
    }

    #[test]
    fn write_previous() -> Result<()> {
        let mut buf: &mut [u8] = &mut [0u8; 512];
        let mut writer = TrackingWriter::new(&mut buf);

        for i in 0..=255 {
            writer.write_u8(i)?;
        }

        writer.write_previous(192, 128)?;
        assert_eq!(writer.byte_count(), 384);

        assert!(writer.write_previous(10000, 20).is_err());
        assert_eq!(writer.byte_count(), 384);

        assert!(writer.write_previous(256, 256).is_err());
        assert_eq!(writer.byte_count(), 512);

        assert!(writer.write_previous(1, 1).is_err());
        assert_eq!(writer.byte_count(), 512);
        assert_eq!(writer.crc32(), 2733545866);

        Ok(())
    }
}
