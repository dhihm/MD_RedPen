//! Grapheme construction helpers for Markdown projection.

use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

use super::{DisplayGrapheme, ProjectionError, SemanticKind};

pub(super) fn project_event(
    source: &str,
    rendered: &str,
    range: Range<usize>,
    paragraph: usize,
    semantic: SemanticKind,
    link_target: Option<&str>,
    output: &mut Vec<DisplayGrapheme>,
) -> Result<(), ProjectionError> {
    let source_slice = source
        .get(range.clone())
        .ok_or(ProjectionError::InvalidSourceRange {
            start: range.start,
            end: range.end,
        })?;

    if source_slice == rendered {
        for (relative_start, grapheme) in rendered.grapheme_indices(true) {
            let start = range.start + relative_start;
            output.push(DisplayGrapheme {
                text: grapheme.to_owned(),
                source: start..start + grapheme.len(),
                selectable: true,
                paragraph,
                semantic,
                link_target: link_target.map(str::to_owned),
            });
        }
    } else {
        project_lossy_event(rendered, range, paragraph, semantic, link_target, output);
    }

    Ok(())
}

pub(super) fn project_lossy_event(
    rendered: &str,
    range: Range<usize>,
    paragraph: usize,
    semantic: SemanticKind,
    link_target: Option<&str>,
    output: &mut Vec<DisplayGrapheme>,
) {
    output.extend(rendered.graphemes(true).map(|grapheme| DisplayGrapheme {
        text: grapheme.to_owned(),
        source: range.clone(),
        selectable: false,
        paragraph,
        semantic,
        link_target: link_target.map(str::to_owned),
    }));
}

pub(super) fn push_pending_breaks(
    output: &mut Vec<DisplayGrapheme>,
    pending: &mut u8,
    source_offset: usize,
) {
    for _ in 0..*pending {
        push_synthetic(output, "\n", source_offset);
    }
    *pending = 0;
}

pub(super) fn push_line_break(output: &mut Vec<DisplayGrapheme>, source: Range<usize>) {
    output.push(DisplayGrapheme {
        text: "\n".to_owned(),
        source,
        selectable: false,
        paragraph: usize::MAX,
        semantic: SemanticKind::Synthetic,
        link_target: None,
    });
}

pub(super) fn push_synthetic(output: &mut Vec<DisplayGrapheme>, text: &str, source_offset: usize) {
    output.push(DisplayGrapheme {
        text: text.to_owned(),
        source: source_offset..source_offset,
        selectable: false,
        paragraph: usize::MAX,
        semantic: SemanticKind::Synthetic,
        link_target: None,
    });
}
