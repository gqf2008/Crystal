# GameScene.cs 绘制和处理代码注释总结

## 📋 已注释的方法

### 1. DrawControl() - 主渲染方法
**功能**: 游戏场景的主渲染方法

**渲染顺序**:
1. **MapControl.DrawControl()** - 地图、对象、特效
2. **base.DrawControl()** - UI对话框和控件
3. **鼠标拖动物品** - 跟随鼠标的物品图标
4. **屏幕中央消息** - OutputLines[10] 输出消息

**关键逻辑**:
```csharp
// 鼠标拖动物品跟随鼠标
if (PickedUpGold || (SelectedCell != null && SelectedCell.Item != null))
{
    int image = PickedUpGold ? 116 : SelectedCell.Item.Image; // 金币(116) 或 物品图标
    Point p = CMain.MPoint.Add(-imgSize.Width / 2, -imgSize.Height / 2); // 鼠标中心对齐
    
    // 边界检测: 防止超出屏幕
    if (p.X + imgSize.Width >= Settings.ScreenWidth)
        p.X = Settings.ScreenWidth - imgSize.Width;
    if (p.Y + imgSize.Height >= Settings.ScreenHeight)
        p.Y = Settings.ScreenHeight - imgSize.Height;
        
    Libraries.Items.Draw(image, p.X, p.Y);
}
```

---

### 2. Process() - 游戏逻辑更新
**功能**: 每帧调用，更新游戏状态

**更新流程**:

#### 前置检查
```csharp
if (MapControl == null || User == null) return;
```

#### 移动时间控制 (100ms间隔)
```csharp
if (CMain.Time >= MoveTime)
{
    MoveTime = CMain.Time + 100; // 移动速度
    CanMove = true; // 允许移动
    MapControl.AnimationCount++; // 动画帧+1
    MapControl.TextureValid = false; // 标记地板纹理需重绘
}
```

#### 心跳包发送 (60秒间隔)
```csharp
if (CMain.Time >= CMain.NextPing)
{
    CMain.NextPing = CMain.Time + 60000; // 60秒
    Network.Enqueue(new C.KeepAlive() { Time = CMain.Time });
}
```

#### 小工具更新
- `TimerControl.Process()` - 计时器
- `CompassControl.Process()` - 指南针
- `RankingDialog.Process()` - 排行榜

#### 物品提示标签更新
```csharp
MirItemCell cell = MouseControl as MirItemCell;

// 如果悬停物品变化，重新创建提示标签
if (cell != null && HoverItem != cell.Item && HoverItem != cell.ShadowItem)
{
    DisposeItemLabel(); // 销毁旧标签
    HoverItem = null;
    CreateItemLabel(cell.Item); // 创建新标签
}

// 标签跟随鼠标，但不超出屏幕
if (ItemLabel != null && !ItemLabel.IsDisposed)
{
    ItemLabel.BringToFront();
    int x = CMain.MPoint.X + 15, y = CMain.MPoint.Y; // 右下方15像素偏移
    
    // 边界检测
    if (x + ItemLabel.Size.Width > Settings.ScreenWidth)
        x = Settings.ScreenWidth - ItemLabel.Size.Width;
    if (y + ItemLabel.Size.Height > Settings.ScreenHeight)
        y = Settings.ScreenHeight - ItemLabel.Size.Height;
        
    ItemLabel.Location = new Point(x, y);
}
```

同样逻辑应用于:
- `MailLabel` - 邮件提示
- `MemoLabel` - 留言提示
- `GuildBuffLabel` - 公会Buff提示

#### 复活提示检测
```csharp
if (!User.Dead) ShowReviveMessage = false;

if (ShowReviveMessage && CMain.Time > User.DeadTime && User.CurrentAction == MirAction.Dead)
{
    ShowReviveMessage = false;
    MirMessageBox messageBox = new MirMessageBox(GameLanguage.DiedTip, MirMessageBoxButtons.YesNo, false);
    
    // 点击"是"按钮: 回城复活
    messageBox.YesButton.Click += (o, e) =>
    {
        if (User.Dead) Network.Enqueue(new C.TownRevive());
    };
    
    // 如果玩家已复活则自动关闭对话框
    messageBox.AfterDraw += (o, e) =>
    {
        if (!User.Dead) messageBox.Dispose();
    };
    
    messageBox.Show();
}
```

