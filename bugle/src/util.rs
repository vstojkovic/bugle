use std::path::{Path, PathBuf};

use chrono::Weekday;

pub trait PathExt {
    fn join_all<P: AsRef<Path>, I: IntoIterator<Item = P>>(&self, iter: I) -> PathBuf;
}

impl PathExt for Path {
    fn join_all<P: AsRef<Path>, I: IntoIterator<Item = P>>(&self, iter: I) -> PathBuf {
        let mut result = self.to_path_buf();
        result.extend(iter);
        result
    }
}

pub fn weekday_iter() -> impl Iterator<Item = Weekday> {
    (0..7u8).map(|day| day.try_into().unwrap())
}

macro_rules! weak_cb {
    (@munch_args [$this:ident = $from:expr] [$($args:tt)*] $arg:pat_param , $($tail:tt)+) => {
        weak_cb!(@munch_args [$this = $from] [$($args)* $arg,] $($tail)+)
    };
    (@munch_args [$this:ident = $from:expr] [$($args:tt)*] $arg:pat_param | $($tail:tt)+) => {
        weak_cb!(@expand [$this = $from] [$($args)* $arg] $($tail)+)
    };
    (@munch_args [$this:ident = $from:expr] [$($args:tt)*] $arg_id:tt $(: $arg_ty:ty)? , $($tail:tt)+) => {
        weak_cb!(@munch_args [$this = $from] [$($args)* $arg_id $(: $arg_ty)?,] $($tail)+)
    };
    (@munch_args [$this:ident = $from:expr] [$($args:tt)*] $arg_id:tt $(: $arg_ty:ty)? | $($tail:tt)+) => {
        weak_cb!(@expand [$this = $from] [$($args)* $arg_id $(: $arg_ty)?] $($tail)+)
    };
    (@munch_args [$this:ident = $from:expr] [] | $(tail:tt)+) => {
        weak_cb!(@expand [$this = $from] [] $($tail)+)
    };
    (@expand [$this:ident = $from:expr] [$($args:tt)*] $body:expr $(; $($epi:tt)+)?) => {
        {
            let $this = Rc::downgrade($from);
            move |$($args)*| {
                if let Some($this) = $this.upgrade() {
                    $body
                }
                $($($epi)+)?
            }
        }
    };
    ([$this:ident = $from:expr] => || $($tail:tt)+) => {
        weak_cb!(@expand [$this = $from] [] $($tail)+)
    };
    ([$this:ident] => || $($tail:tt)+) => {
        weak_cb!([$this = &$this] => || $($tail)+)
    };
    ([$this:ident = $from:expr] => |$($tail:tt)+) => {
        weak_cb!(@munch_args [$this = $from] [] $($tail)+)
    };
    ([$this:ident] => |$($tail:tt)+) => {
        weak_cb!([$this = &$this] => |$($tail)+)
    };
}
pub(super) use weak_cb;
