#![forbid(unsafe_code)]

use std::{
    convert::TryFrom,
    io::{BufRead, Write},
};

use anyhow::{bail, format_err, Error, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use log::*;

use crate::bit_reader::BitReader;
use crate::deflate::CompressionType::{DynamicTree, FixedTree, Reserved, Uncompressed};
use crate::huffman_coding::{
    decode_litlen_distance_trees, static_litlen_distance_trees, DistanceToken, HuffmanCoding,
    LitLenToken,
};
use crate::tracking_writer::TrackingWriter;

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct BlockHeader {
    pub is_final: bool,
    pub compression_type: CompressionType,
}

#[derive(Debug)]
pub enum CompressionType {
    Uncompressed = 0,
    FixedTree = 1,
    DynamicTree = 2,
    Reserved = 3,
}

impl TryFrom<u16> for CompressionType {
    type Error = Error;

    fn try_from(value: u16) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(Uncompressed),
            1 => Ok(FixedTree),
            2 => Ok(DynamicTree),
            3 => Ok(Reserved),
            _ => Err(format_err!("unsupported block type: {}", value)),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct DeflateReader<R, W> {
    bit_reader: BitReader<R>,
    tracking_writer: TrackingWriter<W>,
}

impl<R: BufRead, W: Write> DeflateReader<R, W> {
    pub fn new(bit_reader: BitReader<R>, tracking_writer: TrackingWriter<W>) -> Self {
        Self {
            bit_reader,
            tracking_writer,
        }
    }

    fn read_block_header(&mut self) -> Result<BlockHeader> {
        log!(Level::Info, "reading data block header");

        let to_ret = Ok(BlockHeader {
            is_final: self.bit_reader.read_bits(1)?.bits() != 0,
            compression_type: self.bit_reader.read_bits(2)?.bits().try_into()?,
        });

        log!(Level::Info, "data block header read successfully");

        to_ret
    }

    fn read_uncompressed_block(&mut self) -> Result<()> {
        log!(Level::Info, "reading data block contents (uncompressed)");

        let reader = self.bit_reader.borrow_reader_from_boundary();

        let len = reader.read_u16::<LittleEndian>()?;
        let nlen = reader.read_u16::<LittleEndian>()?;
        if len != !nlen {
            bail!(
                "nlen check failed: len = {}, !len = {}, nlen = {}",
                len,
                !len,
                nlen,
            );
        }

        let mut buf = vec![0u8; len as usize];
        reader.read_exact(buf.as_mut_slice())?;
        self.tracking_writer.write_all(buf.as_slice())?;

        Ok(())
    }

    fn read_compressed_block(
        &mut self,
        litlen: &HuffmanCoding<LitLenToken>,
        dist: &HuffmanCoding<DistanceToken>,
    ) -> Result<()> {
        log!(Level::Info, "reading compressed data");

        let mut idx = 1;
        loop {
            log!(Level::Debug, "reading symbol #{}", idx);

            let litlen_token = litlen.read_symbol(&mut self.bit_reader)?;
            match litlen_token {
                LitLenToken::Literal(value) => {
                    log!(
                        Level::Debug,
                        "symbol #{} is a literal: \'{}\'",
                        idx,
                        value as char,
                    );
                    self.tracking_writer.write_u8(value)?
                }
                LitLenToken::EndOfBlock => {
                    log!(Level::Debug, "symbol #{} is an end of block", idx);
                    break;
                }
                LitLenToken::Length {
                    base: len_base,
                    extra_bits: len_extra,
                } => {
                    log!(Level::Debug, "symbol #{} is a repetition", idx);

                    let len = len_base + self.bit_reader.read_bits(len_extra)?.bits();

                    let DistanceToken {
                        base: dist_base,
                        extra_bits: dist_extra,
                    } = dist.read_symbol(&mut self.bit_reader)?;
                    let dist = dist_base + self.bit_reader.read_bits(dist_extra)?.bits();

                    log!(
                        Level::Debug,
                        "symbol #{}: dist = {}, len = {}",
                        idx,
                        dist,
                        len
                    );

                    self.tracking_writer
                        .write_previous(dist as usize, len as usize)?;
                }
            }

            log!(Level::Debug, "symbol #{} read successfully", idx);
            idx += 1;
        }

        log!(Level::Info, "compressed data read successfully");

        Ok(())
    }

    fn read_fixed_tree_block(&mut self) -> Result<()> {
        log!(Level::Info, "reading data block contents (fixed tree)");

        log!(Level::Info, "building fixed huffman codes");
        let (litlen, dist) = static_litlen_distance_trees()?;
        log!(Level::Info, "fixed huffman codes built successfully");

        self.read_compressed_block(&litlen, &dist)
    }

    fn read_dynamic_tree_block(&mut self) -> Result<()> {
        log!(Level::Info, "reading data block contents (dynamic tree)");

        log!(Level::Info, "building dynamic huffman codes");
        let (litlen, dist) = decode_litlen_distance_trees(&mut self.bit_reader)?;
        log!(Level::Info, "dynamic huffman codes built successfully");

        self.read_compressed_block(&litlen, &dist)
    }

    fn read_block(&mut self, compression_type: CompressionType) -> Result<()> {
        let to_ret = match compression_type {
            Uncompressed => self.read_uncompressed_block(),
            FixedTree => self.read_fixed_tree_block(),
            DynamicTree => self.read_dynamic_tree_block(),
            Reserved => bail!("unsupported block type: reserved"),
        };

        log!(Level::Info, "data block contents read successfully");

        to_ret
    }

    pub fn read(mut self) -> Result<(BitReader<R>, TrackingWriter<W>)> {
        let mut idx = 1;
        loop {
            log!(Level::Info, "reading data block #{}", idx);

            let header = self.read_block_header()?;
            self.read_block(header.compression_type)?;
            if header.is_final {
                break;
            }

            log!(Level::Info, "data block #{} read successfully", idx);
            idx += 1;
        }

        Ok((self.bit_reader, self.tracking_writer))
    }
}
