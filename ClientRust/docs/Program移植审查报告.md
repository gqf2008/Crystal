# Program.cs 移植审查报告

## 审查时间
2025年10月5日

## 文件对比

### C# 原文件
- **路径**: `Client/Program.cs`
- **行数**: 185 行
- **主要功能**: 应用程序入口点

### Rust 移植文件
- **主文件**: `ClientRust/src/main.rs` (156 行)
- **辅助模块**: `ClientRust/src/program.rs` (97 行)

---

## 功能对比分析

### ✅ 已正确移植的功能

#### 1. 命令行参数解析 ✅
**C#**:
```csharp
if (args.Length > 0)
{
    foreach (var arg in args)
    {
        if (arg.ToLower() == "-tc") Settings.UseTestConfig = true;
    }
}
```

**Rust**:
```rust
let cli_flag = args
    .iter()
    .skip(1)
    .any(|arg| matches!(arg.to_ascii_lowercase().as_str(), "-tc" | "--test-config"));
```
✅ **评估**: 正确，还支持 `--test-config` 长选项，更符合 Unix 习惯

#### 2. 环境变量支持 ✅ (Rust 增强)
**Rust 新增**:
```rust
let env_flag = std::env::var("MIR2_CLIENT_USE_TEST_CONFIG")
    .ok()
    .map(|value| {
        let normalized = value.trim().to_ascii_lowercase();
        matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
    })
    .unwrap_or(false);
```
✅ **评估**: 比 C# 更灵活，支持环境变量配置

#### 3. Debug 模式处理 ✅
**C#**:
```csharp
#if DEBUG
    Settings.UseTestConfig = true;
#endif
```

**Rust**:
```rust
// 通过 cfg!(debug_assertions) 或环境变量控制
let use_test_config = cli_flag || env_flag;
```
✅ **评估**: 使用运行时配置，更灵活

#### 4. 设置加载 ✅
**C#**:
```csharp
Settings.Load();
```

**Rust**:
```rust
let settings = ClientSettings::load(use_test_config, None)?;
```
✅ **评估**: 正确，使用 `Result` 处理错误更安全

#### 5. 窗口和事件循环 ✅
**C#**:
```csharp
Application.EnableVisualStyles();
Application.SetCompatibleTextRenderingDefault(false);
Application.Run(Form = new CMain());
```

**Rust**:
```rust
let event_loop = EventLoop::new()?;
let window = event_loop.create_window(
    winit::window::WindowAttributes::default()
        .with_title("Legend of Mir 2")
        .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0))
)?;
event_loop.run(move |event, elwt| { ... })?;
```
✅ **评估**: 正确，使用 `winit` 替代 WinForms

#### 6. 分辨率检查 ✅
**C#**:
```csharp
public static void CheckResolutionSetting()
{
    var parsedOK = DisplayResolutions.GetDisplayResolutions();
    if (!parsedOK) { /* 错误处理 */ }
    
    if (!DisplayResolutions.IsSupported(Settings.Resolution))
    {
        MessageBox.Show(...);
        Settings.Resolution = (int)eSupportedResolution.w1024h768;
        Settings.Save();
    }
}
```

**Rust**:
```rust
// 在 resolution 模块中实现
// src/resolution/display_resolutions.rs
pub fn get_display_resolutions() -> Vec<SupportedResolution> { ... }
pub fn is_supported(resolution: SupportedResolution) -> bool { ... }
```
✅ **评估**: Resolution 模块已移植完成 (见之前的报告)

#### 7. 网络管理器初始化 ✅
**Rust**:
```rust
let settings_arc = Arc::new(RwLock::new(settings.clone()));
let network_manager = NetworkManager::new(settings_arc.clone(), event_tx, command_rx);

std::thread::spawn(move || {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(network_task(network_manager));
});
```
✅ **评估**: 异步网络处理，比 C# 更现代

#### 8. 图形系统初始化 ✅
**Rust**:
```rust
let dx_manager = pollster::block_on(graphics::DXManager::new(window_arc.clone()));
```
✅ **评估**: 使用 `wgpu` 替代 DirectX 9，跨平台

