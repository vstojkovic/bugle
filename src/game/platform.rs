use std::rc::Rc;

use anyhow::Result;

use super::ModInfo;

pub mod steam;

pub trait ModDirectory {
    fn resolve(self: Rc<Self>, mods: &mut [(u64, Option<String>)]);
    fn needs_update(self: Rc<Self>, mod_ref: &ModInfo) -> Result<bool>;
}
