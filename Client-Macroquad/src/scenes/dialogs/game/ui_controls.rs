// UI Controls - 基础 UI 控件
// C# reference: Client/MirControls/

use macroquad::prelude::*;
use super::native_ui_utils::*;

/// Approximate width of a single character in the default font at size ~13
const CHAR_WIDTH_ESTIMATE: f32 = 7.0;

// ============================================================
// CheckBox - 复选框
// C# reference: MirCheckBox
// ============================================================

pub struct CheckBoxHybrid {
    pub checked: bool,
    pub label: String,
    pub position: Vec2,
    pub enabled: bool,
    pub visible: bool,
    size: f32,
}

impl CheckBoxHybrid {
    pub fn new(label: &str, position: Vec2) -> Self {
        Self {
            checked: false,
            label: label.to_string(),
            position,
            enabled: true,
            visible: true,
            size: 16.0,
        }
    }

    pub fn toggle(&mut self) {
        if self.enabled {
            self.checked = !self.checked;
        }
    }

    /// Draw and return true if clicked
    pub fn draw(&mut self) -> bool {
        if !self.visible {
            return false;
        }

        let pos = self.position;
        let box_rect = Rect::new(pos.x, pos.y, self.size, self.size);
        let mouse = mouse_position();
        let mouse_pos = vec2(mouse.0, mouse.1);

        // Box background
        let bg_color = if self.enabled {
            Color::new(0.15, 0.15, 0.2, 1.0)
        } else {
            Color::new(0.1, 0.1, 0.1, 0.5)
        };
        draw_rectangle(pos.x, pos.y, self.size, self.size, bg_color);
        draw_rectangle_lines(pos.x, pos.y, self.size, self.size, 1.0, GRAY);

        // Check mark
        if self.checked {
            draw_text("✓", pos.x + 2.0, pos.y + 13.0, 14.0, GREEN);
        }

        // Hover highlight
        if self.enabled && box_rect.contains(mouse_pos) {
            draw_rectangle_lines(pos.x, pos.y, self.size, self.size, 1.0, WHITE);
        }

        // Label
        let label_color = if self.enabled { WHITE } else { GRAY };
        draw_text(&self.label, pos.x + self.size + 5.0, pos.y + 13.0, 13.0, label_color);

        // Click detection
        let full_rect = Rect::new(pos.x, pos.y, self.size + 5.0 + (self.label.len() as f32 * CHAR_WIDTH_ESTIMATE), self.size);
        if self.enabled && is_mouse_button_pressed(MouseButton::Left) && full_rect.contains(mouse_pos) {
            self.checked = !self.checked;
            return true;
        }

        false
    }
}

// ============================================================
// TextBox - 文本输入框
// C# reference: MirTextBox
// ============================================================

pub struct TextBoxHybrid {
    pub text: String,
    pub placeholder: String,
    pub position: Vec2,
    pub width: f32,
    pub height: f32,
    pub max_length: usize,
    pub focused: bool,
    pub enabled: bool,
    pub visible: bool,
    pub password_mode: bool,
    cursor_pos: usize,
    cursor_blink: f32,
}

impl TextBoxHybrid {
    pub fn new(position: Vec2, width: f32) -> Self {
        Self {
            text: String::new(),
            placeholder: String::new(),
            position,
            width,
            height: 22.0,
            max_length: 100,
            focused: false,
            enabled: true,
            visible: true,
            password_mode: false,
            cursor_pos: 0,
            cursor_blink: 0.0,
        }
    }

