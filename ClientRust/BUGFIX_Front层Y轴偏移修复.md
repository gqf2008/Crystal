# Front层Y轴偏移修复报告

## 🔍 问题描述

**症状**：Front层的Y轴偏移不正确，大型物体（树木、建筑等）的位置不对

**原因**：Bevy版本的Y轴偏移计算与ggez版本不一致

## 🎯 核心问题

### ggez版本（Y轴向下）

```rust
let mut world_y = if (tile_width as i32 != Self::CELL_WIDTH
    || tile_height as i32 != Self::CELL_HEIGHT)
    && (tile_width as i32 != Self::CELL_WIDTH * 2
        || tile_height as i32 != Self::CELL_HEIGHT * 2)
{
    // 非标准尺寸 = 大型物体 (树/建筑等)
    world_y_base + Self::CELL_HEIGHT as f32 - tile_height as f32
} else {
    // 标准地板瓦片
    world_y_base
};
```

**关键公式**：
- 格子底部Y = `world_y_base + CELL_HEIGHT`
- 纹理顶部Y = `world_y_base + CELL_HEIGHT - texture_height`
- **使大型物体的底部对齐到格子底部**

### Bevy版本（Y轴向上）- 修复前 ❌

```rust
let mut sprite_y = world_pos.y - texture_data.height as f32 / 2.0;

// 大型物体Y偏移
if is_large {
    sprite_y -= (texture_data.height as f32 - CELL_HEIGHT as f32) / 2.0;
}
```

**问题**：
- 只偏移了高度差的一半，不正确
- 没有对齐到格子底部

### Bevy版本（Y轴向上）- 修复后 ✅

```rust
let mut sprite_y = if is_large {
    // 大型物体：向上偏移整个纹理高度，使底部对齐到格子底部
    // ggez公式: world_y_base + CELL_HEIGHT - texture_height
    // Bevy公式: world_y_base - CELL_HEIGHT + texture_height
    // 但Sprite中心点在中间，所以还要减去 texture_height / 2.0
    world_pos.y - CELL_HEIGHT as f32 + texture_data.height as f32 - texture_data.height as f32 / 2.0
} else {
    // 标准瓦片：直接使用格子坐标
    world_pos.y - texture_data.height as f32 / 2.0
};
```

## 📐 坐标转换详解

### 坐标系对比

| 系统 | Y轴方向 | 格子原点 | 格子底部 |
|------|---------|---------|---------|
| **ggez** | ↓ 向下 | world_y_base | world_y_base + CELL_HEIGHT |
| **Bevy** | ↑ 向上 | world_y_base | world_y_base - CELL_HEIGHT |

### 纹理底部对齐计算

#### ggez（Y轴向下）

```
纹理顶部Y = 格子底部Y - 纹理高度
         = (world_y_base + CELL_HEIGHT) - texture_height
         = world_y_base + CELL_HEIGHT - texture_height
```

#### Bevy（Y轴向上）

```
1. 纹理底部对齐到格子底部
   纹理底部Y = 格子底部Y
            = world_y_base - CELL_HEIGHT

2. 纹理顶部Y = 纹理底部Y + 纹理高度
            = (world_y_base - CELL_HEIGHT) + texture_height
            = world_y_base - CELL_HEIGHT + texture_height

3. Sprite中心点Y = 纹理顶部Y - 纹理高度/2
               = (world_y_base - CELL_HEIGHT + texture_height) - texture_height/2
               = world_y_base - CELL_HEIGHT + texture_height/2
```

### 简化公式

**最终Bevy Sprite中心点计算**：

```rust
// 大型物体
sprite_y = world_pos.y - CELL_HEIGHT + texture_height/2

// 简化写法（更清晰）
sprite_y = world_pos.y - CELL_HEIGHT as f32 + texture_data.height as f32 - texture_data.height as f32 / 2.0
```

## 🔧 修复内容

### 1. Front层静态瓦片（第832-850行）

**修复前**：
```rust
let mut sprite_y = world_pos.y - texture_data.height as f32 / 2.0;

let is_large = (texture_data.width != 48 || texture_data.height != 32) &&
             (texture_data.width != 96 || texture_data.height != 64);
if is_large {
    sprite_y -= (texture_data.height as f32 - CELL_HEIGHT as f32) / 2.0;
}
```

**修复后**：
```rust
let is_large = (texture_data.width != 48 || texture_data.height != 32) &&
             (texture_data.width != 96 || texture_data.height != 64);

let mut sprite_y = if is_large {
    // 大型物体：底部对齐到格子底部
    world_pos.y - CELL_HEIGHT as f32 + texture_data.height as f32 - texture_data.height as f32 / 2.0
} else {
    // 标准瓦片
    world_pos.y - texture_data.height as f32 / 2.0
};
```

