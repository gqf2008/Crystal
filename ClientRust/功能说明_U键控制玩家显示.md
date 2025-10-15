# GameScene U键控制玩家显示功能说明

## 功能概述
**U键**: 切换玩家角色的显示/隐藏状态

## 实现位置

### 1. 结构体字段
**文件**: `src/scenes/game_scene.rs` 第421-423行

```rust
// ==================== 调试控制 ====================
/// 是否显示玩家角色 (调试用，U键控制)
show_player: bool,
```

### 2. 字段初始化
**文件**: `src/scenes/game_scene.rs` 第580行

```rust
// 调试控制
show_player: true, // 默认显示玩家
```

### 3. 键盘处理
**文件**: `src/scenes/game_scene.rs` 第2036-2048行

```rust
// U键: 切换玩家显示
KeyCode::KeyU => {
    self.show_player = !self.show_player;
    println!(
        "👤 玩家显示: {}",
        if self.show_player {
            "开启"
        } else {
            "关闭"
        }
    );
    true
}
```

### 4. 绘制控制
**文件**: `src/scenes/game_scene.rs` 第1463-1474行

```rust
// 5c. 绘制玩家角色 (使用摄像机转换坐标)
if self.user.is_some() && self.show_player {
    tracing::trace!("👤 开始绘制玩家角色...");
    if let Err(e) = self.draw_player_with_camera(ctx, canvas, &user_pos) {
        tracing::error!("❌ 玩家绘制失败: {:?}", e);
    } else {
        tracing::trace!("✅ 玩家绘制成功");
    }
} else if !self.show_player {
    tracing::trace!("👤 玩家显示已关闭 (U键控制)");
} else {
    tracing::warn!("⚠️  没有玩家数据，跳过玩家绘制");
}
```

## 使用方法

### 基本操作
1. **运行游戏**: `cargo run --bin mir2_client`
2. **进入游戏场景**: 登录 → 选择角色 → 进入游戏
3. **按 U 键**: 切换玩家显示状态
4. **查看控制台**: 看到 "👤 玩家显示: 开启/关闭" 消息

### 视觉效果
- **U键按下 (第一次)**: 玩家角色消失,只显示地图
- **U键按下 (第二次)**: 玩家角色重新出现
- **切换即时生效**: 无需等待,立即看到变化

## 应用场景

### 1. 调试地图渲染
```
目的: 纯粹观察地图,不被角色遮挡
步骤:
  1. 按 U 键隐藏角色
  2. 按 G 键显示网格
  3. 按 B 键显示边框
  4. 检查地图纹理对齐情况
```

### 2. 调试碰撞检测
```
目的: 观察障碍层与角色的关系
步骤:
  1. 按 O 键显示障碍层
  2. 按 U 键隐藏角色
  3. 观察红色障碍矩形覆盖范围
  4. 按 U 键恢复角色,测试碰撞
```

### 3. 性能分析
```
目的: 测试角色渲染的性能开销
步骤:
  1. 记录当前 FPS
  2. 按 U 键隐藏角色
  3. 观察 FPS 变化
  4. 判断角色渲染是否有性能问题
```

### 4. 截图/录制
```
目的: 获取纯地图画面
步骤:
  1. 按 U 键隐藏角色
  2. 截图或录屏
  3. 得到干净的地图图像
```

## 技术细节

### 控制流程
```
用户按 U 键
    ↓
ggez KeyboardInput 事件
    ↓
main.rs event_handler
    ↓
scene_manager.handle_key_press()
    ↓
GameScene.handle_key_press(KeyCode::KeyU)
    ↓
self.show_player = !self.show_player  ← 切换状态
    ↓
println!("👤 玩家显示: 开启/关闭")
    ↓
下一帧 draw() 时检查 show_player
    ↓
if self.show_player { draw_player() }  ← 条件渲染
```

### 状态管理
- **字段类型**: `bool` (简单开关)
- **默认值**: `true` (显示玩家)
- **切换方式**: `!self.show_player` (取反)
- **作用域**: 整个 GameScene 生命周期

### 绘制逻辑
**修改前**:
```rust
if self.user.is_some() {
    self.draw_player_with_camera(ctx, canvas, &user_pos)?;
}
```

**修改后**:
```rust
if self.user.is_some() && self.show_player {
    self.draw_player_with_camera(ctx, canvas, &user_pos)?;
}
```

