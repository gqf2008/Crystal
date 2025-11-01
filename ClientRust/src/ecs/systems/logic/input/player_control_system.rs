// ============================================================================
// Player Control System - 玩家控制系统 (重构版)
// ============================================================================
//
// **新职责** (GlobalEvents 架构):
// - 从 GlobalEvents 读取鼠标/键盘事件
// - 双击检测 → 生成移动命令（自动寻路）
// - 长按检测 → 生成跟随命令（直接移动）
// - 屏幕坐标 → 世界坐标转换
// - 将处理后的命令写入 PlayerInput
//
// **数据流**:
// ```
// GlobalEvents.mouse_events (Vec<MouseEvent>)
//     ↓
// PlayerControlSystem::update()
//     ↓ (双击检测/长按检测)
// PlayerInput 组件
//     ↓
// 其他系统读取 PlayerInput
// ```
//
// **不负责**:
// - ❌ 实际移动（由 MovementSystem 处理）
// - ❌ 寻路计算（由 PathfindingService 处理）
// - ❌ 网络发送（由 NetworkSendSystem 处理）
//
// ============================================================================

use crate::ecs::WorldExt;
use crate::ecs::components::{
    Camera, GameEvent, GlobalEvents, InputEvent, LocalPlayer, MonsterData, MoveMode, NPCData,
    Player, PlayerAction, PlayerInput, Position,
};
use crate::ecs::systems::System;
use ggez::winit::event::MouseButton;
use ggez::GameResult;
use hecs::World;
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

/// 玩家控制系统 (重构版)
///
/// 从 GlobalEvents 读取输入事件并生成玩家命令
pub struct PlayerControlSystem {
    mouse_state: MouseState,
    double_click_threshold: Duration,
    long_press_threshold: Duration,
}

impl PlayerControlSystem {
    pub fn new() -> Self {
        Self {
            mouse_state: MouseState::default(),
            double_click_threshold: Duration::from_millis(500),
            long_press_threshold: Duration::from_millis(300), // 约5帧 @ 60fps
        }
    }

    /// 处理鼠标事件并更新内部状态
    fn process_mouse_events(&mut self, events: &[InputEvent]) {
        for event in events {
            match event {
                InputEvent::MouseDown { button, x, y } => {
                    let now = Instant::now();
                    match button {
                        MouseButton::Left => {
                            self.mouse_state.left_pressed = true;
                            self.mouse_state.left_press_start = Some(now);
                            self.mouse_state.left_press_position = Some((*x, *y));
                        }
                        MouseButton::Right => {
                            self.mouse_state.right_pressed = true;
                            self.mouse_state.right_press_start = Some(now);
                            self.mouse_state.right_press_position = Some((*x, *y));
                        }
                        _ => {}
                    }
                }
                InputEvent::MouseUp { button, x, y } => {
                    let now = Instant::now();
                    match button {
                        MouseButton::Left => {
                            self.mouse_state.left_pressed = false;
                            // 检测双击
                            if let Some(last_click) = self.mouse_state.left_last_click_time {
                                if now.duration_since(last_click) < self.double_click_threshold {
                                    // 双击！
                                    tracing::debug!("🖱️ 检测到左键双击 at ({:.1}, {:.1})", x, y);
                                    // 双击事件将在 update 中处理
                                    self.mouse_state.left_last_click_time = None;
                                // 防止三击
                                } else {
                                    self.mouse_state.left_last_click_time = Some(now);
                                }
                            } else {
                                self.mouse_state.left_last_click_time = Some(now);
                            }
                            self.mouse_state.left_press_start = None;
                        }
                        MouseButton::Right => {
                            self.mouse_state.right_pressed = false;
                            // 检测双击
                            if let Some(last_click) = self.mouse_state.right_last_click_time {
                                if now.duration_since(last_click) < self.double_click_threshold {
                                    // 双击！
                                    tracing::debug!("🖱️ 检测到右键双击 at ({:.1}, {:.1})", x, y);
                                    self.mouse_state.right_last_click_time = None;
                                } else {
                                    self.mouse_state.right_last_click_time = Some(now);
                                }
                            } else {
                                self.mouse_state.right_last_click_time = Some(now);
                            }
                            self.mouse_state.right_press_start = None;
                        }
                        _ => {}
                    }
                }
                InputEvent::MouseMove { x, y, dx, dy } => {
                    self.mouse_state.current_position = (*x, *y);
                }
                _ => {}
            }
        }
    }

