// ============================================================================
// GameShopDialogHybrid - 渲染模块
// ============================================================================
// 
// 所有绘制和交互相关方法
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::types::{ShopSectionHybrid, ShopClassHybrid, ShopCategoryHybrid};
use super::dialog::GameShopDialogHybrid;
use super::super::native_ui_utils::{
    draw_library_button_with_offset,
    draw_library_image_with_offset,
    mouse_pos as ui_mouse_pos,
};

impl GameShopDialogHybrid {
    /// 更新和绘制
    pub fn update_and_draw(&mut self) {
        if !self.visible { return; }
        
        let pos = self.position;
        
        // 1. 绘制背景
        self.draw_background(pos);
        
        // 2. 绘制标题
        self.draw_title(pos);
        
        // 3. 绘制分类标签
        self.draw_section_tabs(pos);
        self.draw_class_tabs(pos);
        
        // 4. 绘制左侧分类列表
        self.draw_category_list(pos);
        
        // 5. 绘制商品网格
        self.draw_item_grid(pos);
        
        // 6. 绘制分页
        self.draw_pagination(pos);
        
        // 7. 绘制货币信息和支付方式
        self.draw_currency_info(pos);
        self.draw_payment_options(pos);
        
        // 8. 绘制搜索框
        self.draw_search_box(pos);
        
        // 9. 绘制预览窗口
        self.draw_preview_window(pos);
        
        // 10. 绘制悬停提示框（需要在最后绘制以显示在最上层）
        self.draw_item_tooltip();
        
        // 11. 处理拖拽（使用mqui）
        self.handle_dragging(pos);
        
        // 12. 处理关闭按钮
        self.handle_close_button(pos);
    }
    
    /// 绘制背景
    pub(super) fn draw_background(&self, pos: Vec2) {
        if let Some(ref tex) = self.background_texture {
            draw_texture_ex(tex, pos.x, pos.y, WHITE, DrawTextureParams {
                dest_size: Some(vec2(Self::DIALOG_WIDTH, Self::DIALOG_HEIGHT)),
                ..Default::default()
            });
        } else {
            // 备用背景
            draw_rectangle(pos.x, pos.y, Self::DIALOG_WIDTH, Self::DIALOG_HEIGHT, 
                Color::from_rgba(40, 40, 50, 240));
            draw_rectangle_lines(pos.x, pos.y, Self::DIALOG_WIDTH, Self::DIALOG_HEIGHT, 
                2.0, Color::from_rgba(100, 100, 120, 255));
        }
    }
    
    /// 绘制标题
    pub(super) fn draw_title(&self, pos: Vec2) {
        // 标题图标 Title[26] (原版位置: 18, 9)
        if let Some(ref tex) = self.title_label_texture {
            draw_texture_ex(tex, pos.x + 18.0, pos.y + 9.0, WHITE, DrawTextureParams::default());
        } else {
            draw_text_cn("🛒 游戏商城", pos.x + 20.0, pos.y + 25.0, 18.0, 
                Color::from_rgba(255, 215, 0, 255));
        }
    }
    
    /// 绘制主分类标签 (使用纹理)
    pub(super) fn draw_section_tabs(&mut self, pos: Vec2) {
        let mouse_pos = ui_mouse_pos();

        // Section Tabs 纹理索引 (All, TopItems, Deals, New)
        let section_indices: [(usize, usize); 4] = [(770, 771), (776, 777), (772, 773), (774, 775)];

        for (i, section) in ShopSectionHybrid::ALL.iter().enumerate() {
            let tab_x = pos.x + Self::SECTION_TAB_X + (i as f32 * Self::SECTION_TAB_W);
            let tab_y = pos.y + Self::SECTION_TAB_Y;
            let is_selected = self.current_section == *section;

            let hovered = Rect::new(tab_x, tab_y, Self::SECTION_TAB_W, Self::SECTION_TAB_H).contains(mouse_pos);

            if let Some((normal_idx, selected_idx)) = section_indices.get(i).copied() {
                let indices = if is_selected {
                    [selected_idx, selected_idx, selected_idx]
                } else {
                    [normal_idx, selected_idx, selected_idx]
                };

                let has_texture = LibraryName::Title
                    .get_texture(indices[0])
                    .and_then(|info| info.image)
                    .is_some();

                if has_texture {
                    if draw_library_button_with_offset(
                        LibraryName::Title,
                        indices,
                        vec2(tab_x, tab_y),
                        mouse_pos,
                    ) && !self.dragging {
                        self.current_section = *section;
                        self.refresh_categories_and_items();
                    }
                    continue;
                }
            }

            self.draw_fallback_section_tab(tab_x, tab_y, section, is_selected, hovered);

            if hovered && is_mouse_button_pressed(MouseButton::Left) && !self.dragging {
                self.current_section = *section;
                self.refresh_categories_and_items();
            }
        }
    }
    
