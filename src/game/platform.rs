use std::rc::Rc;

use anyhow::Result;

use crate::workers::TaskState;

use super::ModEntry;

pub mod steam;

pub trait ModDirectory {
    fn resolve(self: Rc<Self>, mods: &mut [(u64, Option<String>)]);
    fn needs_update(self: Rc<Self>, mod_ref: &ModEntry) -> Result<bool>;
    fn can_update(self: Rc<Self>) -> bool;
    fn start_update(self: Rc<Self>, mod_ref: &ModEntry) -> Result<Rc<dyn ModUpdate>>;
}

pub trait ModUpdate {
    fn state(&self) -> TaskState<Result<()>>;
    fn progress(&self) -> Option<(u64, u64)>;
}
