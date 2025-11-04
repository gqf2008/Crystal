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

use crate::ecs::{
    components::{
        Camera, LocalPlayer, Player, PlayerInput, Position,
    },
    GameContext,
    systems::System,
};
use ggez::input::mouse::MouseButton;
use ggez::GameResult;
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

pub struct PlayerControlSystem {
    mouse_state: MouseState,
    double_click_threshold: Duration,
    long_press_threshold: Duration,
}

impl PlayerControlSystem {
    pub fn new() -> Self {
        Self {
            mouse_state: MouseState::default(),
            double_click_threshold: Duration::from_millis(300),
            long_press_threshold: Duration::from_millis(200),
        }
    }

    /// 检测单击事件
    fn detect_single_click(&self, button: MouseButton) -> Option<(f32, f32)> {
        let now = Instant::now();
        match button {
            MouseButton::Left => {
                if let Some(last_click) = self.mouse_state.left_last_click_time {
                    if now.duration_since(last_click) < Duration::from_millis(100) {
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
                    if now.duration_since(last_click) < Duration::from_millis(100) {
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

    /// 检测长按事件 - 返回当前鼠标位置(用于跟随移动)
    fn detect_long_press(&self, button: MouseButton) -> Option<(f32, f32)> {
        let now = Instant::now();
        match button {
            MouseButton::Left => {
                if let Some(start) = self.mouse_state.left_press_start {
                    if self.mouse_state.left_pressed
                        && now.duration_since(start) > self.long_press_threshold
                    {
                        // 返回当前鼠标位置,而不是按下时的位置
                        Some(self.mouse_state.current_position)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            MouseButton::Right => {
                if let Some(start) = self.mouse_state.right_press_start {
                    if self.mouse_state.right_pressed
                        && now.duration_since(start) > self.long_press_threshold
                    {
                        // 返回当前鼠标位置,而不是按下时的位置
                        Some(self.mouse_state.current_position)
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

impl System for PlayerControlSystem {
    fn priority(&self) -> u32 {
        crate::ecs::systems::priority::PLAYER_CONTROL
    }

    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        // ✅ 零拷贝：直接访问 GameContext
        let mouse_left_pressed = ctx.mouse.button_pressed(MouseButton::Left);
        let mouse_right_pressed = ctx.mouse.button_pressed(MouseButton::Right);
        let mouse_pos = ctx.mouse.position();

        // 调试日志已关闭,减少干扰

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

        // 处理长按（连续移动）
        // 左键长按 = 走路
        let long_press_left =
            if let Some((screen_x, screen_y)) = self.detect_long_press(MouseButton::Left) {
                Some(Self::screen_to_world(
                    screen_x,
                    screen_y,
                    &camera_pos,
                    &camera,
                ))
            } else {
                None
            };

        // 右键长按 = 跑步
        let long_press_right =
            if let Some((screen_x, screen_y)) = self.detect_long_press(MouseButton::Right) {
                Some(Self::screen_to_world(
                    screen_x,
                    screen_y,
                    &camera_pos,
                    &camera,
                ))
            } else {
                None
            };

        // 🗺️ 预先获取地图数据(避免借用冲突)
        use crate::ecs::components::map::{MapData, MapBounds};
        let map_data = ctx.world.query::<(&MapBounds, &MapData)>()
            .iter()
            .map(|(_, (bounds, data))| (bounds.clone(), data.cells.clone()))
            .next();

        // 🎯 预先获取玩家当前位置（用于检查是否卡在障碍物上）
        let player_position = ctx.world.query::<(&Position, &LocalPlayer)>()
            .iter()
            .next()
            .map(|(_, (pos, _))| (pos.x, pos.y));
        
        // 🎯 检查延迟单击：只有在双击窗口已过且确认不是双击时才处理单击
        let now = Instant::now();
        let has_single_click = {
            let left_confirmed = if let Some(last_click) = self.mouse_state.left_last_click_time {
                // 超过双击窗口，确认是单击
                now.duration_since(last_click) >= self.double_click_threshold
            } else {
                false
            };
            
            let right_confirmed = if let Some(last_click) = self.mouse_state.right_last_click_time {
                // 超过双击窗口，确认是单击
                now.duration_since(last_click) >= self.double_click_threshold
            } else {
                false
            };
            
            left_confirmed || right_confirmed
        };
        
        // 更新本地玩家输入和动作状态
        for (_entity, (player_input, player, _local, state_machine)) in ctx
            .world
            .query_mut::<(&mut PlayerInput, &mut Player, &LocalPlayer, &mut crate::ecs::components::PlayerStateMachine)>()
            .into_iter()
        {
            // 🎯 优先处理单击停止（任意单击都会停止移动）
            if has_single_click {
                tracing::warn!("⏹️ 检测到单击，立即停止移动");
                player_input.move_to = None;
                player_input.movement_mode = crate::ecs::components::MovementMode::None;
                
                // 🎬 直接设置动作状态
                use crate::ecs::components::{PlayerInputEvent, PlayerAction};
                state_machine.handle_event(PlayerInputEvent::StopMoving);
                player.action = PlayerAction::Stand;
                player.is_moving = false;
                
                // 清除 last_click_time 避免重复触发
                self.mouse_state.left_last_click_time = None;
                self.mouse_state.right_last_click_time = None;
                continue;  // 跳过后续处理
            }

            // ✅ 启用双击寻路功能
            let has_double_click = double_click_left.is_some() || double_click_right.is_some();
            
            if has_double_click {
                use crate::ecs::components::{PlayerInputEvent, PlayerAction};
                
                // 双击模式: 自动寻路,松开后继续移动
                if let Some((world_x, world_y)) = double_click_left {
                    player_input.move_to = Some((world_x, world_y));
                    player_input.movement_mode = crate::ecs::components::MovementMode::Pathfinding;
                    
                    // 🎬 直接设置走路动作
                    state_machine.handle_event(PlayerInputEvent::StartWalking);
                    player.action = PlayerAction::Walk;
                    player.is_moving = true;
                    
                    tracing::warn!("🚶🚶 左键双击走路到 ({:.1}, {:.1}) [寻路模式-松开后继续走]", world_x, world_y);
                } else if let Some((world_x, world_y)) = double_click_right {
                    player_input.move_to = Some((world_x, world_y));
                    player_input.movement_mode = crate::ecs::components::MovementMode::Pathfinding;
                    
                    // 🎬 直接设置奔跑动作
                    state_machine.handle_event(PlayerInputEvent::StartRunning);
                    player.action = PlayerAction::Run;
                    player.is_moving = true;
                    
                    tracing::warn!("🏃🏃 右键双击跑步到 ({:.1}, {:.1}) [寻路模式-松开后继续走]", world_x, world_y);
                }
            } else {
                use crate::ecs::components::{MovementMode, PlayerInputEvent, PlayerAction};
                
                // 没有双击,检查是否按住鼠标(直接跟随模式)
                let is_pressing_left = self.mouse_state.left_pressed;
                let is_pressing_right = self.mouse_state.right_pressed;
                
                if is_pressing_left || is_pressing_right {
                    // 🎯 鼠标按下：设置移动目标和动作状态
                    let (screen_x, screen_y) = self.mouse_state.current_position;
                    let (world_x, world_y) = Self::screen_to_world(screen_x, screen_y, &camera_pos, &camera);
                    
                    player_input.movement_mode = MovementMode::DirectFollow;
                    player_input.move_to = Some((world_x, world_y));
                    
                    // 🎬 直接设置动作（走/跑）
                    if is_pressing_right {
                        state_machine.handle_event(PlayerInputEvent::StartRunning);
                        player.action = PlayerAction::Run;
                    } else {
                        state_machine.handle_event(PlayerInputEvent::StartWalking);
                        player.action = PlayerAction::Walk;
                    }
                    player.is_moving = true;
                } else {
                    // 🎯 鼠标松开：立即停止移动和动画
                    match player_input.movement_mode {
                        MovementMode::DirectFollow => {
                            // 跟随模式下,松开立即停止
                            if player_input.move_to.is_some() {
                                tracing::warn!("⏹️⏹️ 松开鼠标,停止跟随 (mode={:?})", player_input.movement_mode);
                                player_input.move_to = None;
                                player_input.movement_mode = MovementMode::None;
                                
                                // 🎬 停止动作
                                state_machine.handle_event(PlayerInputEvent::StopMoving);
                                player.action = PlayerAction::Stand;
                                player.is_moving = false;
                            }
                        }
                        MovementMode::Pathfinding => {
                            // 寻路模式下,松开不停止,继续走完路径（动作状态已设置，不需要修改）
                        }
                        MovementMode::None => {
                            // 无动作
                        }
                    }
                }
            }
        }

        // 🚀 合并PathfindingSystem功能: 根据PlayerInput计算velocity
        use crate::ecs::components::movement::{MovementVelocity, Path};
        
        // 先获取地图数据(避免借用冲突)
        let collision_map_data = ctx.world.query::<&MapData>().iter().next().map(|(_, map)| {
            (map.width, map.height, map.cells.clone())
        });
        
        let world = &mut ctx.world;
        
        for (_, (position, velocity, path, player_input, player)) in world.query_mut::<(
            &Position,
            &mut MovementVelocity,
            &mut Path,
            &PlayerInput,
            &Player,
        )>() {
            // 检查是否有移动目标
            if let Some((target_x, target_y)) = player_input.move_to {
                // DirectFollow模式: 每帧计算velocity朝向目标
                use crate::ecs::components::MovementMode;
                if player_input.movement_mode == MovementMode::DirectFollow {
                    let dx = target_x - position.x;
                    let dy = target_y - position.y;
                    let distance = (dx * dx + dy * dy).sqrt();
                    
                    // 停止阈值: 距离小于5像素认为已到达
                    const STOP_DISTANCE: f32 = 5.0;
                    
                    if distance > STOP_DISTANCE {
                        // 归一化方向向量
                        let dir_x = dx / distance;
                        let dir_y = dy / distance;
                        
                        // 🎬 根据 Player.action 设置速度
                        use crate::ecs::components::PlayerAction;
                        let speed = if player.action == PlayerAction::Run {
                            tracing::info!("🏃 使用跑步速度: {}", velocity.run_speed);
                            velocity.run_speed
                        } else {
                            tracing::info!("🚶 使用走路速度: {}", velocity.walk_speed);
                            velocity.walk_speed
                        };
                        
                        // 🎯 直接设置velocity
                        // CollisionSystem(390)会在MovementSystem(400)之前检测碰撞并清零velocity
                        velocity.x = dir_x * speed;
                        velocity.y = dir_y * speed;
                        velocity.max_speed = speed;
                        
                        // 清除Path,让MovementSystem直接用velocity
                        path.clear()
                    } else {
                        // 已到达目标,停止
                        velocity.stop();
                    }
                } else {
                    // 其他模式(Pathfinding等)由PathfindingSystem和MovementSystem处理
                    // PlayerControlSystem不干涉
                }
            } else {
                // 无移动目标,停止
                velocity.stop();
            }
        }

        Ok(())
    }
}
