//! Ranking Dialog
//!
//! Leaderboards and player rankings display dialog.
//! Corresponds to Client/MirScenes/Dialogs/RankingDialog.cs

use std::time::{Duration, Instant};

/// 排名职业类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankClass {
    All = 0,    // 全部
    Warrior = 1, // 战士
    Wizard = 2,  // 巫师
    Taoist = 3,  // 道士
    Assassin = 4, // 刺客
    Archer = 5,  // 弓箭手
}

/// 排名角色信息
#[derive(Debug, Clone)]
pub struct RankCharacterInfo {
    pub player_id: u32,
    pub name: String,
    pub level: u16,
    pub class: RankClass,
}

/// 排名行 - 对应C#中的RankingRow类
#[derive(Debug, Clone)]
pub struct RankingRow {
    pub character_info: Option<RankCharacterInfo>,
    pub rank_index: usize,
    pub visible: bool,
}

impl RankingRow {
    pub fn new() -> Self {
        Self {
            character_info: None,
            rank_index: 0,
            visible: false,
        }
    }

    /// 更新排名行显示
    pub fn update(&mut self, info: RankCharacterInfo, rank_index: usize) {
        self.character_info = Some(info);
        self.rank_index = rank_index;
        self.visible = true;
    }

    /// 清除排名行
    pub fn clear(&mut self) {
        self.character_info = None;
        self.rank_index = 0;
        self.visible = false;
    }
}

/// 排行榜对话框
#[derive(Debug, Clone)]
pub struct RankingDialog {
    /// 是否可见
    pub visible: bool,
    /// 窗口位置
    pub location: (i32, i32),

    /// 当前排名类型
    pub rank_type: RankClass,
    /// 排名行
    pub rows: Vec<RankingRow>,
    /// 排名列表（按职业分组）
    pub rank_lists: Vec<Vec<RankCharacterInfo>>,
    /// 各职业的排名
    pub ranks: Vec<i32>,
    /// 滚动偏移
    pub row_offset: usize,
    /// 总排名数
    pub rank_count: usize,
    /// 是否只显示在线玩家
    pub online_only: bool,
    /// 每行间距
    pub gap_per_row: f32,
    /// 滚动高度
    pub scroll_height: f32,
    /// 下次请求时间
    pub next_request_time: Option<Instant>,
}

impl Default for RankingDialog {
    fn default() -> Self {
        Self {
            visible: false,
            location: (288, 162), // Centered on 800x600 screen
            rank_type: RankClass::All,
            rows: vec![RankingRow::new(); 20], // 20 rows like C#
            rank_lists: vec![Vec::new(); 6], // 6 classes
            ranks: vec![0; 6],
            row_offset: 0,
            rank_count: 0,
            online_only: false,
            gap_per_row: 0.0,
            scroll_height: 276.0, // Based on C# values
            next_request_time: None,
        }
    }
}

impl RankingDialog {
    /// 创建新的排行榜对话框
    pub fn new() -> Self {
        Self::default()
    }

    /// 显示对话框
    pub fn show(&mut self) {
        if self.visible {
            return;
        }
        self.visible = true;
        self.request_ranks(self.rank_type);
    }

    /// 隐藏对话框
    pub fn hide(&mut self) {
        if !self.visible {
            return;
        }
        self.visible = false;
    }

    /// 切换显示状态
    pub fn toggle(&mut self) {
        if !self.visible {
            self.show();
        } else {
            self.hide();
        }
    }

    /// 处理鼠标滚轮
    pub fn handle_mouse_wheel(&mut self, delta: i32) {
        let scroll_lines = delta / 120; // Standard mouse wheel delta
        self.move_offset(-(scroll_lines as i32));
    }

    /// 移动偏移
    pub fn move_offset(&mut self, distance: i32) {
        if distance > 0 {
            // 向下滚动
            self.row_offset = self.row_offset.saturating_add(distance as usize);
            if self.row_offset > self.rank_count.saturating_sub(20) {
                self.row_offset = self.rank_count.saturating_sub(20);
            }
        } else {
            // 向上滚动
            self.row_offset = self.row_offset.saturating_sub((-distance) as usize);
        }
        self.next_request_time = Some(Instant::now() + Duration::from_millis(500));
    }

    /// 请求排名数据
    pub fn request_ranks(&mut self, rank_type: RankClass) {
        // TODO: 发送网络请求获取排名
        // MirNetwork.Network.Enqueue(new ClientPackets.GetRanking {
        //     RankType = rank_type as byte,
        //     RankIndex = self.row_offset,
        //     OnlineOnly = self.online_only
        // });
        println!("Requesting ranks for type: {:?}, offset: {}, online_only: {}",
                rank_type, self.row_offset, self.online_only);
    }

    /// 接收排名数据
    pub fn receive_ranks(&mut self, rankings: Vec<RankCharacterInfo>, rank_type: RankClass, my_rank: i32, count: usize) {
        let type_index = rank_type as usize;
        self.rank_lists[type_index] = rankings;
        self.ranks[type_index] = my_rank;
        self.rank_count = count;
        self.update_ranks();

        let extra_rows = count.saturating_sub(20);
        self.gap_per_row = if extra_rows > 0 {
            self.scroll_height / extra_rows as f32
        } else {
            0.0
        };
    }

    /// 选择排名类型
    pub fn select_rank(&mut self, rank_type: RankClass) {
        self.rank_type = rank_type;
        // 清除所有行
        for row in &mut self.rows {
            row.clear();
        }
        self.row_offset = 0;
        self.request_ranks(rank_type);
    }

