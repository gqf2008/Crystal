# GGEZ Z 轴深度排序完整说明

## 📚 官方文档定义

```rust
/// Value describing the Z "coordinate" of a draw.
///
/// Greater values correspond to the foreground, and lower values
/// correspond to the background.
///
/// InstanceArray internally uphold this order for their instances, 
/// _if_ they're created with `ordered` set to `true`.
```

**关键点**：
- **数值越大 = 前景（Foreground）**
- **数值越小 = 背景（Background）**
- 类型：`i32`

---

## 🎯 两种使用方式

### 方式 1：InstanceArray (推荐用于大量相同纹理)

```rust
use ggez::graphics::{InstanceArray, DrawParam};

// 创建有序的实例数组
let mut instances = InstanceArray::new(ctx, texture);
instances.set_ordered(true);  // ✅ 必须设置为 true

// 添加实例时设置 z 值
instances.push(
    DrawParam::default()
        .dest([100.0, 100.0])
        .z(0)  // 背景
);

instances.push(
    DrawParam::default()
        .dest([120.0, 120.0])
        .z(100)  // 前景，会覆盖 z=0 的
);

// 绘制（自动按 z 排序）
canvas.draw(&instances, DrawParam::default());
```

**优势**：
- ✅ 自动按 z 值排序
- ✅ 性能高（批量绘制）
- ✅ 适合大量相同纹理（粒子、瓦片等）

**限制**：
- ⚠️ 只能用于相同纹理
- ⚠️ 必须设置 `ordered=true`

---

### 方式 2：手动排序 + 单独绘制 (我们使用的)

```rust
// 1. 收集所有绘制对象及其 z 值
let mut draw_list: Vec<(i32, DrawData)> = vec![];
draw_list.push((0, back_layer_data));     // 背景
draw_list.push((1000, middle_layer_data)); // 中间层
draw_list.push((2000, front_layer_data));  // 前景

// 2. 按 z 值排序（从小到大 = 从后往前）
draw_list.sort_by_key(|(z, _)| *z);

// 3. 按顺序绘制
for (_z, data) in draw_list {
    canvas.draw(&data.texture, DrawParam::default()
        .dest([data.x, data.y])
        // 注意：这里的 .z() 可能不生效（单个 draw 调用）
        // 但顺序已经正确了
    );
}
```

**优势**：
- ✅ 完全可控
- ✅ 支持不同纹理
- ✅ 不依赖 GGEZ 内部排序

**我们的实现**：
```rust
// src/bin/map_viewer_ecs.rs 第 390-420 行
visible_with_sort_key.sort_by(|a, b| {
    match a.1.cmp(&b.1) {  // 先按 z_order
        std::cmp::Ordering::Equal => a.2.cmp(&b.2),  // 相同则按 Y
        other => other,
    }
});
```

---

## 🎨 典型 Z 值分配方案

### 游戏场景分层示例

```rust
// 背景层 (0-999)
const Z_SKY: i32 = 0;
const Z_FAR_MOUNTAINS: i32 = 100;
const Z_BACKGROUND: i32 = 500;

// 地面层 (1000-1999)
const Z_GROUND: i32 = 1000;
const Z_GROUND_DECALS: i32 = 1100;

// 游戏对象层 (2000-8999)
const Z_ITEMS: i32 = 2000;
const Z_NPCs: i32 = 3000;
const Z_PLAYER: i32 = 4000;
const Z_BUILDINGS: i32 = 5000;
const Z_EFFECTS: i32 = 6000;

// UI层 (9000+)
const Z_UI_BACKGROUND: i32 = 9000;
const Z_UI_ELEMENTS: i32 = 9500;
const Z_UI_TOOLTIPS: i32 = 10000;
```

### 传奇地图分层（本项目）

```rust
// MapTile.z_order 定义
TileLayer::Back   => z_order = 0     // 地面
TileLayer::Middle => z_order = 1000  // 物体
TileLayer::Front  => z_order = 2000  // 建筑物顶部
```

