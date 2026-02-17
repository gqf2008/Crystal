// ============================================================================
// BigMapDialogHybrid - 大地图对话框（对齐 C# BigMapDialog）
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/BigMapDialog.cs (~858 行)
// - 背景：Prguse[219]
// - 地图视口：显示当前地图的缩略图
// - NPC 列表：右侧可滚动 NPC 列表
// - 功能：点击传送、NPC 定位、坐标显示
// - 玩家位置标记：黄色点
// - 组队成员：蓝色点
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::*;

// ============================================================================
// 常量
// ============================================================================

/// 窗口尺寸
const DIALOG_WIDTH: f32 = 480.0;
const DIALOG_HEIGHT: f32 = 380.0;
/// 地图视口区域
const MAP_X: f32 = 10.0;
const MAP_Y: f32 = 35.0;
const MAP_W: f32 = 320.0;
const MAP_H: f32 = 300.0;
/// NPC 列表区域
const NPC_LIST_X: f32 = 340.0;
const NPC_LIST_Y: f32 = 55.0;
const NPC_LIST_W: f32 = 130.0;
const NPC_ROW_H: f32 = 18.0;
/// 地图坐标缩放因子（归一化→游戏单位）
const MAP_COORDINATE_SCALE: f32 = 300.0;
/// NPC 列表可见行数
const NPC_VISIBLE_ROWS: usize = 15;

// ============================================================================
// 类型定义
// ============================================================================

/// 地图 NPC 标记
#[derive(Debug, Clone)]
pub struct MapNpc {
    pub name: String,
    /// 在地图上的归一化坐标 (0.0-1.0)
    pub map_x: f32,
    pub map_y: f32,
    /// NPC 类型（区分图标颜色）
    pub is_merchant: bool,
}

/// 玩家标记
#[derive(Debug, Clone)]
pub struct MapPlayer {
    pub name: String,
    pub map_x: f32,
    pub map_y: f32,
    pub is_group_member: bool,
}

/// 大地图动作
#[derive(Debug, Clone, PartialEq)]
pub enum BigMapAction {
    /// 点击坐标进行寻路 (归一化坐标)
    MoveTo(f32, f32),
    /// 选择 NPC
    SelectNpc(String),
    /// 关闭
    Close,
    /// NPC 列表滚动
    ScrollNpcList(i32),
}

/// 大地图对话框
pub struct BigMapDialogHybrid {
    pub visible: bool,
    pub map_title: String,
    /// 玩家当前位置（归一化坐标 0.0-1.0）
    pub player_x: f32,
    pub player_y: f32,
    /// NPC 标记列表
    pub npcs: Vec<MapNpc>,
    /// 玩家标记（组队成员等）
    pub players: Vec<MapPlayer>,
    /// NPC 列表选中索引
    pub selected_npc: Option<usize>,
    /// NPC 列表滚动偏移
    npc_scroll: usize,
    position: Vec2,
    // UI
    bg_texture: BackgroundTexture,
    map_texture: Option<Texture2D>,
    close_btn: CloseButton,
    drag_helper: DragHelper,
    // 坐标显示
    mouse_map_coord: Option<(f32, f32)>,
}

