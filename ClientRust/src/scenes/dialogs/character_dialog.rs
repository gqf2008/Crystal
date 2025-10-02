// Character Dialog - 角色面板对话框
// 显示角色装备、属性、状态、技能等信息

use super::Dialog;
use crate::network::protocol::UserItem;

/// 角色页面类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterPage {
    Character, // 角色/装备页
    Status,    // 战斗属性页
    State,     // 状态属性页
    Skill,     // 技能页
}

/// 装备槽位枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentSlot {
    Weapon = 0,      // 武器
    Armour = 1,      // 护甲
    Helmet = 2,      // 头盔
    Torch = 3,       // 火把/照明
    Necklace = 4,    // 项链
    BraceletL = 5,   // 左手镯
    BraceletR = 6,   // 右手镯
    RingL = 7,       // 左戒指
    RingR = 8,       // 右戒指
    Amulet = 9,      // 护身符
    Belt = 10,       // 腰带
    Boots = 11,      // 靴子
    Stone = 12,      // 宝石/符文
    Mount = 13,      // 坐骑
}

impl EquipmentSlot {
    /// 总装备槽位数量
    pub const COUNT: usize = 14;
}

/// 角色统计数据
#[derive(Debug, Clone, Default)]
pub struct CharacterStats {
    // 基础属性
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub experience: u64,
    pub max_experience: u64,

    // 攻击属性
    pub min_dc: i32, // 最小物理攻击
    pub max_dc: i32, // 最大物理攻击
    pub min_mc: i32, // 最小魔法攻击
    pub max_mc: i32, // 最大魔法攻击
    pub min_sc: i32, // 最小道术攻击
    pub max_sc: i32, // 最大道术攻击

    // 防御属性
    pub min_ac: i32, // 最小物理防御
    pub max_ac: i32, // 最大物理防御
    pub min_mac: i32, // 最小魔法防御
    pub max_mac: i32, // 最大魔法防御

    // 高级属性
    pub critical_rate: i32,   // 暴击率 (%)
    pub critical_damage: i32, // 暴击伤害
    pub attack_speed: i32,    // 攻击速度
    pub accuracy: i32,        // 准确度
    pub agility: i32,         // 敏捷度
    pub luck: i32,            // 幸运值

    // 状态属性
    pub bag_weight: i32,        // 背包负重
    pub current_bag_weight: i32, // 当前背包重量
    pub wear_weight: i32,       // 装备负重
    pub current_wear_weight: i32, // 当前装备重量
    pub hand_weight: i32,       // 手持负重
    pub current_hand_weight: i32, // 当前手持重量

    // 抗性属性
    pub magic_resist: i32,   // 魔法抗性
    pub poison_resist: i32,  // 毒素抗性
    pub poison_recovery: i32, // 毒素恢复
    pub health_recovery: i32, // 生命恢复
    pub mana_recovery: i32,   // 魔法恢复
    pub holy: i32,           // 神圣值
    pub freezing: i32,       // 冰冻值
    pub poison_attack: i32,  // 毒素攻击
}

/// 角色对话框
pub struct CharacterDialog {
    visible: bool,
    current_page: CharacterPage,

    // 角色信息
    pub player_name: String,
    pub guild_name: String,
    pub lover_name: String,
    pub level: u16,
    pub class: String,
    pub gender: String,
    pub hair: u8,

    // 装备栏 (14个槽位)
    pub equipment: Vec<Option<UserItem>>,

    // 角色统计数据
    pub stats: CharacterStats,

    // 魔法列表
    pub magics: Vec<MagicInfo>,
}

/// 魔法信息
#[derive(Debug, Clone)]
pub struct MagicInfo {
    pub spell: String,
    pub name: String,
    pub level: u8,
    pub key: u8,        // 快捷键
    pub icon: u16,      // 图标索引
    pub base_cost: i32, // 基础消耗
    pub level_cost: i32, // 等级消耗
    pub delay: i32,     // 冷却时间(ms)
}

impl CharacterDialog {
    /// 创建新的角色对话框
    pub fn new() -> Self {
        Self {
            visible: false,
            current_page: CharacterPage::Character,
            player_name: String::new(),
            guild_name: String::new(),
            lover_name: String::new(),
            level: 1,
            class: String::from("Warrior"),
            gender: String::from("Male"),
            hair: 0,
            equipment: vec![None; EquipmentSlot::COUNT],
            stats: CharacterStats::default(),
            magics: Vec::new(),
        }
    }

    /// 切换页面
    pub fn set_page(&mut self, page: CharacterPage) {
        self.current_page = page;
    }

    /// 获取当前页面
    pub fn get_page(&self) -> CharacterPage {
        self.current_page
    }

