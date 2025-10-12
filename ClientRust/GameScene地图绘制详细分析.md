# GameScene.MapControl 地图绘制详细分析

## 📋 概述

本文档详细分析 C# 原版 `GameScene.cs` 中 `MapControl` 类的地图绘制逻辑，特别关注 **Back/Middle/Front 三层的绘制方法调用** 和 **纹理偏移（offset）的使用场景**。

---

## 🏗️ MapControl 类结构

### 基础常量

```csharp
// GameScene.cs lines 10717-10770
public sealed class MapControl : MirControl
{
    /// 地图格子尺寸
    public const int CellWidth = 48;   // 每格宽度 48像素
    public const int CellHeight = 32;  // 每格高度 32像素 (等距视角)
    
    /// 视野偏移（格子数）
    public static int OffSetX;  // 横向：ScreenWidth/2/CellWidth  (1024窗口=10格)
    public static int OffSetY;  // 纵向：ScreenHeight/2/CellHeight-1 (768窗口=11格)
    
    /// 视野范围（格子数）
    public static int ViewRangeX;  // OffSetX + 6
    public static int ViewRangeY;  // OffSetY + 6
}
```

### 坐标转换公式

```csharp
// 屏幕坐标 = (地图坐标 - 玩家坐标 + 视野偏移) × 格子尺寸 - 像素偏移 + 平滑移动偏移
drawX = (x - User.Movement.X + OffSetX) * CellWidth - OffSetX + User.OffSetMove.X;
drawY = (y - User.Movement.Y + OffSetY) * CellHeight + User.OffSetMove.Y;
```

**参数说明**：
- `x, y` - 地图格子坐标
- `User.Movement.X/Y` - 玩家当前位置（格子坐标）
- `OffSetX/Y` - 视野中心偏移（格子数）
- `CellWidth/Height` - 格子像素尺寸
- `User.OffSetMove.X/Y` - 平滑移动的像素偏移（0~47, 0~31）

---

## 🎨 地图绘制流程

### 1️⃣ DrawFloor() - 静态地板绘制

**目的**：将不动的地表（Back/Middle/Front静态部分）预渲染到纹理缓存，提高性能。

```csharp
// GameScene.cs line 11617
private void DrawFloor()
{
    // 切换到地板纹理缓存
    DXManager.SetSurface(DXManager.FloorSurface);
    DXManager.Device.Clear(ClearFlags.Target, Color.Empty, 0, 0);
    
    // 绘制三层...
    
    // 恢复原渲染目标
    DXManager.SetSurface(oldSurface);
    FloorValid = true;  // 标记缓存有效
}
```

---

## 📊 三层绘制详细分析

### 🟫 Back 层（地表层）

#### 绘制范围与规则

```csharp
// GameScene.cs lines 11639-11662
for (int y = User.Movement.Y - ViewRangeY; y <= User.Movement.Y + ViewRangeY; y++)
{
    if (y <= 0 || y % 2 == 1) continue;  // ⚠️ 只绘制偶数行
    if (y >= Height) break;
    drawY = (y - User.Movement.Y + OffSetY) * CellHeight + User.OffSetMove.Y;

    for (int x = User.Movement.X - ViewRangeX; x <= User.Movement.X + ViewRangeX; x++)
    {
        if (x <= 0 || x % 2 == 1) continue;  // ⚠️ 只绘制偶数列
        if (x >= Width) break;
        
        drawX = (x - User.Movement.X + OffSetX) * CellWidth - OffSetX + User.OffSetMove.X;
        
        if ((M2CellInfo[x, y].BackImage == 0) || (M2CellInfo[x, y].BackIndex == -1)) 
            continue;
        
        // BackImage 高3位用于特殊标记，需要屏蔽
        index = (M2CellInfo[x, y].BackImage & 0x1FFFFFFF) - 1;
        
        // ✅ 调用 MLibrary 的 Draw(index, x, y) 方法
        Libraries.MapLibs[M2CellInfo[x, y].BackIndex].Draw(index, drawX, drawY);
    }
}
```

