// ObjectManager - 管理所有地图对象 (玩家、怪物、NPC、掉落物)
//
// 职责:
// - 管理所有对象的生命周期
// - 提供对象查询和更新接口
// - 空间索引加速鼠标检测 (TODO: 后续优化)
// - 视野裁剪减少不必要的更新 (TODO: 后续优化)

use std::collections::HashMap;
use mir2_shared::Point;
use mir2_shared::enums::{MirDirection, MirAction};

use crate::objects::{UserObject, HeroObject, MonsterObject, NPCObject, ItemObject};
use crate::systems::InputSystem;
use crate::scenes::game_scene::camera::Camera;
use crate::scenes::game_scene::map_renderer::MapRenderer;

/// 对象管理器
pub struct ObjectManager {
    /// 玩家对象 (唯一)
    user: Option<UserObject>,
    
    /// 英雄对象 (唯一)
    hero: Option<HeroObject>,
    
    /// 所有其他对象 (怪物、NPC、掉落物等)
    /// Key: ObjectID (来自服务器)
    objects: HashMap<u32, MapObjectWrapper>,
    
    // TODO: 空间索引 (加速鼠标碰撞检测)
    // spatial_index: Grid<Vec<u32>>,
    
    // TODO: 视野裁剪
    // visible_objects: Vec<u32>,
}

/// 对象包装器 - 统一管理不同类型的对象
#[derive(Debug, Clone)]
pub enum MapObjectWrapper {
    Monster(MonsterObject),
    Npc(NPCObject),
    Item(ItemObject),
    // TODO: 添加其他对象类型
}

impl ObjectManager {
    /// 创建新的对象管理器
    pub fn new() -> Self {
        Self {
            user: None,
            hero: None,
            objects: HashMap::new(),
        }
    }
    
    // ==================== 玩家对象管理 ====================
    
    /// 设置玩家对象
    pub fn set_user(&mut self, user: UserObject) {
        self.user = Some(user);
    }
    
    /// 获取玩家对象 (只读)
    pub fn user(&self) -> Option<&UserObject> {
        self.user.as_ref()
    }
    
    /// 获取玩家对象 (可变)
    pub fn user_mut(&mut self) -> Option<&mut UserObject> {
        self.user.as_mut()
    }
    
    // ==================== 英雄对象管理 ====================
    
    /// 设置英雄对象
    pub fn set_hero(&mut self, hero: HeroObject) {
        self.hero = Some(hero);
    }
    
    /// 获取英雄对象 (只读)
    pub fn hero(&self) -> Option<&HeroObject> {
        self.hero.as_ref()
    }
    
    /// 获取英雄对象 (可变)
    pub fn hero_mut(&mut self) -> Option<&mut HeroObject> {
        self.hero.as_mut()
    }
    
    // ==================== 通用对象管理 ====================
    
    /// 添加怪物对象
    pub fn add_monster(&mut self, monster: MonsterObject) {
        let id = monster.map_object.object_id;
        self.objects.insert(id, MapObjectWrapper::Monster(monster));
    }
    
    /// 添加 NPC 对象
    pub fn add_npc(&mut self, npc: NPCObject) {
        let id = npc.map_object.object_id;
        self.objects.insert(id, MapObjectWrapper::Npc(npc));
    }
    
    /// 添加掉落物对象
    pub fn add_item(&mut self, item: ItemObject) {
        let id = item.map_object.object_id;
        self.objects.insert(id, MapObjectWrapper::Item(item));
    }
    
    /// 移除对象
    pub fn remove_object(&mut self, object_id: u32) -> Option<MapObjectWrapper> {
        self.objects.remove(&object_id)
    }
    
    /// 获取对象 (只读)
    pub fn get_object(&self, object_id: u32) -> Option<&MapObjectWrapper> {
        self.objects.get(&object_id)
    }
    
    /// 获取对象 (可变)
    pub fn get_object_mut(&mut self, object_id: u32) -> Option<&mut MapObjectWrapper> {
        self.objects.get_mut(&object_id)
    }
    
    /// 获取所有对象ID
    pub fn all_object_ids(&self) -> Vec<u32> {
        self.objects.keys().copied().collect()
    }
    