    pub fn with_placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = placeholder.to_string();
        self
    }

    pub fn with_max_length(mut self, max: usize) -> Self {
        self.max_length = max;
        self
    }

    pub fn with_password(mut self) -> Self {
        self.password_mode = true;
        self
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor_pos = 0;
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.chars().take(self.max_length).collect();
        self.cursor_pos = self.text.len();
    }

    /// Handle character input. Returns true if text changed.
    pub fn handle_char(&mut self, c: char) -> bool {
        if !self.focused || !self.enabled {
            return false;
        }
        if c.is_control() {
            return false;
        }
        if self.text.len() >= self.max_length {
            return false;
        }
        self.text.insert(self.cursor_pos, c);
        self.cursor_pos += 1;
        true
    }

    /// Handle special keys. Returns true if text changed.
    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        if !self.focused || !self.enabled {
            return false;
        }
        match key {
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.text.remove(self.cursor_pos);
                    return true;
                }
            }
            KeyCode::Delete => {
                if self.cursor_pos < self.text.len() {
                    self.text.remove(self.cursor_pos);
                    return true;
                }
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_pos < self.text.len() {
                    self.cursor_pos += 1;
                }
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
            }
            KeyCode::End => {
                self.cursor_pos = self.text.len();
            }
            _ => {}
        }
        false
    }

    /// Draw and return true if clicked (gained focus)
    pub fn draw(&mut self, dt: f32) -> bool {
        if !self.visible {
            return false;
        }

        let pos = self.position;
        let rect = Rect::new(pos.x, pos.y, self.width, self.height);
        let mouse = mouse_position();
        let mouse_pos = vec2(mouse.0, mouse.1);

        // Background
        let bg_color = if self.focused {
            Color::new(0.2, 0.2, 0.25, 1.0)
        } else {
            Color::new(0.12, 0.12, 0.15, 1.0)
        };
        draw_rectangle(pos.x, pos.y, self.width, self.height, bg_color);

        let border_color = if self.focused { WHITE } else { GRAY };
        draw_rectangle_lines(pos.x, pos.y, self.width, self.height, 1.0, border_color);

        // Text or placeholder
        let display_text = if self.password_mode {
            "●".repeat(self.text.len())
        } else {
            self.text.clone()
        };

        if self.text.is_empty() && !self.focused {
            draw_text(&self.placeholder, pos.x + 4.0, pos.y + 16.0, 13.0, DARKGRAY);
        } else {
            draw_text(&display_text, pos.x + 4.0, pos.y + 16.0, 13.0, WHITE);
        }

        // Cursor
        if self.focused {
            self.cursor_blink += dt;
            if self.cursor_blink % 1.0 < 0.5 {
                let cursor_x = pos.x + 4.0 + (self.cursor_pos as f32) * CHAR_WIDTH_ESTIMATE;
                draw_line(cursor_x, pos.y + 3.0, cursor_x, pos.y + self.height - 3.0, 1.0, WHITE);
            }
        }

        // Click to focus
        let clicked = is_mouse_button_pressed(MouseButton::Left);
        if clicked {
            let was_focused = self.focused;
            self.focused = self.enabled && rect.contains(mouse_pos);
            if self.focused {
                self.cursor_blink = 0.0;
            }
            return self.focused && !was_focused;
        }

        false
    }
}

// ============================================================
// DropDownBox - 下拉选择框
// C# reference: MirDropDownBox
// ============================================================

pub struct DropDownBoxHybrid {
    pub options: Vec<String>,
    pub selected_index: usize,
    pub position: Vec2,
    pub width: f32,
    pub enabled: bool,
    pub visible: bool,
    expanded: bool,
    item_height: f32,
}

impl DropDownBoxHybrid {
    pub fn new(options: Vec<String>, position: Vec2, width: f32) -> Self {
        Self {
            options,
            selected_index: 0,
            position,
            width,
            enabled: true,
            visible: true,
            expanded: false,
            item_height: 20.0,
        }
    }

    pub fn selected_text(&self) -> &str {
        self.options.get(self.selected_index).map(|s| s.as_str()).unwrap_or("")
    }

    pub fn set_options(&mut self, options: Vec<String>) {
        self.options = options;
        if self.selected_index >= self.options.len() {
            self.selected_index = 0;
        }
    }

