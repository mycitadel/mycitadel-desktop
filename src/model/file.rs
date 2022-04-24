// MyCitadel desktop wallet: bitcoin & RGB wallet based on GTK framework.
//
// Written in 2022 by
//     Dr. Maxim Orlovsky <orlovsky@pandoraprime.ch>
//
// Copyright (C) 2022 by Pandora Prime Sarl, Switzerland.
//
// This software is distributed without any warranty. You should have received
// a copy of the AGPL-3.0 License along with this software. If not, see
// <https://www.gnu.org/licenses/agpl-3.0-standalone.html>.

use std::io::{Seek, Write};
use std::path::{Path, PathBuf};
use std::{fs, io};
use strict_encoding::{StrictDecode, StrictEncode};

use crate::model::Wallet;

/// Equals to first 4 bytes of SHA256("mycitadel:wallet:v1")
/// = a4546a8ef3a51f1faf2dab1517346e9d84b249f7f52d29339b4ee53fe870d14f
/// Check with `echo -n "mycitadel:wallet:v1" | shasum -a 256`
const WALLET_DOC_MAGIC: [u8; 4] = [0xa4, 0x54, 0x6a, 0x8e];

pub struct RefWrap<'doc, T>(pub(self) &'doc T)
where
    T: StrictEncode;

impl<'doc, T> StrictEncode for RefWrap<'doc, T>
where
    T: StrictEncode,
{
    fn strict_encode<E: Write>(&self, e: E) -> Result<usize, strict_encoding::Error> {
        self.0.strict_encode(e)
    }
}

#[derive(StrictDecode)]
pub struct DocReader<T>
where
    T: StrictDecode,
{
    pub(self) magic: [u8; 4],
    pub(self) data: T,
}

impl<T> DocReader<T>
where
    T: StrictDecode,
{
    pub fn magic_u32(&self) -> u32 {
        u32::from_be_bytes(self.magic)
    }
}

#[derive(StrictEncode)]
pub struct DocWriter<'doc, T>
where
    T: StrictEncode,
{
    pub(self) magic: [u8; 4],
    pub(self) data: RefWrap<'doc, T>,
}

impl<'doc, T> DocWriter<'doc, T>
where
    T: StrictEncode,
    RefWrap<'doc, T>: StrictEncode,
{
    pub fn with(magic: [u8; 4], data: &'doc T) -> Self {
        DocWriter {
            magic,
            data: RefWrap(data),
        }
    }
}

#[derive(Debug, Error, From, Display)]
#[display(inner)]
pub enum Error {
    #[from]
    File(io::Error),
    #[from]
    Encoding(strict_encoding::Error),
    #[display("incorrect file format or future version (expected {expected:#X}, got {actual:#X})")]
    Magic { expected: u32, actual: u32 },
    #[display("extra data after the end of file")]
    DataNotEntirelyConsumed,
}

pub trait FileDocument {
    const DOC_MAGIC: [u8; 4];

    const FILE_EXT: &'static str;

    fn magic_u32() -> u32 {
        u32::from_be_bytes(Self::DOC_MAGIC)
    }

    fn file_name(base: &str, order_no: usize) -> String {
        let mut path = PathBuf::from(format!("{}-{}", base, order_no));
        path.set_extension(Self::FILE_EXT);
        path.display().to_string()
    }

    fn read_file(path: impl AsRef<Path>) -> Result<Self, Error>
    where
        Self: StrictDecode,
    {
        let mut file = fs::OpenOptions::new()
            .create(false)
            .write(false)
            .read(true)
            .open(&path)?;
        let doc = DocReader::<Self>::strict_decode(&mut file)?;
        if fs::metadata(path)?.len() != file.stream_position()? {
            return Err(Error::DataNotEntirelyConsumed);
        }
        if doc.magic != Self::DOC_MAGIC {
            return Err(Error::Magic {
                expected: Self::magic_u32(),
                actual: doc.magic_u32(),
            });
        }
        Ok(doc.data)
    }

    fn write_file(&self, path: impl AsRef<Path>) -> Result<usize, Error>
    where
        Self: Sized + StrictEncode,
    {
        let doc = DocWriter::with(Self::DOC_MAGIC, self);
        let file = fs::File::create(path)?;
        doc.strict_encode(file).map_err(Error::Encoding)
    }
}

impl FileDocument for Wallet {
    const DOC_MAGIC: [u8; 4] = WALLET_DOC_MAGIC;
    const FILE_EXT: &'static str = "mcw";
}
