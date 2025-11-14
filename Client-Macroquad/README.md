# Crystal - Rust macroquad 游戏客户端

传奇2客户端 - 基于 macroquad 的现代化 Rust 实现

## 项目结构

本项目采用 Rust 标准的**库+多可执行文件**架构:

```
Client-Macroquad/
├── src/
│   ├── lib.rs              # 库根文件 (导出所有模块)
│   ├── bin/
│   │   ├── test_login.rs   # 登录场景测试工具
│   │   └── map_viewer.rs   # 地图查看器工具
│   ├── scenes/             # 场景系统
│   ├── resources/          # 资源管理 (MLibrary)
│   ├── map_renderer/       # 地图渲染系统
│   ├── network/            # 网络通信
│   └── ...
├── assets/                 # 字体等静态资源
└── Data/                   # 游戏数据文件 (.Lib格式)
```

## 快速开始

### 开发模式

```bash
# 测试登录界面
cargo run --bin test_login

# 查看游戏地图
cargo run --bin map_viewer
```

### 发布模式

```bash
# 登录场景 (优化性能)
cargo run --bin test_login --release

# 地图查看器 (优化性能)
cargo run --bin map_viewer --release
```

## 可执行文件

### test_login - 登录场景测试

测试登录UI、资源加载、场景切换逻辑

**功能**:
- macroquad::ui 集成 (账号/密码输入)
- 19帧背景动画 (Data/ChrSel.Lib)
- 场景切换系统 (Login → CharacterSelect → Game)

**快捷键**:
- `Tab` - 切换输入框
- `Enter` - 登录
- `ESC` - 退出

### map_viewer - 地图查看器

查看游戏地图、测试地图渲染

**操作**:
- `W/A/S/D` - 移动摄像机
- `+/-` - 缩放
- `L` - 切换图层
- `H` - 显示帮助

## 技术架构

### 场景系统
- **enum_dispatch**: 零成本场景抽象
- **SceneHandler trait**: 统一场景接口

### 资源管理
- **MLibrary**: .Lib 文件解析器
- **自动纹理缓存**: get_or_create_texture()

### UI 系统
- **macroquad::ui**: 基于 megaui 的现代 UI 框架
- **Window/InputText/Button**: 丰富的组件库

## 开发状态

✅ **已完成**:
- 项目架构重构 (lib + bin)
- 登录场景 UI
- 19帧背景动画
- 资源加载系统
- 场景切换系统
- 地图查看器

🚧 **进行中**:
- 输入验证
- 网络登录
- UI皮肤

📋 **待实现**:
- 创建账号对话框
- 角色选择场景
- 游戏主场景

## 文档

详细文档位于 `docs/` 目录:
- `09.Macroquad_UI学习笔记.md` - macroquad UI 教程
- `00-08.*` - 项目设计文档

完整使用指南见旧版 `GAME_MODE.md` (已过时, 待更新)

## 贡献

欢迎提交 Issue 和 Pull Request!

## 许可证

(待添加)
