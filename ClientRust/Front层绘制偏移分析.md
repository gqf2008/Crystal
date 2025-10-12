# Front层绘制偏移分析

## 问题
为什么Front层绘制时需要：
1. 向上偏移纹理高度
2. 再向下偏移一个格子高度（32像素）

即：`final_y = screen_y - info.height + CELL_HEIGHT`

## 答案

### 1. 坐标系统差异

#### DirectX (C#原版)
- **原点位置**: 左上角 (0, 0)
- **Y轴方向**: 向下为正
- **纹理锚点**: 左上角
- **绘制位置**: `Draw(texture, x, y)` 表示纹理**左上角**放在 (x, y)

#### OpenGL (ggez/Rust版)
- **原点位置**: 左下角 (0, 0)  
- **Y轴方向**: 向上为正
- **纹理锚点**: 左下角
- **绘制位置**: `draw(texture, [x, y])` 表示纹理**左下角**放在 (x, y)

### 2. ImageInfo中的offset (x, y)

在C#和Rust中，`ImageInfo.X` 和 `ImageInfo.Y` 的含义相同：

```csharp
// C# MLibrary.cs
public sealed class MImage
{
    public short Width, Height, X, Y;  // X, Y 是纹理相对锚点的偏移
}
```

```rust
// Rust mlibrary.rs
pub struct ImageInfo {
    pub width: i16,
    pub height: i16,
    pub x: i16,     // 偏移量X
    pub y: i16,     // 偏移量Y
}
```

**重要**：这个 `x, y` 是纹理**相对于锚点的偏移**，用于精确定位大型物体（如建筑、树木）的底部位置。

### 3. C#原版的绘制方式

#### DrawFloor() 中的 Back/Middle/Front 层
```csharp
// GameScene.cs Line 11617-11750
private void DrawFloor()
{
    // Back/Middle/Front 层都使用相同的绘制方式
    drawX = (x - User.Movement.X + OffSetX) * CellWidth - OffSetX + User.OffSetMove.X;
    drawY = (y - User.Movement.Y + OffSetY) * CellHeight + User.OffSetMove.Y;
    
    Libraries.MapLibs[fileIndex].Draw(index, drawX, drawY);
}
```

#### Draw(int index, int x, int y) 方法
```csharp
// MLibrary.cs Line 700-717
public void Draw(int index, int x, int y)
{
    MImage mi = _images[index];
    
    // 关键：直接使用 x, y，不应用 mi.X/mi.Y offset
    DXManager.Draw(mi.Image, 
        new Rectangle(0, 0, mi.Width, mi.Height),
        new Vector3((float)x, (float)y, 0.0F),  // 纹理左上角位置
        Color.White);
}
```

#### Draw(int index, Point point, Color colour, bool offSet = false) 方法
```csharp
// MLibrary.cs Line 718-733
public void Draw(int index, Point point, Color colour, bool offSet = false)
{
    MImage mi = _images[index];
    
    // 如果 offSet=true，应用 mi.X/mi.Y 偏移
    if (offSet) point.Offset(mi.X, mi.Y);
    
    DXManager.Draw(mi.Image, 
        new Rectangle(0, 0, mi.Width, mi.Height),
        new Vector3((float)point.X, (float)point.Y, 0.0F),
        colour);
}
```

**结论**：C#原版在 `DrawFloor()` 中使用 `Draw(index, x, y)` 重载，**不应用** `mi.X/mi.Y` 偏移！

### 4. 为什么Rust版需要偏移？

#### 问题根源
C#（DirectX）和Rust（OpenGL/ggez）的**纹理锚点不同**：

```
DirectX (C#):              OpenGL (ggez):
┌─────────┐                ┌─────────┐
│ (x,y)   │                │         │
│  ↓      │                │         │
│ texture │                │ texture │
│         │                │  ↑      │
│         │                │ (x,y)   │
└─────────┘                └─────────┘
左上角锚点                   左下角锚点
```

