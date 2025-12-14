// ============================================================================
// NpcGoodsDialogHybrid - NPC 商店（对齐 C# NPCGoodsDialog）
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/NPCDialogs.cs -> NPCGoodsDialog
// - 背景：Prguse[1000]
// - 商品单元：8 行列表（MirGoodsCell）
// - 滚动条：Prguse2[197-199] / [207-209] / [205-206]
// - 关闭按钮：Prguse2[360-362]
// - 购买按钮：Title[312-314]（Craft 时隐藏）
// - 标题标签：Title[27]（Craft 时 Title[12]）
// - New 图标：Prguse[550]
//
// 备注：Rust 侧已实现 C# 的子商品（BuySub）弹窗、以及数量输入框。

use macroquad::prelude::*;

use crate::network::NetContext;
use crate::resources::LibraryName;
use crate::scenes::dialogs::game::native_ui_utils::{
    draw_tooltip_at_mouse, ButtonState, ButtonTextures,
};
use crate::ui::text_renderer::draw_text_cn;

use mir2_shared::data::item::UserItem;
use mir2_shared::enums::PanelType;

#[derive(Debug, Clone)]
pub enum NpcGoodsDialogAction {
    /// 打开子商品列表（对齐 C# CheckSubGoods -> NPCSubGoodsDialog.NewGoods(list)）
    OpenSubGoods {
        items: Vec<UserItem>,
        rate: f32,
        hide_added_stats: bool,
    },
    /// 打开数量输入框（对齐 C# MirAmountBox）
    OpenAmountBox {
        title: String,
        image_index: u16,
        default_amount: u32,
        unique_id: u64,
        item_index: i32,
        stack_size: u16,
        unit_price: u32,
        use_pearls: bool,
    },
    /// 请求购买（对齐 C#：BuyItem 始终 Type=PanelType::Buy）
    RequestBuy {
        unique_id: u64,
        count: u32,
        item_index: i32,
        stack_size: u16,
        unit_price: u32,
        use_pearls: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HoverTarget {
    Cell(usize),
}

pub struct NpcGoodsDialogHybrid {
    visible: bool,

    // 对齐 C# 字段
    ptype: PanelType,
    use_pearls: bool,
    start_index: usize,
    selected_unique_id: Option<u64>,

    goods: Vec<UserItem>,
    display_goods: Vec<UserItem>,

    // 服务端下发
    npc_rate: f32,
    hide_added_stats: bool,

    // 输入
    last_click_time: f64,
    last_click_row: Option<usize>,

    // 滚动条拖动
    scroll_dragging: bool,
    scroll_drag_offset_y: f32,

    // 纹理
    bg_texture: Option<Texture2D>,
    bg_size: Vec2,

    close_btn: ButtonTextures,
    buy_btn: ButtonTextures,

    scroll_up_btn: ButtonTextures,
    scroll_down_btn: ButtonTextures,
    scroll_bar_btn: ButtonTextures,

    title_label: Option<Texture2D>,
    title_label_craft: Option<Texture2D>,
    new_icon: Option<Texture2D>,

    hover: Option<HoverTarget>,

    pending_action: Option<NpcGoodsDialogAction>,
}

impl NpcGoodsDialogHybrid {
    // 布局参数（与 C# 原版一致）
    const POS_X: f32 = 0.0;
    const POS_Y: f32 = 224.0;

    const CELL_X: f32 = 10.0;
    const CELL_Y: f32 = 34.0;
    const CELL_STEP_Y: f32 = 33.0;
    const CELL_W: f32 = 205.0;
    const CELL_H: f32 = 32.0;
    const CELL_ROWS: usize = 8;

    const ICON_W: f32 = 40.0;

    const CLOSE_X: f32 = 217.0;
    const CLOSE_Y: f32 = 3.0;

    const BUY_X: f32 = 77.0;
    const BUY_Y: f32 = 304.0;

    const LABEL_X: f32 = 20.0;
    const LABEL_Y: f32 = 9.0;

    const SCROLL_X: f32 = 219.0;
    const SCROLL_UP_Y: f32 = 35.0;
    const SCROLL_DOWN_Y: f32 = 284.0;

    // PositionBar：y in [49, 282 - bar_h]
    const SCROLL_BAR_MIN_Y: f32 = 49.0;
    const SCROLL_BAR_MAX_Y: f32 = 282.0;

    const DOUBLE_CLICK_TIME: f64 = 0.3;

    pub fn new() -> Self {
        Self {
            visible: false,

            ptype: PanelType::Buy,
            use_pearls: false,
            start_index: 0,
            selected_unique_id: None,

            goods: Vec::new(),
            display_goods: Vec::new(),

            npc_rate: 1.0,
            hide_added_stats: false,

            last_click_time: 0.0,
            last_click_row: None,

            scroll_dragging: false,
            scroll_drag_offset_y: 0.0,

            bg_texture: None,
            bg_size: vec2(0.0, 0.0),

            close_btn: ButtonTextures::load_from_library(LibraryName::Prguse2, 360),
            buy_btn: ButtonTextures::load_from_library(LibraryName::Title, 312),

            scroll_up_btn: ButtonTextures::load_from_library(LibraryName::Prguse2, 197),
            scroll_down_btn: ButtonTextures::load_from_library(LibraryName::Prguse2, 207),
            scroll_bar_btn: ButtonTextures::load_from_indices(LibraryName::Prguse2, [205, 206, 206]),

            title_label: LibraryName::Title.get_texture(27).map(|i| i.image).flatten(),
            title_label_craft: LibraryName::Title.get_texture(12).map(|i| i.image).flatten(),
            new_icon: LibraryName::Prguse.get_texture(550).map(|i| i.image).flatten(),

            hover: None,

            pending_action: None,
        }
    }

    pub fn take_action(&mut self) -> Option<NpcGoodsDialogAction> {
        self.pending_action.take()
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.scroll_dragging = false;
        self.pending_action = None;
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn rect(&self) -> Rect {
        Rect::new(Self::POS_X, Self::POS_Y, self.bg_size.x.max(235.0), self.bg_size.y.max(340.0))
    }

    pub fn is_mouse_over(&self, mouse_pos: Vec2) -> bool {
        self.visible && self.rect().contains(mouse_pos)
    }

    pub fn new_goods(&mut self, list: Vec<UserItem>, rate: f32, panel_type: PanelType, hide_added_stats: bool) {
        self.ptype = panel_type;
        self.npc_rate = rate;
        self.hide_added_stats = hide_added_stats;

        self.goods.clear();
        self.display_goods.clear();
        self.start_index = 0;
        self.selected_unique_id = None;
        self.pending_action = None;

        self.add_goods(list);
        self.show();
    }

    fn add_goods(&mut self, list: Vec<UserItem>) {
        // 对齐 C#：BuySub 时按价格排序（x.Price()）
        let mut list = list;
        if self.ptype == PanelType::BuySub {
            list.sort_by(|a, b| {
                let ap = a.info.as_ref().map(|i| i.price).unwrap_or(0);
                let bp = b.info.as_ref().map(|i| i.price).unwrap_or(0);
                ap.cmp(&bp).then_with(|| a.unique_id.cmp(&b.unique_id))
            });
        }

        for item in list {
            if self.ptype == PanelType::Buy && !self.use_pearls {
                self.goods.push(item.clone());
                if self
                    .display_goods
                    .iter()
                    .any(|x| x.item_index == item.item_index)
                {
                    continue;
                }
            }
            self.display_goods.push(item);
        }

        self.clamp_start_index();
    }

    fn clamp_start_index(&mut self) {
        if self.display_goods.len() <= Self::CELL_ROWS {
            self.start_index = 0;
            return;
        }
        let max_start = self.display_goods.len().saturating_sub(Self::CELL_ROWS);
        if self.start_index > max_start {
            self.start_index = max_start;
        }
    }

    fn ensure_textures_loaded(&mut self) {
        if self.bg_texture.is_some() {
            return;
        }
        if let Some(info) = LibraryName::Prguse.get_texture(1000) {
            self.bg_texture = info.image;
            self.bg_size = vec2(info.width as f32, info.height as f32);
        } else {
            // fallback（不应该发生）
            self.bg_size = vec2(235.0, 340.0);
        }
    }

    fn selected_item(&self) -> Option<&UserItem> {
        let uid = self.selected_unique_id?;
        self.display_goods.iter().find(|x| x.unique_id == uid)
    }

    fn cell_rect(row: usize) -> Rect {
        Rect::new(
            Self::POS_X + Self::CELL_X,
            Self::POS_Y + Self::CELL_Y + row as f32 * Self::CELL_STEP_Y,
            Self::CELL_W,
            Self::CELL_H,
        )
    }

    fn close_rect(&self) -> Rect {
        Rect::new(
            Self::POS_X + Self::CLOSE_X,
            Self::POS_Y + Self::CLOSE_Y,
            self.close_btn.size.x,
            self.close_btn.size.y,
        )
    }

    fn buy_rect(&self) -> Rect {
        Rect::new(
            Self::POS_X + Self::BUY_X,
            Self::POS_Y + Self::BUY_Y,
            self.buy_btn.size.x,
            self.buy_btn.size.y,
        )
    }

    fn scroll_up_rect(&self) -> Rect {
        Rect::new(
            Self::POS_X + Self::SCROLL_X,
            Self::POS_Y + Self::SCROLL_UP_Y,
            self.scroll_up_btn.size.x,
            self.scroll_up_btn.size.y,
        )
    }

    fn scroll_down_rect(&self) -> Rect {
        Rect::new(
            Self::POS_X + Self::SCROLL_X,
            Self::POS_Y + Self::SCROLL_DOWN_Y,
            self.scroll_down_btn.size.x,
            self.scroll_down_btn.size.y,
        )
    }

    fn scroll_bar_rect(&self) -> Rect {
        let bar_h = self.scroll_bar_btn.size.y.max(1.0);

        if self.display_goods.len() <= Self::CELL_ROWS {
            return Rect::new(Self::POS_X + Self::SCROLL_X, Self::POS_Y + Self::SCROLL_BAR_MIN_Y, self.scroll_bar_btn.size.x, bar_h);
        }

        // 对齐 C#：h = 233 - bar_h; pos = 49 + (h/(count-8))*StartIndex
        let count = self.display_goods.len();
        let track_h = 233.0 - bar_h;
        let steps = (count - Self::CELL_ROWS) as f32;
        let offset = (track_h / steps) * self.start_index as f32;
        let y = Self::SCROLL_BAR_MIN_Y + offset;

        Rect::new(Self::POS_X + Self::SCROLL_X, Self::POS_Y + y, self.scroll_bar_btn.size.x, bar_h)
    }

    fn can_scroll(&self) -> bool {
        self.display_goods.len() > Self::CELL_ROWS
    }

    fn scroll_by(&mut self, delta: i32) {
        if !self.can_scroll() {
            return;
        }
        if delta == 0 {
            return;
        }
        if delta < 0 {
            self.start_index = self.start_index.saturating_sub((-delta) as usize);
        } else {
            self.start_index = self.start_index.saturating_add(delta as usize);
        }
        self.clamp_start_index();
    }

    fn queue_buy_action(&mut self) {
        if self.pending_action.is_some() {
            return;
        }

        let Some(item) = self.selected_item().cloned() else {
            return;
        };

        // 对齐 C#：CheckSubGoods 仅在 Buy && !UsePearls
        if self.ptype == PanelType::Buy && !self.use_pearls {
            let list: Vec<UserItem> = self
                .goods
                .iter()
                .filter(|x| x.item_index == item.item_index)
                .cloned()
                .collect();

            if list.len() > 1 {
                self.pending_action = Some(NpcGoodsDialogAction::OpenSubGoods {
                    items: list,
                    rate: self.npc_rate,
                    hide_added_stats: self.hide_added_stats,
                });
                return;
            }
        }

        // 对齐 C#：StackSize > 1 弹 MirAmountBox
        if let Some(info) = item.info.as_ref() {
            let base_price = info.price as f32;
            let unit_price = (base_price * self.npc_rate).round() as u32;

            if info.stack_size > 1 {
                let default_amount = (item.count.max(1) as u32).min(info.stack_size as u32);

                self.pending_action = Some(NpcGoodsDialogAction::OpenAmountBox {
                    title: "Purchase Amount:".to_string(),
                    image_index: info.image,
                    default_amount,
                    unique_id: item.unique_id,
                    item_index: item.item_index,
                    stack_size: info.stack_size,
                    unit_price,
                    use_pearls: self.use_pearls,
                });
                return;
            }

            // 否则直接买 1（仍携带价格/堆叠信息供外层做前置校验）
            self.pending_action = Some(NpcGoodsDialogAction::RequestBuy {
                unique_id: item.unique_id,
                count: 1,
                item_index: item.item_index,
                stack_size: info.stack_size,
                unit_price,
                use_pearls: self.use_pearls,
            });
            return;
        }

        // 没有 info（价格/堆叠未知）时，按最小信息发起购买请求
        self.pending_action = Some(NpcGoodsDialogAction::RequestBuy {
            unique_id: item.unique_id,
            count: 1,
            item_index: item.item_index,
            stack_size: 1,
            unit_price: 0,
            use_pearls: self.use_pearls,
        });
    }

    fn draw_cell(&self, row: usize, item: &UserItem, selected: bool, hovered: bool, multiple_available: bool) {
        let rect = Self::cell_rect(row);

        // 边框（对齐 C#：BorderColour=Color.Lime，并在 x=40 处有分割线）
        let border_color = if selected {
            YELLOW
        } else {
            Color::new(0.0, 1.0, 0.0, 1.0)
        };

        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, border_color);
        draw_line(rect.x + Self::ICON_W, rect.y, rect.x + Self::ICON_W, rect.y + rect.h, 1.0, border_color);

        if hovered {
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::new(1.0, 1.0, 1.0, 0.06));
        }

        // 图标：优先使用 item.info.image；否则留空
        if let Some(info) = item.info.as_ref() {
            if let Some(img) = LibraryName::Items.get_texture(info.image as usize).and_then(|i| i.image) {
                let icon_w = img.width();
                let icon_h = img.height();
                let off_x = rect.x + (Self::ICON_W - icon_w) / 2.0;
                let off_y = rect.y + (rect.h - icon_h) / 2.0;
                draw_texture(&img, off_x, off_y, WHITE);
            }
        }

        // 名称/数量
        let name = item
            .info
            .as_ref()
            .map(|i| i.name.as_str())
            .unwrap_or_else(|| "<Unknown>");
        let name_text = if item.info.is_some() {
            name.to_string()
        } else {
            format!("Item #{}", item.item_index)
        };

        draw_text_cn(&name_text, rect.x + 44.0, rect.y + 12.0, 14.0, WHITE);

        if item.count > 1 {
            draw_text_cn(&format!("{}", item.count), rect.x + 23.0, rect.y + 28.0, 14.0, YELLOW);
        }

        // 价格（对齐 C#：Price: {price * rate} gold）
        let base_price = item.info.as_ref().map(|i| i.price).unwrap_or(0) as f32;
        let price = (base_price * self.npc_rate).round() as u32;
        let price_text = if self.use_pearls {
            format!("Price: {} pearl", price)
        } else {
            format!("Price: {} gold", price)
        };
        draw_text_cn(&price_text, rect.x + 44.0, rect.y + 28.0, 14.0, WHITE);

        // New 图标：对齐 C#：!IsShopItem || MultipleAvailable
        if let Some(icon) = self.new_icon.as_ref() {
            let show = !item.is_shop_item || multiple_available;
            if show {
                draw_texture(icon, rect.x + 190.0, rect.y + 5.0, WHITE);
            }
        }
    }

    pub fn update_and_draw(&mut self, net: Option<&NetContext>) -> bool {
        self.update_and_draw_with_input(net, true)
    }

    pub fn update_and_draw_with_input(&mut self, net: Option<&NetContext>, input_enabled: bool) -> bool {
        if !self.visible {
            return false;
        }

        self.ensure_textures_loaded();

        let (mx, my) = mouse_position();
        let mouse_pos = if input_enabled {
            vec2(mx, my)
        } else {
            vec2(-1.0e9, -1.0e9)
        };

        // 背景
        if let Some(bg) = self.bg_texture.as_ref() {
            draw_texture(bg, Self::POS_X, Self::POS_Y, WHITE);
        } else {
            draw_rectangle(Self::POS_X, Self::POS_Y, self.bg_size.x, self.bg_size.y, Color::new(0.0, 0.0, 0.0, 0.6));
        }

        // 标题
        let label_tex = if self.ptype == PanelType::Craft {
            self.title_label_craft.as_ref()
        } else {
            self.title_label.as_ref()
        };
        if let Some(tex) = label_tex {
            draw_texture(tex, Self::POS_X + Self::LABEL_X, Self::POS_Y + Self::LABEL_Y, WHITE);
        }

        // 关闭按钮
        let close_rect = self.close_rect();
        let close_clicked = if input_enabled {
            let state = ButtonState::from_mouse(close_rect, mouse_pos);
            self.close_btn.draw(vec2(close_rect.x, close_rect.y), state);
            ButtonState::is_clicked(close_rect, mouse_pos)
        } else {
            let state = ButtonState::from_mouse(close_rect, mouse_pos);
            self.close_btn.draw(vec2(close_rect.x, close_rect.y), state);
            false
        };
        if close_clicked {
            self.hide();
            return true;
        }

        // 滚动按钮
        let mut consumed = false;
        let _ = net;
        if input_enabled && self.can_scroll() {
            let up_rect = self.scroll_up_rect();
            if self.scroll_up_btn.draw_button(up_rect, mouse_pos) {
                self.scroll_by(-1);
                consumed = true;
            }

            let down_rect = self.scroll_down_rect();
            if self.scroll_down_btn.draw_button(down_rect, mouse_pos) {
                self.scroll_by(1);
                consumed = true;
            }

            // PositionBar 拖动
            let bar_rect = self.scroll_bar_rect();
            let bar_state = ButtonState::from_mouse(bar_rect, mouse_pos);
            self.scroll_bar_btn.draw(vec2(bar_rect.x, bar_rect.y), bar_state);

            if is_mouse_button_pressed(MouseButton::Left) && bar_rect.contains(mouse_pos) {
                self.scroll_dragging = true;
                self.scroll_drag_offset_y = mouse_pos.y - bar_rect.y;
                consumed = true;
            }
            if is_mouse_button_released(MouseButton::Left) {
                self.scroll_dragging = false;
            }
            if self.scroll_dragging {
                let bar_h = bar_rect.h.max(1.0);
                let mut y = (mouse_pos.y - self.scroll_drag_offset_y) - Self::POS_Y;
                let max_y = Self::SCROLL_BAR_MAX_Y - bar_h;
                if y < Self::SCROLL_BAR_MIN_Y {
                    y = Self::SCROLL_BAR_MIN_Y;
                }
                if y > max_y {
                    y = max_y;
                }

                let count = self.display_goods.len();
                if count > Self::CELL_ROWS {
                    let track_h = 233.0 - bar_h;
                    let steps = (count - Self::CELL_ROWS) as f32;
                    let new_start = ((y - Self::SCROLL_BAR_MIN_Y) / (track_h / steps)).round() as i32;
                    let new_start = new_start.clamp(0, (count - Self::CELL_ROWS) as i32) as usize;
                    if new_start != self.start_index {
                        self.start_index = new_start;
                    }
                }
                consumed = true;
            }
        }

        // 鼠标滚轮（对齐 C#：StartIndex -= count）
        let wheel = mouse_wheel().1;
        if input_enabled && wheel != 0.0 {
            // macroquad：向上滚 wheel>0
            if self.rect().contains(mouse_pos) {
                let delta = if wheel > 0.0 { -1 } else { 1 };
                self.scroll_by(delta);
                consumed = true;
            }
        }

        // 商品列表
        self.hover = None;

        for row in 0..Self::CELL_ROWS {
            let idx = self.start_index + row;
            if idx >= self.display_goods.len() {
                continue;
            }

            let mut trigger_buy = false;

            let item_unique_id = self.display_goods[idx].unique_id;
            let item_index = self.display_goods[idx].item_index;
            let rect = Self::cell_rect(row);
            let hovered = rect.contains(mouse_pos);
            let selected = self.selected_unique_id == Some(item_unique_id);

            // 对齐 C#：MultipleAvailable = matchingGoods.Count()>1 && matchingGoods.Any(!IsShopItem)
            let multiple_available = if self.ptype == PanelType::Buy && !self.use_pearls {
                let mut count = 0usize;
                let mut any_not_shop = false;
                for g in self.goods.iter().filter(|g| g.item_index == item_index) {
                    count += 1;
                    if !g.is_shop_item {
                        any_not_shop = true;
                    }
                }
                count > 1 && any_not_shop
            } else {
                false
            };

            if input_enabled && hovered {
                self.hover = Some(HoverTarget::Cell(row));

                // 单击选择 + 双击购买
                if is_mouse_button_released(MouseButton::Left) {
                    let now = get_time();
                    let is_double = self.last_click_row == Some(row)
                        && (now - self.last_click_time) <= Self::DOUBLE_CLICK_TIME;

                    self.selected_unique_id = Some(item_unique_id);

                    if is_double && self.ptype != PanelType::Craft {
                        trigger_buy = true;
                        consumed = true;
                    }

                    self.last_click_time = now;
                    self.last_click_row = Some(row);
                }

                // 悬停提示：对齐 C# 的 CreateItemLabel（这里用简化 tooltip）
                let (title, base_price) = {
                    let item = &self.display_goods[idx];
                    let title = item
                        .info
                        .as_ref()
                        .map(|i| i.name.clone())
                        .unwrap_or_else(|| format!("Item #{}", item.item_index));
                    let base_price = item.info.as_ref().map(|i| i.price).unwrap_or(0) as f32;
                    (title, base_price)
                };
                let price = (base_price * self.npc_rate).round() as u32;
                let tip = if self.use_pearls {
                    format!("{}\nPrice: {} pearl", title, price)
                } else {
                    format!("{}\nPrice: {} gold", title, price)
                };
                draw_tooltip_at_mouse(&tip, vec2(14.0, 14.0));
            }

            {
                let item = &self.display_goods[idx];
                self.draw_cell(row, item, selected, hovered, multiple_available);
            }

            if input_enabled && trigger_buy {
                self.queue_buy_action();
            }
        }

        // Buy 按钮（Craft 隐藏）
        if input_enabled && self.ptype != PanelType::Craft {
            let buy_rect = self.buy_rect();
            if self.buy_btn.draw_button(buy_rect, mouse_pos) {
                self.queue_buy_action();
                consumed = true;
            }
        }

        // 对齐 C#：HideAddedStoreStats（暂仅保存字段，供将来 item label 使用）
        let _ = self.hide_added_stats;

        consumed
    }
}