**动态 Z 调整示例**：
```rust
// 根据 Y 坐标微调（实现上下遮挡）
let z = base_z + (grid_y as i32);  // Y 越大越靠前

// 特殊物体提升
if is_important {
    z += 1000;  // 提升到更前面
}
```

---

## ⚠️ 常见陷阱

### 陷阱 1：单个 draw 调用 z 参数可能不生效

```rust
// ❌ 错误：期望 z 参数自动排序
canvas.draw(&texture1, DrawParam::default().z(100));
canvas.draw(&texture2, DrawParam::default().z(0));  // 仍然后绘制

// ✅ 正确：手动控制顺序
canvas.draw(&texture2, DrawParam::default().z(0));   // 先画背景
canvas.draw(&texture1, DrawParam::default().z(100)); // 后画前景
```

### 陷阱 2：忘记设置 InstanceArray.ordered

```rust
// ❌ 错误：不会自动排序
let instances = InstanceArray::new(ctx, texture);
// ... 添加实例 ...
canvas.draw(&instances, DrawParam::default());

// ✅ 正确
let mut instances = InstanceArray::new(ctx, texture);
instances.set_ordered(true);  // 必须！
```

### 陷阱 3：Z 值范围太小导致冲突

```rust
// ❌ 错误：容易冲突
const Z_LAYER1: i32 = 0;
const Z_LAYER2: i32 = 1;
const Z_LAYER3: i32 = 2;

// ✅ 正确：留足空间
const Z_LAYER1: i32 = 0;
const Z_LAYER2: i32 = 1000;
const Z_LAYER3: i32 = 2000;
```

---

## 🚀 性能对比

| 方法 | 绘制调用次数 | 排序开销 | 适用场景 |
|------|------------|---------|---------|
| **InstanceArray (ordered)** | 1 次 | GGEZ 内部 | 大量相同纹理 |
| **手动排序 + 单独绘制** | N 次 | 预先排序 | 不同纹理、复杂逻辑 |
| **批量渲染（按 BlendMode）** | 2-3 次 | 预先分组 | 本项目方案 |

**本项目策略**：
```rust
// 1. 手动排序（按 z_order + Y 坐标）
visible_with_sort_key.sort_by(...)

// 2. 按 BlendMode 分组（减少状态切换）
for tile in normal_tiles { draw(tile); }  // 一次状态设置
for tile in blend_tiles { draw(tile); }   // 一次状态设置

// 结果：5000+ 绘制调用，但只有 2 次状态切换
```

---

## 📝 总结

### DrawParam.z() 什么时候有用？

✅ **有用的情况**：
- 使用 `InstanceArray` 且设置 `ordered=true`
- 批量绘制相同纹理的大量对象
- 粒子系统、瓦片地图（使用 InstanceArray）

❌ **不一定有用的情况**：
- 单个 `canvas.draw()` 调用
- 不同纹理之间的排序
- 复杂的分层逻辑

### 推荐方案

1. **简单场景**：手动控制绘制顺序（最可靠）
2. **大量相同纹理**：InstanceArray + ordered=true
3. **复杂场景**：手动排序 + 批量渲染（本项目）

### 本项目选择

我们选择了 **手动排序 + 批量渲染**：
- ✅ 完全可控的绘制顺序
- ✅ 支持不同纹理和混合模式
- ✅ 减少状态切换（2 次 vs 5000+ 次）
- ✅ 性能优秀（35-160 FPS）

---

## 🔗 参考资料

- GGEZ 官方文档: https://docs.rs/ggez/latest/ggez/
- DrawParam API: https://docs.rs/ggez/latest/ggez/graphics/struct.DrawParam.html
- InstanceArray API: https://docs.rs/ggez/latest/ggez/graphics/struct.InstanceArray.html
