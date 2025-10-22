# ECS 系统提取进度

## ✅ 已完成

### 架构创建
- ✅ 创建 `src/ecs/systems/` 目录
- ✅ 创建 `src/bin/mir2x.rs` 完整客户端入口
- ✅ 创建项目架构说明文档

### 系统提取
- ✅ **CameraSystem** → `src/ecs/systems/camera.rs`
  - 屏幕/世界坐标转换
  - 拖拽控制
  - 缩放
  - 边缘滚屏

## 🚧 进行中

### 待提取系统

1. **PlayerSystem** (Line 525 in map_viewer_ecs.rs)
   - 角色移动
   - 寻路
   - 动画控制
   - 状态机

2. **AnimationSystem** (Line 949)
   - 动画瓦片更新

3. **DoorSystem** (Line 962)
   - 门动画

4. **RenderSystem** (Line 1002)
   - 地图渲染
   - 角色渲染
   - 路径绘制

## 📋 下一步计划

### Step 1: 继续提取系统 (1-2小时)
```bash
# 提取顺序:
1. PlayerSystem    → src/ecs/systems/player.rs
2. AnimationSystem → src/ecs/systems/animation.rs
3. DoorSystem      → src/ecs/systems/door.rs
4. RenderSystem    → src/ecs/systems/render.rs
```

### Step 2: 更新 map_viewer_ecs.rs (30分钟)
```rust
// 使用共享系统
use mir2_client::ecs::systems::*;

// 删除本地系统定义
// struct CameraSystem; // 删除
// impl CameraSystem { ... } // 删除
```

### Step 3: 测试验证 (30分钟)
```bash
cargo run --bin map_viewer_ecs
# 确保功能正常
```

### Step 4: 开始网络功能 (1-2周)
```rust
// src/ecs/systems/network.rs
pub struct NetworkSystem;

impl NetworkSystem {
    pub fn connect(addr: &str) -> Result<Self> { ... }
    pub fn send_packet(&mut self, packet: ClientPacket) { ... }
    pub fn recv_packets(&mut self) -> Vec<ServerPacket> { ... }
}
```

## 🎯 目标

**短期目标 (今天):**
- 提取所有系统到 ecs/systems/
- 更新 map_viewer_ecs.rs 使用共享系统
- 测试确保功能正常

**中期目标 (本周):**
- 完善 ECS 架构
- 开始网络系统设计

**长期目标 (1-2周):**
- 实现网络连接
- 登录系统
- 角色移动同步

## 📝 注意事项

### 类型定义问题
当前 camera.rs 中有临时类型定义:
```rust
// 临时类型定义 - 后续会使用 ecs/components.rs
pub struct Position { ... }
pub struct Camera { ... }
```

**TODO:** 统一使用 `src/ecs/components.rs` 中的定义

### 编译状态
- ✅ 库编译成功 (`cargo build --lib`)
- ✅ mir2x 编译成功 (`cargo build --bin mir2x`)
- ⏳ map_viewer_ecs 待更新

## 🚀 继续重构?

**选项:**
1. **继续提取** → 完成所有系统提取 (推荐)
2. **暂停休息** → 稍后继续
3. **直接网络** → 跳过重构,开始网络功能

**我的建议:** 选项 1,一鼓作气完成所有系统提取,预计1-2小时。
