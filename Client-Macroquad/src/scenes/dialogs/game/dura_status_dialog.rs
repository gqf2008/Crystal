// ============================================================================
// DuraStatusDialogHybrid - 装备耐久显示（对齐 C# DuraStatusDialog + CharacterDuraPanel）
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/MainDialogs.cs:3886-4060
// - DuraStatusDialog：触发按钮（Prguse[2111-2113]），位于小地图下方
// - CharacterDuraPanel：耐久面板（Prguse[2105]），显示各部位耐久色条
//   - 背景：Prguse[2105]
//   - 灰色底层：Prguse[2161]
//   - 彩色覆盖：Prguse[2162]
//   - 14 个部位耐久指示：绿色(>50%)/黄色(>20%)/红色(≤20%)
//
// ============================================================================

use macroquad::prelude::*;
use crate::ui::text_renderer::draw_text_cn;

// ============================================================================
// 常量
// ============================================================================

/// 耐久面板宽度
const PANEL_WIDTH: f32 = 62.0;
/// 耐久面板高度
const PANEL_HEIGHT: f32 = 86.0;

// ============================================================================
// 类型定义
// ============================================================================

/// 装备部位耐久数据
#[derive(Debug, Clone)]
pub struct EquipDurability {
    pub slot_name: &'static str,
    pub current: u32,
    pub max: u32,
}

impl EquipDurability {
    pub fn new(slot_name: &'static str, current: u32, max: u32) -> Self {
        Self { slot_name, current, max }
    }

    /// 耐久百分比 (0.0 ~ 1.0)
    pub fn ratio(&self) -> f32 {
        if self.max == 0 {
            return 1.0;
        }
        (self.current as f32 / self.max as f32).clamp(0.0, 1.0)
    }

    /// 耐久颜色（绿 > 黄 > 红）
    pub fn color(&self) -> Color {
        let ratio = self.ratio();
        if ratio > 0.5 {
            GREEN
        } else if ratio > 0.2 {
            YELLOW
        } else {
            RED
        }
    }
}

/// 装备部位在面板中的位置和尺寸
struct DuraPiece {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

/// 获取各部位在面板中的布局（对齐 C# CharacterDuraPanel）
fn dura_pieces() -> [DuraPiece; 14] {
    [
        DuraPiece { x: 4.0,  y: 5.0,  w: 12.0, h: 33.0 },  // 武器
        DuraPiece { x: 16.0, y: 11.0, w: 28.0, h: 32.0 },  // 盔甲
        DuraPiece { x: 24.0, y: 3.0,  w: 12.0, h: 12.0 },  // 头盔
        DuraPiece { x: 44.0, y: 5.0,  w: 8.0,  h: 32.0 },  // 火把
        DuraPiece { x: 3.0,  y: 67.0, w: 12.0, h: 12.0 },  // 项链
        DuraPiece { x: 3.0,  y: 43.0, w: 12.0, h: 8.0 },   // 左手镯
        DuraPiece { x: 43.0, y: 43.0, w: 12.0, h: 8.0 },   // 右手镯
        DuraPiece { x: 3.0,  y: 54.0, w: 12.0, h: 12.0 },  // 左戒指
        DuraPiece { x: 43.0, y: 54.0, w: 12.0, h: 12.0 },  // 右戒指
        DuraPiece { x: 16.0, y: 54.0, w: 12.0, h: 12.0 },  // 护符
        DuraPiece { x: 23.0, y: 23.0, w: 12.0, h: 7.0 },   // 腰带
        DuraPiece { x: 17.0, y: 43.0, w: 24.0, h: 9.0 },   // 靴子
        DuraPiece { x: 30.0, y: 54.0, w: 12.0, h: 12.0 },  // 宝石
        DuraPiece { x: 43.0, y: 68.0, w: 12.0, h: 12.0 },  // 坐骑
    ]
}

/// 装备耐久显示对话框
pub struct DuraStatusDialogHybrid {
    /// 触发按钮是否可见
    button_visible: bool,
    /// 面板是否可见
    panel_visible: bool,
    /// 触发按钮位置
    button_position: Vec2,
    /// 面板位置
    panel_position: Vec2,

    // === 数据 ===
    /// 14 个装备部位的耐久数据
    durabilities: [Option<EquipDurability>; 14],

    // === 交互 ===
    hovered_piece: Option<usize>,
}

impl DuraStatusDialogHybrid {
    pub fn new() -> Self {
        Self {
            button_visible: true,
            panel_visible: false,
            button_position: vec2(screen_width() - 80.0, 120.0),
            panel_position: vec2(screen_width() - 61.0, 200.0),

            durabilities: Default::default(),

            hovered_piece: None,
        }
    }

    // === 公共 API ===

    pub fn button_visible(&self) -> bool {
        self.button_visible
    }

    pub fn set_button_visible(&mut self, visible: bool) {
        self.button_visible = visible;
    }

    pub fn panel_visible(&self) -> bool {
        self.panel_visible
    }

    pub fn toggle_panel(&mut self) {
        self.panel_visible = !self.panel_visible;
    }

    /// 设置装备部位耐久
    pub fn set_durability(&mut self, slot: usize, dura: Option<EquipDurability>) {
        if slot < 14 {
            self.durabilities[slot] = dura;
        }
    }

    /// 更新面板位置（小地图下方）
    pub fn update_position(&mut self, minimap_x: f32, minimap_height: f32) {
        self.button_position = vec2(minimap_x + 86.0, minimap_height);
        self.panel_position = vec2(screen_width() - 61.0, 200.0);
    }

    // === 绘制 ===