    /// Draw and return Some(index) if selection changed
    pub fn draw(&mut self) -> Option<usize> {
        if !self.visible {
            return None;
        }

        let pos = self.position;
        let mouse = mouse_position();
        let mouse_pos = vec2(mouse.0, mouse.1);
        let mut result = None;

        // Main box
        let main_rect = Rect::new(pos.x, pos.y, self.width, self.item_height);
        let bg_color = if self.enabled {
            Color::new(0.15, 0.15, 0.2, 1.0)
        } else {
            Color::new(0.1, 0.1, 0.1, 0.5)
        };
        draw_rectangle(pos.x, pos.y, self.width, self.item_height, bg_color);
        draw_rectangle_lines(pos.x, pos.y, self.width, self.item_height, 1.0, GRAY);

        // Selected text
        let text_color = if self.enabled { WHITE } else { GRAY };
        draw_text(self.selected_text(), pos.x + 5.0, pos.y + 15.0, 13.0, text_color);

        // Arrow
        let arrow = if self.expanded { "▲" } else { "▼" };
        draw_text(arrow, pos.x + self.width - 18.0, pos.y + 15.0, 12.0, text_color);

        // Click to expand/collapse
        if self.enabled && is_mouse_button_pressed(MouseButton::Left) && main_rect.contains(mouse_pos) {
            self.expanded = !self.expanded;
        }

        // Dropdown list
        if self.expanded {
            let list_height = self.options.len() as f32 * self.item_height;
            let list_y = pos.y + self.item_height;

            // Background
            draw_rectangle(pos.x, list_y, self.width, list_height, Color::new(0.12, 0.12, 0.18, 0.98));
            draw_rectangle_lines(pos.x, list_y, self.width, list_height, 1.0, GRAY);

            for (i, option) in self.options.iter().enumerate() {
                let item_y = list_y + (i as f32) * self.item_height;
                let item_rect = Rect::new(pos.x, item_y, self.width, self.item_height);

                // Hover highlight
                if item_rect.contains(mouse_pos) {
                    draw_rectangle(pos.x, item_y, self.width, self.item_height,
                        Color::new(0.3, 0.3, 0.5, 0.5));
                }

                // Selection marker
                let color = if i == self.selected_index { YELLOW } else { WHITE };
                draw_text(option, pos.x + 5.0, item_y + 15.0, 13.0, color);

                if is_mouse_button_pressed(MouseButton::Left) && item_rect.contains(mouse_pos) {
                    self.selected_index = i;
                    self.expanded = false;
                    result = Some(i);
                }
            }

            // Close dropdown if clicking outside
            if is_mouse_button_pressed(MouseButton::Left) && !main_rect.contains(mouse_pos) {
                let full_rect = Rect::new(pos.x, list_y, self.width, list_height);
                if !full_rect.contains(mouse_pos) {
                    self.expanded = false;
                }
            }
        }

        result
    }
}

// ============================================================
// ScrollingLabel - 滚动文本
// C# reference: MirScrollingLabel
// ============================================================

pub struct ScrollingLabelHybrid {
    pub text: String,
    pub position: Vec2,
    pub width: f32,
    pub height: f32,
    pub visible: bool,
    pub auto_scroll: bool,
    scroll_offset: f32,
    line_height: f32,
}

