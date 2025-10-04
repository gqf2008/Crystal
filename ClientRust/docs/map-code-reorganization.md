# Map 模块重组记录

**日期**: 2025-10-04  
**操作**: 将地图相关代码重组为符合 C# Client 结构

---

## 改动说明

### 1. 创建 `src/objects/map_code.rs`

**对应 C# 文件**: `Client/MirObjects/MapCode.cs`

包含两个核心类：

#### **CellInfo** (格子信息)
```rust
pub struct CellInfo {
    // 地形层
    pub back_index: i16,
    pub back_image: i32,
    pub middle_index: i16,
    pub middle_image: i32,
    pub front_index: i16,
    pub front_image: i32,
    
    // 门和动画
    pub door_index: u8,
    pub door_offset: u8,
    pub front_animation_frame: u8,
    pub front_animation_tick: u8,
    pub middle_animation_frame: u8,
    pub middle_animation_tick: u8,
    
    // 对象管理
    pub cell_objects: Option<Vec<u32>>, // ObjectID 列表
    
    // 其他属性
    pub fishing_cell: bool,
    ...
}
```

**方法**:
- `add_object(object_id)` - 添加对象到格子
- `remove_object(object_id)` - 从格子移除对象
- `find_object(object_id)` - 查找对象是否在格子中
- `sort()` - 排序对象（简化版，完整逻辑在 GameScene）

#### **MapReader** (地图加载器)
```rust
pub struct MapReader {
    pub width: i32,
    pub height: i32,
    pub map_cells: Vec<Vec<CellInfo>>, // 地图格子数组
    file_name: String,
    bytes: Vec<u8>,
}
```

**方法**:
- `new(file_name)` - 构造函数，自动加载地图
- `initiate()` - 初始化，检测格式并加载
- `detect_and_load()` - 检测地图格式（9种格式）
- `load_map_type_0()` - 加载老格式 (12 bytes/cell) ✅
- `load_map_type_1()` - 加载 Map 2010 (14 bytes/cell) ✅
- `load_map_type_2()` - 加载旧 Shanda (10 bytes/cell) ✅
- `load_map_type_3()` - 加载 Shanda 2012 (14 bytes/cell) ✅
- `load_map_type_4()` - Wemade AntiHack ⏳ (TODO)
- `load_map_type_5()` - Wemade Mir3 ⏳ (TODO)
- `load_map_type_6()` - Shanda Mir3 ⏳ (TODO)
- `load_map_type_7()` - 3/4 Heroes ⏳ (TODO)
- `load_map_type_100()` - C# 自定义 ⏳ (TODO)
- `get_cell(x, y)` - 获取指定位置的格子

### 2. 更新 `src/objects/mod.rs`

添加：
```rust
mod map_code;
pub use map_code::{MapReader, CellInfo};
```

### 3. 标记 `src/map/` 为已弃用

更新 `src/map/README.md`，说明：
- 本目录下的重构版本不再使用
- 功能已移至 `src/objects/map_code.rs`
- 代码保留仅供参考

---

## 与 C# 的对应关系

| Rust | C# |
|------|-----|
| `src/objects/map_code.rs::CellInfo` | `Client/MirObjects/MapCode.cs::CellInfo` |
| `src/objects/map_code.rs::MapReader` | `Client/MirObjects/MapCode.cs::MapReader` |
| (待实现) `src/scenes/game_scene.rs::MapControl` | `Client/MirScenes/GameScene.cs::MapControl` |

---

## 设计差异说明

### C# 的设计
```csharp
class CellInfo {
    public List<MapObject> CellObjects;  // 直接存对象引用
}

class MapControl {
    public static int OffSetX, OffSetY;  // 相机作为静态变量
    public CellInfo[,] M2CellInfo;       // 二维数组
    
    void DrawFloor() {
        // 地图渲染逻辑直接在这里
    }
}
```

### Rust 的移植
```rust
struct CellInfo {
    pub cell_objects: Option<Vec<u32>>,  // 存 ObjectID，不是对象本身
}

struct MapReader {
    pub map_cells: Vec<Vec<CellInfo>>,   // Vec<Vec>，不是二维数组
}

// MapControl 的功能将在 GameScene 中实现
struct GameScene {
    map_reader: Option<MapReader>,
    offset_x: i32,    // 对应 OffSetX
    offset_y: i32,    // 对应 OffSetY
    view_range_x: i32,
    view_range_y: i32,
    // ...
}
```

**为什么 CellInfo 不直接存对象？**
- Rust 的所有权系统不允许多个地方同时拥有对象
- 对象需要在 GameScene 中统一管理
- 格子只存 ObjectID，通过 ID 查找实际对象

---

## 测试状态

✅ **编译成功**  
✅ **单元测试通过** (2/2)
- `test_cell_info_creation` - CellInfo 创建测试
- `test_cell_info_add_remove_object` - 对象添加/移除测试

---

## 下一步工作

### 1. 完成 MapReader 的剩余格式 (P1)
- [ ] `load_map_type_4()` - Wemade AntiHack
- [ ] `load_map_type_5()` - Wemade Mir3
- [ ] `load_map_type_6()` - Shanda Mir3
- [ ] `load_map_type_7()` - 3/4 Heroes
- [ ] `load_map_type_100()` - C# 自定义

### 2. 在 GameScene 中集成 (P0 - 必须)
```rust
impl GameScene {
    fn load_map(&mut self, map_path: &str) {
        self.map_reader = MapReader::new(map_path).ok();
        
        // 初始化相机参数（对应 MapControl 构造函数）
        self.offset_x = SCREEN_WIDTH / 2 / CELL_WIDTH;
        self.offset_y = SCREEN_HEIGHT / 2 / CELL_HEIGHT - 1;
        self.view_range_x = self.offset_x + 6;
        self.view_range_y = self.offset_y + 6;
    }
    
    fn draw_floor(&mut self, /* ... */) {
        // 对应 MapControl.DrawFloor()
        // 遍历可见范围的格子
        // 调用 MLibrary 渲染瓦片
    }
}
```

### 3. 实现对象管理 (P0 - 必须)
```rust
impl GameScene {
    fn add_object_to_cell(&mut self, x: i32, y: i32, object_id: u32) {
        if let Some(ref mut map) = self.map_reader {
            if let Some(cell) = map.get_cell_mut(x, y) {
                cell.add_object(object_id);
            }
        }
    }
    
    fn remove_object_from_cell(&mut self, x: i32, y: i32, object_id: u32) {
        // 类似实现
    }
}
```

---

## 命名规范

**遵循 C# Client 的命名**:
- ✅ `CellInfo` (不是 `Cell`)
- ✅ `MapReader` (不是 `MapLoader` 或 `MapData`)
- ✅ `cell_objects` (不是 `objects` 或 `entities`)
- ✅ `back_image`, `middle_image`, `front_image` (保持 C# 的术语)

**原则**: 命名与 C# 保持一致，方便人类排错和对照源码

---

## 参考文档

- C# 源文件: `Client/MirObjects/MapCode.cs` (675 lines)
- C# 源文件: `Client/MirScenes/GameScene.cs` (MapControl 类, line 10060+)
- 移植路线图: `docs/DIRECT_MIGRATION_ROADMAP.md`

---

**文档版本**: v1.0  
**状态**: ✅ 完成基础移植  
**下一步**: 完成剩余地图格式 + GameScene 集成