    /// 备用分类标签绘制
    pub(super) fn draw_fallback_section_tab(&self, x: f32, y: f32, section: &ShopSectionHybrid, selected: bool, hovered: bool) {
        let bg_color = if selected {
            Color::from_rgba(200, 180, 140, 255)
        } else if hovered {
            Color::from_rgba(100, 100, 140, 255)
        } else {
            Color::from_rgba(60, 60, 80, 255)
        };
        draw_rectangle(x, y, Self::SECTION_TAB_W, Self::SECTION_TAB_H, bg_color);
        draw_rectangle_lines(x, y, Self::SECTION_TAB_W, Self::SECTION_TAB_H, 1.0, 
            Color::from_rgba(150, 150, 170, 255));
        
        let text_color = if selected { BLACK } else { WHITE };
        draw_text_cn(section.name(), x + 15.0, y + 16.0, 12.0, text_color);
    }
    
    /// 绘制职业分类标签 (使用纹理)
    pub(super) fn draw_class_tabs(&mut self, pos: Vec2) {
        let mouse_pos = ui_mouse_pos();

        // Class Tabs 纹理索引 (All, Warrior, Assassin, Taoist, Wizard, Archer)
        let class_indices: [(usize, usize, usize); 6] = [
            (751, 752, 753),
            (754, 755, 756),
            (757, 758, 759),
            (760, 761, 762),
            (763, 764, 765),
            (766, 767, 768),
        ];

        for (i, class) in ShopClassHybrid::ALL.iter().enumerate() {
            let tab_x = pos.x + Self::CLASS_TAB_X + (i as f32 * Self::CLASS_TAB_SIZE);
            let tab_y = pos.y + Self::CLASS_TAB_Y;
            let is_selected = self.current_class == *class;

            let hovered = Rect::new(tab_x, tab_y, Self::CLASS_TAB_SIZE, Self::CLASS_TAB_SIZE - 3.0)
                .contains(mouse_pos);

            if let Some((normal_idx, hover_idx, pressed_idx)) = class_indices.get(i).copied() {
                let indices = if is_selected {
                    [pressed_idx, pressed_idx, pressed_idx]
                } else {
                    [normal_idx, hover_idx, pressed_idx]
                };

                let has_texture = LibraryName::Title
                    .get_texture(indices[0])
                    .and_then(|info| info.image)
                    .is_some();

                if has_texture {
                    if draw_library_button_with_offset(
                        LibraryName::Title,
                        indices,
                        vec2(tab_x, tab_y),
                        mouse_pos,
                    ) && !self.dragging {
                        self.current_class = *class;
                        self.refresh_categories_and_items();
                    }
                    continue;
                }
            }

            self.draw_fallback_class_tab(tab_x, tab_y, class, is_selected, hovered);

            if hovered && is_mouse_button_pressed(MouseButton::Left) && !self.dragging {
                self.current_class = *class;
                self.refresh_categories_and_items();
            }
        }
    }
    
    /// 备用职业标签绘制
    pub(super) fn draw_fallback_class_tab(&self, x: f32, y: f32, class: &ShopClassHybrid, selected: bool, hovered: bool) {
        let bg_color = if selected {
            Color::from_rgba(200, 180, 140, 255)
        } else if hovered {
            Color::from_rgba(100, 100, 140, 255)
        } else {
            Color::from_rgba(60, 60, 80, 255)
        };
        draw_rectangle(x, y, Self::CLASS_TAB_SIZE, Self::CLASS_TAB_SIZE - 3.0, bg_color);
        
        let text_color = if selected { BLACK } else { WHITE };
        draw_text_cn(class.name(), x + 4.0, y + 14.0, 12.0, text_color);
    }
    
