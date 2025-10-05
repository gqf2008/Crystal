# Ggez 迁移完成指南

## ✅ 已完成的工作

### 1. 架构迁移
- ✅ 从 winit + wgpu → ggez 0.10.0-rc0
- ✅ 移除独立的 winit 依赖(使用 `ggez::winit`)
- ✅ 删除旧的 main.rs(winit + DXManager 架构)
- ✅ 统一入口点: `mir2_client` → `src/main_ggez.rs`

### 2. 核心功能实现
- ✅ Scene 系统(Scene trait + 自定义类型)
- ✅ 输入系统(键盘/鼠标事件转换)
- ✅ MLibrary ggez 渲染函数
  - `MLibrary::draw_to_canvas()` - 基础渲染
  - `MLibrary::draw_to_canvas_with_offset()` - 带偏移渲染
- ✅ UI 模块适配 ggez

### 3. 编译状态
```
✓ 0 个错误
⚠ 589 个警告(未使用代码,不影响功能)
```

---

## 🎯 下一步:运行测试

### 步骤 1: 准备游戏资源文件

程序需要传奇 MIR2 的 `.lib` 图像库文件。

**1.1 创建 resources 目录**
```powershell
New-Item -ItemType Directory -Force ClientRust/resources
```

**1.2 复制 .lib 文件**
将以下文件复制到 `ClientRust/resources/` 目录:
- `Prguse.lib` - 主要 UI 资源 (必需)
- `Prguse2.lib` - 次要 UI 资源
- `Magic.lib` - 魔法效果
- `Items.lib` - 物品图标
- ...等等

**文件位置示例:**
```
ClientRust/
  ├── resources/
  │   ├── Prguse.lib  ← 从传奇客户端 Data 目录复制
  │   ├── Prguse2.lib
  │   ├── Magic.lib
  │   └── ...
  ├── src/
  └── Cargo.toml
```

### 步骤 2: 运行程序

```powershell
cd ClientRust
cargo run --bin mir2_client
```

### 步骤 3: 预期结果

✅ **成功标志:**
1. 窗口打开(深蓝色背景)
2. 日志显示: `✓ 测试渲染成功`
3. 窗口中央显示一个测试图像
4. LoginScene 初始化

❌ **如果看到警告:**
```
✗ 加载失败 Prguse: 系统找不到指定的文件
```
→ 说明 resources 目录中缺少 .lib 文件

---

## 🧪 当前测试代码

**位置:** `src/main_ggez.rs:167-184`

```rust
// 🧪 测试 MLibrary 渲染
if let Some(lib_arc) = get_library(LibraryName::Prguse) {
    if let Ok(mut lib) = lib_arc.try_lock() {
        match lib.draw_to_canvas(ctx, &mut canvas, 0, 100.0, 100.0, true) {
            Ok(_) => {
                tracing::debug!("✓ 测试渲染成功");
            }
            Err(e) => {
                tracing::warn!("测试渲染失败: {}", e);
            }
        }
    }
}
```

**说明:**
- 尝试从 Prguse.lib 渲染索引 0 的图像
- 坐标: (100, 100)
- 如果成功,你会在窗口左上角附近看到一个图像

---

## 📋 接下来的开发任务

### 阶段 1: 完善 LoginScene 渲染 (1-2天)
- [ ] 绘制登录背景
- [ ] 绘制登录对话框
- [ ] 绘制输入框和按钮
- [ ] 实现键盘输入(用户名/密码)
- [ ] 实现鼠标点击(按钮)

### 阶段 2: 网络连接 (1天)
- [ ] 自动连接服务器
- [ ] 版本验证
- [ ] 登录请求
- [ ] 显示连接状态

### 阶段 3: SelectScene (0.5天)
- [ ] 角色选择界面
- [ ] 角色列表显示
- [ ] 创建角色

### 阶段 4: GameScene (2-3天)
- [ ] 地图渲染
- [ ] 角色渲染
- [ ] 移动控制
- [ ] 聊天系统

---

## 🐛 已知问题

### 1. Blend Mode 未实现
**问题:** ggez 0.10 的 blend mode 设置方式变化
**解决方案:** 需要研究 ggez 0.10 的正确混合模式设置
**影响:** 透明度可能不正确

### 2. 纹理缓存
**问题:** 每次 draw 都创建新的 Image
**解决方案:** 实现纹理缓存机制(参考 ggez_manager_simple.rs)
**影响:** 性能较低

---

## 📖 API 参考

### MLibrary 渲染

```rust
use crate::graphics::libraries::{get_library, LibraryName};

// 获取库
let lib_arc = get_library(LibraryName::Prguse)?;
let mut lib = lib_arc.lock().unwrap();

// 基础渲染
lib.draw_to_canvas(
    ctx,       // ggez Context
    canvas,    // ggez Canvas
    0,         // 图像索引
    100.0,     // x 坐标
    100.0,     // y 坐标
    true       // 是否混合(暂时无效)
)?;

// 带偏移渲染(自动应用 ImageInfo.x/y)
lib.draw_to_canvas_with_offset(ctx, canvas, 0, 100.0, 100.0, true)?;
```

### Scene 绘制

```rust
impl Scene for LoginScene {
    fn draw(&self, canvas: &mut Canvas, ggez_manager: &GgezManager) {
        // 获取 Context (从 ggez_manager 或其他地方)
        // let ctx = ...;
        
        // 绘制背景
        // ...
    }
}
```

---

## 🎉 里程碑

- ✅ **2025-10-05**: 完成 ggez 迁移
- ✅ **2025-10-05**: 实现 MLibrary 渲染函数
- ✅ **2025-10-05**: 修复所有编译错误
- ⏳ **下一步**: 加载游戏资源,看到第一个画面!

---

## 💡 提示

1. **Ctrl+Q** 退出程序
2. 日志级别: 设置 `RUST_LOG=debug` 查看详细日志
3. 性能分析: 使用 `--release` 编译获得更好性能

```powershell
$env:RUST_LOG="debug"
cargo run --bin mir2_client --release
```
