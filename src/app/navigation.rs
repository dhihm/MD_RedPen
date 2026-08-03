//! Browse and keyboard-selection navigation.

use crossterm::event::{KeyCode, KeyEvent};

use super::App;
use crate::{app_error::AppError, app_state::Mode};

impl App {
    pub(super) fn handle_browse(&mut self, key: KeyEvent) -> Result<(), AppError> {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('v') => {
                self.editor.start_visual();
                self.mode = Mode::Visual;
                self.set_neutral("선택 중 · ←/→로 범위를 조절하세요");
            }
            KeyCode::Char('w') => {
                self.editor.select_current_word();
                self.mode = Mode::Visual;
                self.set_neutral("단어 선택됨 · a 수동 메모 · c Codex");
            }
            KeyCode::Left | KeyCode::Char('h') => self.editor.move_left(),
            KeyCode::Right | KeyCode::Char('l') => self.editor.move_right(),
            KeyCode::Up | KeyCode::Char('k') => {
                if self.editor.move_paragraph_up() {
                    self.set_neutral("이전 문단으로 이동");
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.editor.move_paragraph_down() {
                    self.set_neutral("다음 문단으로 이동");
                }
            }
            KeyCode::Enter => self.follow_current_link(),
            KeyCode::Char('b') => self.return_from_endnote(),
            _ => {}
        }
        Ok(())
    }

    pub(super) fn handle_visual(&mut self, key: KeyEvent) -> Result<(), AppError> {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => self.editor.move_left(),
            KeyCode::Right | KeyCode::Char('l') => self.editor.move_right(),
            KeyCode::Char('a') => {
                self.mode = Mode::ManualInput;
                self.input.clear();
                self.set_neutral("미주 내용을 입력하고 Enter로 저장하세요");
            }
            KeyCode::Char('c') => self.start_codex()?,
            KeyCode::Esc => {
                self.editor.clear_visual();
                self.mode = Mode::Browse;
                self.set_neutral("선택 취소됨");
            }
            _ => {}
        }
        Ok(())
    }

    fn follow_current_link(&mut self) {
        let Some(target) = self.editor.current_link_target().map(str::to_owned) else {
            self.set_neutral("현재 위치에는 연결된 미주가 없습니다");
            return;
        };
        let Some(anchor) = target.strip_prefix('#') else {
            self.set_neutral("외부 링크는 v1에서 열지 않습니다");
            return;
        };
        let markup = format!("<a id=\"{anchor}\"></a>");
        if let Some(offset) = self.snapshot.source().find(&markup) {
            self.return_cursor = Some(self.editor.cursor());
            self.editor.jump_to_source(offset + markup.len());
            self.set_neutral("미주로 이동 · b로 본문 복귀");
        } else {
            self.set_error("연결된 미주 앵커를 찾을 수 없습니다");
        }
    }

    fn return_from_endnote(&mut self) {
        if let Some(cursor) = self.return_cursor.take() {
            self.editor.restore_cursor(cursor);
            self.set_neutral("본문으로 복귀");
        }
    }
}
