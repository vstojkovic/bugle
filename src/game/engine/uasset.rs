use std::cmp::Ordering;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::ops::Deref;

use anyhow::{bail, Result};
use binread::{BinRead, BinReaderExt, BinResult, ReadOptions};

use super::name::{HardcodedName, Name, NameRegistry};
use super::pak::{Archive, ArchiveEntryReader};
use super::property::{PropertyTag, PropertyType};
use super::{skip_string, ReaderSection};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct NameIndex {
    pub index: usize,
    pub instance: Option<u32>,
}

impl From<usize> for NameIndex {
    fn from(index: usize) -> Self {
        NameIndex {
            index,
            instance: None,
        }
    }
}

#[derive(Clone, Copy)]
pub struct NameRef<'p, 'r> {
    pkg: &'p PackageSummary<'r>,
    idx: NameIndex,
    name: &'p Name,
}

impl<'p, 'r> NameRef<'p, 'r> {
    pub fn text(&self) -> &str {
        self.pkg.name_registry.text(self.name)
    }
}

impl<'p, 'r> Deref for NameRef<'p, 'r> {
    type Target = Name;
    fn deref(&self) -> &Self::Target {
        self.name
    }
}

impl<'p, 'r> ToString for NameRef<'p, 'r> {
    fn to_string(&self) -> String {
        if let Some(instance) = self.idx.instance {
            format!("{}_{}", self.text(), instance - 1)
        } else {
            self.text().to_owned()
        }
    }
}

impl BinRead for NameIndex {
    type Args = ();
    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &ReadOptions,
        _: Self::Args,
    ) -> BinResult<Self> {
        let index = u32::read_options(reader, options, ())? as usize;
        let instance = u32::read_options(reader, options, ())?;
        let instance = if instance > 0 { Some(instance - 1) } else { None };
        BinResult::Ok(Self { index, instance })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ResourceIndex {
    Null,
    Import(usize),
    Export(usize),
}

impl<'p, 'r> From<ImportRef<'p, 'r>> for ResourceIndex {
    fn from(imp: ImportRef<'p, 'r>) -> Self {
        Self::Import(imp.index())
    }
}

impl<'p, 'r> From<ExportRef<'p, 'r>> for ResourceIndex {
    fn from(exp: ExportRef<'p, 'r>) -> Self {
        Self::Export(exp.index())
    }
}

impl BinRead for ResourceIndex {
    type Args = ();
    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &ReadOptions,
        args: Self::Args,
    ) -> BinResult<Self> {
        let idx = i32::read_options(reader, options, args)?;
        BinResult::Ok(match idx.cmp(&0) {
            Ordering::Equal => Self::Null,
            Ordering::Greater => Self::Export((idx - 1) as usize),
            Ordering::Less => Self::Import((-idx - 1) as usize),
        })
    }
}

#[derive(BinRead, Debug, PartialEq, Eq)]
#[br(little)]
pub struct Import {
    pub package: NameIndex,
    pub class: NameIndex,
    pub outer: ResourceIndex,
    pub name: NameIndex,
}

#[derive(Clone, Copy)]
pub struct ImportRef<'p, 'r> {
    pkg: &'p PackageSummary<'r>,
    idx: usize,
    imp: &'p Import,
}

impl<'p, 'r> ImportRef<'p, 'r> {
    pub fn package(&self) -> NameRef<'p, 'r> {
        self.pkg.name_ref(self.package)
    }

    pub fn class(&self) -> NameRef<'p, 'r> {
        self.pkg.name_ref(self.class)
    }

    pub fn outer(&self) -> ResourceRef<'p, 'r> {
        self.pkg.resource_ref(self.outer)
    }

    pub fn name(&self) -> NameRef<'p, 'r> {
        self.pkg.name_ref(self.name)
    }

    pub fn index(&self) -> usize {
        self.idx
    }
}

impl<'p, 'r> Deref for ImportRef<'p, 'r> {
    type Target = Import;
    fn deref(&self) -> &Self::Target {
        self.imp
    }
}

#[derive(BinRead, Debug, PartialEq, Eq)]
#[br(little)]
pub struct Export {
    pub class: ResourceIndex,
    pub super_struct: ResourceIndex,
    pub template: ResourceIndex,
    pub outer: ResourceIndex,
    pub name: NameIndex,
    pub flags: u32,
    pub size: u32,
    pub offset: u32,
    #[br(pad_before(60))]
    _pad: (),
}

