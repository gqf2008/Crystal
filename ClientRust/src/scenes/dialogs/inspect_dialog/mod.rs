/// InspectDialog - 查看玩家对话框
///
/// 显示其他玩家的装备、信息，并提供交互功能
///
/// # 功能特性
/// - 显示玩家装备（14个槽位）
/// - 显示玩家基本信息（名字、等级、职业、性别、公会、恋人）
/// - 提供交互按钮（组队、好友、邮件、交易、恋人、观察）

use std::collections::HashMap;

/// 装备槽位枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquipmentSlot {
    Weapon = 0,      // 武器
    Armor = 1,       // 盔甲
    Helmet = 2,      // 头盔
    Torch = 3,       // 火把
    Necklace = 4,    // 项链
    BraceletL = 5,   // 左手镯
    BraceletR = 6,   // 右手镯
    RingL = 7,       // 左戒指
    RingR = 8,       // 右戒指
    Amulet = 9,      // 护身符
    Belt = 10,       // 腰带
    Boots = 11,      // 鞋子
    Stone = 12,      // 宝石
    Mount = 13,      // 坐骑
}

impl EquipmentSlot {
    /// 获取所有槽位
    pub fn all() -> [EquipmentSlot; 14] {
        [
            EquipmentSlot::Weapon,
            EquipmentSlot::Armor,
            EquipmentSlot::Helmet,
            EquipmentSlot::Torch,
            EquipmentSlot::Necklace,
            EquipmentSlot::BraceletL,
            EquipmentSlot::BraceletR,
            EquipmentSlot::RingL,
            EquipmentSlot::RingR,
            EquipmentSlot::Amulet,
            EquipmentSlot::Belt,
            EquipmentSlot::Boots,
            EquipmentSlot::Stone,
            EquipmentSlot::Mount,
        ]
    }
}

/// 职业枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirClass {
    Warrior = 0,
    Wizard = 1,
    Taoist = 2,
    Assassin = 3,
    Archer = 4,
}

/// 性别枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirGender {
    Male = 0,
    Female = 1,
}

/// 简化的用户物品（用于显示）
#[derive(Debug, Clone)]
pub struct UserItem {
    pub unique_id: u64,
    pub item_index: i32,
    pub name: String,
}

/// 查看玩家对话框
pub struct InspectDialog {
    /// 是否可见
    pub visible: bool,

    /// 对话框位置
    pub position: (i32, i32),

    /// 对话框大小 (Index 430)
    pub size: (i32, i32),

    /// 是否可移动
    pub movable: bool,

    /// 是否排序
    pub sort: bool,

    /// 被查看玩家的ID
    pub inspect_id: u32,

    /// 玩家名字
    pub name: String,

    /// 公会名字
    pub guild_name: String,

    /// 公会职位
    pub guild_rank: String,

    /// 职业
    pub class: MirClass,

    /// 性别
    pub gender: MirGender,

    /// 发型
    pub hair: u8,

    /// 等级
    pub level: u16,

    /// 恋人名字
    pub lover_name: String,

    /// 是否允许观察
    pub allow_observe: bool,

    /// 装备物品
    pub items: HashMap<EquipmentSlot, Option<UserItem>>,
}

impl InspectDialog {
    /// 创建新的查看玩家对话框
    pub fn new() -> Self {
        let size = (280, 400);
        let position = (536, 0);

        Self {
            visible: false,
            position,
            size,
            movable: true,
            sort: true,
            inspect_id: 0,
            name: String::new(),
            guild_name: String::new(),
            guild_rank: String::new(),
            class: MirClass::Warrior,
            gender: MirGender::Male,
            hair: 0,
            level: 1,
            lover_name: String::new(),
            allow_observe: false,
            items: HashMap::new(),
        }
    }

    /// 显示对话框并设置玩家信息
    pub fn show_player(&mut self, inspect_id: u32, name: String, level: u16,
                        class: MirClass, gender: MirGender, hair: u8) {
        self.inspect_id = inspect_id;
        self.name = name;
        self.level = level;
        self.class = class;
        self.gender = gender;
        self.hair = hair;
        self.visible = true;

        // 清空装备
        self.items.clear();
    }

    /// 隐藏对话框
    pub fn hide(&mut self) {
        self.visible = false;
        self.items.clear();
    }

    /// 检查是否可见
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// 设置装备
    pub fn set_equipment(&mut self, slot: EquipmentSlot, item: Option<UserItem>) {
        self.items.insert(slot, item);
    }

    /// 获取装备
    pub fn get_equipment(&self, slot: EquipmentSlot) -> Option<&UserItem> {
        self.items.get(&slot).and_then(|opt| opt.as_ref())
    }

    /// 设置公会信息
    pub fn set_guild(&mut self, guild_name: String, guild_rank: String) {
        self.guild_name = guild_name;
        self.guild_rank = guild_rank;
    }

    /// 设置恋人名字
    pub fn set_lover(&mut self, lover_name: String) {
        self.lover_name = lover_name;
    }

    /// 设置观察权限
    pub fn set_allow_observe(&mut self, allow: bool) {
        self.allow_observe = allow;
    }

    /// 获取显示文本
    pub fn get_name_text(&self) -> String {
        format!("{} Lv.{}", self.name, self.level)
    }

    /// 获取公会文本
    pub fn get_guild_text(&self) -> String {
        if self.guild_name.is_empty() {
            "No Guild".to_string()
        } else if self.guild_rank.is_empty() {
            self.guild_name.clone()
        } else {
            format!("{} [{}]", self.guild_name, self.guild_rank)
        }
    }

    /// 获取恋人文本
    pub fn get_lover_text(&self) -> String {
        if self.lover_name.is_empty() {
            "Single".to_string()
        } else {
            format!("♥ {}", self.lover_name)
        }
    }

