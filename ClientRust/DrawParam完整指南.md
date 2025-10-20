# DrawParam 参数完整指南

## 🎯 你提到的 GGEZ ZIndex 官方文档

```rust
/// Value describing the Z "coordinate" of a draw.
///
/// Greater values correspond to the foreground, and lower values
/// correspond to the background.
///
/// InstanceArray internally uphold this order for their instances, 
/// _if_ they're created with `ordered` set to `true`.
```

**关键信息**：
- ✅ **Greater values = Foreground（前景）** ← 这是重点！
- ✅ **Lower values = Background（背景）**
- ✅ 类型：`i32`（不是 `f32`）
- ⚠️ 只在 `InstanceArray` 且 `ordered=true` 时自动排序

---

## 📚 DrawParam 四大核心参数

### 1. 🎯 z - 深度排序

```rust
DrawParam::default()
    .dest([100.0, 100.0])
    .z(2000)  // i32 类型，数值越大越靠前
```

**正确理解**：
```rust
// 绘制顺序（从后往前）
.z(0)     // 天空（最后面）
.z(1000)  // 地面
.z(2000)  // 建筑物
.z(3000)  // 角色
.z(9000)  // UI（最前面）
```

**注意事项**：
- ⚠️ 单个 `canvas.draw()` 调用时，**z 参数可能不会自动排序**
- ✅ 必须配合 `InstanceArray` + `ordered=true` 使用
- ✅ 更推荐：**手动控制绘制顺序**（本项目做法）

---

### 2. 🔄 transform - 2D 变换矩阵

```rust
use glam::{Mat4, Vec3, Quat};

// 方式 1：使用辅助函数
let transform = Mat4::from_scale_rotation_translation(
    Vec3::new(2.0, 2.0, 1.0),  // 缩放 2 倍
    Quat::from_rotation_z(0.5), // 旋转 0.5 弧度
    Vec3::new(100.0, 100.0, 0.0) // 平移
);

DrawParam::default().transform(transform)

// 方式 2：组合变换
let transform = Mat4::IDENTITY
    .mul_mat4(&Mat4::from_translation(Vec3::new(100.0, 100.0, 0.0)))
    .mul_mat4(&Mat4::from_rotation_z(0.5))
    .mul_mat4(&Mat4::from_scale(Vec3::new(2.0, 2.0, 1.0)));
```

**应用场景**：
- ✅ 复杂的组合变换
- ✅ 父子关系的坐标系（UI 嵌套）
- ✅ 骨骼动画、物理变换

**vs 简单参数**：
```rust
// 简单场景：直接用单独参数
DrawParam::default()
    .dest([x, y])
    .scale([2.0, 2.0])
    .rotation(0.5)

// 复杂场景：用 transform
DrawParam::default()
    .transform(my_complex_matrix)
```

---

### 3. 📐 dest - 目标位置

```rust
DrawParam::default()
    .dest([100.0, 200.0])  // 绘制到 (100, 200)
    .dest(mint::Point2 { x: 100.0, y: 200.0 })  // 也支持 mint 类型
```

**相对 offset**：
```rust
DrawParam::default()
    .dest([100.0, 100.0])
    .offset([0.5, 0.5])  // 从纹理中心开始绘制
```

---

### 4. 🎨 color - 颜色调制

```rust
use ggez::graphics::Color;

// 完全不透明
DrawParam::default()
    .color(Color::WHITE)

// 半透明红色
DrawParam::default()
    .color(Color::from_rgba(255, 0, 0, 128))

// 变暗效果
DrawParam::default()
    .color(Color::from_rgba(128, 128, 128, 255))

// 完全透明（不可见）
DrawParam::default()
    .color(Color::from_rgba(255, 255, 255, 0))
```

**技巧**：
```rust
// 亮度调制（本项目使用）
let brightness = 1.5;  // 50% 更亮
let color = Color::from_rgba(
    (255.0 * brightness) as u8,
    (255.0 * brightness) as u8,
    (255.0 * brightness) as u8,
    255
);
```

---

## 🎮 完整示例

```rust
use ggez::graphics::{DrawParam, Color, Canvas};
use glam::{Mat4, Vec3, Quat};

fn draw_complex_sprite(canvas: &mut Canvas, texture: &Image) -> GameResult {
    // 示例 1：简单绘制
    canvas.draw(
        texture,
        DrawParam::default()
            .dest([100.0, 100.0])
            .scale([2.0, 2.0])
            .rotation(0.5)
            .color(Color::WHITE)
            .z(1000)  // 中间层
    );

    // 示例 2：使用 transform
    let transform = Mat4::from_scale_rotation_translation(
        Vec3::new(2.0, 2.0, 1.0),
        Quat::from_rotation_z(0.5),
        Vec3::new(200.0, 200.0, 0.0)
    );
    
    canvas.draw(
        texture,
        DrawParam::default()
            .transform(transform)
            .color(Color::from_rgba(255, 0, 0, 128))
            .z(2000)  // 前景
    );

    // 示例 3：偏移绘制（从中心点）
    canvas.draw(
        texture,
        DrawParam::default()
            .dest([300.0, 300.0])
            .offset([0.5, 0.5])  // 纹理中心对齐到 dest
            .scale([1.5, 1.5])
            .z(500)  // 背景
    );

    Ok(())
}
```

