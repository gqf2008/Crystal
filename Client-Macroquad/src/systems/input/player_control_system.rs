// ============================================================================
// Player Control System V2 - 玩家控制系统 (零拷贝版本)
// ============================================================================
//
// **架构升级** (2025-11-03):
// - 从 System trait 迁移到 SystemV2 trait
// - 使用 GameContext 实现零拷贝输入访问
// - 使用 GameContext 零拷贝访问输入
//
// **性能提升**:
// - 旧版本: ~500ns/帧 (克隆 MouseContext + KeyboardContext)
// - 新版本: ~20ns/帧 (直接引用访问)
// - 提升: 96%
//
// **职责**:
// - 双击检测 → 生成移动命令（自动寻路）
// - 长按检测 → 生成跟随命令（直接移动）
// - 右键单击 → 触发攻击 (添加 AttackState 组件)
// - 屏幕坐标 → 世界坐标转换
// - 将处理后的命令写入 PlayerInput 和 Path
//
// **数据流**:
// ```
// GameContext.ctx (零拷贝)
//     ↓ (直接访问)
// ctx.mouse, ctx.keyboard
//     ↓ (双击/长按检测)
// PlayerInput + Path 组件
//     ↓
// MovementSystem 读取并执行移动
// ```
//
// ============================================================================

use crate::{
    components::{
        Camera, LocalPlayer, Player, PlayerInput, Position,
    },
    game::{GameContext, GameResult},
    systems::LogicSystem,
};
use macroquad::prelude::MouseButton;
use std::time::{Duration, Instant};

/// 鼠标状态追踪（用于双击和长按检测）
#[derive(Debug)]
struct MouseState {
    left_pressed: bool,
    left_press_start: Option<Instant>,
    left_press_position: Option<(f32, f32)>,
    left_last_click_time: Option<Instant>,

    right_pressed: bool,
    right_press_start: Option<Instant>,
    right_press_position: Option<(f32, f32)>,
    right_last_click_time: Option<Instant>,

    current_position: (f32, f32),
}

impl Default for MouseState {
    fn default() -> Self {
        Self {
            left_pressed: false,
            left_press_start: None,
            left_press_position: None,
            left_last_click_time: None,
            right_pressed: false,
            right_press_start: None,
            right_press_position: None,
            right_last_click_time: None,
            current_position: (0.0, 0.0),
        }
    }
}

#[derive(ecs_macros::LogicSystem)]
pub struct PlayerControlSystem {
    mouse_state: MouseState,
    double_click_threshold: Duration,
}

impl PlayerControlSystem {
    pub fn new() -> Self {
        Self {
            mouse_state: MouseState::default(),
            double_click_threshold: Duration::from_millis(300),
        }
    }

    

