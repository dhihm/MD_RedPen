//! Document rendering, cursor-follow scrolling, and screen hit testing.

use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use unicode_width::UnicodeWidthStr;

use crate::{app::App, markdown::SemanticKind, theme};

pub(super) fn render_document(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let block = document_block(app);
    if app.editor().projection().graphemes().is_empty() {
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let centered = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(2),
            Constraint::Fill(1),
        ])
        .split(inner);
        let message = Paragraph::new("Empty Markdown document\nv select · q quit")
            .alignment(Alignment::Center)
            .style(Style::default().bg(theme::CANVAS).fg(theme::TEXT_MUTED));
        frame.render_widget(message, centered[1]);
        return;
    }

    let inner = block.inner(area);
    let paragraph = Paragraph::new(document_lines(app, None, display_style(app)))
        .style(Style::default().bg(theme::CANVAS).fg(theme::TEXT_PRIMARY))
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset(app, inner), 0));
    frame.render_widget(paragraph, area);
}

/// Maps a terminal cell inside the document viewport to a selectable grapheme.
#[must_use]
pub(super) fn hit_test(app: &App, area: Rect, column: u16, row: u16) -> Option<usize> {
    let inner = document_block(app).inner(area);
    if column < inner.x
        || column >= inner.x.saturating_add(inner.width)
        || row < inner.y
        || row >= inner.y.saturating_add(inner.height)
    {
        return None;
    }

    let lines = document_lines(app, None, |index, _| encoded_style(index));
    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset(app, inner), 0));
    let mut buffer = Buffer::empty(inner);
    paragraph.render(inner, &mut buffer);
    let offset =
        usize::from(row - inner.y) * usize::from(inner.width) + usize::from(column - inner.x);
    if let Some(index) = decode_hit(app, buffer.content().get(offset)?) {
        return Some(index);
    }
    let previous = offset.checked_sub(1)?;
    if column == inner.x {
        return None;
    }
    let index = decode_hit(app, buffer.content().get(previous)?)?;
    let grapheme = app.editor().projection().graphemes().get(index)?;
    (UnicodeWidthStr::width(grapheme.text()) > 1).then_some(index)
}

pub(super) fn scroll_bounds(app: &App, area: Rect) -> (u16, u16) {
    let inner = document_block(app).inner(area);
    let maximum = max_scroll_offset(app, inner);
    (scroll_offset(app, inner).min(maximum), maximum)
}

fn decode_hit(app: &App, cell: &ratatui::buffer::Cell) -> Option<usize> {
    let Color::Rgb(red, green, blue) = cell.fg else {
        return None;
    };
    let encoded = (u32::from(red) << 16) | (u32::from(green) << 8) | u32::from(blue);
    let index = usize::try_from(encoded.checked_sub(1)?).ok()?;
    app.editor()
        .projection()
        .graphemes()
        .get(index)
        .filter(|item| item.is_selectable())
        .map(|_| index)
}

fn document_block(app: &App) -> Block<'static> {
    Block::default()
        .title(format!(" MD RedPen · {} ", app.path().display()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER_QUIET))
        .style(Style::default().bg(theme::CANVAS).fg(theme::TEXT_PRIMARY))
}

fn scroll_offset(app: &App, inner: Rect) -> u16 {
    if let Some(offset) = app.viewport_scroll() {
        return offset.min(max_scroll_offset(app, inner));
    }
    let end = app.editor().cursor().saturating_add(1);
    let lines = document_lines(app, Some(end), |_, _| Style::default());
    let rows = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .line_count(inner.width);
    let offset = rows.saturating_sub(usize::from(inner.height));
    u16::try_from(offset).unwrap_or(u16::MAX)
}

fn max_scroll_offset(app: &App, inner: Rect) -> u16 {
    let lines = document_lines(app, None, |_, _| Style::default());
    let rows = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .line_count(inner.width);
    let offset = rows.saturating_sub(usize::from(inner.height));
    u16::try_from(offset).unwrap_or(u16::MAX)
}

