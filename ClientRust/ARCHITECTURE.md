# Crystal Client 架构设计文档

## 架构分层

```
┌─────────────────────────────────────────────────┐
│           游戏逻辑层 (Game Logic)               │
│  ECS Components, Systems, Game Rules            │
└─────────────┬───────────────────────────────────┘
              │
┌─────────────▼───────────────────────────────────┐
│         图形逻辑层 (Graphics Logic)              │
│  Animation, Effects, Particles, UI               │
└─────────────┬───────────────────────────────────┘
              │
┌─────────────▼───────────────────────────────────┐
│       渲染引擎层 (Rendering Engine)              │
│  Renderer Trait, Sprite System, Text Rendering  │
│  ┌──────────┬──────────┬──────────┐             │
│  │ ggez     │ macroquad│ wgpu     │ (backends)  │
│  └──────────┴──────────┴──────────┘             │
└─────────────┬───────────────────────────────────┘
              │
┌─────────────▼───────────────────────────────────┐
│       资源管理层 (Resource Management)           │
│  Image Loader, Map Loader, Cache, Asset Manager │
│  (纯数据，无渲染依赖)                             │
└──────────────────────────────────────────────────┘
```

## 模块职责

### 1. resources/ - 资源管理层

**职责**: 加载和管理游戏资源数据（纯数据，与渲染后端无关）

```rust
// 资源管理器
pub struct ResourceManager {
    image_cache: LruCache<String, ImageData>,
    map_cache: LruCache<String, MapData>,
}

// 图像加载器 (.lib 文件)
pub struct ImageLoader {
    file_path: PathBuf,
    indices: Vec<ImageIndex>,
}

impl ImageLoader {
    // 返回纯 RGBA 数据
    pub fn load_image(&mut self, index: usize) -> Result<ImageData>;
}

// 图像数据（纯数据）
pub struct ImageData {
    pub width: u16,
    pub height: u16,
    pub offset_x: i16,
    pub offset_y: i16,
    pub rgba_data: Vec<u8>,  // 已解压、已转换的 RGBA
    pub mask: Option<MaskData>,
}
```

**特点**:

- ✅ 不依赖 ggez、macroquad 等渲染库
- ✅ 只使用 std、byteorder、flate2
- ✅ 可在服务器端、工具端复用
- ✅ 支持异步加载、流式加载

### 2. rendering/ - 渲染引擎层

**职责**: 提供跨后端的渲染抽象，管理 GPU 资源

```rust
// 渲染器 trait
pub trait Renderer {
    fn clear(&mut self, color: Color);
    fn draw_sprite(&mut self, texture: &Texture, params: DrawParams);
    fn draw_text(&mut self, text: &str, params: TextParams);
    fn present(&mut self);
}

// 纹理管理器
pub trait TextureManager {
    fn create_texture(&mut self, data: &ImageData) -> TextureId;
    fn get_texture(&self, id: TextureId) -> Option<&Texture>;
    fn delete_texture(&mut self, id: TextureId);
}

// 精灵系统
pub struct SpriteRenderer {
    texture_manager: Box<dyn TextureManager>,
    batch: Vec<SpriteBatch>,
}

impl SpriteRenderer {
    // 从资源数据创建精灵
    pub fn create_sprite(&mut self, image_data: &ImageData) -> SpriteHandle;
    
    // 绘制精灵
    pub fn draw(&mut self, sprite: SpriteHandle, x: f32, y: f32);
}
```

**特点**:

- ✅ 后端无关的抽象接口
- ✅ 管理 GPU 资源生命周期
- ✅ 支持批处理优化
- ✅ 条件编译不同后端

### 3. graphics/ - 图形逻辑层

**职责**: 实现游戏特定的图形功能

```rust
// 动画系统
pub struct AnimationPlayer {
    frames: Vec<SpriteHandle>,
    current_frame: usize,
    frame_duration: f32,
}

// 特效系统
pub struct EffectSystem {
    effects: Vec<Effect>,
}

// 粒子系统
pub struct ParticleEmitter {
    particles: Vec<Particle>,
}
```

**特点**:

- ✅ 使用 resources/ 加载数据
- ✅ 使用 rendering/ 绘制
- ✅ 实现游戏逻辑（动画、特效）

## 依赖关系

