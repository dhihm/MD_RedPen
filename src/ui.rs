//! Ratatui rendering for the reading and note-entry surfaces.

mod document;
mod status;

use std::rc::Rc;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::{app::App, app_state::Mode, theme};

use self::document::render_document;
use self::status::{render_status, render_too_small};

/// Renders one application frame.
pub fn render(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    if area.width < 50 || area.height < 12 {
        render_too_small(frame, area);
        return;
    }

    let editing = is_editing(app);
    let compact = is_compact(area);
    let sections = screen_sections(app, area);

    render_document(frame, app, sections[0]);
    if editing {
        render_input(frame, app, sections[1]);
        render_status(frame, app, sections[2], compact);
    } else {
        render_status(frame, app, sections[1], compact);
    }
}

/// Maps a terminal cell to a selectable rendered document grapheme.
#[must_use]
pub fn document_hit_test(app: &App, area: Rect, column: u16, row: u16) -> Option<usize> {
    if area.width < 50 || area.height < 12 {
        return None;
    }
    document::hit_test(app, screen_sections(app, area)[0], column, row)
}

fn screen_sections(app: &App, area: Rect) -> Rc<[Rect]> {
    let editing = is_editing(app);
    let compact = is_compact(area);
    if editing && compact {
        Layout::vertical([
            Constraint::Percentage(50),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(area)
    } else if editing {
        Layout::vertical([
            Constraint::Min(3),
            Constraint::Length(5),
            Constraint::Length(2),
        ])
        .split(area)
    } else if compact {
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).split(area)
    } else {
        Layout::vertical([Constraint::Min(3), Constraint::Length(2)]).split(area)
    }
}

fn is_editing(app: &App) -> bool {
    matches!(app.mode(), Mode::ManualInput | Mode::Review)
}

const fn is_compact(area: Rect) -> bool {
    area.width < 80 || area.height < 20
}

fn render_input(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let value = format!("{}▌", app.editing_text());
    let title = if app.mode() == Mode::Review {
        " Codex review · Enter save · Esc discard "
    } else {
        " Manual endnote · Enter save · Esc cancel "
    };
    let editor = Paragraph::new(value)
        .style(
            Style::default()
                .fg(theme::TEXT_PRIMARY)
                .bg(theme::NOTE_SURFACE),
        )
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::FOCUS_BLUE)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(editor, area);
}
