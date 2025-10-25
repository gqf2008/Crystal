# 渲染和网络系统修复方案

## 问题分析

### 当前问题
1. **网络包处理不完整** - 服务器发送的某些包没有被正确处理
2. **渲染系统不完整** - 只渲染了地图,没有渲染NPC/怪物/特效等
3. **碰撞检测缺失** - 没有实现与原版一致的碰撞检测

### 原版C#客户端的绘制流程 (GameScene.cs DrawObjects)

```
绘制顺序 (按Y坐标从上到下):
1. 背景特效 (Effects.DrawBehind = true)  - 地面火焰、毒圈等
2. 尸体对象 (DrawDeadObjects)           - 确保被活物遮挡
3. Shanda动画层 (TileAnimationImage)    - 流水、岩浆等动态地表
4. Middle中层瓦片 (MiddleImage)          - 带动画的装饰层
5. Front前景瓦片 (FrontImage)           - 前景物体
6. 活体对象 (DrawObjects)               - 玩家/怪物/NPC/掉落物
   - 按 Y 坐标排序,实现正确遮挡
   - 先绘制低Y坐标的对象(远处)
   - 后绘制高Y坐标的对象(近处)
7. 目标高亮边框 (HighlightTarget)       - 选中目标的边框
8. 前景特效 (Effects.DrawBehind = false) - 火球、闪电等飞行特效
9. 文字标签 (名字/血条/聊天/伤害数字)
```

## 修复方案

### 阶段1: 完善网络包处理 ✅

**目标**: 确保所有服务器包都被正确处理并转换为GameEvent

**已实现的事件**:
- ✅ Connected, Disconnected
- ✅ LoginSuccess, StartGameResponse
- ✅ MapInformation, UserInformation
- ✅ PlayerMoved (UserLocation)
- ✅ ObjectSpawned, ObjectRemoved
- ✅ ObjectTurned, ObjectWalked, ObjectRan
- ✅ ObjectAttacked, ObjectPushed
- ✅ ChatReceived

**需要确认的包处理**:
```rust
// 在 NetworkSystem::process_event 中处理:
- ObjectSpawned  → 创建NPC/怪物实体
- ObjectRemoved  → 移除对象
- ObjectTurned   → 更新对象朝向
- ObjectWalked   → 更新对象移动(走)
- ObjectRan      → 更新对象移动(跑)
- ObjectAttacked → 播放攻击动画
- ObjectPushed   → 播放被击退动画
```

### 阶段2: 实现对象渲染系统

**2.1 创建渲染对象组件**
```rust
// src/ecs/components.rs
pub struct RenderObject {
    pub object_id: u32,
    pub object_type: NetworkObjectType, // Player/NPC/Monster/Item
    pub race: ObjectType,               // 种族类型
    pub body_library: LibraryName,      // 身体图库
    pub weapon_library: Option<LibraryName>, // 武器图库
    pub current_action: MirAction,      // 当前动作
    pub direction: MirDirection,        // 朝向
    pub frame_index: usize,             // 当前帧
    pub draw_y: i32,                    // Y坐标排序用
}

pub struct DeadObject {
    pub dead_time: std::time::Instant,
}
```

**2.2 修改渲染系统**
```rust
// src/ecs/systems/render.rs

impl RenderSystem {
    /// 绘制所有对象 (仿照原版DrawObjects)
    pub fn draw_objects(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        camera: &Camera,
    ) -> GameResult<()> {
        // 1. 背景特效
        Self::draw_background_effects(ctx, canvas, world, camera)?;
        
        // 2. 尸体对象
        Self::draw_dead_objects(ctx, canvas, world, camera)?;
        
        // 3. 活体对象 (按Y坐标排序)
        Self::draw_living_objects(ctx, canvas, world, camera)?;
        
        // 4. 前景特效
        Self::draw_foreground_effects(ctx, canvas, world, camera)?;
        
        // 5. 名字/血条/聊天
        Self::draw_labels(ctx, canvas, world, camera)?;
        
        Ok(())
    }
    
    /// 绘制活体对象 (玩家/NPC/怪物)
    fn draw_living_objects(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        camera: &Camera,
    ) -> GameResult<()> {
        // 收集所有需要绘制的对象
        let mut objects = Vec::new();
        
        for (_entity, (pos, obj, _)) in world.query::<(
            &Position,
            &RenderObject,
            Without<DeadObject>,
        )>().iter() {
            // 视野裁剪
            if !camera.is_in_view(pos.x, pos.y) {
                continue;
            }
            
            objects.push((pos.y, _entity, pos, obj));
        }
        
        // 按Y坐标排序 (从上到下)
        objects.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        
        // 绘制
        for (_, entity, pos, obj) in objects {
            Self::draw_object(ctx, canvas, entity, pos, obj, camera)?;
        }
        
        Ok(())
    }
    
    /// 绘制单个对象
    fn draw_object(
        ctx: &mut Context,
        canvas: &mut Canvas,
        entity: Entity,
        pos: &Position,
        obj: &RenderObject,
        camera: &Camera,
    ) -> GameResult<()> {
        // 获取图库
        let library = get_library(obj.body_library)?;
        
        // 计算帧索引: 动作基础索引 + 方向偏移 + 帧偏移
        let action_base = obj.current_action.frame_offset();
        let dir_offset = obj.direction.frame_offset();
        let frame = action_base + dir_offset + obj.frame_index;
        
        // 计算屏幕坐标
        let screen_pos = camera.world_to_screen(pos.x, pos.y);
        
        // 绘制身体
        library.draw(
            canvas,
            frame as u32,
            screen_pos.0,
            screen_pos.1,
            Color::WHITE,
            false,
        )?;
        
        // 绘制武器 (如果有)
        if let Some(weapon_lib) = obj.weapon_library {
            let weapon = get_library(weapon_lib)?;
            weapon.draw(
                canvas,
                frame as u32,
                screen_pos.0,
                screen_pos.1,
                Color::WHITE,
                false,
            )?;
        }
        
        Ok(())
    }
}
```

