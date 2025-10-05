# Forms 模块使用指南

> **快速参考**: Forms是客户端的UI层，包含启动器、游戏主窗口和配置窗口

---

## 📚 目录

- [LauncherWindow - 启动器](#launcherwindow)
- [MainWindow - 游戏窗口](#mainwindow)
- [ConfigWindow - 配置窗口](#configwindow)
- [常见用法](#常见用法)
- [最佳实践](#最佳实践)

---

## LauncherWindow

### 基本用法

```rust
use mir2_client::forms::LauncherWindow;
use mir2_client::settings::ClientSettings;
use std::sync::Arc;

// 创建窗口
let event_loop = winit::event_loop::EventLoop::new()?;
let window = winit::window::WindowBuilder::new()
    .with_title("Legend of Mir 2 - Launcher")
    .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
    .build(&event_loop)?;

let settings = ClientSettings::load()?;
let mut launcher = LauncherWindow::new(Arc::new(window), settings);

// 启动补丁流程
launcher.start()?;
```

### 监控下载进度

```rust
// 检查是否完成
if launcher.completed {
    println!("Patching completed!");
}

// 获取进度
let progress = launcher.progress_percent();
println!("Download progress: {}%", progress);

// 获取详细信息
println!("Files: {}/{}", launcher.current_count, launcher.file_count);
println!("Bytes: {}/{}", launcher.completed_bytes, launcher.total_bytes);
```

### 事件循环集成

```rust
event_loop.run(move |event, elwt| {
    match event {
        Event::WindowEvent { event, .. } => {
            if launcher.handle_event(&event) {
                elwt.exit(); // 窗口关闭
            }
        }
        Event::AboutToWait => {
            // 渲染UI
            launcher.render();
            window.request_redraw();
        }
        _ => {}
    }
})?;
```

### 错误处理

```rust
use mir2_client::forms::save_error;

// 记录错误到文件
if let Err(e) = launcher.start() {
    save_error(&format!("Failed to start launcher: {}", e))?;
    eprintln!("Error: {}", e);
}
```

---

## MainWindow

### 基本用法

```rust
use mir2_client::forms::MainWindow;

// 创建主窗口
let event_loop = winit::event_loop::EventLoop::new()?;
let window = winit::window::WindowBuilder::new()
    .with_title("Legend of Mir 2")
    .with_inner_size(winit::dpi::LogicalSize::new(1024, 768))
    .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
    .build(&event_loop)?;

let settings = ClientSettings::load()?;
let mut game = MainWindow::new(Arc::new(window), settings);

// 初始化游戏
game.initialize()?;
```

### 游戏循环

```rust
use std::time::Instant;

let mut last_update = Instant::now();

event_loop.run(move |event, elwt| {
    match event {
        Event::WindowEvent { event, .. } => {
            if game.handle_event(&event) {
                elwt.exit();
            }
        }
        Event::AboutToWait => {
            // 计算delta时间
            let now = Instant::now();
            let delta = now.duration_since(last_update);
            last_update = now;
            
            // 更新游戏逻辑
            game.update(delta.as_secs_f32());
            
            // 渲染
            game.render();
            
            window.request_redraw();
        }
        _ => {}
    }
})?;
```

### FPS/Ping 显示

```rust
// 获取性能指标
let fps = game.get_fps();
let dps = game.get_dps();
let ping = game.get_ping();

println!("FPS: {} | DPS: {} | Ping: {}ms", fps, dps, ping);

// 切换FPS显示
game.toggle_fps();
```

### 事件处理

```rust
use winit::event::{WindowEvent, MouseButton, ElementState};

match event {
    WindowEvent::MouseInput { state, button, .. } => {
        if state == ElementState::Pressed && button == MouseButton::Left {
            // 鼠标左键点击
        }
    }
    WindowEvent::KeyboardInput { event, .. } => {
        if event.state == ElementState::Pressed {
            // 键盘按下
        }
    }
    _ => {}
}
```

---

## ConfigWindow

### 基本用法

```rust
use mir2_client::forms::ConfigWindow;

// 创建配置窗口
let event_loop = winit::event_loop::EventLoop::new()?;
let window = winit::window::WindowBuilder::new()
    .with_title("Settings")
    .with_inner_size(winit::dpi::LogicalSize::new(600, 400))
    .build(&event_loop)?;

let settings = ClientSettings::load()?;
let mut config = ConfigWindow::new(Arc::new(window), settings);
```

### 修改设置

```rust
// 设置分辨率
let resolutions = config.get_resolutions();
println!("Available resolutions:");
for (i, res) in resolutions.iter().enumerate() {
    println!("  {}: {}x{}", i, res.width, res.height);
}
config.set_resolution(2); // 选择第3个

// 全屏模式
config.set_fullscreen(true);

// 音量设置
config.set_sound_volume(75); // 0-100
config.set_music_volume(50);

// FPS限制
config.set_fps_cap(true);
```

### 保存设置

```rust
// 检查是否修改
if config.is_dirty() {
    println!("Settings have been changed");
    
    // 保存
    config.save()?;
    println!("Settings saved!");
}

// 获取当前设置
let current = config.settings();
println!("Current resolution: {}x{}", current.width, current.height);
println!("Fullscreen: {}", current.fullscreen);
println!("Sound: {}%, Music: {}%", current.volume, current.music_volume);
```

### 重置设置

```rust
// 恢复默认设置
config.reset_to_defaults();
println!("Settings reset to defaults");

// 保存默认设置
config.save()?;
```

---

## 常见用法

### 启动流程

```rust
use mir2_client::{
    forms::{LauncherWindow, MainWindow},
    settings::ClientSettings,
};
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    // 1. 显示启动器，检查更新
    let launcher_result = run_launcher()?;
    
    if !launcher_result {
        return Ok(()); // 用户取消
    }
    
    // 2. 启动游戏
    run_game()?;
    
    Ok(())
}

fn run_launcher() -> anyhow::Result<bool> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    let window = create_launcher_window(&event_loop)?;
    let settings = ClientSettings::load()?;
    
    let mut launcher = LauncherWindow::new(Arc::new(window), settings);
    launcher.start()?;
    
    // 等待完成
    while !launcher.completed {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    
    Ok(!launcher.error_found)
}

fn run_game() -> anyhow::Result<()> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    let window = create_game_window(&event_loop)?;
    let settings = ClientSettings::load()?;
    
    let mut game = MainWindow::new(Arc::new(window), settings);
    game.initialize()?;
    
    // 游戏循环（见上面的例子）
    
    Ok(())
}
```

### 窗口配置

```rust
fn create_launcher_window(
    event_loop: &winit::event_loop::EventLoop<()>
) -> anyhow::Result<winit::window::Window> {
    use winit::window::WindowBuilder;
    use winit::dpi::LogicalSize;
    
    let window = WindowBuilder::new()
        .with_title("Legend of Mir 2 - Launcher")
        .with_inner_size(LogicalSize::new(800, 600))
        .with_resizable(false)
        .with_decorations(true)
        .build(event_loop)?;
    
    Ok(window)
}

fn create_game_window(
    event_loop: &winit::event_loop::EventLoop<()>
) -> anyhow::Result<winit::window::Window> {
    use winit::window::{WindowBuilder, Fullscreen};
    
    let settings = ClientSettings::load()?;
    
    let mut builder = WindowBuilder::new()
        .with_title("Legend of Mir 2")
        .with_inner_size(winit::dpi::LogicalSize::new(
            settings.width,
            settings.height
        ));
    
    if settings.fullscreen {
        builder = builder.with_fullscreen(Some(Fullscreen::Borderless(None)));
    }
    
    Ok(builder.build(event_loop)?)
}
```

---

## 最佳实践

### 1. 错误处理

```rust
// ✅ 好的做法
if let Err(e) = launcher.start() {
    tracing::error!("Launcher error: {}", e);
    save_error(&format!("Launcher: {}", e))?;
    // 显示用户友好的错误消息
}

// ❌ 不好的做法
launcher.start().unwrap(); // 可能崩溃
```

### 2. 资源清理

```rust
// 游戏结束时清理
impl Drop for MainWindow {
    fn drop(&mut self) {
        self.shutdown();
        tracing::info!("Game window cleaned up");
    }
}

// 或手动调用
game.shutdown();
```

### 3. 日志记录

```rust
use tracing::{info, warn, error};

// 启动
info!("Launching game");

// 警告
if launcher.clean_files {
    warn!("Cleaning old files");
}

// 错误
if launcher.error_found {
    error!("Errors detected during patching");
}
```

### 4. 进度反馈

```rust
// 使用 indicatif 显示进度条
use indicatif::{ProgressBar, ProgressStyle};

let pb = ProgressBar::new(launcher.total_bytes);
pb.set_style(
    ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
);

loop {
    pb.set_position(launcher.completed_bytes);
    
    if launcher.completed {
        pb.finish_with_message("Download complete!");
        break;
    }
    
    std::thread::sleep(std::time::Duration::from_millis(100));
}
```

### 5. 配置持久化

```rust
// 保存前验证
if config.is_dirty() {
    // 验证设置
    let settings = config.settings();
    
    if settings.volume > 100 || settings.music_volume > 100 {
        eprintln!("Invalid volume settings");
        return Err(anyhow::anyhow!("Invalid settings"));
    }
    
    // 保存
    config.save()?;
    info!("Settings saved successfully");
}
```

### 6. 异步下载（计划）

```rust
// 未来的实现
use tokio::task::JoinSet;

async fn download_files(&mut self) -> Result<()> {
    let mut tasks = JoinSet::new();
    
    for file_info in &self.download_queue {
        let url = format!("{}/{}", self.settings.patch_url, file_info.file_name);
        let path = PathBuf::from(&file_info.file_name);
        
        tasks.spawn(async move {
            download_file(&url, &path).await
        });
    }
    
    while let Some(result) = tasks.join_next().await {
        result??;
        self.current_count += 1;
    }
    
    Ok(())
}
```

---

## 🔧 故障排查

### 问题: 启动器卡住不动

**原因**: 网络连接失败或patch server不可达

**解决**:
```rust
// 设置超时
use tokio::time::timeout;
use std::time::Duration;

match timeout(Duration::from_secs(30), launcher.start()).await {
    Ok(Ok(_)) => println!("Launcher completed"),
    Ok(Err(e)) => eprintln!("Launcher error: {}", e),
    Err(_) => eprintln!("Launcher timeout after 30s"),
}
```

### 问题: 游戏FPS过低

**原因**: 渲染循环阻塞

**解决**:
```rust
// 限制更新频率
const TARGET_FPS: u32 = 60;
const FRAME_TIME: Duration = Duration::from_micros(1_000_000 / TARGET_FPS as u64);

let mut last_frame = Instant::now();

loop {
    let now = Instant::now();
    let elapsed = now - last_frame;
    
    if elapsed < FRAME_TIME {
        std::thread::sleep(FRAME_TIME - elapsed);
        continue;
    }
    
    last_frame = now;
    game.update(elapsed.as_secs_f32());
    game.render();
}
```

### 问题: 配置不保存

**原因**: 文件权限或路径错误

**解决**:
```rust
// 检查权限
if let Err(e) = config.save() {
    if e.kind() == std::io::ErrorKind::PermissionDenied {
        eprintln!("Permission denied. Try running as administrator.");
    } else {
        eprintln!("Failed to save: {}", e);
    }
}
```

---

## 📖 相关文档

- [Forms移植完成报告.md](./Forms移植完成报告.md) - 详细的移植分析
- [Settings使用指南.md](./Settings使用指南.md) - 配置系统说明
- [Resources使用指南.md](./Resources使用指南.md) - 资源加载说明

---

## 🎯 下一步

Forms模块的**业务逻辑**已完成，但**UI渲染**尚未实现。下一步工作：

1. **LauncherWindow**:
   - HTTP下载实现 (reqwest)
   - 进度条渲染 (wgpu + Resources)
   - 并发下载 (tokio)

2. **MainWindow**:
   - wgpu渲染管线
   - Scene系统集成
   - Input处理

3. **ConfigWindow**:
   - egui UI实现
   - 滑块和下拉菜单
   - 实时预览

参考 [Forms移植完成报告.md](./Forms移植完成报告.md) 了解详细的实现计划。