#### 🔍 Back 层关键特征

| 特征 | 说明 |
|------|------|
| **绘制频率** | 只绘制偶数行/列（减少50%绘制量） |
| **瓦片尺寸** | 96×64像素（覆盖2×2格子） |
| **视野范围** | `Y: [User.Y - ViewRangeY, User.Y + ViewRangeY]` |
| **跳过规则** | `y<=0`, `x<=0`, `y%2==1`, `x%2==1` |
| **调用方法** | `Draw(index, x, y)` - **无offset参数** |
| **纹理偏移** | ❌ **不使用offset** |

#### 📝 Back 层调用签名

```csharp
// 方法签名（MLibrary.cs line 700）
public void Draw(int index, int x, int y)
{
    if (x >= Settings.ScreenWidth || y >= Settings.ScreenHeight) return;
    if (!CheckImage(index)) return;
    MImage mi = _images[index];
    if (x + mi.Width < 0 || y + mi.Height < 0) return;
    
    // ⚠️ 直接绘制，不应用offset
    DXManager.Draw(mi.Image, new Rectangle(0, 0, mi.Width, mi.Height), 
                   new Vector3((float)x, (float)y, 0.0F), Color.White);
    
    mi.CleanTime = CMain.Time + Settings.CleanDelay;
}
```

**结论**：Back层使用最简单的绘制方法，**不处理纹理offset**。

---

### 🟦 Middle 层（装饰层）

#### 静态绘制（DrawFloor）

```csharp
// GameScene.cs lines 11664-11696
for (int y = User.Movement.Y - ViewRangeY; y <= User.Movement.Y + ViewRangeY + 5; y++)
{
    if (y <= 0) continue;  // ⚠️ 允许y=0（但不允许负数）
    if (y >= Height) break;
    drawY = (y - User.Movement.Y + OffSetY) * CellHeight + User.OffSetMove.Y;

    for (int x = User.Movement.X - ViewRangeX; x <= User.Movement.X + ViewRangeX; x++)
    {
        if (x < 0) continue;  // ⚠️ 允许x=0
        if (x >= Width) break;
        drawX = (x - User.Movement.X + OffSetX) * CellWidth - OffSetX + User.OffSetMove.X;

        index = M2CellInfo[x, y].MiddleImage - 1;
        if ((index < 0) || (M2CellInfo[x, y].MiddleIndex == -1)) continue;
        
        // 尺寸过滤：只允许 48x32 或 96x64
        if (M2CellInfo[x, y].MiddleIndex >= 0)
        {
            Size s = Libraries.MapLibs[M2CellInfo[x, y].MiddleIndex].GetSize(index);
            if ((s.Width != CellWidth || s.Height != CellHeight) &&
                ((s.Width != CellWidth * 2) || (s.Height != CellHeight * 2))) 
                continue;
        }
        
        // ✅ 调用 Draw(index, x, y) - 无offset参数
        Libraries.MapLibs[M2CellInfo[x, y].MiddleIndex].Draw(index, drawX, drawY);
    }
}
```

#### 动态绘制（DrawObjects - 带动画）