---

### ❌ 未移植/部分移植的功能

#### 1. AutoPatcher 更新逻辑 ❌ **缺失**
**C#**:
```csharp
private static bool UpdatePatcher()
{
    const string fromName = @".\AutoPatcher.gz", toName = @".\AutoPatcher.exe";
    if (!File.Exists(fromName)) return false;
    
    // 1. 检查并杀死旧的 AutoPatcher 进程
    Process[] processes = Process.GetProcessesByName("AutoPatcher");
    // ...
    
    // 2. 解压缩并替换
    File.Move(fromName, toName);
    Process.Start(toName, "Auto");
    
    return true;
}
```

**Rust**: 未实现
```rust
// TODO: 需要实现自动更新逻辑
// 1. 检查 AutoPatcher.gz
// 2. 解压缩到 AutoPatcher.exe
// 3. 启动更新程序
```

⚠️ **影响**: 自动更新功能缺失，需要手动更新客户端

**建议实现**:
```rust
fn update_patcher() -> Result<bool> {
    const FROM_NAME: &str = "./AutoPatcher.gz";
    const TO_NAME: &str = "./AutoPatcher.exe";
    
    if !std::path::Path::new(FROM_NAME).exists() {
        return Ok(false);
    }
    
    // 1. 查找并终止旧进程
    #[cfg(target_os = "windows")]
    {
        use sysinfo::{ProcessExt, SystemExt};
        let mut system = sysinfo::System::new_all();
        system.refresh_all();
        
        for (pid, process) in system.processes() {
            if process.name() == "AutoPatcher.exe" {
                // 终止进程
                process.kill();
            }
        }
        
        // 等待进程退出（最多3秒）
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
    
    // 2. 解压缩
    use flate2::read::GzDecoder;
    use std::fs::File;
    use std::io::copy;
    
    let input = File::open(FROM_NAME)?;
    let mut decoder = GzDecoder::new(input);
    let mut output = File::create(TO_NAME)?;
    copy(&mut decoder, &mut output)?;
    
    // 3. 启动更新程序
    #[cfg(target_os = "windows")]
    std::process::Command::new(TO_NAME)
        .arg("Auto")
        .spawn()?;
    
    Ok(true)
}
```

#### 2. 启动器界面 (Patcher Form) ⚠️ **部分移植**
**C#**:
```csharp
Launch = false;
if (Settings.P_Patcher)
    Application.Run(PForm = new AMain());  // 启动器窗口
else
    Launch = true;

if (Launch)
    Application.Run(Form = new CMain());    // 游戏主窗口
```

**Rust**: 只有主窗口
```rust
let window = event_loop.create_window(
    winit::window::WindowAttributes::default()
        .with_title("Legend of Mir 2")
        .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0))
)?;
```

⚠️ **影响**: 缺少独立的启动器界面（Launcher/Patcher）

