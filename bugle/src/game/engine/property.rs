use std::io::SeekFrom;

use super::uasset::NameRef;

#[derive(Clone)]
pub enum PropertyType<'p, 'r> {
    Byte {
        enum_name: NameRef<'p, 'r>,
    },
    Enum {
        enum_name: NameRef<'p, 'r>,
    },
    Bool(bool),
    Array {
        inner_type: NameRef<'p, 'r>,
    },
    Struct {
        struct_name: NameRef<'p, 'r>,
    },
    Set {
        inner_type: NameRef<'p, 'r>,
    },
    Map {
        inner_type: NameRef<'p, 'r>,
        value_type: NameRef<'p, 'r>,
    },
    Other(NameRef<'p, 'r>),
}

#[derive(Clone)]
pub struct PropertyTag<'p, 'r> {
    pub name: NameRef<'p, 'r>,
    pub type_info: PropertyType<'p, 'r>,
    pub skip_offset: SeekFrom,
    pub array_idx: u32,
}
