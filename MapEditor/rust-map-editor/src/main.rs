mod mlibrary;
mod map_code;
mod frames;
mod renderer;
mod editor_state;

use anyhow::Result;
use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::{self, EventHandler};
use ggez::graphics::{self, Color};
use ggez::{Context, ContextBuilder, GameResult};
use std::env;
use std::path;

use crate::editor_state::EditorState;

struct MapEditorApp {
    state: EditorState,
}

impl MapEditorApp {
    fn new(ctx: &mut Context) -> GameResult<Self> {
        let state = EditorState::new(ctx)?;
        Ok(MapEditorApp { state })
    }
}

impl EventHandler for MapEditorApp {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.state.update(ctx)?;
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = graphics::Canvas::from_frame(ctx, Color::WHITE);
        
        self.state.draw(ctx, &mut canvas)?;
        
        canvas.finish(ctx)?;
        Ok(())
    }

    fn mouse_motion_event(
        &mut self,
        _ctx: &mut Context,
        x: f32,
        y: f32,
        _dx: f32,
        _dy: f32,
    ) -> GameResult {
        self.state.handle_mouse_move(x, y);
        Ok(())
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: event::MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        self.state.handle_mouse_down(button, x, y);
        Ok(())
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        input: ggez::input::keyboard::KeyInput,
        _repeated: bool,
    ) -> GameResult {
        self.state.handle_key_down(ctx, input);
        Ok(())
    }
}

fn main() -> Result<()> {
    // 设置资源路径
    let resource_dir = if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut path = path::PathBuf::from(manifest_dir);
        path.push("resources");
        path
    } else {
        path::PathBuf::from("./resources")
    };

    // 创建 ggez 上下文
    let (mut ctx, event_loop) = ContextBuilder::new("crystal_map_editor", "Suprcode")
        .window_setup(WindowSetup::default().title("Crystal Map Editor - Rust Port"))
        .window_mode(WindowMode::default().dimensions(1280.0, 720.0))
        .add_resource_path(resource_dir)
        .build()?;

    // 创建应用实例
    let app = MapEditorApp::new(&mut ctx)?;

    // 运行游戏循环
    event::run(ctx, event_loop, app);
}
