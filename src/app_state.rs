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
    /// Choosing between Codex revision and endnote generation.
    CodexChoice,
    /// Editing the instruction for a Codex sentence revision.
    RevisionInput,
    /// Waiting for a cancellable Codex process.
    CodexRunning,
    /// Editing the returned Codex note before commit.
    Review,
}

/// The pending Codex result's persistence behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodexAction {
    /// Add the result as a linked endnote.
    Endnote,
    /// Replace the selected source with the reviewed result.
    Revision,
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
