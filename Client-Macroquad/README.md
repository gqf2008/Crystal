# Client-Macroquad

传奇2客户端 - Macroquad 独立版本

## 项目简介

这是一个独立的 macroquad 项目，与现有的 `ClientRust` (ggez 版本) 并行开发。

## 目标

- ✅ 建立独立的项目结构
- ⏳ 从 ClientRust 移植地图渲染核心
- ⏳ 移植游戏逻辑和UI系统
- ⏳ 实现完整的游戏客户端

## 项目结构

```
Client-Macroquad/
├── Cargo.toml          # 项目配置
├── README.md           # 本文件
├── Data/               # 游戏资源 (软链接到 ../ClientRust/Data)
├── Map/                # 地图文件 (软链接到 ../ClientRust/Map)
└── src/
    ├── main.rs         # 主程序入口
    ├── graphics/       # 图形渲染模块 (待移植)
    ├── map/            # 地图系统 (待移植)
    ├── network/        # 网络通信 (待移植)
    └── ui/             # 用户界面 (待移植)
```

## 与 ClientRust 的关系

### 共享资源
- `Data/` 和 `Map/` 通过软链接共享资源文件
- 避免重复存储大量游戏资源

### 独立开发
- 不依赖 ClientRust 的代码库
- 可以独立编译和运行
- 渐进式移植，风险可控

## 当前状态

### 已完成
- ✅ 项目框架搭建
- ✅ 基础窗口系统
- ✅ macroquad 环境配置

### 进行中
- ⏳ 地图渲染模块移植

### 待完成
- ⏹ 角色渲染
- ⏹ 动画系统
- ⏹ UI 系统
- ⏹ 网络通信
- ⏹ 音效系统

## 编译运行

```bash
# 开发版本
cargo run

# 发布版本
cargo run --release

# 或直接运行可执行文件
./target/release/client_macroquad
```

## 下一步计划

1. 从 `ClientRust/src/bin/map_viewer_macroquad_v2.rs` 提取核心代码
2. 移植必要的模块：
   - `graphics/mlibrary.rs` - 图块库加载
   - `map/map_data.rs` - 地图数据结构
   - 地图渲染逻辑
3. 逐步添加游戏逻辑

## 技术栈

- **渲染引擎**: macroquad 0.4
- **性能分析**: macroquad-profiler 0.2
- **图像处理**: image 0.25
- **序列化**: serde + bincode

## 优势

### vs ggez 版本
- ✅ 更简单的 API
- ✅ 更好的跨平台支持
- ✅ 内置性能分析工具
- ✅ 更快的编译速度

### 独立项目
- ✅ 不影响现有 ggez 版本
- ✅ 可以并行对比测试
- ✅ 渐进式迁移，风险可控
- ✅ 代码结构更清晰

## 参考

- [macroquad 文档](https://docs.rs/macroquad/)
- [ClientRust 项目](../ClientRust/)
- [地图查看器示例](../ClientRust/src/bin/map_viewer_macroquad_v2.rs)
