# 游戏商城对话框 (GameShopDialog)

## 概述

游戏商城对话框是传奇2客户端中的商品购买界面，参考背包系统实现，提供完整的商品浏览、筛选和预览功能。

## 功能特性

### 🎯 基础功能
- ✅ 拖拽标题栏移动窗口
- ✅ 点击关闭按钮或按ESC关闭商城
- ✅ 商品分类标签页切换（全部/热销/特价/新品）
- ✅ 职业筛选标签页（全部/战士/刺客/道士/法师/弓箭手）

### 🛍️ 商品浏览
- ✅ 4x2网格布局（每页8个商品）
- ✅ 商品图标和名称显示
- ✅ 价格信息（金币/元宝）
- ✅ 库存状态标记（缺货显示灰色）
- ✅ 热销/新品标签（🔥图标和NEW文字）
- ✅ 选中商品高亮显示

### 🔍 商品预览
- ✅ 点击商品显示详细预览窗口
- ✅ 预览窗口可独立拖拽移动
- ✅ 显示物品名称、描述、价格信息
- ✅ 8方向预览切换（◀/▶按钮）
- ✅ 点击外部区域关闭预览（模态对话框）
- ✅ 按ESC键关闭预览
- ✅ 再次点击同一商品关闭预览

### 📄 分页功能
- ✅ 上一页/下一页按钮导航
- ✅ 页码显示（当前页/总页数）
- ✅ 切换分类/职业时自动重置到第1页
- ✅ 翻页时自动关闭预览器

### 💰 货币系统
- ✅ 实时显示玩家金币数量
- ✅ 实时显示玩家元宝数量
- ✅ 支持金币和元宝双货币定价
- ✅ 可扩展购买前余额检查

## 技术实现

### 数据结构

```rust
/// 商品分类
pub enum GameShopSection {
    All,        // 全部商品
    TopItems,   // 热销商品
    Deals,      // 特价商品
    New,        // 新品
}

/// 职业筛选
pub enum GameShopClass {
    All,        // 全职业
    Warrior,    // 战士
    Assassin,   // 刺客
    Taoist,     // 道士
    Wizard,     // 法师
    Archer,     // 弓箭手
}

/// 商品信息
pub struct ShopItem {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub icon_index: usize,
    pub price_gold: u32,      // 金币价格
    pub price_ingot: u32,     // 元宝价格
    pub category: ShopCategory,
    pub in_stock: bool,       // 库存状态
    pub hot: bool,            // 热销标记
    pub new: bool,            // 新品标记
}

/// 商品预览器
pub struct ShopItemViewer {
    pub item: ShopItem,
    pub direction: u8,        // 预览方向(1-8)
    pub visible: bool,
    pub position: egui::Pos2,
    pub dragging: bool,
    pub drag_offset: egui::Vec2,
}
```

### 核心方法

#### 初始化
```rust
pub fn new() -> Self
```
创建商城对话框，初始化示例商品和默认状态。

#### 商品过滤
```rust
pub fn filter_items(&mut self)
```
根据当前选中的分类和职业筛选商品列表。

#### 绘制方法
```rust
fn draw_background(&self, ui: &mut egui::Ui, ctx: &egui::Context) -> egui::Rect
fn draw_category_tabs(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect)
fn draw_item_grid(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect)
fn draw_pagination(&mut self, ui: &mut egui::Ui, bg_rect: &egui::Rect)
fn draw_currency_info(&self, ui: &mut egui::Ui, bg_rect: &egui::Rect)
```

#### 窗口拖拽
```rust
fn handle_window_dragging(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect)
```
处理主窗口标题栏拖拽移动。

#### 独立预览器
```rust
fn draw_item_viewer_separate(ctx: &egui::Context, viewer: &mut ShopItemViewer, main_dialog_pos: &egui::Pos2) -> bool
```
使用模态对话框模式显示商品预览，避免阻挡主对话框交互。

## 纹理资源