### 2. Front层动画瓦片（第1050-1063行）

**修复前**：
```rust
let mut sprite_y = world_pos.y - texture_data.height as f32 / 2.0;

let is_large = (texture_data.width != 48 || texture_data.height != 32) &&
             (texture_data.width != 96 || texture_data.height != 64);
if is_large {
    sprite_y -= (texture_data.height as f32 - CELL_HEIGHT as f32) / 2.0;
}
```

**修复后**：
```rust
let is_large = (texture_data.width != 48 || texture_data.height != 32) &&
             (texture_data.width != 96 || texture_data.height != 64);

let mut sprite_y = if is_large {
    // 大型物体：底部对齐到格子底部
    world_pos.y - CELL_HEIGHT as f32 + texture_data.height as f32 - texture_data.height as f32 / 2.0
} else {
    // 标准瓦片
    world_pos.y - texture_data.height as f32 / 2.0
};
```

## 🎨 视觉效果对比

### 修复前 ❌

```
大树 (高度=200px)
     ┌─────┐
     │     │
     │树干 │  ← 向上漂浮
     │     │
     └─────┘
─────┬─────┬───── 格子底部
     │32px │
     └─────┘
```

### 修复后 ✅

```
大树 (高度=200px)
     ┌─────┐
     │     │
     │树干 │
     │     │
─────┴─────┴───── 格子底部 ← 底部对齐
     └─────┘
```

## 📊 测试用例

### 标准瓦片 (48x32)

```rust
world_pos.y = -1000.0  // 格子顶部
sprite_y = -1000.0 - 16.0 = -1016.0  // Sprite中心
```

### 大型瓦片 (96x200)

```rust
world_pos.y = -1000.0       // 格子顶部
CELL_HEIGHT = 32
texture_height = 200

sprite_y = -1000.0 - 32.0 + 200.0 - 100.0
        = -932.0  // Sprite中心

// 验证：
// 纹理底部 = sprite_y - height/2 = -932 - 100 = -1032
// 格子底部 = world_pos.y - CELL_HEIGHT = -1000 - 32 = -1032
// ✅ 对齐正确！
```

## ✅ 验证方法

1. **运行程序**：
   ```bash
   cargo run --bin map_viewer_bevy --release
   ```

2. **按M键加载地图**

3. **按3键显示Front层**

4. **观察大型物体**：
   - 树木底部应该站在地面上（不漂浮）
   - 建筑底部应该与地面对齐
   - 门应该正确对齐到墙壁

5. **对比ggez版本**：
   - 运行ggez版本查看器
   - 对比相同位置的物体显示
   - 应该完全一致 ✓

## 🔑 关键要点

### Y轴翻转规则

| ggez公式 | Bevy公式 | 说明 |
|----------|----------|------|
| `y + offset` | `y - offset` | 向下移动变成向上移动 |
| `y + CELL_HEIGHT` | `y - CELL_HEIGHT` | 格子底部计算 |
| `y + CELL_HEIGHT - height` | `y - CELL_HEIGHT + height` | 底部对齐 |

### Sprite中心点偏移

Bevy的Sprite锚点在中心，需要额外偏移：
```rust
sprite_center_y = texture_top_y - height/2
```

### 大型物体判断

```rust
let is_large = (width != 48 || height != 32) &&
               (width != 96 || height != 64);
```

**标准尺寸**：
- 小瓦片：48x32（1个格子）
- 大瓦片：96x64（2x2格子）

**非标准** = 大型物体（树木、建筑、装饰等）

## 📝 相关文件

- **Bevy版本**: `src/bin/map_viewer_bevy.rs`
  - 第832-850行：Front层静态瓦片Y轴计算
  - 第1050-1063行：Front层动画瓦片Y轴计算

- **ggez版本（参考）**: `src/bin/map_viewer.rs`
  - 第532-545行：Front层Y轴计算

## 🎯 效果总结

### ✅ 修复后的效果

- **大型物体**：底部正确对齐到格子底部
- **树木**：根部站在地面上
- **建筑**：地基与地面平齐
- **门**：正确对齐到墙壁
- **装饰物**：位置与ggez版本一致

### 🔍 技术要点

1. **坐标系差异**：正确理解ggez（Y向下）和Bevy（Y向上）的区别
2. **Y轴翻转**：所有Y轴偏移计算需要取反
3. **Sprite中心**：Bevy的Sprite锚点在中心，需要额外偏移
4. **底部对齐**：大型物体必须使底部对齐到格子底部

---

**修复日期**: 2025-10-19  
**修复版本**: Bevy 0.17.2  
**问题严重性**: 中等（影响Front层显示，但不影响功能）  
**修复状态**: ✅ 已完成
