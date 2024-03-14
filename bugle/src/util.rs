use std::path::{Path, PathBuf};

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
