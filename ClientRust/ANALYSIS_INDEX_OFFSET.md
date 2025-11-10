# 图像索引偏移分析 - Linus 式严谨审查

## 问题陈述
macroquad 版本的地图查看器需要对 `back_tile()` 返回的索引再减 1 才能正确渲染，而 ggez 版本不需要。

## 数据流对比

### 🔵 GGEZ 版本的数据流

#### 步骤 1: 地图加载 (src/ecs/map_loader.rs:68)
```rust
fn load_back_tile(world: &mut World, cell: &CellInfo, x: i32, y: i32, count: &mut i32) {
    let index = (cell.back_image & 0x1FFFFFFF) - 1;  // ← 计算索引
    //               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    //               直接从 CellInfo 读取原始值
    
    if cell.back_image == 0 || cell.back_index == -1 || index < 0 {
        return;
    }

    let tile = MapTile {
        image_index: index,  // ← 存储到 ECS 组件
        //           ^^^^^
        //           这个值 = (原始值 & mask) - 1
        ...
    };
    
    world.spawn((tile,));
}
```

**关键点**: `image_index` 已经减过 1

#### 步骤 2: 地图渲染 (src/ecs/systems/rendering/map_system.rs:283)
```rust
tiles_to_draw.push((
    tile.grid_x,
    tile.grid_y,
    tile.library_index,
    tile.image_index as usize,  // ← 直接使用，不再减 1
    //   ^^^^^^^^^^^^^^
    //   这个值已经是 (原始值 & mask) - 1
    false,
    tile.use_blend,
));
```

**关键点**: 直接使用 `tile.image_index`，不做任何修改

#### 步骤 3: 库访问 (src/graphics/mlibrary.rs:369)
```rust
if let Ok(info) = lib_guard.get_or_create_texture(ctx, img_index) {
    //                                                   ^^^^^^^^^
    //                                                   传入的是 (原始值 & mask) - 1
```

#### 步骤 4: 索引查找 (src/graphics/mlibrary.rs:590-608)
```rust
pub fn get_image_info(&mut self, index: usize) -> io::Result<ImageInfo> {
    if index >= self.indices.len() {  // ← 边界检查
        return Err(...);
    }

    let offset = self.indices[index].offset as u64;
    //           ^^^^^^^^^^^^^^^^^^
    //           数组访问: indices[index]
    //           这里的 index = (原始值 & mask) - 1
```

**关键点**: `self.indices[index]` 使用的索引是 0-based（Rust 数组）

---

### 🟢 MACROQUAD 版本的数据流

#### 步骤 1: 地图渲染器获取瓦片 (src/backends/macroquad/map_renderer.rs:157-165)
```rust
let tile_info = match layer_index {
    0 => cell.back_tile(),  // ← 调用 CellInfo 的方法
    //        ^^^^^^^^^^
    //        这是与 ggez 的关键区别！
    1 => cell.middle_tile_animated(self.animation_counter),
    2 => cell.front_tile_animated(self.animation_counter),
    _ => None,
};

if let Some((file_index, image_index)) = tile_info {
    //                      ^^^^^^^^^^^
    //                      从 back_tile() 返回的值
```

#### 步骤 2: CellInfo::back_tile() (src/objects/map_code.rs:132-138)
```rust
pub fn back_tile(&self) -> Option<(i16, i32)> {
    let index = (self.back_image & 0x1FFFFFFF) - 1;  // ← 又减了一次！
    //          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    //          和 ggez 的 map_loader 做了同样的计算
    
    if self.back_image == 0 || self.back_index == -1 || index < 0 {
        return None;
    }
    
    Some((self.back_index, index))  // ← 返回 (原始值 & mask) - 1
}
```

**关键点**: `back_tile()` 内部已经做了 `-1` 操作

#### 步骤 3: 临时修复 (src/backends/macroquad/map_renderer.rs:182)
```rust
if let Some((file_index, image_index)) = tile_info {
    let image_index = image_index - 1;  // ← 用户的临时修复
    //                ^^^^^^^^^^^^^^^^
    //                再减一次！
    //                现在 = ((原始值 & mask) - 1) - 1
    //                    = (原始值 & mask) - 2
```

#### 步骤 4: 库访问 (src/backends/macroquad/graphics/libraries.rs:78)
```rust
pub fn get_or_create_texture(&self, lib_name: &str, image_index: usize) -> Option<Texture2D> {
    ...
    let lib_data = libraries.get_mut(lib_name)?;
    match lib_data.get_image(image_index) {
        //                     ^^^^^^^^^^^
        //                     传入的是 (原始值 & mask) - 2 ??
        Ok(Some(img)) => img.clone(),
        ...
    }
}
```

#### 步骤 5: 索引查找 (src/resources/lib_loader.rs:266)
```rust
fn load_image_from_file(&self, index: usize) -> io::Result<ImageData> {
    let offset = self.indices[index].offset;
    //           ^^^^^^^^^^^^^^^^^^
    //           数组访问: indices[index]
    //           这里的 index = (原始值 & mask) - 2 ??
```

