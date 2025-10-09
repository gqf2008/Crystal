# 地图渲染调试报告

## 当前状态 (2025-10-09)

### ✅ 已完成
1. **MapLibs加载**: 137/400 个地图库成功加载
2. **图像数据**: 230,000+ 张图像可用
3. **渲染循环**: 每帧渲染约 502 个瓦片
4. **简化渲染**: 禁用了Middle/Front/Objects层,只渲染Back层

### ❌ 存在的问题

#### 1. 所有瓦片显示相同图像
**观察**:
```
📍 瓦片[88,85]: library=0, image=0
📍 瓦片[89,85]: library=0, image=0  
📍 瓦片[90,85]: library=0, image=0
```

**原因**: 所有瓦片都从同一个库加载同一张图像

**可能原因**:
- 地图文件解析错误 (cell.back_index和back_image都是0)
- 或者测试地图确实使用同一种瓦片

#### 2. 瓦片坐标错位
**观察**:
```
📍 瓦片[88,85]: 相对偏移=(-12,-15), 屏幕=(-176.0, -180.0)
```

**问题**: 屏幕坐标是负数,瓦片会绘制到屏幕外看不见

**原因分析**:
- 窗口中心点设置为 (400, 300)
- 玩家位置 (100, 100)
- 相对偏移 = 瓦片位置 - 玩家位置 = (88-100, 85-100) = (-12, -15)
- 屏幕坐标 = 400 + (-12 * 48) = 400 - 576 = -176 ❌

**正确计算应该是**:
- 等距投影需要考虑菱形网格
- 标准传奇2公式: 
  - screen_x = center_x + (map_x - player_x - map_y + player_y) * CELL_WIDTH / 2
  - screen_y = center_y + (map_x - player_x + map_y - player_y) * CELL_HEIGHT / 2

#### 3. 图像偏移被禁用
**当前**: 不使用 info.x 和 info.y 偏移
```rust
let draw_x = x; // 不使用 info.x 偏移
let draw_y = y; // 不使用 info.y 偏移
```

**影响**: 瓦片对齐可能不正确

## 下一步修复方案

### 方案A: 修复等距投影坐标系统
```rust
// 正确的等距投影公式
let dx = x - user_pos.x;
let dy = y - user_pos.y;

// 等距投影 (菱形网格)
let draw_x = center_x + ((dx - dy) * Self::CELL_WIDTH / 2) as f32;
let draw_y = center_y + ((dx + dy) * Self::CELL_HEIGHT / 2) as f32;
```

### 方案B: 检查地图数据
验证地图文件是否正确加载:
1. 读取 GameMaster.map 文件
2. 检查 cell 数据是否有多样性
3. 打印前100个格子的 back_index 和 back_image

### 方案C: 使用C#的完整坐标系统
复制 C# MapControl 的完整坐标计算:
```csharp
// C# line 10476
int drawX = (x - User.Location.X + OffsetX) * CellWidth - OffsetX + User.OffsetX;
int drawY = (y - User.Location.Y + OffsetY) * CellHeight + User.OffsetY;
```

## 测试步骤

### 测试1: 验证地图数据多样性
```rust
for y in 0..10 {
    for x in 0..10 {
        if let Some(cell) = self.get_cell(x, y) {
            println!("[{},{}]: back_index={}, back_image={}",
                x, y, cell.back_index, cell.back_image);
        }
    }
}
```

### 测试2: 使用正确的等距投影
修改 draw_floor_simple() 使用方案A的坐标公式

### 测试3: 恢复图像偏移
测试是否需要 info.x/info.y 来正确对齐瓦片

## 当前代码位置
- `src/scenes/game_scene/map_control.rs:draw_floor_simple()` - 渲染循环
- `src/scenes/game_scene/map_control.rs:draw_tile_simple()` - 绘制函数
