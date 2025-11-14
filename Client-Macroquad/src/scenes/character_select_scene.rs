// CharacterSelectScene - 角色选择

use crate::game::GameResult;
use crate::scenes::{SceneHandler, SceneTransition};
use macroquad::prelude::*;

pub struct CharacterSelectScene;

impl CharacterSelectScene {
    pub fn new() -> Self { Self }
}

impl SceneHandler for CharacterSelectScene {
    fn name(&self) -> &str { "角色选择" }
    
    fn on_enter(&mut self) -> GameResult {
        println!("🎬 进入角色选择");
        Ok(())
    }
    
    fn on_exit(&mut self) -> GameResult {
        println!("🎬 离开角色选择");
        Ok(())
    }
    
    fn update(&mut self, _dt: f32) -> GameResult<SceneTransition> {
        if is_key_pressed(KeyCode::Enter) {
            return Ok(SceneTransition::Game);
        }
        if is_key_pressed(KeyCode::Escape) {
            return Ok(SceneTransition::Login);
        }
        Ok(SceneTransition::None)
    }
    
    fn render(&mut self) -> GameResult {
        clear_background(Color::from_rgba(30, 30, 40, 255));
        let w = screen_width();
        let h = screen_height();
        
        draw_text_ex("角色选择界面", w / 2.0 - 150.0, h / 2.0, TextParams {
            font: None, font_size: 48, color: WHITE, ..Default::default()
        });
        
        draw_text_ex("Enter: 进入游戏 | ESC: 返回登录", w / 2.0 - 150.0, h - 50.0, TextParams {
            font: None, font_size: 16, color: GRAY, ..Default::default()
        });
        
        Ok(())
    }
    
    fn handle_input(&mut self) -> GameResult { Ok(()) }
}