    /// 获取职业文本
    pub fn get_class_text(&self) -> &str {
        match self.class {
            MirClass::Warrior => "Warrior",
            MirClass::Wizard => "Wizard",
            MirClass::Taoist => "Taoist",
            MirClass::Assassin => "Assassin",
            MirClass::Archer => "Archer",
        }
    }

    /// 获取性别文本
    pub fn get_gender_text(&self) -> &str {
        match self.gender {
            MirGender::Male => "Male",
            MirGender::Female => "Female",
        }
    }

    /// 鼠标点击事件处理
    ///
    /// # Returns
    /// 点击的按钮类型
    pub fn on_mouse_click(&mut self, x: i32, y: i32) -> Option<InspectAction> {
        if !self.visible {
            return None;
        }

        // 关闭按钮 (241, 3)
        let close_x = self.position.0 + 241;
        let close_y = self.position.1 + 3;
        if x >= close_x && x < close_x + 20 && y >= close_y && y < close_y + 20 {
            self.hide();
            return Some(InspectAction::Close);
        }

        let base_y = self.position.1 + 357;

        // 组队按钮 (55, 357)
        let group_x = self.position.0 + 55;
        if x >= group_x && x < group_x + 28 && y >= base_y && y < base_y + 28 {
            return Some(InspectAction::InviteToGroup);
        }

        // 好友按钮 (85, 357)
        let friend_x = self.position.0 + 85;
        if x >= friend_x && x < friend_x + 28 && y >= base_y && y < base_y + 28 {
            return Some(InspectAction::AddFriend);
        }

        // 邮件按钮 (115, 357)
        let mail_x = self.position.0 + 115;
        if x >= mail_x && x < mail_x + 28 && y >= base_y && y < base_y + 28 {
            return Some(InspectAction::SendMail);
        }

        // 交易按钮 (145, 357)
        let trade_x = self.position.0 + 145;
        if x >= trade_x && x < trade_x + 28 && y >= base_y && y < base_y + 28 {
            return Some(InspectAction::RequestTrade);
        }

        // 观察按钮 (175, 357)
        let observe_x = self.position.0 + 175;
        if x >= observe_x && x < observe_x + 28 && y >= base_y && y < base_y + 28 {
            if self.allow_observe {
                return Some(InspectAction::Observe);
            } else {
                return Some(InspectAction::ObserveDisabled);
            }
        }

        None
    }
}

/// 查看对话框交互动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectAction {
    Close,
    InviteToGroup,
    AddFriend,
    SendMail,
    RequestTrade,
    Observe,
    ObserveDisabled,
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspect_dialog_creation() {
        let dialog = InspectDialog::new();

        assert!(!dialog.visible);
        assert_eq!(dialog.name, "");
        assert_eq!(dialog.level, 1);
    }

    #[test]
    fn test_show_player() {
        let mut dialog = InspectDialog::new();

        dialog.show_player(
            12345,
            "TestPlayer".to_string(),
            50,
            MirClass::Wizard,
            MirGender::Female,
            3
        );

        assert!(dialog.visible);
        assert_eq!(dialog.inspect_id, 12345);
        assert_eq!(dialog.name, "TestPlayer");
        assert_eq!(dialog.level, 50);
        assert_eq!(dialog.class, MirClass::Wizard);
        assert_eq!(dialog.gender, MirGender::Female);
        assert_eq!(dialog.hair, 3);
    }

    #[test]
    fn test_equipment_management() {
        let mut dialog = InspectDialog::new();

        let weapon = UserItem {
            unique_id: 1,
            item_index: 100,
            name: "Legendary Sword".to_string(),
        };

        dialog.set_equipment(EquipmentSlot::Weapon, Some(weapon.clone()));

        let equipped = dialog.get_equipment(EquipmentSlot::Weapon);
        assert!(equipped.is_some());
        assert_eq!(equipped.unwrap().name, "Legendary Sword");

        // 空槽位
        let empty = dialog.get_equipment(EquipmentSlot::Armor);
        assert!(empty.is_none());
    }

    #[test]
    fn test_guild_info() {
        let mut dialog = InspectDialog::new();

        assert_eq!(dialog.get_guild_text(), "No Guild");

        dialog.set_guild("Warriors".to_string(), "Leader".to_string());
        assert_eq!(dialog.get_guild_text(), "Warriors [Leader]");

        dialog.set_guild("Mages".to_string(), String::new());
        assert_eq!(dialog.get_guild_text(), "Mages");
    }

    #[test]
    fn test_lover_info() {
        let mut dialog = InspectDialog::new();

        assert_eq!(dialog.get_lover_text(), "Single");

        dialog.set_lover("Alice".to_string());
        assert_eq!(dialog.get_lover_text(), "♥ Alice");
    }

    #[test]
    fn test_display_texts() {
        let mut dialog = InspectDialog::new();
        dialog.show_player(
            1,
            "Hero".to_string(),
            99,
            MirClass::Warrior,
            MirGender::Male,
            0
        );

        assert_eq!(dialog.get_name_text(), "Hero Lv.99");
        assert_eq!(dialog.get_class_text(), "Warrior");
        assert_eq!(dialog.get_gender_text(), "Male");
    }

    #[test]
    fn test_all_equipment_slots() {
        let slots = EquipmentSlot::all();
        assert_eq!(slots.len(), 14);
        assert_eq!(slots[0], EquipmentSlot::Weapon);
        assert_eq!(slots[13], EquipmentSlot::Mount);
    }

    #[test]
    fn test_observe_permission() {
        let mut dialog = InspectDialog::new();

        assert!(!dialog.allow_observe);

        dialog.set_allow_observe(true);
        assert!(dialog.allow_observe);
    }
}