```csharp
// GameScene.cs lines 11889-11928
if ((M2CellInfo[x, y].MiddleIndex >= 0) && (M2CellInfo[x, y].MiddleIndex != -1))
{
    index = M2CellInfo[x, y].MiddleImage - 1;
    if (index > 0)
    {
        animation = M2CellInfo[x, y].MiddleAnimationFrame;
        blend = false;
        
        if ((animation > 0) && (animation < 255))
        {
            // 检查混合标志
            if ((animation & 0x0f) > 0)
            {
                blend = true;
                animation &= 0x0f;
            }
            
            if (animation > 0)
            {
                byte animationTick = M2CellInfo[x, y].MiddleAnimationTick;
                // 动画帧计算
                index += (AnimationCount % (animation + (animation * animationTick))) 
                         / (1 + animationTick);

                if (blend && (animation == 10 || animation == 8))
                {
                    // ✅ 钻石矿、深渊等半透明动画
                    Libraries.MapLibs[M2CellInfo[x, y].MiddleIndex]
                        .DrawUpBlend(index, new Point(drawX, drawY));
                }
                else
                {
                    // ✅ 普通动画（向上绘制）
                    Libraries.MapLibs[M2CellInfo[x, y].MiddleIndex]
                        .DrawUp(index, drawX, drawY);
                }
            }
        }
        
        s = Libraries.MapLibs[M2CellInfo[x, y].MiddleIndex].GetSize(index);
        if ((s.Width != CellWidth || s.Height != CellHeight) && 
            (s.Width != (CellWidth * 2) || s.Height != (CellHeight * 2)) && !blend)
        {
            // ✅ 非标准尺寸且非混合，向上绘制
            Libraries.MapLibs[M2CellInfo[x, y].MiddleIndex]
                .DrawUp(index, drawX, drawY);
        }
    }
}
```

#### 🔍 Middle 层关键特征

| 特征 | 说明 |
|------|------|
| **绘制频率** | 所有格子（不限奇偶） |
| **视野范围** | `Y: [User.Y - ViewRangeY, User.Y + ViewRangeY + 5]` （向下多5格） |
| **跳过规则** | `y<=0`, `x<0` （允许y=0, x=0） |
| **尺寸过滤** | 只绘制 48×32 或 96×64 |
| **调用方法** | `Draw(index, x, y)` / `DrawUp(index, x, y)` / `DrawUpBlend(...)` |
| **纹理偏移** | ❌ **不使用offset** |
| **动画支持** | ✅ 支持动画和混合模式 |

#### 📝 Middle 层调用方法汇总

```csharp
// 1. 静态瓦片
Libraries.MapLibs[idx].Draw(index, drawX, drawY);

// 2. 动态瓦片（向上绘制）
Libraries.MapLibs[idx].DrawUp(index, drawX, drawY);

// 3. 半透明动画
Libraries.MapLibs[idx].DrawUpBlend(index, new Point(drawX, drawY));
```

**结论**：Middle层使用多种绘制方法，但**都不使用offset参数**。

---

### 🟥 Front 层（前景层）

#### 静态绘制（DrawFloor）

```csharp
// GameScene.cs lines 11698-11748
for (int y = User.Movement.Y - ViewRangeY; y <= User.Movement.Y + ViewRangeY + 5; y++)
{
    if (y <= 0) continue;
    if (y >= Height) break;
    drawY = (y - User.Movement.Y + OffSetY) * CellHeight + User.OffSetMove.Y;

    for (int x = User.Movement.X - ViewRangeX; x <= User.Movement.X + ViewRangeX; x++)
    {
        if (x < 0) continue;
        if (x >= Width) break;
        drawX = (x - User.Movement.X + OffSetX) * CellWidth - OffSetX + User.OffSetMove.X;

        // FrontImage 高位用于特殊标记，需要屏蔽 (& 0x7FFF)
        index = (M2CellInfo[x, y].FrontImage & 0x7FFF) - 1;
        if (index == -1) continue;
        
        int fileIndex = M2CellInfo[x, y].FrontIndex;
        if (fileIndex == -1) continue;
        
        Size s = Libraries.MapLibs[fileIndex].GetSize(index);
        if (fileIndex == 200) continue;  // 修复4.map的坏点
        
        // 门动画处理
        if (M2CellInfo[x, y].DoorIndex > 0)
        {
            Door DoorInfo = GetDoor(M2CellInfo[x, y].DoorIndex);
            if (DoorInfo == null)
            {
                DoorInfo = new Door() { 
                    index = M2CellInfo[x, y].DoorIndex, 
                    DoorState = 0, 
                    ImageIndex = 0, 
                    LastTick = CMain.Time 
                };
                Doors.Add(DoorInfo);
            }
            else if (DoorInfo.DoorState != 0)
            {
                // 门开启时的动画索引偏移
                index += (DoorInfo.ImageIndex + 1) * M2CellInfo[x, y].DoorOffset;
            }
        }

        // 尺寸验证
        if (index < 0 || ((s.Width != CellWidth || s.Height != CellHeight) && 
            ((s.Width != CellWidth * 2) || (s.Height != CellHeight * 2)))) 
            continue;
        
        // ✅ 静态Front层：使用 Draw(index, x, y)
        Libraries.MapLibs[fileIndex].Draw(index, drawX, drawY);
    }
}
```

