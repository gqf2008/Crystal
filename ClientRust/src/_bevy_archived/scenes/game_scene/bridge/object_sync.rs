// Object Sync - MapObject ↔ Bevy Entity 同步
// 
// 功能说明:
// 在传统的 MapObject 系统和 Bevy ECS 之间同步状态
// 
// 设计原则:
// 1. MapObject 作为数据源 (保持 ggez 版本的逻辑)
// 2. Bevy Entity 作为渲染和物理表示
// 3. 每帧同步: MapObject → Entity (位置, 动画等)
// 4. 偶尔同步: Entity → MapObject (碰撞检测结果)

use bevy::prelude::*;
use std::sync::{Arc, Mutex};

// 复用现有的对象系统
use crate::objects::{
    MapObject, MapObjectType,
    UserObject, MonsterObject,
};

use mir2_shared::Point;

/// 组件: MapObject 引用
/// 
/// 包装一个 MapObject 的引用
/// 用于在 Bevy Entity 和 MapObject 之间建立关联
#[derive(Component)]
pub struct MapObjectRef {
    /// MapObject 的共享引用 (线程安全)
    pub object: Arc<Mutex<MapObject>>,
    
    /// 对象类型
    pub object_type: MapObjectType,
    
    /// 对象 ID (服务器分配)
    pub object_id: u32,
}

impl MapObjectRef {
    /// 创建新的 MapObject 引用
    pub fn new(object: MapObject, object_type: MapObjectType, object_id: u32) -> Self {
        Self {
            object: Arc::new(Mutex::new(object)),
            object_type,
            object_id,
        }
    }
    
    /// 获取对象位置 (像素坐标)
    pub fn get_position(&self) -> Option<Vec2> {
        self.object.lock().ok().map(|obj| {
            let loc = obj.current_location();
            Vec2::new(loc.x as f32, loc.y as f32)
        })
    }
    
    /// 更新对象 (调用 MapObject::update)
    pub fn update(&self, _delta: f32) {
        if let Ok(mut _obj) = self.object.lock() {
            // obj.update(delta);
            // TODO: MapObject update 方法需要实现
        }
    }
}

/// 系统: MapObject → Entity 同步 (每帧)
/// 
/// 将 MapObject 的状态同步到 Bevy Entity
/// 包括: 位置, 旋转, 可见性, 动画帧等
pub fn sync_objects_to_entities(
    mut query: Query<(&mut Transform, &mut Visibility, &MapObjectRef)>,
    time: Res<Time>,
) {
    for (mut transform, mut visibility, obj_ref) in query.iter_mut() {
        // 1. 更新 MapObject (调用其 update 方法)
        obj_ref.update(time.delta_secs());
        
        // 2. 同步位置
        if let Some(pos) = obj_ref.get_position() {
            transform.translation.x = pos.x;
            transform.translation.y = pos.y;
        }
        
        // 3. 同步可见性
        if let Ok(obj) = obj_ref.object.lock() {
            if obj.dead || obj.hidden {
                *visibility = Visibility::Hidden;
            } else {
                *visibility = Visibility::Visible;
            }
        }
        
        // 4. TODO: 同步动画帧 (需要 Sprite 组件)
    }
}

/// 系统: Entity → MapObject 同步 (按需)
/// 
/// 将 Bevy Entity 的物理计算结果同步回 MapObject
/// 主要用于碰撞检测等物理交互
pub fn sync_entities_to_objects(
    query: Query<(&Transform, &MapObjectRef), Changed<Transform>>,
) {
    for (transform, obj_ref) in query.iter() {
        // TODO: 将 Transform 更新回 MapObject
        // 注意: 大部分情况下 MapObject 是数据源,这里主要处理物理反馈
    }
}

/// 系统: 生成 Entity 从 MapObject
/// 
/// 当接收到服务器的对象生成包时调用
/// 创建对应的 Bevy Entity 并关联 MapObject
pub fn spawn_entity_from_object(
    object: MapObject,
    object_type: MapObjectType,
    object_id: u32,
    commands: &mut Commands,
    // TODO: 添加纹理资源等参数
) -> Entity {
    let obj_ref = MapObjectRef::new(object, object_type, object_id);
    let pos = obj_ref.get_position().unwrap_or(Vec2::ZERO);
    
    // 创建 Entity
    commands.spawn((
        Transform::from_xyz(pos.x, pos.y, 0.0),
        Visibility::default(),
        obj_ref,
        // TODO: 添加 Sprite 组件
    )).id()
}

/// 系统: 清理死亡的对象
/// 
/// 移除标记为 dead 或 hidden 的 Entity
pub fn cleanup_dead_objects_system(
    mut commands: Commands,
    query: Query<(Entity, &MapObjectRef)>,
) {
    for (entity, obj_ref) in query.iter() {
        if let Ok(obj) = obj_ref.object.lock() {
            // TODO: 根据实际逻辑决定何时真正 despawn
            // 可能需要等待死亡动画播放完成
            if obj.dead && obj.dead_time > 0 {
                commands.entity(entity).despawn();
            }
        }
    }
}