**关键点**:
- 同时检查 `user` 存在和 `show_player` 开启
- 短路求值: 如果 `user` 不存在,不会检查 `show_player`
- 即时生效: 每帧重新检查,无缓存

### 性能影响
| 操作 | 开销 | 说明 |
|------|------|------|
| 切换状态 | 极小 | 只改变一个 bool 值 |
| 绘制检查 | 极小 | 一次布尔判断 |
| 隐藏角色 | **负开销** | 跳过整个角色渲染流程 |

**估算**:
- 角色渲染约 0.5-1ms (取决于动画复杂度)
- 隐藏角色可省略这部分开销
- 在低端设备上可能提升 1-2 FPS

## 与其他快捷键的配合

### 组合1: 纯地图观察
```
G + B + U
  ↓
网格 + 边框 - 角色
  ↓
观察地图格子对齐情况
```

### 组合2: 层级调试
```
1/2/3 + U
  ↓
单独显示某层 - 角色
  ↓
定位层级渲染问题
```

### 组合3: 障碍检测
```
O + U
  ↓
障碍层 - 角色
  ↓
检查障碍覆盖范围
```

### 组合4: 动画分析
```
A + U
  ↓
关闭动画 + 隐藏角色
  ↓
只观察静态地图
```

## 调试消息

### 控制台输出
```
👤 玩家显示: 开启   ← U键第一次按下 (隐藏 → 显示)
👤 玩家显示: 关闭   ← U键第二次按下 (显示 → 隐藏)
```

### Trace 日志 (--features tracing)
```
TRACE 👤 开始绘制玩家角色...        ← show_player = true
TRACE ✅ 玩家绘制成功
TRACE 👤 玩家显示已关闭 (U键控制)  ← show_player = false
```

## 测试验证

### 功能测试
```bash
# 1. 编译项目
cargo build --lib

# 2. 运行游戏
cargo run --bin mir2_client

# 3. 进入游戏场景
登录 → 选择角色 → 进入游戏

# 4. 测试 U 键
按 U 键 → 玩家消失
按 U 键 → 玩家出现
```

### 预期结果
- ✅ 按 U 键后角色立即消失/出现
- ✅ 控制台打印 "👤 玩家显示: 开启/关闭"
- ✅ 地图渲染不受影响
- ✅ 摄像机继续跟随(即使角色隐藏)

### 已知限制
- **摄像机仍跟随**: 即使角色隐藏,摄像机仍以角色位置为中心
  - **原因**: 摄像机更新在绘制检查之前
  - **影响**: 无,这是预期行为
- **碰撞仍生效**: 角色虽然隐藏,但碰撞检测仍在运行
  - **原因**: 碰撞逻辑在 `update()` 中,与绘制分离
  - **影响**: 无,这是预期行为

## 扩展建议

### 1. 添加 UI 指示器
```rust
// 在屏幕角落显示当前显示状态
if !self.show_player {
    let text = Text::new("👤 角色已隐藏 (U键)");
    canvas.draw(&text, DrawParam::default()
        .dest([10.0, 10.0])
        .color(Color::RED));
}
```

### 2. 支持其他对象隐藏
```rust
show_npcs: bool,      // N键控制 NPC 显示
show_monsters: bool,  // M键控制怪物显示
show_players: bool,   // P键控制其他玩家显示
show_items: bool,     // I键控制地面物品显示
```

### 3. 批量控制
```rust
KeyCode::KeyH => {
    // H键: 隐藏所有对象,只保留地图
    self.show_player = false;
    self.show_npcs = false;
    self.show_monsters = false;
    println!("🙈 隐藏所有对象");
    true
}
```

### 4. 配置保存
```rust
// 保存到配置文件,下次启动时恢复
#[derive(Serialize, Deserialize)]
struct DebugSettings {
    show_player: bool,
    show_grid: bool,
    show_borders: bool,
    // ...
}
```

## 相关文档
- **GameScene键盘调试控制说明.md**: 完整的键盘快捷键列表
- **MapRenderer显示控制功能说明.md**: 地图渲染层控制详解

## 更新历史
- **2025-10-15**: 初始实现 U键控制玩家显示功能
- **编译状态**: ✅ 通过 (3.55秒)
- **测试状态**: ⏳ 等待用户测试验证

---

**快速参考**:
```
U键 = 切换玩家显示
默认 = 显示 (show_player: true)
用途 = 调试地图/性能分析/截图
```
