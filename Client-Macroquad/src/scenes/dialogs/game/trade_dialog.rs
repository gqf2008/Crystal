// ============================================================================
// TradeDialogHybrid - 交易对话框（混合版本）
// ============================================================================
//
// 【C# 原版参考】
// - 背景: Title[22]
// - 标题: Title[18]
// - 关闭按钮: Prguse2[360/361/362]
// - 两侧物品栏（左侧自己的，右侧对方的）
// - 底部: 金币输入框 + 确认按钮 + 取消按钮
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::{DragHelper, ButtonTextures, ItemDragState, CellHighlight, CellStyle, draw_cell_frame, draw_item_icon, draw_item_count};
use tracing;

/// 交易栏位物品
#[derive(Debug, Clone)]
pub struct TradeItemSlot {
    pub icon_index: Option<usize>,
    pub name: String,
    pub count: u32,
}

/// 交易对话框（混合版本）
pub struct TradeDialogHybrid {
    /// 窗口位置
    position: Vec2,
    /// 是否可见
    visible: bool,
    /// 对话框尺寸
    size: Vec2,
    /// 交易对方名称
    partner_name: String,

    // 自己的交易内容
    my_items: Vec<TradeItemSlot>,
    my_gold: u32,
    my_confirmed: bool,

    // 对方的交易内容
    their_items: Vec<TradeItemSlot>,
    their_gold: u32,
    their_confirmed: bool,

    /// 是否被对方锁定（对方已确认）
    partner_locked: bool,

    /// 拖拽中的物品
    dragging_item: Option<ItemDragState>,
    /// 拖拽源（用于判断是添加还是移除）
    drag_source: Option<DragSource>,

    /// 金币输入状态
    editing_gold: bool,
    gold_input_text: String,

    /// 拖拽辅助器
    drag_helper: DragHelper,
    /// 背景纹理
    bg_texture: Option<Texture2D>,
    /// 标题纹理
    title_texture: Option<Texture2D>,
    /// 关闭按钮
    close_button_textures: [Option<Texture2D>; 3],
    /// 确认按钮纹理
    confirm_btn: ButtonTextures,
    /// 取消按钮纹理
    cancel_btn: ButtonTextures,
    /// 物品图标纹理缓存
    item_texture_cache: std::collections::HashMap<usize, Texture2D>,
}

impl TradeDialogHybrid {
    pub fn new() -> Self {
        Self {
            position: vec2(250.0, 100.0),
            visible: false,
            size: vec2(340.0, 380.0),
            partner_name: String::new(),
            my_items: Vec::new(),
            my_gold: 0,
            my_confirmed: false,
            their_items: Vec::new(),
            their_gold: 0,
            their_confirmed: false,
            partner_locked: false,
            editing_gold: false,
            gold_input_text: String::new(),
            drag_helper: DragHelper::new(),
            bg_texture: None,
            title_texture: None,
            close_button_textures: [None, None, None],
            confirm_btn: ButtonTextures::new(),
            cancel_btn: ButtonTextures::new(),
            item_texture_cache: std::collections::HashMap::new(),
            dragging_item: None,
            drag_source: None,
        }
    }

    /// 打开交易窗口（对方发起并接受）
    pub fn open_trade(&mut self, partner: &str) {
        self.partner_name = partner.to_string();
        self.my_items.clear();
        self.my_gold = 0;
        self.my_confirmed = false;
        self.their_items.clear();
        self.their_gold = 0;
        self.their_confirmed = false;
        self.partner_locked = false;
        self.editing_gold = false;
        self.gold_input_text.clear();
        self.visible = true;
    }

    /// 打开交易对话框
    pub fn open(&mut self) {
        self.open_trade("");
    }

    /// 检查点是否在对话框内
    pub fn contains(&self, point: Vec2) -> bool {
        if !self.visible {
            return false;
        }
        Rect::new(self.position.x, self.position.y, self.size.x, self.size.y).contains(point)
    }

    /// 关闭对话框
    pub fn close(&mut self) {
        self.visible = false;
        self.my_items.clear();
        self.their_items.clear();
        self.my_confirmed = false;
        self.their_confirmed = false;
        self.partner_locked = false;
        self.item_texture_cache.clear();
    }

    /// 切换显示状态
    #[allow(dead_code)]
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// 是否可见
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// 设置位置
    pub fn set_position(&mut self, pos: Vec2) {
        self.position = pos;
    }

