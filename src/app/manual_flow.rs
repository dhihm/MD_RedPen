//! Manual note entry and shared reviewed-note commit.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation;

use super::App;
use crate::{
    annotation::{AnnotationId, AnnotationRequest, NoteKind, annotate},
    app_error::AppError,
    app_state::{Mode, StatusTone},
    editor::Editor,
    storage::{DocumentSnapshot, commit},
};

impl App {
    pub(super) fn handle_manual_input(&mut self, key: KeyEvent) -> Result<(), AppError> {
        match key.code {
            KeyCode::Enter => self.commit_manual_note()?,
            KeyCode::Esc => {
                self.input.clear();
                self.mode = Mode::Visual;
                self.set_neutral("입력 취소됨 · 선택은 유지됩니다");
            }
            KeyCode::Backspace => remove_last_grapheme(&mut self.input),
            KeyCode::Char(character) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.push(character);
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn commit_note(&mut self, kind: NoteKind, note: &str) -> Result<(), AppError> {
        let selection = self
            .editor
            .selection_source_range()
            .ok_or(AppError::MissingSelection)?;
        let request = AnnotationRequest {
            id: AnnotationId::generate(),
            kind,
            note,
            selection,
        };
        let next_source = annotate(self.snapshot.source(), &request)?;
        commit(&self.snapshot, &next_source)?;
        self.snapshot = DocumentSnapshot::load(self.snapshot.path())?;
        self.editor = Editor::new(self.snapshot.source().to_owned())?;
        self.input.clear();
        self.review.clear();
        self.mode = Mode::Browse;
        self.status = "저장됨 · 형광펜 미주 추가".to_owned();
        self.status_tone = StatusTone::Success;
        Ok(())
    }

    fn commit_manual_note(&mut self) -> Result<(), AppError> {
        if self.input.trim().is_empty() {
            self.set_error("Note cannot be empty");
            return Ok(());
        }
        let note = self.input.trim().to_owned();
        self.commit_note(NoteKind::Manual, &note)
    }
}

fn remove_last_grapheme(input: &mut String) {
    if let Some((index, _)) = input.grapheme_indices(true).next_back() {
        input.truncate(index);
    }
}
