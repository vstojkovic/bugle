use syn::{Attribute, Path, Result};

use crate::attr::{extract_path, extract_str, unknown_attr, IniAttr};

#[derive(Default)]
pub struct StructAttr {
    pub key_format: Option<String>,
}

#[derive(Default)]
pub struct FieldAttr {
    pub key_name: Option<String>,
    pub key_format: Option<String>,
    pub flatten: Option<()>,
    pub load_fn: Option<LoadFn>,
    pub remove_fn: Option<Path>,
    pub append_fn: Option<AppendFn>,
}

#[derive(Default)]
pub struct EnumAttr {
    pub repr: Option<()>,
    pub ignore_case: Option<()>,
}

pub enum LoadFn {
    InPlace(Path),
    Constructed(Path),
    Parsed(Path),
}

pub enum AppendFn {
    Append(Path),
    Display(Path),
}

impl IniAttr for StructAttr {
    fn update_from_ast(&mut self, attr: &Attribute) -> Result<()> {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("key_format") {
                let key_format = extract_str(&meta)?;
                if self.key_format.is_some() {
                    return Err(meta.error("duplicate ini attribute `key_format`"));
                }
                self.key_format = Some(key_format);
                return Ok(());
            }
            unknown_attr(meta)
        })?;
        Ok(())
    }
}

impl IniAttr for FieldAttr {
    fn update_from_ast(&mut self, attr: &Attribute) -> Result<()> {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let key = extract_str(&meta)?;
                if self.key_name.is_some() {
                    return Err(meta.error("duplicate ini attribute `rename`"));
                }
                if self.flatten.is_some() {
                    return Err(meta.error("cannot rename a flattened field"));
                }
                self.key_name = Some(key);
                return Ok(());
            }
            if meta.path.is_ident("key_format") {
                let key_format = extract_str(&meta)?;
                if self.key_format.is_some() {
                    return Err(meta.error("duplicate ini attribute `key_format`"));
                }
                if self.flatten.is_some() {
                    return Err(meta.error("cannot define a key format for a flattened field"));
                }
                self.key_format = Some(key_format);
                return Ok(());
            }
            if meta.path.is_ident("load_in_with") {
                let path = extract_path(&meta)?;
                if self.load_fn.is_some() {
                    return Err(meta.error("conflicting load function specified"));
                }
                if self.flatten.is_some() {
                    return Err(meta.error("cannot define a load function for a flattened field"));
                }
                self.load_fn = Some(LoadFn::InPlace(path));
                return Ok(());
            }
            if meta.path.is_ident("load_with") {
                let path = extract_path(&meta)?;
                if self.load_fn.is_some() {
                    return Err(meta.error("conflicting load function specified"));
                }
                if self.flatten.is_some() {
                    return Err(meta.error("cannot define a load function for a flattened field"));
                }
                self.load_fn = Some(LoadFn::Constructed(path));
                return Ok(());
            }
            if meta.path.is_ident("parse_with") {
                let path = extract_path(&meta)?;
                if self.load_fn.is_some() {
                    return Err(meta.error("conflicting load function specified"));
                }
                if self.flatten.is_some() {
                    return Err(meta.error("cannot define a load function for a flattened field"));
                }
                self.load_fn = Some(LoadFn::Parsed(path));
                return Ok(());
            }
            if meta.path.is_ident("remove_with") {
                let path = extract_path(&meta)?;
                if self.remove_fn.is_some() {
                    return Err(meta.error("conflicting remove function specified"));
                }
                if self.flatten.is_some() {
                    return Err(meta.error("cannot define a remove function for a flattened field"));
                }
                self.remove_fn = Some(path);
                return Ok(());
            }
            if meta.path.is_ident("append_with") {
                let path = extract_path(&meta)?;
                if self.append_fn.is_some() {
                    return Err(meta.error("conflicting append function specified"));
                }
                if self.flatten.is_some() {
                    return Err(meta.error("cannot define a append function for a flattened field"));
                }
                self.append_fn = Some(AppendFn::Append(path));
                return Ok(());
            }
            if meta.path.is_ident("display_with") {
                let path = extract_path(&meta)?;
                if self.append_fn.is_some() {
                    return Err(meta.error("conflicting append function specified"));
                }
                if self.flatten.is_some() {
                    return Err(meta.error("cannot define a append function for a flattened field"));
                }
                self.append_fn = Some(AppendFn::Display(path));
                return Ok(());
            }
            if meta.path.is_ident("flatten") {
                if self.key_name.is_some() {
                    return Err(meta.error("cannot flatten a renamed field"));
                }
                if self.key_format.is_some() {
                    return Err(meta.error("cannot flatten a field with a defined key format"));
                }
                if self.load_fn.is_some() {
                    return Err(meta.error("cannot flatten a field with a defined load function"));
                }
                self.flatten = Some(());
                return Ok(());
            }
            unknown_attr(meta)
        })?;
        Ok(())
    }
}

impl IniAttr for EnumAttr {
    fn update_from_ast(&mut self, attr: &Attribute) -> Result<()> {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("repr") {
                if self.ignore_case.is_some() {
                    return Err(meta.error("cannot specify both `repr` and `ignore_case`"));
                }
                self.repr = Some(());
                return Ok(());
            }
            if meta.path.is_ident("ignore_case") {
                if self.repr.is_some() {
                    return Err(meta.error("cannot specify both `repr` and `ignore_case`"));
                }
                self.ignore_case = Some(());
                return Ok(());
            }
            unknown_attr(meta)
        })?;
        Ok(())
    }
}