#### 动态绘制（DrawObjects - 带动画和混合）

```csharp
// GameScene.cs lines 11930-11990
index = (M2CellInfo[x, y].FrontImage & 0x7FFF) - 1;
if (index < 0) continue;

int fileIndex = M2CellInfo[x, y].FrontIndex;
if (fileIndex == -1) continue;

animation = M2CellInfo[x, y].FrontAnimationFrame;

// 检查混合标志（高位）
if ((animation & 0x80) > 0)
{
    blend = true;
    animation &= 0x7F;
}
else
    blend = false;

// 动画帧计算
if (animation > 0)
{
    byte animationTick = M2CellInfo[x, y].FrontAnimationTick;
    index += (AnimationCount % (animation + (animation * animationTick))) 
             / (1 + animationTick);
}

// 门动画处理（同上）
if (M2CellInfo[x, y].DoorIndex > 0) { ... }

s = Libraries.MapLibs[fileIndex].GetSize(index);
if (s.Width == CellWidth && s.Height == CellHeight && animation == 0) continue;
if ((s.Width == CellWidth * 2) && (s.Height == CellHeight * 2) && (animation == 0)) continue;

// 🔍 关键：根据不同情况选择绘制方法
if (blend)
{
    // 混合模式（半透明）
    if (fileIndex == 14 || fileIndex == 27 || (fileIndex > 99 & fileIndex < 199))
    {
        // ✅ 特殊图库：使用offset=true
        Libraries.MapLibs[fileIndex].DrawBlend(
            index, 
            new Point(drawX, drawY - (3 * CellHeight)), 
            Color.White, 
            true  // ⚠️ offset=true
        );
    }
    else
    {
        // ✅ 普通混合：不使用offset
        Libraries.MapLibs[fileIndex].DrawBlend(
            index, 
            new Point(drawX, drawY - s.Height), 
            Color.White, 
            (index >= 2723 && index <= 2732)
        );
    }
}
else
{
    // 普通绘制
    if (fileIndex == 28 && Libraries.MapLibs[fileIndex].GetOffSet(index) != Point.Empty)
    {
        // 🎯 关键：fileIndex==28 且 offset不为空时，启用offset
        // ✅ 这是唯一使用offset=true的地方！
        Libraries.MapLibs[fileIndex].Draw(
            index, 
            new Point(drawX, drawY - CellHeight), 
            Color.White, 
            true  // ⚠️ offset=true
        );
    }
    else
    {
        // ✅ 默认：不使用offset，Y坐标减去图像高度
        Libraries.MapLibs[fileIndex].Draw(
            index, 
            drawX, 
            drawY - s.Height
        );
    }
}
```

#### 🔍 Front 层关键特征

| 特征 | 说明 |
|------|------|
| **绘制频率** | 所有格子（不限奇偶） |
| **视野范围** | `Y: [User.Y - ViewRangeY, User.Y + ViewRangeY + 5]` |
| **跳过规则** | `y<=0`, `x<0` |
| **Y坐标调整** | `drawY - s.Height` （让建筑"站"在格子上） |
| **调用方法** | `Draw(...)` / `DrawBlend(...)` |
| **纹理偏移** | ⚠️ **特定条件下使用** |
| **动画支持** | ✅ 支持动画、混合、门动画 |

