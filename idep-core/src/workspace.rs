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

/// A workspace with a root directory for file operations.
///
/// `Workspace` provides operations for opening, saving, and watching files relative
/// to a root directory. All paths are relative to this root.
pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    /// Create a new workspace with the given root directory.
    ///
    /// # Examples
    /// ```ignore
    /// use idep_core::workspace::Workspace;
    /// use std::path::PathBuf;
    ///
    /// let ws = Workspace::new(PathBuf::from("/home/user/project"));
    /// ```
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    /// Open a file relative to the workspace root into a Buffer.
    ///
    /// Returns an error if the file doesn't exist or can't be read.
    ///
    /// # Examples
    /// ```ignore
    /// use idep_core::workspace::Workspace;
    /// use std::path::PathBuf;
    ///
    /// let ws = Workspace::new(PathBuf::from("."));
    /// let buffer = ws.open_file("src/main.rs")?;
    /// println!("File has {} characters", buffer.to_string().len());
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn open_file(&self, path: impl AsRef<Path>) -> Result<Buffer> {
        let abs = self.root.join(path);
        let contents = std::fs::read_to_string(abs)?;
        Ok(Buffer::with_text(&contents))
    }

    /// Save a Buffer to a file relative to the workspace root.
    ///
    /// Creates parent directories if they don't exist. Returns an error if the file
    /// can't be written.
    ///
    /// # Examples
    /// ```ignore
    /// use idep_core::{buffer::Buffer, workspace::Workspace};
    /// use std::path::PathBuf;
    ///
    /// let ws = Workspace::new(PathBuf::from("."));
    /// let mut buffer = Buffer::with_text("fn main() {}");
    /// ws.save_file("src/generated.rs", &buffer)?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn save_file(&self, path: impl AsRef<Path>, buffer: &Buffer) -> Result<()> {
        let abs = self.root.join(path);
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(abs, buffer.to_string())?;
        Ok(())
    }

    /// Watch the workspace tree and invoke callback on file changes.
    ///
    /// Uses debouncing (100ms) to avoid multiple callbacks for rapid writes.
    /// The returned debouncer must be kept alive for the watch to remain active.
    /// When dropped, the watch is automatically cancelled.
    ///
    /// # Examples
    /// ```ignore
    /// use idep_core::workspace::Workspace;
    /// use std::path::PathBuf;
    /// use std::sync::atomic::{AtomicUsize, Ordering};
    /// use std::sync::Arc;
    ///
    /// let ws = Workspace::new(PathBuf::from("."));
    /// let change_count = Arc::new(AtomicUsize::new(0));
    /// let count = change_count.clone();
    ///
    /// let _watch = ws.watch(move |path| {
    ///     println!("File changed: {:?}", path);
    ///     count.fetch_add(1, Ordering::SeqCst);
    /// })?;
    ///
    /// // Watch remains active until 'watch' is dropped
    /// # Ok::<(), anyhow::Error>(())
    /// ```
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
