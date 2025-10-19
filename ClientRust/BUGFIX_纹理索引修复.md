# Bevy地图查看器 - 纹理索引修复报告

## 🔍 问题诊断

### 症状
- 地图显示大量白块（纹理缺失）
- 与ggez版本对比，Bevy版本显示不正确

### 根本原因
**传奇地图格式的纹理索引计算错误**

传奇地图的纹理索引存储格式：
- 存储值从1开始（1表示第一个纹理）
- MLibrary API需要从0开始的索引（0表示第一个纹理）
- **关键：需要减1转换**

## 🔧 修复内容

### 1. Back层纹理索引修复

**错误代码**：
```rust
if cell.back_image > 0 {
    mlibrary.get_map_texture(
        cell.back_index,
        cell.back_image,  // ❌ 错误：直接使用原始值
        &mut images,
    )
}
```

**修复后**：
```rust
if cell.back_image > 0 {
    // 🔧 关键修复：传奇格式需要提取实际索引并减1
    let texture_index = ((cell.back_image & 0x1FFFFFFF) - 1) as i32;
    
    mlibrary.get_map_texture(
        cell.back_index,
        texture_index,  // ✅ 正确：减1后的索引
        &mut images,
    )
}
```

**说明**：
- `0x1FFFFFFF`：掩码，去除高位标志位
- `-1`：转换为从0开始的索引

### 2. Middle层纹理索引修复

**错误代码**：
```rust
if cell.middle_image > 0 {
    mlibrary.get_map_texture(
        cell.middle_index,
        cell.middle_image,  // ❌ 错误
        &mut images,
    )
}
```

**修复后**：
```rust
if cell.middle_image > 0 {
    // 🔧 关键修复：传奇格式需要减1
    let texture_index = (cell.middle_image - 1) as i32;
    
    mlibrary.get_map_texture(
        cell.middle_index,
        texture_index,  // ✅ 正确
        &mut images,
    )
}
```

### 3. Front层纹理索引修复

**错误代码**：
```rust
if cell.front_image > 0 {
    mlibrary.get_map_texture(
        cell.front_index,
        cell.front_image,  // ❌ 错误
        &mut images,
    )
}
```

**修复后**：
```rust
if cell.front_image > 0 {
    // 🔧 关键修复：传奇格式需要提取实际索引并减1
    let texture_index = ((cell.front_image & 0x7FFF) - 1) as i32;
    
    mlibrary.get_map_texture(
        cell.front_index,
        texture_index,  // ✅ 正确
        &mut images,
    )
}
```

**说明**：
- `0x7FFF`：掩码，去除Front层的特殊标志位
- 第15位（0x8000）：LowWall标记
- 第31位（0x80000000）：Blend标记

### 4. 门纹理索引修复

**错误代码**：
```rust
if cell.door_index > 0 {
    let door_image_index = cell.door_index as i32 + cell.door_offset as i32;
    
    mlibrary.get_map_texture(
        cell.front_index,
        door_image_index,  // ❌ 错误：未减1
        &mut images,
    )
}
```

**修复后**：
```rust
if cell.door_index > 0 {
    // 🔧 关键修复：门的图像索引计算
    // door_index: 基础索引（传奇格式需要减1）
    // door_offset: 0=关闭, 1-7=打开动画帧
    let base_index = (cell.door_index - 1) as i32;
    let door_image_index = base_index + cell.door_offset as i32;
    
    mlibrary.get_map_texture(
        cell.front_index,
        door_image_index,  // ✅ 正确
        &mut images,
    )
}
```

### 5. Middle层动画修复

**错误代码**：
```rust
let image_index = cell.tile_animation_image as i32 
                + cell.tile_animation_offset as i32 
                + current_frame;
```

**修复后**：
```rust
// 🔧 关键修复：动画基础索引需要减1
let base_index = (cell.tile_animation_image - 1) as i32;

let image_index = base_index 
                + cell.tile_animation_offset as i32 
                + current_frame;
```

