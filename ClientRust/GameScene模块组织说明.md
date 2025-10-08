# GameScene 模块组织说明

## C# 与 Rust 架构对比

### C# 架构 (单文件嵌套类)

```csharp
// Client/MirScenes/GameScene.cs (12,294 行)
public sealed class GameScene : MirScene {
    // GameScene 主体 (line 27-10207)
    public static UserObject User;
    public static Dictionary<uint, MapObject> Objects;
    // ...
    
    // MapControl 嵌套类 (line 10209-11241)
    public sealed class MapControl : MirControl {
        public M2CellInfo[,] M2CellInfo;
        // ...
    }
}
```

**优点**: 
- 逻辑上 MapControl 是 GameScene 的一部分,嵌套体现了从属关系

**缺点**:
- 单文件超过 12,000 行,难以维护
- MapControl 逻辑复杂(1,000+ 行),混在一起降低可读性

---

### Rust 架构 (模块化)

```
ClientRust/src/scenes/
├── game_scene_v2.rs              # GameScene 主结构 (~800 行)
└── game_scene/                   # GameScene 子模块
    ├── mod.rs                    # 模块导出
    ├── map_control.rs            # MapControl 实现 (~300 行)
    ├── cell_info.rs              # (使用 objects::CellInfo)
    └── tile_texture_manager.rs   # 纹理管理 (~200 行)
```

**导入方式**:
```rust
// game_scene_v2.rs
use crate::scenes::game_scene::MapControl;
use crate::objects::CellInfo;

pub struct GameScene {
    map_control: Option<MapControl>,
    // ...
}
```

**优点**:
- ✅ **职责清晰**: 每个文件专注一个功能模块
- ✅ **易于维护**: 文件大小适中(200-800 行)
- ✅ **便于测试**: 可以单独测试 MapControl
- ✅ **避免重复**: 不会在多个文件中重复定义结构
- ✅ **符合 Rust 惯例**: Rust 推荐用模块系统而非嵌套类型

**从属关系保持**:
虽然是独立模块,但通过命名空间体现从属:
- `crate::scenes::game_scene::MapControl` (清楚表明属于 game_scene)
- `GameScene` 持有 `Option<MapControl>` 字段

---

## 为什么不在一个文件里?

### 1. Rust 没有"嵌套类"概念

C# 的嵌套类:
```csharp
public class Outer {
    public class Inner { }  // 嵌套类
}
```

Rust 只能嵌套模块:
```rust
pub struct Outer {}

// ❌ 不能在 struct 内定义 struct
// pub struct Outer {
//     pub struct Inner {}  // 编译错误!
// }

// ✅ 正确做法: 使用模块
pub mod outer {
    pub struct Outer {}
}
pub mod inner {
    pub struct Inner {}
}
```

### 2. 单文件 12,000 行不符合 Rust 生态

Rust 社区推荐:
- 单文件不超过 1,000 行
- 复杂模块拆分为多个文件
- 用 `mod.rs` 或 `module_name/mod.rs` 组织子模块

参考 Rust 标准库:
```
std/
├── collections/
│   ├── mod.rs
│   ├── hash_map.rs
│   ├── hash_set.rs
│   └── vec_deque.rs
└── io/
    ├── mod.rs
    ├── buffered.rs
    └── cursor.rs
```

### 3. 便于并行开发

模块化后:
- 开发者 A 可以专注 `game_scene_v2.rs` (GameScene 逻辑)
- 开发者 B 可以专注 `map_control.rs` (地图渲染)
- Git 冲突减少

### 4. 测试隔离

```rust
// tests/map_control_tests.rs
use client_rust::scenes::game_scene::MapControl;

#[test]
fn test_draw_floor() {
    let map = MapControl::new();
    // 只测试 MapControl,不依赖 GameScene
}
```

---

## 模块组织原则总结

### ✅ 遵循的原则

1. **Single Responsibility (单一职责)**:
   - `game_scene_v2.rs`: GameScene 主逻辑
   - `map_control.rs`: 地图渲染
   - `tile_texture_manager.rs`: 纹理缓存

2. **DRY (Don't Repeat Yourself)**:
   - MapControl 只在 `map_control.rs` 定义一次
   - CellInfo 复用 `objects::CellInfo`

3. **Rust Conventions**:
   - 使用模块系统而非"嵌套类型"
   - 文件大小适中(200-800 行)
   - 公共类型通过 `pub use` 重导出

### ❌ 避免的反模式

1. **❌ 在多个文件重复定义**: 
   - 之前 `game_scene_v2.rs` 和 `map_control.rs` 都定义了 MapControl

2. **❌ 巨型单文件**: 
   - 单文件超过 2,000 行难以维护

3. **❌ 循环依赖**: 
   - 模块 A 依赖 B,B 又依赖 A (通过精心设计避免)

---

## 实际效果对比

### Before (单文件混合)
```rust
// game_scene_v2.rs (1,500+ 行)
pub struct GameScene { /* ... */ }
pub struct MapControl { /* ... */ }  // ❌ 重复定义
pub struct M2CellInfo { /* ... */ }  // ❌ 重复定义
impl GameScene { /* ... */ }
impl MapControl { /* ... */ }
```

### After (模块化)
```rust
// game_scene_v2.rs (~800 行)
use crate::scenes::game_scene::MapControl;
use crate::objects::CellInfo;

pub struct GameScene {
    map_control: Option<MapControl>,
    // ...
}

// game_scene/map_control.rs (~300 行)
pub struct MapControl {
    cells: Vec<Vec<CellInfo>>,
    // ...
}
```

---

## 总结

虽然 C# 中 MapControl 是 GameScene 的嵌套类,但在 Rust 中:**使用子模块是更好的实践**。

**关键优势**:
- ✅ 符合 Rust 语言特性(无嵌套类)
- ✅ 文件大小适中,易于导航
- ✅ 单一职责,便于测试
- ✅ 避免重复定义
- ✅ 保持逻辑从属关系(`game_scene::MapControl`)

这不是"丢失"了 C# 的嵌套结构,而是用 Rust 的方式**更好地表达**了相同的架构意图。
