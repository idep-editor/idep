use crate::buffer::Buffer;
use anyhow::Result;
use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
use std::path::{Path, PathBuf};
use std::time::Duration;

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

    /// Watch the workspace tree and invoke callback on file changes.
    /// Uses debouncing (100ms) to avoid multiple callbacks for rapid writes.
    /// Caller must keep the returned debouncer alive.
    pub fn watch<F>(
        &self,
        mut on_change: F,
    ) -> Result<notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>>
    where
        F: FnMut(&Path) + Send + 'static,
    {
        let root = self.root.clone();
        let mut debouncer = new_debouncer(
            Duration::from_millis(100),
            move |res: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
                if let Ok(events) = res {
                    for event in events {
                        on_change(event.path.as_path());
                    }
                }
            },
        )
        .map_err(|e| anyhow::anyhow!("Failed to create debouncer: {}", e))?;

        debouncer
            .watcher()
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|e| anyhow::anyhow!("Failed to watch directory: {}", e))?;
        Ok(debouncer)
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
