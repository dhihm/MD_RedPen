//! Codex-running and review state transitions.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation;

use super::App;
use crate::{
    annotation::NoteKind,
    app_error::AppError,
    app_state::{Mode, StatusTone},
    codex::CodexRequest,
};

impl App {
    pub(super) fn start_codex(&mut self) -> Result<(), AppError> {
        let selection = self
            .editor
            .selection_source_range()
            .ok_or(AppError::MissingSelection)?;
        let selected = self
            .snapshot
            .source()
            .get(selection.clone())
            .ok_or(AppError::MissingSelection)?
            .to_owned();
        let context = line_context(self.snapshot.source(), selection);
        self.codex_client.check_chatgpt_login()?;
        self.codex_job = Some(
            self.codex_client
                .start(&CodexRequest::explain(selected, context))?,
        );
        self.mode = Mode::CodexRunning;
        self.spinner_frame = 0;
        self.status = "Codex가 미주 초안을 작성 중…".to_owned();
        self.status_tone = StatusTone::Neutral;
        Ok(())
    }

    pub(super) fn handle_codex_running(&mut self, key: KeyEvent) -> Result<(), AppError> {
        if key.code == KeyCode::Esc {
            self.cancel_codex()?;
            self.mode = Mode::Visual;
            self.status = "Codex 요청 취소됨 · 선택은 유지됩니다".to_owned();
            self.status_tone = StatusTone::Neutral;
        }
        Ok(())
    }

    pub(super) fn handle_review(&mut self, key: KeyEvent) -> Result<(), AppError> {
        match key.code {
            KeyCode::Enter => {
                if self.review.trim().is_empty() {
                    self.set_error("Codex review cannot be empty");
                    return Ok(());
                }
                let note = self.review.trim().to_owned();
                self.commit_note(NoteKind::Explanation, &note)?;
            }
            KeyCode::Esc => {
                self.review.clear();
                self.mode = Mode::Visual;
                self.status = "Codex 초안 폐기됨 · 선택은 유지됩니다".to_owned();
                self.status_tone = StatusTone::Neutral;
            }
            KeyCode::Backspace => remove_last_grapheme(&mut self.review),
            KeyCode::Char(character) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.review.push(character);
            }
            _ => {}
        }
        Ok(())
    }

    /// Advances a running Codex job without blocking the terminal.
    pub fn tick(&mut self) {
        if self.mode == Mode::CodexRunning {
            self.spinner_frame = (self.spinner_frame + 1) % super::CODEX_SPINNER.len();
        }
        let outcome = self
            .codex_job
            .as_mut()
            .and_then(crate::codex::CodexJob::poll);
        if let Some(result) = outcome {
            self.codex_job.take();
            self.accept_codex_result(result);
        }
    }

    /// Waits for a Codex result through its process-completion signal.
    ///
    /// # Errors
    ///
    /// Returns [`AppError`] if no job is active or Codex fails.
    pub fn wait_for_codex(&mut self) -> Result<(), AppError> {
        let job = self.codex_job.take().ok_or(AppError::MissingSelection)?;
        let note = job.wait()?;
        self.accept_codex_result(Ok(note));
        Ok(())
    }

    /// Terminates any active Codex process before terminal teardown.
    ///
    /// # Errors
    ///
    /// Returns [`AppError`] if the process group cannot be cleaned up.
    pub fn shutdown(&mut self) -> Result<(), AppError> {
        if let Some(job) = self.codex_job.take() {
            job.cancel()?;
        }
        Ok(())
    }

    fn cancel_codex(&mut self) -> Result<(), AppError> {
        if let Some(job) = self.codex_job.take() {
            job.cancel()?;
        }
        Ok(())
    }

    fn accept_codex_result(&mut self, result: Result<String, crate::codex::CodexError>) {
        match result {
            Ok(note) => {
                self.review = note;
                self.mode = Mode::Review;
                self.status = "Codex 초안 검토 · Enter 저장 · Esc 폐기".to_owned();
                self.status_tone = StatusTone::Neutral;
            }
            Err(error) => {
                self.mode = Mode::Visual;
                self.status = error.to_string();
                self.status_tone = StatusTone::Error;
            }
        }
    }
}

fn line_context(source: &str, selection: std::ops::Range<usize>) -> String {
    let start = source[..selection.start]
        .rfind('\n')
        .map_or(0, |index| index.saturating_add(1));
    let end = source[selection.end..]
        .find('\n')
        .map_or(source.len(), |index| selection.end + index);
    source[start..end].to_owned()
}

fn remove_last_grapheme(input: &mut String) {
    if let Some((index, _)) = input.grapheme_indices(true).next_back() {
        input.truncate(index);
    }
}