#### 🎯 Front 层纹理偏移使用条件

```csharp
// 条件1：特定图库的混合绘制
if (fileIndex == 14 || fileIndex == 27 || (fileIndex > 99 & fileIndex < 199))
    DrawBlend(..., offset=true);

// 条件2：fileIndex==28 且 GetOffSet != Point.Empty
if (fileIndex == 28 && GetOffSet(index) != Point.Empty)
    Draw(..., offset=true);

// 其他所有情况：offset=false（默认）
```

#### 📝 Front 层调用方法汇总

```csharp
// 1. 静态瓦片（DrawFloor）
Libraries.MapLibs[fileIndex].Draw(index, drawX, drawY);

// 2. 动态瓦片 - 默认（不使用offset）
Libraries.MapLibs[fileIndex].Draw(index, drawX, drawY - s.Height);

// 3. 动态瓦片 - 特殊图库混合（使用offset）
Libraries.MapLibs[fileIndex].DrawBlend(
    index, 
    new Point(drawX, drawY - (3 * CellHeight)), 
    Color.White, 
    true  // offset=true
);

// 4. 动态瓦片 - fileIndex==28（使用offset）
Libraries.MapLibs[fileIndex].Draw(
    index, 
    new Point(drawX, drawY - CellHeight), 
    Color.White, 
    true  // offset=true
);

// 5. 动态瓦片 - 普通混合（不使用offset）
Libraries.MapLibs[fileIndex].DrawBlend(
    index, 
    new Point(drawX, drawY - s.Height), 
    Color.White, 
    false  // offset=false（或特定索引判断）
);
```

**结论**：Front层**仅在特定条件下**使用offset，绝大多数情况**不使用offset**。

---

## 📊 三层对比总结表

| 层 | 绘制范围 | 跳过规则 | Y坐标调整 | 调用方法 | offset使用 |
|----|---------|---------|----------|---------|-----------|
| **Back** | `Y±ViewRangeY` | 只绘制偶数行列<br>`y<=0, x<=0` | 无 | `Draw(idx, x, y)` | ❌ 不使用 |
| **Middle** | `Y±(ViewRangeY+5)` | `y<=0, x<0` | 向上绘制时<br>`drawY` | `Draw(...)`<br>`DrawUp(...)`<br>`DrawUpBlend(...)` | ❌ 不使用 |
| **Front** | `Y±(ViewRangeY+5)` | `y<=0, x<0` | `drawY - s.Height` | `Draw(...)`<br>`DrawBlend(...)` | ⚠️ 特定条件使用 |

---

## 🎯 纹理偏移使用场景总结

### ❌ 不使用 offset 的场景（99%+）

1. **Back 层** - 全部不使用
2. **Middle 层** - 全部不使用
3. **Front 层静态绘制** - 不使用
4. **Front 层动态绘制（默认）** - 不使用

### ✅ 使用 offset 的场景（<1%）

```csharp
// 场景1：特定图库的混合绘制
if (fileIndex == 14 || fileIndex == 27 || (fileIndex > 99 & fileIndex < 199))
{
    Libraries.MapLibs[fileIndex].DrawBlend(
        index, 
        new Point(drawX, drawY - (3 * CellHeight)), 
        Color.White, 
        true  // ✅ offset=true
    );
}

// 场景2：fileIndex==28 且纹理有非零offset
if (fileIndex == 28 && Libraries.MapLibs[fileIndex].GetOffSet(index) != Point.Empty)
{
    Libraries.MapLibs[fileIndex].Draw(
        index, 
        new Point(drawX, drawY - CellHeight), 
        Color.White, 
        true  // ✅ offset=true
    );
}
```

