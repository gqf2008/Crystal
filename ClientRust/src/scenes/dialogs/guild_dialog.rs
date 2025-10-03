// Guild Dialog - 公会对话框
// 管理公会成员、公告、仓库、等级、Buff等

use super::Dialog;
use crate::network::protocol::UserItem;
use std::collections::HashMap;

/// 公会页面类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildPage {
    Notice,  // 公告页
    Members, // 成员页
    Storage, // 仓库页
    Rank,    // 权限页
    Buff,    // Buff页
    Status,  // 状态页
}

/// 公会成员信息
#[derive(Debug, Clone)]
pub struct GuildMember {
    pub name: String,
    pub rank_id: u8,
    pub rank_name: String,
    pub online: bool,
    pub level: u16,
    pub class: String,
}

impl GuildMember {
    pub fn new(name: String, rank_id: u8, rank_name: String) -> Self {
        Self {
            name,
            rank_id,
            rank_name,
            online: false,
            level: 1,
            class: String::from("Warrior"),
        }
    }
}

/// 公会权限
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildRankOptions {
    pub can_recruit: bool,        // 招募成员
    pub can_kick: bool,           // 踢出成员
    pub can_edit_ranks: bool,     // 编辑权限
    pub can_store_items: bool,    // 存入仓库
    pub can_retrieve_items: bool, // 取出仓库
    pub can_alter_alliance: bool, // 管理联盟
    pub can_change_notice: bool,  // 修改公告
    pub can_activate_buff: bool,  // 激活Buff
}

impl Default for GuildRankOptions {
    fn default() -> Self {
        Self {
            can_recruit: false,
            can_kick: false,
            can_edit_ranks: false,
            can_store_items: true,
            can_retrieve_items: false,
            can_alter_alliance: false,
            can_change_notice: false,
            can_activate_buff: false,
        }
    }
}

/// 公会职位
#[derive(Debug, Clone)]
pub struct GuildRank {
    pub id: u8,
    pub name: String,
    pub options: GuildRankOptions,
}

impl GuildRank {
    pub fn new(id: u8, name: String) -> Self {
        Self {
            id,
            name,
            options: GuildRankOptions::default(),
        }
    }
}

/// 公会Buff信息
#[derive(Debug, Clone)]
pub struct GuildBuff {
    pub id: u8,
    pub name: String,
    pub icon: u16,
    pub level: u8,
    pub cost: u32,           // 激活消耗
    pub duration: i32,       // 持续时间(秒)
    pub active: bool,        // 是否已激活
    pub time_remaining: i32, // 剩余时间(秒)
}

/// 公会对话框
pub struct GuildDialog {
    visible: bool,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    current_page: GuildPage,

    // 公会信息
    pub guild_name: String,
    pub level: u8,
    pub experience: u64,
    pub max_experience: u64,
    pub gold: u32,
    pub spare_points: u8, // 可用点数

    // 成员管理
    pub members: Vec<GuildMember>,
    pub max_members: usize,
    pub show_offline: bool, // 是否显示离线成员
    pub member_scroll_index: usize,

    // 公告
    pub notice: String,
    pub notice_editable: bool,
    pub notice_scroll_index: usize,

    // 仓库
    pub storage: Vec<Option<UserItem>>, // 112个槽位
    pub storage_scroll_index: usize,

    // 权限
    pub ranks: Vec<GuildRank>,
    pub my_rank_id: u8,
    pub my_options: GuildRankOptions,

    // Buff系统
    pub buffs: Vec<GuildBuff>,
    pub enabled_buffs: Vec<u8>, // 已激活的Buff ID列表

    // 状态
    pub voting: bool, // 是否正在投票
}

impl GuildDialog {
    /// 创建新的公会对话框
    pub fn new() -> Self {
        Self {
            visible: false,
            x: 250,
            y: 150,
            width: 500,
            height: 450,
            current_page: GuildPage::Notice,
            guild_name: String::new(),
            level: 1,
            experience: 0,
            max_experience: 1000,
            gold: 0,
            spare_points: 0,
            members: Vec::new(),
            max_members: 50,
            show_offline: true,
            member_scroll_index: 0,
            notice: String::new(),
            notice_editable: false,
            notice_scroll_index: 0,
            storage: vec![None; 112],
            storage_scroll_index: 0,
            ranks: Vec::new(),
            my_rank_id: 0,
            my_options: GuildRankOptions::default(),
            buffs: Vec::new(),
            enabled_buffs: Vec::new(),
            voting: false,
        }
    }

    /// 切换页面
    pub fn set_page(&mut self, page: GuildPage) {
        self.current_page = page;
    }

    /// 获取当前页面
    pub fn get_page(&self) -> GuildPage {
        self.current_page
    }

