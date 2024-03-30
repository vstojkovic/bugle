use syn::{Attribute, Result};

use crate::attr::{extract_str, unknown_attr, IniAttr};

#[derive(Default)]
pub struct FieldAttr {
    pub section: Option<Option<String>>,
}

impl IniAttr for FieldAttr {
    fn update_from_ast(&mut self, attr: &Attribute) -> Result<()> {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("section") {
                let section = extract_str(&meta)?;
                if self.section.is_some() {
                    return Err(meta.error("conflicting section specified"));
                }
                self.section = Some(Some(section));
                return Ok(());
            }
            if meta.path.is_ident("general") {
                if self.section.is_some() {
                    return Err(meta.error("conflicting section specified"));
                }
                self.section = Some(None);
                return Ok(());
            }
            unknown_attr(meta)
        })?;
        Ok(())
    }
}