    /// 更新排名显示
    pub fn update_ranks(&mut self) {
        let type_index = self.rank_type as usize;
        let current_list = &self.rank_lists[type_index];

        for (i, row) in self.rows.iter_mut().enumerate() {
            let data_index = self.row_offset + i;
            if data_index < current_list.len() {
                row.update(current_list[data_index].clone(), self.row_offset + i + 1);
            } else {
                row.clear();
            }
        }
    }

    /// 跳转到我的排名
    pub fn go_to_my_rank(&mut self) {
        let my_rank = self.ranks[self.rank_type as usize];
        if my_rank > 0 {
            // 跳转到我的排名位置（显示在列表中间）
            self.row_offset = (my_rank as usize).saturating_sub(10).max(0);
            self.next_request_time = Some(Instant::now());
        }
    }

    /// 切换在线过滤
    pub fn toggle_online_only(&mut self) {
        self.online_only = !self.online_only;
        self.row_offset = 0;
        self.next_request_time = Some(Instant::now());
    }

    /// 处理每帧更新
    pub fn process(&mut self) {
        if let Some(next_time) = self.next_request_time {
            if Instant::now() >= next_time {
                self.next_request_time = None;
                self.request_ranks(self.rank_type);
            }
        }
    }

    /// 获取我的排名文本
    pub fn get_my_rank_text(&self) -> String {
        let my_rank = self.ranks[self.rank_type as usize];
        if my_rank == 0 {
            "Not Listed".to_string()
        } else {
            format!("Ranked: {}", my_rank)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ranking_dialog_creation() {
        let dialog = RankingDialog::new();
        assert!(!dialog.visible);
        assert_eq!(dialog.rank_type, RankClass::All);
        assert_eq!(dialog.rows.len(), 20);
        assert_eq!(dialog.rank_lists.len(), 6);
        assert_eq!(dialog.ranks.len(), 6);
        assert!(!dialog.online_only);
    }

    #[test]
    fn test_ranking_dialog_show_hide() {
        let mut dialog = RankingDialog::new();
        dialog.show();
        assert!(dialog.visible);
        dialog.hide();
        assert!(!dialog.visible);
    }

    #[test]
    fn test_ranking_dialog_toggle() {
        let mut dialog = RankingDialog::new();
        dialog.toggle();
        assert!(dialog.visible);
        dialog.toggle();
        assert!(!dialog.visible);
    }

    #[test]
    fn test_select_rank() {
        let mut dialog = RankingDialog::new();
        dialog.select_rank(RankClass::Warrior);
        assert_eq!(dialog.rank_type, RankClass::Warrior);
        assert_eq!(dialog.row_offset, 0);
    }

    #[test]
    fn test_receive_ranks() {
        let mut dialog = RankingDialog::new();
        let rankings = vec![
            RankCharacterInfo {
                player_id: 1,
                name: "Player1".to_string(),
                level: 50,
                class: RankClass::Warrior,
            },
            RankCharacterInfo {
                player_id: 2,
                name: "Player2".to_string(),
                level: 45,
                class: RankClass::Wizard,
            },
        ];

        dialog.receive_ranks(rankings.clone(), RankClass::All, 5, 100);

        assert_eq!(dialog.rank_lists[0].len(), 2);
        assert_eq!(dialog.ranks[0], 5);
        assert_eq!(dialog.rank_count, 100);
        assert!(dialog.gap_per_row > 0.0);
    }

    #[test]
    fn test_update_ranks() {
        let mut dialog = RankingDialog::new();
        let rankings = vec![
            RankCharacterInfo {
                player_id: 1,
                name: "Player1".to_string(),
                level: 50,
                class: RankClass::Warrior,
            },
        ];

        dialog.receive_ranks(rankings, RankClass::All, 1, 1);
        dialog.update_ranks();

        assert!(dialog.rows[0].visible);
        assert_eq!(dialog.rows[0].rank_index, 1);
        assert_eq!(dialog.rows[0].character_info.as_ref().unwrap().name, "Player1");
        assert!(!dialog.rows[1].visible); // Second row should be empty
    }

    #[test]
    fn test_move_offset() {
        let mut dialog = RankingDialog::new();
        dialog.rank_count = 50;

        dialog.move_offset(5);
        assert_eq!(dialog.row_offset, 5);

        dialog.move_offset(-3);
        assert_eq!(dialog.row_offset, 2);

        dialog.move_offset(-10); // Should not go below 0
        assert_eq!(dialog.row_offset, 0);
    }

    #[test]
    fn test_toggle_online_only() {
        let mut dialog = RankingDialog::new();
        assert!(!dialog.online_only);

        dialog.toggle_online_only();
        assert!(dialog.online_only);

        dialog.toggle_online_only();
        assert!(!dialog.online_only);
    }

    #[test]
    fn test_get_my_rank_text() {
        let mut dialog = RankingDialog::new();

        // Not ranked
        assert_eq!(dialog.get_my_rank_text(), "Not Listed");

        // Ranked
        dialog.ranks[0] = 15;
        assert_eq!(dialog.get_my_rank_text(), "Ranked: 15");
    }

    #[test]
    fn test_ranking_row() {
        let mut row = RankingRow::new();
        assert!(!row.visible);

        let info = RankCharacterInfo {
            player_id: 123,
            name: "TestPlayer".to_string(),
            level: 30,
            class: RankClass::Warrior,
        };

        row.update(info.clone(), 5);
        assert!(row.visible);
        assert_eq!(row.rank_index, 5);
        assert_eq!(row.character_info.as_ref().unwrap().name, "TestPlayer");

        row.clear();
        assert!(!row.visible);
        assert!(row.character_info.is_none());
    }

    #[test]
    fn test_go_to_my_rank() {
        let mut dialog = RankingDialog::new();
        dialog.ranks[0] = 25; // My rank is 25

        dialog.go_to_my_rank();
        assert_eq!(dialog.row_offset, 15); // 25 - 10 = 15
    }
}