    /// 添加成员
    pub fn add_member(&mut self, member: GuildMember) {
        if self.members.len() < self.max_members {
            self.members.push(member);
        }
    }

    /// 移除成员
    pub fn remove_member(&mut self, name: &str) -> bool {
        if let Some(index) = self.members.iter().position(|m| m.name == name) {
            self.members.remove(index);
            return true;
        }
        false
    }

    /// 查找成员
    pub fn find_member(&self, name: &str) -> Option<&GuildMember> {
        self.members.iter().find(|m| m.name == name)
    }

    /// 查找成员(可变)
    pub fn find_member_mut(&mut self, name: &str) -> Option<&mut GuildMember> {
        self.members.iter_mut().find(|m| m.name == name)
    }

    /// 更新成员在线状态
    pub fn set_member_online(&mut self, name: &str, online: bool) {
        if let Some(member) = self.find_member_mut(name) {
            member.online = online;
        }
    }

    /// 获取在线成员数量
    pub fn online_member_count(&self) -> usize {
        self.members.iter().filter(|m| m.online).count()
    }

    /// 获取可见成员列表
    pub fn get_visible_members(&self) -> Vec<&GuildMember> {
        if self.show_offline {
            self.members.iter().collect()
        } else {
            self.members.iter().filter(|m| m.online).collect()
        }
    }

    /// 切换显示离线成员
    pub fn toggle_show_offline(&mut self) {
        self.show_offline = !self.show_offline;
        self.member_scroll_index = 0;
    }

    /// 设置公告
    pub fn set_notice(&mut self, notice: String) {
        self.notice = notice;
    }

    /// 检查是否可以编辑公告
    pub fn can_edit_notice(&self) -> bool {
        self.my_options.can_change_notice
    }

    /// 添加职位
    pub fn add_rank(&mut self, rank: GuildRank) {
        if !self.ranks.iter().any(|r| r.id == rank.id) {
            self.ranks.push(rank);
        }
    }

    /// 更新职位
    pub fn update_rank(&mut self, rank_id: u8, options: GuildRankOptions) {
        if let Some(rank) = self.ranks.iter_mut().find(|r| r.id == rank_id) {
            rank.options = options;
        }
    }

    /// 检查权限
    pub fn has_permission(&self, check: impl Fn(&GuildRankOptions) -> bool) -> bool {
        check(&self.my_options)
    }

    /// 添加Buff
    pub fn add_buff(&mut self, buff: GuildBuff) {
        if !self.buffs.iter().any(|b| b.id == buff.id) {
            self.buffs.push(buff);
        }
    }

    /// 激活Buff
    pub fn activate_buff(&mut self, buff_id: u8) -> bool {
        if !self.my_options.can_activate_buff {
            return false;
        }
        if let Some(buff) = self.buffs.iter_mut().find(|b| b.id == buff_id) {
            if !buff.active && self.gold >= buff.cost {
                buff.active = true;
                buff.time_remaining = buff.duration;
                self.enabled_buffs.push(buff_id);
                self.gold -= buff.cost;
                return true;
            }
        }
        false
    }

    /// 更新Buff状态
    pub fn update_buff(&mut self, buff_id: u8, active: bool, time_remaining: i32) {
        if let Some(buff) = self.buffs.iter_mut().find(|b| b.id == buff_id) {
            buff.active = active;
            buff.time_remaining = time_remaining;
        }
    }

    /// 计算经验百分比
    pub fn get_exp_percent(&self) -> f32 {
        if self.max_experience == 0 {
            return 0.0;
        }
        (self.experience as f32 / self.max_experience as f32) * 100.0
    }

    /// 仓库操作
    pub fn set_storage_item(&mut self, slot: usize, item: Option<UserItem>) {
        if slot < self.storage.len() {
            self.storage[slot] = item;
        }
    }

    pub fn get_storage_item(&self, slot: usize) -> Option<&UserItem> {
        self.storage.get(slot)?.as_ref()
    }

    pub fn find_empty_storage_slot(&self) -> Option<usize> {
        self.storage.iter().position(|slot| slot.is_none())
    }
}

