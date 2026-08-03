//! Responsive mode, status, and terminal-size rendering.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    app::App,
    app_state::{Mode, StatusTone},
    theme,
};

pub(super) fn render_status(frame: &mut Frame<'_>, app: &App, area: Rect, compact: bool) {
    let status_style = match app.status_tone() {
        StatusTone::Neutral => Style::default().fg(theme::TEXT_MUTED),
        StatusTone::Success => Style::default().fg(theme::SUCCESS_GREEN),
        StatusTone::Error => Style::default()
            .fg(theme::ERROR_RED)
            .add_modifier(Modifier::BOLD),
    };
    let mode = match app.mode() {
        Mode::Browse => "BROWSE",
        Mode::Visual => "VISUAL",
        Mode::ManualInput => "INPUT",
        Mode::CodexRunning => "CODEX",
        Mode::Review => "REVIEW",
    };
    let message = match app.status_tone() {
        StatusTone::Error => format!("Error: {}", app.status()),
        StatusTone::Neutral | StatusTone::Success if compact => compact_message(app),
        StatusTone::Neutral | StatusTone::Success if app.mode() == Mode::CodexRunning => {
            format!("{} {}", app.codex_spinner(), app.status())
        }
        StatusTone::Neutral | StatusTone::Success => app.status().to_owned(),
    };
    let help = help_text(app.mode(), compact);
    let mut first_line = vec![
        Span::styled(
            format!(" {mode} "),
            Style::default()
                .fg(theme::CANVAS)
                .bg(theme::FOCUS_BLUE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(message, status_style),
    ];
    if compact {
        first_line.push(Span::styled(
            format!("  {help}"),
            Style::default().fg(theme::TEXT_MUTED),
        ));
    }
    let mut lines = vec![Line::from(first_line)];
    if !compact {
        lines.push(Line::from(Span::styled(
            format!(" {help}"),
            Style::default().fg(theme::TEXT_MUTED),
        )));
    }
    let status = Paragraph::new(lines).style(Style::default().bg(theme::CANVAS));
    frame.render_widget(status, area);
}

fn compact_message(app: &App) -> String {
    if app.status_tone() == StatusTone::Success {
        return "저장됨".to_owned();
    }
    match app.mode() {
        Mode::Browse => "준비".to_owned(),
        Mode::Visual => "선택 중".to_owned(),
        Mode::ManualInput => "미주 입력".to_owned(),
        Mode::CodexRunning => format!("{} Codex 작성 중", app.codex_spinner()),
        Mode::Review => "초안 검토".to_owned(),
    }
}

pub(super) fn render_too_small(frame: &mut Frame<'_>, area: Rect) {
    let message = Paragraph::new("Error: terminal must be at least 50x12 · q quits")
        .style(
            Style::default()
                .fg(theme::ERROR_RED)
                .bg(theme::CANVAS)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(message, area);
}

const fn help_text(mode: Mode, compact: bool) -> &'static str {
    match (mode, compact) {
        (Mode::Browse, true) => "↑/↓ paragraph · drag select · q quit",
        (Mode::Visual, true) => "h/l extend · a note · c Codex · Esc",
        (Mode::ManualInput, true) => "Enter save · Esc cancel",
        (Mode::CodexRunning, true) => "Esc cancel",
        (Mode::Review, true) => "Enter save · Esc discard",
        (Mode::Browse, false) => {
            "↑/↓ paragraph  drag select  v/w select  Enter follow  b back  q quit"
        }
        (Mode::Visual, false) => "←/→ or h/l extend  a manual  c Codex  Esc cancel",
        (Mode::ManualInput, false) => "type note  Enter atomic save  Esc cancel",
        (Mode::CodexRunning, false) => "Codex running  Esc cancel",
        (Mode::Review, false) => "edit draft  Enter atomic save  Esc discard",
    }
}
