# 🚀 明天开发快速参考

## 今天完成了什么 ✅

### 核心功能
- ✅ 文本输入系统（账号/密码）
- ✅ 光标闪烁动画（0.5秒/次）
- ✅ 按钮悬停效果（4个按钮）
- ✅ 焦点管理（Tab切换，点击切换）
- ✅ 键盘快捷键（Enter, Tab, Backspace, Escape, Ctrl+Q）
- ✅ 输入验证（字母数字，3-20字符）
- ✅ 密码遮挡显示（***）

### 代码位置
```
ClientRust/src/scenes/login_scene/
├── login_dialog.rs      ← 对话框逻辑（重点修改）
└── mod.rs              ← LoginScene（少量修改）

ClientRust/src/scenes/
├── mod.rs              ← Scene trait（添加了 handle_text_input）
└── scene_manager.rs    ← 添加了 handle_text_input 转发

ClientRust/src/main_ggez.rs ← 添加了 text_input_event
```

---

## 明天要做什么 🎯

### 优先级 1: 登录逻辑 (1-2小时)

#### 任务 4.1: 网络请求发送
```rust
// 在 LoginScene::submit_login() 中
fn submit_login(&mut self) {
    if let Some((username, password)) = self.login_dialog.get_credentials() {
        // TODO: 实际发送网络请求
        // 参考: Client/MirScenes/LoginScene.cs Line 500+
        
        // 创建 Login packet
        use mir2_shared::packets::client::Login;
        let login_packet = Login {
            account_id: username,
            password: password,
        };
        
        // 发送到网络线程
        // self.network_tx.send(NetworkCommand::Login(login_packet))?;
    }
}
```

#### 任务 4.2: 错误消息显示
```rust
// 在 LoginScene 中添加
pub struct LoginScene {
    // ... 现有字段 ...
    pub error_message: Option<String>,
    pub error_timer: f32,
}

// 在 draw() 中
if let Some(err) = &self.error_message {
    let text = Text::new(err);
    let params = DrawParam::default()
        .dest([dialog_x + 50.0, dialog_y + 135.0])
        .color(Color::from_rgb(255, 50, 50)); // 红色
    canvas.draw(&text, params);
}
```

#### 任务 4.3: 连接状态显示
```rust
// 登录中...
if self.connecting {
    let text = Text::new("正在登录...");
    canvas.draw(&text, params);
}
```

---

### 优先级 2: 背景动画 (30分钟-1小时)

#### 任务 6.1: 添加动画状态
```rust
// 在 LoginScene 中
pub struct LoginScene {
    // ... 现有字段 ...
    
    // 背景动画
    pub background_frame: usize,
    pub background_timer: f32,
    pub background_animating: bool,
}
```

#### 任务 6.2: 更新动画帧
```rust
// 在 update() 中
fn update(&mut self, delta_time: f32) {
    // ... 现有代码 ...
    
    // 背景动画 (100ms 每帧)
    if self.background_animating {
        self.background_timer += delta_time;
        if self.background_timer >= 0.1 {
            self.background_frame = (self.background_frame + 1) % 19; // 0-18
            self.background_timer = 0.0;
        }
    }
}
```

#### 任务 6.3: 绘制动画帧
```rust
// 在 draw() 中，替换现有的背景绘制
let frame_index = if self.background_animating {
    self.background_frame  // 使用当前动画帧
} else {
    0  // 静态使用第0帧
};

let _ = lib.draw_to_canvas(ctx, canvas, frame_index, 0.0, 0.0, false);
```

---

### 优先级 3: 新建账号对话框 (可选)

#### 相关文件
```
ClientRust/src/scenes/login_scene/
├── new_account_dialog.rs      ← 对话框实现
└── change_password_dialog.rs  ← 对话框实现
```

#### 参考代码位置
```
C# 原版:
Client/MirScenes/LoginScene.cs
- NewAccountDialog (Line 560-780)
- ChangePasswordDialog (Line 800-1020)
```

---

## 快速命令 ⚡