fn document_lines<F>(app: &App, end: Option<usize>, mut style_for: F) -> Vec<Line<'static>>
where
    F: FnMut(usize, SemanticKind) -> Style,
{
    let graphemes = app.editor().projection().graphemes();
    let end = end.unwrap_or(graphemes.len()).min(graphemes.len());
    let mut lines = Vec::new();
    let mut spans = Vec::new();
    let mut managed_note_line = false;
    for (index, grapheme) in graphemes[..end].iter().enumerate() {
        if grapheme.text() == "\n" {
            lines.push(document_line(std::mem::take(&mut spans), managed_note_line));
            managed_note_line = false;
            continue;
        }
        managed_note_line |= is_managed_note(grapheme.semantic());
        spans.push(Span::styled(
            grapheme.text().to_owned(),
            style_for(index, grapheme.semantic()),
        ));
    }
    if !spans.is_empty() || lines.is_empty() {
        lines.push(document_line(spans, managed_note_line));
    }
    lines
}

fn display_style(app: &App) -> impl FnMut(usize, SemanticKind) -> Style + '_ {
    move |index, semantic| {
        let mut style = semantic_style(semantic, app.no_color());
        if app.editor().is_selected(index) {
            style = if app.no_color() {
                Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
            } else {
                Style::default()
                    .bg(theme::SELECTION_BLUE)
                    .fg(theme::SELECTION_TEXT)
            };
        }
        if app.editor().cursor() == index {
            style = if app.endnote_is_focused() && !app.no_color() {
                style
                    .fg(theme::FOCUS_BLUE)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                style.add_modifier(Modifier::UNDERLINED)
            };
        }
        style
    }
}

fn encoded_style(index: usize) -> Style {
    let Some(encoded) = u32::try_from(index)
        .ok()
        .and_then(|value| value.checked_add(1))
        .filter(|value| *value <= 0x00ff_ffff)
    else {
        return Style::default();
    };
    Style::default().fg(Color::Rgb(
        u8::try_from((encoded >> 16) & 0xff).unwrap_or_default(),
        u8::try_from((encoded >> 8) & 0xff).unwrap_or_default(),
        u8::try_from(encoded & 0xff).unwrap_or_default(),
    ))
}

fn document_line(spans: Vec<Span<'static>>, managed_note: bool) -> Line<'static> {
    let line = Line::from(spans);
    if managed_note {
        line.style(Style::default().bg(theme::NOTE_SURFACE))
    } else {
        line
    }
}

const fn is_managed_note(kind: SemanticKind) -> bool {
    matches!(
        kind,
        SemanticKind::ManagedNote | SemanticKind::ManagedNoteHeading | SemanticKind::EndnoteLabel
    )
}

fn semantic_style(kind: SemanticKind, no_color: bool) -> Style {
    if no_color {
        return match kind {
            SemanticKind::Heading
            | SemanticKind::ManagedNoteHeading
            | SemanticKind::EndnoteLabel => Style::default().add_modifier(Modifier::BOLD),
            SemanticKind::Annotation => Style::default().add_modifier(Modifier::REVERSED),
            SemanticKind::Code => Style::default().add_modifier(Modifier::DIM),
            SemanticKind::Body | SemanticKind::ManagedNote | SemanticKind::Synthetic => {
                Style::default()
            }
        };
    }
    match kind {
        SemanticKind::Body | SemanticKind::Synthetic => {
            Style::default().fg(theme::TEXT_PRIMARY).bg(theme::CANVAS)
        }
        SemanticKind::Heading => Style::default()
            .fg(theme::TEXT_PRIMARY)
            .bg(theme::CANVAS)
            .add_modifier(Modifier::BOLD),
        SemanticKind::Code => Style::default()
            .fg(theme::TEXT_PRIMARY)
            .bg(theme::NOTE_SURFACE),
        SemanticKind::Annotation => Style::default()
            .fg(theme::MARKER_TEXT)
            .bg(theme::MARKER_YELLOW),
        SemanticKind::ManagedNote => Style::default()
            .fg(theme::TEXT_PRIMARY)
            .bg(theme::NOTE_SURFACE),
        SemanticKind::ManagedNoteHeading => Style::default()
            .fg(theme::TEXT_PRIMARY)
            .bg(theme::NOTE_SURFACE)
            .add_modifier(Modifier::BOLD),
        SemanticKind::EndnoteLabel => Style::default()
            .fg(theme::MARKER_YELLOW)
            .bg(theme::NOTE_SURFACE)
            .add_modifier(Modifier::BOLD),
    }
}
