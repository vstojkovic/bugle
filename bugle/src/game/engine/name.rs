use std::collections::HashMap;
use std::hash::Hash;
use std::io::{Read, Seek};
use std::rc::Rc;

use binread::BinRead;
use crc32fast::Hasher;
use strum::EnumCount;
use strum_macros::{AsRefStr, EnumCount, EnumIter};

use super::UString;

#[derive(PartialEq, Eq, Clone)]
pub enum Name {
    Hardcoded(HardcodedName),
    Interred(usize),
    AdHoc(String),
}

#[derive(AsRefStr, EnumCount, EnumIter, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum HardcodedName {
    None,
    ByteProperty,
    BoolProperty,
    ArrayProperty,
    StructProperty,
    MapProperty,
    SetProperty,
    EnumProperty,
}

impl HardcodedName {
    pub fn text(&self) -> &str {
        self.as_ref()
    }
}

pub struct NameEntry {
    text: String,
    hash: u16,
}

impl Hash for NameEntry {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u16(self.hash);
    }
}

impl PartialEq for NameEntry {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

impl Eq for NameEntry {}

impl From<String> for NameEntry {
    fn from(text: String) -> Self {
        let hash = hash_name(&text);
        Self { text, hash }
    }
}

impl From<&str> for NameEntry {
    fn from(text: &str) -> Self {
        text.to_owned().into()
    }
}

impl BinRead for NameEntry {
    type Args = ();
    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &binread::ReadOptions,
        _: Self::Args,
    ) -> binread::BinResult<Self> {
        let text = UString::read_options(reader, options, ())?.into();
        u16::read_options(reader, options, ())?; // skip the case-insensitive hash
        let hash = u16::read_options(reader, options, ())?;
        Ok(Self { text, hash })
    }
}

pub struct NameRegistry {
    lookup: HashMap<Rc<NameEntry>, Name>,
    interred: Vec<Rc<NameEntry>>,
}

impl NameRegistry {
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        use strum::IntoEnumIterator;

        let mut lookup = HashMap::with_capacity(capacity + HardcodedName::COUNT);
        let interred = Vec::with_capacity(capacity);

        for name in HardcodedName::iter() {
            let entry = Rc::new(name.as_ref().into());
            lookup.insert(entry, Name::Hardcoded(name));
        }

        Self { lookup, interred }
    }

    pub fn inter(&mut self, text: String) -> Name {
        let entry = text.into();
        if let Some(name) = self.lookup.get(&entry) {
            name.clone()
        } else {
            let id = self.interred.len();
            let entry = Rc::new(entry);
            self.lookup.insert(Rc::clone(&entry), Name::Interred(id));
            self.interred.push(entry);
            Name::Interred(id)
        }
    }

    pub fn lookup(&self, entry: NameEntry) -> Name {
        if let Some(name) = self.lookup.get(&entry) {
            name.clone()
        } else {
            Name::AdHoc(entry.text)
        }
    }

    pub fn text<'r, 's: 'r, 'n: 'r>(&'s self, name: &'n Name) -> &'r str {
        match name {
            Name::Hardcoded(hardcoded) => hardcoded.text(),
            Name::Interred(id) => &self.interred[*id].text,
            Name::AdHoc(ad_hoc) => &ad_hoc,
        }
    }
}

fn hash_name(s: &str) -> u16 {
    let mut hasher = Hasher::new();
    for c in s.chars() {
        hasher.update(&(c as u32).to_le_bytes());
    }
    (hasher.finalize() & 0xffff) as _
}
