//! Application state and keyboard commands.

mod codex_flow;
mod manual_flow;
mod mouse;
mod navigation;
mod revision_flow;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    app_error::AppError,
    app_state::{CodexAction, Mode, StatusTone},
    codex::{CodexClient, CodexJob},
    editor::Editor,
    storage::DocumentSnapshot,
};

const CODEX_SPINNER: [char; 8] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];

/// Mutable state for one open document.
pub struct App {
    snapshot: DocumentSnapshot,
    editor: Editor,
    mode: Mode,
    input: String,
    status: String,
    status_tone: StatusTone,
    return_cursor: Option<usize>,
    should_quit: bool,
    no_color: bool,
    codex_client: CodexClient,
    codex_job: Option<CodexJob>,
    codex_action: Option<CodexAction>,
    spinner_frame: usize,
    review: String,
    mouse_selecting: bool,
    viewport_scroll: Option<u16>,
}

impl App {
    /// Opens application state from a captured document snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`AppError`] if the Markdown cannot be projected.
    pub fn new(snapshot: DocumentSnapshot) -> Result<Self, AppError> {
        Self::with_codex(snapshot, CodexClient::system()?)
    }

    /// Opens state with an injected Codex executable configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AppError`] if the Markdown cannot be projected.
    pub fn with_codex(
        snapshot: DocumentSnapshot,
        codex_client: CodexClient,
    ) -> Result<Self, AppError> {
        let editor = Editor::new(snapshot.source().to_owned())?;
        Ok(Self {
            snapshot,
            editor,
            mode: Mode::Browse,
            input: String::new(),
            status: "v로 선택을 시작하세요".to_owned(),
            status_tone: StatusTone::Neutral,
            return_cursor: None,
            should_quit: false,
            no_color: std::env::var_os("NO_COLOR").is_some(),
            codex_client,
            codex_job: None,
            codex_action: None,
            spinner_frame: 0,
            review: String::new(),
            mouse_selecting: false,
            viewport_scroll: None,
        })
    }

    /// Applies one terminal key event.
    pub fn handle_key(&mut self, key: KeyEvent) {
        self.mouse_selecting = false;
        self.viewport_scroll = None;
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }
        let result = match self.mode {
            Mode::Browse => self.handle_browse(key),
            Mode::Visual => self.handle_visual(key),
            Mode::ManualInput => self.handle_manual_input(key),
            Mode::CodexChoice => self.handle_codex_choice(key),
            Mode::RevisionInput => self.handle_revision_input(key),
            Mode::CodexRunning => self.handle_codex_running(key),
            Mode::Review => self.handle_review(key),
        };
        if let Err(error) = result {
            self.status = error.to_string();
            self.status_tone = StatusTone::Error;
        }
    }

    /// Returns document selection/rendering state.
    #[must_use]
    pub const fn editor(&self) -> &Editor {
        &self.editor
    }

    pub(crate) const fn viewport_scroll(&self) -> Option<u16> {
        self.viewport_scroll
    }

    /// Returns the current mode.
    #[must_use]
    pub const fn mode(&self) -> Mode {
        self.mode
    }

    /// Returns editable note input.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }

    /// Returns the editable reviewed Codex note.
    #[must_use]
    pub fn review(&self) -> &str {
        &self.review
    }

    pub(crate) fn codex_is_revision(&self) -> bool {
        self.codex_action == Some(CodexAction::Revision)
    }

    /// Returns the text currently displayed in the editor panel.
    #[must_use]
    pub fn editing_text(&self) -> &str {
        if self.mode == Mode::Review {
            &self.review
        } else {
            &self.input
        }
    }

    /// Returns the latest user-visible status.
    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }

    /// Returns status semantics.
    #[must_use]
    pub const fn status_tone(&self) -> StatusTone {
        self.status_tone
    }

    /// Returns the opened path for the viewport title.
    #[must_use]
    pub fn path(&self) -> &std::path::Path {
        self.snapshot.path()
    }

    /// Reports whether the event loop should exit.
    #[must_use]
    pub const fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Reports whether semantic colors are disabled.
    #[must_use]
    pub const fn no_color(&self) -> bool {
        self.no_color
    }

    /// Returns the deterministic Codex progress frame.
    #[must_use]
    pub const fn codex_spinner(&self) -> char {
        CODEX_SPINNER[self.spinner_frame]
    }

    /// Reports whether the cursor is at a followed endnote destination.
    #[must_use]
    pub const fn endnote_is_focused(&self) -> bool {
        self.return_cursor.is_some()
    }

    fn set_neutral(&mut self, message: &str) {
        self.status = message.to_owned();
        self.status_tone = StatusTone::Neutral;
    }

    fn set_error(&mut self, message: &str) {
        self.status = message.to_owned();
        self.status_tone = StatusTone::Error;
    }
}
