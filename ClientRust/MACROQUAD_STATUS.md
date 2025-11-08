# Macroquad 移植进度报告

## ✅ 已完成工作

### 1. 项目架构设计

- ✅ 添加了 `macroquad` 依赖到 `Cargo.toml`
- ✅ 创建了 feature flags 用于在 `backend-ggez` 和 `backend-macroquad` 之间切换
- ✅ 设置了条件编译，ECS 模块仅在 ggez 后端时编译

### 2. 渲染抽象层 (`src/backends/`)

- ✅ 创建了 `Renderer` trait - 统一的渲染接口
- ✅ 创建了 `TextureManager` trait - 纹理管理接口
- ✅ 定义了通用类型 (`Vec2`, `Color`, `Rect`, `DrawParams` 等)
- ✅ 实现了 `MacroquadRenderer` - 完整的 macroquad 渲染后端

### 3. 演示程序

- ✅ 创建了 `demo_macroquad.rs` - 独立的 macroquad 演示
- ✅ 创建了 `map_viewer_macroquad.rs` - 地图查看器框架
- ✅ 添加了 Cargo 二进制目标配置

### 4. 代码模块化

- ✅ 将 `src/graphics/mod.rs` 中的 ggez 绘制函数移到 `ggez_helpers` 子模块
- ✅ 为 `ecs` 和 `objects` 模块添加条件编译 (#[cfg(feature = "backend-ggez")])

## ⚠️ 当前问题

### MLibrary 模块的 ggez 依赖

`src/graphics/mlibrary.rs` 中的 `ImageInfo` 结构直接包含 `ggez::graphics::Image`：

```rust
pub struct ImageInfo {
    pub image: Option<ggez::graphics::Image>,  // ❌ 硬编码 ggez 依赖
    pub mask_image: Option<ggez::graphics::Image>,
    // ...
}
```

**影响：**

- ❌ 无法在 `backend-macroquad` feature 下编译
- ❌ 当前有 61 个编译错误
- ❌ `demo_macroquad` 无法构建

## 📋 解决方案建议

### 方案 A：完整重构 MLibrary（推荐，长期最佳）

将 `ImageInfo` 改为只存储 RGBA 数据，不依赖任何渲染后端：

```rust
pub struct ImageInfo {
    pub width: u16,
    pub height: u16,
    pub x: i16,
    pub y: i16,
    pub data: Vec<u8>,  // 原始 RGBA 数据
    pub mask_data: Option<Vec<u8>>,  // 遮罩数据
}
```

然后创建后端特定的纹理缓存：

```rust
// ggez 版本
#[cfg(feature = "backend-ggez")]
struct GgezTextureCache {
    textures: HashMap<usize, ggez::graphics::Image>,
}

// macroquad 版本
#[cfg(feature = "backend-macroquad")]  
struct MacroquadTextureCache {
    textures: HashMap<usize, macroquad::texture::Texture2D>,
}
```

**优点：**

- ✅ 完全解耦渲染后端
- ✅ 数据层可以在任何后端间共享
- ✅ 符合架构设计原则

**缺点：**

- ⚠️ 需要大量重构现有代码
- ⚠️ 可能破坏现有的 ggez 代码

### 方案 B：使用类型别名和条件编译（快速方案）

使用 `cfg` 为不同后端定义不同的 `ImageInfo`：

```rust
#[cfg(feature = "backend-ggez")]
pub struct ImageInfo {
    pub image: Option<ggez::graphics::Image>,
    pub mask_image: Option<ggez::graphics::Image>,
    // ...
}

#[cfg(feature = "backend-macroquad")]
pub struct ImageInfo {
    pub width: u16,
    pub height: u16,
    pub data: Vec<u8>,
    // ...
}
```

**优点：**

- ✅ 快速实现
- ✅ 不破坏现有 ggez 代码

**缺点：**

- ⚠️ 代码重复
- ⚠️ API 不一致

### 方案 C：先独立实现 macroquad 版本（当前推荐）

暂时不修改 `MLibrary`，让 `demo_macroquad` 完全独立：

1. **立即可运行：** `demo_macroquad` 不依赖 `MLibrary`
2. **并行开发：** 在新分支逐步重构 `MLibrary`
3. **验证架构：** 先验证 macroquad 后端的可行性

## 🎯 下一步行动

### 立即执行（让 demo_macroquad 可以运行）

```bash
# 1. 测试当前的 demo_macroquad
cargo run --bin demo_macroquad --no-default-features --features backend-macroquad

# 2. 如果成功，尝试构建 WASM 版本
rustup target add wasm32-unknown-unknown
cargo build --bin demo_macroquad --target wasm32-unknown-unknown \
  --no-default-features --features backend-macroquad --release
```

### 中期计划（重构 MLibrary）

1. 创建 `src/graphics/texture_data.rs` - 存储原始图像数据
2. 修改 `MLibrary` 只负责加载和解析 `.lib` 文件
3. 创建 `TextureCache` trait 和两个实现
4. 更新所有调用代码

### 长期目标（完整的多后端支持）

1. 实现 `GgezBackend` 适配器
2. 将 ECS 渲染系统改为使用 `Renderer` trait
3. 统一输入处理 (键盘/鼠标/触摸)
4. 音频系统抽象
5. 移动端测试 (iOS/Android)

## 运行命令快速参考

```bash
# ggez 版本 (默认)
cargo run --bin map_viewer_v3

# macroquad 演示
cargo run --bin demo_macroquad --no-default-features --features backend-macroquad

# 检查编译错误数量
cargo check --bin demo_macroquad --no-default-features --features backend-macroquad 2>&1 | grep "error\[" | wc -l
```

## 📊 当前状态总结

| 项目 | 状态 | 备注 |
|------|------|------|
| 渲染抽象层 | ✅ 完成 | `Renderer` 和 `TextureManager` trait |
| MacroquadRenderer | ✅ 完成 | 基本实现完成 |
| demo_macroquad | ⚠️ 部分 | 代码完成，但无法编译 |
| MLibrary 重构 | ❌ 未开始 | 需要解耦 ggez 依赖 |
| map_viewer_macroquad | ❌ 未开始 | 依赖 MLibrary 重构 |
| Web (WASM) 支持 | ⏸️ 待验证 | 等demo 运行后测试 |

## 💡 建议

基于当前进度，我建议：

1. **先让 `demo_macroquad` 运行起来** - 这是最小可行产品，可以验证 macroquad 的可行性
2. **在独立分支重构 MLibrary** - 不影响主线开发
3. **并行推进** - ggez 版本继续开发，macroquad 版本逐步完善

您想：

- A. 继续完成 `demo_macroquad` 让它可以运行？
- B. 开始重构 `MLibrary` 支持多后端？
- C. 先在 Web 上测试当前的 demo？
