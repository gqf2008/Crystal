# 🛠️ Crystal MIR2 Rust客户端开发指南

**面向:** 想要参与开发或理解代码的开发者

---

## 🏗️ 项目结构

### 源码组织

```
ClientRust/
├── src/
│   ├── main.rs              # 入口点,启动网络任务
│   ├── app.rs               # MirClientApp主应用逻辑
│   ├── settings.rs          # 配置管理
│   ├── version.rs           # 版本信息和哈希
│   │
│   ├── graphics/            # 图形渲染模块
│   │   ├── mod.rs
│   │   └── texture_loader.rs    # .lib纹理加载
│   │
│   ├── sounds/              # 音频模块
│   │   ├── mod.rs
│   │   └── sound_loader.rs      # 音效和音乐播放
│   │
│   ├── network/             # 网络通信模块
│   │   ├── mod.rs
│   │   ├── network_stack.rs     # TCP连接层
│   │   ├── network_manager.rs   # 协调层
│   │   ├── game_client.rs       # 游戏客户端逻辑
│   │   └── protocol.rs          # 数据包协议
│   │
│   └── scenes/              # 游戏场景模块
│       ├── mod.rs
│       ├── scene_trait.rs       # Scene trait定义
│       ├── login_scene.rs       # 登录界面
│       ├── select_scene.rs      # 角色选择
│       └── game_scene.rs        # 游戏主场景
│
├── docs/                    # 文档
│   ├── p0-2-network-integration-report.md
│   ├── p0-3-texture-loading-report.md
│   └── p0-complete-report.md
│
├── Cargo.toml              # 依赖配置
├── QUICKSTART.md           # 快速启动指南
├── PROGRESS.md             # 开发进度追踪
└── DEVGUIDE.md             # 本文档
```

---

## 🎯 核心概念

### 1. 场景系统 (Scenes)

游戏使用**场景(Scene)**来组织不同的界面:

```rust
pub trait Scene {
    fn show(&mut self);         // 显示场景
    fn hide(&mut self);         // 隐藏场景
    fn update(&mut self, dt: f32);  // 更新逻辑(每帧调用)
    fn render(&mut self, ui: &mut egui::Ui);  // 渲染UI
}

pub enum SceneType {
    Login,   // 登录界面
    Select,  // 角色选择
    Game,    // 游戏主场景
}
```

**切换场景:**

```rust
// 在app.rs中
fn switch_scene(&mut self, scene_type: SceneType) {
    // 隐藏当前场景
    match self.current_scene {
        SceneType::Login => self.login_scene.hide(),
        // ...
    }
    
    // 显示新场景
    self.current_scene = scene_type;
    match scene_type {
        SceneType::Login => self.login_scene.show(),
        // ...
    }
}
```

### 2. 网络架构

```
┌─────────────────────────────────────────────────┐
│                  main.rs                        │
│  ┌──────────────┐         ┌──────────────┐    │
│  │ 网络线程      │         │ UI线程(主)    │    │
│  │ (Tokio)      │         │ (egui)       │    │
│  └──────────────┘         └──────────────┘    │
│         │                        ↑              │
│         │ event_tx               │ event_rx    │
│         └────────────────────────┘              │
└─────────────────────────────────────────────────┘

网络线程详细:
┌──────────────────────────────────────┐
│       network_task()                 │
│  ┌────────────────────────────────┐ │
│  │    NetworkManager              │ │
│  │  ┌──────────┐  ┌───────────┐  │ │
│  │  │NetworkStack│◄─►GameClient│  │ │
│  │  │   (TCP)  │  │  (逻辑)   │  │ │
│  │  └──────────┘  └───────────┘  │ │
│  │         ↓                       │ │
│  │    event_tx (发送GameEvent)    │ │
│  └────────────────────────────────┘ │
└──────────────────────────────────────┘
```

**关键组件:**

