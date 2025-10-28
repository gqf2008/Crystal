// ============================================================================
// Mouse Event System (Layer 5 - UI)
// ============================================================================
//
// 职责：
// - 分发鼠标事件（UI层优先，然后传递到游戏世界）
// - 处理鼠标点击（左键：选中NPC/怪物，右键：移动/攻击）
// - 处理鼠标移动（更新鼠标坐标）
// - 处理鼠标滚轮（缩放）
// - 维护鼠标状态（按下/释放、长按计时、双击检测）
//
// 调用时机：
// - on_mouse_button_down_event
// - on_mouse_button_up_event
// - on_mouse_motion_event
// - on_mouse_wheel_event
// - update() - 每帧更新鼠标状态
//
// ============================================================================

use hecs::World;
use tokio::sync::mpsc;
use ggez::input::mouse::MouseButton;
use crate::network::NetworkCommand;
use crate::ecs::systems::{UISystem, CameraSystem, NPCSystem};
use crate::ecs::components::{MouseInput, Position, Camera, NPCData, MonsterData, LocalPlayer};

pub struct MouseEventSystem;

impl MouseEventSystem {
    /// 处理鼠标点击
    /// 
    /// # 参数
    /// - `world`: ECS 世界
    /// - `button`: 鼠标按钮
    /// - `ui_x, ui_y`: UI 坐标（用于UI层点击检测）
    /// - `window_x, window_y`: 窗口坐标（用于游戏世界点击检测）
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
        
        // 更新鼠标按下状态
        Self::update_mouse_press_state(world, button, true);
        
        false
    }
    
    /// 处理鼠标释放
    pub fn process_mouse_up(world: &mut World, button: MouseButton) {
        Self::update_mouse_press_state(world, button, false);
    }
    
    /// 处理鼠标移动
    pub fn process_mouse_move(world: &mut World, x: f32, y: f32) {
        if let Some((_, mouse_input)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
            mouse_input.x = x;
            mouse_input.y = y;
        }
    }
    
    /// 处理鼠标滚轮
    pub fn process_mouse_wheel(world: &mut World, delta_y: f32) {
        use crate::ecs::components::Camera;
        
        const ZOOM_SPEED: f32 = 0.1;
        const MIN_ZOOM: f32 = 0.5;
        const MAX_ZOOM: f32 = 2.0;
        
        // 查找相机实体并更新缩放
        for (_, camera) in world.query_mut::<&mut Camera>() {
            let zoom_delta = delta_y * ZOOM_SPEED;
            camera.zoom = (camera.zoom + zoom_delta).clamp(MIN_ZOOM, MAX_ZOOM);
            tracing::debug!("🔍 相机缩放: {:.2}", camera.zoom);
            break; // 只处理第一个相机
        }
    }
    
    /// 更新鼠标输入状态（每帧调用）
    /// 用于更新长按计时器和清除双击事件
    pub fn update_mouse_input(world: &mut World) {
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
    // 内部方法
    // ========================================================================
    
    /// 处理游戏世界点击
    fn handle_world_click(
        world: &mut World,
        button: MouseButton,
        x: f32,
        y: f32,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) {
        // 左键点击检测NPC/怪物
        if button == MouseButton::Left {
            Self::handle_left_click_on_entities(world, x, y, network_tx);
        }
        // 右键点击由 PathfindingSystem/MovementSystem 处理（已迁移到 LocalPredictionSystem）
    }
    
    /// 处理左键点击实体（NPC/怪物）
    fn handle_left_click_on_entities(
        world: &mut World,
        x: f32,
        y: f32,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) {
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
    
    /// 更新鼠标按下状态
    fn update_mouse_press_state(world: &mut World, button: MouseButton, pressed: bool) {
        if let Some((_, mouse_input)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
            match button {
                MouseButton::Left => {
                    if pressed {
                        mouse_input.left_pressed = true;
                        mouse_input.left_press_time = 0;
                    } else {
                        mouse_input.left_pressed = false;
                    }
                }
                MouseButton::Right => {
                    if pressed {
                        mouse_input.right_pressed = true;
                        mouse_input.right_press_time = 0;
                    } else {
                        mouse_input.right_pressed = false;
                    }
                }
                _ => {}
            }
        }
    }
}
