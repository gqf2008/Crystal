// ============================================================================
// FriendDialogHybrid - 好友列表对话框（对齐 C# FriendDialog）
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/FriendDialog.cs (~570 行)
// - 背景：Prguse[216]
// - 好友/黑名单双页签
// - 2 列 × 6 行 = 12 个显示行
// - 操作按钮：添加、删除、备忘、邮件、私聊
// - 翻页：上一页 / 下一页
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::*;

// ============================================================================
// 常量
// ============================================================================

/// 每页显示行数（2 列 × 6 行）
const ROWS_PER_COL: usize = 6;
const COLS: usize = 2;
const ROWS_PER_PAGE: usize = ROWS_PER_COL * COLS;
/// 行高
const ROW_HEIGHT: f32 = 17.0;
/// 列宽
const COL_WIDTH: f32 = 115.0;
/// 列表起始偏移
const LIST_X: f32 = 16.0;
const LIST_Y: f32 = 58.0;
/// 窗口尺寸
const DIALOG_WIDTH: f32 = 270.0;
const DIALOG_HEIGHT: f32 = 260.0;

/// 好友信息
#[derive(Debug, Clone)]
pub struct FriendEntry {
    pub name: String,
    pub online: bool,
    pub blocked: bool,
    pub memo: String,
}

impl FriendEntry {
    pub fn new(name: &str, online: bool) -> Self {
        Self {
            name: name.to_string(),
            online,
            blocked: false,
            memo: String::new(),
        }
    }
}

/// 好友对话框页签
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FriendTab {
    Friends,
    Blacklist,
}

/// 对话框动作
#[derive(Debug, Clone, PartialEq)]
pub enum FriendAction {
    /// 添加好友（需要后续输入名字）
    AddFriend,
    /// 删除选中好友
    RemoveFriend(String),
    /// 查看/编辑备忘
    ViewMemo(String),
    /// 发送邮件
    SendMail(String),
    /// 私聊
    Whisper(String),
    /// 关闭对话框
    Close,
    /// 切换页签
    SwitchTab(FriendTab),
    /// 上一页
    PrevPage,
    /// 下一页
    NextPage,
}

/// 好友列表对话框
pub struct FriendDialogHybrid {
    pub visible: bool,
    pub tab: FriendTab,
    pub friends: Vec<FriendEntry>,
    pub selected_index: Option<usize>,
    page: usize,
    position: Vec2,
    // UI
    bg_texture: BackgroundTexture,
    close_btn: CloseButton,
    drag_helper: DragHelper,
}