impl ScrollingLabelHybrid {
    pub fn new(position: Vec2, width: f32, height: f32) -> Self {
        Self {
            text: String::new(),
            position,
            width,
            height,
            visible: true,
            auto_scroll: false,
            scroll_offset: 0.0,
            line_height: 15.0,
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    pub fn append(&mut self, text: &str) {
        self.text.push_str(text);
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        let lines = self.text.lines().count() as f32;
        let visible_lines = self.height / self.line_height;
        self.scroll_offset = (lines - visible_lines).max(0.0) * self.line_height;
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0.0;
    }

    fn total_content_height(&self) -> f32 {
        self.text.lines().count() as f32 * self.line_height
    }

    pub fn draw(&mut self) {
        if !self.visible {
            return;
        }

        let pos = self.position;
        let mouse = mouse_position();
        let mouse_pos = vec2(mouse.0, mouse.1);
        let rect = Rect::new(pos.x, pos.y, self.width, self.height);

        // Background
        draw_rectangle(pos.x, pos.y, self.width, self.height, Color::new(0.05, 0.05, 0.1, 0.8));

        // Text lines
        let lines: Vec<&str> = self.text.lines().collect();
        let start_line = (self.scroll_offset / self.line_height) as usize;
        let visible_count = (self.height / self.line_height) as usize + 1;

        for (i, line) in lines.iter().enumerate().skip(start_line).take(visible_count) {
            let y = pos.y + ((i - start_line) as f32) * self.line_height
                + self.line_height
                - (self.scroll_offset % self.line_height);
            if y > pos.y && y < pos.y + self.height {
                draw_text(line, pos.x + 5.0, y, 12.0, LIGHTGRAY);
            }
        }

        // Scrollbar (if content overflows)
        let content_h = self.total_content_height();
        if content_h > self.height {
            let bar_height = (self.height / content_h * self.height).max(20.0);
            let bar_y = pos.y + (self.scroll_offset / content_h * self.height);
            draw_rectangle(pos.x + self.width - 6.0, bar_y, 4.0, bar_height,
                Color::new(0.5, 0.5, 0.5, 0.5));
        }

        // Mouse wheel scrolling
        let (_wx, wy) = mouse_wheel();
        if rect.contains(mouse_pos) && wy.abs() > 0.0 {
            self.scroll_offset = (self.scroll_offset - wy * 20.0)
                .max(0.0)
                .min((content_h - self.height).max(0.0));
        }
    }
}

// ============================================================
// GoodsCell - 商品格子
// C# reference: MirGoodsCell
// ============================================================

/// Shop item data
#[derive(Debug, Clone)]
pub struct ShopGoodsItem {
    pub item_id: u32,
    pub name: String,
    pub icon_index: i32,
    pub price: u64,
    pub count: u32,
    pub durability: u16,
    pub max_durability: u16,
    pub description: String,
}

pub struct GoodsCellHybrid {
    pub item: Option<ShopGoodsItem>,
    pub position: Vec2,
    pub selected: bool,
    pub visible: bool,
    width: f32,
    height: f32,
}

const GOODS_CELL_WIDTH: f32 = 200.0;
const GOODS_CELL_HEIGHT: f32 = 36.0;

impl GoodsCellHybrid {
    pub fn new(position: Vec2) -> Self {
        Self {
            item: None,
            position,
            selected: false,
            visible: true,
            width: GOODS_CELL_WIDTH,
            height: GOODS_CELL_HEIGHT,
        }
    }

    pub fn set_item(&mut self, item: ShopGoodsItem) {
        self.item = Some(item);
    }

    pub fn clear(&mut self) {
        self.item = None;
        self.selected = false;
    }

    /// Draw and return true if clicked
    pub fn draw(&mut self) -> bool {
        if !self.visible {
            return false;
        }

        let pos = self.position;
        let rect = Rect::new(pos.x, pos.y, self.width, self.height);
        let mouse = mouse_position();
        let mouse_pos = vec2(mouse.0, mouse.1);

        // Background
        let bg_color = if self.selected {
            Color::new(0.2, 0.25, 0.35, 1.0)
        } else if rect.contains(mouse_pos) {
            Color::new(0.18, 0.18, 0.25, 1.0)
        } else {
            Color::new(0.12, 0.12, 0.18, 1.0)
        };
        draw_rectangle(pos.x, pos.y, self.width, self.height, bg_color);
        draw_rectangle_lines(pos.x, pos.y, self.width, self.height, 1.0, GRAY);

        if let Some(item) = &self.item {
            // Icon placeholder
            draw_rectangle(pos.x + 2.0, pos.y + 2.0, 32.0, 32.0, Color::new(0.2, 0.2, 0.3, 1.0));
            draw_rectangle_lines(pos.x + 2.0, pos.y + 2.0, 32.0, 32.0, 1.0, DARKGRAY);

            // Name
            draw_text(&item.name, pos.x + 38.0, pos.y + 14.0, 13.0, WHITE);

            // Price
            let price_text = format!("{}金", item.price);
            draw_text(&price_text, pos.x + 38.0, pos.y + 30.0, 11.0, Color::new(1.0, 0.84, 0.0, 1.0));

            // Count (if stackable)
            if item.count > 1 {
                draw_text(&format!("x{}", item.count), pos.x + 140.0, pos.y + 30.0, 11.0, YELLOW);
            }

            // Tooltip on hover
            if rect.contains(mouse_pos) && !item.description.is_empty() {
                let tip_x = mouse.0 + 15.0;
                let tip_y = mouse.1;
                let tip_w = 180.0;
                let lines: Vec<&str> = item.description.lines().collect();
                let tip_h = 20.0 + (lines.len() as f32) * 14.0;

                draw_rectangle(tip_x, tip_y, tip_w, tip_h, Color::new(0.05, 0.05, 0.1, 0.95));
                draw_rectangle_lines(tip_x, tip_y, tip_w, tip_h, 1.0, GRAY);
                draw_text(&item.name, tip_x + 5.0, tip_y + 14.0, 13.0, YELLOW);
                for (i, line) in lines.iter().enumerate() {
                    draw_text(line, tip_x + 5.0, tip_y + 30.0 + (i as f32) * 14.0, 11.0, LIGHTGRAY);
                }
            }
        }

        // Click detection
        if is_mouse_button_pressed(MouseButton::Left) && rect.contains(mouse_pos) {
            self.selected = true;
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // CheckBox tests
    #[test]
    fn test_checkbox_new() {
        let cb = CheckBoxHybrid::new("Test", vec2(10.0, 20.0));
        assert!(!cb.checked);
        assert!(cb.enabled);
        assert!(cb.visible);
        assert_eq!(cb.label, "Test");
    }

    #[test]
    fn test_checkbox_toggle() {
        let mut cb = CheckBoxHybrid::new("Test", vec2(0.0, 0.0));
        assert!(!cb.checked);
        cb.toggle();
        assert!(cb.checked);
        cb.toggle();
        assert!(!cb.checked);
    }

    #[test]
    fn test_checkbox_disabled() {
        let mut cb = CheckBoxHybrid::new("Test", vec2(0.0, 0.0));
        cb.enabled = false;
        cb.toggle();
        assert!(!cb.checked); // Should not toggle when disabled
    }

    // TextBox tests
    #[test]
    fn test_textbox_new() {
        let tb = TextBoxHybrid::new(vec2(0.0, 0.0), 200.0);
        assert!(tb.text.is_empty());
        assert!(!tb.focused);
        assert!(tb.enabled);
        assert_eq!(tb.max_length, 100);
    }

    #[test]
    fn test_textbox_input() {
        let mut tb = TextBoxHybrid::new(vec2(0.0, 0.0), 200.0);
        tb.focused = true;
        assert!(tb.handle_char('H'));
        assert!(tb.handle_char('i'));
        assert_eq!(tb.text, "Hi");
        assert_eq!(tb.cursor_pos, 2);
    }

    #[test]
    fn test_textbox_max_length() {
        let mut tb = TextBoxHybrid::new(vec2(0.0, 0.0), 200.0).with_max_length(3);
        tb.focused = true;
        tb.handle_char('a');
        tb.handle_char('b');
        tb.handle_char('c');
        assert!(!tb.handle_char('d')); // Should fail
        assert_eq!(tb.text, "abc");
    }

    #[test]
    fn test_textbox_backspace() {
        let mut tb = TextBoxHybrid::new(vec2(0.0, 0.0), 200.0);
        tb.focused = true;
        tb.handle_char('a');
        tb.handle_char('b');
        tb.handle_key(KeyCode::Backspace);
        assert_eq!(tb.text, "a");
        assert_eq!(tb.cursor_pos, 1);
    }

    #[test]
    fn test_textbox_set_clear() {
        let mut tb = TextBoxHybrid::new(vec2(0.0, 0.0), 200.0);
        tb.set_text("Hello");
        assert_eq!(tb.text, "Hello");
        assert_eq!(tb.cursor_pos, 5);
        tb.clear();
        assert!(tb.text.is_empty());
        assert_eq!(tb.cursor_pos, 0);
    }

    // DropDown tests
    #[test]
    fn test_dropdown_new() {
        let dd = DropDownBoxHybrid::new(
            vec!["Option A".into(), "Option B".into()],
            vec2(0.0, 0.0), 150.0
        );
        assert_eq!(dd.selected_index, 0);
        assert_eq!(dd.selected_text(), "Option A");
        assert!(!dd.expanded);
    }

    #[test]
    fn test_dropdown_selection() {
        let mut dd = DropDownBoxHybrid::new(
            vec!["A".into(), "B".into(), "C".into()],
            vec2(0.0, 0.0), 150.0
        );
        dd.selected_index = 2;
        assert_eq!(dd.selected_text(), "C");
    }

    #[test]
    fn test_dropdown_set_options() {
        let mut dd = DropDownBoxHybrid::new(
            vec!["Old".into()],
            vec2(0.0, 0.0), 150.0
        );
        dd.selected_index = 0;
        dd.set_options(vec!["New A".into(), "New B".into()]);
        assert_eq!(dd.selected_index, 0); // Reset to valid index
        assert_eq!(dd.selected_text(), "New A");
    }

    // ScrollingLabel tests
    #[test]
    fn test_scrolling_label_new() {
        let sl = ScrollingLabelHybrid::new(vec2(0.0, 0.0), 200.0, 100.0);
        assert!(sl.text.is_empty());
        assert!(sl.visible);
        assert!(!sl.auto_scroll);
    }

    #[test]
    fn test_scrolling_label_set_text() {
        let mut sl = ScrollingLabelHybrid::new(vec2(0.0, 0.0), 200.0, 100.0);
        sl.set_text("Line 1\nLine 2\nLine 3");
        assert_eq!(sl.text.lines().count(), 3);
    }

    #[test]
    fn test_scrolling_label_append() {
        let mut sl = ScrollingLabelHybrid::new(vec2(0.0, 0.0), 200.0, 100.0);
        sl.set_text("Hello");
        sl.append(" World");
        assert_eq!(sl.text, "Hello World");
    }

    // GoodsCell tests
    #[test]
    fn test_goods_cell_new() {
        let gc = GoodsCellHybrid::new(vec2(10.0, 20.0));
        assert!(gc.item.is_none());
        assert!(!gc.selected);
        assert!(gc.visible);
    }

    #[test]
    fn test_goods_cell_set_item() {
        let mut gc = GoodsCellHybrid::new(vec2(0.0, 0.0));
        gc.set_item(ShopGoodsItem {
            item_id: 1,
            name: "Sword".to_string(),
            icon_index: 100,
            price: 500,
            count: 1,
            durability: 10,
            max_durability: 10,
            description: "A sharp sword".to_string(),
        });
        assert!(gc.item.is_some());
        assert_eq!(gc.item.as_ref().unwrap().name, "Sword");
    }

    #[test]
    fn test_goods_cell_clear() {
        let mut gc = GoodsCellHybrid::new(vec2(0.0, 0.0));
        gc.set_item(ShopGoodsItem {
            item_id: 1, name: "Test".into(), icon_index: 0,
            price: 100, count: 1, durability: 5, max_durability: 10,
            description: String::new(),
        });
        gc.selected = true;
        gc.clear();
        assert!(gc.item.is_none());
        assert!(!gc.selected);
    }
}
