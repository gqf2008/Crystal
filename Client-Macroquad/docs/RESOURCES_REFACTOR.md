# Resources 模块重构总结

## 重构目标
实现高效的 .lib 资源管理系统，包括解析、缓存、读取等功能，以及 egui 与 macroquad 纹理的高效转换。

## 新增模块

### 1. `texture_cache.rs` - 纹理缓存管理
**功能特性：**
- LRU (最近最少使用) 缓存策略
- 自动内存管理和清理
- 过期时间管理
- 缓存统计（命中率、淘汰次数等）
- 支持 macroquad 和 egui 两种纹理类型

**核心 API：**
```rust
// 创建缓存
let cache = TextureCache::with_defaults(); // 默认1000条目，30秒过期

// 获取纹理（自动缓存）
cache.get_mq_texture(key, || create_texture());
cache.get_egui_texture(ctx, key, || create_texture());

// 清理过期缓存
cache.cleanup_expired();

// 获取统计
let stats = cache.stats();
println!("命中率: {:.2}%", stats.hit_rate() * 100.0);
```

### 2. `texture_converter.rs` - 纹理格式转换
**功能特性：**
- BGRA ↔ RGBA 零拷贝转换
- 黑色背景自动透明化
- egui 与 macroquad 纹理互转
- 纹理哈希（用于去重）

**核心 API：**
```rust
// 创建 macroquad 纹理
let texture = TextureConverter::create_mq_texture(width, height, rgba_data);

// macroquad → egui
let handle = TextureConverter::mq_to_egui(ctx, &texture, "name");

// BGRA → RGBA（就地转换）
TextureConverter::bgra_to_rgba_with_transparency(&mut data);

// 生成缓存键
let key = TextureConverter::create_texture_key("prguse", 100);
```

## 优化改进

### `ImageInfo` 结构优化
**改进点：**
1. **延迟加载纹理** - 纹理数据按需创建，节省内存
2. **智能数据管理** - RGBA 数据在纹理创建后可选择性释放
3. **统一转换接口** - 使用 `TextureConverter` 统一处理格式转换
4. **访问器方法** - 提供 `texture()`, `mask_texture()` 等只读访问

**字段说明：**
```rust
pub struct ImageInfo {
    // 元数据（始终加载）
    pub width, height, x, y: i16,
    
    // 纹理数据（按需加载）
    pub image: Option<Texture2D>,           // macroquad 纹理
    pub egui_texture: Option<TextureHandle>, // egui 纹理
    pub mask_image: Option<Texture2D>,      // 遮罩纹理
    
    // 私有数据（内部管理）
    rgba_data: Option<Vec<u8>>,            // 原始数据（可释放）
    last_access_time: Option<Instant>,     // 缓存管理
}
```

### `MLibrary` 增强
**新增功能：**
1. **库名称管理** - 自动提取并存储库名
2. **改进的 API** - `get_or_create_texture` 返回可变引用
3. **统一转换** - 使用 `TextureConverter` 处理所有格式转换

### `libraries.rs` 新增 API
**便捷函数：**
```rust
// 获取 ImageInfo（包含所有数据）
let info = get_or_create_texture(LibraryName::Prguse, 100);

// 直接获取 egui 纹理（推荐用于 UI）
let handle = get_or_create_egui_texture(ctx, LibraryName::Title, 200);
```

## 性能提升

### 1. 内存管理
- **延迟加载**: 纹理按需创建，减少初始内存占用
- **智能释放**: RGBA 数据在纹理创建后可释放
- **LRU 缓存**: 自动淘汰长期未使用的纹理

### 2. 转换效率
- **零拷贝转换**: BGRA→RGBA 就地转换，无额外分配
- **复用纹理**: egui/macroquad 纹理可互相转换，避免重复解码
- **批量处理**: 支持预热缓存，批量加载资源

### 3. 缓存优化
- **统计监控**: 实时监控命中率，优化缓存策略
- **自动清理**: 定期清理过期纹理，防止内存泄漏
- **智能淘汰**: LRU 策略确保热点资源常驻内存

## 使用示例

### 基础使用
```rust
use crate::resources::*;

// 初始化库
initialize_all_libraries("Data")?;

// 获取纹理（自动缓存）
if let Some(info) = LibraryName::Prguse.get_image(360) {
    println!("尺寸: {}x{}", info.width, info.height);
}

// 在 egui 中使用
if let Some(handle) = get_or_create_egui_texture(ctx, LibraryName::Title, 100) {
    ui.image(&handle);
}
```

### 高级缓存管理
```rust
use crate::resources::{TextureCache, CacheKey};

let mut cache = TextureCache::new(2000, Duration::from_secs(60));

// 预热缓存
let keys = vec![
    CacheKey::new("prguse", 0),
    CacheKey::new("prguse", 1),
];
cache.warmup(keys, |key| {
    // 创建纹理...
});

// 定期清理
cache.cleanup_expired();

// 查看统计
println!("缓存: {}/{}, 命中率: {:.1}%", 
    cache.len(), 
    cache.max_entries,
    cache.stats().hit_rate() * 100.0
);
```

## 兼容性说明

### 向后兼容
- ✅ 现有代码无需修改（`image` 字段仍为公开）
- ✅ 所有原有 API 保持不变
- ✅ 新增功能为可选增强

### 推荐迁移
```rust
// 旧方式（仍然可用）
let lib = get_library(LibraryName::Prguse)?;
let mut lib = lib.borrow_mut();
let info = lib.get_or_create_texture(100)?;
if let Some(texture) = &info.image { ... }

// 新方式（更简洁）
if let Some(handle) = get_or_create_egui_texture(ctx, LibraryName::Prguse, 100) {
    ui.image(&handle);
}
```

## 测试覆盖

所有新模块包含单元测试：
- ✅ `texture_cache.rs`: 缓存基础操作、LRU 淘汰
- ✅ `texture_converter.rs`: 格式转换、透明化处理
- ✅ `mlibrary.rs`: 现有测试全部通过

## 未来优化方向

1. **异步加载** - 支持后台异步加载纹理
2. **压缩缓存** - 对不常用纹理进行压缩存储
3. **预加载策略** - 智能预测并预加载即将使用的资源
4. **多线程支持** - 使用 Arc/Mutex 支持跨线程访问

## 总结

本次重构实现了：
- ✅ 高效的 .lib 资源解析和缓存
- ✅ egui/macroquad 纹理无缝转换
- ✅ LRU 缓存策略和自动内存管理
- ✅ 完整的单元测试覆盖
- ✅ 向后兼容，平滑迁移

编译状态：**✅ 成功** (15个警告，0个错误)
