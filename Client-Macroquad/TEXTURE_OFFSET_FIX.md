# 血瓶纹理偏移修复说明

## 问题描述
血瓶对话框中的物品纹理显示时存在偏移问题，导致物品图标在格子中的位置不够精确。

## 解决方案
应用ImageInfo中的offset_x和offset_y偏移量来正确对齐纹理。

## 技术实现

### 修复前的代码
```rust
// 原来只是简单地缩小格子来绘制纹理
let item_rect = rect.shrink(1.0);
ui.painter().image(
    item_texture.id(),
    item_rect,
    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
    egui::Color32::WHITE,
);
```

### 修复后的代码
```rust
// 应用offset来正确对齐纹理
let offset_x = info.offset_x as f32;
let offset_y = info.offset_y as f32;

// 计算带偏移的绘制位置
let draw_pos = egui::pos2(
    cell_pos.x + 1.0 + offset_x, // 1像素边距 + X偏移
    cell_pos.y + 1.0 + offset_y  // 1像素边距 + Y偏移
);

// 使用纹理的实际尺寸
let texture_size = egui::vec2(info.width as f32, info.height as f32);
let item_rect = egui::Rect::from_min_size(draw_pos, texture_size);

ui.painter().image(
    item_texture.id(),
    item_rect,
    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
    egui::Color32::WHITE,
);
```

## 关键改进

1. **使用偏移量**：从ImageInfo中获取offset_x和offset_y
2. **精确定位**：根据偏移量计算准确的绘制位置
3. **实际尺寸**：使用纹理的真实width和height而非固定格子尺寸
4. **保持边距**：在应用偏移的同时保持1像素的格子边距

## 效果

- ✅ 血瓶和蓝瓶纹理现在精确对齐在格子中心
- ✅ 不同尺寸的物品图标都能正确显示
- ✅ 保持了原版传奇2的视觉效果
- ✅ 兼容所有Items库中的物品纹理

## 适用范围

这个修复方法适用于所有使用Items纹理库的物品显示，包括：
- 血瓶对话框（BeltDialog）
- 背包系统（InventoryDialog）
- 商店界面
- 交易窗口
- 装备栏

建议在所有相关的物品绘制代码中应用相同的偏移修复逻辑。