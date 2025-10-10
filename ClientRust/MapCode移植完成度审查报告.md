# MapCode.cs 移植完成度审查报告

**审查日期**: 2025-10-10  
**源文件**: `Client/MirObjects/MapCode.cs` (617 lines)  
**目标文件**: `ClientRust/src/objects/map_code.rs` (831 lines)  
**总体完成度**: **60%** ⚠️

---

## 📋 执行摘要

### 关键发现

✅ **已完成部分** (60%):
- CellInfo 结构完整移植 (100%)
- MapReader 基础架构 (100%)
- 3种地图格式加载器 (Type 0, 1, 2, 3, 100)
- 对象管理基础方法 (add_object, remove_object, find_object)

⚠️ **未完成部分** (40%):
- 5种地图格式未实现 (Type 4, 5, 6, 7 - 返回 Unsupported 错误)
- Sort 排序逻辑未实现 (仅占位符)
- draw_objects 实现简化 (缺少特殊怪物处理)
- is_walkable 逻辑简化 (未考虑所有情况)

### 建议优先级

🔴 **高优先级** (必需):
- ✅ Type 0 (旧格式) - 已实现
- ✅ Type 100 (C#自定义) - 已实现
- ⚠️ Sort 排序逻辑 - 影响渲染顺序

🟡 **中优先级** (常用):
- ✅ Type 1 (2010格式) - 已实现
- ✅ Type 2 (Shanda老版本) - 已实现
- ✅ Type 3 (Shanda 2012) - 已实现

🟢 **低优先级** (可选):
- ❌ Type 4 (AntiHack迷宫)
- ❌ Type 5 (Mir3 Wemade)
- ❌ Type 6 (Mir3 Shanda)
- ❌ Type 7 (3/4 Heroes)

---

## 🔍 详细对比分析

### 1. CellInfo 结构

#### C# 实现 (MapCode.cs lines 9-126)

```csharp
public class CellInfo
{
    // ==================== Back Layer (地表层) ====================
    public short BackIndex;
    public int BackImage;
    
    // ==================== Middle Layer (建筑层) ====================
    public short MiddleIndex;
    public int MiddleImage;
    
    // ==================== Front Layer (前景层) ====================
    public short FrontIndex;
    public int FrontImage;

    // ==================== 门系统 ====================
    public byte DoorIndex;
    public byte DoorOffset;

    // ==================== Front层动画 ====================
    public byte FrontAnimationFrame;
    public byte FrontAnimationTick;

    // ==================== Middle层动画 ====================
    public byte MiddleAnimationFrame;
    public byte MiddleAnimationTick;

    // ==================== Shanda瓦片动画 (流水/岩浆等) ====================
    public short TileAnimationImage;
    public short TileAnimationOffset;
    public byte  TileAnimationFrames;

    // ==================== 其他属性 ====================
    public byte Light;
    public byte Unknown;
    public List<MapObject> CellObjects;
    public Boolean FishingCell;

    // 方法
    public void AddObject(MapObject ob);
    public void RemoveObject(MapObject ob);
    public MapObject FindObject(uint ObjectID);
    public void DrawObjects();
    public void DrawDeadObjects();
    public void Sort();
}
```

#### Rust 实现 (map_code.rs lines 18-180)

```rust
#[derive(Debug, Clone)]
pub struct CellInfo {
    // 背景层
    pub back_index: i16,      // ✅
    pub back_image: i32,       // ✅
    
    // 中间层
    pub middle_index: i16,     // ✅
    pub middle_image: i32,     // ✅
    
    // 前景层
    pub front_index: i16,      // ✅
    pub front_image: i32,      // ✅
    
    // 门相关
    pub door_index: u8,        // ✅
    pub door_offset: u8,       // ✅
    
    // 动画相关
    pub front_animation_frame: u8,   // ✅
    pub front_animation_tick: u8,    // ✅
    pub middle_animation_frame: u8,  // ✅
    pub middle_animation_tick: u8,   // ✅
    
    pub tile_animation_image: i16,   // ✅
    pub tile_animation_offset: i16,  // ✅
    pub tile_animation_frames: u8,   // ✅
    
    // 光照
    pub light: u8,             // ✅
    pub unknown: u8,           // ✅
    
    // 对象列表
    pub cell_objects: Option<Vec<u32>>,  // ⚠️ 简化版 (存ID而非对象)
    
    // 钓鱼点
    pub fishing_cell: bool,    // ✅
}

impl CellInfo {
    pub fn add_object(&mut self, object_id: u32);      // ✅
    pub fn remove_object(&mut self, object_id: u32);   // ✅
    pub fn find_object(&self, object_id: u32) -> bool; // ✅
    fn sort(&mut self);                                // ⚠️ 占位符
    pub fn draw_objects(...);                          // ⚠️ 简化版
    pub fn draw_dead_objects(...);                     // ⚠️ 简化版
    pub fn is_walkable(&self) -> bool;                 // ⚠️ 简化版
}
```

#### 对比结果

| 字段/方法 | C# | Rust | 状态 | 备注 |
|----------|-----|------|------|------|
| **字段** | 17个 | 17个 | ✅ 完整 | 所有字段已移植 |
| **AddObject** | ✅ | ✅ | ✅ 完整 | 功能等效 |
| **RemoveObject** | ✅ | ✅ | ✅ 完整 | 功能等效 |
| **FindObject** | ✅ | ✅ | ✅ 完整 | 返回类型不同但功能等效 |
| **Sort** | ✅ | ⚠️ | ⚠️ 未实现 | 仅占位符，待实现 |
| **DrawObjects** | ✅ | ⚠️ | ⚠️ 简化 | 缺少特殊怪物处理 |
| **DrawDeadObjects** | ✅ | ⚠️ | ⚠️ 简化 | 缺少特殊怪物过滤 |

#### 架构差异

**C# 设计**:
```csharp
public List<MapObject> CellObjects;  // 直接存储对象引用
```

**Rust 设计**:
```rust
pub cell_objects: Option<Vec<u32>>,  // 存储对象ID
```

**原因**: Rust 的所有权系统不允许多个位置持有同一对象的可变引用。通过存储 ID，实际对象统一在 `GameScene.Objects: HashMap<u32, MapObject>` 中管理。

**优点**:
- ✅ 避免所有权冲突
- ✅ 统一对象管理
- ✅ 更符合 Rust 惯用法

**缺点**:
- ⚠️ 查找对象需要额外的 HashMap 查询
- ⚠️ API 稍显复杂

---

### 2. Sort 方法 - 对象排序逻辑

#### C# 实现 (MapCode.cs lines 115-126)

```csharp
public void Sort()
{
    CellObjects.Sort(delegate(MapObject ob1, MapObject ob2)
    {
        // 1. 掉落物品优先渲染（最底层）
        if (ob1.Race == ObjectType.Item && ob2.Race != ObjectType.Item)
            return -1;
        if (ob2.Race == ObjectType.Item && ob1.Race != ObjectType.Item)
            return 1;
        
        // 2. 法术特效次优先
        if (ob1.Race == ObjectType.Spell && ob2.Race != ObjectType.Spell)
            return -1;
        if (ob2.Race == ObjectType.Spell && ob1.Race != ObjectType.Spell)
            return 1;

        // 3. 死亡对象排在活着的对象之后
        int i = ob2.Dead.CompareTo(ob1.Dead);
        
        // 4. 相同状态按 ObjectID 排序（保证稳定性）
        return i == 0 ? ob1.ObjectID.CompareTo(ob2.ObjectID) : i;
    });
}
```

#### Rust 实现 (map_code.rs line 121)

```rust
fn sort(&mut self) {
    // TODO: 实现对象排序逻辑
    // 注意：C# 中的排序逻辑比较复杂，涉及对象类型、死亡状态等
    // 暂时保留简单版本，后续在 GameScene 中实现完整排序
}
```

#### 状态: ⚠️ **未实现** (需要补充)

**影响**:
- 🎨 渲染顺序错误
- 🐛 掉落物可能被遮挡
- 🎭 特效层级混乱

**实现建议**:

```rust
fn sort(&mut self, objects_map: &HashMap<u32, Box<dyn DrawableMapObject>>) {
    if let Some(ref mut objects) = self.cell_objects {
        objects.sort_by(|&id1, &id2| {
            let obj1 = objects_map.get(&id1);
            let obj2 = objects_map.get(&id2);
            
            if obj1.is_none() || obj2.is_none() {
                return id1.cmp(&id2);
            }
            
            let obj1 = obj1.unwrap();
            let obj2 = obj2.unwrap();
            
            // 1. 掉落物品优先
            match (obj1.race(), obj2.race()) {
                (ObjectType::Item, ObjectType::Item) => {},
                (ObjectType::Item, _) => return std::cmp::Ordering::Less,
                (_, ObjectType::Item) => return std::cmp::Ordering::Greater,
                _ => {},
            }
            
            // 2. 法术特效次优先
            match (obj1.race(), obj2.race()) {
                (ObjectType::Spell, ObjectType::Spell) => {},
                (ObjectType::Spell, _) => return std::cmp::Ordering::Less,
                (_, ObjectType::Spell) => return std::cmp::Ordering::Greater,
                _ => {},
            }
            
            // 3. 死亡对象排在后面
            match (obj1.is_dead(), obj2.is_dead()) {
                (false, true) => return std::cmp::Ordering::Less,
                (true, false) => return std::cmp::Ordering::Greater,
                _ => {},
            }
            
            // 4. 按 ObjectID 排序（稳定性）
            id1.cmp(&id2)
        });
    }
}
```

---

### 3. DrawObjects / DrawDeadObjects 方法

#### C# 实现 (MapCode.cs lines 55-113)

```csharp
public void DrawObjects()
{
    if (CellObjects == null) return;

    for (int i = 0; i < CellObjects.Count; i++)
    {
        if (!CellObjects[i].Dead)
        {
            CellObjects[i].Draw();
            continue;
        }

        // 特殊处理：死亡的城墙怪物仍然渲染
        if(CellObjects[i].Race == ObjectType.Monster)
        {
            switch(((MonsterObject)CellObjects[i]).BaseImage)
            {
                case Monster.PalaceWallLeft:
                case Monster.PalaceWall1:
                case Monster.PalaceWall2:
                case Monster.SSabukWall1:
                case Monster.SSabukWall2:
                case Monster.SSabukWall3:
                case Monster.HellLord:
                    CellObjects[i].Draw();
                    break;
                default:
                    continue;
            }
        }
    }
}

public void DrawDeadObjects()
{
    if (CellObjects == null) return;
    for (int i = 0; i < CellObjects.Count; i++)
    {
        if (!CellObjects[i].Dead) continue;

        // 跳过城墙怪物（已在 DrawObjects 中绘制）
        if (CellObjects[i].Race == ObjectType.Monster)
        {
            switch (((MonsterObject)CellObjects[i]).BaseImage)
            {
                case Monster.PalaceWallLeft:
                case Monster.PalaceWall1:
                case Monster.PalaceWall2:
                case Monster.SSabukWall1:
                case Monster.SSabukWall2:
                case Monster.SSabukWall3:
                case Monster.HellLord:
                    continue;  // ← 跳过，不重复绘制
            }
        }

        CellObjects[i].Draw();
    }
}
```

#### Rust 实现 (map_code.rs lines 130-173)

```rust
pub fn draw_objects(
    &self,
    ctx: &mut Context,
    canvas: &mut Canvas,
    objects_map: &HashMap<u32, Box<dyn DrawableMapObject>>,
    draw_location: Point,
) -> GameResult {
    if let Some(ref cell_objects) = self.cell_objects {
        for &object_id in cell_objects.iter() {
            if let Some(obj) = objects_map.get(&object_id) {
                if !obj.is_dead() && !obj.is_hidden() {
                    obj.draw(ctx, canvas, draw_location)?;
                }
            }
        }
    }
    Ok(())
}

pub fn draw_dead_objects(
    &self,
    ctx: &mut Context,
    canvas: &mut Canvas,
    objects_map: &HashMap<u32, Box<dyn DrawableMapObject>>,
    draw_location: Point,
) -> GameResult {
    if let Some(ref cell_objects) = self.cell_objects {
        for &object_id in cell_objects.iter() {
            if let Some(obj) = objects_map.get(&object_id) {
                if obj.is_dead() && !obj.is_hidden() {
                    // TODO: Add special handling for dead monsters (walls, HellLord, etc.)
                    // C# checks for ((MonsterObject)CellObjects[i]).EternalStatue
                    obj.draw(ctx, canvas, draw_location)?;
                }
            }
        }
    }
    Ok(())
}
```

#### 对比结果: ⚠️ **简化实现**

**缺失功能**:
- ❌ 特殊怪物判断 (城墙、地狱领主等)
- ❌ BaseImage 检查
- ❌ 不可摧毁建筑的特殊渲染逻辑

**影响**:
- 🏰 城墙类怪物可能渲染错误
- 👹 特殊Boss渲染可能有问题

**修复建议**: 在 `DrawableMapObject` trait 中添加方法：
```rust
pub trait DrawableMapObject {
    fn is_eternal_statue(&self) -> bool { false }
    fn base_image(&self) -> Option<MonsterType> { None }
}
```

---

### 4. MapReader 结构

#### C# 实现 (MapCode.cs lines 128-617)

```csharp
class MapReader
{
    public int Width;
    public int Height;
    public CellInfo[,] MapCells;
    private string FileName;
    private byte[] Bytes;

    public MapReader(string FileName);
    private void initiate();
    
    // 8种地图格式加载器
    private void LoadMapType0();   // ✅ 旧版传奇2
    private void LoadMapType1();   // ✅ Wemade 2010
    private void LoadMapType2();   // ✅ Shanda老版本
    private void LoadMapType3();   // ✅ Shanda 2012 (瓦片动画)
    private void LoadMapType4();   // ✅ Wemade防hack (迷宫)
    private void LoadMapType5();   // ✅ Wemade Mir3
    private void LoadMapType6();   // ✅ Shanda Mir3
    private void LoadMapType7();   // ✅ 3/4 Heroes
    private void LoadMapType100(); // ✅ C#自定义
}
```

#### Rust 实现 (map_code.rs lines 182-831)

```rust
pub struct MapReader {
    pub width: i32,
    pub height: i32,
    pub map_cells: Vec<Vec<CellInfo>>,  // ⚠️ Vec<Vec> 而非 [,]
    pub file_name: String,
    bytes: Vec<u8>,
}

impl MapReader {
    pub fn new(file_name: &str) -> io::Result<Self>;
    fn initiate(&mut self) -> io::Result<()>;
    fn detect_and_load(&mut self) -> io::Result<()>;
    
    // 地图格式加载器
    fn load_map_type_0(&mut self) -> io::Result<()>;    // ✅ 已实现
    fn load_map_type_1(&mut self) -> io::Result<()>;    // ✅ 已实现
    fn load_map_type_2(&mut self) -> io::Result<()>;    // ✅ 已实现
    fn load_map_type_3(&mut self) -> io::Result<()>;    // ✅ 已实现
    fn load_map_type_4(&mut self) -> io::Result<()>;    // ❌ 未实现
    fn load_map_type_5(&mut self) -> io::Result<()>;    // ❌ 未实现
    fn load_map_type_6(&mut self) -> io::Result<()>;    // ❌ 未实现
    fn load_map_type_7(&mut self) -> io::Result<()>;    // ❌ 未实现
    fn load_map_type_100(&mut self) -> io::Result<()>;  // ✅ 已实现
    
    // 辅助方法
    pub fn get_cell(&self, x: i32, y: i32) -> Option<&CellInfo>;
    pub fn get_cell_mut(&mut self, x: i32, y: i32) -> Option<&mut CellInfo>;
}
```

#### 地图格式实现状态

| 格式 | 名称 | C# | Rust | 状态 | 优先级 | 使用场景 |
|------|------|-----|------|------|--------|----------|
| **Type 0** | 旧版传奇2 | ✅ | ✅ | ✅ 完成 | 🔴 高 | 经典传奇地图 |
| **Type 1** | Wemade 2010 | ✅ | ✅ | ✅ 完成 | 🟡 中 | 新版传奇地图 |
| **Type 2** | Shanda老版本 | ✅ | ✅ | ✅ 完成 | 🟡 中 | 盛大早期地图 |
| **Type 3** | Shanda 2012 | ✅ | ✅ | ✅ 完成 | 🟡 中 | 瓦片动画地图 |
| **Type 4** | Wemade防hack | ✅ | ❌ | ⚠️ TODO | 🟢 低 | 迷宫地图 |
| **Type 5** | Wemade Mir3 | ✅ | ❌ | ⚠️ TODO | 🟢 低 | 传奇3地图 |
| **Type 6** | Shanda Mir3 | ✅ | ❌ | ⚠️ TODO | 🟢 低 | 盛大传奇3 |
| **Type 7** | 3/4 Heroes | ✅ | ❌ | ⚠️ TODO | 🟢 低 | 英雄版地图 |
| **Type 100** | C#自定义 | ✅ | ✅ | ✅ 完成 | 🔴 高 | 服务器自定义 |

---

## 📊 统计数据

### 代码行数对比

| 项目 | C# | Rust | 比例 |
|------|-----|------|------|
| CellInfo | ~120 lines | ~180 lines | 150% |
| MapReader | ~497 lines | ~650 lines | 131% |
| **总计** | **617 lines** | **831 lines** | **135%** |

Rust 代码更长的原因：
1. ✅ 详细的注释和文档
2. ✅ 显式错误处理 (`io::Result`)
3. ✅ 类型安全检查
4. ✅ 单元测试

### 功能完成度

```
CellInfo 结构:        ████████████████████ 100% ✅
  - 字段定义:         ████████████████████ 100% ✅
  - 对象管理:         ███████████████░░░░░  75% ⚠️
  - 绘制方法:         ████████████░░░░░░░░  60% ⚠️
  
MapReader 结构:       ████████████░░░░░░░░  60% ⚠️
  - 基础架构:         ████████████████████ 100% ✅
  - 格式检测:         ████████████████████ 100% ✅
  - Type 0:           ████████████████████ 100% ✅
  - Type 1:           ████████████████████ 100% ✅
  - Type 2:           ████████████████████ 100% ✅
  - Type 3:           ████████████████████ 100% ✅
  - Type 4-7:         ░░░░░░░░░░░░░░░░░░░░   0% ❌
  - Type 100:         ████████████████████ 100% ✅
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
总体完成度:           ████████████░░░░░░░░  60% ⚠️
```

---

## 🔧 需要补充的功能

### 1. 高优先级 (必需)

#### 1.1 Sort 方法完整实现

**当前状态**: 仅占位符  
**影响**: 渲染顺序错误  
**工作量**: 🕐 1小时

**实现步骤**:
```rust
// 在 CellInfo impl 中补充
fn sort(&mut self, objects_map: &HashMap<u32, Box<dyn DrawableMapObject>>) {
    // 实现完整排序逻辑（见上文建议）
}

// 修改 add_object 和 remove_object 调用
pub fn add_object(&mut self, object_id: u32, objects_map: &HashMap<...>) {
    // ...
    self.sort(objects_map);  // ← 传入对象映射
}
```

#### 1.2 DrawObjects 特殊怪物处理

**当前状态**: 简化实现  
**影响**: 城墙/Boss渲染错误  
**工作量**: 🕐 0.5小时

**实现步骤**:
```rust
// 在 DrawableMapObject trait 中添加
pub trait DrawableMapObject {
    fn is_eternal_statue(&self) -> bool { false }
    fn base_image(&self) -> Option<MonsterType> { None }
}

// 在 draw_objects 中使用
if obj.is_dead() {
    if let Some(base_image) = obj.base_image() {
        match base_image {
            MonsterType::PalaceWallLeft |
            MonsterType::PalaceWall1 |
            MonsterType::PalaceWall2 |
            MonsterType::SSabukWall1 |
            MonsterType::SSabukWall2 |
            MonsterType::SSabukWall3 |
            MonsterType::HellLord => {
                obj.draw(ctx, canvas, draw_location)?;
                continue;
            }
            _ => continue,
        }
    }
    continue;
}
```

### 2. 中优先级 (可选)

#### 2.1 is_walkable 完整逻辑

**当前状态**: 简化实现  
**影响**: 寻路可能不准确  
**工作量**: 🕐 1小时

**参考C#逻辑** (需要检查服务器端 Cell 枚举):
```csharp
// Server/MirEnvir/Map.cs
public enum Cell : byte
{
    None = 0,
    HighWall = 1,
    LowWall = 2,
}

public bool ValidPoint(Point p)
{
    return p.X >= 0 && p.X < Width && 
           p.Y >= 0 && p.Y < Height && 
           !Cells[p.X, p.Y].HasFlag(Cell.HighWall);
}
```

**Rust 实现建议**:
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellType {
    None = 0,
    HighWall = 1,
    LowWall = 2,
}

impl CellInfo {
    pub fn cell_type(&self) -> CellType {
        // 根据 BackImage, FrontImage 的标志位判断
        if (self.back_image & 0x20000000) != 0 {
            return CellType::HighWall;
        }
        if (self.front_image & 0x8000) != 0 {
            return CellType::LowWall;
        }
        CellType::None
    }
    
    pub fn is_walkable(&self) -> bool {
        self.cell_type() != CellType::HighWall
    }
}
```

### 3. 低优先级 (扩展)

#### 3.1 未实现的地图格式

**Type 4 - Wemade AntiHack**:
- 迷宫地图专用
- 使用率极低
- 工作量: 🕐 2小时

**Type 5 - Wemade Mir3**:
- 传奇3地图格式
- 复杂度高（特殊Back层读取）
- 工作量: 🕐 4小时

**Type 6 - Shanda Mir3**:
- 盛大传奇3格式
- 工作量: 🕐 4小时

**Type 7 - 3/4 Heroes**:
- 英雄版本地图
- 使用率低
- 工作量: 🕐 2小时

**总工作量**: 🕐 12小时

**建议**: 按需实现，仅当实际使用到这些格式时再补充

---

## 🎯 实施建议

### 立即行动 (本周)

1. **Sort 方法** (1小时)
   - 实现完整排序逻辑
   - 修改 add_object/remove_object 调用签名

2. **DrawObjects 特殊处理** (0.5小时)
   - 添加 trait 方法
   - 实现城墙/Boss特殊渲染

### 短期优化 (下周)

3. **is_walkable 逻辑** (1小时)
   - 添加 CellType 枚举
   - 实现完整可行走判断

4. **单元测试** (1小时)
   - 测试对象管理
   - 测试地图加载

### 长期扩展 (按需)

5. **Type 4-7 格式** (12小时)
   - 仅在实际需要时实现
   - 优先级最低

---

## 💡 架构建议

### 当前设计的优点

✅ **类型安全**:
```rust
pub fn get_cell(&self, x: i32, y: i32) -> Option<&CellInfo>
```
边界检查由 Rust 编译器保证

✅ **错误处理**:
```rust
fn load_map_type_0(&mut self) -> io::Result<()>
```
明确的错误传播

✅ **所有权清晰**:
```rust
pub cell_objects: Option<Vec<u32>>  // ID而非引用
```
避免所有权冲突

### 可改进之处

⚠️ **排序需要传递对象映射**:
```rust
// 当前
fn sort(&mut self);  // ❌ 无法访问对象信息

// 建议
fn sort(&mut self, objects_map: &HashMap<u32, Box<dyn DrawableMapObject>>);
```

⚠️ **绘制方法签名复杂**:
```rust
// 当前
pub fn draw_objects(
    &self,
    ctx: &mut Context,
    canvas: &mut Canvas,
    objects_map: &HashMap<u32, Box<dyn DrawableMapObject>>,
    draw_location: Point,
) -> GameResult;

// 可能的简化（需要权衡）
pub fn draw_objects(&self, renderer: &mut GameRenderer) -> GameResult;
```

---

## 📚 参考文档

### 已创建的相关文档

1. `MLibrary字段用途说明.md` - 图像字段用途
2. `MapControl完整注释说明文档.md` - 地图控制器
3. `地图瓦片加载_修复完成.md` - 地图渲染
4. `地图与资源文件处理逻辑与模块说明.md` - 模块说明

### C# 关键代码位置

1. **CellInfo定义**: `Client/MirObjects/MapCode.cs` lines 9-126
2. **MapReader定义**: `Client/MirObjects/MapCode.cs` lines 128-617
3. **服务器端Cell**: `Server/MirEnvir/Map.cs`
4. **MapControl使用**: `Client/MirScenes/GameScene.cs` lines 10717+

### Rust 实现位置

1. **map_code.rs**: `ClientRust/src/objects/map_code.rs`
2. **使用示例**: `ClientRust/src/scenes/game_scene.rs`

---

## 🏆 总结

### 当前状态: **基本可用** ⚠️

**核心功能完成度: 60%**

**已完成的关键部分**:
- ✅ CellInfo 结构 100%
- ✅ 5种主流地图格式
- ✅ 基础对象管理

**待完成的重要部分**:
- ⚠️ Sort 排序逻辑
- ⚠️ 特殊怪物处理
- ⚠️ 4种扩展地图格式

### 建议: **补充高优先级功能后可用于生产**

**立即补充**:
1. Sort 方法 (1小时)
2. DrawObjects 特殊处理 (0.5小时)

**短期优化**:
3. is_walkable 完整逻辑 (1小时)

**长期扩展**:
4. Type 4-7 格式 (按需实现)

**预计总工作量**: 🕐 2.5小时 (高优先级)

完成后可达到 **85% 完成度**，满足绝大多数使用场景。

---

**评审人**: AI Assistant  
**评审日期**: 2025-10-10  
**下次审查**: 完成高优先级功能后
