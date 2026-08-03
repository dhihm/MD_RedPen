//! Parser-aware selection exclusion ranges.

use std::ops::Range;

use pulldown_cmark::{Event, Parser, Tag, TagEnd};

use crate::annotation::{AnnotationError, NOTES_END, NOTES_START};

pub(crate) fn managed_notes_range(source: &str) -> Option<Range<usize>> {
    let mut start = None;
    for (event, range) in Parser::new(source).into_offset_iter() {
        let html = match event {
            Event::Html(html) | Event::InlineHtml(html) => html,
            _ => continue,
        };
        if html.trim() == NOTES_START {
            start = Some(range.start);
        } else if html.trim() == NOTES_END
            && let Some(start) = start
        {
            return Some(start..range.start);
        }
    }
    None
}

pub(crate) fn validate_context(
    source: &str,
    selection: &Range<usize>,
) -> Result<(), AnnotationError> {
    let mut link_depth = 0_u32;
    let mut image_start = None;
    let mut mark_start = None;
    let mut link_ranges = Vec::new();
    let mut image_ranges = Vec::new();
    let mut mark_ranges = Vec::new();
    let mut code_ranges = Vec::new();

    for (event, range) in Parser::new(source).into_offset_iter() {
        match event {
            Event::Start(Tag::Link { .. }) => link_depth = link_depth.saturating_add(1),
            Event::End(TagEnd::Link) => link_depth = link_depth.saturating_sub(1),
            Event::Start(Tag::Image { .. }) => image_start = Some(range.start),
            Event::End(TagEnd::Image) => {
                if let Some(start) = image_start.take() {
                    image_ranges.push(start..range.end);
                }
            }
            Event::Text(_) if link_depth > 0 => link_ranges.push(range),
            Event::Code(_) => code_ranges.push(range),
            Event::InlineHtml(html) if html.trim().eq_ignore_ascii_case("<mark>") => {
                mark_start = Some(range.start);
            }
            Event::InlineHtml(html) if html.trim().eq_ignore_ascii_case("</mark>") => {
                if let Some(start) = mark_start.take() {
                    mark_ranges.push(start..range.end);
                }
            }
            _ => {}
        }
    }

    if managed_notes_range(source).is_some_and(|notes| selection.end > notes.start) {
        return Err(AnnotationError::ExistingAnnotationOverlap);
    }
    if overlaps_any(&mark_ranges, selection) {
        return Err(AnnotationError::ExistingAnnotationOverlap);
    }
    if overlaps_any(&image_ranges, selection) {
        return Err(AnnotationError::ImageOverlap);
    }
    if overlaps_any(&code_ranges, selection) {
        return Err(AnnotationError::InlineCodeOverlap);
    }
    if overlaps_any(&link_ranges, selection) {
        return Err(AnnotationError::ExistingLinkOverlap);
    }
    Ok(())
}

fn overlaps_any(ranges: &[Range<usize>], selection: &Range<usize>) -> bool {
    ranges
        .iter()
        .any(|range| range.start < selection.end && selection.start < range.end)
}
