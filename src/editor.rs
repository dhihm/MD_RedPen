//! Rendered-grapheme cursor and visual-selection state.

mod navigation;

use std::ops::Range;

use thiserror::Error;

use crate::markdown::{Projection, ProjectionError, project_text};

/// Selection state over one Markdown source.
#[derive(Debug, Clone)]
pub struct Editor {
    source: String,
    projection: Projection,
    cursor: usize,
    anchor: Option<usize>,
}

impl Editor {
    /// Creates an editor at the first selectable grapheme.
    ///
    /// # Errors
    ///
    /// Returns [`EditorError`] when the Markdown projection is invalid.
    pub fn new(source: impl Into<String>) -> Result<Self, EditorError> {
        let source = source.into();
        let projection = project_text(&source)?;
        let cursor = projection
            .graphemes()
            .iter()
            .position(crate::markdown::DisplayGrapheme::is_selectable)
            .unwrap_or_default();
        Ok(Self {
            source,
            projection,
            cursor,
            anchor: None,
        })
    }

    /// Starts visual selection at the cursor.
    pub const fn start_visual(&mut self) {
        self.anchor = Some(self.cursor);
    }

    /// Clears the visual selection.
    pub const fn clear_visual(&mut self) {
        self.anchor = None;
    }

    /// Starts visual selection at a selectable rendered grapheme.
    pub fn start_visual_at(&mut self, index: usize) -> bool {
        if !self.set_cursor(index) {
            return false;
        }
        self.anchor = Some(index);
        true
    }

    /// Extends visual selection to a selectable rendered grapheme.
    pub fn extend_visual_to(&mut self, index: usize) -> bool {
        self.anchor.is_some() && self.set_cursor(index)
    }

    /// Returns selected source text.
    #[must_use]
    pub fn selected_text(&self) -> Option<&str> {
        self.selection_source_range()
            .and_then(|range| self.source.get(range))
    }

    /// Returns the exact selected source byte range.
    #[must_use]
    pub fn selection_source_range(&self) -> Option<Range<usize>> {
        let indices = self.selection_indices()?;
        let graphemes = self.projection.graphemes();
        let first = graphemes.get(indices.start)?;
        let last = graphemes.get(indices.end.checked_sub(1)?)?;
        Some(first.source_range().start..last.source_range().end)
    }

    /// Reports whether a rendered index is selected.
    #[must_use]
    pub fn is_selected(&self, index: usize) -> bool {
        self.selection_indices()
            .is_some_and(|range| range.contains(&index))
    }

    /// Returns the rendered projection.
    #[must_use]
    pub const fn projection(&self) -> &Projection {
        &self.projection
    }

    /// Returns the cursor's rendered index.
    #[must_use]
    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    /// Returns the current annotation link target.
    #[must_use]
    pub fn current_link_target(&self) -> Option<&str> {
        self.projection
            .graphemes()
            .get(self.cursor)
            .and_then(crate::markdown::DisplayGrapheme::link_target)
    }

    /// Moves to the first selectable grapheme at or after a source offset.
    pub fn jump_to_source(&mut self, source_offset: usize) {
        if let Some(index) = self
            .projection
            .graphemes()
            .iter()
            .position(|item| item.is_selectable() && item.source_range().start >= source_offset)
        {
            self.cursor = index;
        }
    }

    /// Sets a previously captured rendered cursor.
    pub fn restore_cursor(&mut self, cursor: usize) {
        if self
            .projection
            .graphemes()
            .get(cursor)
            .is_some_and(crate::markdown::DisplayGrapheme::is_selectable)
        {
            self.cursor = cursor;
        }
    }

    fn selection_indices(&self) -> Option<Range<usize>> {
        let anchor = self.anchor?;
        let start = anchor.min(self.cursor);
        let end = anchor.max(self.cursor).saturating_add(1);
        Some(start..end)
    }

    fn set_cursor(&mut self, index: usize) -> bool {
        if self
            .projection
            .graphemes()
            .get(index)
            .is_some_and(crate::markdown::DisplayGrapheme::is_selectable)
        {
            self.cursor = index;
            true
        } else {
            false
        }
    }
}

/// Editor projection failure.
#[derive(Debug, Error)]
pub enum EditorError {
    /// Markdown projection failed.
    #[error(transparent)]
    Projection(#[from] ProjectionError),
}
