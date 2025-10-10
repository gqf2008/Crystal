# MapControl 架构与渲染管线详解

## 📋 概述

MapControl 是 Crystal Mir2 客户端的核心渲染组件，负责：
- 地图瓦片渲染 (Back/Middle/Front 三层)
- 动态对象管理 (玩家/怪物/NPC/掉落物)
- 用户输入处理 (鼠标点击/移动/寻路)
- 光照与天气效果
- 摄像机控制与视野裁剪

---

## 🎨 渲染管线 (Rendering Pipeline)

### 总体流程 (CreateTexture)

```
1. DrawFloor()      - 静态瓦片层 (缓存到 FloorTexture)
   ├─ Back Layer    - 地表瓦片 (草地/沙地)
   ├─ Middle Layer  - 建筑层 (房屋/树木)
   └─ Front Layer   - 前景层 (屋顶/树冠)

2. DrawBackground() - 远景背景 (山脉/沙漠/长城)

3. DrawObjects()    - 动态对象与特效
   ├─ 背景特效     - 地面火焰/毒圈
   ├─ 尸体对象     - 死亡的玩家/怪物
   ├─ 瓦片动画     - 流水/岩浆
   ├─ 动态层       - Middle/Front 动画瓦片
   ├─ 对象本体     - 玩家/怪物/NPC
   ├─ 高亮边框     - 目标选中效果
   ├─ 前景特效     - 火球/闪电
   └─ UI 元素      - 名字/血条/伤害数字

4. DrawLights()     - 光照遮罩 (夜晚效果)

5. DrawNames()      - 顶层 UI (掉落物品名称)
```

---

## 📐 坐标系统 (Coordinate System)

### 三种坐标类型

#### 1. 地图坐标 (Map Coordinates)
```csharp
// 格子坐标 (整数)
Point mapPos = new Point(100, 100);
```

#### 2. 屏幕坐标 (Screen Coordinates)
```csharp
// 像素坐标 (整数)
// 核心转换公式:
int drawX = (x - User.Movement.X + OffSetX) * CellWidth - OffSetX + User.OffSetMove.X;
int drawY = (y - User.Movement.Y + OffSetY) * CellHeight + User.OffSetMove.Y;

// 参数说明:
// - User.Movement: 玩家当前渲染位置 (移动时平滑变化)
// - OffSetX/Y: 视野中心偏移 (10, 11)
// - CellWidth/Height: 格子尺寸 (48, 32)
// - User.OffSetMove: 像素级移动偏移 (0-47, 0-31)
```

#### 3. 视野范围 (View Range)
```csharp
// 1024x768 窗口计算:
OffSetX = 1024 / 2 / 48 = 10;      // 横向偏移 10 格
OffSetY = 768 / 2 / 32 - 1 = 11;   // 纵向偏移 11 格

ViewRangeX = OffSetX + 6 = 16;     // 视野范围 16 格
ViewRangeY = OffSetY + 6 = 17;     // 视野范围 17 格

// 可见区域:
// X: User.Movement.X ± 16 格
// Y: User.Movement.Y ± 17 格 (或 +22 用于对象渲染)
```

---

## 🗺️ 地图瓦片渲染 (DrawFloor)

### Back Layer (地表层)
```csharp
// 特点:
// - 只渲染偶数行列 (y % 2 == 0 && x % 2 == 0)
// - 大地表瓦片 (草地/沙地/石板)
// - 视野范围: Y ± ViewRangeY

for (int y = User.Movement.Y - ViewRangeY; y <= User.Movement.Y + ViewRangeY; y++)
{
    if (y <= 0 || y % 2 == 1) continue; // 跳过奇数行
    
    for (int x = User.Movement.X - ViewRangeX; x <= User.Movement.X + ViewRangeX; x++)
    {
        if (x <= 0 || x % 2 == 1) continue; // 跳过奇数列
        
        // BackImage 高3位用于特殊标记，需要屏蔽
        index = (M2CellInfo[x, y].BackImage & 0x1FFFFFFF) - 1;
        Libraries.MapLibs[BackIndex].Draw(index, drawX, drawY);
    }
}
```