**2.3 修改网络系统创建对象**
```rust
// src/ecs/systems/network.rs

impl NetworkSystem {
    fn handle_object_spawned(&mut self, world: &mut World, object: &GameObject) {
        // 创建实体
        let entity = world.spawn((
            Position::new(object.location.x as f32 * 48.0, object.location.y as f32 * 32.0),
            RenderObject {
                object_id: object.object_id,
                object_type: self.get_object_type(&object),
                race: object.race,
                body_library: self.get_body_library(&object),
                weapon_library: None,
                current_action: MirAction::Standing,
                direction: object.direction,
                frame_index: 0,
                draw_y: object.location.y,
            },
            NetworkSync {
                object_id: object.object_id,
                last_update: Instant::now(),
                object_type: NetworkObjectType::from_race(object.race),
            },
        ));
        
        // 记录映射
        self.object_map.insert(object.object_id, entity);
    }
    
    fn handle_object_moved(
        &mut self,
        world: &mut World,
        object_id: u32,
        direction: MirDirection,
        location: &Point,
        action: MirAction,
    ) {
        if let Some(&entity) = self.object_map.get(&object_id) {
            if let Ok(mut obj) = world.get::<&mut RenderObject>(entity) {
                obj.direction = direction;
                obj.current_action = action;
                obj.frame_index = 0; // 重置动画帧
            }
            
            if let Ok(mut pos) = world.get::<&mut Position>(entity) {
                pos.x = location.x as f32 * 48.0;
                pos.y = location.y as f32 * 32.0;
            }
        }
    }
}
```

### 阶段3: 实现碰撞检测

**3.1 地图碰撞数据**
```rust
// src/ecs/components.rs
pub struct MapCollision {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<bool>, // true = 可通过, false = 阻挡
}

impl MapCollision {
    pub fn can_walk(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return false;
        }
        self.cells[y as usize * self.width + x as usize]
    }
}
```

**3.2 原版碰撞规则** (来自Client/MirObjects/UserObject.cs)
```csharp
// 检查是否可以移动到目标位置
public bool CanMove(Point location)
{
    // 1. 检查地图边界
    if (location.X < 0 || location.Y < 0 || 
        location.X >= MapControl.Width || 
        location.Y >= MapControl.Height)
        return false;
    
    // 2. 检查地形阻挡 (墙壁/水面等)
    Cell cell = MapControl.GetCell(location.X, location.Y);
    if (!cell.Valid) return false;
    
    // 3. 检查其他对象阻挡
    for (int i = 0; i < MapControl.Objects.Count; i++)
    {
        MapObject ob = MapControl.Objects[i];
        if (ob.CurrentLocation == location && ob.Blocking)
            return false;
    }
    
    return true;
}
```

**3.3 实现碰撞检测系统**
```rust
// src/ecs/systems/collision.rs
pub struct CollisionSystem;

impl CollisionSystem {
    /// 检查是否可以移动到目标位置
    pub fn can_move_to(
        world: &World,
        from: Point,
        to: Point,
        entity: Entity,
    ) -> bool {
        // 1. 检查地图碰撞
        for (_e, map) in world.query::<&MapCollision>().iter() {
            if !map.can_walk(to.x, to.y) {
                return false;
            }
        }
        
        // 2. 检查对象碰撞
        for (_e, (pos, obj)) in world.query::<(&Position, &RenderObject)>().iter() {
            if _e == entity {
                continue; // 跳过自己
            }
            
            let obj_grid_x = (pos.x / 48.0).round() as i32;
            let obj_grid_y = (pos.y / 32.0).round() as i32;
            
            if obj_grid_x == to.x && obj_grid_y == to.y {
                // 检查是否阻挡
                if obj.object_type == NetworkObjectType::Monster ||
                   obj.object_type == NetworkObjectType::Player {
                    return false;
                }
            }
        }
        
        true
    }
}
```

## 实施优先级

### 立即修复 (P0)
1. ✅ 确认 NetworkSync 组件正确添加
2. ✅ 修复 PlayerMoved 事件处理
3. 🔄 添加调试日志验证网络包接收

### 短期修复 (P1 - 本周)
1. 实现 RenderObject 组件
2. 修改渲染系统支持多对象绘制
3. 实现对象按Y坐标排序
4. 处理 ObjectSpawned/ObjectRemoved 事件

### 中期修复 (P2 - 下周)
1. 实现完整的动画系统
2. 实现碰撞检测
3. 实现特效渲染
4. 实现名字/血条显示

### 长期优化 (P3)
1. 性能优化 (空间分区/视野裁剪)
2. 动画插值平滑
3. 粒子特效系统
4. 光照系统

## 测试验证

### 网络测试
- [ ] 登录后收到 MapInformation
- [ ] 收到 UserInformation
- [ ] 收到 UserLocation (PlayerMoved)
- [ ] 看到 "🔒 已添加 NetworkSync 组件" 日志
- [ ] 收到 ObjectSpawned (NPC/怪物)

### 渲染测试
- [ ] 玩家角色正确显示
- [ ] NPC正确显示
- [ ] 怪物正确显示
- [ ] 角色朝向正确
- [ ] 行走动画播放
- [ ] Y坐标遮挡正确

### 碰撞测试
- [ ] 不能穿墙
- [ ] 不能穿过NPC/怪物
- [ ] 斜向移动正确
- [ ] 跑步碰撞正确