    /// 绘制左侧分类列表
    pub(super) fn draw_category_list(&mut self, pos: Vec2) {
        // 绘制分类列表背景 Title[769] + 获取命中区域（用于滚轮滚动）
        let filter_bg_pos = vec2(pos.x + Self::FILTER_BG_X, pos.y + Self::FILTER_BG_Y);

        let filter_bg_hit_rect = draw_library_image_with_offset(
            LibraryName::Title,
            769,
            filter_bg_pos,
            WHITE,
        )
        .unwrap_or_else(|| {
            // 备用背景
            draw_rectangle(filter_bg_pos.x, filter_bg_pos.y, 110.0, 340.0,
                Color::from_rgba(30, 30, 40, 200));
            Rect::new(filter_bg_pos.x, filter_bg_pos.y, 110.0, 340.0)
        });
        
        // 绘制分类项
        let list_x = pos.x + Self::CATEGORY_LIST_X;
        let list_y = pos.y + Self::CATEGORY_LIST_Y;
        let mouse_pos = ui_mouse_pos();

        // 与 C# 原版一致：滚轮在分类背景上滚动时上下滚分类
        if !self.dragging {
            let (_wheel_x, wheel_y) = mouse_wheel();
            if wheel_y.abs() > 0.0 && filter_bg_hit_rect.contains(mouse_pos) {
                let max_scroll = self.categories.len().saturating_sub(Self::CATEGORY_MAX_VISIBLE);
                if max_scroll > 0 {
                    if wheel_y > 0.0 {
                        self.category_scroll = self.category_scroll.saturating_sub(1);
                    } else {
                        self.category_scroll = (self.category_scroll + 1).min(max_scroll);
                    }
                }
            }
        }
        
        for i in 0..Self::CATEGORY_MAX_VISIBLE {
            let idx = self.category_scroll + i;
            if idx >= self.categories.len() { break; }
            
            let item_y = list_y + (i as f32 * Self::CATEGORY_ITEM_STEP);

            // 与 C# 原版 MirLabel Filters 对齐的命中区域
            let item_rect = Rect::new(list_x, item_y, Self::CATEGORY_ITEM_W, Self::CATEGORY_ITEM_H);
            let hovered = item_rect.contains(mouse_pos);
            let selected = self.selected_category == Some(idx);

            // 文字颜色对齐 C#：默认灰、悬停棕、选中金
            let text_color = if selected {
                Color::from_rgba(230, 200, 160, 255)
            } else if hovered {
                Color::from_rgba(160, 140, 110, 255)
            } else {
                Color::from_rgba(128, 128, 128, 255)
            };

            draw_text_cn(&self.categories[idx], list_x, item_y + 13.0, 7.0, text_color);

            if hovered && is_mouse_button_pressed(MouseButton::Left) && !self.dragging {
                self.selected_category = Some(idx);
                self.current_page = 0;
                self.preview_item = None;
                self.quantities = [1; 8];
                self.filter_items();
            }
        }
        
        // 滚动条
        self.draw_category_scrollbar(pos);
    }
    