#### Buff对话框更新
- `BuffsDialog.Process()` - 玩家Buff
- `HeroBuffsDialog?.Process()` - 英雄Buff

#### 核心对话框更新
- `MapControl.Process()` - 地图控制器
- `MainDialog.Process()` - 主界面
- `InventoryDialog.Process()` - 背包
- `GameShopDialog.Process()` - 商城
- `MiniMapDialog.Process()` - 小地图

#### 技能栏和粒子特效
```csharp
// 更新所有技能栏
foreach (SkillBarDialog Bar in Scene.SkillBarDialogs)
    Bar.Process();

// 更新所有粒子引擎(天气/魔法效果)
foreach (ParticleEngine pe in ParticleEngines)
    pe.Process();
```

#### 最终处理
- `DialogProcess()` - 对话框显示/隐藏控制
- `ProcessOuput()` - 屏幕消息输出
- `UpdateMouseCursor()` - 鼠标光标更新
- `SoundManager.ProcessDelayedSounds()` - 延迟音效

---

### 3. DialogProcess() - 对话框控制
**功能**: 控制对话框的显示/隐藏和位置

#### 技能栏显示控制
```csharp
if(Settings.SkillBar)
{
    foreach (SkillBarDialog Bar in Scene.SkillBarDialogs)
        Bar.Show();
}
else
{
    foreach (SkillBarDialog Bar in Scene.SkillBarDialogs)
        Bar.Hide();
}
```

#### 技能栏位置恢复
```csharp
for (int i = 0; i < Scene.SkillBarDialogs.Count; i++)
{
    // 边界检查
    if (i * 2 > Settings.SkillbarLocation.Length) break;
    
    // 边界验证: 防止超出屏幕
    // X坐标: 不能超过屏幕宽度-100
    // Y坐标: 不能超过700
    if ((Settings.SkillbarLocation[i, 0] > Settings.Resolution - 100) || 
        (Settings.SkillbarLocation[i, 1] > 700)) 
        continue;
    
    // 恢复位置
    Scene.SkillBarDialogs[i].Location = new Point(
        Settings.SkillbarLocation[i, 0], 
        Settings.SkillbarLocation[i, 1]
    );
}
```

#### 耐久度面板控制
```csharp
if (Settings.DuraView)
    CharacterDuraPanel.Show();
else
    CharacterDuraPanel.Hide();
```

---

### 4. ProcessOutput() - 处理输出消息
**功能**: 更新屏幕中央的消息显示

#### 移除过期消息
```csharp
for (int i = 0; i < OutputMessages.Count; i++)
{
    if (CMain.Time >= OutputMessages[i].ExpireTime)
        OutputMessages.RemoveAt(i);
}
```

#### 更新消息显示
```csharp
for (int i = 0; i < OutputLines.Length; i++)
{
    if (OutputMessages.Count > i)
    {
        // 根据消息类型设置颜色
        Color color;
        switch (OutputMessages[i].Type)
        {
            case OutputMessageType.Quest: // 任务
                color = Color.Gold; // 金色
                break;
            case OutputMessageType.Guild: // 公会
                color = Color.DeepPink; // 深粉色
                break;
            default: // 普通(经验/拾取)
                color = Color.LimeGreen; // 亮绿色
                break;
        }
        
        OutputLines[i].Text = OutputMessages[i].Message;
        OutputLines[i].ForeColour = color;
        OutputLines[i].Visible = true;
    }
    else
    {
        // 没有消息时隐藏
        OutputLines[i].Text = string.Empty;
        OutputLines[i].Visible = false;
    }
}
```

---

### 5. OutputMessage() - 输出消息
**功能**: 在屏幕中央显示消息

```csharp
public void OutputMessage(string message, OutputMessageType type = OutputMessageType.Normal)
{
    // 添加新消息，5秒过期
    OutputMessages.Add(new OutPutMessage { 
        Message = message, 
        ExpireTime = CMain.Time + 5000, 
        Type = type 
    });
    
    // 限制最多10条消息
    if (OutputMessages.Count > 10)
        OutputMessages.RemoveAt(0);
}
```