### 6. Front层动画修复

**错误代码**：
```rust
let image_index = cell.front_image + current_frame;
```

**修复后**：
```rust
// 🔧 关键修复：动画基础索引需要提取并减1
let base_index = ((cell.front_image & 0x7FFF) - 1) as i32;

let image_index = base_index + current_frame;
```

## 🎨 增强功能

### 悬停信息面板优化

添加了详细的纹理索引调试信息：

```rust
📦 纹理索引信息 (原始值 → 实际索引):
┌─────────────────────────────────────────┐
│ Back层:   123 → 122  (FileIndex: 0)
│ Middle层: 456 → 455  (FileIndex: 1)
│ Front层:  789 → 788  (FileIndex: 2)
└─────────────────────────────────────────┘
```

**显示内容**：
- 地图坐标 (X, Y)
- 原始纹理值（存储在地图文件中的值）
- 实际纹理索引（传递给MLibrary的值 = 原始值 - 1）
- 文件索引（Tiles/Smtiles/Objects）

## 📊 对比参考

### ggez版本（正确实现）

```rust
// src/bin/map_viewer.rs:327
let index = (cell.back_image & 0x1FFFFFFF) - 1;
if cell.back_image == 0 || cell.back_index == -1 || index < 0 {
    continue;
}
self.draw_normal(
    ctx,
    canvas,
    camera,
    cell.back_index as i32,
    index as usize,  // ✅ 使用减1后的索引
    world_x,
    world_y,
    show_borders,
    Color::from_rgb(255, 0, 0),
)?;
```

## ✅ 验证方法

1. **运行程序**：
   ```bash
   cargo run --bin map_viewer_bevy --release
   ```

2. **按M键加载地图**

3. **移动鼠标**：
   - 查看悬停面板的纹理索引信息
   - 对比"原始值"和"实际索引"
   - 实际索引 = 原始值 - 1 ✓

4. **观察地图显示**：
   - 白块消失 ✓
   - 纹理正确显示 ✓
   - 与ggez版本一致 ✓

## 🔑 关键要点

### 传奇地图索引规则

| 层 | 原始值掩码 | 索引计算 | 特殊标志 |
|---|-----------|---------|---------|
| **Back** | `0x1FFFFFFF` | `(value & mask) - 1` | 第30位：HighWall (0x20000000) |
| **Middle** | 无需掩码 | `value - 1` | 无 |
| **Front** | `0x7FFF` | `(value & mask) - 1` | 第15位：LowWall (0x8000)<br>第31位：Blend (0x80000000) |

### 为什么需要减1？

1. **地图文件格式**：
   - 值0表示"无纹理"
   - 值1表示"第一个纹理"
   - 值N表示"第N个纹理"

2. **MLibrary API**：
   - 索引0表示"第一个纹理"
   - 索引N表示"第N+1个纹理"

3. **转换公式**：
   ```
   MLibrary索引 = 地图文件值 - 1
   ```

## 📝 修改文件

- `src/bin/map_viewer_bevy.rs`:
  - 第727-733行：Back层静态
  - 第777-783行：Middle层静态
  - 第824-830行：Front层静态
  - 第881-886行：门
  - 第975-981行：Middle层动画
  - 第1029-1035行：Front层动画
  - 第1335-1381行：悬停信息面板

## 🎯 测试建议

1. **白块问题**：应该完全消失
2. **纹理对齐**：与ggez版本一致
3. **动画效果**：水流、火焰等正常播放
4. **门动画**：开关门正常显示

## 📚 参考

- ggez版本实现：`src/bin/map_viewer.rs`
- CellInfo结构：`src/objects/mod.rs`
- MLibrary API：`src/graphics/libraries/mod.rs`

---

**修复日期**: 2025-10-19  
**修复版本**: Bevy 0.17.2  
**问题严重性**: 严重（导致地图显示大量白块）  
**修复状态**: ✅ 已完成