```
Game Logic
    ↓ (uses)
Graphics Logic
    ↓ (uses)
Rendering Engine ←─────┐
    ↓ (uses)           │ (no dependency)
Resource Management ───┘
```

**关键点**:

- 资源层不依赖渲染层（可独立测试、复用）
- 渲染层依赖资源层获取数据
- 图形层协调资源层和渲染层

## 迁移策略

### 第一阶段：资源层独立

1. ✅ 创建 `src/resources/image_loader.rs`（基于 `mlibrary_data.rs`）
2. ✅ 创建 `src/resources/cache.rs`（LRU 缓存）
3. ✅ 创建 `src/resources/manager.rs`（统一资源管理）

### 第二阶段：渲染层抽象

1. ✅ 重命名 `backends/` → `rendering/backends/`
2. ✅ 完善 `Renderer` 和 `TextureManager` trait
3. ✅ 实现 `SpriteRenderer`、`TextRenderer`

### 第三阶段：图形层解耦

1. 🔄 移除 `graphics/mlibrary.rs` 对 ggez 的依赖
2. 🔄 重构 `graphics/libraries.rs` 使用资源层
3. ✅ 实现动画、特效系统

### 第四阶段：游戏逻辑迁移

1. 🔄 ECS 组件使用新的渲染抽象
2. ✅ 场景系统支持多后端

## 优势分析

### 当前架构问题

❌ `graphics/mlibrary.rs` 直接依赖 ggez
❌ `graphics/libraries.rs` 与渲染后端耦合
❌ 资源加载和 GPU 纹理创建混在一起
❌ 难以支持多渲染后端
❌ 无法在非图形环境（服务器、测试）使用

### 新架构优势

✅ **清晰分层**: 资源 → 渲染 → 图形 → 逻辑
✅ **后端无关**: 资源层可在任何环境使用
✅ **易于测试**: 每层可独立单元测试
✅ **性能优化**: 资源预加载、渲染批处理
✅ **可扩展性**: 轻松添加新渲染后端
✅ **代码复用**: 服务器可复用资源加载逻辑

## 实施建议

### 推荐方案：渐进式重构

```bash
# 1. 先创建资源层（不破坏现有代码）
src/resources/
    mod.rs
    image_loader.rs    # 复制 mlibrary_data.rs
    map_loader.rs
    cache.rs
    manager.rs

# 2. 重命名渲染层（保持向后兼容）
src/rendering/
    mod.rs
    traits.rs          # 移自 backends/mod.rs
    types.rs           # 移自 backends/types.rs
    sprite.rs          # 🆕 精灵系统
    text.rs            # 🆕 文字渲染
    backends/
        ggez.rs        # 移自 backends/ggez_backend.rs
        macroquad.rs   # 移自 backends/macroquad_backend.rs

# 3. 重构图形层（使用新架构）
src/graphics/
    mod.rs
    animation.rs       # 🆕 使用资源层 + 渲染层
    effects.rs         # 🆕
    particle.rs        # 重构现有代码
    
# 4. 保持兼容性（过渡期）
src/backends/          # 别名，重导出 rendering/
src/graphics/mlibrary.rs  # 标记为 deprecated
```

### 具体步骤

1. **创建资源层**（不影响现有代码）

   ```bash
   git checkout -b refactor/resource-layer
   # 创建 src/resources/，复制 mlibrary_data.rs
   # 添加测试
   ```

2. **重构渲染层**（渐进式）

   ```bash
   git checkout -b refactor/rendering-layer
   # 移动 backends/ 到 rendering/
   # 添加精灵、文字渲染抽象
   ```

3. **更新调用方**（分批进行）

   ```bash
   # 先更新 demo 程序
   # 再更新主程序
   # 最后移除旧代码
   ```

## 总结

**核心思想**: 分离关注点（Separation of Concerns）

- **资源管理**: 只管数据加载、缓存
- **渲染引擎**: 只管 GPU 资源、绘制调用
- **图形逻辑**: 只管动画、特效等游戏逻辑
- **游戏逻辑**: 只管 ECS、场景、玩法

**下一步行动**:

1. ✅ 创建 `src/resources/` 目录结构
2. ✅ 移植 `mlibrary_data.rs` 到资源层
3. ✅ 实现统一的资源管理器
4. 🔄 重构现有代码使用新架构
