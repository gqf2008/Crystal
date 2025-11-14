// LoadingScene - 加载场景

use crate::game::GameResult;
use crate::scenes::{Scene, SceneTransition};
use macroquad::prelude::*;

pub struct LoadingScene;

impl LoadingScene {
    pub fn new() -> Self { Self }
}

impl Scene for LoadingScene {
    fn name(&self) -> &str { "加载中" }
    fn on_enter(&mut self) -> GameResult { Ok(()) }
    fn on_exit(&mut self) -> GameResult { Ok(()) }
    fn update(&mut self, _dt: f32) -> GameResult<SceneTransition> { Ok(SceneTransition::None) }
    fn render(&mut self) -> GameResult {
        clear_background(BLACK);
        draw_text_ex("加载中...", screen_width() / 2.0 - 80.0, screen_height() / 2.0, TextParams {
            font: None, font_size: 32, color: WHITE, ..Default::default()
        });
        Ok(())
    }
    fn handle_input(&mut self) -> GameResult { Ok(()) }
}