    /// 获取位置
    pub fn get_position(&self) -> Vec2 {
        self.position
    }

    // === 网络事件同步方法 ===

    /// 设置对方的金币数量
    pub fn set_their_gold(&mut self, gold: u32) {
        self.their_gold = gold;
        self.their_confirmed = false;
    }

    /// 设置对方的物品列表
    pub fn set_their_items(&mut self, items: &[mir2_shared::UserItem]) {
        self.their_items.clear();
        for item in items {
            let icon = item.info.as_ref().map(|info| info.shape as usize);
            let name = item.info.as_ref()
                .map(|info| info.name.to_string())
                .unwrap_or_else(|| "物品".to_string());
            self.their_items.push(TradeItemSlot {
                icon_index: icon,
                name,
                count: (item.count as u32).max(1),
            });
        }
        self.their_confirmed = false;
    }

    /// 对方已确认锁定
    pub fn set_partner_locked(&mut self, locked: bool) {
        self.partner_locked = locked;
    }

    /// 对方取消确认
    pub fn unset_partner_locked(&mut self) {
        self.partner_locked = false;
    }

    /// 增加对方的金币（服务器 TradeGoldAdded 事件）
    pub fn add_their_gold(&mut self, amount: u32) {
        self.their_gold = self.their_gold.saturating_add(amount);
        self.their_confirmed = false;
    }

    /// 重置双方确认状态（服务器 TradeCancelled 事件）
    pub fn reset_confirmations(&mut self) {
        self.my_confirmed = false;
        self.their_confirmed = false;
        self.partner_locked = false;
    }

    /// 设置对方确认/锁定状态（服务器 TradeConfirmedEvent）
    pub fn set_partner_confirmed(&mut self, locked: bool) {
        self.partner_locked = locked;
    }

    /// 添加自己的物品到交易栏
    pub fn add_my_item(&mut self, item: &mir2_shared::UserItem) {
        let icon = item.info.as_ref().map(|info| info.shape as usize);
        let name = item.info.as_ref()
            .map(|info| info.name.to_string())
            .unwrap_or_else(|| "物品".to_string());
        self.my_items.push(TradeItemSlot {
            icon_index: icon,
            name,
            count: (item.count as u32).max(1),
        });
        self.my_confirmed = false;
    }

    /// 移除自己的物品
    pub fn remove_my_item(&mut self, index: usize) {
        if index < self.my_items.len() {
            self.my_items.remove(index);
            self.my_confirmed = false;
        }
    }

    /// 清空自己的交易物品
    pub fn clear_my_items(&mut self) {
        self.my_items.clear();
        self.my_confirmed = false;
    }

    /// 设置自己的金币
    pub fn set_my_gold(&mut self, gold: u32) {
        self.my_gold = gold;
        self.my_confirmed = false;
    }

    // === 用户操作 ===

    /// 用户确认交易（发送 TradeConfirm 请求）
    pub fn confirm_trade(&mut self) -> bool {
        if self.partner_locked {
            self.my_confirmed = true;
            true
        } else {
            false
        }
    }

    /// 用户取消交易
    pub fn cancel_trade(&mut self) {
        self.my_confirmed = false;
    }

    fn on_confirm(&mut self) -> TradeAction {
        if self.partner_locked {
            self.my_confirmed = true;
            TradeAction::Confirm
        } else {
            TradeAction::None
        }
    }

    fn on_cancel(&mut self) -> TradeAction {
        self.my_confirmed = false;
        TradeAction::Cancel
    }

    /// 异步加载纹理
    pub fn load_textures(&mut self) {
        // 背景纹理 - Title[22]（待确认实际索引）
        if let Some(texture) = LibraryName::Title.get_texture(22) {
            self.size = vec2(texture.width as f32, texture.height as f32);
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
                tracing::debug!("💱 交易对话框背景 Title[22]: {}x{}", texture.width, texture.height);
            }
        }

        // 标题纹理 - Title[18]
        if let Some(texture) = LibraryName::Title.get_texture(18) {
            if let Some(tex) = texture.image {
                self.title_texture = Some(tex);
                tracing::debug!("💱 交易对话框标题 Title[18] 加载成功");
            }
        }

        // 关闭按钮
        for (i, idx) in [360, 361, 362].iter().enumerate() {
            if let Some(texture) = LibraryName::Prguse2.get_texture(*idx) {
                if let Some(tex) = texture.image {
                    self.close_button_textures[i] = Some(tex);
                }
            }
        }

