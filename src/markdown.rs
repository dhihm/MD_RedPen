//! Projection from rendered Markdown text to source byte ranges.

mod graphemes;

use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use thiserror::Error;

use self::graphemes::{
    project_event, project_lossy_event, push_line_break, push_pending_breaks, push_synthetic,
};

/// Semantic rendering role for one grapheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticKind {
    /// Ordinary Markdown prose.
    Body,
    /// Heading text.
    Heading,
    /// Inline code that is visible but not selectable in v1.
    Code,
    /// Persisted MD RedPen body highlight.
    Annotation,
    /// Ordinary prose inside the managed endnote block.
    ManagedNote,
    /// Managed endnote heading.
    ManagedNoteHeading,
    /// Strong label that introduces one managed endnote.
    EndnoteLabel,
    /// Renderer-created spacing or list marker.
    Synthetic,
}

/// A rendered grapheme that retains its source byte range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayGrapheme {
    text: String,
    source: Range<usize>,
    selectable: bool,
    paragraph: usize,
    semantic: SemanticKind,
    link_target: Option<String>,
}

impl DisplayGrapheme {
    /// Returns the rendered grapheme.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the half-open source byte range.
    #[must_use]
    pub fn source_range(&self) -> Range<usize> {
        self.source.clone()
    }

    /// Reports whether selecting this grapheme is byte-exact.
    #[must_use]
    pub const fn is_selectable(&self) -> bool {
        self.selectable
    }

    /// Returns the rendered Markdown paragraph identifier.
    #[must_use]
    pub const fn paragraph(&self) -> usize {
        self.paragraph
    }

    /// Returns the semantic rendering role.
    #[must_use]
    pub const fn semantic(&self) -> SemanticKind {
        self.semantic
    }

    /// Returns the resolved Markdown link target, if any.
    #[must_use]
    pub fn link_target(&self) -> Option<&str> {
        self.link_target.as_deref()
    }
}

/// Source-backed graphemes in rendered document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projection {
    graphemes: Vec<DisplayGrapheme>,
}

impl Projection {
    /// Returns all rendered graphemes.
    #[must_use]
    pub fn graphemes(&self) -> &[DisplayGrapheme] {
        &self.graphemes
    }
}

/// Failure while projecting rendered Markdown.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProjectionError {
    /// A parser range was not a valid UTF-8 source slice.
    #[error("parser returned invalid source range {start}..{end}")]
    InvalidSourceRange {
        /// Start byte.
        start: usize,
        /// End byte.
        end: usize,
    },
}

/// Projects rendered Markdown text to source-backed graphemes.
///
/// # Errors
///
/// Returns [`ProjectionError::InvalidSourceRange`] if the parser emits a range
/// that cannot index the original UTF-8 source.
pub fn project_text(source: &str) -> Result<Projection, ProjectionError> {
    let options = Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(source, options).into_offset_iter();
    let mut graphemes = Vec::new();
    let mut semantic = SemanticKind::Body;
    let mut marker = false;
    let mut strong = false;
    let mut link_target = None;
    let mut pending_breaks = 0_u8;
    let mut paragraph = 0_usize;
    let managed_notes = crate::annotation_context::managed_notes_range(source);
    let mut notes_started = false;

    for (event, range) in parser {
        let in_managed_notes = managed_notes
            .as_ref()
            .is_some_and(|notes| range.start >= notes.start && range.start < notes.end);
        if in_managed_notes && !notes_started {
            pending_breaks = pending_breaks.max(2);
            notes_started = true;
        }
        match event {
            Event::Start(Tag::Heading { .. }) => semantic = SemanticKind::Heading,
            Event::End(TagEnd::Heading(_)) => {
                semantic = SemanticKind::Body;
                pending_breaks = pending_breaks.max(2);
                paragraph = paragraph.saturating_add(1);
            }
            Event::End(TagEnd::Paragraph) => {
                pending_breaks = pending_breaks.max(1);
                paragraph = paragraph.saturating_add(1);
            }
            Event::End(TagEnd::Item) => {
                pending_breaks = pending_breaks.max(1);
            }
            Event::Start(Tag::Item) => {
                push_pending_breaks(&mut graphemes, &mut pending_breaks, range.start);
                push_synthetic(&mut graphemes, "•", range.start);
                push_synthetic(&mut graphemes, " ", range.start);
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                link_target = Some(dest_url.into_string());
            }
            Event::End(TagEnd::Link) => link_target = None,
            Event::Start(Tag::Strong) => strong = true,
            Event::End(TagEnd::Strong) => strong = false,
            Event::InlineHtml(html) if html.trim().eq_ignore_ascii_case("<mark>") => marker = true,
            Event::InlineHtml(html) if html.trim().eq_ignore_ascii_case("</mark>") => {
                marker = false
            }
            Event::Text(text) => {
                push_pending_breaks(&mut graphemes, &mut pending_breaks, range.start);
                let kind = if marker {
                    SemanticKind::Annotation
                } else if in_managed_notes && strong {
                    SemanticKind::EndnoteLabel
                } else if in_managed_notes {
                    match semantic {
                        SemanticKind::Heading => SemanticKind::ManagedNoteHeading,
                        SemanticKind::Body
                        | SemanticKind::Code
                        | SemanticKind::Annotation
                        | SemanticKind::ManagedNote
                        | SemanticKind::ManagedNoteHeading
                        | SemanticKind::EndnoteLabel
                        | SemanticKind::Synthetic => SemanticKind::ManagedNote,
                    }
                } else {
                    semantic
                };
                project_event(
                    source,
                    text.as_ref(),
                    range,
                    paragraph,
                    kind,
                    link_target.as_deref(),
                    &mut graphemes,
                )?;
            }
            Event::Code(text) => {
                push_pending_breaks(&mut graphemes, &mut pending_breaks, range.start);
                let kind = if in_managed_notes {
                    SemanticKind::ManagedNote
                } else {
                    SemanticKind::Code
                };
                project_lossy_event(
                    text.as_ref(),
                    range,
                    paragraph,
                    kind,
                    link_target.as_deref(),
                    &mut graphemes,
                );
            }
            Event::SoftBreak | Event::HardBreak => {
                push_line_break(&mut graphemes, range);
                pending_breaks = 0;
            }
            Event::Rule => {
                push_pending_breaks(&mut graphemes, &mut pending_breaks, range.start);
                push_synthetic(&mut graphemes, "─", range.start);
                pending_breaks = 1;
            }
            _ => {}
        }
    }

    Ok(Projection { graphemes })
}