### Middle Layer (建筑层)
```csharp
// 特点:
// - 渲染所有格子 (不限奇偶)
// - 建筑物/树木/山体
// - 视野范围: Y ± (ViewRangeY + 5) - 向下多渲染5格
// - 尺寸过滤: 只渲染 48x32 或 96x64 的瓦片

for (int y = User.Movement.Y - ViewRangeY; y <= User.Movement.Y + ViewRangeY + 5; y++)
{
    for (int x = User.Movement.X - ViewRangeX; x <= User.Movement.X + ViewRangeX; x++)
    {
        index = M2CellInfo[x, y].MiddleImage - 1;
        
        // 尺寸验证 (防止绘制条状错误瓦片)
        Size s = Libraries.MapLibs[MiddleIndex].GetSize(index);
        if ((s.Width != 48 || s.Height != 32) &&
            (s.Width != 96 || s.Height != 64)) continue;
        
        Libraries.MapLibs[MiddleIndex].Draw(index, drawX, drawY);
    }
}
```

### Front Layer (前景层)
```csharp
// 特点:
// - 建筑顶部/树冠等前景物体
// - 支持门动画 (DoorIndex, DoorOffset)
// - Y 偏移: drawY = baseY - 32 (向上偏移一格)

index = (M2CellInfo[x, y].FrontImage & 0x7FFF) - 1;

// 门动画处理
if (M2CellInfo[x, y].DoorIndex > 0)
{
    Door door = GetDoor(DoorIndex);
    if (door.DoorState != 0) // 门已开启
    {
        // 动画索引计算: 基础索引 + (动画帧 + 1) * 偏移量
        index += (door.ImageIndex + 1) * M2CellInfo[x, y].DoorOffset;
    }
}

Libraries.MapLibs[FrontIndex].Draw(index, drawX, drawY);
```

---

## 🎭 动态对象渲染 (DrawObjects)

### 渲染顺序 (9个步骤)

```
1. 背景特效 (Effects DrawBehind = true)
   └─ 地面火焰/毒圈/魔法阵

2. 尸体对象 (DeadObjects)
   └─ 死亡的玩家/怪物 (先绘制确保被活物遮挡)

3. Shanda 瓦片动画层 (TileAnimationImage)
   └─ 流水/岩浆等动态地表

4. Middle 动态层 (MiddleLayer with animation)
   └─ 建筑动画 (旋转风车/飘动旗帜)

5. Front 动态层 (FrontLayer with animation/doors)
   └─ 前景动画 + 门开关

6. 对象本体 (M2CellInfo[x,y].DrawObjects)
   └─ 玩家/怪物/NPC 按 Y 坐标排序绘制

7. 高亮边框 (HighlightTarget)
   └─ 鼠标悬停/目标选中的发光边框

8. 前景特效 (Effects DrawBehind = false)
   └─ 火球/闪电/爆炸

9. UI 元素
   ├─ 名字标签 (NameLabel)
   ├─ 血条 (HealthBar)
   ├─ 聊天气泡 (ChatBubble)
   ├─ 中毒图标 (PoisonIcon)
   └─ 伤害数字 (DamageNumbers)
```

### 动画系统

#### 瓦片动画 (Tile Animation)
```csharp
// Shanda 动画层 (流水/岩浆)
index = TileAnimationImage - 1;
int animationOffset = TileAnimationOffset ^ 0x2000;
index += animationOffset * (AnimationCount % TileAnimationFrames);
Libraries.MapLibs[190].DrawUp(index, drawX, drawY);
```

#### Middle/Front 动画
```csharp
// 动画帧计算
animation = MiddleAnimationFrame;
if (animation > 0 && animation < 255)
{
    int tick = MiddleAnimationTick;
    int frames = animation & 0x0f;
    index += (AnimationCount % (frames + frames * tick)) / (1 + tick);
}
```

---

## 💡 光照系统 (DrawLights)

### 工作原理

