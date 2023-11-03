#![forbid(unsafe_code)]

use std::io::{BufRead, ErrorKind, Write};

use log::*;

use anyhow::{anyhow, bail, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use crc::Crc;

use crate::{bit_reader::BitReader, deflate::DeflateReader, tracking_writer::TrackingWriter};

////////////////////////////////////////////////////////////////////////////////

const ID1: u8 = 0x1f;
const ID2: u8 = 0x8b;

const CM_DEFLATE: u8 = 8;

const FTEXT_OFFSET: u8 = 0;
const FHCRC_OFFSET: u8 = 1;
const FEXTRA_OFFSET: u8 = 2;
const FNAME_OFFSET: u8 = 3;
const FCOMMENT_OFFSET: u8 = 4;

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct MemberHeader {
    pub compression_method: CompressionMethod,
    pub modification_time: u32,
    pub extra: Option<Vec<u8>>,
    pub name: Option<String>,
    pub comment: Option<String>,
    pub extra_flags: u8,
    pub os: u8,
    pub has_crc: bool,
    pub is_text: bool,
}

impl MemberHeader {
    pub fn crc16(&self) -> u16 {
        let crc = Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);
        let mut digest = crc.digest();

        digest.update(&[ID1, ID2, self.compression_method.into(), self.flags().0]);
        digest.update(&self.modification_time.to_le_bytes());
        digest.update(&[self.extra_flags, self.os]);

        if let Some(extra) = &self.extra {
            digest.update(&(extra.len() as u16).to_le_bytes());
            digest.update(extra);
        }

        if let Some(name) = &self.name {
            digest.update(name.as_bytes());
            digest.update(&[0]);
        }

        if let Some(comment) = &self.comment {
            digest.update(comment.as_bytes());
            digest.update(&[0]);
        }

        (digest.finalize() & 0xffff) as u16
    }

    pub fn flags(&self) -> MemberFlags {
        let mut flags = MemberFlags(0);
        flags.set_is_text(self.is_text);
        flags.set_has_crc(self.has_crc);
        flags.set_has_extra(self.extra.is_some());
        flags.set_has_name(self.name.is_some());
        flags.set_has_comment(self.comment.is_some());
        flags
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug)]
pub enum CompressionMethod {
    Deflate,
    Unknown(u8),
}

impl From<u8> for CompressionMethod {
    fn from(value: u8) -> Self {
        match value {
            CM_DEFLATE => Self::Deflate,
            x => Self::Unknown(x),
        }
    }
}