        // 确认/取消按钮 - Prguse[1960-1962] (使用菜单按钮样式)
        self.confirm_btn = ButtonTextures::load_from_indices(LibraryName::Prguse, [1960, 1961, 1962]);
        self.cancel_btn = ButtonTextures::load_from_indices(LibraryName::Prguse, [1960, 1961, 1962]);

        tracing::debug!("💱 交易对话框纹理加载完成");
    }

    /// 更新和绘制
    pub fn update_and_draw(&mut self) -> TradeAction {
        if !self.visible {
            return TradeAction::None;
        }

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);
        let mouse_down = is_mouse_button_down(MouseButton::Left);
        let mouse_just_pressed = is_mouse_button_pressed(MouseButton::Left);

        // 拖拽
        let drag_area = Rect::new(self.position.x, self.position.y, self.size.x, 30.0);
        self.drag_helper.apply(drag_area, &mut self.position);

        // 绘制背景
        self.draw_background();

        // 绘制双方物品栏
        self.draw_item_panels(mouse_pos, mouse_down, mouse_just_pressed);

        // 绘制底部控制按钮
        let action = self.draw_controls(mouse_pos);

        // 绘制拖拽中的物品（最上层）
        if self.dragging_item.is_some() {
            self.draw_dragging_item();
        }

        // 检查拖拽释放：从交易栏移除
        if is_mouse_button_released(MouseButton::Left) {
            if let (Some(_drag), Some(DragSource::MyTradeSlot(slot_idx))) = (&self.dragging_item, &self.drag_source) {
                let removed_idx = *slot_idx;
                self.dragging_item = None;
                self.drag_source = None;
                self.my_confirmed = false;
                return TradeAction::RemoveItem { slot_index: removed_idx };
            }
            self.dragging_item = None;
            self.drag_source = None;
        }

        action
    }

    /// 绘制背景
    fn draw_background(&self) {
        if let Some(texture) = &self.bg_texture {
            draw_texture_ex(
                texture,
                self.position.x,
                self.position.y,
                WHITE,
                DrawTextureParams::default(),
            );
        } else {
            draw_rectangle(
                self.position.x,
                self.position.y,
                self.size.x,
                self.size.y,
                Color::from_rgba(40, 45, 50, 240),
            );
            draw_rectangle_lines(
                self.position.x,
                self.position.y,
                self.size.x,
                self.size.y,
                2.0,
                Color::from_rgba(150, 150, 150, 255),
            );
        }

        // 标题
        if let Some(title_tex) = &self.title_texture {
            draw_texture_ex(
                title_tex,
                self.position.x + 10.0,
                self.position.y + 5.0,
                WHITE,
                DrawTextureParams::default(),
            );
        } else {
            let title = format!("交易 - {}", self.partner_name);
            let tw = title.chars().count() as f32 * 7.0;
            draw_text_cn(
                &title,
                self.position.x + (self.size.x - tw) / 2.0,
                self.position.y + 14.0,
                14.0,
                WHITE,
            );
        }
    }

    /// 绘制双方物品面板（支持拖拽交互）
    fn draw_item_panels(&mut self, mouse_pos: Vec2, mouse_down: bool, mouse_just_pressed: bool) {
        let panel_w = 140.0;
        let panel_h = 200.0;
        let top_y = self.position.y + 35.0;
        let left_x = self.position.x + 10.0;
        let right_x = self.position.x + self.size.x - 10.0 - panel_w;

        // 左侧面板（自己的）- 可点击移除
        self.draw_item_panel_slots(
            left_x, top_y, panel_w, panel_h,
            "我的", mouse_pos, mouse_down, mouse_just_pressed,
            true, // 可移除
        );

        // 右侧面板（对方的）- 仅显示
        Self::draw_item_panel_slots_static(
            right_x, top_y, panel_w, panel_h,
            &self.partner_name, &self.their_items, mouse_pos,
        );

        // 中间箭头
        let mid_x = self.position.x + self.size.x / 2.0;
        draw_text_cn("→", mid_x - 5.0, top_y + panel_h / 2.0 - 5.0, 20.0, Color::from_rgba(200, 200, 200, 255));
    }

    /// 绘制物品格子槽位（支持交互）
    fn draw_item_panel_slots(
        &mut self,
        x: f32, y: f32, w: f32, h: f32,
        title: &str,
        mouse_pos: Vec2,
        mouse_down: bool,
        mouse_just_pressed: bool,
        can_remove: bool,
    ) {
        // 面板背景
        draw_rectangle(x, y, w, h, Color::from_rgba(30, 30, 35, 200));
        draw_rectangle_lines(x, y, w, h, 1.0, Color::from_rgba(100, 100, 100, 255));

        // 标题
        draw_text_cn(title, x + 5.0, y + 12.0, 11.0, Color::from_rgba(200, 200, 200, 255));

        let slot_size = 32.0;
        let cols = 4;
        let slot_start_x = x + 5.0;
        let slot_start_y = y + 25.0;
        let style = CellStyle::inventory_style();
        let item_count = self.my_items.len();

        for i in 0..12 {
            let col = i % cols;
            let row = i / cols;
            let sx = slot_start_x + col as f32 * (slot_size + 2.0);
            let sy = slot_start_y + row as f32 * (slot_size + 2.0);
            let slot_rect = Rect::new(sx, sy, slot_size, slot_size);

            let has_item = i < item_count;
            let is_hovered = slot_rect.contains(mouse_pos);

            // 高亮状态
            let highlight = if is_hovered && has_item && can_remove {
                if mouse_down {
                    CellHighlight::DragTarget
                } else {
                    CellHighlight::Hovered
                }
            } else {
                CellHighlight::None
            };

            draw_cell_frame(slot_rect, highlight, &style);

            // 物品图标
            if let Some(slot) = self.my_items.get(i) {
                let icon_idx = slot.icon_index;
                let count = slot.count;

                if let Some(icon_idx) = icon_idx {
                    // 尝试从缓存获取纹理
                    let cache_hit = self.item_texture_cache.contains_key(&icon_idx);
                    if !cache_hit {
                        if let Some(info) = LibraryName::Prguse2.get_texture(icon_idx % 500) {
                            if let Some(tex) = info.image {
                                self.item_texture_cache.insert(icon_idx, tex);
                            }
                        }
                    }

                    if let Some(texture) = self.item_texture_cache.get(&icon_idx) {
                        draw_item_icon(slot_rect, texture, 1.0);
                    } else {
                        // 无纹理时用色块占位
                        let color = match i % 4 {
                            0 => Color::from_rgba(200, 150, 50, 255),
                            1 => Color::from_rgba(100, 150, 255, 255),
                            2 => Color::from_rgba(150, 255, 100, 255),
                            _ => Color::from_rgba(255, 100, 100, 255),
                        };
                        draw_rectangle(sx + 4.0, sy + 4.0, slot_size - 8.0, slot_size - 8.0, color);
                    }

                    // 数量
                    if count > 1 {
                        draw_item_count(slot_rect, count, true);
                    }
                }
            }

            // 点击/拖拽处理：从交易栏移除物品
            if can_remove && has_item && is_hovered {
                if mouse_just_pressed {
                    let icon_idx = self.my_items.get(i).and_then(|s| s.icon_index).unwrap_or(0);
                    let count = self.my_items.get(i).map(|s| s.count).unwrap_or(1);
                    self.dragging_item = Some(ItemDragState::new(i, icon_idx, count));
                    self.drag_source = Some(DragSource::MyTradeSlot(i));
                }
            }
        }
    }

    /// 绘制物品格子槽位（静态，仅显示）
    fn draw_item_panel_slots_static(
        x: f32, y: f32, w: f32, h: f32,
        title: &str,
        items: &[TradeItemSlot],
        mouse_pos: Vec2,
    ) {
        // 面板背景
        draw_rectangle(x, y, w, h, Color::from_rgba(30, 30, 35, 200));
        draw_rectangle_lines(x, y, w, h, 1.0, Color::from_rgba(100, 100, 100, 255));

        // 标题
        draw_text_cn(title, x + 5.0, y + 12.0, 11.0, Color::from_rgba(200, 200, 200, 255));

        let slot_size = 32.0;
        let cols = 4;
        let slot_start_x = x + 5.0;
        let slot_start_y = y + 25.0;
        let style = CellStyle::default();

        for i in 0..12 {
            let col = i % cols;
            let row = i / cols;
            let sx = slot_start_x + col as f32 * (slot_size + 2.0);
            let sy = slot_start_y + row as f32 * (slot_size + 2.0);
            let slot_rect = Rect::new(sx, sy, slot_size, slot_size);

            let is_hovered = slot_rect.contains(mouse_pos);
            let highlight = if is_hovered { CellHighlight::Hovered } else { CellHighlight::None };
            draw_cell_frame(slot_rect, highlight, &style);

            if let Some(slot) = items.get(i) {
                if slot.icon_index.is_some() {
                    let color = match i % 4 {
                        0 => Color::from_rgba(200, 150, 50, 255),
                        1 => Color::from_rgba(100, 150, 255, 255),
                        2 => Color::from_rgba(150, 255, 100, 255),
                        _ => Color::from_rgba(255, 100, 100, 255),
                    };
                    draw_rectangle(sx + 4.0, sy + 4.0, slot_size - 8.0, slot_size - 8.0, color);
                }

                if slot.count > 1 {
                    draw_item_count(slot_rect, slot.count, true);
                }
            }
        }
    }

    /// 绘制拖拽中的物品（跟随鼠标）
    fn draw_dragging_item(&self) {
        if let Some(drag) = &self.dragging_item {
            let (mx, my) = mouse_position();
            // 绘制半透明物品图标
            if let Some(info) = LibraryName::Prguse2.get_texture(drag.icon_index % 500) {
                if let Some(tex) = info.image {
                    let color = Color::new(1.0, 1.0, 1.0, 0.6);
                    draw_texture(&tex, mx - 16.0, my - 16.0, color);
                }
            }
            // 数量
            if drag.count > 1 {
                draw_text_cn(&format!("x{}", drag.count), mx + 10.0, my + 5.0, 12.0, Color::from_rgba(255, 255, 100, 200));
            }
        }
    }

    /// 绘制底部控制按钮
    fn draw_controls(&mut self, mouse_pos: Vec2) -> TradeAction {
        let bottom_y = self.position.y + self.size.y - 50.0;
        let btn_w = 60.0;
        let btn_h = 24.0;

        // 金币输入框（左侧）
        let gold_input_w = 80.0;
        let gold_input_h = 20.0;
        let gold_input_x = self.position.x + 10.0;
        let gold_input_rect = Rect::new(gold_input_x, bottom_y + 2.0, gold_input_w, gold_input_h);
        let gold_bg = if self.editing_gold {
            Color::from_rgba(50, 50, 60, 255)
        } else {
            Color::from_rgba(40, 40, 45, 255)
        };
        draw_rectangle(gold_input_rect.x, gold_input_rect.y, gold_input_rect.w, gold_input_rect.h, gold_bg);
        draw_rectangle_lines(gold_input_rect.x, gold_input_rect.y, gold_input_rect.w, gold_input_rect.h, 1.0, Color::from_rgba(120, 120, 120, 255));

        let gold_display = if self.editing_gold {
            &self.gold_input_text
        } else {
            &format!("{}", self.my_gold)
        };
        let gold_text_color = if self.editing_gold { WHITE } else { Color::from_rgba(255, 215, 0, 255) };
        draw_text_cn(gold_display, gold_input_rect.x + 5.0, gold_input_rect.y + 13.0, 11.0, gold_text_color);

        // 点击输入框激活
        if gold_input_rect.contains(mouse_pos) && is_mouse_button_pressed(MouseButton::Left) {
            self.editing_gold = !self.editing_gold;
            if self.editing_gold {
                self.gold_input_text = "0".to_string();
            }
        } else if self.editing_gold && is_mouse_button_pressed(MouseButton::Left) && !gold_input_rect.contains(mouse_pos) {
            self.editing_gold = false;
        }

        // 对方锁定状态
        if self.partner_locked {
            draw_text_cn("已锁定", self.position.x + self.size.x / 2.0 - 15.0, bottom_y + 6.0, 12.0, Color::from_rgba(50, 200, 50, 255));
        } else {
            draw_text_cn("未锁定", self.position.x + self.size.x / 2.0 - 15.0, bottom_y + 6.0, 12.0, Color::from_rgba(255, 100, 100, 255));
        }

        // 确认按钮
        let confirm_x = self.position.x + self.size.x - btn_w - 70.0;
        let confirm_rect = Rect::new(confirm_x, bottom_y, btn_w, btn_h);
        let confirm_hovered = confirm_rect.contains(mouse_pos);
        let confirm_color = if confirm_hovered {
            Color::from_rgba(60, 180, 60, 255)
        } else {
            Color::from_rgba(40, 140, 40, 255)
        };
        draw_rectangle(confirm_rect.x, confirm_rect.y, confirm_rect.w, confirm_rect.h, confirm_color);
        draw_text_cn("确认", confirm_x + 18.0, bottom_y + 6.0, 12.0, WHITE);

        // 取消按钮
        let cancel_x = self.position.x + self.size.x - 10.0 - btn_w;
        let cancel_rect = Rect::new(cancel_x, bottom_y, btn_w, btn_h);
        let cancel_hovered = cancel_rect.contains(mouse_pos);
        let cancel_color = if cancel_hovered {
            Color::from_rgba(200, 60, 60, 255)
        } else {
            Color::from_rgba(160, 40, 40, 255)
        };
        draw_rectangle(cancel_rect.x, cancel_rect.y, cancel_rect.w, cancel_rect.h, cancel_color);
        draw_text_cn("取消", cancel_x + 18.0, bottom_y + 6.0, 12.0, WHITE);

        // 点击处理
        if is_mouse_button_pressed(MouseButton::Left) {
            if confirm_rect.contains(mouse_pos) {
                return self.on_confirm();
            }
            if cancel_rect.contains(mouse_pos) {
                return self.on_cancel();
            }
        }

        // 键盘输入处理（金币输入框）
        if self.editing_gold {
            if let Some(action) = self.process_gold_input() {
                return action;
            }
        }

        // ESC: 如果在编辑则取消编辑，否则取消交易
        if is_key_pressed(KeyCode::Escape) {
            if self.editing_gold {
                self.editing_gold = false;
                self.gold_input_text.clear();
            } else {
                return self.on_cancel();
            }
        }

        TradeAction::None
    }

    /// 处理金币输入的键盘事件（返回 TradeAction 如果提交了）
    fn process_gold_input(&mut self) -> Option<TradeAction> {
        // 数字键
        for key in [KeyCode::Key0, KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4,
                    KeyCode::Key5, KeyCode::Key6, KeyCode::Key7, KeyCode::Key8, KeyCode::Key9] {
            if is_key_pressed(key) {
                let digit = match key {
                    KeyCode::Key0 => '0', KeyCode::Key1 => '1', KeyCode::Key2 => '2',
                    KeyCode::Key3 => '3', KeyCode::Key4 => '4', KeyCode::Key5 => '5',
                    KeyCode::Key6 => '6', KeyCode::Key7 => '7', KeyCode::Key8 => '8',
                    KeyCode::Key9 => '9', _ => unreachable!(),
                };
                if self.gold_input_text.len() < 10 {
                    if self.gold_input_text == "0" {
                        self.gold_input_text = digit.to_string();
                    } else {
                        self.gold_input_text.push(digit);
                    }
                }
                return None; // 一次只处理一个按键
            }
        }

        // Backspace
        if is_key_pressed(KeyCode::Backspace) {
            self.gold_input_text.pop();
            if self.gold_input_text.is_empty() {
                self.gold_input_text = "0".to_string();
            }
            return None;
        }

        // Enter 确认金币数量
        if is_key_pressed(KeyCode::Enter) {
            if let Ok(amount) = self.gold_input_text.parse::<u32>() {
                self.editing_gold = false;
                return Some(TradeAction::SetGold { amount });
            }
        }

        None
    }

    /// 提交金币输入（解析并返回金额）
    #[allow(dead_code)]
    pub fn commit_gold_input(&mut self) -> Option<u32> {
        if self.editing_gold {
            self.editing_gold = false;
        }
        self.gold_input_text.parse::<u32>().ok()
    }

    /// 获取物品槽位的物品信息（用于tooltip）
    #[allow(dead_code)]
    pub fn get_slot_tooltip(&self, slot_index: usize) -> Option<&TradeItemSlot> {
        if slot_index < self.my_items.len() {
            Some(&self.my_items[slot_index])
        } else if slot_index >= 100 {
            let their_idx = slot_index - 100;
            if their_idx < self.their_items.len() {
                Some(&self.their_items[their_idx])
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// 拖拽源类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DragSource {
    /// 从自己的交易栏拖出（移除）
    MyTradeSlot(usize),
}

/// 交易操作返回
#[derive(Debug, Clone, PartialEq)]
pub enum TradeAction {
    None,
    /// 确认交易（发送 TradeConfirm 请求）
    Confirm,
    /// 取消交易
    Cancel,
    /// 添加物品到交易栏（索引来自背包/地面）
    AddItem { item_index: usize },
    /// 从交易栏移除物品
    RemoveItem { slot_index: usize },
    /// 设置自己的金币数量
    SetGold { amount: u32 },
}
