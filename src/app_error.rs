//! Application-level error boundary.

use thiserror::Error;

use crate::{
    annotation::AnnotationError, codex::CodexError, editor::EditorError, storage::StorageError,
};

/// Operation failure surfaced in the TUI status line.
#[derive(Debug, Error)]
pub enum AppError {
    /// Markdown projection failed.
    #[error(transparent)]
    Editor(#[from] EditorError),
    /// Annotation request was invalid.
    #[error(transparent)]
    Annotation(#[from] AnnotationError),
    /// Document persistence failed.
    #[error(transparent)]
    Storage(#[from] StorageError),
    /// Codex authentication or execution failed.
    #[error(transparent)]
    Codex(#[from] CodexError),
    /// A command required a visual selection.
    #[error("select a word or sentence first")]
    MissingSelection,
}
