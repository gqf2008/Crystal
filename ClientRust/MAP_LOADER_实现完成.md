# 🗺️ 地图加载系统实现 - Phase 1 完成

**日期**: 2025-10-08  
**状态**: ✅ 地图加载器完成  
**阶段**: Phase 1 - 数据加载

---

## 📋 本阶段完成内容

### ✅ 已实现

1. **地图文件加载器** (`map_loader.rs`)
   - ✅ .map 文件解析
   - ✅ 读取地图尺寸
   - ✅ 读取单元格数据 (walkable, fishable, tile_index)
   - ✅ 读取门数据
   - ✅ 自动查找地图文件路径

2. **MapControl 集成**
   - ✅ 在 GameScene 中添加 map_control 字段
   - ✅ 地图数据结构准备就绪

3. **网络事件系统**
   - ✅ 添加 `GameEvent::MapInformation` 事件
   - ✅ 服务器发送地图信息时触发加载
   - ✅ 自动加载对应的 .map 文件

4. **错误处理**
   - ✅ MapLoadError 类型
   - ✅ 文件不存在时的后备方案 (创建空地图)
   - ✅ 详细的日志输出

---

## 📂 新增文件

### `ClientRust/src/scenes/game_scene/map_loader.rs`

地图文件加载器,包含:

**核心功能**:
```rust
pub fn load_map<P: AsRef<Path>>(path: P) -> Result<MapControl, MapLoadError>
pub fn load_map_by_name(map_name: &str) -> Result<MapControl, MapLoadError>
pub fn find_map_file(map_name: &str) -> Option<std::path::PathBuf>
```

**地图文件格式**:
```
Header:
  - version: u8
  - width: i16
  - height: i16

Cells (width * height):
  - flags: u8 (bit 0: walkable, bit 1: fishable, bit 2: has door)
  - file_index: i16
  - tile_index: u16

Doors:
  - door_count: i32
  - [door_data...]:
      - x, y: i16
      - image_index: i32
      - opened: u8
      - offset: u8
```

**文件搜索路径**:
1. `Data/Map/{map_name}.map`
2. `../Data/Map/{map_name}.map`
3. `../../Data/Map/{map_name}.map`
4. `../../../Build/Client/Data/Map/{map_name}.map`

---

## 🔄 修改的文件

### 1. `ClientRust/src/scenes/game_scene.rs`

**添加**:
```rust
pub mod map_loader;  // 新模块

// 在 GameScene 结构体中
pub map_control: Option<map_control::MapControl>,

// 在 process_event 中
GameEvent::MapInformation { map_index, file_name, title } => {
    // 加载地图文件
    match map_loader::load_map_by_name(file_name) {
        Ok(map) => {
            self.map_control = Some(map);
        }
        Err(e) => {
            // 创建空地图
            self.map_control = Some(map_control::MapControl::new(100, 100));
        }
    }
}
```

### 2. `ClientRust/src/network/game_client.rs`

**添加事件类型**:
```rust
pub enum GameEvent {
    // ...
    MapInformation { map_index: i32, file_name: String, title: String },
    MapChanged { file_name: String, location: Point },
    // ...
}
```

**修改 on_map_information**:
```rust
fn on_map_information(&mut self, packet: packets::MapInformation) {
    // 保存地图信息
    self.map_info = Some(MapInfo { ... });
    
    // 发送事件到UI层
    self.send_event(GameEvent::MapInformation {
        map_index: packet.map_index,
        file_name: packet.file_name,
        title: packet.title,
    });
}
```

---

## 🔍 工作流程

### 玩家进入游戏时的地图加载流程

```
1. 玩家点击 "Start Game"
   ↓
2. 服务器发送 PlayerSpawned (玩家数据)
   ↓
3. 服务器发送 MapInformation (地图信息)
   ↓
4. GameClient 接收 MapInformation 包
   ↓
5. 发送 GameEvent::MapInformation 事件
   ↓
6. GameScene::process_event() 接收事件
   ↓
7. 调用 map_loader::load_map_by_name()
   ↓
8. 查找地图文件 (搜索多个路径)
   ↓
9. 解析 .map 文件
   ↓
10. 创建 MapControl 对象
   ↓
11. 保存到 self.map_control
   ↓
✅ 地图数据加载完成!
```

---

## 📊 MapControl 数据结构

```rust
pub struct MapControl {
    // 地图尺寸
    pub width: i32,
    pub height: i32,
    
    // 地图元数据
    pub filename: String,
    pub title: String,
    pub minimap: u16,
    pub bigmap: u16,
    
    // 单元格网格 (cells[x][y])
    pub cells: Vec<Vec<CellInfo>>,
    
    // 门列表
    pub doors: Vec<Door>,
    
    // 视口偏移
    pub offset_x: i32,
    pub offset_y: i32,
    
    // ... 其他字段
}

pub struct CellInfo {
    pub walkable: bool,    // 可行走
    pub fishable: bool,    // 可钓鱼
    pub door_index: Option<usize>,
    pub door_offset: u8,
    pub frame_index: u16,  // 瓦片索引
    pub file_index: i32,   // 瓦片库索引
    pub light_intensity: u8,
}
```

---

## 🧪 测试方法

### 测试地图加载

1. **启动游戏**
2. **登录并选择角色**
3. **点击 Start Game**
4. **观察日志输出**:

**预期日志**:
```
🗺️  Map: 比武场 (0)
📂 Loading map file: Data/Map/0.map
📐 Map dimensions: 120x120
📦 Reading 14400 cells...
🚪 Reading 5 doors...
✅ Map loaded successfully: 0.map (120x120)
```

