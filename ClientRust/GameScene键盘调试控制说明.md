# GameScene 键盘调试控制说明

## 概述
GameScene 现已集成 MapRenderer 显示控制功能,支持通过键盘快捷键实时切换各种调试视图。

## 实现位置
- **文件**: `src/scenes/game_scene.rs`
- **方法**: `handle_key_press()` (第1933-2036行)
- **接口**: 实现了 `Scene` trait 的键盘处理方法

## 键盘快捷键

| 按键 | 功能 | 默认状态 | 说明 |
|------|------|----------|------|
| **G** | 地图网格 | ❌ 关闭 | 显示绿色网格线,每个格子代表一个 tile (48x32) |
| **B** | 纹理边框 | ❌ 关闭 | 显示 tile 纹理边框(正常=红色, 混合=蓝色) |
| **1** | Back层 | ✅ 开启 | 控制背景层(地板/草地)的显示 |
| **2** | Middle层 | ✅ 开启 | 控制中间层(墙壁/树木/建筑)的显示 |
| **3** | Front层 | ✅ 开启 | 控制前景层(屋顶/遮挡)的显示 |
| **O** | 障碍层 | ❌ 关闭 | 显示红色半透明矩形标记不可通行区域 |
| **A** | 动画效果 | ✅ 开启 | 控制所有动画(水流/火焰/门/特效)的播放 |
| **U** | 玩家显示 | ✅ 开启 | 控制玩家角色的显示/隐藏 |

## 使用示例

### 调试地图渲染问题
```
1. 按 G 键显示网格,检查格子对齐
2. 按 B 键显示边框,检查纹理偏移
3. 按 1/2/3 键单独查看各层,定位渲染层级问题
4. 按 U 键隐藏角色,纯粹观察地图渲染
```

### 调试碰撞检测
```
1. 按 O 键显示障碍层
2. 移动角色测试碰撞
3. 红色区域应该与阻挡位置一致
4. 按 U 键隐藏角色,观察障碍层覆盖范围
```

### 性能调试
```
1. 按 A 键关闭动画,测试性能影响
2. 按 1/2/3 键关闭某些层,隔离性能瓶颈
3. 按 U 键隐藏角色,测试角色渲染开销
```

## 技术细节

### 实现机制
```rust
// Scene trait 方法签名 (src/scenes/mod.rs)
fn handle_key_press(&mut self, key: KeyCode, modifiers: ModifiersState) -> bool;

// GameScene 实现 (src/scenes/game_scene.rs)
fn handle_key_press(&mut self, key: KeyCode, _modifiers: ModifiersState) -> bool {
    match key {
        KeyCode::KeyG => {
            self.map_renderer.show_grid = !self.map_renderer.show_grid;
            println!("🔍 地图网格: {}", if self.map_renderer.show_grid { "开启" } else { "关闭" });
            true
        },
        // ... 其他按键处理
        _ => false,
    }
}
```

### 状态切换
- 每次按键都会切换对应参数的布尔值(`true` ↔ `false`)
- 控制台会打印状态变化消息(带表情符号)
- 返回 `true` 表示按键已被处理,事件不再传播

### MapRenderer 参数
所有控制参数都直接修改 `self.map_renderer` 的公开字段:
```rust
pub struct MapRenderer {
    pub show_grid: bool,         // G 键控制
    pub show_borders: bool,      // B 键控制
    pub show_layer_back: bool,   // 1 键控制
    pub show_layer_middle: bool, // 2 键控制
    pub show_layer_front: bool,  // 3 键控制
    pub show_obstacles: bool,    // O 键控制
    pub show_animations: bool,   // A 键控制
    // ... 其他字段
}
```

## 视觉反馈

### 网格显示 (G 键)
- **颜色**: 绿色(RGBA: 0, 255, 0, 100)
- **样式**: 1像素宽度线条
- **覆盖**: 垂直线和水平线形成格子

### 边框显示 (B 键)
- **正常 tile**: 红色边框(RGBA: 255, 0, 0, 200)
- **混合 tile**: 蓝色边框(RGBA: 0, 0, 255, 200)
- **线宽**: 1像素

### 障碍层 (O 键)
- **颜色**: 红色半透明(RGBA: 255, 0, 0, 128)
- **样式**: 填充矩形
- **标记**: 所有 blocked cell

