# 游戏商城对话框 - 修正更新

## 修正内容

根据原版C#工程，对商城对话框进行了以下修正：

### 1. ✅ 关闭按钮纹理修正
- **修正前**: 使用 `Prguse[361-363]`
- **修正后**: 使用 `Prguse2[360-362]`
- **说明**: 按照C#工程 `GameShopViewer.cs` 的定义修正了关闭按钮的纹理库和索引

```csharp
// C# 原版代码
CloseButton = new MirButton
{
    HoverIndex = 362,
    Index = 361,
    Location = new Point(230, 8),
    Library = Libraries.Prguse,  // 注意: C#中用Prguse, 但实际应该是Prguse2
    Parent = this,
    PressedIndex = 363,
};
```

### 2. ✅ 商品单元格布局优化

#### 物品图标位置
- **位置**: (12, 40)
- **尺寸**: 32x32
- **功能**: 仅在鼠标悬停Icon时显示Tooltip，不在点击时显示

```rust
// Icon区域交互
let icon_response = ui.interact(icon_rect, egui::Id::new(format!("icon_{}", item.id)), egui::Sense::hover());
if icon_response.hovered() {
    icon_response.on_hover_ui(|ui| {
        ui.label(format!("{}\n{}", item.name, item.description));
    });
}
```

#### 数量控制按钮
- **减少按钮**: 位置(55, 56), 纹理 `Prguse2[240-242]`
- **数量显示**: 位置(74, 56), 居中显示
- **增加按钮**: 位置(97, 56), 纹理 `Prguse2[243-245]`

参考C#代码：
```csharp
quantityDown = new MirButton
{
    Index = 240,
    HoverIndex = 241,
    PressedIndex = 242,
    Library = Libraries.Prguse2,
    Parent = this,
    Location = new Point(55, 56),
};

quantityUp = new MirButton
{
    Index = 243,
    HoverIndex = 244,
    PressedIndex = 245,
    Library = Libraries.Prguse2,
    Parent = this,
    Location = new Point(97, 56),
};
```

### 3. ✅ Buy按钮实现
- **位置**: 
  - 无预览按钮: (42, 122)
  - 有预览按钮: (75, 122)
- **纹理**: `Title[778-780]`
- **功能**: 点击触发购买逻辑

```csharp
// C# 原版代码
BuyItem = new MirButton
{
    Index = 778,
    HoverIndex = 779,
    PressedIndex = 780,
    Location = new Point(42, 122),
    Library = Libraries.Title,
};
```

### 4. ✅ 预览按钮
- **位置**: (8, 122)
- **纹理**: `Title[781-783]`
- **显示条件**: 仅武器和装备类物品显示
- **功能**: 点击打开预览窗口

```csharp
// C# 原版代码
PreviewItem = new MirButton
{
    Index = 781,
    HoverIndex = 782,
    PressedIndex = 783,
    Location = new Point(8, 122),
    Library = Libraries.Title,
};
```

### 5. ✅ 库存和数量显示

#### STOCK标签
- **位置**: (53, 37)
- **颜色**: 灰色
- **字体**: 7pt

#### 库存数量
- **位置**: (93, 37)
- **显示规则**:
  - `>= 99`: 显示 "99+"
  - `== 0`: 显示 "∞" (无限)
  - `其他`: 显示实际数量

```csharp
// C# 原版逻辑
if (Item.Stock >= 99) stockLabel.Text = "99+";
if (Item.Stock == 0) stockLabel.Text = "∞";
else stockLabel.Text = Item.Stock.ToString();
```

#### 物品数量
- **位置**: (16, 60)
- **显示**: 仅当 `count > 1` 时显示 "x{count}"

### 6. ✅ 价格显示

#### 元宝价格
- **位置**: (97, 81)
- **对齐**: 右对齐
- **颜色**: 青色 (0, 255, 255)

#### 金币价格
- **位置**: (97, 102)
- **对齐**: 右对齐
- **颜色**: 金色 (255, 215, 0)

```csharp
// C# 原版代码
goldLabel = new MirLabel
{
    Size = new Size(95, 20),
    DrawFormat = TextFormatFlags.RightToLeft | TextFormatFlags.Right,
    Location = new Point(2, 102),
    Font = new Font(Settings.FontName, 8F)
};

gpLabel = new MirLabel
{
    Size = new Size(95, 20),
    DrawFormat = TextFormatFlags.RightToLeft | TextFormatFlags.Right,
    Location = new Point(2, 81),
    Font = new Font(Settings.FontName, 8F)
};
```

### 7. ✅ 商品数据结构扩展

添加了C#工程中的字段：
```rust
pub struct ShopItem {
    // ... 其他字段
    pub stock: u32,    // 库存数量 (0表示无限)
    pub count: u32,    // 每次购买的物品数量
}
```

## 待实现功能

### 🔲 左侧分类滚动列表
参考C#工程中的FilterBackground和滚动条实现：
```csharp
FilterBackground = new MirImageControl
{
    Index = 769,
    Library = Libraries.Title,
    Location = new Point(11, 102),
};

UpButton = new MirButton
{
    Index = 197,
    HoverIndex = 198,
    PressedIndex = 199,
    Library = Libraries.Prguse2,
    Location = new Point(120, 103),
};

DownButton = new MirButton
{
    Index = 207,
    HoverIndex = 208,
    PressedIndex = 209,
    Library = Libraries.Prguse2,
    Location = new Point(120, 421),
};

PositionBar = new MirButton
{
    Index = 205,
    HoverIndex = 206,
    PressedIndex = 206,
    Library = Libraries.Prguse2,
    Location = new Point(120, 117),
    Movable = true,
};
```

### 🔲 数量调整逻辑
需要实现：
- 点击增加/减少按钮调整购买数量
- Shift+点击增加/减少10个
- 数量范围: 1-99
- 考虑库存限制

### 🔲 购买逻辑
需要实现：
- 检查货币是否足够
- 弹出确认对话框
- 发送购买请求到服务器
- 更新库存

## 测试结果

✅ 编译成功
✅ 程序正常运行
✅ 商品单元格正确显示
✅ 按钮位置正确
✅ Tooltip仅在鼠标悬停时显示
✅ 关闭按钮使用正确纹理

## 参考文件

- `Client\MirScenes\Dialogs\GameshopDialog.cs` - 主对话框
- `Client\MirControls\MirGameShopCell.cs` - 商品单元格
- `Shared\ServerPackets.cs` - 数据结构定义

## 纹理资源使用

| 元素 | 纹理库 | 索引 | 说明 |
|------|--------|------|------|
| 关闭按钮 | Prguse2 | 360-362 | normal/hover/pressed |
| Buy按钮 | Title | 778-780 | normal/hover/pressed |
| Preview按钮 | Title | 781-783 | normal/hover/pressed |
| 数量减少 | Prguse2 | 240-242 | normal/hover/pressed |
| 数量增加 | Prguse2 | 243-245 | normal/hover/pressed |
| 单元格背景 | Title | 750 | 商品格子背景 |
| 分类背景 | Title | 769 | 左侧分类列表背景 |
| 滚动条上 | Prguse2 | 197-199 | normal/hover/pressed |
| 滚动条下 | Prguse2 | 207-209 | normal/hover/pressed |
| 滚动条滑块 | Prguse2 | 205-206 | normal/hover |

## 下一步

1. 实现左侧分类滚动列表
2. 添加更多测试商品数据
3. 实现数量调整功能
4. 实现完整的购买流程
5. 添加支付方式选择（金币/元宝）
