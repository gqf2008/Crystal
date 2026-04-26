# Crystal - Rust macroquad 游戏客户端

传奇2客户端 — 基于 macroquad 的现代化 Rust 实现

## 项目结构

```
Client-Macroquad/
├── src/
│   ├── lib.rs              # 库根
│   ├── main.rs             # 主入口
│   ├── bin/                # 测试/工具入口
│   ├── scenes/             # 场景系统 (login/select/game)
│   ├── systems/            # ECS 系统 (6层: infra/input/logic/presentation/rendering/dbug)
│   ├── components/         # ECS 组件
│   ├── network/            # 网络通信 (双线程, 17 handler)
│   ├── resources/          # 资源管理 (MLibrary .lib 解析)
│   ├── map_renderer/       # 地图渲染
│   ├── event_bus/          # 事件总线
│   └── ui/                 # UI 组件
├── assets/                 # 字体等静态资源
├── Data/                   # 游戏数据 (.Lib 格式)
└── docs/                   # 技术文档
```

## 快速开始

```bash
# 编译
cargo build

# 测试 (10 E2E + 单元测试)
cargo test --lib

# 发布版
cargo build --release
```

## 状态

- **移植进度**: ~99%
- **ECS 系统**: 46 个就绪
- **对话框**: 37/38 移植完成
- **网络覆盖**: 17 handler 覆盖所有协议

服务端状态见 `ServerRust/docs/PORT_STATUS.md`
