# Crystal MIR2 Rust客户端 - 快速启动指南

## 📋 前置条件

### 必需
- ✅ Rust 1.70+ (cargo 1.70+)
- ✅ Windows 10/11 (当前测试平台)
- ✅ 约2GB磁盘空间(包含依赖)

### 可选
- Crystal MIR2服务器(用于测试网络功能)
- 游戏资源文件(.lib图像库和音频文件)

---

## 🚀 快速启动

### 1. 克隆仓库

```powershell
cd D:\Users\gxh\Documents\GitHub
git clone <repository-url> Crystal
cd Crystal\ClientRust
```

### 2. 首次构建

```powershell
# Debug版本(开发用,带调试符号)
cargo build

# Release版本(优化,用于测试性能)
cargo build --release
```

**预计时间:**
- 首次构建: ~5分钟(下载依赖+编译)
- 后续增量编译: ~30秒

### 3. 运行客户端

```powershell
# 运行Debug版本
cargo run

# 或直接运行可执行文件
.\target\debug\mir2_client.exe

# 运行Release版本
cargo run --release
# 或
.\target\release\mir2_client.exe
```

---

## ⚙️ 配置文件

### 创建配置文件

在 `ClientRust/` 目录下创建 `settings.json`:

```json
{
  "network": {
    "server_ip": "127.0.0.1",
    "server_port": 7000,
    "connect_timeout_secs": 10,
    "read_timeout_secs": 30,
    "auto_reconnect": true
  },
  "graphics": {
    "fullscreen": false,
    "resolution_width": 1024,
    "resolution_height": 768,
    "vsync": true,
    "max_fps": 60
  },
  "sound": {
    "master_volume": 1.0,
    "music_volume": 0.5,
    "effect_volume": 0.8,
    "muted": false
  },
  "paths": {
    "data_path": "Data/",
    "config_path": "Config/",
    "save_path": "Save/"
  }
}
```

### 配置说明

#### network (网络配置)
- `server_ip`: 服务器IP地址
- `server_port`: 服务器端口(默认7000)
- `connect_timeout_secs`: 连接超时(秒)
- `read_timeout_secs`: 读取超时(秒)
- `auto_reconnect`: 断线自动重连

#### graphics (图形配置)
- `fullscreen`: 全屏模式
- `resolution_width/height`: 窗口分辨率
- `vsync`: 垂直同步(防止画面撕裂)
- `max_fps`: 最大帧率限制

#### sound (音频配置)
- `master_volume`: 主音量 (0.0-1.0)
- `music_volume`: 音乐音量 (0.0-1.0)
- `effect_volume`: 音效音量 (0.0-1.0)
- `muted`: 静音开关

#### paths (路径配置)
- `data_path`: 游戏资源路径(.lib文件等)
- `config_path`: 配置文件路径
- `save_path`: 存档路径

---

## 📁 目录结构

### 运行时需要的目录

```
ClientRust/
├── target/
│   └── debug/
│       └── mir2_client.exe    ← 可执行文件
├── settings.json              ← 配置文件
├── Data/                      ← 游戏资源(可选)
│   ├── Prguse.lib
│   ├── ChrSel.lib
│   ├── Items.lib
│   ├── Sounds/
│   │   ├── click.wav
│   │   └── ...
│   └── Music/
│       ├── theme01.ogg
│       └── ...
├── Config/                    ← 配置文件(自动创建)
└── Save/                      ← 存档文件(自动创建)
```

### 资源文件(可选)

如果要测试纹理加载和音频播放,需要准备:

**纹理库(.lib文件):**
- `Data/Prguse.lib` - 主UI元素
- `Data/ChrSel.lib` - 角色选择界面
- `Data/Title.lib` - 标题/Logo

**音频文件:**
- `Data/Sounds/*.wav` - 音效文件
- `Data/Music/*.ogg` - 背景音乐

---

## 🎮 使用说明

### 登录界面

启动后会看到登录界面(纯UI,暂无背景图):

1. **输入用户名**
2. **输入密码**
3. **点击登录按钮**

**当前状态:**
- ✅ UI显示正常
- ✅ 用户名/密码输入
- ✅ 连接状态显示
- ⏳ 实际登录数据包发送(待实现)

### 网络连接测试

如果有运行的Crystal MIR2服务器:

1. 配置 `settings.json` 中的 `server_ip` 和 `server_port`
2. 启动客户端
3. 观察连接状态:
   - "已连接" - 连接成功
   - "未连接" - 连接失败
   - "连接中..." - 正在尝试连接