**建议**:
- 实现 `LauncherWindow` (对应 C# 的 `AMain`)
- 根据 `Settings.P_Patcher` 决定显示哪个窗口
- 启动器负责版本检查、文件下载等

#### 3. 应用程序重启逻辑 ❌ **缺失**
**C#**:
```csharp
public static bool Restart;

// ...在 Main() 结尾
if (Restart)
{
    Application.Restart();
}
```

**Rust**: 未实现

**建议实现**:
```rust
// 在退出前检查是否需要重启
if should_restart {
    // 获取当前可执行文件路径
    let exe_path = std::env::current_exe()?;
    
    // 重新启动
    #[cfg(target_os = "windows")]
    std::process::Command::new(exe_path)
        .spawn()?;
    
    // 退出当前进程
    std::process::exit(0);
}
```

#### 4. 错误保存功能 ⚠️ **不完整**
**C#**:
```csharp
catch (Exception ex)
{
    CMain.SaveError(ex.ToString());
}
```

**Rust**: 只有日志
```rust
tracing_subscriber::fmt()
    .with_env_filter(...)
    .init();
```

**建议**:
```rust
fn save_error(error: &str) -> std::io::Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;
    
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("Error.txt")?;
    
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    writeln!(file, "\n=== {} ===", timestamp)?;
    writeln!(file, "{}", error)?;
    
    Ok(())
}

// 在 main() 中使用
fn main() -> Result<()> {
    if let Err(e) = run() {
        save_error(&format!("{:?}", e))?;
        return Err(e);
    }
    Ok(())
}
```

#### 5. RuntimePolicyHelper (CLR 绑定) ✅ **不需要**
**C#**:
```csharp
public static class RuntimePolicyHelper
{
    // 绑定 .NET 2.0 运行时
    void BindAsLegacyV2Runtime();
}
```

✅ **评估**: Rust 不需要此功能，C# 特有的兼容性代码

---

## 架构差异

### C# 架构
```
Program.cs
├─ Main()
│  ├─ UpdatePatcher() → 自动更新
│  ├─ Settings.Load()
│  ├─ CheckResolutionSetting()
│  └─ Application.Run(AMain/CMain) → WinForms
└─ RuntimePolicyHelper → CLR 绑定
```

### Rust 架构
```
main.rs + program.rs
├─ main()
│  ├─ 命令行参数解析
│  ├─ ClientSettings::load()
│  ├─ NetworkManager::new() → 异步网络
│  ├─ DXManager::new() → wgpu 图形
│  └─ EventLoop::run() → winit 窗口
└─ program.rs (备用的 Runtime 抽象)
```

---

## 详细问题列表

### 🔴 关键缺失功能

| 功能 | C# | Rust | 优先级 | 建议 |
|------|----|----|--------|------|
| AutoPatcher 更新 | ✅ | ❌ | **高** | 使用 `flate2` + `sysinfo` 实现 |
| 启动器窗口 (AMain) | ✅ | ❌ | **高** | 创建 `LauncherWindow` |
| 应用重启 | ✅ | ❌ | 中 | 实现 `restart_application()` |
| 错误日志保存 | ✅ | ⚠️ | 中 | 添加 `save_error()` 到文件 |
| 分辨率检查弹窗 | ✅ | ⚠️ | 低 | 使用 native dialog 或终端提示 |

### 🟡 架构改进建议

#### 1. 统一入口点
**当前**: `main.rs` 和 `program.rs` 功能重复

**建议**: 
```rust
// main.rs - 简单的入口点
fn main() -> Result<()> {
    program::ClientRuntime::bootstrap()?
}

// program.rs - 完整的启动逻辑
impl ClientRuntime {
    pub fn bootstrap() -> Result<()> {
        // 1. 检查更新
        if self.update_patcher()? {
            return Ok(()); // 启动更新程序后退出
        }
        
        // 2. 加载设置
        self.load_settings()?;
        
        // 3. 检查分辨率
        self.check_resolution()?;
        
        // 4. 启动界面
        if self.settings.show_patcher {
            self.run_launcher()?;
        }
        
        self.run_game()?;
        
        // 5. 处理重启
        if self.should_restart {
            self.restart()?;
        }
        
        Ok(())
    }
}
```

#### 2. 窗口管理
**建议**: 实现窗口状态机
```rust
enum WindowState {
    Launcher,  // 启动器/更新器
    Game,      // 游戏主窗口
}

impl ClientRuntime {
    fn run_window_loop(&mut self) -> Result<()> {
        loop {
            match self.window_state {
                WindowState::Launcher => {
                    if self.run_launcher()? {
                        self.window_state = WindowState::Game;
                    } else {
                        break; // 用户取消
                    }
                }
                WindowState::Game => {
                    self.run_game()?;
                    break;
                }
            }
        }
        Ok(())
    }
}
```

---

## 测试覆盖

### C# 功能测试
- ✅ 命令行参数 `-tc`
- ✅ AutoPatcher 更新流程
- ✅ 分辨率检查
- ✅ 启动器 → 游戏窗口切换
- ✅ 应用重启

### Rust 测试覆盖
- ✅ 命令行参数 `-tc` / `--test-config`
- ✅ 环境变量 `MIR2_CLIENT_USE_TEST_CONFIG`
- ✅ 设置加载
- ✅ 窗口创建
- ✅ 事件循环
- ❌ AutoPatcher 更新 (未实现)
- ❌ 启动器窗口 (未实现)
- ❌ 应用重启 (未实现)

---

## 依赖项对比

### C# 依赖
```csharp
using System.Diagnostics;      // Process 管理
using System.Runtime.InteropServices;  // COM 互操作
using Launcher;                // 启动器窗口
using Client.Resolution;       // 分辨率检查
```

### Rust 依赖
```toml
[dependencies]
anyhow = "1"           # 错误处理
winit = "0.30"         # 窗口管理
wgpu = "27.0"          # 图形 API
tokio = "1"            # 异步运行时
parking_lot = "0.12"   # 高效锁
tracing = "0.1"        # 日志

# 需要添加的依赖
flate2 = "1"           # GZip 解压 (AutoPatcher)
sysinfo = "0.30"       # 进程管理 (AutoPatcher)
native-dialog = "0.7"  # 原生对话框 (分辨率错误)
```

---

## 总体评估

### ✅ 优点
1. **核心功能正确**: 设置加载、窗口创建、事件循环
2. **架构改进**: 使用 `winit` + `wgpu` 跨平台
3. **错误处理**: 使用 `Result` 类型更安全
4. **网络异步**: 使用 `tokio` 更高效
5. **配置灵活**: 支持环境变量

### ⚠️ 需要改进
1. **AutoPatcher 缺失**: 自动更新功能未实现
2. **启动器窗口缺失**: 没有独立的 Launcher/Patcher UI
3. **重启逻辑缺失**: 无法重启应用
4. **错误日志不完整**: 只有 tracing，没有错误文件
5. **代码组织**: `main.rs` 和 `program.rs` 功能重复

### 📊 完成度评分

| 模块 | 完成度 | 说明 |
|------|--------|------|
| 命令行参数 | 100% | ✅ 完全实现 + 增强 |
| 设置加载 | 100% | ✅ 完全实现 |
| 窗口系统 | 80% | ✅ 主窗口完成，❌ 启动器缺失 |
| 分辨率检查 | 100% | ✅ 模块已完成 |
| 网络系统 | 100% | ✅ 异步网络管理器 |
| 图形系统 | 90% | ✅ wgpu 初始化完成 |
| 自动更新 | 0% | ❌ 完全缺失 |
| 应用重启 | 0% | ❌ 完全缺失 |
| 错误处理 | 60% | ⚠️ 有日志，缺文件保存 |
| **总体** | **75%** | 核心功能完成，辅助功能缺失 |

---

## 行动计划

### 第一阶段 - 补齐核心功能 (1-2 周)
1. ✅ ~~Resolution 模块~~ (已完成)
2. ✅ ~~Resources 模块~~ (已完成)
3. 🔲 实现 AutoPatcher 更新逻辑
4. 🔲 添加错误日志保存
5. 🔲 实现应用重启

### 第二阶段 - 启动器实现 (2-3 周)
1. 🔲 创建 `LauncherWindow` (对应 `AMain`)
2. 🔲 实现进度条和状态显示
3. 🔲 添加服务器选择
4. 🔲 集成版本检查

### 第三阶段 - UI 完善 (长期)
1. 🔲 实现游戏主窗口 (对应 `CMain`)
2. 🔲 集成 MirControls
3. 🔲 实现 MirScenes
4. 🔲 完整的渲染循环

---

## 结论

### 移植质量: **良好 (B+)**

**优点**:
- ✅ 核心启动流程正确
- ✅ 使用现代 Rust 最佳实践
- ✅ 跨平台架构 (winit + wgpu)
- ✅ 异步网络处理

**缺点**:
- ❌ 自动更新功能缺失 (关键)
- ❌ 启动器界面缺失
- ⚠️ 错误处理不完整

### 推荐行动
1. **立即**: 实现 AutoPatcher 更新逻辑
2. **短期**: 添加错误日志和重启功能
3. **中期**: 实现启动器窗口
4. **长期**: 完善游戏主循环

---

**审查人**: GitHub Copilot  
**日期**: 2025年10月5日  
**版本**: Rust 客户端 v0.1.0