#[derive(Clone, Copy)]
pub struct ExportRef<'p, 'r> {
    _pkg: &'p PackageSummary<'r>,
    idx: usize,
    exp: &'p Export,
}

impl<'p, 'r> ExportRef<'p, 'r> {
    pub fn index(&self) -> usize {
        self.idx
    }
}

impl<'p, 'r> Deref for ExportRef<'p, 'r> {
    type Target = Export;
    fn deref(&self) -> &Self::Target {
        self.exp
    }
}

#[derive(Clone, Copy)]
pub enum ResourceRef<'p, 'r> {
    Null,
    Import(ImportRef<'p, 'r>),
    Export(ExportRef<'p, 'r>),
}

pub struct PackageSummary<'r> {
    pub name_registry: &'r NameRegistry,
    pub header_size: u32,
    pub names: Vec<Name>,
    pub imports: Vec<Import>,
    pub exports: Vec<Export>,
}

impl<'r> PackageSummary<'r> {
    pub fn name_ref<'p>(&'p self, idx: NameIndex) -> NameRef<'p, 'r> {
        let name = &self.names[idx.index];
        NameRef {
            pkg: self,
            idx,
            name,
        }
    }

    pub fn import_ref<'p>(&'p self, idx: usize) -> ImportRef<'p, 'r> {
        ImportRef {
            pkg: self,
            idx,
            imp: &self.imports[idx],
        }
    }

    pub fn iter_imports(&self) -> impl Iterator<Item = ImportRef> {
        self.imports.iter().enumerate().map(|(idx, imp)| ImportRef {
            pkg: self,
            idx,
            imp,
        })
    }

    pub fn export_ref<'p>(&'p self, idx: usize) -> ExportRef<'p, 'r> {
        ExportRef {
            _pkg: self,
            idx,
            exp: &self.exports[idx],
        }
    }

    pub fn iter_exports(&self) -> impl Iterator<Item = ExportRef> {
        self.exports.iter().enumerate().map(|(idx, exp)| ExportRef {
            _pkg: self,
            idx,
            exp,
        })
    }

    pub fn resource_ref<'p>(&'p self, idx: ResourceIndex) -> ResourceRef<'p, 'r> {
        match idx {
            ResourceIndex::Null => ResourceRef::Null,
            ResourceIndex::Import(idx) => ResourceRef::Import(self.import_ref(idx)),
            ResourceIndex::Export(idx) => ResourceRef::Export(self.export_ref(idx)),
        }
    }
}

pub struct Package<'a, 'r> {
    pub pak: &'a Archive,
    pub path: String,
    pub summary: PackageSummary<'r>,
}

impl<'a, 'r> Deref for Package<'a, 'r> {
    type Target = PackageSummary<'r>;
    fn deref(&self) -> &Self::Target {
        &self.summary
    }
}

impl<'a, 'r> Package<'a, 'r> {
    pub fn new(pak: &'a Archive, path: &str, name_registry: &'r NameRegistry) -> Result<Self> {
        let mut stream = pak.open_entry(&format!("{}.uasset", path))?;

        if stream.read_le::<u32>()? != 0x9e2a83c1 {
            bail!("not a .uasset file");
        }

        let file_version: i32 = stream.read_le()?;
        if file_version != -7 {
            bail!("unsupported .uasset file version: {}", file_version);
        }

        // skip LegacyUE3Version, FileVersionUE4, FileVersionLicenseeUE4
        stream.seek(SeekFrom::Current(12))?;

        let custom_version_count: i64 = stream.read_le::<i32>()?.into();
        if custom_version_count > 0 {
            stream.seek(SeekFrom::Current(20i64 * custom_version_count))?;
        }

        let header_size: u32 = stream.read_le::<u32>()?;
        let header_bytes_read = stream.stream_position()?;
        let header_buf_size = (header_size as usize) - (header_bytes_read as usize);
        let mut buf = vec![0u8; header_buf_size];
        stream.read_exact(&mut buf)?;

        let mut cursor = Cursor::new(buf);

        // skip FolderName, PackageFlags
        skip_string(&mut cursor)?;
        cursor.seek(SeekFrom::Current(4))?;

        let name_count = cursor.read_le::<u32>()? as usize;
        let name_offset: u64 = cursor.read_le::<u32>()?.into();

        // skip GatherableTextDataCount, GatherableTextDataOffset
        cursor.seek(SeekFrom::Current(8))?;

        let export_count = cursor.read_le::<u32>()? as usize;
        let export_offset: u64 = cursor.read_le::<u32>()?.into();
        let import_count = cursor.read_le::<u32>()? as usize;
        let import_offset: u64 = cursor.read_le::<u32>()?.into();

        cursor.seek(SeekFrom::Start(name_offset - header_bytes_read))?;
        let mut names = Vec::with_capacity(name_count);
        for _ in 0..name_count {
            let entry = cursor.read_le()?;
            names.push(name_registry.lookup(entry));
        }

        cursor.seek(SeekFrom::Start(import_offset - header_bytes_read))?;
        let mut imports = Vec::with_capacity(import_count);
        for _ in 0..import_count {
            imports.push(cursor.read_le()?);
        }

        cursor.seek(SeekFrom::Start(export_offset - header_bytes_read))?;
        let mut exports = Vec::with_capacity(export_count);
        for _ in 0..export_count {
            exports.push(cursor.read_le()?);
        }

        let header = PackageSummary {
            name_registry,
            header_size,
            names,
            imports,
            exports,
        };
        Ok(Package {
            pak,
            path: path.to_string(),
            summary: header,
        })
    }

