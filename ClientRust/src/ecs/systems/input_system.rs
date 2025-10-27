// ============================================================================
// 输入系统 - 统一处理所有键盘、鼠标输入
// ============================================================================
//
// 职责：
// - 键盘输入处理和快捷键分发
// - 鼠标点击、移动、滚轮事件处理
// - UI 焦点检测
// - 输入事件分发到各个系统
//
// 设计原则：
// - 遵循 ECS 架构，所有逻辑通过查询 World 组件实现
// - 解耦输入处理和业务逻辑
// - 支持动态快捷键配置（预留）
//
// ============================================================================

use hecs::World;
use ggez::winit::keyboard::KeyCode;  // 🔧 使用 winit 的 KeyCode
use ggez::winit::event::MouseButton;
use tokio::sync::mpsc;

use crate::network::NetworkCommand;
use crate::ecs::systems::{
    ItemSystem, MagicCastSystem, NPCSystem, UISystem,
};
use crate::ecs::ui::DialogType;

/// 输入系统 - 负责处理所有键盘、鼠标输入
pub struct InputSystem;

impl InputSystem {
    /// 处理键盘输入
    /// 
    /// # 参数
    /// - `world`: ECS 世界
    /// - `keycode`: 按键代码
    /// - `network_tx`: 网络命令发送器
    pub fn process_keyboard(
        world: &mut World,
        keycode: KeyCode,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) {
        // 调试日志
        static mut KEY_COUNT: u32 = 0;
        unsafe {
            if KEY_COUNT < 10 {
                println!("🎹 键盘输入: {:?}", keycode);
                KEY_COUNT += 1;
            }
        }
        
        // 检查是否有 UI 焦点（如果有输入框激活，不处理游戏快捷键）
        if Self::has_text_input_focus(world) {
            // 文本输入激活时，只处理 ESC 关闭
            if keycode == KeyCode::Escape {
                Self::close_text_input(world);
            }
            return;
        }
        
        // ✅ 统一处理所有快捷键，不区分是否有对话框打开
        // F1-F8、1-8 等快捷键应该始终可用
        Self::handle_game_keyboard(world, keycode, network_tx);
    }
    
    /// 处理游戏世界快捷键
    fn handle_game_keyboard(
        world: &mut World,
        keycode: KeyCode,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) {
        use KeyCode::*;
        
        match keycode {
            // === UI 对话框快捷键 ===
            KeyI => UISystem::toggle_dialog(world, DialogType::Inventory),
            KeyC => UISystem::toggle_dialog(world, DialogType::Character),
            KeyS => UISystem::toggle_dialog(world, DialogType::Skills),
            KeyK => UISystem::toggle_dialog(world, DialogType::MagicLearning),
            KeyQ => UISystem::toggle_dialog(world, DialogType::Quest),
            KeyT => UISystem::toggle_dialog(world, DialogType::Trade),
            
            // === 游戏操作快捷键 ===
            
            // 空格键 - 拾取物品
            Space => {
                Self::pickup_item(world, network_tx);
            }
            
            // Z键 - 整理背包
            KeyZ => {
                ItemSystem::organize_inventory(world);
                tracing::info!("📦 整理背包");
            }
            
            // N键 - 与最近的NPC对话
            KeyN => {
                if let Some(npc_id) = NPCSystem::find_nearest_npc(world) {
                    NPCSystem::click_npc(world, npc_id, network_tx);
                } else {
                    tracing::warn!("⚠️ 附近没有NPC");
                }
            }
            
            // Tab键 - 切换目标
            Tab => {
                MagicCastSystem::cycle_target(world);
            }
            
            // === 技能快捷键 F1-F8 ===
            F1 => Self::cast_spell_in_slot(world, 0, network_tx),
            F2 => Self::cast_spell_in_slot(world, 1, network_tx),
            F3 => Self::cast_spell_in_slot(world, 2, network_tx),
            F4 => Self::cast_spell_in_slot(world, 3, network_tx),
            F5 => Self::cast_spell_in_slot(world, 4, network_tx),
            F6 => Self::cast_spell_in_slot(world, 5, network_tx),
            F7 => Self::cast_spell_in_slot(world, 6, network_tx),
            F8 => Self::cast_spell_in_slot(world, 7, network_tx),
            
            // === 物品快捷键 1-8 ===
            Digit1 => {ItemSystem::use_item(world, 0, network_tx);}
            Digit2 => {ItemSystem::use_item(world, 1, network_tx);}
            Digit3 => {ItemSystem::use_item(world, 2, network_tx);}
            Digit4 => {ItemSystem::use_item(world, 3, network_tx);}
            Digit5 => {ItemSystem::use_item(world, 4, network_tx);}
            Digit6 => {ItemSystem::use_item(world, 5, network_tx);}
            Digit7 => {ItemSystem::use_item(world, 6, network_tx);}
            Digit8 => {ItemSystem::use_item(world, 7, network_tx);}
            
            // === 调试快捷键 ===
            KeyB => Self::toggle_debug_borders(world),
            F9 => Self::toggle_npc_borders(world),      // F9键 - NPC边框(青色)
            F10 => Self::toggle_monster_borders(world), // F10键 - Monster边框(紫色)
            F11 => Self::toggle_effect_borders(world),  // F11键 - 特效边框(绿色)
            KeyG => Self::toggle_debug_grid(world),
            KeyO => Self::toggle_debug_obstacles(world),
            KeyP => Self::toggle_debug_path(world),
            
            _ => {}
        }
    }
    