- **NetworkStack:** 管理TCP连接,发送/接收原始字节
- **GameClient:** 游戏逻辑,处理数据包,生成GameEvent
- **NetworkManager:** 协调NetworkStack和GameClient
- **event_tx/event_rx:** 跨线程通信的事件通道

### 3. 资源管理

**纹理加载流程:**

```rust
// 1. 加载库文件
texture_manager.load_library("Prguse", Path::new("Data/Prguse.lib"))?;

// 2. 获取纹理(自动缓存)
let (info, texture) = texture_manager.get_texture(
    ui.ctx(),       // egui上下文
    "Prguse",       // 库名
    123,            // 图像索引
)?;

// 3. 渲染
ui.image(texture.id(), [info.width as f32, info.height as f32]);
```

**音频播放流程:**

```rust
// 1. 初始化
let mut sound_manager = SoundManager::new()?;

// 2. 注册音频
sound_manager.register_sound(
    "click",
    PathBuf::from("Data/Sounds/click.wav"),
    SoundType::Effect,
    1.0,  // 音量
);

// 3. 播放
sound_manager.play_effect("click")?;
sound_manager.play_music("theme01")?;  // 循环
```

### 4. 数据包处理

**发送数据包:**

```rust
use mir2_shared::packets::client::Login;

let packet = Login {
    username: "test".to_string(),
    password: "123456".to_string(),
};

network_manager.send_packet(&packet)?;
```

**接收数据包:**

```rust
// 在GameClient中实现handler
impl GameClient {
    pub fn handle_login_success(&mut self, packet: LoginSuccess) {
        // 处理登录成功
        self.emit_event(GameEvent::LoginSuccess);
    }
}

// 数据包会通过protocol::dispatch_packet路由到对应handler
```

---

## 🔧 开发工作流

### 添加新场景

1. **创建场景文件** `src/scenes/my_scene.rs`:

```rust
use super::scene_trait::Scene;
use egui;

pub struct MyScene {
    visible: bool,
    // 场景状态
}

impl MyScene {
    pub fn new() -> Self {
        Self {
            visible: false,
        }
    }
}

impl Scene for MyScene {
    fn show(&mut self) {
        self.visible = true;
    }
    
    fn hide(&mut self) {
        self.visible = false;
    }
    
    fn update(&mut self, _dt: f32) {
        // 更新逻辑
    }
    
    fn render(&mut self, ui: &mut egui::Ui) {
        if !self.visible {
            return;
        }
        
        // 渲染UI
        ui.heading("My Scene");
    }
}
```

2. **在scenes/mod.rs中导出:**

```rust
pub mod my_scene;
pub use my_scene::MyScene;
```

3. **在SceneType中添加:**

```rust
pub enum SceneType {
    Login,
    Select,
    Game,
    MyScene,  // 新增
}
```

4. **在app.rs中集成:**

```rust
pub struct MirClientApp {
    // ...
    my_scene: Option<MyScene>,
}

impl MirClientApp {
    pub fn new(...) -> Self {
        // ...
        my_scene: None,
    }
    
    fn switch_scene(&mut self, scene_type: SceneType) {
        match scene_type {
            // ...
            SceneType::MyScene => {
                if self.my_scene.is_none() {
                    self.my_scene = Some(MyScene::new());
                }
                self.my_scene.as_mut().unwrap().show();
            }
        }
    }
}
```

### 添加新数据包

1. **在SharedRust中定义** (如果服务器也需要):

```rust
// SharedRust/src/packets/client/my_packet.rs
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyPacket {
    pub field1: String,
    pub field2: i32,
}
```

2. **在protocol.rs中添加opcode:**

```rust
pub enum PacketOpcode {
    // ...
    MyPacket = 0x99,
}
```

3. **在GameClient中添加handler:**

```rust
impl GameClient {
    pub fn handle_my_packet(&mut self, packet: MyPacket) {
        // 处理数据包
        println!("Received: {:?}", packet);
        
        // 发送事件到UI线程
        self.emit_event(GameEvent::MyEvent(packet.field1));
    }
}
```

