# MLibrary 图像字段用途说明

## 📋 字段概述

MImage 结构中包含多个定位和渲染相关的字段：

```csharp
// 主图层字段
public short Width, Height;     // 图像实际宽高
public short X, Y;               // 图像偏移量（用于精确定位）
public short ShadowX, ShadowY;   // 阴影偏移量
public byte Shadow;              // 阴影标志（高位标记是否有Mask层）

// Mask（遮罩/混合）图层字段
public short MaskWidth, MaskHeight;  // 遮罩层宽高
public short MaskX, MaskY;           // 遮罩层偏移量
public int MaskLength;               // 遮罩层数据长度
public bool HasMask;                 // 是否有遮罩层
```

---

## 🎯 字段详细用途

### 1. **Width, Height** - 图像尺寸

**用途**: 图像的实际像素尺寸

**使用场景**:
- 创建纹理时确定缓冲区大小
- 裁剪判断（culling）- 检查图像是否在屏幕范围内
- 绘制时确定源矩形区域

**代码示例**:
```csharp
// MLibrary.cs line 708-709
if (x + mi.Width < 0 || y + mi.Height < 0)
    return;  // 超出屏幕，不绘制

// line 715
DXManager.Draw(mi.Image, 
    new Rectangle(0, 0, mi.Width, mi.Height),  // ← 使用 Width/Height
    new Vector3((float)x, (float)y, 0.0F), 
    Color.White);
```

---

### 2. **X, Y** - 图像偏移量（锚点）

**用途**: 图像相对于逻辑坐标的**绘制偏移**，用于精确对齐

**关键作用**:
- **精确定位**: 将图像的"锚点"对齐到游戏世界坐标
- **多帧对齐**: 动画不同帧的图像大小可能不同，通过偏移保证对齐
- **武器/装备对齐**: 角色身体、武器、衣服等多层图像需要精确对齐

**使用场景**:
```csharp
// MLibrary.cs line 724 - Draw 方法的 offSet 参数
public void Draw(int index, Point point, Color colour, bool offSet = false)
{
    if (offSet) 
        point.Offset(mi.X, mi.Y);  // ← 应用偏移量
    
    // 然后在调整后的位置绘制
    DXManager.Draw(mi.Image, ..., 
        new Vector3((float)point.X, (float)point.Y, 0.0F), 
        colour);
}
```

**实际使用场景**:

#### 场景1: 角色/NPC 动画帧对齐
```csharp
// NPCObject.cs line 112 - 计算最终绘制位置
FinalDrawLocation = DrawLocation.Add(BodyLibrary.GetOffSet(DrawFrame));
//                                   ^^^^^^^^^^^^^^^^^^^^^^
//                                   获取当前帧的 X,Y 偏移

// MLibrary.cs line 642 - GetOffSet 方法
public Point GetOffSet(int index)
{
    return new Point(_images[index].X, _images[index].Y);  // ← 返回 X,Y
}
```

#### 场景2: MapControl 地图物件渲染
```csharp
// GameScene.cs line 11975-11976 - 地图物件特殊偏移
// fileIndex 28 的地图物件使用偏移量精确定位
if (fileIndex == 28 && Libraries.MapLibs[fileIndex].GetOffSet(index) != Point.Empty)
    Libraries.MapLibs[fileIndex].Draw(index, 
        new Point(drawX, drawY - CellHeight), 
        Color.White, 
        true);  // ← offSet=true 应用偏移

// GameScene.cs line 12339 - 动画物件偏移（光源位置计算）
if (M2CellInfo[x, y].FrontAnimationFrame > 0)
    p.Offset(Libraries.MapLibs[fileIndex].GetOffSet(imageIndex));
    // ↑ 动画物件（如火把、旗帜）需要偏移来对齐到地图格子
```

**为什么需要偏移？**

假设角色挥剑动画有 5 帧：
```
帧1: 剑在身后, 图像尺寸 100x150, 偏移 (-20, -10)
帧2: 剑在头顶, 图像尺寸 120x180, 偏移 (-30, -25)
帧3: 剑在前方, 图像尺寸 110x160, 偏移 (-15, -12)
```

如果没有偏移量，不同帧的图像会"跳动"（因为图像大小和中心点不同）。
通过偏移量，所有帧都能对齐到角色的同一个逻辑位置（如脚底）。

---

### 3. **ShadowX, ShadowY** - 阴影偏移量

**用途**: **阴影图层相对于主图层的偏移**

**⚠️ 当前代码状态**: **未使用！**

在当前 C# 代码中，搜索结果显示 `ShadowX` 和 `ShadowY` 只在以下地方出现：
1. 字段定义 (line 922)
2. 从文件读取 (line 947-948)
3. LibraryEditor 中的转换代码

**但是在绘制代码中没有使用这两个字段！**

**推测用途** (基于传奇2设计):
```
可能的用途:
1. 地面阴影投影 - 角色脚下的椭圆阴影
2. 物体阴影 - 物体投射到地面的阴影
3. 立体效果 - 通过偏移创建深度感
```