    /// 绘制分类滚动条 (使用纹理)
    pub(super) fn draw_category_scrollbar(&mut self, pos: Vec2) {
        let scroll_x = pos.x + Self::SCROLL_X;
        let mouse_pos = ui_mouse_pos();
        
        // 上箭头 Prguse2[197-199]
        let up_y = pos.y + Self::SCROLL_UP_Y;

        if draw_library_button_with_offset(
            LibraryName::Prguse2,
            [197, 198, 199],
            vec2(scroll_x, up_y),
            mouse_pos,
        )
            && self.category_scroll > 0 {
                self.category_scroll -= 1;
            }
        
        // 下箭头 Prguse2[207-209]
        let down_y = pos.y + Self::SCROLL_DOWN_Y;

        if draw_library_button_with_offset(
            LibraryName::Prguse2,
            [207, 208, 209],
            vec2(scroll_x, down_y),
            mouse_pos,
        ) {
            let max_scroll = self.categories.len().saturating_sub(Self::CATEGORY_MAX_VISIBLE);
            if self.category_scroll < max_scroll {
                self.category_scroll += 1;
            }
        }
        
        // 滚动块 Prguse2[205-206]
        let thumb_h = LibraryName::Prguse2
            .get_texture(205)
            .map(|info| info.height as f32)
            .unwrap_or(20.0);

        let scrollbar_height = Self::SCROLL_DOWN_Y - Self::SCROLL_UP_Y - Self::SCROLL_BTN_H;
        let scroll_ratio = if self.categories.len() > Self::CATEGORY_MAX_VISIBLE {
            self.category_scroll as f32 / (self.categories.len() - Self::CATEGORY_MAX_VISIBLE) as f32
        } else {
            0.0
        };

        let usable_h = (scrollbar_height - thumb_h).max(0.0);
        let bar_y = pos.y + Self::SCROLL_UP_Y + Self::SCROLL_BTN_H + (scroll_ratio * usable_h);

        let bar_hit_rect = if let Some(info) = LibraryName::Prguse2.get_texture(205) {
            Rect::new(
                scroll_x + info.offset_x as f32,
                bar_y + info.offset_y as f32,
                info.width as f32,
                info.height as f32,
            )
        } else {
            Rect::new(scroll_x, bar_y, Self::SCROLL_BTN_W, thumb_h)
        };

        let bar_hovered = bar_hit_rect.contains(mouse_pos);
        let bar_idx = if bar_hovered { 206 } else { 205 };

        if draw_library_image_with_offset(LibraryName::Prguse2, bar_idx, vec2(scroll_x, bar_y), WHITE).is_none() {
            draw_rectangle(scroll_x, bar_y, Self::SCROLL_BTN_W, thumb_h,
                Color::from_rgba(100, 100, 120, 255));
        }
    }
    
    /// 绘制商品网格
    pub(super) fn draw_item_grid(&mut self, pos: Vec2) {
        // 每帧重置悬停状态
        self.hover_item = None;
        
        let start_idx = self.current_page * self.items_per_page;
        
        for i in 0..self.items_per_page {
            let item_idx = start_idx + i;
            
            // 计算网格位置
            let grid_x = if i < 4 {
                pos.x + Self::GRID_START_X + (i as f32 * Self::CELL_SPACING)
            } else {
                pos.x + Self::GRID_START_X + ((i - 4) as f32 * Self::CELL_SPACING)
            };
            let grid_y = if i < 4 {
                pos.y + Self::GRID_ROW1_Y
            } else {
                pos.y + Self::GRID_ROW2_Y
            };
            
            // 绘制商品格子
            if item_idx < self.filtered_items.len() {
                self.draw_item_cell(grid_x, grid_y, item_idx);
            } else {
                // 空格子
                self.draw_empty_cell(grid_x, grid_y);
            }
        }
    }
    
