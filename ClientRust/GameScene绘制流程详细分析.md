# GameScene 绘制流程详细分析

## 📋 目录
1. [问题描述](#问题描述)
2. [完整绘制流程图](#完整绘制流程图)
3. [详细代码流程](#详细代码流程)
4. [关键代码位置](#关键代码位置)
5. [调试日志说明](#调试日志说明)
6. [常见问题排查](#常见问题排查)

---

## 问题描述

**用户反馈**: "登陆背景纹理被绘制在屏幕上了"

**现象**: 进入游戏场景后,登录场景的 ChrSel 动画背景仍然可见,没有被正确清除。

---

## 完整绘制流程图

```
程序启动
   │
   ├─→ main() 
   │      │
   │      └─→ Program::new()
   │             │
   │             ├─→ 初始化网络系统
   │             ├─→ 初始化 Ggez 渲染管理器
   │             ├─→ 加载图形库 (ChrSel, Prguse 等)
   │             └─→ 初始化场景管理器
   │                    │
   │                    └─→ LoginScene (初始场景)
   │
   ├─→ 游戏主循环 (60 FPS)
   │      │
   │      ├─→ Program::update() - 每帧更新
   │      │      │
   │      │      ├─→ 处理网络事件
   │      │      │      │
   │      │      │      ├─→ LoginSuccess → 切换到 SelectScene
   │      │      │      ├─→ StartGameResponse → 切换到 GameScene
   │      │      │      ├─→ MapInformation → 加载地图
   │      │      │      └─→ UserInformation → 创建玩家
   │      │      │
   │      │      └─→ scene_manager.update() - 更新当前场景
   │      │
   │      └─→ Program::draw() - 每帧绘制 ⭐ 关键入口
   │             │
   │             ├─→ [步骤 1] 创建 Canvas (动态背景色)
   │             │      │
   │             │      ├─→ LoginScene: Color::from_rgb(0, 0, 0) - 黑色
   │             │      ├─→ SelectScene: Color::from_rgb(0, 0, 0) - 黑色
   │             │      └─→ GameScene: Color::from_rgb(0, 32, 0) - 深绿色 ⭐
   │             │
   │             └─→ [步骤 2] 调用场景的 draw()
   │                    │
   │                    └─→ GameScene::draw() ⭐ 核心绘制逻辑
   │
   └─→ GameScene::draw() 详细流程 (见下方)
```

---

## 详细代码流程

### 🎯 Program::draw() - src/program.rs (lines 638-665)

```rust
fn draw(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult {
    self.ggez_manager.begin_frame();
    
    // ⭐ 关键修复: 根据场景类型动态选择背景色
    use ggez::graphics::Color;
    let bg_color = {
        let scene_manager = self.scene_manager.read();
        match scene_manager.current_scene_type() {
            // 登录/选择场景: 黑色背景
            Some(crate::scenes::SceneType::Login) | Some(crate::scenes::SceneType::Select) => {
                Color::from_rgb(0, 0, 0) // RGB(0, 0, 0) - 纯黑
            },
            // 游戏场景: 深绿色背景 (传奇2标准地图底色)
            Some(crate::scenes::SceneType::Game) => {
                Color::from_rgb(0, 32, 0) // RGB(0, 32, 0) - 深绿
            },
            None => Color::from_rgb(0, 0, 0),
        }
    };
    
    // 创建 Canvas 并自动清除 framebuffer (使用 bg_color)
    let mut canvas = ggez::graphics::Canvas::from_frame(ctx, bg_color);
    
    // 调用当前场景的 draw() 方法
    {
        let mut scene_manager = self.scene_manager.write();
        scene_manager.draw(ctx, &mut canvas);
    }
    
    // 完成绘制并显示
    canvas.finish(ctx)?;
    self.ggez_manager.end_frame();
    Ok(())
}
```

**关键点**:
- ✅ Canvas::from_frame() 会用 bg_color 清除整个 framebuffer
- ✅ 游戏场景使用深绿色 (0, 32, 0), 登录场景使用黑色 (0, 0, 0)
- ✅ 这是第一层防护

---

### 🎮 GameScene::draw() - src/scenes/game_scene.rs (lines 1223-1500+)

#### **步骤 1: 清除整个屏幕 (第二层防护)**

```rust
fn draw(&mut self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas) {
    let (screen_width, screen_height) = ctx.gfx.drawable_size();
    
    // ════════════════════════════════════════════════════════════
    // 步骤 1: 清除整个屏幕 (防止前一场景残留)
    // ════════════════════════════════════════════════════════════
    use ggez::graphics::{Color, Rect, DrawMode, Mesh, DrawParam};
    let clear_color = Color::from_rgb(0, 32, 0); // 深绿色
    let clear_rect = Rect::new(0.0, 0.0, screen_width, screen_height);
    
    if let Ok(clear_mesh) = Mesh::new_rectangle(ctx, DrawMode::fill(), clear_rect, clear_color) {
        canvas.draw(&clear_mesh, DrawParam::default());
        // 日志: ✅ 屏幕已清除为深绿色
    } else {
        // 日志: ❌ 无法创建清除用的矩形!
    }
```

**关键点**:
- ✅ 手动绘制全屏矩形,确保任何残留纹理被覆盖
- ✅ 这是第二层防护 (双重保险)

---

#### **步骤 2: 打印当前帧状态**

```rust
    // ════════════════════════════════════════════════════════════
    // 步骤 2: 打印当前帧计数和状态
    // ════════════════════════════════════════════════════════════
    static mut DRAW_COUNTER: u32 = 0;
    unsafe {
        DRAW_COUNTER += 1;
        if DRAW_COUNTER <= 10 || DRAW_COUNTER % 60 == 1 {
            println!("╔════════════════════════════════════════════════════════════════");
            println!("║ 🎬 GameScene::draw() 被调用 - 第 {} 帧", DRAW_COUNTER);
            println!("╚════════════════════════════════════════════════════════════════");
            println!("   当前状态: {:?}", self.state);
            println!("   屏幕尺寸: {:.0}x{:.0}", screen_width, screen_height);
            println!("   地图已加载: {}", self.map_renderer.width > 0);
            println!("   玩家已创建: {}", self.user.is_some());
            if let Some(ref user) = self.user {
                println!("   玩家位置: ({}, {})", 
                    user.player.map_object.movement.x, 
                    user.player.map_object.movement.y);
            }
            println!("════════════════════════════════════════════════════════════════\n");
        }
    }
```

**日志输出示例**:
```
╔════════════════════════════════════════════════════════════════
║ 🎬 GameScene::draw() 被调用 - 第 1 帧
╚════════════════════════════════════════════════════════════════
   当前状态: Ready
   屏幕尺寸: 1024x768
   地图已加载: true
   玩家已创建: true
   玩家位置: (100, 100)
════════════════════════════════════════════════════════════════
```

---

#### **步骤 3: 状态机检查**

```rust
    // ════════════════════════════════════════════════════════════
    // 步骤 3: 状态机检查 - 只有 Ready 状态才渲染游戏
    // ════════════════════════════════════════════════════════════
    match &self.state {
        GameSceneState::WaitingForData => {
            // 显示 "等待服务器数据..." 提示
            self.draw_loading_screen(canvas, "等待服务器数据...", screen_width, screen_height);
            return; // ⚠️ 提前返回,不渲染游戏内容
        },
        GameSceneState::LoadingMap(map_name) => {
            // 显示 "正在加载地图: XXX" 提示
            let msg = format!("正在加载地图: {}", map_name);
            self.draw_loading_screen(canvas, &msg, screen_width, screen_height);
            return; // ⚠️ 提前返回
        },
        GameSceneState::WaitingForPlayer => {
            // 显示 "等待角色数据..." 提示
            self.draw_loading_screen(canvas, "等待角色数据...", screen_width, screen_height);
            return; // ⚠️ 提前返回
        },
        GameSceneState::Ready => {
            // ✅ 状态正常,继续渲染游戏
            println!("╔════════════════════════════════════════════════════════════════");
            println!("║ ✅ 状态: Ready - 正常渲染游戏");
            println!("╚════════════════════════════════════════════════════════════════");
            println!("   📍 地图尺寸: {}x{}", self.map_renderer.width, self.map_renderer.height);
            println!("   👤 玩家存在: {}", self.user.is_some());
            println!("   🎥 摄像机: ({:.1}, {:.1})", self.camera.x, self.camera.y);
            println!("════════════════════════════════════════════════════════════════\n");
        }
    }
```

**关键点**:
- ⚠️ 只有 `Ready` 状态才会继续渲染游戏内容
- ⚠️ 其他状态会显示加载提示并 `return`

**状态转换流程**:
```
WaitingForData → LoadingMap → WaitingForPlayer → Ready
      ↑              ↑              ↑               ↑
  场景初始化     收到MapInfo    收到UserInfo    可以渲染游戏
```

---

#### **步骤 4: 更新摄像机**

```rust
    // ════════════════════════════════════════════════════════════
    // 步骤 4: 更新摄像机
    // ════════════════════════════════════════════════════════════
    
    // 4a. 更新摄像机屏幕尺寸
    self.camera.update_screen_size(screen_width, screen_height);
    
    // 4b. 让摄像机跟随玩家 (带地图边界限制)
    if let Some(ref user) = self.user {
        // 计算玩家世界坐标 (像素)
        let player_world_x = (user.player.map_object.movement.x as f32 * MapRenderer::CELL_WIDTH as f32) 
            + user.player.map_object.offset_move.x as f32;
        let player_world_y = (user.player.map_object.movement.y as f32 * MapRenderer::CELL_HEIGHT as f32) 
            + user.player.map_object.offset_move.y as f32;
        
        // 计算地图像素尺寸
        let map_width_px = self.map_renderer.width as f32 * MapRenderer::CELL_WIDTH as f32;
        let map_height_px = self.map_renderer.height as f32 * MapRenderer::CELL_HEIGHT as f32;
        
        // 🐛 DEBUG: 详细打印摄像机更新过程
        println!("╔════════════════════════════════════════════════════════════════");
        println!("║ 🎥 摄像机更新 #1");
        println!("╚════════════════════════════════════════════════════════════════");
        println!("   📍 玩家格子: ({}, {})", 
            user.player.map_object.movement.x, 
            user.player.map_object.movement.y);
        println!("   📐 玩家偏移: ({}, {})", 
            user.player.map_object.offset_move.x, 
            user.player.map_object.offset_move.y);
        println!("   🌍 玩家世界坐标: ({:.1}, {:.1}) 像素", player_world_x, player_world_y);
        println!("   🗺️  地图尺寸: {:.1} x {:.1} 像素", map_width_px, map_height_px);
        println!("   🎥 摄像机更新前: ({:.1}, {:.1})", self.camera.x, self.camera.y);
        
        // 让摄像机跟随玩家 (带边界限制)
        self.camera.follow_target_clamped(player_world_x, player_world_y, map_width_px, map_height_px);
        
        println!("   🎥 摄像机更新后: ({:.1}, {:.1})", self.camera.x, self.camera.y);
        println!("════════════════════════════════════════════════════════════════\n");
    }
```

**日志输出示例**:
```
╔════════════════════════════════════════════════════════════════
║ 🎥 摄像机更新 #1
╚════════════════════════════════════════════════════════════════
   📍 玩家格子: (100, 100)
   📐 玩家偏移: (0, 0)
   🌍 玩家世界坐标: (4800.0, 3200.0) 像素
   🗺️  地图尺寸: 9600.0 x 6400.0 像素
   🎥 摄像机更新前: (4800.0, 3200.0)
   🎥 摄像机更新后: (4800.0, 3200.0)
════════════════════════════════════════════════════════════════
```

---

#### **步骤 5: 绘制地图与游戏对象**

```rust
    // ════════════════════════════════════════════════════════════
    // 步骤 5: 绘制地图与游戏对象
    // ════════════════════════════════════════════════════════════
    
    // 5a. 准备玩家位置数据
    let user_pos = if let Some(ref user) = self.user {
        UserPosition {
            x: user.player.map_object.movement.x,
            y: user.player.map_object.movement.y,
            offset_x: user.player.map_object.offset_move.x,
            offset_y: user.player.map_object.offset_move.y,
        }
    } else {
        UserPosition {
            x: self.map_renderer.width / 2,
            y: self.map_renderer.height / 2,
            offset_x: 0,
            offset_y: 0,
        }
    };
    
    // 5b. 绘制地图 (MapRenderer 会根据摄像机计算可见区域)
    if let Err(e) = self.map_renderer.draw(ctx, canvas, &self.camera) {
        // 日志: ❌ 地图绘制失败
    } else {
        // 日志: ✅ 地图绘制成功
    }
    
    // 5c. 绘制玩家角色
    if let Some(ref user) = self.user {
        if let Err(e) = self.draw_player_with_camera(ctx, canvas, &user_pos) {
            // 日志: ❌ 玩家绘制失败
        } else {
            // 日志: ✅ 玩家绘制成功
        }
    }
```

---

#### **步骤 6-7: UI 与顶层元素 (TODO)**

```rust
    // ════════════════════════════════════════════════════════════
    // 步骤 6: 绘制 UI 控件树 (TODO)
    // ════════════════════════════════════════════════════════════
    // TODO: 遍历 self.controls 并调用 draw
    
    // ════════════════════════════════════════════════════════════
    // 步骤 7: 绘制顶层元素 (TODO)
    // ════════════════════════════════════════════════════════════
    // TODO: 绘制鼠标提示、输出消息、对话框等
}
```

---

### 👤 GameScene::draw_player_with_camera()

```rust
fn draw_player_with_camera(&self, ctx: &mut ggez::Context, canvas: &mut ggez::graphics::Canvas, _user_pos: &UserPosition) -> ggez::GameResult<()> {
    if let Some(ref user) = self.user {
        // 步骤 1: 计算玩家世界坐标 (像素)
        let player_world_x = (user.player.map_object.movement.x as f32 * MapRenderer::CELL_WIDTH as f32) 
            + user.player.map_object.offset_move.x as f32;
        let player_world_y = (user.player.map_object.movement.y as f32 * MapRenderer::CELL_HEIGHT as f32) 
            + user.player.map_object.offset_move.y as f32;
        
        // 步骤 2: 世界坐标转屏幕坐标
        let (screen_x, screen_y) = self.camera.world_to_screen(player_world_x, player_world_y);
        
        // 🐛 DEBUG: 首帧详细打印
        println!("╔════════════════════════════════════════════════════════════════");
        println!("║ 👤 玩家角色绘制详细信息");
        println!("╚════════════════════════════════════════════════════════════════");
        println!("   📍 格子位置: ({}, {})", 
            user.player.map_object.movement.x, 
            user.player.map_object.movement.y);
        println!("   📐 移动偏移: ({}, {})", 
            user.player.map_object.offset_move.x, 
            user.player.map_object.offset_move.y);
        println!("   🌍 世界坐标: ({:.1}, {:.1}) 像素", player_world_x, player_world_y);
        println!("   🎥 摄像机位置: ({:.1}, {:.1})", self.camera.x, self.camera.y);
        println!("   🖥️  屏幕坐标: ({:.1}, {:.1})", screen_x, screen_y);
        println!("   📺 屏幕尺寸: ({:.0}, {:.0})", 
            self.camera.get_screen_size().0, 
            self.camera.get_screen_size().1);
        println!("   🧭 朝向: {:?}", user.player.map_object.direction);
        println!("   👤 性别: {:?}, 职业: {:?}", user.player.gender, user.player.class);
        println!("════════════════════════════════════════════════════════════════\n");
        
        // 步骤 3a: 绘制占位符 - 绿色矩形框
        let player_rect = Rect::new(screen_x - 20.0, screen_y - 40.0, 40.0, 60.0);
        let rect_mesh = Mesh::new_rectangle(
            ctx,
            DrawMode::stroke(2.0),
            player_rect,
            Color::from_rgb(0, 255, 0), // 绿色边框
        )?;
        canvas.draw(&rect_mesh, DrawParam::default());
        // 日志: ✅ 玩家占位框已绘制
        
        // 步骤 3b: 绘制占位符 - 黄色圆点
        let circle_mesh = Mesh::new_circle(
            ctx,
            DrawMode::fill(),
            [screen_x, screen_y],
            5.0,
            0.1,
            Color::from_rgb(255, 255, 0), // 黄色
        )?;
        canvas.draw(&circle_mesh, DrawParam::default());
        // 日志: ✅ 玩家中心点已绘制
        
        // 步骤 4: 计算角色纹理索引
        let class_base = match user.player.class {
            MirClass::Warrior => 0,      // 战士: 0-39
            MirClass::Wizard => 40,      // 法师: 40-79
            MirClass::Taoist => 80,      // 道士: 80-119
            MirClass::Assassin => 120,   // 刺客: 120-159
            MirClass::Archer => 160,     // 弓手: 160-199
        };
        
        let gender_offset = match user.player.gender {
            MirGender::Male => 0,        // 男性: +0
            MirGender::Female => 20,     // 女性: +20
        };
        
        let direction = user.player.map_object.direction as usize; // 0-7
        let frame_index = class_base + gender_offset + direction;
        
        // 日志: 🎨 角色纹理索引: frame_index (职业:class_base + 性别:gender_offset + 方向:direction)
        
        // 步骤 5: 绘制角色纹理 (ChrSel 库)
        if let Some(lib_arc) = get_library(LibraryName::ChrSel) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                let image_count = lib.count();
                
                if frame_index < image_count {
                    // 日志: 🎨 开始绘制 ChrSel[frame_index] 纹理...
                    
                    match lib.draw_with_color(
                        ctx,
                        canvas,
                        frame_index,
                        screen_x,
                        screen_y - 20.0, // 稍微往上偏移
                        Color::WHITE,
                        true, // use_offset (使用图像偏移量)
                    ) {
                        Ok(_) => {
                            // 日志: ✅ ChrSel[frame_index] 纹理绘制成功
                        },
                        Err(e) => {
                            // 日志: ❌ ChrSel[frame_index] 纹理绘制失败
                        }
                    }
                } else {
                    // 日志: ❌ 角色纹理索引越界
                }
            }
        }
    }
    
    Ok(())
}
```

**日志输出示例**:
```
╔════════════════════════════════════════════════════════════════
║ 👤 玩家角色绘制详细信息
╚════════════════════════════════════════════════════════════════
   📍 格子位置: (100, 100)
   📐 移动偏移: (0, 0)
   🌍 世界坐标: (4800.0, 3200.0) 像素
   🎥 摄像机位置: (4800.0, 3200.0)
   🖥️  屏幕坐标: (512.0, 384.0)
   📺 屏幕尺寸: (1024, 768)
   🧭 朝向: Up
   👤 性别: Male, 职业: Warrior
════════════════════════════════════════════════════════════════
```

---

## 关键代码位置

| 文件 | 行数 | 功能 | 重要性 |
|------|------|------|--------|
| `src/program.rs` | 638-665 | 动态 Canvas 背景色选择 | ⭐⭐⭐ 核心修复 |
| `src/scenes/game_scene.rs` | 1223-1260 | Canvas 清除 (第二层防护) | ⭐⭐⭐ 核心修复 |
| `src/scenes/game_scene.rs` | 1330-1365 | 状态机检查 | ⭐⭐ 重要 |
| `src/scenes/game_scene.rs` | 1390-1450 | 摄像机更新 | ⭐⭐ 重要 |
| `src/scenes/game_scene.rs` | 1465-1510 | 地图与玩家绘制 | ⭐⭐ 重要 |
| `src/scenes/game_scene.rs` | 950-1120 | 玩家角色绘制详细逻辑 | ⭐ 辅助 |

---

## 调试日志说明

### 🎬 GameScene::draw() 入口日志

**频率**: 前10帧 + 每60帧一次

**输出**:
```
╔════════════════════════════════════════════════════════════════
║ 🎬 GameScene::draw() 被调用 - 第 N 帧
╚════════════════════════════════════════════════════════════════
   当前状态: Ready
   屏幕尺寸: 1024x768
   地图已加载: true
   玩家已创建: true
   玩家位置: (100, 100)
════════════════════════════════════════════════════════════════
```

**检查点**:
- ✅ 状态应该是 `Ready`
- ✅ 地图已加载应该是 `true`
- ✅ 玩家已创建应该是 `true`

---

### 🎥 摄像机更新日志

**频率**: 前5帧 + 每60帧一次

**输出**:
```
╔════════════════════════════════════════════════════════════════
║ 🎥 摄像机更新 #N
╚════════════════════════════════════════════════════════════════
   📍 玩家格子: (100, 100)
   📐 玩家偏移: (0, 0)
   🌍 玩家世界坐标: (4800.0, 3200.0) 像素
   🗺️  地图尺寸: 9600.0 x 6400.0 像素
   🎥 摄像机更新前: (4800.0, 3200.0)
   🎥 摄像机更新后: (4800.0, 3200.0)
════════════════════════════════════════════════════════════════
```

**检查点**:
- ✅ 玩家世界坐标 = 格子 * 格子尺寸 + 偏移
- ✅ 摄像机位置应该跟随玩家

---

### 👤 玩家角色绘制日志

**频率**: 仅首帧

**输出**:
```
╔════════════════════════════════════════════════════════════════
║ 👤 玩家角色绘制详细信息
╚════════════════════════════════════════════════════════════════
   📍 格子位置: (100, 100)
   📐 移动偏移: (0, 0)
   🌍 世界坐标: (4800.0, 3200.0) 像素
   🎥 摄像机位置: (4800.0, 3200.0)
   🖥️  屏幕坐标: (512.0, 384.0)
   📺 屏幕尺寸: (1024, 768)
   🧭 朝向: Up
   👤 性别: Male, 职业: Warrior
════════════════════════════════════════════════════════════════
```

**检查点**:
- ✅ 屏幕坐标应该在屏幕中央附近 (约 512, 384)
- ✅ 世界坐标 - 摄像机位置 = 相对位置

---

## 常见问题排查

### ❌ 问题 1: 登录背景残留

**现象**: 进入游戏后,能看到登录场景的 ChrSel 动画背景

**原因**:
1. Canvas::from_frame() 没有正确清除 framebuffer
2. GameScene::draw() 没有手动清除屏幕

**解决方案**:
- ✅ 已修复: program.rs 动态选择 bg_color
- ✅ 已修复: game_scene.rs 手动绘制全屏矩形

**验证日志**:
```
✅ 屏幕已清除为深绿色 (1024x768)
```

---

### ❌ 问题 2: 游戏场景不显示

**现象**: 黑屏或绿屏,没有地图和玩家

**可能原因**:
1. 状态不是 `Ready` (停在 WaitingForData/LoadingMap/WaitingForPlayer)
2. 地图加载失败 (map_renderer.width == 0)
3. 玩家未创建 (self.user == None)

**检查日志**:
```
当前状态: Ready               ← 应该是 Ready
地图已加载: true              ← 应该是 true
玩家已创建: true              ← 应该是 true
```

---

### ❌ 问题 3: 摄像机不跟随玩家

**现象**: 玩家在屏幕边缘或看不见

**可能原因**:
1. 摄像机位置计算错误
2. 边界限制太严格

**检查日志**:
```
🎥 玩家世界坐标: (4800.0, 3200.0) 像素
🎥 摄像机更新后: (4800.0, 3200.0)     ← 应该接近玩家坐标
```

---

### ❌ 问题 4: 玩家不显示

**现象**: 地图显示正常,但没有玩家角色

**可能原因**:
1. self.user 为 None
2. ChrSel 库未加载
3. 纹理索引越界

**检查日志**:
```
玩家已创建: true                          ← 应该是 true
✅ 玩家占位框已绘制                       ← 应该看到这条
✅ ChrSel[0] 纹理绘制成功                 ← 应该看到这条
```

---

## 总结

### 双重保险机制

**第一层 (program.rs)**:
```rust
// Canvas 创建时根据场景类型选择背景色
let bg_color = match scene_type {
    Login | Select => Color::from_rgb(0, 0, 0),  // 黑色
    Game => Color::from_rgb(0, 32, 0),           // 深绿色
};
let mut canvas = Canvas::from_frame(ctx, bg_color); // 自动清除
```

**第二层 (game_scene.rs)**:
```rust
// 手动绘制全屏矩形覆盖任何残留
let clear_rect = Rect::new(0.0, 0.0, screen_width, screen_height);
let clear_mesh = Mesh::new_rectangle(ctx, DrawMode::fill(), clear_rect, clear_color)?;
canvas.draw(&clear_mesh, DrawParam::default());
```

### 绘制顺序

```
1. 清除屏幕 (深绿色全屏矩形)
2. 检查状态 (不是 Ready 就返回)
3. 更新摄像机 (跟随玩家)
4. 绘制地图 (MapRenderer)
5. 绘制玩家 (draw_player_with_camera)
6. 绘制 UI (TODO)
7. 绘制顶层元素 (TODO)
```

### 关键状态转换

```
程序启动 → LoginScene → SelectScene → GameScene
                                           ↓
                                    WaitingForData
                                           ↓ (收到 MapInformation)
                                    LoadingMap
                                           ↓ (地图加载完成)
                                    WaitingForPlayer
                                           ↓ (收到 UserInformation)
                                    Ready ⭐ 开始渲染游戏
```

---

**文档生成时间**: 2025-10-15  
**作者**: GitHub Copilot  
**版本**: 1.0
