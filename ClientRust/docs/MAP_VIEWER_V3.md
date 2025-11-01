# Map Viewer V3 - 地图查看器

基于新 ECS 架构的地图查看器开发工具。

## 功能特性

- ✅ 使用新的 SystemScheduler 调度器
- ✅ 使用 GlobalEvents 事件系统  
- ✅ 使用 MockNetwork 模拟网络（无需真实服务器）
- ✅ 支持地图浏览、缩放、拖拽
- ✅ 离线开发和调试

## 架构设计

### 模拟网络 (MockNetwork)

位于 `src/network/mock.rs`，提供完整的网络模拟：

```rust
// 创建模拟网络
let net_ctx = NetworkBuilder::new(settings.network.clone())
    .mock(true)  // 启用模拟模式
    .build()?;
```

**特性：**
- 自动发送 Connected 事件
- 响应登录、角色、地图等请求
- 模拟网络延迟
- 与真实网络相同的 NetContext 接口

### 简化场景 (MapViewerScene)

位于 `src/bin/map_viewer/scene.rs`，从 GameScene 简化而来：

**包含系统：**
- MovementSystem - 移动系统
- AnimationSystem - 动画系统
- CameraSystem - 相机系统

**不包含：**
- UI 系统
- 战斗系统
- AI 系统
- 网络同步系统

## 使用方法

### 编译运行

```bash
cargo run --bin map_viewer_v3
```

### 控制键位

- **鼠标拖拽** - 移动地图视角
- **滚轮** - 缩放地图
- **ESC** - 退出程序
- **G** - 切换网格显示（计划中）
- **O** - 切换障碍显示（计划中）

## 文件结构

```
src/
├── network/
│   ├── mod.rs           # 导出 MockNetwork
│   ├── mock.rs          # 模拟网络实现
│   └── builder.rs       # NetworkBuilder.mock(true)
└── bin/
    ├── map_viewer_v3.rs # 主程序入口
    └── map_viewer/
        ├── mod.rs       # 模块定义
        └── scene.rs     # MapViewerScene
```

## 开发优势

1. **无需服务器**：完全离线开发，MockNetwork 自动响应
2. **快速迭代**：修改代码后直接运行，无需启动服务器
3. **纯净环境**：只加载必要的系统，减少干扰
4. **真实接口**：使用与主程序相同的 NetContext API

## 扩展开发

### 添加新的模拟响应

编辑 `src/network/mock.rs` 中的 `handle_game_event` 方法：

```rust
match event {
    GameEvent::YourNewRequest { .. } => {
        // 处理逻辑
        let _ = response_tx.send(GameEvent::YourNewResponse { .. });
    }
    // ...
}
```

### 添加新系统

编辑 `src/bin/map_viewer/scene.rs` 中的 `create_system_scheduler` 方法：

```rust
scheduler
    .add_system(YourNewSystem::new())
    .add_system(AnotherSystem);
```

## 注意事项

- 地图文件需要放在 `Map/` 目录下
- 图形资源需要在 `Data/` 目录下的 .lib 文件
- 默认尝试加载：0.map, 1.map, 3.map, D01.map
- 如果没有地图文件，程序仍会启动但显示空白

## 与主程序的区别

| 特性 | MapViewerV3 | mir2x 主程序 |
|------|-------------|--------------|
| 网络 | MockNetwork | 真实TCP连接 |
| 场景 | MapViewerScene | 完整的3场景 |
| 系统 | 3个基础系统 | 15个完整系统 |
| UI | 无 | 完整UI界面 |
| 用途 | 开发调试 | 实际游戏 |

## 未来计划

- [ ] 添加地图选择器
- [ ] 添加性能监控面板
- [ ] 添加实体生成测试
- [ ] 添加路径寻找可视化
- [ ] 添加图层切换功能