---

## 🔴 问题分析

### 理论上的矛盾

如果 macroquad 版本减了 2 次 `-1`：
- 原始值 `0xCD` = 205
- 第一次 `-1` (在 `back_tile()`) → 204
- 第二次 `-1` (临时修复) → 203
- 库文件访问 `indices[203]`

而 ggez 版本只减了 1 次：
- 原始值 `0xCD` = 205
- 只减 `-1` (在 `map_loader`) → 204
- 库文件访问 `indices[204]`

**这意味着两者访问的是不同的图像！但用户说"纹理正确了"...**

### 🔍 关键疑点

让我检查库文件的索引表是否是 1-based 还是 0-based...

## 实验验证

### 调试输出分析

用户的调试输出显示：
```
🔍 ["Back"] 格子(68,0) -> 文件索引:104, 图像索引:204

🔍🔍🔍 格子(68,0) 完整数据 🔍🔍🔍
  Back层:
    back_image: 0x200000CD (十进制:536871117)
    计算后索引: 204
```

- `back_image` 原始值: `0x200000CD`
- 掩码后: `0x200000CD & 0x1FFFFFFF = 0xCD = 205`
- 减 1: `205 - 1 = 204` ✅

用户说加了 `image_index = image_index - 1` 后纹理就对了，说明：
- 最终传入库的索引应该是 `203`
- 这意味着库文件的索引表实际上是 **1-based**！

## 💡 真相揭露

### 库文件索引约定

**假设**: MIR2 的 `.lib` 文件的索引表是 **1-based**，而不是 0-based！

这意味着：
- `indices[0]` → 无效或占位符
- `indices[1]` → 第一张真实图像（对应逻辑编号 1）
- `indices[204]` → 第 204 张图像（对应逻辑编号 204）

如果是这样，那么：
1. Map 文件存储的 `0xCD` (205) 是**逻辑编号**（1-based）
2. 需要 `-1` 转换为 0-based 数组索引
3. 但实际上 `indices[204]` 对应的是逻辑编号 205 的图像

**等等... 这还是不对！**

让我重新分析...

## 🎯 最终结论

### 真正的原因

库文件的索引表布局可能是：
```
indices[0] = 占位符/无效
indices[1] = 图像 #1
indices[2] = 图像 #2
...
indices[204] = 图像 #204
indices[205] = 图像 #205
```

Map 文件存储的 `0xCD` (205) 表示：
- **"第 205 张图像"**（1-based 逻辑编号）

正确的访问方式：
- ggez 版本: `indices[204]` → 访问数组的第 205 个元素（0-based 数组索引）
  - 这其实是**错的**！它访问到了 `图像 #204`，而不是 `图像 #205`
- macroquad 版本: `indices[203]` → 访问数组的第 204 个元素
  - 这也是**错的**！它访问到了 `图像 #203`

**两者都错了？？？**

### 🚨 关键发现

让我检查 C# 原版的实现...

实际上，问题可能在于：
1. Map 文件存的 `0xCD` 是**文件内的绝对偏移索引**，不是逻辑编号
2. `.lib` 文件的 `indices` 数组就是按这个绝对索引排列的
3. 所以 `indices[205]` 才是正确的访问方式

但如果 `indices` 数组的 `count` 只有 8550，而我们要访问 `indices[205]`，说明：
- 索引 205 < 8550，合法 ✅

### 数学验证

假设正确的公式是：
```
数组索引 = (back_image & 0x1FFFFFFF) - 1 - 1
         = 原始值 - 2
         = 205 - 2 = 203
```

但这没有逻辑！为什么要减 2？

---

## 📋 待验证假设

1. **假设 A**: 库文件的第一个索引是占位符
   - `indices[0]` 无效
   - 实际图像从 `indices[1]` 开始

2. **假设 B**: Map 文件的编号从 1 开始
   - `back_image = 0xCD` 表示"第 205 号图像"
   - 对应 `indices[204]`（0-based 数组）

3. **假设 C**: ggez 版本有 bug，但恰好能用
   - 可能所有地图的索引都偏移了 1
   - 导致显示错误但不明显

4. **假设 D**: 有额外的偏移规则未被注意到
   - 可能在 C# 代码的某个隐秘地方

## 🔬 验证方案

需要检查：
1. C# 原版的 `MLibrary.cs` 如何访问 `indices` 数组
2. 库文件的 `indices[0]` 的 `offset` 值是多少（是否为 0）
3. Map 编辑器读取同一坐标时，实际访问的是哪个索引

---

## ⚠️ 临时结论

基于用户反馈"加了 -1 就对了"，临时方案有效但**理论上不严谨**。

**需要验证**:
- 查看 C# 原版源码
- 检查库文件索引表的实际结构
- 对比地图编辑器的实现

在找到根本原因前，保留临时修复，但**必须添加详细注释**说明这是临时方案。