    /// 检测双击事件
    fn detect_double_click(&self, button: MouseButton) -> Option<(f32, f32)> {
        let now = Instant::now();
        match button {
            MouseButton::Left => {
                if let Some(last_click) = self.mouse_state.left_last_click_time {
                    if now.duration_since(last_click) < Duration::from_millis(100) {
                        // 刚刚发生了双击
                        return self.mouse_state.left_press_position;
                    }
                }
            }
            MouseButton::Right => {
                if let Some(last_click) = self.mouse_state.right_last_click_time {
                    if now.duration_since(last_click) < Duration::from_millis(100) {
                        return self.mouse_state.right_press_position;
                    }
                }
            }
            _ => {}
        }
        None
    }

    /// 检测长按事件
    fn detect_long_press(&self, button: MouseButton) -> Option<(f32, f32)> {
        let now = Instant::now();
        match button {
            MouseButton::Left => {
                if self.mouse_state.left_pressed {
                    if let Some(start) = self.mouse_state.left_press_start {
                        if now.duration_since(start) >= self.long_press_threshold {
                            return Some(self.mouse_state.current_position);
                        }
                    }
                }
            }
            MouseButton::Right => {
                if self.mouse_state.right_pressed {
                    if let Some(start) = self.mouse_state.right_press_start {
                        if now.duration_since(start) >= self.long_press_threshold {
                            return Some(self.mouse_state.current_position);
                        }
                    }
                }
            }
            _ => {}
        }
        None
    }

    /// 屏幕坐标转世界坐标
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