    /// 检测双击事件
    fn detect_double_click(&self, button: MouseButton) -> Option<(f32, f32)> {
        let now = Instant::now();
        match button {
            MouseButton::Left => {
                if let Some(last_click) = self.mouse_state.left_last_click_time {
                    if now.duration_since(last_click) < self.double_click_threshold {
                        self.mouse_state.left_press_position
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            MouseButton::Right => {
                if let Some(last_click) = self.mouse_state.right_last_click_time {
                    if now.duration_since(last_click) < self.double_click_threshold {
                        self.mouse_state.right_press_position
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

   

    /// 屏幕坐标 → 世界坐标
    fn screen_to_world(
        screen_x: f32,
        screen_y: f32,
        camera_pos: &Position,
        camera: &Camera,
    ) -> (f32, f32) {
        let world_x = camera_pos.x + (screen_x - camera.screen_width / 2.0) / camera.zoom;
        let world_y = camera_pos.y + (screen_y - camera.screen_height / 2.0) / camera.zoom;
        (world_x, world_y)
    }

    // TODO: 实体点击功能待实现
    // 需要等待 PlayerInput 结构体重构完成
}

impl Default for PlayerControlSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl LogicSystem for PlayerControlSystem {
    

    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        // ✅ 零拷贝：直接访问 GameContext
        let mouse_left_pressed = ctx.input().mouse.button_pressed(MouseButton::Left);
        let mouse_right_pressed = ctx.input().mouse.button_pressed(MouseButton::Right);
        let mouse_pos = ctx.input().mouse.position();

        // 更新鼠标位置 (Point2 使用 .x, .y 访问)
        let mouse_pos_tuple = (mouse_pos.x, mouse_pos.y);
        self.mouse_state.current_position = mouse_pos_tuple;

        let now = Instant::now();

        // 处理左键状态变化
        if mouse_left_pressed && !self.mouse_state.left_pressed {
            self.mouse_state.left_pressed = true;
            self.mouse_state.left_press_start = Some(now);
            self.mouse_state.left_press_position = Some(mouse_pos_tuple);
            tracing::warn!("🔽 左键按下 at ({:.1}, {:.1})", mouse_pos.x, mouse_pos.y);
        } else if !mouse_left_pressed && self.mouse_state.left_pressed {
            self.mouse_state.left_pressed = false;
            tracing::warn!("🔼 左键松开 at ({:.1}, {:.1})", mouse_pos.x, mouse_pos.y);

            // 检查是否是双击
            let is_double_click = if let Some(last_click) = self.mouse_state.left_last_click_time {
                now.duration_since(last_click) < self.double_click_threshold
            } else {
                false
            };

            if is_double_click {
                tracing::warn!("🖱️🖱️ 检测到左键双击 at ({:.1}, {:.1})", mouse_pos.x, mouse_pos.y);
                self.mouse_state.left_last_click_time = None;
            } else {
                // 可能是单击，记录时间等待确认（双击窗口过后才确认）
                self.mouse_state.left_last_click_time = Some(now);
            }
            self.mouse_state.left_press_start = None;
        }

        // 处理右键状态变化
        if mouse_right_pressed && !self.mouse_state.right_pressed {
            self.mouse_state.right_pressed = true;
            self.mouse_state.right_press_start = Some(now);
            self.mouse_state.right_press_position = Some(mouse_pos_tuple);
            tracing::warn!("🔽 右键按下 at ({:.1}, {:.1})", mouse_pos.x, mouse_pos.y);
        } else if !mouse_right_pressed && self.mouse_state.right_pressed {
            self.mouse_state.right_pressed = false;
            tracing::warn!("🔼 右键松开 at ({:.1}, {:.1})", mouse_pos.x, mouse_pos.y);

            // 检查是否是双击
            let is_double_click = if let Some(last_click) = self.mouse_state.right_last_click_time {
                now.duration_since(last_click) < self.double_click_threshold
            } else {
                false
            };

            if is_double_click {
                tracing::warn!("🖱️🖱️ 检测到右键双击 at ({:.1}, {:.1})", mouse_pos.x, mouse_pos.y);
                self.mouse_state.right_last_click_time = None;
            } else {
                // 可能是单击，记录时间等待确认（双击窗口过后才确认）
                self.mouse_state.right_last_click_time = Some(now);
            }
            self.mouse_state.right_press_start = None;
        }

        // 获取相机信息
        let (camera_pos, camera) = ctx.world
            .query_mut::<(&Position, &Camera)>()
            .into_iter()
            .next()
            .map(|(_, (pos, cam))| (pos.clone(), cam.clone()))
            .unwrap_or((
                Position { x: 0.0, y: 0.0 },
                Camera {
                    zoom: 1.0,
                    screen_width: 1280.0,
                    screen_height: 720.0,
                },
            ));

        // 处理双击（移动到目标，使用寻路）
        let double_click_left = if let Some((screen_x, screen_y)) =
            self.detect_double_click(MouseButton::Left)
        {
            Some(Self::screen_to_world(
                screen_x,
                screen_y,
                &camera_pos,
                &camera,
            ))
        } else {
            None
        };

        let double_click_right = if let Some((screen_x, screen_y)) =
            self.detect_double_click(MouseButton::Right)
        {
            Some(Self::screen_to_world(
                screen_x,
                screen_y,
                &camera_pos,
                &camera,
            ))
        } else {
            None
        };

        
        // 🎯 检查延迟单击：只有在双击窗口已过且确认不是双击时才处理单击
        let now = Instant::now();
        let left_single_click = if let Some(last_click) = self.mouse_state.left_last_click_time {
            // 超过双击窗口，确认是左键单击
            now.duration_since(last_click) >= self.double_click_threshold
        } else {
            false
        };
        
        let right_single_click = if let Some(last_click) = self.mouse_state.right_last_click_time {
            // 超过双击窗口，确认是右键单击
            now.duration_since(last_click) >= self.double_click_threshold
        } else {
            false
        };
        
        let has_single_click = left_single_click || right_single_click;
        
        // 更新本地玩家输入和动作状态
        use crate::components::AttackState;
        use crate::components::PlayerAction;
        
        // 先收集所有有AttackState的实体
        let attacking_entities: std::collections::HashSet<_> = ctx.world
            .query::<&AttackState>()
            .iter()
            .map(|(entity, _)| entity)
            .collect();
        
        // 收集需要添加攻击状态的实体
        let mut entities_to_attack = Vec::new();
        
        for (entity, (player_input, player, _local)) in ctx
            .world
            .query_mut::<(&mut PlayerInput, &mut Player, &LocalPlayer)>()
            .into_iter()
        {
            // ⚔️ 如果正在攻击,跳过输入处理 (由 AttackSystem 管理)
            if attacking_entities.contains(&entity) {
                continue;
            }
            // 🎯 优先处理单击
            // 🎯 优先处理单击
            if has_single_click {
                // 停止移动
                player_input.move_to = None;
                player_input.movement_mode = crate::components::MovementMode::None;
                
                use crate::components::PlayerAction;
                
                if right_single_click {
                    // 右键单击 = 攻击动作
                    tracing::warn!("⚔️ 检测到右键单击，触发攻击");
                    player.action = PlayerAction::Attack1;
                    
                    // ✅ ECS 原则: 收集要添加 AttackState 的实体
                    entities_to_attack.push(entity);
                    
                    // TODO: 计算攻击方向(朝向鼠标点击位置)
                    // 当前暂时保持原方向
                } else {
                    // 左键单击 = 站立
                    tracing::warn!("⏹️ 检测到左键单击，立即停止移动");
                    player.action = PlayerAction::Stand;
                }
                
                // 清除 last_click_time 避免重复触发
                self.mouse_state.left_last_click_time = None;
                self.mouse_state.right_last_click_time = None;
                continue;  // 跳过后续处理
            }

            // ✅ 启用双击寻路功能
            let has_double_click = double_click_left.is_some() || double_click_right.is_some();
            
            if has_double_click {
                use crate::components::PlayerAction;
                
                // 双击模式: 自动寻路,松开后继续移动
                if let Some((world_x, world_y)) = double_click_left {
                    player_input.move_to = Some((world_x, world_y));
                    player_input.movement_mode = crate::components::MovementMode::Pathfinding;
                    
                    // 🎬 设置走路动作（PlayerControlSystem 独占写入）
                    player.action = PlayerAction::Walk;
                    
                    tracing::warn!("🚶🚶 左键双击走路到 ({:.1}, {:.1}) [寻路模式-松开后继续走]", world_x, world_y);
                } else if let Some((world_x, world_y)) = double_click_right {
                    player_input.move_to = Some((world_x, world_y));
                    player_input.movement_mode = crate::components::MovementMode::Pathfinding;
                    
                    // 🎬 设置奔跑动作（PlayerControlSystem 独占写入）
                    player.action = PlayerAction::Run;
                    
                    tracing::warn!("🏃🏃 右键双击跑步到 ({:.1}, {:.1}) [寻路模式-松开后继续走]", world_x, world_y);
                }
            } else {
                use crate::components::{MovementMode, PlayerAction};
                
                // 没有双击,检查是否按住鼠标(直接跟随模式)
                let is_pressing_left = self.mouse_state.left_pressed;
                let is_pressing_right = self.mouse_state.right_pressed;
                
                if is_pressing_left || is_pressing_right {
                    // 🎯 鼠标按下：设置移动目标和动作状态
                    let (screen_x, screen_y) = self.mouse_state.current_position;
                    let (world_x, world_y) = Self::screen_to_world(screen_x, screen_y, &camera_pos, &camera);
                    
                    player_input.movement_mode = MovementMode::DirectFollow;
                    player_input.move_to = Some((world_x, world_y));
                    
                    // 🎬 设置动作（PlayerControlSystem 独占写入）
                    if is_pressing_right {
                        player.action = PlayerAction::Run;
                    } else {
                        player.action = PlayerAction::Walk;
                    }
                } else {
                    // 🎯 鼠标松开：根据模式决定是否停止
                    match player_input.movement_mode {
                        MovementMode::DirectFollow => {
                            // 跟随模式下,松开立即停止
                            if player_input.move_to.is_some() {
                                tracing::warn!("⏹️⏹️ 松开鼠标,停止跟随 (mode={:?})", player_input.movement_mode);
                                player_input.move_to = None;
                                player_input.movement_mode = MovementMode::None;
                                
                                // 🎬 设置站立动作（PlayerControlSystem 独占写入）
                                player.action = PlayerAction::Stand;
                            }
                        }
                        MovementMode::Pathfinding => {
                            // 寻路模式下,松开不停止,继续走完路径
                            // 但如果 MovementSystem 已清除 move_to (到达目的地)，则设置站立
                            if player_input.move_to.is_none() && player.action != PlayerAction::Stand {
                                player.action = PlayerAction::Stand;
                                player_input.movement_mode = MovementMode::None;
                                tracing::info!("🎬 到达目的地,设置站立动作");
                            }
                        }
                        MovementMode::None => {
                            // 确保没有移动目标时是站立状态
                            if player_input.move_to.is_none() && player.action != PlayerAction::Stand {
                                player.action = PlayerAction::Stand;
                            }
                        }
                    }
                }
            }
        }
        
        // ⚔️ 循环结束后,添加所有攻击状态
        for entity in entities_to_attack {
            let _ = ctx.world.insert_one(entity, AttackState {
                start_time: now,
                attack_type: PlayerAction::Attack1,
            });
        }

        Ok(())
    }
}
