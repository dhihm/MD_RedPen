//! Conflict-aware atomic Markdown persistence.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use atomic_write_file::AtomicWriteFile;
use fs2::FileExt;
use thiserror::Error;

/// Markdown bytes captured before an edit starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSnapshot {
    path: PathBuf,
    original: String,
}

impl DocumentSnapshot {
    /// Loads a UTF-8 Markdown document.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] when the file cannot be read or is not UTF-8.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref().to_path_buf();
        let bytes = fs::read(&path).map_err(|error| io_error("read", &path, error))?;
        let original =
            String::from_utf8(bytes).map_err(|_| StorageError::InvalidUtf8(path.clone()))?;
        Ok(Self { path, original })
    }

    /// Returns the source captured for this transaction.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.original
    }

    /// Returns the document path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Atomic storage failure.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum StorageError {
    /// Filesystem operation failed.
    #[error("cannot {operation} {path}: {kind}")]
    Io {
        /// Operation name.
        operation: &'static str,
        /// Affected path.
        path: PathBuf,
        /// Stable platform error category.
        kind: io::ErrorKind,
    },
    /// Markdown source is not valid UTF-8.
    #[error("Markdown document is not valid UTF-8: {0}")]
    InvalidUtf8(PathBuf),
    /// File bytes changed after the snapshot was captured.
    #[error("document changed outside MD RedPen; reload before saving")]
    ExternalChange,
}

/// Atomically replaces the snapshot file if its bytes are still unchanged.
///
/// # Errors
///
/// Returns [`StorageError::ExternalChange`] without writing if another process
/// changed the document. Other failures are returned as [`StorageError::Io`].
pub fn commit(snapshot: &DocumentSnapshot, next_source: &str) -> Result<(), StorageError> {
    let mut locked = open_locked(snapshot.path())?;
    let mut current = String::new();
    locked
        .read_to_string(&mut current)
        .map_err(|error| io_error("read locked", snapshot.path(), error))?;
    if current != snapshot.original {
        return Err(StorageError::ExternalChange);
    }

    let mut atomic = AtomicWriteFile::open(snapshot.path())
        .map_err(|error| io_error("open atomic writer for", snapshot.path(), error))?;
    atomic
        .write_all(next_source.as_bytes())
        .map_err(|error| io_error("write", snapshot.path(), error))?;
    atomic
        .commit()
        .map_err(|error| io_error("commit", snapshot.path(), error))?;
    Ok(())
}

fn open_locked(path: &Path) -> Result<File, StorageError> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|error| io_error("open", path, error))?;
    file.lock_exclusive()
        .map_err(|error| io_error("lock", path, error))?;
    Ok(file)
}

fn io_error(operation: &'static str, path: &Path, error: io::Error) -> StorageError {
    StorageError::Io {
        operation,
        path: path.to_path_buf(),
        kind: error.kind(),
    }
}
