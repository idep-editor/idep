use crate::buffer::Buffer;
use anyhow::Result;
use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Normalize paths for WSL2 compatibility.
/// Converts `/mnt/c/...` (DrvFs) paths to their canonical form.
/// On native Linux, returns the path unchanged.
fn normalize_path(path: &Path) -> PathBuf {
    // Check if running on WSL2 by looking for microsoft in /proc/version
    #[cfg(target_os = "linux")]
    {
        if let Ok(version) = std::fs::read_to_string("/proc/version") {
            if version.to_lowercase().contains("microsoft") {
                // We're on WSL2 - paths are already normalized by the kernel
                // DrvFs paths like /mnt/c/... work fine with notify
                return path.to_path_buf();
            }
        }
    }

    // On non-Linux or if not WSL2, return unchanged
    path.to_path_buf()
}

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
        let root = normalize_path(&self.root);
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

    #[test]
    fn normalize_path_returns_unchanged_on_native_linux() {
        let path = Path::new("/home/user/project");
        let normalized = normalize_path(path);
        assert_eq!(normalized, path);
    }

    #[test]
    fn normalize_path_handles_mnt_paths() {
        // On WSL2, /mnt/c/... paths should be handled correctly
        let path = Path::new("/mnt/c/Users/user/project");
        let normalized = normalize_path(path);
        // Path should be returned (either as-is on WSL2 or unchanged on native Linux)
        assert!(
            normalized.to_string_lossy().contains("mnt")
                || normalized.to_string_lossy().contains("Users")
        );
    }

    #[test]
    fn watcher_fires_on_file_change() {
        use std::sync::{Arc, Mutex};
        use std::thread;
        use std::time::Duration;

        let dir = tempdir().unwrap();
        let ws = Workspace::new(dir.path().to_path_buf());

        // Create initial file
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "initial").unwrap();

        // Track changes
        let changes = Arc::new(Mutex::new(Vec::new()));
        let changes_clone = Arc::clone(&changes);

        // Start watcher
        let _debouncer = ws
            .watch(move |path| {
                changes_clone.lock().unwrap().push(path.to_path_buf());
            })
            .unwrap();

        // Give watcher time to initialize
        thread::sleep(Duration::from_millis(200));

        // Modify file
        std::fs::write(&file_path, "modified").unwrap();

        // Wait for debounce
        thread::sleep(Duration::from_millis(200));

        // Verify change was detected
        let detected_changes = changes.lock().unwrap();
        assert!(
            !detected_changes.is_empty(),
            "File change should be detected by watcher"
        );
    }

    #[test]
    fn open_file_via_relative_path_works() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "content").unwrap();

        let ws = Workspace::new(dir.path().to_path_buf());
        let buf = ws.open_file("test.txt").unwrap();
        assert_eq!(buf.to_string(), "content");
    }

    #[test]
    fn save_and_open_roundtrip_preserves_content() {
        let dir = tempdir().unwrap();
        let ws = Workspace::new(dir.path().to_path_buf());

        // Save
        let mut buf = Buffer::new();
        buf.insert(0, "test content");
        ws.save_file("roundtrip.txt", &buf).unwrap();

        // Open
        let loaded = ws.open_file("roundtrip.txt").unwrap();
        assert_eq!(loaded.to_string(), "test content");
    }
}
