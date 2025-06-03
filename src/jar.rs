use std::{
    fs::File,
    path::Path,
};
use zip::ZipArchive;
use anyhow::{Result, Context};

pub fn open_jar_file(jar_path: &Path) -> Result<ZipArchive<File>> {
    let file = File::open(jar_path)
        .with_context(|| format!("Failed to open JAR file: {}", jar_path.display()))?;

    ZipArchive::new(file)
        .with_context(|| format!("Invalid ZIP/JAR format: {}", jar_path.display()))
}