---

## 🔧 本项目实现策略

### 我们的 Z 轴设计

```rust
// src/bin/map_viewer_ecs.rs

// 1. 每个瓦片有 z_order 字段
struct MapTile {
    z_order: i32,  // 自定义深度值
    // ...
}

// 2. 创建时分配 z 值
TileLayer::Back   => z_order = 0,     // 地面
TileLayer::Middle => z_order = 1000,  // 物体
TileLayer::Front  => z_order = 2000,  // 建筑物

// 3. 渲染前手动排序
visible_with_sort_key.sort_by(|a, b| {
    match a.1.cmp(&b.1) {  // 先按 z_order
        std::cmp::Ordering::Equal => a.2.cmp(&b.2),  // 相同则按 Y
        other => other,
    }
});

// 4. 按顺序绘制（不使用 DrawParam.z()）
for tile in sorted_tiles {
    canvas.draw(texture, DrawParam::default()
        .dest([x, y])
        .scale([zoom, zoom])
        // 不使用 .z()，因为已经手动排序了
    );
}
```

### 为什么不用 DrawParam.z()？

1. ⚠️ **单个 draw 调用不会自动排序**
2. ✅ **手动排序更可控**
3. ✅ **支持复杂排序逻辑**（z + Y 坐标）
4. ✅ **更好的调试体验**（排序逻辑清晰可见）

---

## 📊 性能对比

| 方法 | 绘制次数 | 排序开销 | 推荐场景 |
|------|---------|---------|---------|
| InstanceArray (ordered) | 1 | GGEZ 内部 | 大量相同纹理 |
| 手动排序 | N | 每帧排序 | 少量对象、复杂逻辑 |
| 手动排序 + 缓存 | N | 移动时排序 | 本项目（最优） |

**本项目优化**：
- ✅ 只在相机移动时重新排序
- ✅ 缓存排序结果（VisibleArea）
- ✅ 按 BlendMode 批量绘制
- ✅ 结果：35-160 FPS，流畅运行

---

## 🎯 最佳实践总结

### DrawParam.z() 使用场景

✅ **推荐使用**：
```rust
// 场景 1：InstanceArray 批量绘制
let mut instances = InstanceArray::new(ctx, texture);
instances.set_ordered(true);  // 必须！

instances.push(DrawParam::default().dest([x1, y1]).z(0));
instances.push(DrawParam::default().dest([x2, y2]).z(100));
canvas.draw(&instances, DrawParam::default());
```

❌ **不推荐使用**：
```rust
// 场景 2：单个绘制（z 参数可能无效）
canvas.draw(&tex1, DrawParam::default().z(100));
canvas.draw(&tex2, DrawParam::default().z(0));  // 仍然后绘制！
```

### transform 使用场景

✅ **推荐使用**：
- 复杂的组合变换（旋转 + 缩放 + 平移）
- 父子坐标系（UI 嵌套布局）
- 矩阵数学（骨骼动画）

❌ **不推荐使用**：
- 简单的位置、缩放、旋转（用 dest/scale/rotation 更清晰）

---

## 📝 总结

### 你的问题答案

1. **DrawParam.z 是干嘛的？**
   - 深度排序，i32 类型，**数值越大越靠前**
   - 需要 InstanceArray + ordered=true 才自动生效
   - 单个 draw 调用时可能不生效

2. **GGEZ 支持 Z 轴分层吗？**
   - ✅ 支持，通过 DrawParam.z()
   - ⚠️ 但有限制（见上）
   - ✅ 推荐手动排序（更可靠）

3. **transform 是什么？**
   - 2D 变换矩阵，可组合多种变换
   - 适合复杂场景，简单场景用 dest/scale/rotation

4. **中文乱码？**
   - ✅ 已修复，自动加载系统中文字体
   - ✅ 支持 Windows/Linux/macOS

---

## 🔗 相关文档

- [GGEZ_Z_AXIS_说明.md](./GGEZ_Z_AXIS_说明.md) - 详细 Z 轴教程
- [src/bin/map_viewer_ecs.rs](./src/bin/map_viewer_ecs.rs) - 完整实现
- GGEZ 官方文档: https://docs.rs/ggez/latest/ggez/
