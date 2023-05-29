use std::io::{Read, Seek, SeekFrom};

use anyhow::Result;
use binread::{BinRead, BinReaderExt, BinResult, ReadOptions};

pub(super) mod db;
pub(super) mod map;
mod name;
pub(super) mod pak;
mod property;
mod uasset;
pub(super) mod version;

struct UString(String);

impl ToString for UString {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl From<UString> for String {
    fn from(s: UString) -> Self {
        s.0
    }
}

impl BinRead for UString {
    type Args = ();
    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &ReadOptions,
        _: Self::Args,
    ) -> BinResult<Self> {
        let pos = reader.stream_position()?;
        let len = i32::read_options(reader, options, ())?;

        if len == 0 {
            return Ok(Self(String::new()));
        }

        if len < 0 {
            let len = (-len - 1) as usize;

            let ucs2_buf = (0..len)
                .map(|_| reader.read_le())
                .collect::<BinResult<Vec<u16>>>()?;
            reader.read_le::<u16>()?;

            let mut utf8_bytes = vec![0u8; len * 3];
            let len =
                ucs2::decode(&ucs2_buf, &mut utf8_bytes).map_err(|err| binread::Error::Custom {
                    pos,
                    err: Box::new(err),
                })?;

            return Ok(Self(
                String::from_utf8_lossy(&utf8_bytes[0..len]).into_owned(),
            ));
        }

        let len = (len - 1) as usize;
        let mut bytes = vec![0u8; len];
        reader.read_exact(&mut bytes)?;
        reader.read_le::<u8>()?;

        Ok(Self(String::from_utf8_lossy(&bytes).into_owned()))
    }
}

fn read_string<R: Read + Seek>(stream: &mut R) -> Result<String> {
    Ok(stream.read_le::<UString>()?.into())
}

fn skip_string<R: Read + Seek>(stream: &mut R) -> Result<()> {
    let len: i32 = stream.read_le()?;

    if len != 0 {
        let to_skip = if len > 0 { len } else { -len * 2 };
        stream.seek(SeekFrom::Current(to_skip.into()))?;
    }

    Ok(())
}

pub struct ReaderSection<R: Read + Seek> {
    inner: R,
    start: u64,
    end: u64,
    pos: u64,
}

impl<R: Read + Seek> ReaderSection<R> {
    pub(crate) fn new(mut inner: R, offset: u64, size: u64) -> Result<Self> {
        let start = offset;
        let end = start + size;

        inner.seek(SeekFrom::Start(start))?;

        Ok(Self {
            inner,
            start,
            end,
            pos: start,
        })
    }
}

impl<R: Read + Seek> Read for ReaderSection<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let limit = self.end - self.pos;
        if limit == 0 {
            return std::io::Result::Ok(0);
        }

        let to_read = std::cmp::min(buf.len() as u64, limit) as usize;
        let bytes_read = self.inner.read(&mut buf[..to_read])?;
        self.pos += bytes_read as u64;

        std::io::Result::Ok(bytes_read)
    }
}

impl<R: Read + Seek> Seek for ReaderSection<R> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        fn add_offset(base: u64, offset: i64) -> u64 {
            if offset >= 0 {
                base.wrapping_add(offset as u64)
            } else {
                base.wrapping_sub(offset.unsigned_abs())
            }
        }

        self.pos = match pos {
            SeekFrom::Start(offset) => self.start + offset,
            SeekFrom::End(offset) => add_offset(self.end, offset),
            SeekFrom::Current(offset) => add_offset(self.pos, offset),
        }
        .clamp(self.start, self.end);

        self.pos = self.inner.seek(SeekFrom::Start(self.pos))?;

        std::io::Result::Ok(self.pos - self.start)
    }
}