impl From<CompressionMethod> for u8 {
    fn from(method: CompressionMethod) -> u8 {
        match method {
            CompressionMethod::Deflate => CM_DEFLATE,
            CompressionMethod::Unknown(x) => x,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct MemberFlags(u8);

#[allow(unused)]
impl MemberFlags {
    fn bit(&self, n: u8) -> bool {
        (self.0 >> n) & 1 != 0
    }

    fn set_bit(&mut self, n: u8, value: bool) {
        if value {
            self.0 |= 1 << n;
        } else {
            self.0 &= !(1 << n);
        }
    }

    pub fn is_text(&self) -> bool {
        self.bit(FTEXT_OFFSET)
    }

    pub fn set_is_text(&mut self, value: bool) {
        self.set_bit(FTEXT_OFFSET, value)
    }

    pub fn has_crc(&self) -> bool {
        self.bit(FHCRC_OFFSET)
    }

    pub fn set_has_crc(&mut self, value: bool) {
        self.set_bit(FHCRC_OFFSET, value)
    }

    pub fn has_extra(&self) -> bool {
        self.bit(FEXTRA_OFFSET)
    }

    pub fn set_has_extra(&mut self, value: bool) {
        self.set_bit(FEXTRA_OFFSET, value)
    }

    pub fn has_name(&self) -> bool {
        self.bit(FNAME_OFFSET)
    }

    pub fn set_has_name(&mut self, value: bool) {
        self.set_bit(FNAME_OFFSET, value)
    }

    pub fn has_comment(&self) -> bool {
        self.bit(FCOMMENT_OFFSET)
    }

    pub fn set_has_comment(&mut self, value: bool) {
        self.set_bit(FCOMMENT_OFFSET, value)
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct MemberFooter {
    pub data_crc32: u32,
    pub data_size: u32,
}

////////////////////////////////////////////////////////////////////////////////

pub struct GzipReader<R, W> {
    reader: Option<R>,
    writer: Option<W>,
}

impl<R: BufRead, W: Write> GzipReader<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: Some(reader),
            writer: Some(writer),
        }
    }

    fn read_header_except_id1(&mut self) -> Result<MemberHeader> {
        let mut reader = self.reader.take().unwrap();

        // See RFC 1952, section 2.3.
        let id2 = reader.read_u8()?;
        if id2 != ID2 {
            bail!("wrong id values: id2 must be {}, instead got: {}", ID2, id2)
        }

        let compression_method = CompressionMethod::from(reader.read_u8()?);
        let flags = MemberFlags(reader.read_u8()?);
        let modification_time = reader.read_u32::<LittleEndian>()?;
        let extra_flags = reader.read_u8()?;
        let os = reader.read_u8()?;

        let mut extra: Option<Vec<u8>> = None;
        if flags.has_extra() {
            let len = reader.read_u16::<LittleEndian>()?;
            let vec = extra.insert(vec![0u8; len as usize]);
            reader.read_exact(vec.as_mut_slice())?;
        }

        let mut name: Option<String> = None;
        if flags.has_name() {
            let mut buf = vec![];
            reader.read_until(b'\0', &mut buf)?;
            if buf.is_empty() || buf.last().unwrap() != &b'\0' {
                bail!("encountered EOF before '\\0' when reading name")
            }

            name = Some(String::from_utf8(buf)?);
        }

        let mut comment: Option<String> = None;
        if flags.has_comment() {
            let mut buf = vec![];
            reader.read_until(b'\0', &mut buf)?;
            if buf.is_empty() || buf.last().unwrap() != &b'\0' {
                bail!("encountered EOF before '\\0' when reading comment")
            }

            comment = Some(String::from_utf8(buf)?);
        }

        let header = MemberHeader {
            compression_method,
            modification_time,
            extra,
            name,
            comment,
            extra_flags,
            os,
            has_crc: flags.has_crc(),
            is_text: flags.is_text(),
        };

        if flags.has_crc() {
            let recorded = reader.read_u16::<LittleEndian>()?;
            let actual = header.crc16();

            if recorded != actual {
                bail!(
                    "header crc16 check failed: recorded = {}, actual = {}",
                    recorded,
                    actual,
                )
            }
        }

        self.reader = Some(reader);

        Ok(header)
    }

    fn read_header(&mut self) -> Option<Result<MemberHeader>> {
        log!(Level::Info, "reading header");

        let id1 = match self.reader.as_mut().unwrap().read_u8() {
            Ok(value) => value,
            Err(err) => {
                if err.kind() == ErrorKind::UnexpectedEof {
                    return None;
                }

                return Some(Err(anyhow!(err)));
            }
        };
        if id1 != ID1 {
            return Some(Err(anyhow!(
                "wrong id values: id1 must be {}, instead got: {}",
                ID1,
                id1,
            )));
        }

        let to_ret = Some(self.read_header_except_id1());

        log!(Level::Info, "header read successfully");

        to_ret
    }

    fn read_footer(&mut self) -> Result<MemberFooter> {
        log!(Level::Info, "reading footer");

        let to_ret = Ok(MemberFooter {
            data_crc32: self.reader.as_mut().unwrap().read_u32::<LittleEndian>()?,
            data_size: self.reader.as_mut().unwrap().read_u32::<LittleEndian>()?,
        });

        log!(Level::Info, "footer read successfully");

        to_ret
    }

    fn read_member_except_header(&mut self, header: MemberHeader) -> Result<()> {
        if let CompressionMethod::Unknown(value) = header.compression_method {
            bail!(
                "unsupported compression method: {}, currently only deflate ({}) is supported",
                value,
                u8::from(CompressionMethod::Deflate),
            );
        }

        log!(Level::Info, "reading data");

        let deflate_reader = DeflateReader::new(
            BitReader::new(self.reader.take().unwrap()),
            TrackingWriter::new(self.writer.take().unwrap()),
        );
        let (bit_reader, tracking_writer) = deflate_reader.read()?;

        log!(Level::Info, "data read successfully");

        self.reader = Some(bit_reader.inner());
        let (crc32, byte_count, writer) = tracking_writer.finalize();
        self.writer = Some(writer);

        let footer = self.read_footer()?;

        if footer.data_size as usize != byte_count {
            bail!(
                "length check failed: recorded = {}, actual = {}",
                footer.data_size,
                byte_count,
            )
        }

        if footer.data_crc32 != crc32 {
            bail!(
                "crc32 check failed: recorded = {}, actual = {}",
                footer.data_crc32,
                crc32,
            )
        }

        Ok(())
    }

    fn read_member(&mut self) -> Option<Result<()>> {
        let header = match self.read_header()? {
            Ok(header) => header,
            Err(err) => return Some(Err(anyhow!(err))),
        };
        Some(self.read_member_except_header(header))
    }

    pub fn read(&mut self) -> Result<()> {
        let mut idx = 1;
        loop {
            log!(Level::Info, "reading member #{}", idx);

            match self.read_member() {
                None => break Ok(()),
                Some(res) => res?,
            }

            log!(Level::Info, "member #{} read successfully", idx);
            idx += 1;
        }
    }
}
