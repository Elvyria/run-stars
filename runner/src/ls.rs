use std::{fs::{DirEntry, Metadata}, io, path::Path};

pub fn files(p: impl AsRef<Path>) -> Result<impl Iterator<Item = DirEntry>, io::Error> {
    Ok(std::fs::read_dir(p)?
        .flatten()
        .filter(|f| std::fs::metadata(f.path()).as_ref().is_ok_and(Metadata::is_file)))
}