---

### 6. UpdateMouseCursor() - 更新鼠标光标
**功能**: 根据鼠标指向的对象改变光标样式

#### 光标类型
- **Default**: 默认箭头
- **Upgrade**: 宝石镶嵌光标
- **Attack**: 攻击光标 (指向怪物)
- **AttackRed**: 红色攻击光标 (指向玩家+Shift)
- **NPCTalk**: 对话光标 (指向NPC)

#### 光标切换逻辑
```csharp
if (!Settings.UseMouseCursors) return; // 未启用自定义光标

// 1. 悬停在物品上
if (GameScene.HoverItem != null)
{
    // 特殊: 选中宝石+Ctrl → 镶嵌光标
    if (GameScene.SelectedCell?.Item?.Info.Type == ItemType.Gem && CMain.Ctrl)
        CMain.SetMouseCursor(MouseCursor.Upgrade);
    else
        CMain.SetMouseCursor(MouseCursor.Default);
}
// 2. 悬停在地图对象上
else if (MapObject.MouseObject != null)
{
    switch (MapObject.MouseObject.Race)
    {
        case ObjectType.Monster: // 怪物
            CMain.SetMouseCursor(MouseCursor.Attack);
            break;
        case ObjectType.Merchant: // NPC
            CMain.SetMouseCursor(MouseCursor.NPCTalk);
            break;
        case ObjectType.Player: // 玩家
            if (CMain.Shift) // Shift = 强制攻击
                CMain.SetMouseCursor(MouseCursor.AttackRed);
            else
                CMain.SetMouseCursor(MouseCursor.Default);
            break;
        default:
            CMain.SetMouseCursor(MouseCursor.Default);
            break;
    }
}
// 3. 悬停在空地上
else
{
    CMain.SetMouseCursor(MouseCursor.Default);
}
```

---

### 7. GameScene_KeyDown() - 键盘按键处理
**功能**: 处理游戏中的所有快捷键

**支持的快捷键**:
- 技能栏快捷键 (Bar1/Bar2/Hero 各8个技能)
- UI对话框快捷键 (背包/角色/技能等)
- 功能快捷键 (截图/退出/设置等)

**组合键支持**:
- Alt
- Shift
- Ctrl
- Tilde (~)

---

## 🔄 渲染流程总结

```
每帧更新 (Process)
├─ 移动时间控制 (100ms)
│  ├─ CanMove = true
│  ├─ AnimationCount++
│  └─ TextureValid = false
├─ 心跳包 (60秒)
├─ UI更新
│  ├─ 物品提示标签
│  ├─ 邮件/留言/公会Buff标签
│  └─ 复活提示
├─ 对话框更新
│  ├─ BuffsDialog
│  ├─ MapControl
│  ├─ MainDialog
│  ├─ InventoryDialog
│  └─ 等等...
├─ 技能栏和粒子特效
├─ DialogProcess (显示/隐藏控制)
├─ ProcessOutput (消息更新)
├─ UpdateMouseCursor (光标更新)
└─ 延迟音效处理

渲染 (DrawControl)
├─ MapControl.DrawControl()
│  ├─ 地图瓦片
│  ├─ 地图对象
│  └─ 特效
├─ base.DrawControl()
│  └─ 所有UI对话框
├─ 鼠标拖动物品
│  └─ 跟随鼠标的物品图标
└─ 屏幕中央消息
   └─ OutputLines[10]
```

---

## 🎯 关键时间控制

| 项目 | 间隔 | 说明 |
|-----|------|------|
| 移动更新 | 100ms | CanMove, 动画帧+1, 地板重绘 |
| 心跳包 | 60秒 | 保持连接 |
| 消息过期 | 5秒 | 屏幕中央消息自动消失 |
| 消息队列 | 最多10条 | 超出移除最早的消息 |

---

## 📐 坐标和边界处理