4. **在protocol::dispatch_packet中路由:**

```rust
fn dispatch_packet(header: PacketHeader, data: &[u8], handler: &mut GameClient) {
    match header.opcode {
        // ...
        0x99 => {
            if let Ok(packet) = bincode::deserialize::<MyPacket>(data) {
                handler.handle_my_packet(packet);
            }
        }
    }
}
```

### 加载新资源库

1. **在app.rs的new()中加载:**

```rust
let _ = texture_manager.load_library(
    "Items",
    std::path::Path::new("Data/Items.lib")
);
```

2. **在需要的地方使用:**

```rust
let (info, texture) = self.texture_manager.get_texture(
    ui.ctx(),
    "Items",  // 库名
    item_id,  // 物品ID作为索引
)?;

ui.image(texture.id(), [info.width as f32, info.height as f32]);
```

---

## 🧪 测试

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_my_function() {
        let result = my_function();
        assert_eq!(result, expected);
    }
}
```

**运行测试:**

```powershell
cargo test
```

### 集成测试

创建 `tests/integration_test.rs`:

```rust
use mir2_client::graphics::TextureManager;

#[test]
fn test_texture_loading() {
    let mut tm = TextureManager::new();
    // 测试逻辑
}
```

### 性能测试

使用criterion:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_packet_parsing(c: &mut Criterion) {
    c.bench_function("parse login packet", |b| {
        b.iter(|| {
            // 性能测试代码
        });
    });
}

criterion_group!(benches, bench_packet_parsing);
criterion_main!(benches);
```

---

## 🐛 调试技巧

### 1. 日志输出

```rust
use tracing::{info, warn, error, debug, trace};

// 不同级别的日志
trace!("详细调试信息");
debug!("调试信息: {:?}", data);
info!("一般信息");
warn!("警告: {}", msg);
error!("错误: {}", err);
```

**设置日志级别:**

```powershell
$env:RUST_LOG="debug"
cargo run
```

### 2. 断点调试

VS Code + CodeLLDB:

```json
{
  "type": "lldb",
  "request": "launch",
  "name": "Debug",
  "cargo": {
    "args": ["build", "--bin=mir2_client"]
  }
}
```

### 3. 性能分析

```powershell
# 安装flamegraph
cargo install flamegraph

# 生成火焰图
cargo flamegraph
```

### 4. 内存检查

```powershell
# Windows: 使用性能监视器
perfmon

# 或在代码中手动记录
println!("Memory usage: {} MB", 
    std::process::id()
);
```

---

## 📝 代码风格

### Rust最佳实践

**命名约定:**
```rust
// 函数和变量: snake_case
fn load_texture() {}
let my_variable = 0;

// 类型和Trait: PascalCase
struct MyStruct {}
trait MyTrait {}

// 常量: SCREAMING_SNAKE_CASE
const MAX_SIZE: usize = 1024;
```

**错误处理:**
```rust
// 使用Result返回值
fn do_something() -> Result<(), String> {
    // 使用?传播错误
    let data = read_file()?;
    
    // 或显式处理
    match process_data(data) {
        Ok(result) => Ok(()),
        Err(e) => Err(format!("Failed: {}", e)),
    }
}
```

**生命周期:**
```rust
// 尽量避免复杂的生命周期
// 优先使用Arc/Rc或克隆

// 必要时使用生命周期标注
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}
```

### 项目约定

**模块组织:**
- 每个模块一个文件
- 使用mod.rs导出公共API
- 私有helper函数放在模块内部

**注释:**
```rust
/// 公共API的文档注释(三斜杠)
/// 
/// # Examples
/// ```
/// let result = my_function();
/// ```
pub fn my_function() -> i32 {
    // 实现细节的注释(双斜杠)
    42
}
```

**Commit消息:**
```
[模块] 简短描述

详细说明(可选)

- 变更1
- 变更2
```

例如:
```
[network] 添加自动重连机制

