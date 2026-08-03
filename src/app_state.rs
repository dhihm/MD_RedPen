//! User-visible TUI state labels.

/// User-visible application mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Reading and navigating.
    Browse,
    /// Extending a source-backed selection.
    Visual,
    /// Editing a manual endnote.
    ManualInput,
    /// Waiting for a cancellable Codex process.
    CodexRunning,
    /// Editing the returned Codex note before commit.
    Review,
}

/// Semantic status color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusTone {
    /// Neutral guidance.
    Neutral,
    /// Successful persistence.
    Success,
    /// Rejected or failed operation.
    Error,
}