    pub fn open_export<'p>(&'p self, idx: usize) -> Result<ExportReader<'p, 'a, 'r>> {
        let export = &self.summary.exports[idx];
        let inner = self.pak.open_entry(&format!("{}.uexp", self.path))?;
        let inner = ReaderSection::new(
            inner,
            (export.offset - self.summary.header_size) as _,
            export.size as _,
        )?;

        Ok(ExportReader { pkg: self, inner })
    }
}

pub struct ExportReader<'p, 'a, 'r> {
    pkg: &'p Package<'a, 'r>,
    inner: ReaderSection<ArchiveEntryReader<'a>>,
}

impl<'p, 'a, 'r> Read for ExportReader<'p, 'a, 'r> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<'p, 'a, 'r> Seek for ExportReader<'p, 'a, 'r> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl<'p, 'a, 'r> ExportReader<'p, 'a, 'r> {
    pub fn read_property_tag<'s>(&'s mut self) -> Result<Option<PropertyTag<'p, 'r>>>
    where
        'p: 's,
    {
        let name = self.pkg.name_ref(self.inner.read_le()?);
        if let Name::Hardcoded(HardcodedName::None) = *name {
            return Ok(None);
        }

        let type_name: NameIndex = self.inner.read_le()?;
        let size: u32 = self.inner.read_le()?;
        let array_idx = self.inner.read_le()?;

        let type_info = match self.pkg.summary.names[type_name.index] {
            Name::Hardcoded(HardcodedName::ByteProperty) => PropertyType::Byte {
                enum_name: self.pkg.name_ref(self.inner.read_le()?),
            },
            Name::Hardcoded(HardcodedName::EnumProperty) => PropertyType::Enum {
                enum_name: self.pkg.name_ref(self.inner.read_le()?),
            },
            Name::Hardcoded(HardcodedName::BoolProperty) => {
                PropertyType::Bool(self.inner.read_le::<u8>()? != 0)
            }
            Name::Hardcoded(HardcodedName::ArrayProperty) => PropertyType::Array {
                inner_type: self.pkg.name_ref(self.inner.read_le()?),
            },
            Name::Hardcoded(HardcodedName::StructProperty) => {
                let struct_name = self.pkg.name_ref(self.inner.read_le()?);
                self.inner.seek(SeekFrom::Current(16))?; // skip GUID
                PropertyType::Struct { struct_name }
            }
            Name::Hardcoded(HardcodedName::SetProperty) => PropertyType::Set {
                inner_type: self.pkg.name_ref(self.inner.read_le()?),
            },
            Name::Hardcoded(HardcodedName::MapProperty) => {
                let inner_type = self.pkg.name_ref(self.inner.read_le()?);
                let value_type = self.pkg.name_ref(self.inner.read_le()?);
                PropertyType::Map {
                    inner_type,
                    value_type,
                }
            }
            _ => PropertyType::Other(self.pkg.name_ref(type_name)),
        };

        if self.inner.read_le::<u8>()? != 0 {
            // skip the GUID
            self.inner.seek(SeekFrom::Current(16))?;
        }

        let skip_offset = SeekFrom::Start(self.inner.stream_position()? + size as u64);

        Ok(Some(PropertyTag {
            name,
            type_info,
            skip_offset,
            array_idx,
        }))
    }

    pub fn skip_property(&mut self, tag: &PropertyTag) -> Result<()> {
        self.inner.seek(tag.skip_offset)?;
        Ok(())
    }
}