impl BigMapDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            map_title: String::new(),
            player_x: 0.5,
            player_y: 0.5,
            npcs: Vec::new(),
            players: Vec::new(),
            selected_npc: None,
            npc_scroll: 0,
            position: Vec2::new(100.0, 50.0),
            bg_texture: BackgroundTexture::new(),
            map_texture: None,
            close_btn: CloseButton::new(),
            drag_helper: DragHelper::new(),
            mouse_map_coord: None,
        }
    }

    pub fn load_textures(&mut self) {
        self.bg_texture = BackgroundTexture::load(LibraryName::Prguse, 219, None);
        self.close_btn = CloseButton::load_prguse2();
    }

    /// 设置地图缩略图纹理
    pub fn set_map_texture(&mut self, texture: Option<Texture2D>) {
        self.map_texture = texture;
    }

    /// 更新玩家位置（归一化坐标）
    pub fn update_player_position(&mut self, nx: f32, ny: f32) {
        self.player_x = nx.clamp(0.0, 1.0);
        self.player_y = ny.clamp(0.0, 1.0);
    }

    /// 设置 NPC 列表
    pub fn set_npcs(&mut self, npcs: Vec<MapNpc>) {
        self.npcs = npcs;
        self.selected_npc = None;
        self.npc_scroll = 0;
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<BigMapAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        // --- 拖动 ---
        let title_rect = Rect::new(self.position.x, self.position.y, DIALOG_WIDTH, 25.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        // --- 背景 ---
        self.bg_texture.draw(vec2(x, y));

        // --- 标题 ---
        let title = if self.map_title.is_empty() { "大地图" } else { &self.map_title };
        draw_text_cn(title, x + 190.0, y + 6.0, 13.0, GOLD);

        // --- 地图视口 ---
        let map_rect = Rect::new(x + MAP_X, y + MAP_Y, MAP_W, MAP_H);
        draw_rectangle(map_rect.x, map_rect.y, map_rect.w, map_rect.h, Color::new(0.05, 0.1, 0.05, 1.0));
        draw_rectangle_lines(map_rect.x, map_rect.y, map_rect.w, map_rect.h, 1.0, DARKGRAY);

        // 地图纹理
        if let Some(tex) = &self.map_texture {
            draw_texture(tex, map_rect.x, map_rect.y, WHITE);
        }

        // --- 玩家位置标记（黄色三角） ---
        let px = map_rect.x + self.player_x * MAP_W;
        let py = map_rect.y + self.player_y * MAP_H;
        draw_circle(px, py, 4.0, YELLOW);
        draw_circle_lines(px, py, 5.0, 1.0, WHITE);

        // --- 组队成员标记（蓝色点） ---
        for player in &self.players {
            let ppx = map_rect.x + player.map_x * MAP_W;
            let ppy = map_rect.y + player.map_y * MAP_H;
            let color = if player.is_group_member { BLUE } else { WHITE };
            draw_circle(ppx, ppy, 3.0, color);
        }

        // --- NPC 标记（地图上的点） ---
        for (i, npc) in self.npcs.iter().enumerate() {
            let nx = map_rect.x + npc.map_x * MAP_W;
            let ny = map_rect.y + npc.map_y * MAP_H;
            let color = if npc.is_merchant { GREEN } else { RED };
            let is_selected = self.selected_npc == Some(i);
            let radius = if is_selected { 4.0 } else { 2.5 };
            draw_circle(nx, ny, radius, color);
            if is_selected {
                draw_circle_lines(nx, ny, 6.0, 1.0, GOLD);
            }
        }

        // --- 鼠标坐标显示 ---
        self.mouse_map_coord = None;
        if map_rect.contains(mouse) {
            let rel_x = (mouse.x - map_rect.x) / MAP_W;
            let rel_y = (mouse.y - map_rect.y) / MAP_H;
            self.mouse_map_coord = Some((rel_x, rel_y));

            // 坐标提示
            let coord_text = format!("({:.0}, {:.0})", rel_x * MAP_COORDINATE_SCALE, rel_y * MAP_COORDINATE_SCALE);
            draw_text_cn(&coord_text, mouse.x + 12.0, mouse.y - 5.0, 10.0, LIGHTGRAY);

            // 点击寻路
            if is_mouse_button_pressed(MouseButton::Left) {
                action = Some(BigMapAction::MoveTo(rel_x, rel_y));
            }
        }

        // --- NPC 列表标题 ---
        draw_text_cn("NPC 列表", x + NPC_LIST_X, y + 38.0, 12.0, GOLD);

        // --- NPC 列表 ---
        let visible_end = (self.npc_scroll + NPC_VISIBLE_ROWS).min(self.npcs.len());
        for display_i in 0..NPC_VISIBLE_ROWS {
            let npc_i = self.npc_scroll + display_i;
            if npc_i >= self.npcs.len() {
                break;
            }

            let npc = &self.npcs[npc_i];
            let ry = y + NPC_LIST_Y + display_i as f32 * NPC_ROW_H;
            let row_rect = Rect::new(x + NPC_LIST_X, ry, NPC_LIST_W, NPC_ROW_H);

            // 选中高亮
            let is_selected = self.selected_npc == Some(npc_i);
            if is_selected {
                draw_rectangle(row_rect.x, row_rect.y, row_rect.w, row_rect.h,
                    Color::new(0.4, 0.3, 0.1, 0.5));
            }

            // 悬停高亮
            if row_rect.contains(mouse) {
                draw_rectangle(row_rect.x, row_rect.y, row_rect.w, row_rect.h,
                    Color::new(0.3, 0.3, 0.3, 0.3));
            }

            // NPC 颜色标记
            let dot_color = if npc.is_merchant { GREEN } else { RED };
            draw_circle(row_rect.x + 6.0, ry + NPC_ROW_H / 2.0, 3.0, dot_color);

            // NPC 名称
            let name_color = if is_selected { GOLD } else { WHITE };
            draw_text_cn(&npc.name, row_rect.x + 14.0, ry + 2.0, 10.0, name_color);

            // 点击选中
            if is_mouse_button_pressed(MouseButton::Left) && row_rect.contains(mouse) {
                self.selected_npc = Some(npc_i);
                action = Some(BigMapAction::SelectNpc(npc.name.clone()));
            }
        }

        // --- NPC 列表滚动 ---
        let scroll_area = Rect::new(x + NPC_LIST_X, y + NPC_LIST_Y, NPC_LIST_W, NPC_VISIBLE_ROWS as f32 * NPC_ROW_H);
        if scroll_area.contains(mouse) {
            let (_, wheel_y) = mouse_wheel();
            if wheel_y > 0.0 && self.npc_scroll > 0 {
                self.npc_scroll -= 1;
                action = Some(BigMapAction::ScrollNpcList(-1));
            } else if wheel_y < 0.0 && visible_end < self.npcs.len() {
                self.npc_scroll += 1;
                action = Some(BigMapAction::ScrollNpcList(1));
            }
        }

        // --- 坐标信息 ---
        let coord_label = format!("坐标: ({:.0}, {:.0})", self.player_x * MAP_COORDINATE_SCALE, self.player_y * MAP_COORDINATE_SCALE);
        draw_text_cn(&coord_label, x + MAP_X, y + DIALOG_HEIGHT - 15.0, 11.0, WHITE);

        // --- 关闭按钮 ---
        let win_size = vec2(DIALOG_WIDTH, DIALOG_HEIGHT);
        if self.close_btn.draw(self.position, win_size, mouse) {
            self.visible = false;
            action = Some(BigMapAction::Close);
        }

        action
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bigmap_dialog_creation() {
        let dialog = BigMapDialogHybrid::new();
        assert!(!dialog.visible);
        assert!(dialog.npcs.is_empty());
        assert!(dialog.players.is_empty());
        assert!(dialog.selected_npc.is_none());
    }

    #[test]
    fn test_update_player_position() {
        let mut dialog = BigMapDialogHybrid::new();
        dialog.update_player_position(0.5, 0.8);
        assert!((dialog.player_x - 0.5).abs() < f32::EPSILON);
        assert!((dialog.player_y - 0.8).abs() < f32::EPSILON);

        // 超出范围自动钳位
        dialog.update_player_position(-0.1, 1.5);
        assert!((dialog.player_x - 0.0).abs() < f32::EPSILON);
        assert!((dialog.player_y - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_set_npcs() {
        let mut dialog = BigMapDialogHybrid::new();
        let npcs = vec![
            MapNpc { name: "武器商".into(), map_x: 0.3, map_y: 0.5, is_merchant: true },
            MapNpc { name: "野怪".into(), map_x: 0.7, map_y: 0.2, is_merchant: false },
        ];
        dialog.set_npcs(npcs);
        assert_eq!(dialog.npcs.len(), 2);
        assert!(dialog.selected_npc.is_none());
    }

    #[test]
    fn test_map_npc_types() {
        let merchant = MapNpc { name: "铁匠".into(), map_x: 0.5, map_y: 0.5, is_merchant: true };
        let mob = MapNpc { name: "Boss".into(), map_x: 0.8, map_y: 0.8, is_merchant: false };
        assert!(merchant.is_merchant);
        assert!(!mob.is_merchant);
    }
}
