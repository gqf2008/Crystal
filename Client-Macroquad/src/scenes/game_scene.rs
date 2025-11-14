// GameScene - 游戏主场景

use crate::game::GameResult;
use crate::scenes::{SceneHandler, SceneTransition};
use macroquad::prelude::*;

pub struct GameScene;

impl GameScene {
    pub fn new() -> Self { Self }
}

impl SceneHandler for GameScene {
    fn name(&self) -> &str { "游戏场景" }
    
    fn on_enter(&mut self) -> GameResult {
        println!("🎬 进入游戏场景");
        Ok(())
    }
    
    fn on_exit(&mut self) -> GameResult {
        println!("🎬 离开游戏场景");
        Ok(())
    }
    
    fn update(&mut self, _dt: f32) -> GameResult<SceneTransition> {
        if is_key_pressed(KeyCode::Escape) {
            return Ok(SceneTransition::CharacterSelect);
        }
        Ok(SceneTransition::None)
    }
    
    fn render(&mut self) -> GameResult {
        clear_background(Color::from_rgba(40, 60, 40, 255));
        let w = screen_width();
        let h = screen_height();
        
        draw_text_ex("游戏主场景", w / 2.0 - 100.0, h / 2.0, TextParams {
            font: None, font_size: 48, color: WHITE, ..Default::default()
        });
        
        draw_text_ex("ESC: 返回角色选择", w / 2.0 - 100.0, h - 50.0, TextParams {
            font: None, font_size: 16, color: GRAY, ..Default::default()
        });
        
        Ok(())
    }
    
    fn handle_input(&mut self) -> GameResult { Ok(()) }
}