    // ==================== 输入处理 ====================
    
    /// 处理玩家移动输入 (从 InputSystem 获取数据)
    ///
    /// # 参数
    /// - `input_system`: 输入系统 (提供鼠标/键盘状态)
    /// - `camera`: 摄像机 (用于屏幕坐标转换)
    /// - `map_renderer`: 地图渲染器 (用于碰撞检测)
    /// - `network_tx`: 网络发送通道 (用于发送移动包)
    pub fn handle_move_input(
        &mut self,
        input_system: &InputSystem,
        camera: &Camera,
        map_renderer: &MapRenderer,
        network_tx: &Option<tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>>,
    ) {
        let user = match &mut self.user {
            Some(u) => u,
            None => return, // 没有玩家对象,直接返回
        };
        
        // 1. 获取移动输入
        if let Some(running) = input_system.get_move_input() {
            // 获取鼠标世界坐标 (使用闭包进行坐标转换)
            let target_cell = input_system.get_mouse_world_pos(|screen_pos| {
                // 屏幕坐标 -> 世界坐标 -> 地图格子坐标
                let (world_x, world_y) = camera.screen_to_world(screen_pos.x as f32, screen_pos.y as f32);
                let map_x = (world_x / 48.0) as i32; // MapRenderer::CELL_WIDTH = 48
                let map_y = (world_y / 32.0) as i32; // MapRenderer::CELL_HEIGHT = 32 ⚠️ 不是48!
                
                Point { x: map_x, y: map_y }
            });
            let current_cell = user.movement_fsm.current_cell;
            
            // 如果目标和当前位置相同，只更新方向（原地转向）
            if current_cell == target_cell {
                // 仍然可以原地转向（朝向鼠标）
                // 但是如果距离太近，不要转向（避免鼠标在角色身上时疯狂转）
                let dx = (target_cell.x - current_cell.x).abs();
                let dy = (target_cell.y - current_cell.y).abs();
                if dx == 0 && dy == 0 {
                    println!("⚠️ [方向] 目标就是当前格子，忽略");
                    return; // 完全相同的格子，忽略
                }
            }
            
            // 计算方向(目标方向)
            let target_direction = Self::direction_from_point(current_cell, target_cell);
            
            // 如果点击的就是当前格子,只转向不移动
            if current_cell == target_cell {
                user.movement_fsm.direction = target_direction;
                println!("🔄 [转向] 方向={:?}", target_direction);
                return;
            }
            
            // 方向计算
            let target_direction = Self::direction_from_point(current_cell, target_cell);
            
            // 根据状态处理移动
            if user.movement_fsm.is_idle() {
                // 当前静止,开始新的移动
                
                // 检查碰撞(使用目标方向)
                let next_cell = Self::point_move(current_cell, target_direction, 1);
                let can_move = Self::can_walk_to(next_cell, map_renderer);
                
                if can_move {
                    println!("✅ 移动: ({},{}) -> ({},{}), 方向={:?}", 
                        current_cell.x, current_cell.y, target_cell.x, target_cell.y, target_direction);
                    user.movement_fsm.move_to(target_cell, target_direction, running);
                    user.player.set_current_action(if running {
                        MirAction::Running
                    } else {
                        MirAction::Walking
                    });
                } else {
                    // 被阻挡,原地转向
                    println!("🚫 被墙挡: 当前({},{}), 下一格({},{})不可通行, 转向={:?}", 
                        current_cell.x, current_cell.y, next_cell.x, next_cell.y, target_direction);
                    user.movement_fsm.direction = target_direction;
                    // 强制同步方向到 MapObject
                    user.player.map_object.direction = target_direction;
                    user.player.set_current_action(MirAction::Standing);
                }
            } else {
                // 🔧 正在移动中:基于当前位置计算方向,平滑转向
                
                // 更新目标位置
                user.movement_fsm.target_cell = target_cell;
                
                // 基于当前位置重新计算方向
                let new_direction = Self::direction_from_point(current_cell, target_cell);
                
                // 如果方向改变,尝试更新方向 (带冷却,避免画面抖动)
                if new_direction != user.movement_fsm.direction {
                    if user.movement_fsm.change_direction(new_direction) {
                        println!("🔄 [移动中转向] {:?} -> {:?}", 
                            user.movement_fsm.direction, new_direction);
                    }
                }
                
                // 更新跑步状态
                if user.movement_fsm.running != running {
                    user.movement_fsm.running = running;
                }
                
                user.player.set_current_action(if running {
                    MirAction::Running
                } else {
                    MirAction::Walking
                });
            }
        } else {
            // 鼠标释放,停止移动
            if user.movement_fsm.is_moving() {
                user.movement_fsm.stop();
                user.player.set_current_action(MirAction::Standing);
            }
        }
    }
    
