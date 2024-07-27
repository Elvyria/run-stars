use std::{fs::{DirEntry, FileType}, io, path::Path};

pub fn files(p: impl AsRef<Path>) -> Result<impl Iterator<Item = DirEntry>, io::Error> {
    Ok(std::fs::read_dir(p)?
        .flatten()
        .filter(|f| f.file_type().as_ref().is_ok_and(FileType::is_file)))
}