**当前实现:**
- ✅ TCP连接建立
- ✅ ClientVersion自动发送
- ✅ 数据包接收和解析
- ⏳ 完整登录流程(待实现)

### 资源加载测试

当前会尝试加载:
- `Data/Prguse.lib` - 主UI纹理库
- `Data/Sounds/` - 音效目录
- `Data/Music/` - 音乐目录

如果文件不存在,会静默跳过(不影响程序运行)

---

## 🐛 常见问题

### 编译问题

**Q: 编译时提示找不到某个crate**

```powershell
# 清理并重新构建
cargo clean
cargo build
```

**Q: 链接错误(linker error)**

确保安装了Visual Studio Build Tools或完整的Visual Studio

**Q: 编译速度慢**

```powershell
# 使用更多CPU核心
$env:CARGO_BUILD_JOBS=8
cargo build
```

### 运行问题

**Q: 窗口一闪而过**

```powershell
# 在命令行中运行查看错误信息
cargo run

# 或者
.\target\debug\mir2_client.exe
# 窗口会保持打开显示错误
```

**Q: 连接服务器失败**

1. 检查服务器是否运行
2. 检查 `settings.json` 中的IP和端口
3. 检查防火墙设置

**Q: 没有声音**

1. 检查音频设备是否正常
2. 检查 `settings.json` 中 `muted` 是否为 `false`
3. 检查音量设置 (`master_volume`, `music_volume`)

**Q: 纹理加载失败**

1. 确认 `Data/` 目录存在
2. 确认 `.lib` 文件存在且格式正确
3. 查看控制台是否有错误信息

---

## 📊 性能监控

### FPS显示

窗口右上角显示实时FPS

**正常范围:**
- Debug版本: 55-60 FPS
- Release版本: 稳定60 FPS (vsync限制)

**如果FPS过低:**
- 检查GPU驱动是否最新
- 尝试关闭其他占用GPU的程序
- 使用Release版本

### 日志输出

```powershell
# 启用详细日志
$env:RUST_LOG="debug"
cargo run

# 只显示错误
$env:RUST_LOG="error"
cargo run
```

### 内存使用

使用任务管理器查看 `mir2_client.exe` 内存占用

**正常范围:**
- 启动时: ~50MB
- 加载纹理后: ~200MB
- 游戏运行中: ~300-500MB

---

## 🛠️ 开发模式

### 热重载

```powershell
# 安装cargo-watch
cargo install cargo-watch

# 监视文件变化并自动重新编译运行
cargo watch -x run
```

### 调试

**VS Code配置(.vscode/launch.json):**

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug mir2_client",
      "cargo": {
        "args": ["build", "--bin=mir2_client"]
      },
      "args": [],
      "cwd": "${workspaceFolder}/ClientRust"
    }
  ]
}
```

需要安装VS Code扩展:
- rust-analyzer
- CodeLLDB

### 代码检查

```powershell
# 快速检查(不生成可执行文件)
cargo check

# 代码格式化
cargo fmt

# Lint检查
cargo clippy

# 运行测试
cargo test
```

---

## 📝 开发任务

### 当前可做的事情

1. **测试UI**
   - 输入用户名密码
   - 观察连接状态
   - 监控FPS

2. **测试网络**
   - 连接到服务器
   - 观察ClientVersion发送
   - 查看数据包接收日志

3. **测试资源加载**
   - 准备.lib文件
   - 观察加载过程
   - 检查内存使用

### 下一步开发

参见 `docs/p0-complete-report.md` 中的P1阶段计划

---

## 📚 相关文档

- `docs/p0-2-network-integration-report.md` - 网络系统详解
- `docs/p0-3-texture-loading-report.md` - 纹理加载详解
- `docs/p0-complete-report.md` - P0阶段总结
- `README.md` - 项目总览

---

## 🆘 获取帮助

### 查看日志

```powershell
# 启用所有日志
$env:RUST_LOG="trace"
cargo run 2>&1 | Tee-Object -FilePath log.txt
```

### 报告问题

提供以下信息:
1. 操作系统版本
2. Rust版本 (`rustc --version`)
3. 完整错误信息
4. 复现步骤

---

## 🎉 享受游戏!

虽然目前只是框架,但所有核心系统都已就绪。
接下来将逐步添加游戏逻辑,让这个客户端真正可玩!

**Have Fun! 🚀**
