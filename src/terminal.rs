//! Terminal lifecycle and event loop.

use std::{
    io::{self, Stdout},
    time::Duration,
};

use crossterm::{
    cursor,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use thiserror::Error;

use crate::{app::App, app_error::AppError, storage::DocumentSnapshot, ui};

type TuiTerminal = Terminal<CrosstermBackend<Stdout>>;

/// Runs the interactive terminal application.
///
/// # Errors
///
/// Returns [`TuiError`] when setup, rendering, input, application state, or
/// terminal restoration fails.
pub fn run(snapshot: DocumentSnapshot) -> Result<(), TuiError> {
    let mut app = App::new(snapshot)?;
    let mut terminal = open_terminal()?;
    let run_result = event_loop(&mut terminal, &mut app);
    let app_cleanup = app.shutdown();
    let cleanup_result = close_terminal(&mut terminal);

    let mut errors = Vec::new();
    if let Err(error) = run_result {
        errors.push(error.to_string());
    }
    if let Err(error) = app_cleanup {
        errors.push(error.to_string());
    }
    if let Err(error) = cleanup_result {
        errors.push(error.to_string());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(TuiError::CleanupFailures(errors.join("; ")))
    }
}

fn event_loop(terminal: &mut TuiTerminal, app: &mut App) -> io::Result<()> {
    while !app.should_quit() {
        app.tick();
        terminal.draw(|frame| ui::render(frame, app))?;
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        match event::read()? {
            Event::Key(key) if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
                app.handle_key(key);
            }
            Event::Mouse(mouse) => {
                let size = terminal.size()?;
                let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);
                let hit = ui::document_hit_test(app, area, mouse.column, mouse.row);
                app.handle_mouse(mouse.kind, hit);
            }
            Event::FocusGained
            | Event::FocusLost
            | Event::Key(_)
            | Event::Paste(_)
            | Event::Resize(_, _) => {}
        }
    }
    Ok(())
}

fn open_terminal() -> Result<TuiTerminal, TuiError> {
    enable_raw_mode().map_err(TuiError::Io)?;
    let mut stdout = io::stdout();
    if let Err(error) = execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        cursor::Hide
    ) {
        return Err(cleanup_setup_error(error));
    }
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).map_err(cleanup_setup_error)
}

fn cleanup_setup_error(primary: io::Error) -> TuiError {
    let mut cleanup_errors = Vec::new();
    if let Err(error) = disable_raw_mode() {
        cleanup_errors.push(error.to_string());
    }
    let mut stdout = io::stdout();
    if let Err(error) = execute!(
        stdout,
        DisableMouseCapture,
        LeaveAlternateScreen,
        cursor::Show
    ) {
        cleanup_errors.push(error.to_string());
    }
    if cleanup_errors.is_empty() {
        TuiError::Io(primary)
    } else {
        TuiError::RunAndCleanup {
            run: primary.to_string(),
            cleanup: cleanup_errors.join("; "),
        }
    }
}

fn close_terminal(terminal: &mut TuiTerminal) -> io::Result<()> {
    let mut errors = Vec::new();
    if let Err(error) = disable_raw_mode() {
        errors.push(error.to_string());
    }
    if let Err(error) = execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen,
        cursor::Show
    ) {
        errors.push(error.to_string());
    }
    if let Err(error) = terminal.show_cursor() {
        errors.push(error.to_string());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(io::Error::other(errors.join("; ")))
    }
}

/// Terminal application failure.
#[derive(Debug, Error)]
pub enum TuiError {
    /// Application state failed before the event loop.
    #[error(transparent)]
    App(#[from] AppError),
    /// Terminal I/O failed.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// Runtime and terminal cleanup both failed.
    #[error("TUI failed: {run}; terminal cleanup also failed: {cleanup}")]
    RunAndCleanup {
        /// Runtime failure.
        run: String,
        /// Cleanup failure.
        cleanup: String,
    },
    /// One or more runtime and cleanup operations failed.
    #[error("TUI shutdown failures: {0}")]
    CleanupFailures(String),
}