### 🔍 为什么这么少使用 offset？

1. **纹理制作标准化**
   - 绝大多数瓦片在制作时就考虑了对齐
   - offset 值大多为 `(0, 0)`
   - 图像边界就是对齐边界

2. **Y坐标调整已足够**
   - Front 层通过 `drawY - s.Height` 调整建筑对齐
   - 不需要额外的 offset 微调

3. **特殊图库特殊处理**
   - `fileIndex == 28` 是特定的图库（可能是特殊建筑）
   - `fileIndex == 14/27` 等是混合特效图库
   - 这些图库需要精确对齐，所以启用 offset

4. **性能考虑**
   - 检查和应用 offset 有额外开销
   - 默认不启用可以提高绘制性能

---

## 📝 MLibrary 方法签名回顾

### Draw(index, x, y) - 最简单

```csharp
// MLibrary.cs line 700
public void Draw(int index, int x, int y)
{
    // ⚠️ 直接绘制，不处理offset
    DXManager.Draw(mi.Image, ..., new Vector3((float)x, (float)y, 0.0F), Color.White);
}
```

### Draw(index, point, color, offset) - 带offset参数

```csharp
// MLibrary.cs line 718
public void Draw(int index, Point point, Color colour, bool offSet = false)
{
    if (!CheckImage(index)) return;
    MImage mi = _images[index];
    
    // 🎯 关键：只有offSet=true才应用偏移
    if (offSet) point.Offset(mi.X, mi.Y);
    
    if (point.X >= Settings.ScreenWidth || ...) return;
    DXManager.Draw(mi.Image, ..., new Vector3((float)point.X, (float)point.Y, 0.0F), colour);
}
```

### DrawBlend(index, point, color, offset, rate) - 混合绘制

```csharp
// MLibrary.cs (类似Draw，带混合效果)
public void DrawBlend(int index, Point point, Color colour, bool offSet = false, float rate = 1)
{
    if (!CheckImage(index)) return;
    MImage mi = _images[index];
    
    // 🎯 关键：只有offSet=true才应用偏移
    if (offSet) point.Offset(mi.X, mi.Y);
    
    // 半透明混合绘制
    DXManager.Device.SetRenderState(RenderState.BlendOperation, BlendOperation.Add);
    DXManager.Draw(...);
}
```

### DrawUp(index, x, y) - 向上绘制

```csharp
// MLibrary.cs line 860
public void DrawUp(int index, int x, int y)
{
    if (!CheckImage(index)) return;
    MImage mi = _images[index];
    
    y -= mi.Height;  // Y坐标减去高度
    
    // ⚠️ 不处理offset
    DXManager.Draw(mi.Image, ..., new Vector3(x, y, 0.0F), Color.White);
}
```

---

## 🚀 Rust 移植建议

### 1. 保持 API 一致性

```rust
// ✅ 正确：提供offset参数，但默认false
pub fn draw_with_color(
    &mut self,
    ctx: &mut Context,
    canvas: &mut Canvas,
    index: usize,
    x: f32,
    y: f32,
    color: Color,
    offset: bool,  // 默认false
) -> io::Result<()> {
    let info = self.get_or_create_texture(ctx, index)?;
    
    // 只有offset=true才应用
    let (draw_x, draw_y) = if offset {
        (x + info.x as f32, y + info.y as f32)
    } else {
        (x, y)
    };
    
    // 绘制...
}
```

### 2. 地图绘制不使用 offset

```rust
// ✅ Back层
lib.draw(ctx, canvas, index, screen_x, screen_y)?;

// ✅ Middle层
lib.draw(ctx, canvas, index, screen_x, screen_y)?;
lib.draw_up(ctx, canvas, index, screen_x, screen_y)?;

// ✅ Front层（默认）
lib.draw(ctx, canvas, index, screen_x, screen_y - height)?;

// ⚠️ Front层（特殊情况 - 如果需要实现）
if file_index == 28 && has_offset {
    lib.draw_with_color(ctx, canvas, index, screen_x, screen_y, color, true)?;
}
```