### Title库纹理索引
- `749` - 商城背景纹理
- `750` - 商品单元格背景
- `770/771` - 全部分类标签（正常/选中）
- `772/773` - 特价分类标签（正常/选中）
- `774/775` - 新品分类标签（正常/选中）
- `776/777` - 热销分类标签（正常/选中）
- `751/752` - 全职业标签（正常/选中）
- `754/755` - 战士标签（正常/选中）
- `757/758` - 刺客标签（正常/选中）
- `760/761` - 道士标签（正常/选中）
- `763/764` - 法师标签（正常/选中）
- `766/767` - 弓箭手标签（正常/选中）
- `785` - 商品预览窗口背景

### Prguse库纹理索引
- `361/362` - 关闭按钮（正常/悬停）

## 布局规格

### 主窗口
- 背景纹理：Title[749]
- 尺寸：根据纹理实际大小
- 默认位置：(300, 150)

### 分类标签
- Section标签起始位置：(138, 68)
- 标签尺寸：71x23
- 标签间距：71
- Class标签起始位置：(539, 37)
- 标签尺寸：23x20
- 标签间距：23

### 商品网格
- 第一行起始位置：(152, 115)
- 第二行起始位置：(152, 275)
- 单元格尺寸：125x146
- 单元格间距：132
- 布局：4列 x 2行 = 8个商品/页

### 分页控制
- 页码显示位置：(597, 446)
- 上一页按钮位置：(560, 448)
- 下一页按钮位置：(690, 448)

### 预览窗口
- 尺寸：260x300
- 动态定位：根据点击位置自动选择左侧或右侧
- 层级：最高层（Order::Tooltip）

## 使用示例

### 基本使用
```rust
use client_macroquad::scenes::dialogs::game::GameShopDialog;
use client_macroquad::scenes::dialogs::Dialog;

// 创建商城
let mut shop = GameShopDialog::new();
let mut shop_open = true;

// 在主循环中显示
egui_macroquad::ui(|ctx| {
    shop.show(ctx, &mut shop_open);
});
```

### 访问商城状态
```rust
// 获取当前状态
println!("金币: {}", shop.player_gold);
println!("元宝: {}", shop.player_ingot);
println!("分类: {}", shop.selected_section.display_name());
println!("职业: {}", shop.selected_class.display_name());
println!("商品数: {}", shop.filtered_items.len());
println!("当前页: {}/{}", shop.current_page + 1, total_pages);

// 修改玩家货币
shop.player_gold += 10000;
shop.player_ingot += 1000;
```

### 切换分类
```rust
use client_macroquad::scenes::dialogs::game::GameShopSection;

shop.selected_section = GameShopSection::TopItems;
shop.selected_class = GameShopClass::All;
shop.current_page = 0;
shop.filter_items(); // 重新过滤商品
```

## 测试程序

运行测试程序：
```bash
cargo run --bin test_game_shop_dialog
```

测试程序提供：
- 商城状态实时显示
- 快捷操作按钮（增加金币/元宝、切换分类等）
- 分类/职业快速切换
- 预览器控制
- FPS性能监控

## 参考实现

本对话框参考了以下实现：
- **InventoryDialog** - 背包系统的窗口拖拽、物品交互模式
- **原版Crystal客户端** - Title库纹理索引和布局规格
- **egui最佳实践** - 模态对话框和交互层级管理

## 已知限制

1. **职业筛选** - 当前所有职业都能看到所有商品（简化实现）
2. **商品图标** - 使用占位符显示，需要实际物品纹理
3. **购买功能** - 仅UI展示，购买逻辑待实现
4. **数据持久化** - 未实现商品列表和购买记录保存

## 未来优化

- [ ] 实现完整的购买流程（确认对话框、余额检查、购买动画）
- [ ] 添加商品分类图标和装饰元素
- [ ] 实现商品搜索和排序功能
- [ ] 添加商品详细属性展示（装备属性、技能描述等）
- [ ] 实现购物车功能（批量购买）
- [ ] 添加限时特惠和促销活动支持
- [ ] 实现商品预览3D/动画效果
- [ ] 添加购买历史记录查询

## 相关文档

- [背包系统文档](./inventory_dialog.md)
- [对话框基础](./dialogs.md)
- [纹理资源管理](./texture_resources.md)