    /// 设置装备
    pub fn set_equipment(&mut self, slot: EquipmentSlot, item: Option<UserItem>) {
        let index = slot as usize;
        if index < self.equipment.len() {
            self.equipment[index] = item;
        }
    }

    /// 获取装备
    pub fn get_equipment(&self, slot: EquipmentSlot) -> Option<&UserItem> {
        let index = slot as usize;
        self.equipment.get(index).and_then(|opt| opt.as_ref())
    }

    /// 更新角色统计数据
    pub fn update_stats(&mut self, stats: CharacterStats) {
        self.stats = stats;
    }

    /// 添加魔法
    pub fn add_magic(&mut self, magic: MagicInfo) {
        // 检查是否已存在
        if let Some(existing) = self.magics.iter_mut().find(|m| m.spell == magic.spell) {
            *existing = magic;
        } else {
            self.magics.push(magic);
        }
    }

    /// 移除魔法
    pub fn remove_magic(&mut self, spell: &str) {
        self.magics.retain(|m| m.spell != spell);
    }

    /// 获取魔法列表
    pub fn get_magics(&self) -> &[MagicInfo] {
        &self.magics
    }

    /// 计算经验百分比
    pub fn get_exp_percent(&self) -> f32 {
        if self.stats.max_experience == 0 {
            return 0.0;
        }
        (self.stats.experience as f32 / self.stats.max_experience as f32) * 100.0
    }

    /// 计算背包负重百分比
    pub fn get_bag_weight_percent(&self) -> f32 {
        if self.stats.bag_weight == 0 {
            return 0.0;
        }
        (self.stats.current_bag_weight as f32 / self.stats.bag_weight as f32) * 100.0
    }
}

impl Default for CharacterDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Dialog for CharacterDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn update(&mut self, _delta_time: f32) {
        // 更新逻辑 (如冷却时间等)
        // TODO: 实现魔法冷却更新
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // TODO: 实际渲染逻辑
        // 绘制对话框背景、装备槽、属性文本等
    }

    fn is_visible(&self) -> bool {
        self.visible
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_character_dialog_creation() {
        let dialog = CharacterDialog::new();
        assert_eq!(dialog.is_visible(), false);
        assert_eq!(dialog.get_page(), CharacterPage::Character);
        assert_eq!(dialog.equipment.len(), EquipmentSlot::COUNT);
    }

    #[test]
    fn test_page_switching() {
        let mut dialog = CharacterDialog::new();
        dialog.set_page(CharacterPage::Status);
        assert_eq!(dialog.get_page(), CharacterPage::Status);
        dialog.set_page(CharacterPage::Skill);
        assert_eq!(dialog.get_page(), CharacterPage::Skill);
    }

    #[test]
    fn test_equipment_management() {
        let mut dialog = CharacterDialog::new();
        
        // 测试设置装备
        let item = UserItem {
            unique_id: 1001,
            item_index: 42,
            current_dura: 1000,
            max_dura: 1000,
            count: 1,
            ..Default::default()
        };
        
        dialog.set_equipment(EquipmentSlot::Weapon, Some(item.clone()));
        
        // 验证装备
        let equipped = dialog.get_equipment(EquipmentSlot::Weapon);
        assert!(equipped.is_some());
        assert_eq!(equipped.unwrap().unique_id, 1001);
        
        // 测试卸下装备
        dialog.set_equipment(EquipmentSlot::Weapon, None);
        assert!(dialog.get_equipment(EquipmentSlot::Weapon).is_none());
    }

    #[test]
    fn test_magic_management() {
        let mut dialog = CharacterDialog::new();
        
        let magic = MagicInfo {
            spell: "Fireball".to_string(),
            name: "火球术".to_string(),
            level: 3,
            key: 1,
            icon: 5,
            base_cost: 10,
            level_cost: 2,
            delay: 1000,
        };
        
        dialog.add_magic(magic.clone());
        assert_eq!(dialog.get_magics().len(), 1);
        
        // 添加相同法术应该更新
        let mut updated_magic = magic.clone();
        updated_magic.level = 5;
        dialog.add_magic(updated_magic);
        assert_eq!(dialog.get_magics().len(), 1);
        assert_eq!(dialog.get_magics()[0].level, 5);
        
        // 移除法术
        dialog.remove_magic("Fireball");
        assert_eq!(dialog.get_magics().len(), 0);
    }

    #[test]
    fn test_stat_calculations() {
        let mut dialog = CharacterDialog::new();
        dialog.stats.experience = 5000;
        dialog.stats.max_experience = 10000;
        assert_eq!(dialog.get_exp_percent(), 50.0);
        
        dialog.stats.current_bag_weight = 75;
        dialog.stats.bag_weight = 100;
        assert_eq!(dialog.get_bag_weight_percent(), 75.0);
    }
}