    /// 绘制商品格子 (基于原版位置)
    pub(super) fn draw_item_cell(&mut self, x: f32, y: f32, item_idx: usize) {
        let item = &self.filtered_items[item_idx];
        
        // 格子背景 Title[750]
        if let Some(ref tex) = self.cell_texture {
            draw_texture_ex(tex, x, y, WHITE, DrawTextureParams::default());
        } else {
            draw_rectangle(x, y, Self::CELL_WIDTH, Self::CELL_HEIGHT,
                Color::from_rgba(50, 50, 60, 255));
            draw_rectangle_lines(x, y, Self::CELL_WIDTH, Self::CELL_HEIGHT,
                1.0, Color::from_rgba(100, 100, 120, 255));
        }
        
        // 物品名称（对齐 C# GameShopCell.nameLabel: Size(125,15), Location(0,13), Font 8F, HorizontalCenter）
        let name_color = if item.in_stock {
            Color::from_rgba(255, 215, 0, 255)
        } else {
            GRAY
        };
        let name_font_size = 8.0;
        let name_top_y = y + 13.0;
        let name_baseline_y = name_top_y + name_font_size + 2.0;
        crate::ui::text_renderer::draw_text_centered(
            &item.name,
            x + Self::CELL_WIDTH / 2.0,
            name_baseline_y,
            name_font_size,
            name_color,
        );
        
        // 物品图标 (原版位置: 12, 40, 尺寸32x32)
        let icon_x = x + 12.0;
        let icon_y = y + 40.0;
        let icon_w = 32.0;
        let icon_h = 32.0;
        
        // 检测图标区域悬停（用于显示物品提示框）
        let mouse_pos = ui_mouse_pos();
        let icon_hovered = mouse_pos.x >= icon_x && mouse_pos.x <= icon_x + icon_w
            && mouse_pos.y >= icon_y && mouse_pos.y <= icon_y + icon_h;
        if icon_hovered {
            self.hover_item = Some(item_idx);
        }
        
        if let Some(info) = LibraryName::Items.get_texture(item.icon_index) {
            if let Some(ref tex) = info.image {
                // 居中绘制
                let tex_w = info.width as f32;
                let tex_h = info.height as f32;
                let offset_x = (32.0 - tex_w.min(32.0)) / 2.0;
                let offset_y = (32.0 - tex_h.min(32.0)) / 2.0;
                draw_texture_ex(tex, icon_x + offset_x, icon_y + offset_y, WHITE, DrawTextureParams {
                    dest_size: Some(vec2(tex_w.min(32.0), tex_h.min(32.0))),
                    ..Default::default()
                });
            }
        } else {
            draw_rectangle(icon_x, icon_y, 32.0, 32.0, Color::from_rgba(60, 60, 70, 255));
        }
        
        // STOCK标签 (原版位置: 53, 37)
        draw_text_cn("STOCK:", x + 53.0, y + 45.0, 7.0, GRAY);
        
        // 库存数量 (原版位置: 93, 37)
        let stock_text = if item.stock >= 99 {
            "99+".to_string()
        } else if item.stock == 0 {
            "∞".to_string()
        } else {
            item.stock.to_string()
        };
        draw_text_cn(&stock_text, x + 93.0, y + 45.0, 7.0, WHITE);
        
        // 购买数量选择按钮 (原版位置: quantityDown=55,56 quantityUp=97,56 quantity=74,56)
        // 计算当前格子在页面中的索引 (0-7)
        let start_idx = self.current_page * self.items_per_page;
        let grid_idx = item_idx - start_idx;
        
        if grid_idx < 8 {
            let qty = self.quantities[grid_idx];
            
            // 减少按钮 Prguse2[240-242] (原版位置: 55, 56)
            let down_x = x + 55.0;
            let down_y = y + 56.0;

            if draw_library_button_with_offset(
                LibraryName::Prguse2,
                [240, 241, 242],
                vec2(down_x, down_y),
                mouse_pos,
            ) && !self.dragging {
                if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
                    self.quantities[grid_idx] = qty.saturating_sub(10).max(1);
                } else {
                    self.quantities[grid_idx] = qty.saturating_sub(1).max(1);
                }
            }
            
            // 数量显示 (原版位置: 74, 56, 尺寸20x13)
            let qty_x = x + 74.0;
            let qty_y = y + 56.0;
            draw_text_cn(&qty.to_string(), qty_x + 4.0, qty_y + 10.0, 8.0, WHITE);
            
            // 增加按钮 Prguse2[243-245] (原版位置: 97, 56)
            let up_x = x + 97.0;
            let up_y = y + 56.0;

            if draw_library_button_with_offset(
                LibraryName::Prguse2,
                [243, 244, 245],
                vec2(up_x, up_y),
                mouse_pos,
            ) && !self.dragging {
                let max_qty = if item.stock > 0 && item.stock < 99 {
                    item.stock as u8
                } else {
                    99
                };
                if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
                    self.quantities[grid_idx] = (qty + 10).min(max_qty);
                } else {
                    self.quantities[grid_idx] = (qty + 1).min(max_qty);
                }
            }
        }
        
        // 物品数量 (原版位置: 16, 60)
        if item.count > 1 {
            draw_text_cn(&format!("x{}", item.count), x + 16.0, y + 68.0, 7.0, WHITE);
        }
        
        // 元宝价格 (原版位置: 2, 81 右对齐)
        if item.price_ingot > 0 {
            draw_text_cn(&format!("{}", item.price_ingot), x + 75.0, y + 89.0, 8.0,
                Color::from_rgba(0, 255, 255, 255));
        }
        
        // 金币价格 (原版位置: 2, 102 右对齐)
        if item.price_gold > 0 {
            draw_text_cn(&format!("{}", item.price_gold), x + 75.0, y + 110.0, 8.0,
                Color::from_rgba(255, 215, 0, 255));
        }
        
        // 热销/新品标记
        if item.hot {
            draw_text_cn("🔥", x + Self::CELL_WIDTH - 18.0, y + 12.0, 12.0, RED);
        }
        if item.new {
            draw_text_cn("NEW", x + 5.0, y + 12.0, 7.0, GREEN);
        }
        
        let is_previewable = matches!(item.category, ShopCategoryHybrid::Weapon | ShopCategoryHybrid::Armor);
        
        // Preview按钮 Title[781-783] (原版位置: 8, 122)
        if is_previewable {
            let preview_x = x + 8.0;
            let preview_y = y + 122.0;

            if draw_library_button_with_offset(
                LibraryName::Title,
                [781, 782, 783],
                vec2(preview_x, preview_y),
                mouse_pos,
            ) && !self.dragging {
                self.preview_item = Some(item_idx);
            }
        }
        
        // Buy按钮 Title[778-780] (原版位置: 42/75, 122)
        let buy_x = if is_previewable { x + 75.0 } else { x + 42.0 };
        let buy_y = y + 122.0;

        if draw_library_button_with_offset(
            LibraryName::Title,
            [778, 779, 780],
            vec2(buy_x, buy_y),
            mouse_pos,
        ) && !self.dragging {
            // TODO: handle buy action
        }
    }
    
    /// 绘制空格子 (原版也使用 Title[750] 纹理)
    pub(super) fn draw_empty_cell(&self, x: f32, y: f32) {
        // 原版中空格子也会绘制背景纹理 Title[750]
        if let Some(ref tex) = self.cell_texture {
            draw_texture_ex(tex, x, y, WHITE, DrawTextureParams::default());
        } else {
            // 备用绘制
            draw_rectangle(x, y, Self::CELL_WIDTH, Self::CELL_HEIGHT,
                Color::from_rgba(40, 40, 50, 150));
            draw_rectangle_lines(x, y, Self::CELL_WIDTH, Self::CELL_HEIGHT,
                1.0, Color::from_rgba(60, 60, 70, 255));
        }
    }
    
    /// 绘制分页控制 (原版位置: PageLabel=597,446(83x17) PreviousButton=600,448 NextButton=660,448)
    pub(super) fn draw_pagination(&mut self, pos: Vec2) {
        let total_pages = if self.filtered_items.is_empty() {
            1
        } else {
            self.filtered_items.len().div_ceil(self.items_per_page)
        };
        
        let mouse_pos = ui_mouse_pos();
        
        // 上一页按钮 Prguse2[240-242] (原版: 600, 448)
        let prev_x = pos.x + 600.0;
        let prev_y = pos.y + 448.0;

        if draw_library_button_with_offset(
            LibraryName::Prguse2,
            [240, 241, 242],
            vec2(prev_x, prev_y),
            mouse_pos,
        ) && self.current_page > 0 {
            self.current_page -= 1;
            self.preview_item = None;
            self.quantities = [1; 8];  // 重置购买数量
        }
        
        // 下一页按钮 Prguse2[243-245] (原版: 660, 448)
        let next_x = pos.x + 660.0;
        let next_y = pos.y + 448.0;

        if draw_library_button_with_offset(
            LibraryName::Prguse2,
            [243, 244, 245],
            vec2(next_x, next_y),
            mouse_pos,
        ) && self.current_page < total_pages - 1 {
            self.current_page += 1;
            self.preview_item = None;
            self.quantities = [1; 8];  // 重置购买数量
        }
        
        // 页码显示 (原版: 597, 446, 尺寸83x17, 居中对齐)
        // 页码在按钮上方显示，居中在83px宽度内
        let page_label_x = pos.x + 597.0;
        let page_label_y = pos.y + 446.0;
        let page_text = format!("{} / {}", self.current_page + 1, total_pages);
        // 83px宽度内居中 (597 + 83/2 = 638.5)
        draw_text_cn(&page_text, page_label_x + 30.0, page_label_y + 11.0, 9.0, WHITE);
    }
    
    /// 绘制货币信息 (原版位置: totalCredits=5,449 totalGold=123,449)
    pub(super) fn draw_currency_info(&self, pos: Vec2) {
        // 元宝显示 (原版位置: 5, 449, 右对齐100宽)
        let credits_x = pos.x + 5.0;
        let credits_y = pos.y + 449.0;
        draw_text_cn(&format!("{}", self.player_ingot), credits_x + 60.0, credits_y + 12.0,
            10.0, Color::from_rgba(0, 255, 255, 255));
        
        // 金币显示 (原版位置: 123, 449, 右对齐100宽)
        let gold_x = pos.x + 123.0;
        let gold_y = pos.y + 449.0;
        draw_text_cn(&format!("{}", self.player_gold), gold_x + 60.0, gold_y + 12.0, 
            10.0, Color::from_rgba(255, 215, 0, 255));
    }
    
    /// 绘制支付方式选择 (原版位置: PaymentTypeGold=250,449 PaymentTypeCredit=340,449)
    pub(super) fn draw_payment_options(&mut self, pos: Vec2) {
        let mouse_pos = ui_mouse_pos();
        
        // Buy with Gold 复选框 (原版位置: 250, 449)
        let gold_x = pos.x + 250.0;
        let gold_y = pos.y + 449.0;
        let checkbox_size = 14.0;

        let gold_base_info = LibraryName::Prguse.get_texture(2086);
        let (gold_off_x, gold_off_y, gold_h) = match gold_base_info.as_ref() {
            Some(info) => (info.offset_x as f32, info.offset_y as f32, info.height as f32),
            None => (0.0, 0.0, checkbox_size),
        };

        let gold_hovered = mouse_pos.x >= gold_x + gold_off_x && mouse_pos.x <= gold_x + gold_off_x + 120.0
            && mouse_pos.y >= gold_y + gold_off_y && mouse_pos.y <= gold_y + gold_off_y + gold_h.max(checkbox_size);
        
        // 绘制复选框
        if draw_library_image_with_offset(
            LibraryName::Prguse,
            if self.pay_with_gold { 2087 } else { 2086 },
            vec2(gold_x, gold_y),
            WHITE,
        ).is_none() {
            draw_rectangle(gold_x, gold_y, checkbox_size, checkbox_size, 
                Color::from_rgba(60, 60, 80, 255));
            draw_rectangle_lines(gold_x, gold_y, checkbox_size, checkbox_size,
                1.0, Color::from_rgba(150, 150, 170, 255));
            if self.pay_with_gold {
                draw_text_cn("✓", gold_x + 2.0, gold_y + 11.0, 10.0, GREEN);
            }
        }
        draw_text_cn("Buy with Gold", gold_x + 18.0, gold_y + 11.0, 9.0, 
            if gold_hovered { WHITE } else { GRAY });
        
        if gold_hovered && is_mouse_button_pressed(MouseButton::Left) && !self.dragging {
            self.pay_with_gold = true;
        }
        
        // Buy with Credits 复选框 (原版位置: 340, 449)
        let credit_x = pos.x + 340.0;
        let credit_y = pos.y + 449.0;

        let credit_base_info = LibraryName::Prguse.get_texture(2086);
        let (credit_off_x, credit_off_y, credit_h) = match credit_base_info.as_ref() {
            Some(info) => (info.offset_x as f32, info.offset_y as f32, info.height as f32),
            None => (0.0, 0.0, checkbox_size),
        };

        let credit_hovered = mouse_pos.x >= credit_x + credit_off_x && mouse_pos.x <= credit_x + credit_off_x + 130.0
            && mouse_pos.y >= credit_y + credit_off_y && mouse_pos.y <= credit_y + credit_off_y + credit_h.max(checkbox_size);

        if draw_library_image_with_offset(
            LibraryName::Prguse,
            if !self.pay_with_gold { 2087 } else { 2086 },
            vec2(credit_x, credit_y),
            WHITE,
        ).is_none() {
            draw_rectangle(credit_x, credit_y, checkbox_size, checkbox_size, 
                Color::from_rgba(60, 60, 80, 255));
            draw_rectangle_lines(credit_x, credit_y, checkbox_size, checkbox_size,
                1.0, Color::from_rgba(150, 150, 170, 255));
            if !self.pay_with_gold {
                draw_text_cn("✓", credit_x + 2.0, credit_y + 11.0, 10.0, GREEN);
            }
        }
        draw_text_cn("Buy with Credits", credit_x + 18.0, credit_y + 11.0, 9.0,
            if credit_hovered { WHITE } else { GRAY });
        
        if credit_hovered && is_mouse_button_pressed(MouseButton::Left) && !self.dragging {
            self.pay_with_gold = false;
        }
    }
}