```
1. 创建全黑遮罩 (darkness 颜色)
2. 在遮罩上绘制光源
   - 加法混合: SourceAlpha + One
3. 将遮罩应用到场景
   - 乘法混合: Zero + SourceColor
```

### 光照类型

#### 对象光照
```csharp
// 玩家/怪物/NPC 携带的光源
int light = object.Light;
int lightRange = light % 15;         // 0-14 光照范围
int lightType = light / 15;          // 光源类型

switch (lightType)
{
    case 0: // 无光源
        lightColour = Color.FromArgb(255, 60, 60, 60);
        break;
    case 1: // 微弱光
        lightColour = Color.FromArgb(255, 120, 120, 120);
        break;
    case 2: // 蜡烛
        lightColour = Color.FromArgb(255, 180, 180, 180);
        break;
    case 3: // 火把
        lightColour = Color.FromArgb(255, 240, 240, 240);
        break;
    default: // 商人火把
        lightColour = Color.FromArgb(255, 255, 255, 255);
        break;
}
```

#### 地图光照
```csharp
// 地图瓦片自带光源 (路灯/火盆)
if (CellInfo[x, y].Light > 0)
{
    int light = CellInfo[x, y].Light;
    DrawLight(light, drawX, drawY);
}
```

### 黑暗等级
```csharp
switch (setting)
{
    case LightSetting.Night:
        switch (MapDarkLight)
        {
            case 1: darkness = Color.FromArgb(255, 20, 20, 20);  // 极暗 (洞穴)
            case 2: darkness = Color.LightSlateGray;             // 中等 (森林)
            case 3: darkness = Color.SkyBlue;                    // 蓝色 (月光)
            case 4: darkness = Color.Goldenrod;                  // 金黄 (篝火)
            default: darkness = Color.Black;                     // 纯黑 (深渊)
        }
        break;
    case LightSetting.Evening: // 黄昏
    case LightSetting.Dawn:    // 黎明
        darkness = Color.FromArgb(255, 50, 50, 50);
        break;
    case LightSetting.Day:
        darkness = Color.White; // 全亮 (无遮罩)
        break;
}
```

---

## 🔍 关键数据结构

### CellInfo (地图格子信息)
```csharp
public class CellInfo
{
    // Back Layer (地表层)
    public int BackImage;      // 地表图像索引 (高3位用于标记)
    public int BackIndex;      // MapLibs 数组索引

    // Middle Layer (建筑层)
    public int MiddleImage;    // 建筑图像索引
    public int MiddleIndex;    // MapLibs 数组索引
    public byte MiddleAnimationFrame;  // 动画帧数
    public byte MiddleAnimationTick;   // 动画速度

    // Front Layer (前景层)
    public int FrontImage;     // 前景图像索引 (高位用于标记)
    public int FrontIndex;     // MapLibs 数组索引
    public byte FrontAnimationFrame;   // 动画帧数
    public byte FrontAnimationTick;    // 动画速度

    // 动画层 (Shanda)
    public int TileAnimationImage;     // 瓦片动画索引
    public byte TileAnimationFrames;   // 总帧数
    public int TileAnimationOffset;    // 动画偏移

    // 门系统
    public int DoorIndex;      // 门索引
    public int DoorOffset;     // 门动画偏移量

    // 其他
    public int Light;          // 光源强度
    public List<MapObject> Objects;  // 此格子上的对象列表
}
```

### MapObject (地图对象基类)
```csharp
public abstract class MapObject
{
    public Point CurrentLocation;  // 当前地图坐标
    public Point Movement;         // 渲染位置 (移动时平滑变化)
    public Point OffSetMove;       // 像素级偏移 (0-47, 0-31)
    
    public MirDirection Direction; // 朝向
    public int Light;              // 光照强度
    public bool Dead;              // 是否死亡
    
    public abstract void Draw();   // 绘制对象
    public abstract void Process(); // 更新逻辑
}
```

---

## ⚡ 性能优化

### 1. 地板层缓存
```csharp
// FloorValid 标记控制缓存
if (!FloorValid)
{
    DrawFloor(); // 重绘地板到 FloorTexture
    FloorValid = true;
}

// 直接使用缓存纹理
DXManager.Draw(DXManager.FloorTexture, screenRect, Vector3.Zero, Color.White);
```