### 物品拖动图标
```csharp
// 1. 鼠标中心对齐
Point p = CMain.MPoint.Add(-imgSize.Width / 2, -imgSize.Height / 2);

// 2. 边界检测
if (p.X + imgSize.Width >= Settings.ScreenWidth)
    p.X = Settings.ScreenWidth - imgSize.Width;
if (p.Y + imgSize.Height >= Settings.ScreenHeight)
    p.Y = Settings.ScreenHeight - imgSize.Height;
```

### 提示标签
```csharp
// 1. 右下方15像素偏移
int x = CMain.MPoint.X + 15, y = CMain.MPoint.Y;

// 2. 边界检测
if (x + ItemLabel.Size.Width > Settings.ScreenWidth)
    x = Settings.ScreenWidth - ItemLabel.Size.Width;
if (y + ItemLabel.Size.Height > Settings.ScreenHeight)
    y = Settings.ScreenHeight - ItemLabel.Size.Height;
```

### 技能栏位置
```csharp
// X坐标: 不能超过屏幕宽度-100
// Y坐标: 不能超过700
if ((Settings.SkillbarLocation[i, 0] > Settings.Resolution - 100) || 
    (Settings.SkillbarLocation[i, 1] > 700))
    continue;
```

---

## 🎨 消息颜色系统

| 消息类型 | 颜色 | RGB | 用途 |
|---------|------|-----|------|
| Quest | Gold | 金色 | 任务消息 |
| Guild | DeepPink | 深粉色 | 公会消息 |
| Normal | LimeGreen | 亮绿色 | 普通消息(经验/拾取) |

---

## 🖱️ 鼠标光标系统

| 光标类型 | 触发条件 | 说明 |
|---------|---------|------|
| Default | 默认 | 默认箭头 |
| Upgrade | 悬停物品 + 选中宝石 + Ctrl | 宝石镶嵌 |
| Attack | 指向怪物 | 攻击光标 |
| AttackRed | 指向玩家 + Shift | 强制攻击 |
| NPCTalk | 指向NPC | 对话光标 |

---

## ✅ Rust 实现建议

### 1. 渲染顺序
```rust
fn draw_control(&mut self) {
    // 1. 地图层
    if let Some(map_control) = &mut self.map_control {
        map_control.draw_control(ctx);
    }
    
    // 2. UI层
    self.draw_ui(ctx);
    
    // 3. 拖动物品
    if self.picked_up_gold || self.selected_cell.is_some() {
        self.draw_dragged_item(ctx);
    }
    
    // 4. 屏幕消息
    for output_line in &self.output_lines {
        output_line.draw(ctx);
    }
}
```

### 2. 时间控制
```rust
// 使用Instant代替时间戳
use std::time::{Instant, Duration};

struct GameScene {
    move_time: Instant,
    next_ping: Instant,
    // ...
}

fn process(&mut self) {
    let now = Instant::now();
    
    // 移动时间 (100ms)
    if now >= self.move_time {
        self.move_time = now + Duration::from_millis(100);
        self.can_move = true;
        self.map_control.animation_count += 1;
    }
    
    // 心跳包 (60秒)
    if now >= self.next_ping {
        self.next_ping = now + Duration::from_secs(60);
        self.network.send(KeepAlive { time: now });
    }
}
```

### 3. 边界检测
```rust
fn clamp_to_screen(pos: Point, size: Size, screen_size: Size) -> Point {
    Point {
        x: pos.x.min(screen_size.width - size.width).max(0),
        y: pos.y.min(screen_size.height - size.height).max(0),
    }
}
```

---

## 🎉 总结

已为 GameScene.cs 的核心绘制和处理方法添加完整注释:

✅ **DrawControl()** - 主渲染方法  
✅ **Process()** - 游戏逻辑更新  
✅ **DialogProcess()** - 对话框控制  
✅ **ProcessOutput()** - 输出消息处理  
✅ **OutputMessage()** - 消息输出  
✅ **UpdateMouseCursor()** - 光标更新  
✅ **GameScene_KeyDown()** - 键盘处理

所有注释包含:
- 方法功能说明
- 关键逻辑注释
- 时间控制说明
- 边界检测逻辑
- Rust实现建议

现在可以更容易地理解游戏的渲染和处理流程了！🚀
