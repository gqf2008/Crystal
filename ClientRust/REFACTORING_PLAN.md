# ECS 系统重构计划

## 🎯 目标
1. **职责清晰**：每个系统只负责一件事
2. **代码简洁**：单文件不超过500行
3. **平滑移动**：基于世界坐标，不受网格限制
4. **可扩展**：支持玩家、怪物、NPC统一移动逻辑

## 📊 当前状态分析

### 问题
- ❌ `player_system.rs` 1059行，职责混乱
  - 移动逻辑
  - 摄像机跟随 (应在camera_system)
  - 动画更新 (应在animation_system)
- ❌ `camera_system.rs` 只有工具函数，缺少智能跟随
- ❌ `animation_system.rs` 功能不完整

## 🔄 重构方案

### 1. movement_system.rs (新名称，替代player_system)
**职责**：所有实体的移动
- 平滑世界坐标移动（核心）
- 寻路路径跟随
- 碰撞检测
- 方向计算

**组件依赖**：
- Position（世界坐标）
- Velocity（速度）
- MovementTarget（目标位置）
- Path（寻路路径）
- Movable（可移动标记）

**API**：
```rust
pub fn update(world: &mut World, delta_time: f32)
pub fn move_to_target(entity, target_pos)
pub fn follow_path(entity, path: Vec<Point>)
```

### 2. camera_system.rs (增强)
**职责**：摄像机控制
- 智能跟随玩家（从player_system移入）
- 平滑插值
- 边缘检测
- 拖拽、缩放

**API**：
```rust
pub fn update(world: &mut World)
pub fn follow_target(camera_entity, target_entity)
pub fn smooth_move(camera_pos, target_pos, speed)
```

### 3. animation_system.rs (增强)
**职责**：动画播放
- 根据动作+方向更新帧（从player_system移入）
- 帧循环
- 动画速度控制

**API**：
```rust
pub fn update(world: &mut World, delta_time: f32)
pub fn play_animation(entity, action, direction)
```

### 4. pathfinding_system.rs (可选)
**职责**：寻路算法
- A* 寻路
- 路径平滑（可选，用于平滑转角）

## 🚀 实施步骤

### Phase 1: 核心平滑移动 (当前)
1. ✅ 恢复player_system.rs
2. 🔄 实现平滑移动核心：
   - 不按网格对齐
   - 基于世界坐标
   - 插值移动
3. 🔄 简化寻路：
   - 路径只作为引导点
   - 不强制对齐每个点
   - 平滑穿过路径点

### Phase 2: 职责分离
1. 提取摄像机跟随 → camera_system
2. 提取动画更新 → animation_system
3. 重命名 player_system → movement_system

### Phase 3: 泛化
1. 支持怪物使用相同移动系统
2. 支持NPC移动

## 💡 核心设计：平滑移动

### 理念
> 地图网格只用于标记障碍物，角色按世界坐标平滑移动

### 实现要点
```rust
// 每帧更新（不等待网络）
fn update_position(pos: &mut Position, target: Point, speed: f32, delta_time: f32) {
    let dx = target.x - pos.x;
    let dy = target.y - pos.y;
    let distance = (dx * dx + dy * dy).sqrt();
    
    if distance > 0.1 {
        let move_dist = speed * delta_time;
        let ratio = (move_dist / distance).min(1.0);
        pos.x += dx * ratio;
        pos.y += dy * ratio;
    }
}

// 寻路：路径点作为引导，不强制经过
fn follow_path(pos: &mut Position, path: &Vec<Point>, speed: f32) {
    if let Some(next_point) = find_next_visible_point(pos, path) {
        move_towards(pos, next_point, speed);
        
        // 如果接近路径点，跳到下一个
        if distance(pos, next_point) < PATH_THRESHOLD {
            path.advance();
        }
    }
}
```

### 优势
- ✅ 平滑自然
- ✅ 不受网格限制
- ✅ 视觉效果好
- ✅ 性能高（不需要等待网络）

### 网络同步策略
- 客户端：立即平滑移动（预测）
- 服务器：定期发送位置校正
- 冲突：平滑插值到服务器位置

## 📝 代码规范

### 文件大小限制
- 单文件 ≤ 500 行
- 超过拆分到子模块

### 命名约定
- System: 系统逻辑
- Component: 数据组件
- 清晰的职责划分

### 注释要求
- 每个系统顶部：职责说明
- 每个函数：简要说明

## ✅ 检查清单

- [ ] movement_system < 500行
- [ ] camera_system 包含智能跟随
- [ ] animation_system 包含帧更新
- [ ] 平滑移动测试通过
- [ ] 寻路平滑测试通过
- [ ] 无重复代码
- [ ] 编译通过
- [ ] 所有系统职责清晰