### 2. 视野裁剪
```csharp
// 只渲染可见区域 (节省 70% 绘制)
for (int y = User.Movement.Y - ViewRangeY; y <= User.Movement.Y + ViewRangeY; y++)
{
    // 仅处理视野内的格子
}
```

### 3. 偶数行列优化
```csharp
// Back Layer 只渲染偶数行列 (减少 75% 绘制)
if (y % 2 == 1 || x % 2 == 1) continue;
```

### 4. 尺寸过滤
```csharp
// 跳过无效尺寸的瓦片 (防止渲染错误)
if ((s.Width != 48 || s.Height != 32) &&
    (s.Width != 96 || s.Height != 64)) continue;
```

---

## 🐛 常见问题与解决方案

### 问题 1: 瓦片位置偏移
**症状**: 地图瓦片不在正确位置，或出现黑块  
**原因**: Movement 字段未与 CurrentLocation 同步  
**解决**: 
```csharp
// 移动时必须同时更新两个字段
CurrentLocation = newLocation;
Movement = newLocation; // 🔧 关键
```

### 问题 2: 背景图偏移
**症状**: 背景图位置错误 (上下偏移)  
**原因**: DirectX 和 wgpu 坐标系差异  
**解决**:
```csharp
// DirectX (C#): (0,0) 在左上角
Libraries.Background.Draw(index, 0, 0);

// wgpu (Rust): (0,0) 在左下角，需要转换
let draw_y = screen_height - image_height;
canvas.draw(&texture, [0.0, draw_y]);
```

### 问题 3: 对象遮挡错误
**症状**: 后面的对象盖住前面的  
**原因**: 渲染顺序错误  
**解决**: 必须按 Y 坐标从小到大绘制
```csharp
for (int y = minY; y <= maxY; y++) // 从上到下
{
    M2CellInfo[x, y].DrawObjects();
}
```

---

## 📚 参考资料

### C# 源码位置
- MapControl: `Client/MirScenes/GameScene.cs` (10062-12294行)
- DrawFloor: 第 10542-10704 行
- DrawObjects: 第 10747-11014 行
- DrawLights: 第 11043-11216 行

### Rust 实现位置
- MapControl: `ClientRust/src/scenes/game_scene/map_control.rs`
- GameScene: `ClientRust/src/scenes/game_scene.rs`

### 关键常量
```
CellWidth = 48    // 格子宽度
CellHeight = 32   // 格子高度
OffSetX = 10      // 视野中心偏移X
OffSetY = 11      // 视野中心偏移Y
ViewRangeX = 16   // 视野范围X
ViewRangeY = 17   // 视野范围Y
```

---

## 📝 重构建议

### Rust 实现注意事项

1. **坐标系转换**
   - wgpu 使用左下角原点
   - 所有 Y 坐标需要翻转: `screen_height - y`

2. **Movement 同步**
   - 所有修改 `current_location` 的地方必须同步 `movement`
   - 建议使用 `set_current_location()` 方法统一处理

3. **事件驱动**
   - UserInformation 包接收后立即创建 user 对象
   - 不要延迟到 PlayerSpawned 事件

4. **纹理缓存**
   - 考虑使用纹理图集 (Texture Atlas)
   - 避免每帧创建新纹理

5. **批量绘制**
   - 相同纹理的瓦片应该批量提交
   - 减少 draw call 次数

---

## ✅ 总结

MapControl 是一个复杂但结构清晰的渲染系统：
- **分层渲染**: Back → Middle → Front → Objects → Lights → UI
- **坐标转换**: 地图坐标 ↔ 屏幕坐标，考虑视野偏移和移动偏移
- **性能优化**: 缓存、裁剪、批量绘制
- **动画系统**: 瓦片动画、对象动画、特效动画
- **光照系统**: 遮罩渲染、加法/乘法混合

理解这些核心概念后，重构到 Rust 就会容易很多。

