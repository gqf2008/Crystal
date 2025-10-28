// ============================================================================
// Input Collecting System - 输入收集系统
// ============================================================================
//
// 职责（Layer 1: 输入与网络层）：
// - 捕获所有鼠标/键盘输入
// - 双击/长按检测
// - 转换为游戏命令，写入 PlayerInput
//
// 不负责：
// - ❌ 寻路计算（由 LocalPredictionSystem 调用 PathfindingService）
// - ❌ 移动逻辑（由 MovementSystem 处理）
// - ❌ 网络发送（由 ClientNetworkSystem 处理）
//
// ============================================================================

use hecs::World;
use ggez::winit::keyboard::KeyCode;
use ggez::winit::event::MouseButton;
use tokio::sync::mpsc;

use crate::network::NetworkCommand;
use crate::ecs::components::{
    PlayerInput, MouseInput, LocalPlayer, Camera, Position,
};
use crate::ecs::Coordinates;

/// 输入收集系统
pub struct InputCollectingSystem;

impl InputCollectingSystem {
    /// 处理鼠标按下事件
    pub fn process_mouse_down(
        world: &mut World,
        button: MouseButton,
        x: f32,
        y: f32,
    ) {
        if let Some((_, mouse_input)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
            match button {
                MouseButton::Left => {
                    mouse_input.left_pressed = true;
                    mouse_input.left_press_time = 0;
                }
                MouseButton::Right => {
                    mouse_input.right_pressed = true;
                    mouse_input.right_press_time = 0;
                }
                _ => {}
            }
        }
    }
    
    /// 处理鼠标抬起事件（检测双击）
    pub fn process_mouse_up(
        world: &mut World,
        button: MouseButton,
        x: f32,
        y: f32,
    ) {
        use std::time::Instant;
        
        if let Some((_, mouse_input)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
            mouse_input.x = x;
            mouse_input.y = y;
            
            match button {
                MouseButton::Left => {
                    // 检测双击
                    if mouse_input.left_press_time < 30 {
                        let now = Instant::now();
                        let time_since_last_click = now.duration_since(mouse_input.left_last_click_time);
                        
                        if time_since_last_click < std::time::Duration::from_millis(500) {
                            mouse_input.left_double_clicked = true;
                            mouse_input.left_last_click_time = now - std::time::Duration::from_secs(10);
                        } else {
                            mouse_input.left_last_click_time = now;
                            mouse_input.left_double_clicked = false;
                        }
                    }
                    mouse_input.left_pressed = false;
                    mouse_input.left_press_time = 0;
                }
                MouseButton::Right => {
                    // 检测双击
                    if mouse_input.right_press_time < 30 {
                        let now = Instant::now();
                        let time_since_last_click = now.duration_since(mouse_input.right_last_click_time);
                        
                        if time_since_last_click < std::time::Duration::from_millis(500) {
                            mouse_input.right_double_clicked = true;
                            mouse_input.right_last_click_time = now - std::time::Duration::from_secs(10);
                        } else {
                            mouse_input.right_last_click_time = now;
                            mouse_input.right_double_clicked = false;
                        }
                    }
                    mouse_input.right_pressed = false;
                    mouse_input.right_press_time = 0;
                }
                _ => {}
            }
        }
    }
    
    /// 处理鼠标移动
    pub fn process_mouse_move(
        world: &mut World,
        x: f32,
        y: f32,
    ) {
        if let Some((_, mouse_input)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
            mouse_input.x = x;
            mouse_input.y = y;
        }
    }
    
    /// 🎯 核心：更新玩家输入组件（每帧调用）
    /// 
    /// 将原始输入转换为游戏命令
    pub fn update(world: &mut World, _ctx: &mut ggez::Context) {
        // TODO: 未来可以从 ctx 获取输入状态
        // 目前使用 MouseInput 组件
        
        // 获取鼠标输入状态
        let mouse_input = world.query_mut::<&MouseInput>()
            .into_iter()
            .next()
            .map(|(_, input)| input.clone());
        
        let mouse_input = match mouse_input {
            Some(input) => input,
            None => return,
        };
        
        // 获取相机信息（用于坐标转换）
        let (camera_pos, camera) = world.query_mut::<(&Position, &Camera)>()
            .into_iter()
            .next()
            .map(|(_, (pos, cam))| (pos.clone(), cam.clone()))
            .unwrap_or((Position { x: 0.0, y: 0.0 }, Camera { zoom: 1.0, screen_width: 1280.0, screen_height: 720.0 }));
        
        // 🎯 处理本地玩家的输入
        for (_entity, (_, player_input)) in world.query_mut::<(&LocalPlayer, &mut PlayerInput)>() {
            // 清除上一帧的输入
            player_input.clear();
            
            // 1. 双击 → 移动指令（自动寻路）
            if mouse_input.left_double_clicked || mouse_input.right_double_clicked {
                let is_running = mouse_input.right_double_clicked;
                
                // 屏幕坐标转世界坐标
                let world_x = camera_pos.x + (mouse_input.x - camera.screen_width / 2.0) / camera.zoom;
                let world_y = camera_pos.y + (mouse_input.y - camera.screen_height / 2.0) / camera.zoom;
                
                // 写入移动指令（使用寻路）
                player_input.set_move((world_x, world_y), is_running);
                
                tracing::info!("🖱️ 双击移动（寻路）: ({:.1}, {:.1}) 跑={}", world_x, world_y, is_running);
            }
            
            // 2. 长按 → 直接跟随指令（不使用寻路，每帧更新目标）
            else if (mouse_input.left_pressed && mouse_input.left_press_time >= 5) 
                  || (mouse_input.right_pressed && mouse_input.right_press_time >= 5) {
                let is_running = mouse_input.right_pressed;
                
                // 屏幕坐标转世界坐标
                let world_x = camera_pos.x + (mouse_input.x - camera.screen_width / 2.0) / camera.zoom;
                let world_y = camera_pos.y + (mouse_input.y - camera.screen_height / 2.0) / camera.zoom;
                
                // 写入直接跟随指令（不使用寻路）
                player_input.set_follow((world_x, world_y), is_running);
            }
        }
        
        // 🎯 更新鼠标按下时间
        Self::update_mouse_timers(world);
    }
    
    /// 更新鼠标计时器（每帧）
    fn update_mouse_timers(world: &mut World) {
        if let Some((_, mouse_input)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
            // 更新长按计时器
            if mouse_input.left_pressed {
                mouse_input.left_press_time += 1;
            }
            if mouse_input.right_pressed {
                mouse_input.right_press_time += 1;
            }
            
            // 清除双击事件（单帧事件）
            mouse_input.left_double_clicked = false;
            mouse_input.right_double_clicked = false;
        }
    }
    
    /// 处理键盘输入（快捷键等）
    pub fn process_keyboard(
        world: &mut World,
        keycode: KeyCode,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) {
        // ❌ 此方法已废弃
        // 键盘处理已迁移到 KeyboardShortcutSystem (Layer 5)
        // 此占位符保持接口兼容性
        tracing::warn!("⚠️ InputCollectingSystem::process_keyboard is deprecated, use KeyboardShortcutSystem");
    }
}