### 编译测试
```powershell
cd d:\Users\gxh\Documents\GitHub\Crystal\ClientRust
cargo check --bin mir2_client
```

### 运行程序
```powershell
cargo run --bin mir2_client
```

### 查看资源
```powershell
cargo run --bin lib_inspector ChrSel
cargo run --bin lib_inspector Title
```

### 清理编译
```powershell
cargo clean
```

---

## 重要提醒 📌

### 代码规范
1. **一次只改一个功能** - 避免大面积修改
2. **频繁测试** - 每个功能实现后立即运行测试
3. **参考 C# 原版** - 遇到问题看原版代码
4. **保持提交** - 完成一个功能就 commit

### 常见陷阱
1. ❌ 忘记导入 `Drawable` trait → `text.dimensions(ctx)` 报错
2. ❌ 使用 `unwrap_or` 在 Rect 上 → 应该用 `.w` 和 `.h`
3. ❌ 忘记更新 Scene trait → 需要在 mod.rs 中添加方法
4. ❌ 动画帧索引越界 → ChrSel 有 19 帧 (0-18)

### 调试技巧
```rust
// 打印调试信息
println!("当前帧: {}", self.background_frame);
tracing::debug!("登录尝试: {}", username);

// 检查图像是否加载
if let Some(lib) = get_library(LibraryName::ChrSel) {
    println!("ChrSel 库已加载");
}
```

---

## 资源索引快查 🎨

### ChrSel.lib - 登录背景
```
0-18: 背景动画帧 (19帧循环)
```

### Title.lib - UI元素
```
30: 标题 "登录"
31: "账号ID" 标签
32: "密码" 标签

320: OK按钮 (普通)
321: OK按钮 (悬停)
322: OK按钮 (按下)

323-325: 新建账号按钮
326-328: 修改密码按钮
329-331: 关闭按钮
```

### Prguse.lib - 对话框
```
1084: 登录对话框背景 (328x220)
```

---

## 今日成就 🏆

### 代码统计
- 新增方法: ~15个
- 新增字段: ~10个
- 修改文件: 5个
- 编译警告: 589个（可忽略）
- 编译错误: 0个 ✅

### 功能完成度
- LoginScene: ~60% ✅
- 文本输入: 100% ✅
- 按钮交互: 100% ✅
- 登录逻辑: 30% ⏳
- 背景动画: 0% ⏳

### 项目总进度
```
[████████░░░░░░░░░░░░] 40%

已完成:
✅ ggez 迁移
✅ MLibrary 渲染
✅ LoginScene UI
✅ 文本输入系统

进行中:
⏳ 登录逻辑
⏳ 背景动画

待开发:
□ SelectScene
□ GameScene
□ 网络系统完善
```

---

## 参考文档 📚

### 项目文档
- `NEXT_STEPS.md` - 开发计划（已更新）
- `TODAY_PROGRESS.md` - 今日进度报告
- `TEST_GUIDE.md` - 测试指南
- `GGEZ_MIGRATION_COMPLETED.md` - ggez 迁移完成报告

### C# 原版参考
```
Client/MirScenes/LoginScene.cs
- Line 1-100: 基础结构
- Line 200-300: 输入处理
- Line 400-500: 绘制逻辑
- Line 560+: 内嵌对话框
```

### Rust API
- ggez 文档: https://docs.rs/ggez/
- winit 文档: https://docs.rs/winit/

---

## 明天的目标 🎯

### 必须完成
- [ ] 登录逻辑实现（发送网络请求）
- [ ] 背景动画（19帧循环）

### 希望完成
- [ ] 错误消息显示
- [ ] 连接状态显示

### 如果有时间
- [ ] 新建账号对话框UI
- [ ] 优化代码，减少警告

---

## 加油！💪

你已经完成了最难的部分（文本输入和交互）！

明天继续加油，很快就能看到完整的登录流程了！

**记住:** 
- 一步一步来
- 频繁测试
- 参考原版
- 保持耐心

---

**日期:** 2025年10月6日  
**下次开发:** 2025年10月7日  
**预计耗时:** 2-3小时

---

祝明天开发顺利！🚀✨
