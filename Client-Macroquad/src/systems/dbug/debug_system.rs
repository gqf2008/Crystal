use crate::components::RenderConfig;
use crate::systems::logic::physics::MapUpdateSystem;
use crate::systems::RenderSystem;
use macroquad::prelude::KeyCode;  // ✅ 直接使用 macroquad

/// DebugSystemV2 - 使用 GameContext 的调试系统
///
/// **改进点**:
/// - ✅ 使用 GameContext 零拷贝访问 ggez Context (用于渲染)
/// - ✅ 从 GameContext 读取按键事件 (使用边缘检测)
///
/// **职责**:
/// - 检测调试快捷键（图层切换、网格、边框等）
/// - 修改 RenderConfig 组件
/// - 渲染调试信息（FPS、实体数量等）
#[derive(ecs_macros::RenderSystem)]
pub struct DebugSystem;

impl DebugSystem {
    pub fn new() -> Self {
        Self
    }

    /// 处理调试相关的键盘按键
    fn handle_keycode(world: &mut hecs::World, keycode: KeyCode) {
        // M键需要特殊处理，因为它会借用整个 world
        if keycode == KeyCode::M {
            tracing::info!("🗺️ 打开地图选择对话框");

            MapUpdateSystem::trigger_map_selection(world);
            return;
        }

        // 查找 RenderConfig 组件
        let mut config_query = world.query::<&mut RenderConfig>();
        let config = if let Some((_, cfg)) = config_query.iter().next() {
            cfg
        } else {
            return;
        };

        match keycode {
            // 图层切换 (与 C# 版本一致: 1=Front, 2=Middle, 3=Back)
            KeyCode::Key1 => {
                config.show_front = !config.show_front;
                tracing::info!(
                    "🖼️ Front 层: {}",
                    if config.show_front {
                        "显示"
                    } else {
                        "隐藏"
                    }
                );
            }
            KeyCode::Key2 => {
                config.show_middle = !config.show_middle;
                tracing::info!(
                    "🖼️ Middle 层: {}",
                    if config.show_middle {
                        "显示"
                    } else {
                        "隐藏"
                    }
                );
            }
            KeyCode::Key3 => {
                config.show_back = !config.show_back;
                tracing::info!(
                    "🖼️ Back 层: {}",
                    if config.show_back { "显示" } else { "隐藏" }
                );
            }

            // 调试显示选项
            KeyCode::G => {
                config.show_grid = !config.show_grid;
                tracing::info!(
                    "🔲 网格: {}",
                    if config.show_grid { "显示" } else { "隐藏" }
                );
            }
            KeyCode::O => {
                config.show_obstacles = !config.show_obstacles;
                tracing::info!(
                    "🚧 障碍物: {}",
                    if config.show_obstacles {
                        "显示"
                    } else {
                        "隐藏"
                    }
                );
            }
            KeyCode::B => {
                config.show_borders = !config.show_borders;
                tracing::info!(
                    "📦 边框: {}",
                    if config.show_borders {
                        "显示"
                    } else {
                        "隐藏"
                    }
                );
            }
            KeyCode::P => {
                config.show_path = !config.show_path;
                tracing::info!(
                    "🛤️ 路径: {}",
                    if config.show_path { "显示" } else { "隐藏" }
                );
            }
            KeyCode::A => {
                config.show_animations = !config.show_animations;
                tracing::info!(
                    "🎬 动画: {}",
                    if config.show_animations {
                        "播放"
                    } else {
                        "暂停"
                    }
                );
            }
            KeyCode::S => {
                config.show_static_tiles = !config.show_static_tiles;
                tracing::info!(
                    "🗿 静态瓦片: {}",
                    if config.show_static_tiles {
                        "显示"
                    } else {
                        "隐藏"
                    }
                );
            }
            KeyCode::D => {
                config.show_animated_tiles = !config.show_animated_tiles;
                tracing::info!(
                    "🎞️ 动画瓦片: {}",
                    if config.show_animated_tiles {
                        "显示"
                    } else {
                        "隐藏"
                    }
                );
            }
            KeyCode::L => {
                config.enable_lod = !config.enable_lod;
                tracing::info!(
                    "🔍 LOD: {}",
                    if config.enable_lod {
                        "启用"
                    } else {
                        "禁用"
                    }
                );
            }

            // F键 - 调试显示
            KeyCode::F2 => {
                config.show_player_debug = !config.show_player_debug;
                tracing::info!(
                    "👤 玩家调试信息: {}",
                    if config.show_player_debug {
                        "显示"
                    } else {
                        "隐藏"
                    }
                );
            }
            KeyCode::F9 => {
                config.show_monster_borders = !config.show_monster_borders;
                tracing::info!(
                    "👹 怪物边框: {}",
                    if config.show_monster_borders {
                        "显示"
                    } else {
                        "隐藏"
                    }
                );
            }
            KeyCode::F10 => {
                config.show_npc_borders = !config.show_npc_borders;
                tracing::info!(
                    "👤 NPC边框: {}",
                    if config.show_npc_borders {
                        "显示"
                    } else {
                        "隐藏"
                    }
                );
            }
            KeyCode::F11 => {
                config.show_effect_borders = !config.show_effect_borders;
                tracing::info!(
                    "✨ 特效边框: {}",
                    if config.show_effect_borders {
                        "显示"
                    } else {
                        "隐藏"
                    }
                );
            }

            // FPS 控制
            KeyCode::Equal | KeyCode::KpAdd => {
                config.max_fps = (config.max_fps + 10).min(300);
                tracing::info!("⚡ 最大FPS: {}", config.max_fps);
            }
            KeyCode::Minus | KeyCode::KpSubtract => {
                config.max_fps = (config.max_fps.saturating_sub(10)).max(30);
                tracing::info!("⚡ 最大FPS: {}", config.max_fps);
            }

            KeyCode::Escape => {
                tracing::info!("👋 用户按下 ESC，应该退出程序（需要在 App 层处理）");
            }

            _ => {}
        }
    }
}

impl RenderSystem for DebugSystem {
    fn draw(
        &mut self,
        _world: &hecs::World,
    ) -> crate::game::GameResult {
        // TODO: 重写为 macroquad API
        // 原实现包括:
        // 1. FPS 显示
        // 2. 相机信息显示
        // 3. 调试网格绘制
        // 4. 障碍物显示
        // 5. 路径可视化
        Ok(())
    }
}
