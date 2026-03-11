//! Outbox trait: a write-only sink for structured agent output.
//!
//! The `Outbox` trait accepts items and writes them somewhere. The first
//! implementation is `FolderOutbox`, which writes each item as a file to a
//! configured directory. Future backends (S3, database, etc.) swap in without
//! changing callers.

use std::path::PathBuf;

/// An item written to an outbox. Each item becomes one file in the
/// `FolderOutbox` implementation.
pub struct OutboxItem {
    /// Filename for the item (e.g. `"summary.md"`, `"result.json"`).
    pub name: String,
    /// Content to write.
    pub content: String,
}

/// Error produced by outbox operations.
#[derive(Debug)]
pub enum OutboxError {
    /// Failed to create the outbox directory.
    CreateDir(String),
    /// Failed to write the item file.
    Write(String),
}

impl std::fmt::Display for OutboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateDir(msg) => write!(f, "outbox: create dir: {msg}"),
            Self::Write(msg) => write!(f, "outbox: write: {msg}"),
        }
    }
}

impl std::error::Error for OutboxError {}

/// A sink that accepts structured items and writes them somewhere.
pub trait Outbox {
    /// Write an item to the outbox.
    fn send(&self, item: OutboxItem) -> Result<(), OutboxError>;
}

/// Outbox implementation that writes each item as a file in a directory.
///
/// The directory is created on first write if it does not exist.
pub struct FolderOutbox {
    dir: PathBuf,
}

impl FolderOutbox {
    /// Create a `FolderOutbox` that writes to `dir`.
    #[must_use]
    pub const fn new(dir: PathBuf) -> Self {
        Self { dir }
    }
}

impl Outbox for FolderOutbox {
    fn send(&self, item: OutboxItem) -> Result<(), OutboxError> {
        std::fs::create_dir_all(&self.dir)
            .map_err(|e| OutboxError::CreateDir(format!("{}: {e}", self.dir.display())))?;

        let path = self.dir.join(&item.name);
        std::fs::write(&path, &item.content)
            .map_err(|e| OutboxError::Write(format!("{}: {e}", path.display())))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn folder_outbox_writes_item_as_file() {
        let dir = TempDir::new().unwrap();
        let outbox = FolderOutbox::new(dir.path().to_path_buf());
        outbox
            .send(OutboxItem {
                name: "test-item.md".to_string(),
                content: "# Hello\n".to_string(),
            })
            .unwrap();
        let written = fs::read_to_string(dir.path().join("test-item.md")).unwrap();
        assert_eq!(written, "# Hello\n");
    }

    #[test]
    fn folder_outbox_writes_multiple_items() {
        let dir = TempDir::new().unwrap();
        let outbox = FolderOutbox::new(dir.path().to_path_buf());
        outbox
            .send(OutboxItem {
                name: "a.md".to_string(),
                content: "a".to_string(),
            })
            .unwrap();
        outbox
            .send(OutboxItem {
                name: "b.json".to_string(),
                content: "{}".to_string(),
            })
            .unwrap();
        assert!(dir.path().join("a.md").exists());
        assert!(dir.path().join("b.json").exists());
    }

    #[test]
    fn folder_outbox_creates_dir_if_missing() {
        let dir = TempDir::new().unwrap();
        let outbox_dir = dir.path().join("outbox");
        let outbox = FolderOutbox::new(outbox_dir.clone());
        outbox
            .send(OutboxItem {
                name: "item.txt".to_string(),
                content: "test".to_string(),
            })
            .unwrap();
        assert!(outbox_dir.join("item.txt").exists());
    }

    #[test]
    fn folder_outbox_content_matches_exactly() {
        let dir = TempDir::new().unwrap();
        let outbox = FolderOutbox::new(dir.path().to_path_buf());
        let content = "line 1\nline 2\n";
        outbox
            .send(OutboxItem {
                name: "multi.txt".to_string(),
                content: content.to_string(),
            })
            .unwrap();
        let written = fs::read_to_string(dir.path().join("multi.txt")).unwrap();
        assert_eq!(written, content);
    }

    #[test]
    fn folder_outbox_overwrites_existing_item() {
        let dir = TempDir::new().unwrap();
        let outbox = FolderOutbox::new(dir.path().to_path_buf());
        outbox
            .send(OutboxItem {
                name: "item.txt".to_string(),
                content: "original".to_string(),
            })
            .unwrap();
        outbox
            .send(OutboxItem {
                name: "item.txt".to_string(),
                content: "updated".to_string(),
            })
            .unwrap();
        let written = fs::read_to_string(dir.path().join("item.txt")).unwrap();
        assert_eq!(written, "updated");
    }
}