**如果地图文件不存在**:
```
⚠️ Map file not found: 0
❌ Failed to load map 0: Map file not found: 0
[创建空地图 100x100 作为后备]
```

---

## 📐 地图文件位置

地图文件应该放在以下位置之一:

```
ClientRust/Data/Map/*.map
或
Build/Client/Data/Map/*.map
```

**常见地图文件**:
- `0.map` - 比武场
- `1.map` - 盟重省
- `2.map` - 毒蛇山谷
- `3.map` - 银杏山谷
- ... 等等

---

## 🎯 下一步: Phase 2 - 地图渲染

现在地图数据已经加载到内存,下一步是实现**地图渲染**:

### Phase 2 任务清单

🔄 **瓦片纹理加载**
- [ ] 从 Tiles.lib 加载瓦片纹理
- [ ] 创建纹理缓存系统
- [ ] 处理多个 Tiles 库文件

🔄 **地图渲染**
- [ ] 实现瓦片绘制
- [ ] 处理地图层次 (地面、墙壁、装饰)
- [ ] 实现视口裁剪 (只渲染可见区域)

🔄 **相机系统**
- [ ] 实现相机跟随玩家
- [ ] 平滑滚动
- [ ] 边界限制

🔄 **性能优化**
- [ ] 瓦片批量绘制
- [ ] 纹理图集 (texture atlas)
- [ ] 视锥裁剪 (frustum culling)

---

## 📝 代码示例

### 使用 map_loader

```rust
use crate::scenes::game_scene::map_loader;

// 方法1: 按名称加载
match map_loader::load_map_by_name("0") {
    Ok(map) => {
        println!("Loaded map: {} ({}x{})", map.title, map.width, map.height);
    }
    Err(e) => {
        println!("Failed to load map: {}", e);
    }
}

// 方法2: 按路径加载
match map_loader::load_map("Data/Map/0.map") {
    Ok(map) => {
        // 使用地图
    }
    Err(e) => {
        // 处理错误
    }
}

// 方法3: 查找地图文件
if let Some(path) = map_loader::find_map_file("0") {
    println!("Found map at: {}", path.display());
}
```

### 访问地图数据

```rust
if let Some(map) = &self.map_control {
    // 检查位置是否可行走
    if map.is_walkable(10, 20) {
        println!("Can walk to (10, 20)");
    }
    
    // 获取单元格信息
    if let Some(cell) = map.get_cell(10, 20) {
        println!("Tile index: {}", cell.frame_index);
        println!("Walkable: {}", cell.walkable);
    }
    
    // 屏幕坐标 ↔ 地图坐标转换
    let (map_x, map_y) = map.screen_to_map(512, 384);
    let (screen_x, screen_y) = map.map_to_screen(map_x, map_y);
}
```

---

## 🔧 调试信息

### 日志级别

设置环境变量以查看详细日志:
```powershell
$env:RUST_LOG="info,mir2_client=debug"
```

### 关键日志标识

- 🗺️  = 地图信息
- 📂 = 文件操作
- 📐 = 地图尺寸
- 📦 = 数据加载
- 🚪 = 门数据
- ✅ = 成功
- ❌ = 错误
- ⚠️  = 警告

---

## 🎨 视觉效果预览

**当前状态** (Phase 1):
- ✅ 地图数据在内存中
- ❌ 还没有视觉显示
- 屏幕显示: GameScene 信息界面

**Phase 2 完成后**:
- ✅ 地图瓦片渲染
- ✅ 可见的地图背景
- 屏幕显示: 真实的游戏地图!

---

## 📚 参考资料

### C# 原版实现

**文件**: `Client/MirScenes/GameScene.cs`

**地图加载代码** (lines ~10300-10500):
```csharp
private void LoadMap(string fileName)
{
    // Load .map file
    string path = Path.Combine(Settings.MapPath, fileName + ".map");
    
    // Read map data
    using (FileStream stream = File.OpenRead(path))
    using (BinaryReader reader = new BinaryReader(stream))
    {
        int width = reader.ReadInt16();
        int height = reader.ReadInt16();
        
        // Create cells
        for (int x = 0; x < width; x++)
        for (int y = 0; y < height; y++)
        {
            // Read cell data
            byte flags = reader.ReadByte();
            short fileIndex = reader.ReadInt16();
            ushort tileIndex = reader.ReadUInt16();
            
            // Create cell
            MapControl.Cells[x, y] = new Cell {
                Walkable = (flags & 0x01) != 0,
                // ... other properties
            };
        }
    }
}
```

---

## ✅ 阶段总结

### Phase 1 完成度: 100% ✨

- [x] 地图文件格式理解
- [x] 文件读取和解析
- [x] MapControl 数据结构填充
- [x] 网络事件集成
- [x] 错误处理
- [x] 日志系统
- [x] 测试方法

### 技术债务

- ⚠️ 地图文件路径硬编码 (需要配置系统)
- ⚠️ 没有地图缓存 (重复加载同一地图)
- ⚠️ 门动画未实现

### 下一步优先级

**立即**: Phase 2 - 地图渲染
- 实现瓦片纹理加载
- 实现地图绘制
- 实现相机系统

---

**创建时间**: 2025-10-08  
**作者**: GitHub Copilot  
**状态**: ✅ Phase 1 完成  
**下一步**: Phase 2 - 地图渲染 🎨