#### 解决方案
由于OpenGL锚点在左下角，我们需要调整Y坐标，使纹理的**视觉位置**与C#版一致。

### 5. 偏移计算详解

#### 目标
使OpenGL纹理的**左上角**对齐到C#版的 `(drawX, drawY)` 位置。

#### 步骤

1. **C#版绘制位置（左上角）**：
   ```
   drawY = (y - User.Y + OffSetY) * CellHeight
   ```

2. **OpenGL锚点在左下角，需要向上移动纹理高度**：
   ```rust
   // 让纹理左上角对齐到 drawY
   final_y = drawY - texture.height  // 向上移动整个纹理高度
   ```

3. **但这还不够！Front层特殊性**：
   
   Front层纹理（建筑、树木）通常很高，**底部**才是真正应该对齐的位置。
   
   传奇的设计是：**纹理底部对齐到格子底部**（即下一行的顶部）
   
   ```
   格子 (y):        ┌─────┐ ← y * CELL_HEIGHT
                    │     │
                    └─────┘ ← (y+1) * CELL_HEIGHT
   
   大树纹理:           🌲
                      🌲🌲
                     🌲🌲🌲  ← 树顶（很高）
                    🌲🌲🌲🌲
                    🌲🌲🌲🌲
                    ═══════ ← 树底应该对齐这里 (y+1) * CELL_HEIGHT
   ```

4. **所以需要再向下偏移一个格子高度**：
   ```rust
   final_y = drawY - texture.height + CELL_HEIGHT
   ```

#### 完整公式
```rust
// screen_y 是格子顶部的屏幕坐标 (对应 drawY)
let screen_y = world_y - camera.y + screen_height / 2.0;

// 最终绘制位置：
// 1. 向上移动纹理高度（OpenGL锚点在左下角）
// 2. 向下移动一个格子高度（使纹理底部对齐格子底部）
let final_y = screen_y - info.height as f32 + CELL_HEIGHT as f32;
```

### 6. 为什么Back/Middle层不需要？

#### Back层（地砖）
- 尺寸固定：48×32 或 96×64
- **直接对齐格子顶部**，不需要特殊偏移
- 只需要处理OpenGL锚点差异：`final_y = screen_y`（因为地砖就是格子大小）

#### Middle层（小建筑）
- 尺寸也是 48×32 或 96×64（代码中有尺寸过滤）
- 同样对齐格子顶部
- `final_y = screen_y`

#### Front层（大型建筑/树木）
- 尺寸不固定，通常**很高**（如 96×200）
- 需要**底部对齐**到格子底部
- `final_y = screen_y - info.height + CELL_HEIGHT`

### 7. 图解示例

```
C# DirectX (左上角锚点):        Rust OpenGL (左下角锚点):

y * 32 → ┌─────┐                  y * 32 → ┌─────┐
         │     │                           │     │
(y+1)*32 └─────┘                  (y+1)*32 └─────┘
              🌲                                 🌲
             🌲🌲                               🌲🌲
Draw(x, y) →🌲🌲🌲 ← 纹理左上角       draw([x, ?]) 🌲🌲🌲
           🌲🌲🌲🌲                            🌲🌲🌲🌲
           🌲🌲🌲🌲                            🌲🌲🌲🌲
           ═══════ ← 底部在 (y+1)*32          ═══════
                                                ↑
                              需要让左下角在这里：
                              screen_y - height + CELL_HEIGHT
```

### 8. 总结

**Front层特殊偏移的原因**：

1. **坐标系差异**：OpenGL锚点在左下角，DirectX在左上角
   - 需要向上偏移 `texture.height` 补偿

2. **对齐逻辑差异**：Front层大型物体需要底部对齐
   - 需要向下偏移 `CELL_HEIGHT` 让底部对齐格子底部

3. **最终公式**：
   ```rust
   final_y = screen_y - texture.height + CELL_HEIGHT
   ```

这不是纹理坐标在左下角，而是**绘制锚点在左下角**！这是OpenGL/ggez与DirectX的根本差异。