impl FriendDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            tab: FriendTab::Friends,
            friends: Vec::new(),
            selected_index: None,
            page: 0,
            position: Vec2::new(200.0, 100.0),
            bg_texture: BackgroundTexture::new(),
            close_btn: CloseButton::new(),
            drag_helper: DragHelper::new(),
        }
    }

    pub fn load_textures(&mut self) {
        self.bg_texture = BackgroundTexture::load(LibraryName::Prguse, 216, None);
        self.close_btn = CloseButton::load_prguse2();
    }

    /// 设置好友列表
    pub fn set_friends(&mut self, friends: Vec<FriendEntry>) {
        self.friends = friends;
        self.selected_index = None;
        self.page = 0;
    }

    /// 获取当前页签过滤后的列表
    fn filtered_friends(&self) -> Vec<(usize, &FriendEntry)> {
        self.friends
            .iter()
            .enumerate()
            .filter(|(_, f)| match self.tab {
                FriendTab::Friends => !f.blocked,
                FriendTab::Blacklist => f.blocked,
            })
            .collect()
    }

    /// 当前页签的总页数
    fn total_pages(&self) -> usize {
        let count = self.filtered_friends().len();
        if count == 0 { 1 } else { (count + ROWS_PER_PAGE - 1) / ROWS_PER_PAGE }
    }

    /// 获取选中好友名
    fn selected_name(&self) -> Option<String> {
        self.selected_index.and_then(|i| self.friends.get(i)).map(|f| f.name.clone())
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<FriendAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        // --- 拖动 ---
        let title_rect = Rect::new(self.position.x, self.position.y, DIALOG_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        // --- 背景 ---
        self.bg_texture.draw(vec2(x, y));

        // --- 标题 ---
        draw_text_cn("好友列表", x + 100.0, y + 6.0, 13.0, GOLD);

        // --- 页签 ---
        let friend_tab_rect = Rect::new(x + 10.0, y + 30.0, 80.0, 20.0);
        let black_tab_rect = Rect::new(x + 100.0, y + 30.0, 80.0, 20.0);

        let friend_tab_color = if self.tab == FriendTab::Friends { GOLD } else { GRAY };
        let black_tab_color = if self.tab == FriendTab::Blacklist { GOLD } else { GRAY };

        draw_rectangle(friend_tab_rect.x, friend_tab_rect.y, friend_tab_rect.w, friend_tab_rect.h,
            if self.tab == FriendTab::Friends { Color::new(0.3, 0.25, 0.1, 0.8) } else { Color::new(0.15, 0.15, 0.15, 0.5) });
        draw_text_cn("好友", friend_tab_rect.x + 22.0, friend_tab_rect.y + 3.0, 12.0, friend_tab_color);

        draw_rectangle(black_tab_rect.x, black_tab_rect.y, black_tab_rect.w, black_tab_rect.h,
            if self.tab == FriendTab::Blacklist { Color::new(0.3, 0.1, 0.1, 0.8) } else { Color::new(0.15, 0.15, 0.15, 0.5) });
        draw_text_cn("黑名单", black_tab_rect.x + 15.0, black_tab_rect.y + 3.0, 12.0, black_tab_color);

        if is_mouse_button_pressed(MouseButton::Left) {
            if friend_tab_rect.contains(mouse) && self.tab != FriendTab::Friends {
                self.tab = FriendTab::Friends;
                self.page = 0;
                self.selected_index = None;
                action = Some(FriendAction::SwitchTab(FriendTab::Friends));
            } else if black_tab_rect.contains(mouse) && self.tab != FriendTab::Blacklist {
                self.tab = FriendTab::Blacklist;
                self.page = 0;
                self.selected_index = None;
                action = Some(FriendAction::SwitchTab(FriendTab::Blacklist));
            }
        }

        // --- 好友列表 ---
        // Collect filtered data to avoid borrow conflict with self.selected_index
        let filtered: Vec<(usize, String, bool)> = self.friends
            .iter()
            .enumerate()
            .filter(|(_, f)| match self.tab {
                FriendTab::Friends => !f.blocked,
                FriendTab::Blacklist => f.blocked,
            })
            .map(|(i, f)| (i, f.name.clone(), f.online))
            .collect();
        let start = self.page * ROWS_PER_PAGE;
        let end = (start + ROWS_PER_PAGE).min(filtered.len());

        for display_i in 0..(end - start) {
            let (orig_idx, ref name, online) = filtered[start + display_i];
            let col = display_i / ROWS_PER_COL;
            let row = display_i % ROWS_PER_COL;

            let rx = x + LIST_X + col as f32 * COL_WIDTH;
            let ry = y + LIST_Y + row as f32 * ROW_HEIGHT;
            let row_rect = Rect::new(rx, ry, COL_WIDTH - 4.0, ROW_HEIGHT);

            // 选中高亮
            let is_selected = self.selected_index == Some(orig_idx);
            if is_selected {
                draw_rectangle(rx, ry, COL_WIDTH - 4.0, ROW_HEIGHT, Color::new(0.3, 0.3, 0.3, 0.5));
            }

            // 悬停高亮
            if row_rect.contains(mouse) {
                draw_rectangle(rx, ry, COL_WIDTH - 4.0, ROW_HEIGHT, Color::new(0.4, 0.4, 0.4, 0.3));
            }

            // 名称（在线绿色，离线白色）
            let name_color = if online { GREEN } else { WHITE };
            draw_text_cn(name, rx + 4.0, ry + 2.0, 11.0, name_color);

            // 点击选择
            if is_mouse_button_pressed(MouseButton::Left) && row_rect.contains(mouse) {
                self.selected_index = Some(orig_idx);
            }
        }

        // --- 翻页 ---
        let total = self.total_pages();
        let page_text = format!("{}/{}", self.page + 1, total);
        draw_text_cn(&page_text, x + 115.0, y + 170.0, 11.0, WHITE);

        let prev_rect = Rect::new(x + 80.0, y + 168.0, 30.0, 18.0);
        let next_rect = Rect::new(x + 160.0, y + 168.0, 30.0, 18.0);

        if ButtonState::is_clicked(prev_rect, mouse) && self.page > 0 {
            self.page -= 1;
            action = Some(FriendAction::PrevPage);
        }
        let prev_color = if ButtonState::from_mouse(prev_rect, mouse) == ButtonState::Hover { WHITE } else { GRAY };
        draw_text_cn("◀", prev_rect.x + 8.0, prev_rect.y + 2.0, 12.0, prev_color);

        if ButtonState::is_clicked(next_rect, mouse) && self.page + 1 < total {
            self.page += 1;
            action = Some(FriendAction::NextPage);
        }
        let next_color = if ButtonState::from_mouse(next_rect, mouse) == ButtonState::Hover { WHITE } else { GRAY };
        draw_text_cn("▶", next_rect.x + 8.0, next_rect.y + 2.0, 12.0, next_color);

        // --- 操作按钮 ---
        let btn_y = y + 195.0;
        let btn_w = 45.0;
        let btn_h = 20.0;
        let btn_gap = 4.0;

        let buttons = [
            ("添加", 0),
            ("删除", 1),
            ("备忘", 2),
            ("邮件", 3),
            ("私聊", 4),
        ];

        for (label, idx) in &buttons {
            let bx = x + 10.0 + (*idx as f32) * (btn_w + btn_gap);
            let btn_rect = Rect::new(bx, btn_y, btn_w, btn_h);

            // 绘制简单文本按钮
            let state = ButtonState::from_mouse(btn_rect, mouse);
            let color = match state {
                ButtonState::Hover | ButtonState::Pressed => WHITE,
                _ => GRAY,
            };
            draw_rectangle_lines(btn_rect.x, btn_rect.y, btn_rect.w, btn_rect.h, 1.0, Color::new(0.4, 0.4, 0.4, 0.6));
            draw_text_cn(label, bx + 6.0, btn_y + 3.0, 11.0, color);

            if ButtonState::is_clicked(btn_rect, mouse) {
                match idx {
                    0 => action = Some(FriendAction::AddFriend),
                    1 => {
                        if let Some(name) = self.selected_name() {
                            action = Some(FriendAction::RemoveFriend(name));
                        }
                    }
                    2 => {
                        if let Some(name) = self.selected_name() {
                            action = Some(FriendAction::ViewMemo(name));
                        }
                    }
                    3 => {
                        if let Some(name) = self.selected_name() {
                            action = Some(FriendAction::SendMail(name));
                        }
                    }
                    4 => {
                        if let Some(name) = self.selected_name() {
                            action = Some(FriendAction::Whisper(name));
                        }
                    }
                    _ => {}
                }
            }
        }

        // --- 关闭按钮 ---
        let win_size = vec2(DIALOG_WIDTH, DIALOG_HEIGHT);
        if self.close_btn.draw(self.position, win_size, mouse) {
            self.visible = false;
            action = Some(FriendAction::Close);
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
    fn test_friend_dialog_creation() {
        let dialog = FriendDialogHybrid::new();
        assert!(!dialog.visible);
        assert_eq!(dialog.tab, FriendTab::Friends);
        assert!(dialog.friends.is_empty());
        assert!(dialog.selected_index.is_none());
    }

    #[test]
    fn test_set_friends_and_filter() {
        let mut dialog = FriendDialogHybrid::new();
        let friends = vec![
            FriendEntry::new("Alice", true),
            FriendEntry { name: "Troll".into(), online: false, blocked: true, memo: String::new() },
            FriendEntry::new("Bob", false),
        ];
        dialog.set_friends(friends);

        // Friends tab shows non-blocked
        dialog.tab = FriendTab::Friends;
        assert_eq!(dialog.filtered_friends().len(), 2);

        // Blacklist tab shows blocked
        dialog.tab = FriendTab::Blacklist;
        assert_eq!(dialog.filtered_friends().len(), 1);
    }

    #[test]
    fn test_pagination() {
        let mut dialog = FriendDialogHybrid::new();
        // 30 friends -> 3 pages (12 per page)
        let friends: Vec<FriendEntry> = (0..30)
            .map(|i| FriendEntry::new(&format!("Player{}", i), i % 2 == 0))
            .collect();
        dialog.set_friends(friends);
        assert_eq!(dialog.total_pages(), 3);
    }

    #[test]
    fn test_friend_entry() {
        let f = FriendEntry::new("TestPlayer", true);
        assert_eq!(f.name, "TestPlayer");
        assert!(f.online);
        assert!(!f.blocked);
        assert!(f.memo.is_empty());
    }
}