    /// 处理鼠标点击
    /// 
    /// # 参数
    /// - `world`: ECS 世界
    /// - `button`: 鼠标按钮
    /// - `ui_x, ui_y`: UI 设计坐标 (1024×768)
    /// - `window_x, window_y`: 窗口逻辑坐标
    /// - `network_tx`: 网络命令发送器
    /// 
    /// # 返回
    /// - `true`: 事件被 UI 消费
    /// - `false`: 事件传递到游戏世界
    pub fn process_mouse_click(
        world: &mut World,
        button: MouseButton,
        ui_x: f32,
        ui_y: f32,
        window_x: f32,
        window_y: f32,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> bool {
        // 先检查 UI 层点击
        if UISystem::handle_click(world, button, ui_x, ui_y) {
            return true; // UI 消费了事件
        }
        
        // UI 未消费，传递到游戏世界
        Self::handle_world_click(world, button, window_x, window_y, network_tx);
        false
    }
    
    /// 处理游戏世界点击
    fn handle_world_click(
        world: &mut World,
        button: MouseButton,
        x: f32,
        y: f32,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) {
        use crate::ecs::components::{MouseInput, Position, Camera, NPCData, MonsterData, LocalPlayer};
        use crate::ecs::systems::CameraSystem;
        
        // 左键点击检测NPC/怪物
        if button == MouseButton::Left {
            // 获取相机和玩家位置
            let (camera_pos, camera) = {
                let mut camera_query = world.query::<(&Position, &Camera)>();
                if let Some((_, (pos, cam))) = camera_query.into_iter().next() {
                    (pos.clone(), cam.clone())
                } else {
                    return;
                }
            };
            
            // 将屏幕坐标转换为世界坐标
            let world_pos = CameraSystem::screen_to_world(&camera_pos, &camera, x, y);
            
            // 检查是否点击了NPC (优先级高于怪物)
            let mut clicked_npc_id: Option<u32> = None;
            let click_radius = 32.0; // 点击范围
            
            for (_entity, (npc, pos)) in world.query::<(&NPCData, &Position)>().iter() {
                let dx = pos.x - world_pos.0;
                let dy = pos.y - world_pos.1;
                let distance = (dx * dx + dy * dy).sqrt();
                
                if distance < click_radius {
                    clicked_npc_id = Some(npc.id);
                    tracing::info!("🏪 点击NPC: {} (ID: {})", npc.name, npc.id);
                    break;
                }
            }
            
            // 如果点击了NPC,发送NPCRequest
            if let Some(npc_id) = clicked_npc_id {
                if let Err(e) = network_tx.send(NetworkCommand::NPCRequest { npc_object_id: npc_id }) {
                    tracing::error!("❌ 发送NPC请求失败: {}", e);
                }
                return; // 不继续处理怪物点击
            }
            
            // 检查是否点击了怪物
            for (_entity, (monster, pos)) in world.query::<(&MonsterData, &Position)>().iter() {
                let dx = pos.x - world_pos.0;
                let dy = pos.y - world_pos.1;
                let distance = (dx * dx + dy * dy).sqrt();
                
                if distance < click_radius {
                    tracing::info!("👹 点击怪物: {} (ID: {})", monster.name, monster.id);
                    // TODO: 设置攻击目标
                    break;
                }
            }
        }
        
        // 更新鼠标输入状态
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
    
    /// 处理鼠标抬起
    pub fn process_mouse_up(
        world: &mut World,
        button: MouseButton,
        x: f32,
        y: f32,
    ) {
        use crate::ecs::components::MouseInput;
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
                            tracing::debug!("👆👆 左键双击 at ({:.1}, {:.1})", x, y);
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
                            tracing::debug!("👆👆 右键双击 at ({:.1}, {:.1})", x, y);
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
        ui_x: f32,
        ui_y: f32,
        window_x: f32,
        window_y: f32,
    ) {
        use crate::ecs::components::MouseInput;
        
        // 更新鼠标位置
        if let Some((_, mouse_input)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
            mouse_input.x = window_x;
            mouse_input.y = window_y;
        }
        
        // 更新 UI hover 状态
        UISystem::update_hover(world, ui_x, ui_y);
    }
    
    /// 处理鼠标滚轮
    pub fn process_mouse_wheel(
        world: &mut World,
        camera_entity: hecs::Entity,
        _x: f32,
        y: f32,
    ) {
        use crate::ecs::components::Camera;
        
        const ZOOM_SPEED: f32 = 0.1;
        const MIN_ZOOM: f32 = 0.5;
        const MAX_ZOOM: f32 = 2.0;
        
        if let Ok(mut camera) = world.get::<&mut Camera>(camera_entity) {
            let zoom_delta = y * ZOOM_SPEED;
            camera.zoom = (camera.zoom + zoom_delta).clamp(MIN_ZOOM, MAX_ZOOM);
            tracing::debug!("🔍 缩放: {:.1}x", camera.zoom);
        }
    }
    
    /// 更新鼠标输入状态（每帧调用）
    /// 用于更新长按计时器和清除双击事件
    pub fn update_mouse_input(world: &mut World) {
        use crate::ecs::components::MouseInput;
        
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
    
    // ========================================================================
    // 辅助方法
    // ========================================================================
    
    /// 检查是否有文本输入焦点
    fn has_text_input_focus(world: &World) -> bool {
        use crate::ecs::ui::ChatDialog;
        
        // 检查聊天输入框是否激活
        for (_, chat) in world.query::<&ChatDialog>().iter() {
            if chat.is_input_active() {
                return true;
            }
        }
        false
    }
    
    /// 关闭文本输入
    fn close_text_input(world: &mut World) {
        use crate::ecs::ui::ChatDialog;
        
        for (_, chat) in world.query_mut::<&mut ChatDialog>() {
            chat.deactivate_input();
        }
    }
    
    /// 拾取物品
    fn pickup_item(world: &mut World, network_tx: &mpsc::UnboundedSender<NetworkCommand>) {
        use crate::ecs::components::Position;
        use crate::ecs::Coordinates;
        
        if let Some((_, pos)) = world.query::<&Position>().iter().next() {
            let (grid_x, grid_y) = Coordinates::world_to_grid(pos.x, pos.y);
            let _ = network_tx.send(NetworkCommand::PickupItem {
                location: (grid_x, grid_y),
            });
            tracing::info!("📦 拾取物品 at ({}, {})", grid_x, grid_y);
        }
    }
    
    /// 施放技能栏中的技能
    fn cast_spell_in_slot(
        world: &mut World,
        slot: usize,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) {
        use crate::ecs::components::{LocalPlayer, MagicList};
        
        // 从技能栏获取技能
        let spell = {
            let mut spell_opt = None;
            for (_, (_, magic_list)) in world.query::<(&LocalPlayer, &MagicList)>().iter() {
                if let Some(learned_magic) = magic_list.get_by_slot(slot as u8) {
                    spell_opt = Some(learned_magic.spell);
                }
                break;
            }
            spell_opt
        };
        
        if let Some(spell_type) = spell {
            MagicCastSystem::cast_spell(world, spell_type, network_tx);
        } else {
            tracing::debug!("⚠️ 技能栏 F{} 未绑定技能", slot + 1);
        }
    }
    
    /// 切换调试边框显示
    fn toggle_debug_borders(world: &mut World) {
        use crate::ecs::components::RenderConfig;
        
        for (_, config) in world.query_mut::<&mut RenderConfig>() {
            config.show_borders = !config.show_borders;
            tracing::info!("🖼️ 纹理边框 (B): {}", if config.show_borders { "显示" } else { "隐藏" });
            break;
        }
    }
    
    /// 切换NPC边框显示
    fn toggle_npc_borders(world: &mut World) {
        use crate::ecs::components::RenderConfig;
        
        for (_, config) in world.query_mut::<&mut RenderConfig>() {
            config.show_npc_borders = !config.show_npc_borders;
            tracing::info!("👤 NPC边框 (F9): {}", if config.show_npc_borders { "显示" } else { "隐藏" });
            break;
        }
    }
    
    /// 切换Monster边框显示
    fn toggle_monster_borders(world: &mut World) {
        use crate::ecs::components::RenderConfig;
        
        for (_, config) in world.query_mut::<&mut RenderConfig>() {
            config.show_monster_borders = !config.show_monster_borders;
            tracing::info!("👾 Monster边框 (F10): {}", if config.show_monster_borders { "显示" } else { "隐藏" });
            break;
        }
    }
    
    /// 切换特效边框显示
    fn toggle_effect_borders(world: &mut World) {
        use crate::ecs::components::RenderConfig;
        
        for (_, config) in world.query_mut::<&mut RenderConfig>() {
            config.show_effect_borders = !config.show_effect_borders;
            tracing::info!("✨ 特效边框 (F11): {}", if config.show_effect_borders { "显示" } else { "隐藏" });
            break;
        }
    }
    
    /// 切换调试网格显示
    fn toggle_debug_grid(world: &mut World) {
        use crate::ecs::components::RenderConfig;
        
        for (_, config) in world.query_mut::<&mut RenderConfig>() {
            config.show_grid = !config.show_grid;
            tracing::info!("📐 网格 (G): {}", if config.show_grid { "显示" } else { "隐藏" });
            break;
        }
    }
    
    /// 切换调试障碍物显示
    fn toggle_debug_obstacles(world: &mut World) {
        use crate::ecs::components::RenderConfig;
        
        for (_, config) in world.query_mut::<&mut RenderConfig>() {
            config.show_obstacles = !config.show_obstacles;
            tracing::info!("🚧 障碍物 (O): {}", if config.show_obstacles { "显示" } else { "隐藏" });
            break;
        }
    }
    
    /// 切换调试路径显示
    fn toggle_debug_path(world: &mut World) {
        use crate::ecs::components::RenderConfig;
        
        for (_, config) in world.query_mut::<&mut RenderConfig>() {
            config.show_path = !config.show_path;
            tracing::info!("🗺️ 寻路路径 (P): {}", if config.show_path { "显示" } else { "隐藏" });
            break;
        }
    }
}
