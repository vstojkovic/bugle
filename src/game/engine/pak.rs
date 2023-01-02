use std::borrow::Borrow;
use std::cell::{RefMut, RefCell};
use std::collections::HashSet;
use std::fs::File;
use std::hash::Hash;
use std::io::{Read, Seek, SeekFrom, Cursor};
use std::path::{PathBuf, Path};

use anyhow::{bail, Result};
use binread::{BinRead, BinReaderExt, BinResult};
use flate2::bufread::ZlibDecoder;

use super::{read_string, skip_string, ReaderSection};

#[derive(Debug)]
pub enum Compression {
    None,
    Zlib { blocks: Vec<CompressionBlock> },
    Unsupported(u32),
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct CompressionBlock {
    start: u64,
    end: u64,
}

#[derive(Debug)]
pub struct ArchiveEntry {
    pub path: String,
    offset: u64,
    pub size: u64,
    pub compression: Compression,
    pub encrypted: bool,
}

impl Hash for ArchiveEntry {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state)
    }
}

impl PartialEq for ArchiveEntry {
    fn eq(&self, other: &Self) -> bool {
        self.path.eq(&other.path)
    }
}

impl Eq for ArchiveEntry {}

impl Borrow<str> for ArchiveEntry {
    fn borrow(&self) -> &str {
        &self.path
    }
}

trait RandomAccess: Read + Seek {}
impl<R: Read + Seek> RandomAccess for R {}

pub struct ArchiveEntryReader<'a> {
    pub entry: &'a ArchiveEntry,
    inner: Box<dyn RandomAccess>,
    _guard: Option<RefMut<'a, ()>>,
}

impl<'a> ArchiveEntryReader<'a> {
    fn new(entry: &'a ArchiveEntry, mut file: File, guard: Option<RefMut<'a, ()>>) -> Result<Self> {
        match &entry.compression {
            Compression::None => Ok(Self {
                entry,
                inner: Box::new(ReaderSection::new(
                    file,
                    entry.offset + INLINE_HEADER_SIZE,
                    entry.size,
                )?),
                _guard: guard,
            }),
            Compression::Zlib { blocks } => {
                let mut uncompressed = Vec::with_capacity(entry.size as _);

                for block in blocks {
                    file.seek(SeekFrom::Start(block.start))?;

                    let mut block_buf = vec![0u8; (block.end - block.start) as usize];
                    file.read_exact(&mut block_buf)?;

                    let mut decoder = ZlibDecoder::new(&block_buf[..]);
                    decoder.read_to_end(&mut uncompressed)?;
                }

                Ok(Self { entry, inner: Box::new(Cursor::new(uncompressed)), _guard: None })
            }
            Compression::Unsupported(compression) => {
                bail!("unsupported compression: {}", compression);
            }
        }
    }
}

impl<'a> Read for ArchiveEntryReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<'a> Seek for ArchiveEntryReader<'a> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

pub struct Archive {
    path: PathBuf,
    file: File,
    file_lock: RefCell<()>,
    index: HashSet<ArchiveEntry>,
}

impl Archive {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = File::open(path.as_ref())?;

        // NOTE: Conan Exiles .pak files are version 4, but don't have the encrypted index flag
        let footer_offset = file.seek(SeekFrom::End(-44))?;

        if file.read_le::<u32>()? != 0x5a6f12e1 {
            bail!("not a valid .pak file");
        }

        let version: u32 = file.read_le()?;
        if version != 4 {
            bail!("unsupported .pak file version: {}", version);
        }

        let index_offset: u64 = file.read_le()?;
        let index_size: u64 = file.read_le()?;

        if (index_offset + index_size) > footer_offset {
            bail!("invalid index offset and/or size");
        }

        let index = read_index(&mut file, index_offset, index_size)?;

        Ok(Self { path: path.as_ref().to_path_buf(), file, file_lock: RefCell::new(()), index })
    }

    pub fn iter(&self) -> impl Iterator<Item = &ArchiveEntry> {
        self.index.iter()
    }

    pub fn entry(&self, path: &str) -> Option<&ArchiveEntry> {
        self.index.get(path)
    }

    pub fn open_entry(&self, path: &str) -> Result<ArchiveEntryReader> {
        let entry = if let Some(entry) = self.entry(path) {
            if entry.encrypted {
                bail!("encryption is currently not supported");
            }

            entry
        } else {
            bail!("entry not found: {}", path);
        };

        let (file, guard) = if let Ok(guard) = self.file_lock.try_borrow_mut() {
            (self.file.try_clone()?, Some(guard))
        } else {
            (File::open(&self.path)?, None)
        };

        ArchiveEntryReader::new(entry, file, guard)
    }
}

fn read_index(file: &mut File, offset: u64, size: u64) -> Result<HashSet<ArchiveEntry>> {
    file.seek(SeekFrom::Start(offset))?;

    let mut buf = vec![0u8; size as _];
    file.read_exact(&mut buf)?;
    let cursor = &mut Cursor::new(buf);

    // skip the mount point
    skip_string(cursor)?;

    let entry_count = cursor.read_le::<u32>()? as usize;
    let mut entries = HashSet::with_capacity(entry_count);
    for _ in 0..entry_count {
        let path = read_string(cursor)?;
        let offset = cursor.read_le()?;
        let _compressed_size: u64 = cursor.read_le()?;
        let size = cursor.read_le()?;
        let compression = cursor.read_le::<u32>()?;

        // skip SHA1
        cursor.seek(SeekFrom::Current(20))?;

        let compression = match compression {
            0 => Compression::None,
            1 => {
                let block_count: u32 = cursor.read_le()?;
                let blocks = (0..block_count)
                    .map(|_| cursor.read_le())
                    .collect::<BinResult<Vec<CompressionBlock>>>()?;
                Compression::Zlib { blocks }
            }
            unsupported => {
                // skip compression block info (32-bit count, 16 bytes per block record)
                let block_count: u32 = cursor.read_le()?;
                cursor.seek(SeekFrom::Current(block_count as i64 * 16))?;
                Compression::Unsupported(unsupported)
            }
        };

        let encrypted = cursor.read_le::<u8>()? != 0;
        let _compression_block_size: u32 = cursor.read_le()?;

        // skip unknown Conan Exiles field
        cursor.seek(SeekFrom::Current(4))?;

        entries.insert(ArchiveEntry { path, offset, size, compression, encrypted });
    }

    Ok(entries)
}

const INLINE_HEADER_SIZE: u64 = 57;
