//! Inbox trait: poll for new items, yielding structured messages.
//!
//! The first implementation is [`FolderInbox`], which monitors a directory
//! for new files. Files are moved to a `.processed/` subdirectory after
//! being read to prevent re-delivery on subsequent polls.

use std::path::{Path, PathBuf};

/// An item returned from an inbox poll.
#[derive(Debug, Clone)]
pub struct InboxItem {
    content: String,
    source: String,
}

impl InboxItem {
    /// The text content of this item.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// The source identifier for this item (e.g. filename).
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
}

/// Error type for inbox operations.
#[derive(Debug)]
pub struct InboxError(pub String);

impl std::fmt::Display for InboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "inbox error: {}", self.0)
    }
}

impl std::error::Error for InboxError {}

impl From<std::io::Error> for InboxError {
    fn from(e: std::io::Error) -> Self {
        Self(e.to_string())
    }
}

/// Poll an inbox for new items.
///
/// Implementations must ensure that items returned by one call to `poll()`
/// are not returned again by subsequent calls.
pub trait Inbox {
    /// Drain any available items from the inbox.
    ///
    /// Non-destructive with respect to already-processed items: only new
    /// items are returned. Returns an empty vec when nothing is pending.
    fn poll(&mut self) -> Result<Vec<InboxItem>, InboxError>;
}

/// Folder-based inbox implementation.
///
/// Watches a directory for new files. On each [`poll()`][`Inbox::poll`],
/// all top-level regular files in the directory are read and moved to
/// `.processed/` to prevent re-delivery.
pub struct FolderInbox {
    dir: PathBuf,
}

impl FolderInbox {
    /// Create a new `FolderInbox` watching `dir`.
    ///
    /// The `.processed/` subdirectory is created lazily on the first
    /// successful poll that finds files.
    #[must_use]
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
        }
    }
}

impl Inbox for FolderInbox {
    fn poll(&mut self) -> Result<Vec<InboxItem>, InboxError> {
        let mut items = Vec::new();

        let entries = std::fs::read_dir(&self.dir)?;

        for entry in entries {
            let entry = entry?;
            let file_type = entry.file_type()?;

            if !file_type.is_file() {
                continue;
            }

            let path = entry.path();
            let content = std::fs::read_to_string(&path)?;
            let source = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let processed_dir = self.dir.join(".processed");
            std::fs::create_dir_all(&processed_dir)?;
            std::fs::rename(&path, processed_dir.join(&source))?;

            items.push(InboxItem { content, source });
        }

        Ok(items)
    }
}
