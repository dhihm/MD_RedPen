//! Codex action choice and revision-instruction entry.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation;

use super::App;
use crate::{
    app_error::AppError,
    app_state::{Mode, StatusTone},
    editor::Editor,
    storage::{DocumentSnapshot, commit},
};

impl App {
    pub(super) fn handle_codex_choice(&mut self, key: KeyEvent) -> Result<(), AppError> {
        match key.code {
            KeyCode::Char('r') => {
                self.input.clear();
                self.mode = Mode::RevisionInput;
                self.set_neutral("문장 수정 지시를 입력하고 Enter로 Codex에 전송하세요");
            }
            KeyCode::Char('e') => self.start_codex()?,
            KeyCode::Esc => {
                self.mode = Mode::Visual;
                self.set_neutral("Codex 작업 선택 취소됨 · 선택은 유지됩니다");
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn handle_revision_input(&mut self, key: KeyEvent) -> Result<(), AppError> {
        match key.code {
            KeyCode::Enter => {
                if self.input.trim().is_empty() {
                    self.set_error("Revision instruction cannot be empty");
                } else {
                    let instruction = self.input.trim().to_owned();
                    self.start_revision_codex(&instruction)?;
                    self.input.clear();
                }
            }
            KeyCode::Esc => {
                self.input.clear();
                self.mode = Mode::Visual;
                self.set_neutral("문장 수정 취소됨 · 선택은 유지됩니다");
            }
            KeyCode::Backspace => remove_last_grapheme(&mut self.input),
            KeyCode::Char(character) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.push(character);
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn commit_revision(&mut self, revision: &str) -> Result<(), AppError> {
        if revision
            .chars()
            .any(|character| matches!(character, '\r' | '\n'))
        {
            self.set_error("Codex revision must be one Markdown line");
            return Ok(());
        }
        let selection = self
            .editor
            .selection_source_range()
            .ok_or(AppError::MissingSelection)?;
        let mut next_source = self.snapshot.source().to_owned();
        next_source.replace_range(selection.clone(), revision);
        commit(&self.snapshot, &next_source)?;
        self.snapshot = DocumentSnapshot::load(self.snapshot.path())?;
        self.editor = Editor::new(self.snapshot.source().to_owned())?;
        self.editor.jump_to_source(selection.start);
        self.input.clear();
        self.review.clear();
        self.codex_action = None;
        self.mode = Mode::Browse;
        self.status = "저장됨 · Codex 문장 수정 적용".to_owned();
        self.status_tone = StatusTone::Success;
        Ok(())
    }
}

fn remove_last_grapheme(input: &mut String) {
    if let Some((index, _)) = input.grapheme_indices(true).next_back() {
        input.truncate(index);
    }
}