### 层级控制 (1/2/3 键)
- **Back层**: 关闭后看不到地面纹理
- **Middle层**: 关闭后墙壁/建筑消失
- **Front层**: 关闭后屋顶/遮挡消失

## 性能影响

| 功能 | 性能开销 | 说明 |
|------|----------|------|
| 网格 | 极小 | 只画线,几乎无开销 |
| 边框 | 小 | 每个 tile 画4条线,开销可控 |
| 障碍层 | 小 | 只画阻挡格子,通常<30%的格子 |
| 关闭层 | **负开销** | 跳过整层渲染,可提升性能 |
| 关闭动画 | **负开销** | 跳过动画更新/渲染,显著提升性能 |

## 调试技巧

### 问题: 地图偏移
```
1. 按 G 键显示网格
2. 检查 tile 边界是否对齐网格
3. 按 B 键查看具体 tile 纹理边界
```

### 问题: 角色被奇怪位置阻挡
```
1. 按 O 键显示障碍层
2. 移动到问题位置
3. 检查红色矩形是否覆盖不合理位置
```

### 问题: 性能低下
```
1. 按 A 键关闭动画,观察 FPS 变化
2. 按 3 键关闭 Front 层,检查是否因遮挡层过多
3. 按 2 键关闭 Middle 层,检查是否因中间层过重
```

### 问题: 层级错误(物体显示不对)
```
1. 按 1 键单独查看 Back 层
2. 按 2 键单独查看 Middle 层
3. 按 3 键单独查看 Front 层
4. 定位哪一层的渲染有问题
```

## 开发注意事项

### 添加新的键盘控制
要添加新的调试快捷键,需要:

1. **在 MapRenderer 添加字段** (src/scenes/game_scene/map_renderer.rs):
```rust
pub struct MapRenderer {
    pub show_new_feature: bool,  // 新功能开关
    // ...
}
```

2. **在 Default 初始化** (同文件):
```rust
impl Default for MapRenderer {
    fn default() -> Self {
        Self {
            show_new_feature: false,  // 默认值
            // ...
        }
    }
}
```

3. **在 draw() 添加条件渲染** (同文件):
```rust
pub fn draw(&mut self, ...) -> GameResult {
    // ...
    if self.show_new_feature {
        self.draw_new_feature(ctx, canvas, camera)?;
    }
    Ok(())
}
```

4. **在 GameScene 添加键盘处理** (src/scenes/game_scene.rs):
```rust
fn handle_key_press(&mut self, key: KeyCode, _modifiers: ModifiersState) -> bool {
    match key {
        KeyCode::KeyN => {  // 使用 N 键
            self.map_renderer.show_new_feature = !self.map_renderer.show_new_feature;
            println!("🔍 新功能: {}", if self.map_renderer.show_new_feature { "开启" } else { "关闭" });
            true
        },
        // ... 其他按键
    }
}
```

### 键盘事件流程
```
用户按键
    ↓
ggez KeyboardInput 事件
    ↓
main.rs event_handler
    ↓
scene_manager.handle_key_press()
    ↓
active_scene.handle_key_press()  ← GameScene 实现
    ↓
修改 map_renderer 字段
    ↓
下一帧 draw() 时生效
```

## 相关文档
- **MapRenderer显示控制功能说明.md**: 渲染层详细实现
- **src/scenes/mod.rs**: Scene trait 和 KeyCode 定义
- **src/scenes/game_scene/map_renderer.rs**: 渲染实现

## 更新历史
- **2024-01-XX**: 初始实现,支持 7 个调试快捷键(G/B/1/2/3/O/A)
- **编译状态**: ✅ 编译通过,等待实际测试

---

**测试清单**:
- [ ] G 键 - 网格显示正常
- [ ] B 键 - 边框颜色正确(红/蓝)
- [ ] 1 键 - Back 层可正常开关
- [ ] 2 键 - Middle 层可正常开关
- [ ] 3 键 - Front 层可正常开关
- [ ] O 键 - 障碍层红色矩形显示正确
- [ ] A 键 - 动画可以暂停/恢复
- [ ] U 键 - 玩家角色可以显示/隐藏
- [ ] 控制台消息正确打印(带表情符号)