    // ==================== 辅助方法 (坐标/方向/碰撞检测) ====================
    
    /// 计算两点之间的方向
    fn direction_from_point(source: Point, dest: Point) -> MirDirection {
        let dx = dest.x - source.x;
        let dy = dest.y - source.y;
        
        // 游戏使用屏幕坐标系 (Y 轴向下)，需要反转 dy
        let angle = (-dy as f32).atan2(dx as f32).to_degrees();
        
        // 映射到 8 方向
        let direction = if angle >= -22.5 && angle < 22.5 {
            MirDirection::Right
        } else if angle >= 22.5 && angle < 67.5 {
            MirDirection::UpRight
        } else if angle >= 67.5 && angle < 112.5 {
            MirDirection::Up
        } else if angle >= 112.5 && angle < 157.5 {
            MirDirection::UpLeft
        } else if (angle >= 157.5 && angle <= 180.0) || (angle >= -180.0 && angle < -157.5) {
            MirDirection::Left
        } else if angle >= -157.5 && angle < -112.5 {
            MirDirection::DownLeft
        } else if angle >= -112.5 && angle < -67.5 {
            MirDirection::Down
        } else {
            MirDirection::DownRight
        };
        
        direction
    }
    
    /// 根据方向移动点
    fn point_move(p: Point, d: MirDirection, count: i32) -> Point {
        let mut result = p;
        for _ in 0..count {
            match d {
                MirDirection::Up => result.y -= 1,
                MirDirection::UpRight => { result.x += 1; result.y -= 1; }
                MirDirection::Right => result.x += 1,
                MirDirection::DownRight => { result.x += 1; result.y += 1; }
                MirDirection::Down => result.y += 1,
                MirDirection::DownLeft => { result.x -= 1; result.y += 1; }
                MirDirection::Left => result.x -= 1,
                MirDirection::UpLeft => { result.x -= 1; result.y -= 1; }
            }
        }
        result
    }
    
    /// 检查目标格子是否可行走
    fn can_walk_to(target: Point, map_renderer: &MapRenderer) -> bool {
        map_renderer.is_walkable(target.x, target.y)
    }
    
    /// 平滑方向变化 (避免突然转向)
    fn smooth_direction_change(current: MirDirection, target: MirDirection) -> MirDirection {
        let current_idx = current as i32;
        let target_idx = target as i32;
        
        // 计算最短角度差
        let mut diff = target_idx - current_idx;
        if diff > 4 {
            diff -= 8;
        } else if diff < -4 {
            diff += 8;
        }
        
        // 限制单次最大转向角度 (2 个方向)
        let step = if diff > 2 {
            2
        } else if diff < -2 {
            -2
        } else {
            diff
        };
        
        let new_idx = (current_idx + step + 8) % 8;
        unsafe { std::mem::transmute(new_idx as u8) }
    }
    
    // ==================== 更新逻辑 ====================
    
