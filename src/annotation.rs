//! Byte-preserving Markdown annotation serialization.

use std::{fmt, ops::Range};

use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;

pub(crate) const NOTES_START: &str = "<!-- md-redpen:notes:start v=1 -->";
pub(crate) const NOTES_END: &str = "<!-- md-redpen:notes:end -->";
const NOTE_TITLE_MAX_GRAPHEMES: usize = 24;

/// Stable Markdown-safe annotation identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotationId(String);

impl AnnotationId {
    /// Parses a stable RedPen identifier.
    ///
    /// # Errors
    ///
    /// Returns [`AnnotationError::InvalidId`] for a non-RedPen identifier.
    pub fn parse(value: &str) -> Result<Self, AnnotationError> {
        let suffix = value
            .strip_prefix("rp-")
            .ok_or_else(|| AnnotationError::InvalidId(value.to_owned()))?;
        let valid = !suffix.is_empty()
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
        if !valid {
            return Err(AnnotationError::InvalidId(value.to_owned()));
        }
        Ok(Self(value.to_owned()))
    }

    /// Generates a chronologically sortable identifier.
    #[must_use]
    pub fn generate() -> Self {
        Self(format!("rp-{}", Uuid::now_v7()))
    }
}

impl fmt::Display for AnnotationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// User-visible annotation intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteKind {
    /// Explain selected prose.
    Explanation,
    /// Suggest replacement wording without changing the body.
    Revision,
    /// Add supporting detail.
    Expansion,
    /// Store user-authored context.
    Manual,
}

/// One requested body-link and endnote transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotationRequest<'a> {
    /// Stable annotation identifier.
    pub id: AnnotationId,
    /// Annotation intent.
    pub kind: NoteKind,
    /// Reviewed Markdown note body.
    pub note: &'a str,
    /// Selected source byte range.
    pub selection: Range<usize>,
}

/// Annotation validation or serialization failure.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AnnotationError {
    /// Identifier does not use the RedPen namespace.
    #[error("invalid annotation id: {0}")]
    InvalidId(String),
    /// Selection is empty or outside the document.
    #[error("selection is outside the Markdown source")]
    InvalidSelection,
    /// Selection does not align to UTF-8 boundaries.
    #[error("selection splits a UTF-8 code point")]
    InvalidUtf8Boundary,
    /// Selection crosses a block boundary.
    #[error("selection must stay within one Markdown block")]
    MultilineSelection,
    /// Note has no visible content.
    #[error("note cannot be empty")]
    EmptyNote,
    /// Input contains reserved RedPen management syntax.
    #[error("content contains reserved MD RedPen syntax")]
    ReservedSyntax,
    /// Selection intersects an existing Markdown link.
    #[error("selection cannot overlap an existing Markdown link")]
    ExistingLinkOverlap,
    /// Selection intersects an existing RedPen highlight.
    #[error("selection cannot overlap an existing MD RedPen annotation")]
    ExistingAnnotationOverlap,
    /// Selection intersects inline code.
    #[error("selection cannot overlap inline code")]
    InlineCodeOverlap,
    /// Selection intersects a Markdown image.
    #[error("selection cannot overlap a Markdown image")]
    ImageOverlap,
}

/// Wraps selected body text in a highlighted reference link and appends its
/// managed endnote.
///
/// # Errors
///
/// Returns [`AnnotationError`] if the request cannot be serialized without
/// corrupting the source.
pub fn annotate(source: &str, request: &AnnotationRequest<'_>) -> Result<String, AnnotationError> {
    validate_request(source, request)?;
    let selected = source
        .get(request.selection.clone())
        .ok_or(AnnotationError::InvalidUtf8Boundary)?;
    let highlighted = format!("<mark>[{selected}][{}]</mark>", request.id);

    let mut output =
        String::with_capacity(source.len() + highlighted.len() + request.note.len() + 160);
    output.push_str(&source[..request.selection.start]);
    output.push_str(&highlighted);
    output.push_str(&source[request.selection.end..]);

    let note = format_note(request, selected, next_note_number(source));
    if let Some(notes) = crate::annotation_context::managed_notes_range(&output) {
        output.insert_str(notes.end, &note);
    } else {
        if !output.ends_with('\n') {
            output.push('\n');
        }
        if !output.ends_with("\n\n") {
            output.push('\n');
        }
        output.push_str(NOTES_START);
        output.push_str("\n## MD RedPen Notes\n\n");
        output.push_str(&note);
        output.push_str(NOTES_END);
        output.push('\n');
    }

    Ok(output)
}

fn validate_request(source: &str, request: &AnnotationRequest<'_>) -> Result<(), AnnotationError> {
    let range = &request.selection;
    if range.start >= range.end || range.end > source.len() {
        return Err(AnnotationError::InvalidSelection);
    }
    if !source.is_char_boundary(range.start) || !source.is_char_boundary(range.end) {
        return Err(AnnotationError::InvalidUtf8Boundary);
    }

    let selected = &source[range.clone()];
    if selected.contains(['\n', '\r']) {
        return Err(AnnotationError::MultilineSelection);
    }
    if request.note.trim().is_empty() {
        return Err(AnnotationError::EmptyNote);
    }
    if selected.contains(NOTES_START)
        || selected.contains(NOTES_END)
        || request.note.contains("<!-- md-redpen:")
    {
        return Err(AnnotationError::ReservedSyntax);
    }
    crate::annotation_context::validate_context(source, range)?;
    Ok(())
}

fn format_note(request: &AnnotationRequest<'_>, selected: &str, number: usize) -> String {
    format!(
        "<a id=\"rp-note-{id_suffix}\"></a>\n### {number}) {title}\n\n{note}\n\n[{id}]: #rp-note-{id_suffix}\n",
        id = request.id,
        id_suffix = request.id.0.trim_start_matches("rp-"),
        title = selection_title(selected),
        note = request.note.trim(),
    )
}

fn next_note_number(source: &str) -> usize {
    let Some(range) = crate::annotation_context::managed_notes_range(source) else {
        return 1;
    };
    source
        .get(range)
        .map_or(0, |notes| notes.matches("<a id=\"rp-note-").count())
        .saturating_add(1)
}

fn selection_title(selected: &str) -> String {
    let compact = selected.split_whitespace().collect::<Vec<_>>().join(" ");
    let graphemes = compact.graphemes(true).collect::<Vec<_>>();
    let title = if graphemes.len() > NOTE_TITLE_MAX_GRAPHEMES {
        let mut shortened = graphemes[..NOTE_TITLE_MAX_GRAPHEMES - 1].concat();
        shortened.push('…');
        shortened
    } else {
        compact
    };
    escape_markdown_title(&title)
}

fn escape_markdown_title(title: &str) -> String {
    let mut escaped = String::with_capacity(title.len());
    for character in title.chars() {
        if matches!(
            character,
            '\\' | '`'
                | '*'
                | '_'
                | '{'
                | '}'
                | '['
                | ']'
                | '<'
                | '>'
                | '('
                | ')'
                | '#'
                | '+'
                | '-'
                | '.'
                | '!'
                | '|'
                | '~'
        ) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}
