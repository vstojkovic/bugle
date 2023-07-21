use std::rc::Rc;

pub mod steam;

pub trait ModDirectory {
    fn resolve(self: Rc<Self>, mods: &mut [(u64, Option<String>)]);
}