    /// 检测单击事件（用于 NPC/怪物点击）
    ///
    /// 单击定义：按下后快速释放，且没有触发双击/长按
    fn detect_single_click(&self, button: MouseButton) -> Option<(f32, f32)> {
        let now = Instant::now();
        let single_click_max_duration = Duration::from_millis(300); // 最大按住时间

        match button {
            MouseButton::Left => {
                // 检查是否刚释放
                if !self.mouse_state.left_pressed {
                    // 检查按住时长是否小于阈值
                    if let Some(start) = self.mouse_state.left_press_start {
                        if now.duration_since(start) < single_click_max_duration {
                            // 确保不是双击的第二次点击
                            if let Some(last_click) = self.mouse_state.left_last_click_time {
                                if now.duration_since(last_click) > self.double_click_threshold {
                                    return self.mouse_state.left_press_position;
                                }
                            } else {
                                return self.mouse_state.left_press_position;
                            }
                        }
                    }
                }
            }
            MouseButton::Right => {
                if !self.mouse_state.right_pressed {
                    if let Some(start) = self.mouse_state.right_press_start {
                        if now.duration_since(start) < single_click_max_duration {
                            if let Some(last_click) = self.mouse_state.right_last_click_time {
                                if now.duration_since(last_click) > self.double_click_threshold {
                                    return self.mouse_state.right_press_position;
                                }
                            } else {
                                return self.mouse_state.right_press_position;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        None
    }

    /// 处理实体点击（NPC/怪物）
    ///
    /// 优先级：NPC > Monster
    fn handle_entity_click(world: &World, world_x: f32, world_y: f32) -> Option<GameEvent> {
        const CLICK_RADIUS: f32 = 32.0;

        // 1. 检查 NPC（优先级高）
        for (_entity, (npc, pos)) in world.query::<(&NPCData, &Position)>().iter() {
            let dx = pos.x - world_x;
            let dy = pos.y - world_y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance < CLICK_RADIUS {
                tracing::info!("🏪 点击NPC: {} (ID: {})", npc.name, npc.id);
                return Some(GameEvent::NPCCallRequest {
                    npc_object_id: npc.id,
                });
            }
        }

        // 2. 检查怪物
        for (_entity, (monster, pos)) in world.query::<(&MonsterData, &Position)>().iter() {
            let dx = pos.x - world_x;
            let dy = pos.y - world_y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance < CLICK_RADIUS {
                tracing::info!("👹 点击怪物: {} (ID: {})", monster.name, monster.id);
                // TODO: 设置攻击目标或选中目标
                // 暂时不返回网络命令，等待攻击系统实现
                return None;
            }
        }

        None
    }

    /// 处理玩家输入并生成命令
    fn process_player_input(&mut self, world: &mut World, events: &GlobalEvents) -> GameResult {
        // 1. 处理鼠标事件
        self.process_mouse_events(&events.input_events);

        // 2. 获取相机信息（用于坐标转换）
        let (camera_pos, camera) = world
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

        // 3. 处理本地玩家的命令
        for (_entity, (_, player_input, player)) in
            world.query_mut::<(&LocalPlayer, &mut PlayerInput, &mut Player)>()
        {
            // 清除上一帧的输入
            player_input.clear();

            // 4. 检测双击 → 移动命令（使用寻路）
            if let Some((screen_x, screen_y)) = self
                .detect_double_click(MouseButton::Left)
                .or_else(|| self.detect_double_click(MouseButton::Right))
            {
                let is_running = self.detect_double_click(MouseButton::Right).is_some();
                let (world_x, world_y) =
                    Self::screen_to_world(screen_x, screen_y, &camera_pos, &camera);

                // 写入移动命令
                player_input.set_move((world_x, world_y), is_running);

                // 更新玩家状态
                player.target_x = world_x;
                player.target_y = world_y;
                player.is_moving = true;
                player.move_mode = MoveMode::AutoPathfinding;
                player.action = if is_running && player.can_run {
                    PlayerAction::Run
                } else {
                    PlayerAction::Walk
                };

                tracing::info!(
                    "🖱️ 双击移动（寻路）: 世界坐标({:.1}, {:.1}), 跑步={}",
                    world_x,
                    world_y,
                    is_running
                );
            }
            // 5. 检测长按 → 跟随命令（不使用寻路）
            else if let Some((screen_x, screen_y)) = self
                .detect_long_press(MouseButton::Left)
                .or_else(|| self.detect_long_press(MouseButton::Right))
            {
                let is_running = self.detect_long_press(MouseButton::Right).is_some();
                let (world_x, world_y) =
                    Self::screen_to_world(screen_x, screen_y, &camera_pos, &camera);

                // 写入跟随命令
                player_input.set_follow((world_x, world_y), is_running);

                // 更新玩家状态
                player.target_x = world_x;
                player.target_y = world_y;
                player.is_moving = true;
                player.move_mode = MoveMode::DirectFollow;
                player.action = if is_running && player.can_run {
                    PlayerAction::Run
                } else {
                    PlayerAction::Walk
                };

                // 长按每帧都更新，不需要频繁打印日志
            }
            // 6. 没有输入时切换到站立
            else if !player.is_moving {
                player.action = PlayerAction::Stand;
                player.move_mode = MoveMode::Idle;
            }
        }

        Ok(())
    }
}

impl Default for PlayerControlSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for PlayerControlSystem {
    fn name(&self) -> &'static str {
        "PlayerControlSystem"
    }

    fn priority(&self) -> u32 {
        crate::ecs::systems::priority::PLAYER_CONTROL
    }

    fn update(&mut self, world: &mut hecs::World, _delay_time: f32) -> GameResult {
        // 从 GlobalEvents 读取鼠标事件（clone以避免二次可变借用）
        let mouse_events = {
            let mut query = world.query::<&GlobalEvents>();
            if let Some((_, events)) = query.iter().next() {
                events.input_events.clone()
            } else {
                tracing::warn!("⚠️ PlayerControlSystem: GlobalEvents 组件未找到");
                return Ok(());
            }
        };

        // 处理鼠标事件（更新内部状态）
        self.process_mouse_events(&mouse_events);

        // 获取相机信息（用于坐标转换）
        let (camera_pos, camera) = world
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

        // 检测单击并处理实体点击（在循环外先处理，避免借用冲突）
        let single_click_command =
            if let Some((screen_x, screen_y)) = self.detect_single_click(MouseButton::Left) {
                let (world_x, world_y) =
                    Self::screen_to_world(screen_x, screen_y, &camera_pos, &camera);
                Self::handle_entity_click(world, world_x, world_y)
            } else {
                None
            };

        // 发送网络命令（如果有）
        if let Some(command) = single_click_command {
           world.network().send(command).ok();
        }

        // 处理本地玩家的命令
        for (_entity, (_, player_input, player)) in
            world.query_mut::<(&LocalPlayer, &mut PlayerInput, &mut Player)>()
        {
            // 清除上一帧的输入
            player_input.clear();

            // 检测双击 → 移动命令（使用寻路）
            if let Some((screen_x, screen_y)) = self
                .detect_double_click(MouseButton::Left)
                .or_else(|| self.detect_double_click(MouseButton::Right))
            {
                let is_running = self.detect_double_click(MouseButton::Right).is_some();
                let (world_x, world_y) =
                    Self::screen_to_world(screen_x, screen_y, &camera_pos, &camera);

                // 写入移动命令
                player_input.set_move((world_x, world_y), is_running);

                // 更新玩家状态
                player.target_x = world_x;
                player.target_y = world_y;
                player.is_moving = true;
                player.move_mode = MoveMode::AutoPathfinding;
                player.action = if is_running && player.can_run {
                    PlayerAction::Run
                } else {
                    PlayerAction::Walk
                };

                tracing::info!(
                    "🖱️ 双击移动（寻路）: 世界坐标({:.1}, {:.1}), 跑步={}",
                    world_x,
                    world_y,
                    is_running
                );
            }
            // 检测长按 → 跟随命令（不使用寻路）
            else if let Some((screen_x, screen_y)) = self
                .detect_long_press(MouseButton::Left)
                .or_else(|| self.detect_long_press(MouseButton::Right))
            {
                let is_running = self.detect_long_press(MouseButton::Right).is_some();
                let (world_x, world_y) =
                    Self::screen_to_world(screen_x, screen_y, &camera_pos, &camera);

                // 写入跟随命令
                player_input.set_follow((world_x, world_y), is_running);

                // 更新玩家状态
                player.target_x = world_x;
                player.target_y = world_y;
                player.is_moving = true;
                player.move_mode = MoveMode::DirectFollow;
                player.action = if is_running && player.can_run {
                    PlayerAction::Run
                } else {
                    PlayerAction::Walk
                };

                tracing::info!(
                    "🖱️ 长按跟随: 世界坐标({:.1}, {:.1}), 跑步={}",
                    world_x,
                    world_y,
                    is_running
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_control_system_creation() {
        let system = PlayerControlSystem::new();
        assert_eq!(
            system.priority(),
            crate::ecs::systems::priority::PLAYER_CONTROL
        );
        assert_eq!(system.double_click_threshold, Duration::from_millis(500));
        assert_eq!(system.long_press_threshold, Duration::from_millis(300));
    }

    #[test]
    fn test_screen_to_world_conversion() {
        let camera_pos = Position {
            x: 1000.0,
            y: 500.0,
        };
        let camera = Camera {
            zoom: 1.0,
            screen_width: 1280.0,
            screen_height: 720.0,
        };

        // 中心点应该等于相机位置
        let (world_x, world_y) =
            PlayerControlSystem::screen_to_world(640.0, 360.0, &camera_pos, &camera);
        assert_eq!(world_x, 1000.0);
        assert_eq!(world_y, 500.0);
    }
}