**可能的实现方式** (未在当前代码中实现):
```csharp
// 假设的阴影绘制代码（当前不存在）
if (mi.ShadowX != 0 || mi.ShadowY != 0)
{
    // 先绘制阴影层（较暗、偏移位置）
    DXManager.DrawShadow(mi.Image, 
        new Vector3(x + mi.ShadowX, y + mi.ShadowY, 0.0F),
        Color.FromArgb(128, 0, 0, 0));  // 半透明黑色
    
    // 再绘制主图像
    DXManager.Draw(mi.Image, 
        new Vector3(x, y, 0.0F), 
        Color.White);
}
```

---

### 4. **Shadow** - 标志字节

**用途**: **多用途标志字节**（位标志）

**当前使用** (C# 代码):
```csharp
// MLibrary.cs line 955 - 检查是否有 Mask 层
HasMask = ((Shadow >> 7) == 1) ? true : false;
//         ^^^^^^^^^^^^^^^
//         检查最高位（第7位）是否为1
```

**位含义**:
```
Bit 7 (最高位): HasMask 标志
  - 0 = 没有遮罩层（单层图像）
  - 1 = 有遮罩层（双层图像）

Bit 0-6: 可能的其他标志（当前未使用）
  - 可能用于: 透明度、混合模式、渲染标志等
```

**为什么叫 "Shadow"？**
- 可能是历史遗留命名
- 或者最初用于阴影相关设置
- 现在主要用作通用标志字节

---

### 5. **MaskWidth, MaskHeight, MaskX, MaskY** - 遮罩层参数

**用途**: **第二图层（遮罩/混合层）的尺寸和偏移**

#### 什么是 Mask 层？

Mask 层是**第二个图像层**，用于实现特殊视觉效果：

**常见用途**:
1. **发光效果** - 法师技能的光芒、武器附魔光效
2. **半透明混合** - 玻璃、水面、魔法屏障等
3. **颜色叠加** - 不同颜色变体（如不同颜色的装备）
4. **动画效果** - 闪烁、脉动等特效

#### 文件结构

```
[主图层数据]
  Width, Height, X, Y      (8 bytes)
  ShadowX, ShadowY, Shadow (5 bytes)
  Length                   (4 bytes)
  [压缩的图像数据]         (Length bytes)
  
[Mask 图层数据] (如果 HasMask=true)
  MaskWidth, MaskHeight    (4 bytes)
  MaskX, MaskY             (4 bytes)
  MaskLength               (4 bytes)
  [压缩的遮罩图像数据]     (MaskLength bytes)
```

#### 使用代码

**读取 Mask 参数**:
```csharp
// MLibrary.cs line 954-962
HasMask = ((Shadow >> 7) == 1) ? true : false;
if (HasMask)
{
    reader.ReadBytes(Length);  // 跳过主图层数据
    MaskWidth = reader.ReadInt16();
    MaskHeight = reader.ReadInt16();
    MaskX = reader.ReadInt16();
    MaskY = reader.ReadInt16();
    MaskLength = reader.ReadInt32();
}
```

**创建 Mask 纹理**:
```csharp
// MLibrary.cs line 981-992
if (HasMask)
{
    reader.ReadBytes(12);  // 跳过12字节元数据（已读过）
    w = MaskWidth;         // ← 使用 MaskWidth/Height
    h = MaskHeight;
    
    MaskImage = new Texture(DXManager.Device, w, h, ...);
    stream = MaskImage.LockRectangle(0, LockFlags.Discard);
    
    DecompressImage(reader.ReadBytes(MaskLength), stream.Data);
    //                              ^^^^^^^^^^
    //                              读取 MaskLength 字节
    
    stream.Data.Dispose();
    MaskImage.UnlockRectangle(0);
}
```

**绘制时使用 Mask**:
```csharp
// MLibrary.cs line 852-855
if (mi.HasMask)
{
    // 先绘制主图像
    DXManager.Draw(mi.Image, ...);
    
    // 再叠加 Mask 层（使用 Tint 颜色混合）
    DXManager.Draw(mi.MaskImage, 
        new Rectangle(0, 0, mi.Width, mi.Height),  // ← 注意: 使用主图层尺寸！
        new Vector3((float)point.X, (float)point.Y, 0.0F), 
        Tint);  // ← 混合颜色
}
```

**⚠️ 注意**: 绘制时使用的是 **主图层** 的 Width/Height，而不是 MaskWidth/MaskHeight！
这意味着 Mask 图层会被**缩放/拉伸**到主图层大小。

#### 实际例子

**法师技能光效**:
```
主图层: 火球的固体部分 (64x64)
Mask层: 火球的光晕效果 (80x80, 偏移 (-8, -8))
        ↑ 比主图层大一圈，实现发光扩散效果
```

**武器附魔**:
```
主图层: 剑的金属纹理 (50x120)
Mask层: 剑身的魔法光芒 (50x120, 相同尺寸)
        ↑ 通过 Tint 颜色改变光芒颜色（红/蓝/绿）
```

---

## 🔍 字段使用总结

| 字段 | 状态 | 用途 | 重要性 | 使用位置 |
|------|------|------|--------|----------|
| **Width, Height** | ✅ 使用中 | 图像尺寸，裁剪判断 | 🔴 必需 | 所有Draw方法 |
| **X, Y** | ✅ 使用中 | 图像偏移，精确对齐 | 🔴 必需 | MapControl, NPCObject, PlayerObject, MonsterObject |
| **ShadowX, ShadowY** | ⚠️ 未使用 | 阴影偏移（预留） | 🟡 可选 | 仅读取，未实际使用 |
| **Shadow** | ✅ 部分使用 | 标志位（HasMask） | 🔴 必需 | HasMask 判断 |
| **MaskWidth, MaskHeight** | ✅ 使用中 | Mask层尺寸 | 🟢 Mask需要 | CreateTexture |
| **MaskX, MaskY** | ❌ 未使用 | Mask层偏移（预留） | 🟡 可选 | 仅读取，未实际使用 |

### 使用场景统计

**X, Y 偏移量的关键使用**:
1. ✅ **角色对象** (`NPCObject.cs`, `PlayerObject.cs`, `MonsterObject.cs`)
   - 动画帧对齐
   - 装备图层对齐
   
2. ✅ **地图渲染** (`GameScene.MapControl`)
   - 地图物件精确定位 (fileIndex 28)
   - 动画物件偏移 (火把、旗帜等)
   - 光源位置计算

3. ✅ **Draw方法** (`MLibrary.cs`)
   - `offSet` 参数控制是否应用偏移

---

## 💡 Rust 移植建议

### 已正确移植的字段

```rust
pub struct ImageInfo {
    pub width: i16,
    pub height: i16,
    pub x: i16,              // ✅ 已使用
    pub y: i16,              // ✅ 已使用
    pub shadow_x: i16,       // ⚠️ 保留但未使用
    pub shadow_y: i16,       // ⚠️ 保留但未使用
    pub shadow: u8,          // ✅ 用于 has_mask
    
    pub has_mask: bool,      // ✅ 已使用
    pub mask_width: i16,     // ✅ 已使用
    pub mask_height: i16,    // ✅ 已使用
    pub mask_x: i16,         // ⚠️ 保留但未使用
    pub mask_y: i16,         // ⚠️ 保留但未使用
}
```

### 当前实现状态

**✅ 正确处理**:
```rust
// mlibrary.rs - 读取字段
let shadow_x = r.read_i16::<LittleEndian>()?;
let shadow_y = r.read_i16::<LittleEndian>()?;
let shadow = r.read_u8()?;
let has_mask = (shadow >> 7) == 1;  // ✅ 正确解析

// 如果有 Mask
if has_mask {
    mask_width = r.read_i16::<LittleEndian>()?;
    mask_height = r.read_i16::<LittleEndian>()?;
    mask_x = r.read_i16::<LittleEndian>()?;
    mask_y = r.read_i16::<LittleEndian>()?;
    mask_length = r.read_i32::<LittleEndian>()?;
}
```

**✅ 正确使用 X, Y 偏移**:
```rust
// mlibrary.rs - GetOffSet 实现
pub fn get_offset(&self, index: usize) -> Option<(i16, i16)> {
    self.cached_info.get(index).map(|info| (info.x, info.y))
}
```

**⚠️ 需要实现 Mask 绘制**:

当前 Rust 代码可能缺少 Mask 层的绘制逻辑。C# 中的实现：

```csharp
// 需要在 Rust 中实现等效逻辑
if (mi.HasMask)
{
    DXManager.Draw(mi.MaskImage, ..., Tint);  // 叠加绘制 Mask 层
}
```

Rust 实现建议：
```rust
// 在 draw 方法中添加
if info.has_mask {
    if let Some(mask_image) = &info.mask_image {
        // 绘制主图像
        canvas.draw(image, draw_param);
        
        // 叠加 Mask 层（使用混合模式）
        let mask_param = DrawParam::new()
            .dest([x, y])
            .color(tint_color);  // 混合颜色
        canvas.draw(mask_image, mask_param);
    }
}
```

---

## 📚 参考

### C# 关键代码位置

1. **字段定义**: `Client/MirGraphics/MLibrary.cs` line 922-933
2. **读取逻辑**: line 941-962
3. **Mask 纹理创建**: line 981-992
4. **Mask 绘制**: line 852-855
5. **偏移量使用**: line 724, 642-655
6. **实际应用**: `Client/MirObjects/NPCObject.cs` line 112

### 相关文档

- `MLibrary移植完成度审查报告.md` - 移植状态
- `角色图像偏移量修复.md` - X,Y 偏移的实际问题案例

---

**结论**: 

- **ShadowX, ShadowY** 在当前代码中**未使用**，可能是预留字段或历史遗留
- **MaskX, MaskY** 同样**未使用**，Mask 绘制时使用主图层的位置
- **X, Y** 是**关键字段**，用于精确对齐动画帧和多层图像
- **Mask 相关字段** 用于实现**双层混合效果**（发光、特效等）

Rust 实现需要确保正确处理 Mask 层的绘制逻辑！