    /// 更新所有对象 (移动、动画、AI)
    /// 
    /// # 参数
    /// - `network_tx`: 网络发送通道 (用于在完成移动时发送移动包)
    pub fn update(
        &mut self, 
        _delta_time: f32,
        network_tx: &Option<tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>>,
    ) {
        // 1. 更新玩家 (优先级最高)
        if let Some(user) = &mut self.user {
            // 调用 FSM 更新，检查是否完成了一格移动
            let completed_move = user.movement_fsm.update();
            
            if completed_move {
                // 完成了一格移动，发送网络包到服务器
                let new_cell = user.movement_fsm.current_cell;
                let direction = user.movement_fsm.direction;
                
                println!("📦 [网络] 发送移动包: 位置=({},{}), 方向={:?}", 
                    new_cell.x, new_cell.y, direction);
                
                if let Some(ref tx) = network_tx {
                    use crate::network::NetworkCommand;
                    let _ = tx.send(NetworkCommand::Move {
                        direction: direction as u8,
                        location: (new_cell.x, new_cell.y),
                    });
                }
                
                // 同步到 MapObject
                user.player.map_object.current_location = new_cell;
                
                // 检查是否到达目标
                if user.movement_fsm.is_idle() {
                    user.player.set_current_action(MirAction::Standing);
                }
            }
            
            // 🔧 每帧同步 FSM 状态到 MapObject (确保平滑移动和方向跟随)
            user.player.map_object.movement = user.movement_fsm.render_start_cell;
            
            // 🐛 调试：显示同步前后的方向
            let fsm_dir = user.movement_fsm.direction;
            let old_map_dir = user.player.map_object.direction;
            
            user.player.map_object.direction = user.movement_fsm.direction; // 🔧 同步方向
            
            if fsm_dir != old_map_dir {
                println!("🔄 [update同步] FSM方向={:?} -> MapObject方向={:?} (旧值={:?})", 
                    fsm_dir, user.player.map_object.direction, old_map_dir);
            }
            
            let (offset_x, offset_y) = user.movement_fsm.get_render_offset(48, 32); // 使用标准格子尺寸
            user.player.map_object.offset_move = Point::new(offset_x, offset_y);
            
            // 更新动画
            user.player.update_animation();
        }
        
        // 2. 更新英雄
        if let Some(_hero) = &mut self.hero {
            // TODO: 实现英雄更新逻辑
        }
        
        // 3. 更新其他对象
        for obj in self.objects.values_mut() {
            match obj {
                MapObjectWrapper::Monster(_monster) => {
                    // TODO: 更新怪物动画和移动
                }
                MapObjectWrapper::Npc(_npc) => {
                    // TODO: 更新 NPC 动画
                }
                MapObjectWrapper::Item(_item) => {
                    // 掉落物通常不需要更新
                }
            }
        }
        
        // TODO: 更新视野裁剪
        // self.update_visible_objects();
    }
    
    // ==================== 鼠标拾取检测 ====================
    
    /// 鼠标拾取检测 (从鼠标位置找对象)
    /// 
    /// TODO: 实现精确的碰撞检测
    /// 1. 屏幕坐标 -> 世界坐标
    /// 2. 查询空间索引
    /// 3. 精确碰撞检测 (从上到下)
    pub fn pick_object_at(&self, _mouse_pos: Point) -> Option<u32> {
        // TODO: 实现鼠标拾取逻辑
        None
    }
    
    // ==================== 视野裁剪 (TODO) ====================
    
    /// 获取可见对象ID列表
    pub fn visible_objects(&self) -> Vec<u32> {
        // TODO: 实现视野裁剪
        // 目前返回所有对象
        self.all_object_ids()
    }
}

impl Default for ObjectManager {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== MapObjectWrapper 辅助方法 ====================

impl MapObjectWrapper {
    /// 获取对象ID
    pub fn object_id(&self) -> u32 {
        match self {
            MapObjectWrapper::Monster(m) => m.map_object.object_id,
            MapObjectWrapper::Npc(n) => n.map_object.object_id,
            MapObjectWrapper::Item(i) => i.map_object.object_id,
        }
    }
    
    /// 获取对象位置
    pub fn position(&self) -> Point {
        match self {
            MapObjectWrapper::Monster(m) => m.map_object.current_location,
            MapObjectWrapper::Npc(n) => n.map_object.current_location,
            MapObjectWrapper::Item(i) => i.map_object.current_location,
        }
    }
    
    /// 获取对象绘制Y坐标 (用于排序)
    pub fn draw_y(&self) -> i32 {
        match self {
            MapObjectWrapper::Monster(m) => {
                m.map_object.current_location.y + m.map_object.offset_move.y / 32
            }
            MapObjectWrapper::Npc(n) => {
                n.map_object.current_location.y + n.map_object.offset_move.y / 32
            }
            MapObjectWrapper::Item(i) => {
                i.map_object.current_location.y
            }
        }
    }
}
