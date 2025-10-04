# 🎮 Crystal MIR2 Rust客户端

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/status-alpha-yellow.svg)]()

**实验性Rust移植版本** - Crystal传奇2客户端的现代化Rust重写

---

## 📋 项目简介

这是Crystal MIR2(传奇2)游戏客户端从C#到Rust的移植项目。目标是利用Rust的性能、内存安全性和现代化生态系统,重新实现一个高效、稳定、跨平台的游戏客户端。

### ✨ 特性

- 🦀 **纯Rust实现** - 内存安全,无需垃圾回收
- 🎨 **egui即时模式UI** - 现代化的UI框架
- 🌐 **异步网络** - 基于Tokio的高性能网络层
- 🖼️ **资源加载** - 兼容原版.lib纹理格式
- 🔊 **音频系统** - 支持音效和背景音乐
- 🚀 **高性能** - 稳定60 FPS,低内存占用
- 🔌 **模块化设计** - 清晰的架构,易于扩展

---

## 🚀 快速开始

### 前置条件

- **Rust 1.70+** - [安装Rust](https://www.rust-lang.org/tools/install)
- **Windows 10/11** (当前主要开发平台)

### 构建与运行

```powershell
# 克隆仓库
git clone <repository-url> Crystal
cd Crystal/ClientRust

# 构建Debug版本
cargo build

# 运行
cargo run

# 或构建Release版本(优化)
cargo build --release
.\target\release\mir2_client.exe
```

更多详情请参阅 [快速启动指南](QUICKSTART.md)

---

## 📊 当前进度

**开发阶段:** P0完成 → P1进行中

```
✅ P0 基础设施      100% (egui, 网络, 资源, 音频)
⏳ P1 登录系统        0% (登录数据包, 角色列表)
⏳ P2 角色系统        0% (创建, 选择, 删除)
⏳ P3 游戏核心        0% (地图, 战斗, 物品)
⏳ P4 进阶功能        0% (社交, UI, 特效)
```

详细进度请查看 [开发进度追踪](PROGRESS.md)

---

## 🏗️ 架构概览

```
ClientRust/
├── src/
│   ├── main.rs              # 入口,启动网络线程
│   ├── app.rs               # 主应用逻辑
│   ├── graphics/            # ✅ 纹理加载(.lib文件)
│   ├── sounds/              # ✅ 音频播放(rodio)
│   ├── network/             # ✅ 异步网络(Tokio)
│   └── scenes/              # ✅ 场景系统(egui UI)
└── docs/                    # 📚 详细文档
```

### 核心技术栈

| 组件 | 技术 | 用途 |
|------|------|------|
| UI框架 | egui 0.29 | 即时模式图形界面 |
| 渲染 | wgpu | 跨平台GPU渲染 |
| 网络 | Tokio | 异步I/O运行时 |
| 音频 | rodio | 纯Rust音频播放 |
| 序列化 | bincode, serde | 数据包编解码 |
| 压缩 | flate2 | GZip解压(.lib文件) |

---

## 📚 文档

| 文档 | 描述 |
|------|------|
| [QUICKSTART.md](QUICKSTART.md) | 快速启动指南 |
| [PROGRESS.md](PROGRESS.md) | 开发进度追踪 |
| [DEVGUIDE.md](DEVGUIDE.md) | 开发者指南 |
| [docs/p0-complete-report.md](docs/p0-complete-report.md) | P0阶段完成报告 |
| [docs/p0-2-network-integration-report.md](docs/p0-2-network-integration-report.md) | 网络层集成详解 |
| [docs/p0-3-texture-loading-report.md](docs/p0-3-texture-loading-report.md) | 纹理加载详解 |

---

## 🎯 开发计划

### P0: 核心基础设施 ✅ (已完成)

- [x] egui图形框架集成
- [x] 网络层连接(TCP + Tokio)
- [x] 资源加载系统(.lib纹理)
- [x] 音频系统(音效 + 音乐)

### P1: 登录系统 ⏳ (进行中)

- [ ] 实现Login数据包发送
- [ ] 处理LoginSuccess/Failure响应
- [ ] 登录界面美化(背景图 + 纹理按钮)
- [ ] 错误提示对话框

### P2: 角色系统 ⏳ (待开始)

- [ ] SelectScene实现(显示角色列表)
- [ ] 角色创建对话框
- [ ] 角色删除功能
- [ ] 进入游戏

### P3: 游戏核心 ⏳ (待开始)

- [ ] 地图系统(加载 + 渲染)
- [ ] 玩家角色(移动 + 动画)
- [ ] 输入系统(键盘 + 鼠标)
- [ ] NPC系统
- [ ] 怪物系统

详细计划请查看 [PROGRESS.md](PROGRESS.md)

---

## 🤝 贡献

欢迎贡献!请查看 [开发者指南](DEVGUIDE.md) 了解如何参与开发。

### 贡献步骤

1. Fork仓库
2. 创建功能分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m '[module] Add amazing feature'`)
4. 推送分支 (`git push origin feature/AmazingFeature`)
5. 创建Pull Request

---

## 🐛 已知问题

- [ ] 登录数据包尚未发送(P1任务)
- [ ] 资源文件路径硬编码
- [ ] SharedRust的glob re-exports警告

完整问题列表请查看 [PROGRESS.md](PROGRESS.md#-已知问题列表)

---

## 📈 性能指标

| 指标 | 值 |
|------|---|
| FPS | 稳定60 (vsync) |
| 内存使用 | ~50MB (无资源), ~200MB (加载纹理后) |
| 启动时间 | <1秒 |
| 二进制大小 | Debug: ~80MB, Release: ~15MB |
| 编译时间 | 首次: ~5分钟, 增量: ~30秒 |

---

## 📝 许可证

本项目采用 MIT 或 Apache-2.0 双许可证

- MIT License: [LICENSE-MIT](LICENSE-MIT)
- Apache License 2.0: [LICENSE-APACHE](LICENSE-APACHE)

---

## 🙏 致谢

- **原C#版本** - Crystal项目提供了完整的参考实现
- **Rust社区** - 优秀的库和工具支持
- **egui** - 简洁高效的UI框架
- **Tokio** - 强大的异步运行时

---

## 📞 联系

- **Issues:** [GitHub Issues](https://github.com/yourusername/crystal/issues)
- **Discussions:** [GitHub Discussions](https://github.com/yourusername/crystal/discussions)

---

## 🎮 游戏截图

> 待添加(当前阶段为开发框架,UI较为简陋)

### 登录界面
```
┌─────────────────────────────────┐
│     Crystal MIR2 - Rust         │
│                                 │
│  用户名: [              ]       │
│  密码:   [              ]       │
│                                 │
│         [  登 录  ]             │
│                                 │
│  状态: 已连接                   │
│  FPS: 60                        │
└─────────────────────────────────┘
```

---

## 🚀 下一步

查看 [开发进度追踪](PROGRESS.md) 了解即将开发的功能!

**让我们一起用Rust重新定义传奇!** 🦀⚔️
