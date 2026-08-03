//! Mouse-driven visual selection.

use crossterm::event::{MouseButton, MouseEventKind};

use super::App;
use crate::app_state::Mode;

impl App {
    /// Applies one mouse action to an optional rendered grapheme hit.
    pub fn handle_mouse(&mut self, kind: MouseEventKind, rendered_index: Option<usize>) {
        if !matches!(self.mode, Mode::Browse | Mode::Visual) {
            return;
        }
        match kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.mouse_selecting = false;
                if rendered_index.is_some_and(|index| self.editor.start_visual_at(index)) {
                    self.mouse_selecting = true;
                    self.mode = Mode::Visual;
                    self.set_neutral("마우스 선택 중 · 드래그로 범위를 조절하세요");
                }
            }
            MouseEventKind::Drag(MouseButton::Left) if self.mouse_selecting => {
                if let Some(index) = rendered_index {
                    self.editor.extend_visual_to(index);
                }
            }
            MouseEventKind::Up(MouseButton::Left) if self.mouse_selecting => {
                if let Some(index) = rendered_index {
                    self.editor.extend_visual_to(index);
                }
                self.mouse_selecting = false;
                self.set_neutral("마우스 선택됨 · a 수동 메모 · c Codex");
            }
            _ => {}
        }
    }
}
