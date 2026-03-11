use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::buffer::Buffer;

pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    /// Open a file relative to the workspace root into a Buffer
    pub fn open_file(&self, path: impl AsRef<Path>) -> Result<Buffer> {
        let abs = self.root.join(path);
        let contents = std::fs::read_to_string(abs)?;
        Ok(Buffer::with_text(&contents))
    }

    /// Save a Buffer to a file relative to the workspace root
    pub fn save_file(&self, path: impl AsRef<Path>, buffer: &Buffer) -> Result<()> {
        let abs = self.root.join(path);
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(abs, buffer.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn open_reads_file_into_buffer() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("sample.txt");
        std::fs::File::create(&file_path)
            .and_then(|mut f| f.write_all(b"hello"))
            .unwrap();

        let ws = Workspace::new(dir.path().to_path_buf());
        let buf = ws.open_file("sample.txt").unwrap();
        assert_eq!(buf.to_string(), "hello");
    }

    #[test]
    fn save_writes_buffer_to_disk() {
        let dir = tempdir().unwrap();
        let ws = Workspace::new(dir.path().to_path_buf());
        let mut buf = Buffer::new();
        buf.insert(0, "data");

        ws.save_file("nested/file.txt", &buf).unwrap();
        let contents = std::fs::read_to_string(dir.path().join("nested/file.txt")).unwrap();
        assert_eq!(contents, "data");
    }
}