    pub fn draw(&mut self) {
        let mouse_pos = vec2(mouse_position().0, mouse_position().1);

        // 绘制触发按钮
        if self.button_visible {
            let btn_rect = Rect::new(
                self.button_position.x,
                self.button_position.y,
                20.0, 19.0,
            );

            // 按钮颜色（任何部位耐久低于 20% 时变红）
            let has_danger = self.durabilities.iter()
                .filter_map(|d| d.as_ref())
                .any(|d| d.ratio() <= 0.2);

            let btn_color = if has_danger {
                Color::new(1.0, 0.3, 0.3, 1.0)
            } else if btn_rect.contains(mouse_pos) {
                Color::new(0.8, 0.9, 1.0, 1.0)
            } else {
                Color::new(0.6, 0.6, 0.6, 0.8)
            };

            draw_rectangle(
                self.button_position.x,
                self.button_position.y,
                20.0, 19.0,
                btn_color,
            );
            draw_text_cn("耐", self.button_position.x + 4.0, self.button_position.y + 14.0, 10.0, WHITE);

            if ButtonState::is_clicked(btn_rect, mouse_pos) {
                self.panel_visible = !self.panel_visible;
            }
        }

        // 绘制耐久面板
        if self.panel_visible {
            self.draw_panel(mouse_pos);
        }
    }

    fn draw_panel(&mut self, mouse_pos: Vec2) {
        let pos = self.panel_position;
        let pieces = dura_pieces();

        // 面板背景
        draw_rectangle(pos.x, pos.y, PANEL_WIDTH, PANEL_HEIGHT, Color::new(0.1, 0.1, 0.1, 0.9));
        draw_rectangle_lines(pos.x, pos.y, PANEL_WIDTH, PANEL_HEIGHT, 1.0, Color::new(0.5, 0.5, 0.5, 0.8));

        // 灰色底层（身体轮廓）
        draw_rectangle(pos.x + 3.0, pos.y + 3.0, 56.0, 80.0, Color::new(0.3, 0.3, 0.3, 0.4));

        // 绘制各部位
        self.hovered_piece = None;
        for (i, piece) in pieces.iter().enumerate() {
            let px = pos.x + piece.x;
            let py = pos.y + piece.y;
            let piece_rect = Rect::new(px, py, piece.w, piece.h);

            if let Some(dura) = &self.durabilities[i] {
                // 耐久色条
                let color = dura.color();
                let filled_h = piece.h * dura.ratio();
                let empty_h = piece.h - filled_h;

                // 空白部分（灰色）
                if empty_h > 0.0 {
                    draw_rectangle(px, py, piece.w, empty_h, Color::new(0.2, 0.2, 0.2, 0.5));
                }
                // 填充部分
                draw_rectangle(px, py + empty_h, piece.w, filled_h, Color::new(color.r, color.g, color.b, 0.7));
            } else {
                // 无装备 — 暗灰
                draw_rectangle(px, py, piece.w, piece.h, Color::new(0.15, 0.15, 0.15, 0.3));
            }

            // 边框
            draw_rectangle_lines(px, py, piece.w, piece.h, 0.5, Color::new(0.4, 0.4, 0.4, 0.3));

            // 悬停
            if piece_rect.contains(mouse_pos) {
                self.hovered_piece = Some(i);
                draw_rectangle(px, py, piece.w, piece.h, Color::new(1.0, 1.0, 1.0, 0.2));
            }
        }

        // 工具提示
        if let Some(idx) = self.hovered_piece {
            if let Some(dura) = &self.durabilities[idx] {
                let pct = (dura.ratio() * 100.0) as u32;
                let tooltip = format!("{}: {}/{} ({}%)", dura.slot_name, dura.current, dura.max, pct);
                let tip_x = mouse_pos.x + 15.0;
                let tip_y = mouse_pos.y + 15.0;

                draw_rectangle(tip_x, tip_y, 180.0, 24.0, Color::new(0.0, 0.0, 0.0, 0.85));
                draw_rectangle_lines(tip_x, tip_y, 180.0, 24.0, 1.0, Color::new(0.6, 0.6, 0.6, 0.8));
                draw_text_cn(&tooltip, tip_x + 6.0, tip_y + 16.0, 11.0, WHITE);
            }
        }
    }
}

/// Button state helper (reuse from native_ui_utils via import)
struct ButtonState;
impl ButtonState {
    fn is_clicked(rect: Rect, mouse_pos: Vec2) -> bool {
        rect.contains(mouse_pos) && is_mouse_button_pressed(MouseButton::Left)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equip_durability_ratio() {
        let full = EquipDurability::new("武器", 100, 100);
        assert!((full.ratio() - 1.0).abs() < f32::EPSILON);

        let half = EquipDurability::new("盔甲", 50, 100);
        assert!((half.ratio() - 0.5).abs() < f32::EPSILON);

        let low = EquipDurability::new("靴子", 10, 100);
        assert!((low.ratio() - 0.1).abs() < f32::EPSILON);

        let zero_max = EquipDurability::new("test", 0, 0);
        assert!((zero_max.ratio() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_equip_durability_color() {
        let full = EquipDurability::new("武器", 100, 100);
        assert_eq!(full.color(), GREEN);

        let mid = EquipDurability::new("盔甲", 30, 100);
        assert_eq!(mid.color(), YELLOW);

        let low = EquipDurability::new("靴子", 10, 100);
        assert_eq!(low.color(), RED);
    }

    #[test]
    fn test_dura_dialog_basic() {
        let mut dialog = DuraStatusDialogHybrid::new();
        assert!(dialog.button_visible());
        assert!(!dialog.panel_visible());

        dialog.toggle_panel();
        assert!(dialog.panel_visible());

        dialog.set_durability(0, Some(EquipDurability::new("武器", 80, 100)));
        assert!(dialog.durabilities[0].is_some());
    }
}
