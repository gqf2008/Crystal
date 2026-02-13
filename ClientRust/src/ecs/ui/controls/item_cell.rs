// ============================================================================
// 物品格控件 — MirItemCell (对应 C# MirItemCell.cs)
// ============================================================================
//
// 用于显示和交互物品的UI控件。支持多种物品网格类型：
// 背包、装备、仓库、交易、任务等。
// 处理物品的拖放、提示、右键使用等操作。

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;

/// 物品网格类型 (对应 C# MirGridType)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirGridType {
    /// 背包
    Inventory,
    /// 装备
    Equipment,
    /// 仓库
    Storage,
    /// 检查 (查看其他玩家装备)
    Inspect,
    /// 行会仓库
    GuildStorage,
    /// 交易 (自己的)
    Trade,
    /// 交易 (对方的)
    GuestTrade,
    /// 坐骑装备
    Mount,
    /// 钓鱼装备
    Fishing,
    /// 任务物品
    QuestInventory,
    /// 觉醒物品
    AwakenItem,
    /// 邮件
    Mail,
    /// 精炼
    Refine,
    /// 制造
    Craft,
    /// 宝石镶嵌
    Socket,
    /// 掉落面板
    DropPanel,
    /// 寄售商人
    TrustMerchant,
    /// 租借
    Renting,
    /// 客人租借
    GuestRenting,
    /// 英雄装备
    HeroEquipment,
    /// 英雄背包
    HeroInventory,
}

/// 物品格中显示的物品信息
#[derive(Debug, Clone)]
pub struct CellItemInfo {
    /// 物品唯一ID
    pub unique_id: u64,
    /// 物品模板索引
    pub item_index: i32,
    /// 物品名称
    pub name: String,
    /// 物品图像索引
    pub image: i16,
    /// 数量
    pub count: u32,
    /// 耐久度 (当前/最大)
    pub durability: (u16, u16),
    /// 是否已鉴定
    pub identified: bool,
}

/// 物品格控件
pub struct MirItemCell {
    /// 是否可见
    pub visible: bool,
    /// 是否启用
    pub enabled: bool,
    /// 位置 (屏幕坐标)
    pub position: (f32, f32),
    /// 尺寸
    pub size: (f32, f32),
    /// 网格类型
    pub grid_type: MirGridType,
    /// 物品槽位索引
    pub item_slot: i32,
    /// 当前物品信息
    pub item: Option<CellItemInfo>,
    /// 是否高亮 (鼠标悬停)
    pub highlighted: bool,
    /// 是否被选中 (拖动中)
    pub selected: bool,
    /// 特效颜色 (物品品质等)
    pub effect_color: Option<(u8, u8, u8, u8)>,
}

impl MirItemCell {
    /// 创建新的物品格
    pub fn new(grid_type: MirGridType, slot: i32) -> Self {
        Self {
            visible: true,
            enabled: true,
            position: (0.0, 0.0),
            size: (36.0, 32.0),
            grid_type,
            item_slot: slot,
            item: None,
            highlighted: false,
            selected: false,
            effect_color: None,
        }
    }

    /// 设置物品
    pub fn set_item(&mut self, item: Option<CellItemInfo>) {
        self.item = item;
        self.update_effect();
    }

    /// 更新特效颜色 (基于物品品质)
    fn update_effect(&mut self) {
        // TODO: 根据物品品质设置特效颜色
        // 普通=无, 优秀=蓝, 精良=绿, 稀有=金, 传说=紫
        self.effect_color = None;
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.item.is_none()
    }

    /// 处理鼠标悬停
    pub fn handle_hover(&mut self, x: f32, y: f32) -> bool {
        let was_highlighted = self.highlighted;
        self.highlighted = self.contains(x, y);
        self.highlighted != was_highlighted
    }

    /// 检查坐标是否在控件范围内
    pub fn contains(&self, x: f32, y: f32) -> bool {
        self.visible
            && x >= self.position.0
            && x <= self.position.0 + self.size.0
            && y >= self.position.1
            && y <= self.position.1 + self.size.1
    }

    /// 处理左键点击 (选取/放置物品)
    pub fn handle_click(&mut self, x: f32, y: f32) -> bool {
        if !self.visible || !self.enabled {
            return false;
        }
        if self.contains(x, y) {
            tracing::debug!(
                "🖱️ 物品格点击: {:?} slot={}",
                self.grid_type,
                self.item_slot
            );
            return true;
        }
        false
    }

    /// 处理右键点击 (使用物品)
    pub fn handle_right_click(&mut self, x: f32, y: f32) -> bool {
        if !self.visible || !self.enabled {
            return false;
        }
        if self.contains(x, y) {
            if let Some(ref item) = self.item {
                tracing::debug!("🖱️ 右键使用物品: {} (slot={})", item.name, self.item_slot);
                return true;
            }
        }
        false
    }

    /// 绘制物品格
    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult {
        if !self.visible {
            return Ok(());
        }
        // TODO: 绘制物品格背景
        // TODO: 如果有物品，绘制物品图像
        // TODO: 如果物品可堆叠，绘制数量
        // TODO: 如果物品有耐久度，绘制耐久条
        // TODO: 如果高亮，绘制高亮边框
        // TODO: 如果有特效颜色，绘制发光效果
        Ok(())
    }

    /// 绘制物品提示框 (Tooltip)
    pub fn draw_tooltip(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult {
        if !self.highlighted || self.item.is_none() {
            return Ok(());
        }
        // TODO: 绘制物品详细信息提示框
        // 包含：名称、类型、属性、耐久度、描述等
        Ok(())
    }
}