impl Default for GuildDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Dialog for GuildDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn update(&mut self, delta_time: f32) {
        // 更新Buff倒计时
        for buff in &mut self.buffs {
            if buff.active && buff.time_remaining > 0 {
                buff.time_remaining -= delta_time as i32;
                if buff.time_remaining <= 0 {
                    buff.active = false;
                    buff.time_remaining = 0;
                    self.enabled_buffs.retain(|&id| id != buff.id);
                }
            }
        }
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // TODO: 实际渲染逻辑
    }

    fn is_visible(&self) -> bool {
        self.visible
    }
    
    fn name(&self) -> &str {
        "GuildDialog"
    }
    
    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.width &&
        y >= self.y && y < self.y + self.height
    }
    
    fn position(&self) -> (i32, i32) {
        (self.x, self.y)
    }
    
    fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guild_member_creation() {
        let member = GuildMember::new("Player1".to_string(), 1, "Member".to_string());
        assert_eq!(member.name, "Player1");
        assert_eq!(member.rank_id, 1);
        assert!(!member.online);
    }

    #[test]
    fn test_guild_dialog_creation() {
        let dialog = GuildDialog::new();
        assert!(!dialog.is_visible());
        assert_eq!(dialog.get_page(), GuildPage::Notice);
        assert_eq!(dialog.storage.len(), 112);
    }

    #[test]
    fn test_guild_members() {
        let mut dialog = GuildDialog::new();
        
        let member1 = GuildMember::new("Alice".to_string(), 1, "Member".to_string());
        let member2 = GuildMember::new("Bob".to_string(), 2, "Elder".to_string());
        
        dialog.add_member(member1);
        dialog.add_member(member2);
        
        assert_eq!(dialog.members.len(), 2);
        assert!(dialog.find_member("Alice").is_some());
        
        dialog.remove_member("Alice");
        assert_eq!(dialog.members.len(), 1);
        assert!(dialog.find_member("Alice").is_none());
    }

    #[test]
    fn test_guild_online_status() {
        let mut dialog = GuildDialog::new();
        
        let mut member = GuildMember::new("Player".to_string(), 1, "Member".to_string());
        member.online = true;
        dialog.add_member(member);
        
        assert_eq!(dialog.online_member_count(), 1);
        
        dialog.set_member_online("Player", false);
        assert_eq!(dialog.online_member_count(), 0);
    }

    #[test]
    fn test_guild_notice() {
        let mut dialog = GuildDialog::new();
        dialog.my_options.can_change_notice = true;
        
        assert!(dialog.can_edit_notice());
        
        dialog.set_notice("Welcome!".to_string());
        assert_eq!(dialog.notice, "Welcome!");
    }

    #[test]
    fn test_guild_ranks() {
        let mut dialog = GuildDialog::new();
        
        let rank = GuildRank::new(1, "Elder".to_string());
        dialog.add_rank(rank);
        
        assert_eq!(dialog.ranks.len(), 1);
        assert_eq!(dialog.ranks[0].name, "Elder");
    }

    #[test]
    fn test_guild_permissions() {
        let mut dialog = GuildDialog::new();
        dialog.my_options.can_recruit = true;
        dialog.my_options.can_kick = false;
        
        assert!(dialog.has_permission(|opts| opts.can_recruit));
        assert!(!dialog.has_permission(|opts| opts.can_kick));
    }

    #[test]
    fn test_guild_buffs() {
        let mut dialog = GuildDialog::new();
        dialog.gold = 1000;
        dialog.my_options.can_activate_buff = true;
        
        let buff = GuildBuff {
            id: 1,
            name: "Attack Buff".to_string(),
            icon: 100,
            level: 1,
            cost: 500,
            duration: 3600,
            active: false,
            time_remaining: 0,
        };
        
        dialog.add_buff(buff);
        assert_eq!(dialog.buffs.len(), 1);
        
        assert!(dialog.activate_buff(1));
        assert_eq!(dialog.gold, 500);
        assert!(dialog.buffs[0].active);
    }

    #[test]
    fn test_guild_exp_percent() {
        let mut dialog = GuildDialog::new();
        dialog.experience = 500;
        dialog.max_experience = 1000;
        
        assert_eq!(dialog.get_exp_percent(), 50.0);
    }

    #[test]
    fn test_guild_storage() {
        let mut dialog = GuildDialog::new();
        
        let item = UserItem {
            unique_id: 3001,
            item_index: 55,
            count: 1,
            ..Default::default()
        };
        
        dialog.set_storage_item(0, Some(item.clone()));
        assert!(dialog.get_storage_item(0).is_some());
        
        let empty_slot = dialog.find_empty_storage_slot();
        assert_eq!(empty_slot, Some(1));
    }

    #[test]
    fn test_show_offline_toggle() {
        let mut dialog = GuildDialog::new();
        
        let mut member1 = GuildMember::new("Online".to_string(), 1, "Member".to_string());
        member1.online = true;
        let member2 = GuildMember::new("Offline".to_string(), 1, "Member".to_string());
        
        dialog.add_member(member1);
        dialog.add_member(member2);
        
        dialog.toggle_show_offline();
        assert!(!dialog.show_offline);
        
        let visible = dialog.get_visible_members();
        assert_eq!(visible.len(), 1);
    }
}
