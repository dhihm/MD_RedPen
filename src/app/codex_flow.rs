//! Codex-running and review state transitions.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation;

use super::App;
use crate::{
    annotation::NoteKind,
    app_error::AppError,
    app_state::{CodexAction, Mode, StatusTone},
    codex::CodexRequest,
};

impl App {
    pub(super) fn start_codex(&mut self) -> Result<(), AppError> {
        let (selected, context) = self.codex_request_data()?;
        self.launch_codex(
            CodexAction::Endnote,
            &CodexRequest::explain(selected, context),
            "Codex가 미주 초안을 작성 중…",
        )
    }

    pub(super) fn start_revision_codex(&mut self, instruction: &str) -> Result<(), AppError> {
        let (selected, context) = self.codex_request_data()?;
        self.launch_codex(
            CodexAction::Revision,
            &CodexRequest::revise(selected, context, instruction),
            "Codex가 문장 수정안을 작성 중…",
        )
    }

    fn codex_request_data(&self) -> Result<(String, String), AppError> {
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
        Ok((selected, context))
    }

    fn launch_codex(
        &mut self,
        action: CodexAction,
        request: &CodexRequest,
        status: &str,
    ) -> Result<(), AppError> {
        self.codex_client.check_chatgpt_login()?;
        self.codex_job = Some(self.codex_client.start(request)?);
        self.codex_action = Some(action);
        self.mode = Mode::CodexRunning;
        self.spinner_frame = 0;
        self.status = status.to_owned();
        self.status_tone = StatusTone::Neutral;
        Ok(())
    }

    pub(super) fn handle_codex_running(&mut self, key: KeyEvent) -> Result<(), AppError> {
        if key.code == KeyCode::Esc {
            self.cancel_codex()?;
            self.codex_action = None;
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
                let draft = self.review.trim().to_owned();
                match self.codex_action.ok_or(AppError::MissingSelection)? {
                    CodexAction::Endnote => {
                        self.commit_note(NoteKind::Explanation, &draft)?;
                        self.codex_action = None;
                    }
                    CodexAction::Revision => self.commit_revision(&draft)?,
                }
            }
            KeyCode::Esc => {
                self.review.clear();
                self.codex_action = None;
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
                let Some(action) = self.codex_action else {
                    self.mode = Mode::Visual;
                    self.set_error("Codex result has no pending action");
                    return;
                };
                self.review = note;
                self.mode = Mode::Review;
                self.status = match action {
                    CodexAction::Endnote => "Codex 미주 초안 검토 · Enter 저장 · Esc 폐기",
                    CodexAction::Revision => "Codex 수정안 검토 · Enter 적용 · Esc 폐기",
                }
                .to_owned();
                self.status_tone = StatusTone::Neutral;
            }
            Err(error) => {
                self.codex_action = None;
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
