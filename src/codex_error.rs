//! Typed Codex process failures.

use std::{io, path::PathBuf, time::Duration};

use thiserror::Error;

/// Codex authentication, execution, or output failure.
#[derive(Debug, Error)]
pub enum CodexError {
    /// Filesystem or process operation failed.
    #[error("cannot {operation} {path}: {kind}")]
    Io {
        /// Operation name.
        operation: &'static str,
        /// Executable, directory, or stream label.
        path: PathBuf,
        /// Stable platform error category.
        kind: io::ErrorKind,
    },
    /// Codex is not authenticated through ChatGPT.
    #[error("Codex must be logged in using ChatGPT: {0}")]
    NotChatGptLogin(String),
    /// A piped process stream was unexpectedly unavailable.
    #[error("Codex process did not expose piped {0}")]
    MissingPipe(&'static str),
    /// Codex exited unsuccessfully.
    #[error("Codex exited with code {code:?}: {stderr}")]
    Exit {
        /// Platform exit code.
        code: Option<i32>,
        /// Bounded stderr text.
        stderr: String,
    },
    /// Successful process returned no note.
    #[error("Codex returned an empty note")]
    EmptyOutput,
    /// A bounded process stream exceeded its contract.
    #[error("Codex {stream} exceeded {limit} bytes")]
    OutputTooLarge {
        /// Stream label.
        stream: &'static str,
        /// Maximum bytes.
        limit: usize,
    },
    /// Process output was not UTF-8.
    #[error("Codex {0} was not valid UTF-8")]
    InvalidUtf8(&'static str),
    /// Process exceeded its deadline.
    #[error("Codex exceeded the {0:?} timeout")]
    Timeout(Duration),
    /// Reader worker terminated unexpectedly.
    #[error("Codex {0} reader thread terminated unexpectedly")]
    ReaderThread(&'static str),
    /// Primary operation and process cleanup both failed.
    #[error("Codex operation failed: {primary}; cleanup also failed: {cleanup}")]
    OperationAndCleanup {
        /// Primary failure.
        primary: String,
        /// Cleanup failure.
        cleanup: String,
    },
}

pub(crate) fn io_error(
    operation: &'static str,
    path: impl Into<PathBuf>,
    error: io::Error,
) -> CodexError {
    CodexError::Io {
        operation,
        path: path.into(),
        kind: error.kind(),
    }
}