实现了NetworkManager的自动重连功能:
- 检测连接断开
- 指数退避重连
- 最大重试次数限制
```

---

## 🚀 性能优化

### 1. 避免频繁分配

```rust
// ❌ 差
fn process_data(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    for item in data {
        result.push(process(*item));
    }
    result
}

// ✅ 好
fn process_data(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());
    for item in data {
        result.push(process(*item));
    }
    result
}
```

### 2. 使用引用避免复制

```rust
// ❌ 差
fn process(data: Vec<u8>) {
    // data被移动,调用者无法继续使用
}

// ✅ 好
fn process(data: &[u8]) {
    // 借用data,调用者仍可使用
}
```

### 3. 缓存计算结果

```rust
struct TextureManager {
    textures: HashMap<TextureKey, TextureHandle>,  // 缓存
}

impl TextureManager {
    pub fn get_texture(&mut self, ...) -> TextureHandle {
        // 检查缓存
        if let Some(handle) = self.textures.get(&key) {
            return handle.clone();
        }
        
        // 加载并缓存
        let handle = load_texture(...);
        self.textures.insert(key, handle.clone());
        handle
    }
}
```

### 4. 使用合适的数据结构

- `Vec` - 连续存储,快速索引
- `HashMap` - 快速查找
- `BTreeMap` - 有序存储
- `HashSet` - 快速成员检查

---

## 🔒 线程安全

### Arc<RwLock<T>>使用

```rust
use std::sync::Arc;
use parking_lot::RwLock;

// 创建共享状态
let game_client = Arc::new(RwLock::new(GameClient::new()));

// 克隆Arc传递给其他线程
let gc_clone = game_client.clone();
std::thread::spawn(move || {
    // 读取
    let client = gc_clone.read();
    println!("{:?}", client.state);
    
    // 写入
    let mut client = gc_clone.write();
    client.update();
});
```

**注意事项:**
- 尽量缩短锁持有时间
- 避免嵌套锁(可能死锁)
- 优先使用读锁

---

## 📚 推荐资源

### Rust学习
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Rust异步编程](https://rust-lang.github.io/async-book/)

### 库文档
- [egui文档](https://docs.rs/egui/)
- [eframe文档](https://docs.rs/eframe/)
- [tokio文档](https://docs.rs/tokio/)
- [rodio文档](https://docs.rs/rodio/)

### 游戏开发
- [Bevy引擎](https://bevyengine.org/) (如果决定使用ECS)
- [Game Programming Patterns](https://gameprogrammingpatterns.com/)

---

## 🤝 贡献指南

### 开发流程

1. **Fork仓库**
2. **创建功能分支** (`git checkout -b feature/my-feature`)
3. **开发并测试**
4. **Commit** (遵循commit消息约定)
5. **Push到fork** (`git push origin feature/my-feature`)
6. **创建Pull Request**

### Code Review检查清单

- [ ] 代码编译无错误
- [ ] 通过所有测试
- [ ] 添加必要的注释和文档
- [ ] 遵循代码风格
- [ ] 没有unwrap()或panic(除非确实应该panic)
- [ ] 错误处理完善
- [ ] 性能考虑

---

## 📞 获取帮助

### 常见问题

查看 `QUICKSTART.md` 的常见问题部分

### 报告Bug

提供以下信息:
1. Rust版本 (`rustc --version`)
2. 操作系统
3. 完整错误消息
4. 复现步骤
5. 预期行为vs实际行为

### 功能建议

描述:
1. 用例(为什么需要这个功能)
2. 预期行为
3. 可能的实现方案
4. 影响范围

---

## 🎓 下一步

1. **熟悉代码库** - 阅读 `src/` 目录下的代码
2. **运行项目** - 按照 `QUICKSTART.md` 启动
3. **查看进度** - 阅读 `PROGRESS.md` 了解待办事项
4. **选择任务** - 从P1任务开始
5. **开始开发** - 创建分支并开始编码!

**祝开发愉快! 🚀**
