use crate::ecs::components::{InputEvent, RenderConfig};
use crate::ecs::HybridSystem;
use crate::ecs::WorldExt;
use ggez::input::keyboard::KeyCode;
use ggez::GameResult;

/// DebugSystem 是唯一的混合系统，既实现了 System 又实现了 DrawSystem
/// 
/// 职责：
/// - 从 GlobalEvents 读取键盘事件，处理调试相关的快捷键
/// - 修改 RenderConfig 组件（图层切换、网格、边框等）
/// - 渲染调试信息（FPS、实体数量等）
pub struct DebugSystem;

impl HybridSystem for DebugSystem {
    fn priority(&self) -> u32 {
       u32::MAX - 1
    }
    
    fn update(&mut self, world: &mut hecs::World, _delay_time: f32) -> GameResult {
        // 从 GlobalEvents 读取输入事件
        let input_events = {
            let global_events = world.global_events();
            global_events.input_events.clone()
        };

        // 处理键盘事件
        for event in input_events {
            if let InputEvent::KeyDown { keycode, .. } = event {
                Self::handle_keycode(world, keycode);
            }
        }

        Ok(())
    }
    
    fn draw(
        &mut self,
        ctx: &mut ggez::Context,
        canvas: &mut ggez::graphics::Canvas,
        world: &hecs::World,
    ) -> GameResult {
        use crate::ecs::components::{Camera, Position, TimeTracker};
        use ggez::graphics::{Color, DrawParam, Text, TextFragment};

        // 1. 读取调试信息
        let mut fps = 0.0;
        let mut camera_pos = (0.0, 0.0);
        let mut camera_zoom = 1.0;
        let entity_count = world.len();

        // 读取 TimeTracker 获取 FPS
        for (_, time) in world.query::<&TimeTracker>().iter() {
            fps = time.fps;
            break;
        }

        // 读取 Camera 和 Position 获取相机信息
        for (_, (camera, pos)) in world.query::<(&Camera, &Position)>().iter() {
            camera_pos = (pos.x, pos.y);
            camera_zoom = camera.zoom;
            break;
        }

        // 2. 构建调试文本
        let debug_text = format!(
            "FPS: {:.1}\nCamera: ({:.0}, {:.0})\nZoom: {:.2}x\nEntities: {}",
            fps, camera_pos.0, camera_pos.1, camera_zoom, entity_count
        );

        // 3. 绘制调试文本（左上角，黑色背景，白色文字）
        let text = Text::new(TextFragment {
            text: debug_text,
            color: Some(Color::WHITE),
            font: None,
            scale: Some(ggez::graphics::PxScale::from(24.0)),  // 从 16.0 增大到 24.0
        });

        // 绘制半透明黑色背景
        let bg_rect = ggez::graphics::Rect::new(5.0, 5.0, 250.0, 100.0);  // 调整背景大小
        let bg_mesh = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::fill(),
            bg_rect,
            Color::from_rgba(0, 0, 0, 180),
        )?;
        canvas.draw(&bg_mesh, DrawParam::default());

        // 绘制文本
        canvas.draw(&text, DrawParam::default().dest([10.0, 10.0]));

        // TODO: 如果 show_grid, 绘制网格
        // TODO: 如果 show_borders, 绘制实体边框

        Ok(())
    }
}

impl DebugSystem {
    /// 处理调试相关的键盘按键
    fn handle_keycode(world: &mut hecs::World, keycode: KeyCode) {
        // 查找 RenderConfig 组件
        let mut config_query = world.query::<&mut RenderConfig>();
        let config = if let Some((_, cfg)) = config_query.iter().next() {
            cfg
        } else {
            return;
        };

        match keycode {
            // 图层切换
            KeyCode::Digit1 => {
                config.show_back = !config.show_back;
                tracing::info!("🖼️ Back 层: {}", if config.show_back { "显示" } else { "隐藏" });
            }
            KeyCode::Digit2 => {
                config.show_middle = !config.show_middle;
                tracing::info!("🖼️ Middle 层: {}", if config.show_middle { "显示" } else { "隐藏" });
            }
            KeyCode::Digit3 => {
                config.show_front = !config.show_front;
                tracing::info!("🖼️ Front 层: {}", if config.show_front { "显示" } else { "隐藏" });
            }
            
            // 调试显示选项
            KeyCode::KeyG => {
                config.show_grid = !config.show_grid;
                tracing::info!("🔲 网格: {}", if config.show_grid { "显示" } else { "隐藏" });
            }
            KeyCode::KeyO => {
                config.show_obstacles = !config.show_obstacles;
                tracing::info!("🚧 障碍物: {}", if config.show_obstacles { "显示" } else { "隐藏" });
            }
            KeyCode::KeyB => {
                config.show_borders = !config.show_borders;
                tracing::info!("📦 边框: {}", if config.show_borders { "显示" } else { "隐藏" });
            }
            KeyCode::KeyP => {
                config.show_path = !config.show_path;
                tracing::info!("🛤️ 路径: {}", if config.show_path { "显示" } else { "隐藏" });
            }
            KeyCode::KeyA => {
                config.show_animations = !config.show_animations;
                tracing::info!("🎬 动画: {}", if config.show_animations { "播放" } else { "暂停" });
            }
            KeyCode::KeyL => {
                config.enable_lod = !config.enable_lod;
                tracing::info!("🔍 LOD: {}", if config.enable_lod { "启用" } else { "禁用" });
            }
            
            // F键 - 边框显示
            KeyCode::F9 => {
                config.show_monster_borders = !config.show_monster_borders;
                tracing::info!("👹 怪物边框: {}", if config.show_monster_borders { "显示" } else { "隐藏" });
            }
            KeyCode::F10 => {
                config.show_npc_borders = !config.show_npc_borders;
                tracing::info!("👤 NPC边框: {}", if config.show_npc_borders { "显示" } else { "隐藏" });
            }
            KeyCode::F11 => {
                config.show_effect_borders = !config.show_effect_borders;
                tracing::info!("✨ 特效边框: {}", if config.show_effect_borders { "显示" } else { "隐藏" });
            }
            
            // FPS 控制
            KeyCode::Equal | KeyCode::NumpadAdd => {
                config.max_fps = (config.max_fps + 10).min(300);
                tracing::info!("⚡ 最大FPS: {}", config.max_fps);
            }
            KeyCode::Minus | KeyCode::NumpadSubtract => {
                config.max_fps = (config.max_fps.saturating_sub(10)).max(30);
                tracing::info!("⚡ 最大FPS: {}", config.max_fps);
            }
            
            KeyCode::Escape => {
                tracing::info!("👋 用户按下 ESC，应该退出程序（需要在 App 层处理）");
                // 注意：退出程序需要调用 ctx.request_quit()，这需要在有 Context 的地方处理
            }
            
            _ => {}
        }
    }
}
