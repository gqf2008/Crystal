# graphics - 图形渲染模块

**对应C#代码**: `Client/MirGraphics/`  
**文件数**: 3  
**代码行数**: 3,193  
**状态**: ✅ 核心完成，粒子引擎待实现

---

## 📚 目录

1. [模块概述](#-模块概述)
2. [架构设计](#-架构设计)
3. [核心组件](#-核心组件)
4. [图像库系统](#-图像库系统)
5. [使用指南](#-使用指南)
6. [开发状态](#-开发状态)

---

## 📖 模块概述

`graphics` 模块负责游戏的图形资源管理和渲染准备，包括：

- **图像库管理**: 加载和管理 .lib 格式的图像库
- **纹理缓存**: 缓存加载的纹理，优化性能
- **资源定位**: 根据类型和索引快速定位图像
- **偏移量管理**: 处理图像的渲染偏移
- **延迟加载**: 按需加载图像，减少内存占用

> ⚠️ **重要**: 此模块**只负责资源管理**，不包含实际渲染代码。渲染由 ECS 的 RenderSystem 负责。

### 与C#版本的对应关系

| C# 文件 | Rust 文件 | 说明 |
|---------|----------|------|
| `MLibrary.cs` | `mlibrary.rs` | 图像库核心实现 |
| `Libraries.cs` | `libraries.rs` | 全局库管理 |
| `DXManager.cs` | ❌ | Direct3D管理（GGEZ替代） |
| `ParticleEngine.cs` | ⏳ | 粒子引擎（待实现） |

---

## 🏗 架构设计

### 模块结构

```
graphics/
├── mod.rs              # 模块入口，导出API
├── mlibrary.rs         # MLibrary 核心实现 (~2,500行)
└── libraries.rs        # 全局库管理 (~600行)
```

### 架构层次

```
ECS RenderSystem (渲染)
        ↓
   draw_sprite_xxx() (辅助函数)
        ↓
   Libraries (全局管理)
        ↓
   MLibrary (单个库)
        ↓
   ImageInfo (图像信息)
        ↓
   GGEZ Image (GPU纹理)
```

### 数据流向

#### 加载流程

```
游戏启动
    ↓
initialize_all_libraries()
    ↓
load_core_libraries() (加载必需库)
    ↓
Libraries 注册
    ↓
按需加载其他库
    ↓
load_library(name)
    ↓
MLibrary::load(path)
    ↓
解析 .lib 文件
    ↓
ImageInfo 存储
```

#### 渲染流程

```
RenderSystem
    ↓
get_library(name)
    ↓
MLibrary::get_or_create_texture(ctx, index)
    ↓
检查缓存
    ↓ (未缓存)
从 ImageInfo 创建 GGEZ Image
    ↓
缓存纹理
    ↓
返回 ImageInfo (包含 GGEZ Image)
```

---

## 🔧 核心组件

### 1. MLibrary (mlibrary.rs)

**职责**: 单个图像库的管理

#### 核心结构

```rust
pub struct MLibrary {
    /// 库文件名
    file_name: String,
    
    /// 所有图像信息
    images: Vec<ImageInfo>,
    
    /// 是否已初始化
    initialized: bool,
}

pub struct ImageInfo {
    /// GGEZ 图像（纹理）
    pub image: Option<ggez::graphics::Image>,
    
    /// 图像宽度
    pub width: i32,
    
    /// 图像高度
    pub height: i32,
    
    /// X 偏移量
    pub x: i16,
    
    /// Y 偏移量
    pub y: i16,
    
    /// 原始图像数据（用于延迟加载）
    data: Option<Vec<u8>>,
    
    /// 是否有阴影
    pub has_shadow: bool,
    
    /// 阴影 X 偏移
    pub shadow_x: i16,
    
    /// 阴影 Y 偏移
    pub shadow_y: i16,
    
    /// 阴影宽度
    pub shadow_width: i32,
    
    /// 阴影高度
    pub shadow_height: i32,
}
```

#### 主要方法

```rust
impl MLibrary {
    /// 创建新的库（空）
    pub fn new(file_name: String) -> Self;
    
    /// 从文件加载库
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self>;
    
    /// 获取图像数量
    pub fn image_count(&self) -> usize;
    
    /// 获取或创建纹理
    /// 
    /// 如果纹理已缓存，直接返回
    /// 否则从原始数据创建 GGEZ Image 并缓存
    pub fn get_or_create_texture(
        &mut self,
        ctx: &mut ggez::Context,
        index: usize,
    ) -> Result<&ImageInfo>;
    
    /// 获取图像信息（不创建纹理）
    pub fn get_image_info(&self, index: usize) -> Option<&ImageInfo>;
    
    /// 预加载所有纹理
    pub fn preload_all_textures(&mut self, ctx: &mut ggez::Context) -> Result<()>;
    
    /// 清除所有纹理缓存
    pub fn clear_textures(&mut self);
}
```

#### .lib 文件格式

传奇图像库格式：

```
+----------------+
| Header         |  文件头
+----------------+
| Index Table    |  索引表
+----------------+
| Image Data 0   |  图像数据
| Image Data 1   |
| ...            |
+----------------+
```

**Header**:
```rust
struct LibHeader {
    count: i32,           // 图像数量
    // ... 其他字段
}
```

**Index Entry**:
```rust
struct ImageIndex {
    width: i32,           // 图像宽度
    height: i32,          // 图像高度
    x: i16,               // X 偏移
    y: i16,               // Y 偏移
    has_shadow: bool,     // 是否有阴影
    shadow_x: i16,        // 阴影 X 偏移
    shadow_y: i16,        // 阴影 Y 偏移
    length: i32,          // 数据长度
    offset: i32,          // 数据偏移
}
```

**Image Data**:
```
压缩的 ARGB 数据（自定义格式）
```

#### 特性

- ✅ .lib 文件解析
- ✅ 延迟加载（按需创建纹理）
- ✅ 纹理缓存
- ✅ 阴影信息
- ✅ 偏移量支持
- ✅ GGEZ 集成

### 2. Libraries (libraries.rs)

**职责**: 全局图像库管理

#### 核心结构

```rust
/// 库名枚举
pub enum LibraryName {
    // UI 库
    Prguse,
    Prguse2,
    Prguse3,
    ChrSel,
    
    // 地图库
    MapTiles(u8),    // 0-99
    SmTiles(u8),     // 0-99
    Objects(u8),     // 0-99
    
    // 角色库
    Hum,
    Hum2,
    Hum3,
    Hair,
    Weapon,
    Weapon2,
    
    // 怪物库
    Mon(u8),         // 1-50
    
    // NPC 库
    Npc(u8),         // 1-50
    
    // 特效库
    Magic,
    Magic2,
    Magic3,
    Effect,
    
    // 其他
    Title,
    MMapTiles,
    MiniMap,
}

/// 库数组（用于同类型多个库）
pub enum LibraryArray {
    MapTiles,
    SmTiles,
    Objects,
    Mon,
    Npc,
}

/// 全局库管理器
pub struct Libraries {
    /// 数据路径
    data_path: PathBuf,
    
    /// 已加载的库
    libraries: HashMap<String, Arc<Mutex<MLibrary>>>,
}
```

#### 全局实例

```rust
/// 全局库管理器（单例）
pub static LIBRARIES: Lazy<Mutex<Libraries>> = Lazy::new(|| {
    Mutex::new(Libraries::new())
});
```

#### 主要方法

```rust
/// 设置数据路径
pub fn set_data_path<P: AsRef<Path>>(path: P);

/// 初始化所有库（创建空库）
pub fn initialize_all_libraries();

/// 加载核心库（必需的库）
pub fn load_core_libraries(ctx: &mut ggez::Context) -> Result<()>;

/// 加载所有库
pub fn load_all_libraries(ctx: &mut ggez::Context) -> Result<()>;

/// 加载单个库
pub fn load_library(name: LibraryName) -> Result<()>;

/// 获取库
pub fn get_library(name: LibraryName) -> Option<Arc<Mutex<MLibrary>>>;

/// 获取库（从数组）
pub fn get_library_from_array(
    array: LibraryArray,
    index: u8
) -> Option<Arc<Mutex<MLibrary>>>;

/// 获取地图库（智能选择）
pub fn get_map_library(
    map_index: i32
) -> Option<Arc<Mutex<MLibrary>>>;

/// 获取所有地图库
pub fn get_all_map_libraries(
    map_index: i32
) -> (
    Option<Arc<Mutex<MLibrary>>>,  // MapTiles
    Option<Arc<Mutex<MLibrary>>>,  // SmTiles
    Option<Arc<Mutex<MLibrary>>>,  // Objects
);

/// 检查库是否已加载
pub fn is_library_loaded(name: LibraryName) -> bool;
```

#### 库命名规则

| 类型 | 规则 | 示例 |
|------|------|------|
| UI | 固定名称 | `Prguse.lib`, `Prguse2.lib` |
| 地图瓦片 | `Tiles{index}.lib` | `Tiles0.lib`, `Tiles30.lib` |
| 小地图 | `SmTiles{index}.lib` | `SmTiles0.lib` |
| 地图物件 | `Objects{index}.lib` | `Objects0.lib`, `Objects5.lib` |
| 怪物 | `Mon{n}.lib` | `Mon1.lib`, `Mon16.lib` |
| NPC | `Npc{n}.lib` | `Npc1.lib`, `Npc99.lib` |
| 角色 | 固定名称 | `Hum.lib`, `Hum2.lib`, `Hair.lib` |
| 武器 | 固定名称 | `Weapon.lib`, `Weapon2.lib` |
| 魔法 | 固定名称 | `Magic.lib`, `Magic2.lib` |

---

## 📦 图像库系统

### 库分类

#### 1. UI 库（4个）

| 库名 | 用途 | 文件 |
|------|------|------|
| **Prguse** | 主UI界面 | `Prguse.lib` |
| **Prguse2** | 扩展UI | `Prguse2.lib` |
| **Prguse3** | 更多UI | `Prguse3.lib` |
| **ChrSel** | 角色选择界面 | `ChrSel.lib` |

**内容示例**:
- 按钮（普通/悬停/按下）
- 对话框边框
- 图标（物品/技能/状态）
- 数字字体
- 进度条
- 装饰元素

#### 2. 地图库（每类50+个）

**MapTiles** - 地图瓦片（背景和中层）

```rust
LibraryName::MapTiles(0)   // 地图0的瓦片 (Tiles0.lib)
LibraryName::MapTiles(30)  // 地图30的瓦片 (Tiles30.lib)
```

**SmTiles** - 小地图瓦片（前景）

```rust
LibraryName::SmTiles(0)    // 地图0的小瓦片 (SmTiles0.lib)
```

**Objects** - 地图物件（建筑/树木等）

```rust
LibraryName::Objects(0)    // 地图0的物件 (Objects0.lib)
LibraryName::Objects(5)    // 地图5的物件 (Objects5.lib)
```

#### 3. 角色库（7个）

| 库名 | 用途 | 文件 |
|------|------|------|
| **Hum** | 男性角色 | `Hum.lib` |
| **Hum2** | 女性角色 | `Hum2.lib` |
| **Hum3** | 其他角色 | `Hum3.lib` |
| **Hair** | 发型 | `Hair.lib` |
| **Weapon** | 武器 | `Weapon.lib` |
| **Weapon2** | 更多武器 | `Weapon2.lib` |

**动画帧结构**:
```
站立: 4方向 × 4帧
行走: 8方向 × 6帧
奔跑: 8方向 × 6帧
攻击: 8方向 × 6帧
受击: 8方向 × 2帧
死亡: 8方向 × 10帧
```

#### 4. 怪物库（50个）

```rust
LibraryName::Mon(1)    // 鹿 (Mon1.lib)
LibraryName::Mon(2)    // 稻草人 (Mon2.lib)
LibraryName::Mon(16)   // 沃玛战士 (Mon16.lib)
// ... Mon1.lib ~ Mon50.lib
```

**常见怪物**:
- Mon1: 鹿
- Mon2: 稻草人
- Mon3: 鸡
- Mon5: 森林雪人
- Mon16: 沃玛战士
- Mon28: 祖玛雕像

#### 5. NPC库（50个）

```rust
LibraryName::Npc(1)    // 新手村长老 (Npc1.lib)
LibraryName::Npc(3)    // 武器店老板 (Npc3.lib)
// ... Npc1.lib ~ Npc99.lib
```

#### 6. 特效库（4个）

| 库名 | 用途 | 文件 |
|------|------|------|
| **Magic** | 基础魔法特效 | `Magic.lib` |
| **Magic2** | 扩展魔法 | `Magic2.lib` |
| **Magic3** | 更多魔法 | `Magic3.lib` |
| **Effect** | 其他特效 | `Effect.lib` |

**特效类型**:
- 火球术
- 闪电术
- 治愈术
- 召唤术
- 爆炸效果
- 光环效果
- 传送效果

### 加载策略

#### 核心库（游戏启动时加载）

```rust
pub fn load_core_libraries(ctx: &mut ggez::Context) -> Result<()> {
    // UI 库（必需）
    load_library(LibraryName::Prguse)?;
    load_library(LibraryName::Prguse2)?;
    load_library(LibraryName::ChrSel)?;
    
    // 基础角色库
    load_library(LibraryName::Hum)?;
    load_library(LibraryName::Hum2)?;
    
    Ok(())
}
```

#### 按需加载

```rust
// 进入地图时加载对应的地图库
fn enter_map(map_index: i32) {
    load_library(LibraryName::MapTiles(map_index as u8));
    load_library(LibraryName::SmTiles(map_index as u8));
    load_library(LibraryName::Objects(map_index as u8));
}

// 遇到新怪物时加载
fn spawn_monster(monster_image: u8) {
    load_library(LibraryName::Mon(monster_image));
}
```

---

## 📖 使用指南

### 初始化

```rust
use crate::graphics::*;

fn main() -> GameResult {
    // 1. 设置数据路径
    set_data_path("./Data");
    
    // 2. 初始化所有库（创建空库）
    initialize_all_libraries();
    
    // 3. 创建 GGEZ 上下文
    let (mut ctx, event_loop) = ggez::ContextBuilder::new("Mir2", "author")
        .build()?;
    
    // 4. 加载核心库
    load_core_libraries(&mut ctx)?;
    
    // 5. 开始游戏
    // ...
    
    Ok(())
}
```

### 获取和使用图像

#### 方式1: 直接获取库

```rust
// 获取库
if let Some(library) = get_library(LibraryName::Prguse) {
    let mut lib = library.lock().unwrap();
    
    // 获取或创建纹理
    if let Ok(image_info) = lib.get_or_create_texture(&mut ctx, 100) {
        if let Some(image) = &image_info.image {
            // 渲染图像
            canvas.draw(
                image,
                DrawParam::default().dest([x, y])
            );
        }
    }
}
```

#### 方式2: 使用辅助函数

```rust
// 简单绘制（不带偏移）
draw_sprite_at(
    &mut ctx,
    &mut canvas,
    &LibraryName::Prguse,
    100,
    x,
    y
)?;

// 带偏移绘制
draw_sprite_with_offset(
    &mut ctx,
    &mut canvas,
    &LibraryName::Prguse,
    100,
    x,
    y
)?;
```

### 加载地图资源

```rust
// 进入地图
fn enter_map(ctx: &mut ggez::Context, map_index: i32) -> Result<()> {
    // 获取地图相关的所有库
    let (tiles_lib, sm_tiles_lib, objects_lib) = 
        get_all_map_libraries(map_index);
    
    // 预加载纹理（可选）
    if let Some(lib) = tiles_lib {
        let mut lib = lib.lock().unwrap();
        lib.preload_all_textures(ctx)?;
    }
    
    Ok(())
}
```

### 加载怪物资源

```rust
// 生成怪物
fn spawn_monster(monster_image: u8) -> Result<()> {
    // 加载怪物库（如果未加载）
    if !is_library_loaded(LibraryName::Mon(monster_image)) {
        load_library(LibraryName::Mon(monster_image))?;
    }
    
    // 获取怪物库
    let library = get_library(LibraryName::Mon(monster_image))
        .ok_or_else(|| anyhow!("Failed to load monster library"))?;
    
    Ok(())
}
```

### 从数组获取库

```rust
// 获取地图瓦片库（编号30）
let library = get_library_from_array(
    LibraryArray::MapTiles,
    30
);

// 获取怪物库（编号16）
let library = get_library_from_array(
    LibraryArray::Mon,
    16
);
```

### 智能地图库选择

```rust
// 自动选择正确的地图库
let library = get_map_library(map_index);

// 等价于：
let library = if map_index < 100 {
    get_library(LibraryName::MapTiles(map_index as u8))
} else {
    // 处理更大的地图索引
    // ...
};
```

---

## 📊 开发状态

### 完成度统计

| 功能模块 | 完成度 | 说明 |
|---------|--------|------|
| **MLibrary** | 100% | .lib解析、纹理管理完成 |
| **Libraries** | 100% | 全局库管理完成 |
| **延迟加载** | 100% | 按需加载纹理 |
| **纹理缓存** | 100% | 缓存优化完成 |
| **偏移支持** | 100% | 渲染偏移完成 |
| **阴影信息** | 100% | 阴影数据解析完成 |
| **粒子引擎** | 0% | 待实现 |

### 已实现功能清单

#### ✅ 核心功能

- [x] .lib 文件解析
- [x] 图像数据解压
- [x] GGEZ 纹理创建
- [x] 纹理缓存
- [x] 延迟加载
- [x] 偏移量管理
- [x] 阴影信息

#### ✅ 库管理

- [x] 全局库管理器
- [x] 库名枚举
- [x] 库数组支持
- [x] 智能库选择
- [x] 加载状态查询

#### ✅ 优化

- [x] 按需加载
- [x] 纹理缓存
- [x] 批量预加载
- [x] 内存管理

#### ✅ 辅助功能

- [x] 简单绘制函数
- [x] 带偏移绘制函数
- [x] 库存在性检查

### 未实现功能清单

#### ⏳ 粒子引擎

- [ ] **粒子发射器**: 粒子生成和管理
- [ ] **粒子系统**: 生命周期、物理、渲染
- [ ] **预设特效**: 爆炸、火焰、闪电等
- [ ] **性能优化**: 对象池、批量渲染

#### ⏳ 高级功能

- [ ] **纹理压缩**: GPU压缩格式支持
- [ ] **Mipmap**: 多级纹理
- [ ] **纹理图集**: 合并小纹理
- [ ] **异步加载**: 后台加载资源

#### ⏳ 优化

- [ ] **内存优化**: 更智能的缓存策略
- [ ] **加载优化**: 并行加载
- [ ] **渲染优化**: 批处理、实例化

---

## 🚀 未来规划

### 短期目标 (1-2周)

1. **纹理图集** 🟡 中优先级
   - 合并小纹理到大纹理
   - 减少Draw Call
   - 优化GPU性能

2. **异步加载** 🟡 中优先级
   - 后台加载资源
   - 不阻塞主线程
   - 加载进度显示

3. **内存优化** 🟡 中优先级
   - LRU缓存策略
   - 自动卸载未使用的库
   - 内存使用监控

### 中期目标 (3-4周)

4. **粒子引擎** 🔴 高优先级
   - 设计粒子系统架构
   - 实现基础粒子发射器
   - 添加预设特效
   - 性能优化（对象池）

5. **纹理压缩** 🟢 低优先级
   - 支持GPU压缩格式
   - 减少内存占用
   - 提高加载速度

6. **Mipmap支持** 🟢 低优先级
   - 生成多级纹理
   - 优化远距离渲染
   - 减少锯齿

### 长期目标 (1-2月)

7. **渲染优化**
   - 批量渲染
   - 实例化渲染
   - GPU加速

8. **资源编辑器**
   - 图像库查看器
   - 图像导入/导出
   - 偏移量编辑

---

## 🐛 已知问题

### 高优先级

- [ ] 大量纹理创建时可能内存溢出
- [ ] 首次加载大地图时卡顿

### 中优先级

- [ ] 未使用的纹理不会自动卸载
- [ ] 阴影数据已解析但未使用

### 低优先级

- [ ] 部分库的索引超出范围时处理不优雅
- [ ] 加载进度无法显示

---

## 🔧 性能考虑

### 内存使用

**估算**:

```
单个纹理: 512×512×4 = 1MB
地图库(50个地图): ~500MB
怪物库(50个): ~200MB
其他库: ~100MB
总计: ~800MB
```

**优化策略**:

1. **延迟加载**: 只加载当前需要的库
2. **纹理缓存**: 缓存常用纹理
3. **LRU策略**: 自动卸载最少使用的纹理
4. **纹理压缩**: 使用GPU压缩格式

### 加载性能

**当前性能**:

- 单个库加载: ~100ms
- 核心库加载: ~500ms
- 地图库加载: ~200ms

**优化策略**:

1. **并行加载**: 同时加载多个库
2. **异步加载**: 后台加载
3. **预加载**: 提前加载可能需要的库

### 渲染性能

**当前性能**:

- 单个精灵: ~0.01ms
- 1000个精灵: ~10ms
- 瓶颈: Draw Call 数量

**优化策略**:

1. **批量渲染**: 合并相同纹理的Draw Call
2. **实例化渲染**: GPU实例化
3. **纹理图集**: 减少纹理切换

---

## 📝 最佳实践

### 资源加载

```rust
// ✅ 正确：按需加载
fn enter_map(map_index: i32) {
    load_library(LibraryName::MapTiles(map_index as u8));
}

// ❌ 错误：一次加载所有
fn init() {
    for i in 0..100 {
        load_library(LibraryName::MapTiles(i));
    }
}
```

### 纹理使用

```rust
// ✅ 正确：缓存库引用
let library = get_library(LibraryName::Prguse).unwrap();
for i in 0..100 {
    let mut lib = library.lock().unwrap();
    let image = lib.get_or_create_texture(ctx, i)?;
    // 使用图像
}

// ❌ 错误：重复获取库
for i in 0..100 {
    let library = get_library(LibraryName::Prguse).unwrap();
    // ...
}
```

### 错误处理

```rust
// ✅ 正确：优雅处理缺失资源
if let Some(library) = get_library(LibraryName::Mon(monster_image)) {
    // 使用库
} else {
    tracing::warn!("Monster library {} not found", monster_image);
    // 使用默认图像
}

// ❌ 错误：直接unwrap
let library = get_library(LibraryName::Mon(monster_image)).unwrap();
```

---

## 🔗 相关文档

### 内部文档

- **ECS系统**: `../ecs/systems/README.md` - 渲染系统使用图形模块
- **对象系统**: `../objects/README.md` - 对象的图像信息

### 外部资源

- **GGEZ文档**: https://ggez.rs/ - GGEZ图形引擎
- **.lib格式**: 传奇图像库格式说明

---

## 💡 技术细节

### .lib 文件解析

```rust
// 读取文件头
let count = reader.read_i32()?;

// 读取索引表
for i in 0..count {
    let width = reader.read_i32()?;
    let height = reader.read_i32()?;
    let x = reader.read_i16()?;
    let y = reader.read_i16()?;
    let has_shadow = reader.read_u8()? != 0;
    // ...
}

// 读取图像数据
for index in &indices {
    reader.seek(index.offset)?;
    let compressed_data = reader.read_bytes(index.length)?;
    let image_data = decompress(compressed_data)?;
    // ...
}
```

### 纹理创建

```rust
// 从原始数据创建GGEZ Image
let image = ggez::graphics::Image::from_pixels(
    ctx,
    &image_data,      // RGBA数据
    ImageFormat::Rgba8UnormSrgb,
    width as u32,
    height as u32,
);
```

### 偏移量应用

```rust
// 渲染时应用偏移
let render_x = screen_x + image_info.x as f32;
let render_y = screen_y + image_info.y as f32;

canvas.draw(
    image,
    DrawParam::default().dest([render_x, render_y])
);
```

---

**文档版本**: v1.0  
**最后更新**: 2025-10-28  
**维护者**: Crystal Mir2 Team