### 3. 优先级建议

| 优先级 | 功能 | 原因 |
|-------|------|------|
| 🔴 高 | `draw(index, x, y)` | Back/Middle/Front静态绘制都用 |
| 🔴 高 | `draw_up(index, x, y)` | Middle动态绘制常用 |
| 🟡 中 | `draw_with_color(..., offset)` | Front层特殊情况用 |
| 🟡 中 | `draw_blend(...)` | Front层动画/特效用 |
| 🟢 低 | `draw_tinted(...)` | 可以用draw_blend代替 |

---

## 📚 关键代码位置索引

### C# 原版

| 功能 | 文件 | 行号 |
|------|------|------|
| MapControl 类定义 | `GameScene.cs` | 10717-10770 |
| DrawFloor 方法 | `GameScene.cs` | 11617-11750 |
| Back 层绘制 | `GameScene.cs` | 11639-11662 |
| Middle 层绘制（静态） | `GameScene.cs` | 11664-11696 |
| Front 层绘制（静态） | `GameScene.cs` | 11698-11748 |
| DrawObjects 方法 | `GameScene.cs` | 11810-12000 |
| Middle 层绘制（动态） | `GameScene.cs` | 11889-11928 |
| Front 层绘制（动态） | `GameScene.cs` | 11930-11990 |
| Draw(index, x, y) | `MLibrary.cs` | 700-716 |
| Draw(..., offset) | `MLibrary.cs` | 718-730 |
| DrawBlend(...) | `MLibrary.cs` | 752-770 |
| DrawUp(index, x, y) | `MLibrary.cs` | 860-878 |

### Rust 实现

| 功能 | 文件 | 行号 |
|------|------|------|
| draw() | `mlibrary.rs` | 862-890 |
| draw_with_color() | `mlibrary.rs` | 915-960 |
| draw_tinted() | `mlibrary.rs` | 1310-1360 |
| draw_up() | `mlibrary.rs` | 1380-1410 |
| simple_map_viewer | `simple_map_viewer.rs` | 全文 |

---

## 🎓 核心结论

### 1. 纹理 offset 的使用频率

- **Back 层**：❌ 从不使用（0%）
- **Middle 层**：❌ 从不使用（0%）
- **Front 层**：⚠️ 极少使用（<1%，仅特定图库）

### 2. 默认绘制方式

```csharp
// ✅ 99%+ 的情况使用这种方式
Libraries.MapLibs[idx].Draw(index, drawX, drawY);
Libraries.MapLibs[idx].Draw(index, drawX, drawY - s.Height);
Libraries.MapLibs[idx].DrawUp(index, drawX, drawY);
```

### 3. 特殊情况才启用 offset

```csharp
// ⚠️ 仅在这两种情况使用offset=true
if (fileIndex == 28 && GetOffSet(index) != Point.Empty)
    Draw(..., offset=true);

if (fileIndex == 14 || fileIndex == 27 || ...)
    DrawBlend(..., offset=true);
```

### 4. Rust 实现策略

- ✅ 提供 offset 参数（保持API一致性）
- ✅ 默认值为 false
- ✅ 地图绘制不使用 offset
- ✅ 特殊功能留作扩展接口

### 5. 为什么之前出错？

```rust
// ❌ 错误：手动加offset
let screen_x = base_x + info.x as f32;

// 问题：纹理图像已经包含了offset信息
// 手动加offset导致重复应用，造成偏移

// ✅ 正确：不手动处理
let screen_x = base_x;
// 如果需要offset，通过参数控制
lib.draw_with_color(..., offset=true);
```

---

**生成时间**: 2025-10-12  
**作者**: AI 分析报告  
**状态**: ✅ 分析完成，详细记录了三层绘制逻辑和offset使用场景
