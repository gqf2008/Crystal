// Skill Bar Dialog - 技能栏对话框
// 显示8个快捷技能格子，支持多个技能栏

use super::Dialog;
use std::time::{Duration, Instant};

/// 技能栏中的技能槽位
#[derive(Debug, Clone)]
pub struct SkillSlot {
    pub key: u8,         // 快捷键编号 (1-8)
    pub spell: String,   // 法术名称
    pub name: String,    // 显示名称
    pub icon: u16,       // 图标索引
    pub level: u8,       // 技能等级
    pub base_cost: i32,  // 基础MP消耗
    pub level_cost: i32, // 等级MP消耗
    pub delay: i32,      // 冷却时间(ms)
    pub last_cast_time: Option<Instant>, // 上次施放时间
}

impl SkillSlot {
    /// 创建空技能槽
    pub fn empty(key: u8) -> Self {
        Self {
            key,
            spell: String::new(),
            name: String::new(),
            icon: 0,
            level: 0,
            base_cost: 0,
            level_cost: 0,
            delay: 0,
            last_cast_time: None,
        }
    }

    /// 是否为空槽
    pub fn is_empty(&self) -> bool {
        self.spell.is_empty()
    }

    /// 计算总MP消耗
    pub fn total_cost(&self) -> i32 {
        self.base_cost + (self.level_cost * self.level as i32)
    }

    /// 检查是否在冷却中
    pub fn is_on_cooldown(&self) -> bool {
        if let Some(last_cast) = self.last_cast_time {
            let elapsed = Instant::now().duration_since(last_cast);
            elapsed < Duration::from_millis(self.delay as u64)
        } else {
            false
        }
    }

    /// 获取剩余冷却时间(ms)
    pub fn get_cooldown_remaining(&self) -> i32 {
        if let Some(last_cast) = self.last_cast_time {
            let elapsed = Instant::now().duration_since(last_cast);
            let elapsed_ms = elapsed.as_millis() as i32;
            (self.delay - elapsed_ms).max(0)
        } else {
            0
        }
    }

    /// 获取冷却百分比 (0-100)
    pub fn get_cooldown_percent(&self) -> f32 {
        if self.delay == 0 {
            return 0.0;
        }
        let remaining = self.get_cooldown_remaining();
        (remaining as f32 / self.delay as f32) * 100.0
    }

    /// 重置冷却
    pub fn reset_cooldown(&mut self) {
        self.last_cast_time = None;
    }

    /// 开始冷却
    pub fn start_cooldown(&mut self) {
        self.last_cast_time = Some(Instant::now());
    }
}

/// 技能栏对话框
pub struct SkillBarDialog {
    visible: bool,
    pub bar_index: u8,   // 技能栏索引 (0=Bar1, 1=Bar2)
    pub slots: Vec<SkillSlot>, // 8个技能槽

    // 位置和配置
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub movable: bool,
}

impl SkillBarDialog {
    /// 创建新的技能栏
    pub fn new(bar_index: u8) -> Self {
        let mut slots = Vec::with_capacity(8);
        for i in 0..8 {
            slots.push(SkillSlot::empty((bar_index * 8 + i + 1) as u8));
        }

        Self {
            visible: false,
            bar_index,
            slots,
            x: 0,
            y: bar_index as i32 * 28, // 每个技能栏垂直间隔28像素
            width: 400,
            height: 28,
            movable: true,
        }
    }

    /// 设置技能到槽位
    pub fn set_skill(
        &mut self,
        slot_index: usize,
        spell: String,
        name: String,
        icon: u16,
        level: u8,
        base_cost: i32,
        level_cost: i32,
        delay: i32,
    ) {
        if slot_index < self.slots.len() {
            let slot = &mut self.slots[slot_index];
            slot.spell = spell;
            slot.name = name;
            slot.icon = icon;
            slot.level = level;
            slot.base_cost = base_cost;
            slot.level_cost = level_cost;
            slot.delay = delay;
        }
    }

    /// 清空槽位
    pub fn clear_slot(&mut self, slot_index: usize) {
        if slot_index < self.slots.len() {
            let key = self.slots[slot_index].key;
            self.slots[slot_index] = SkillSlot::empty(key);
        }
    }

    /// 清空所有槽位
    pub fn clear_all_slots(&mut self) {
        for i in 0..self.slots.len() {
            self.clear_slot(i);
        }
    }

    /// 获取槽位
    pub fn get_slot(&self, slot_index: usize) -> Option<&SkillSlot> {
        self.slots.get(slot_index)
    }

    /// 获取槽位(可变)
    pub fn get_slot_mut(&mut self, slot_index: usize) -> Option<&mut SkillSlot> {
        self.slots.get_mut(slot_index)
    }

    /// 按快捷键查找槽位
    pub fn find_slot_by_key(&self, key: u8) -> Option<&SkillSlot> {
        self.slots.iter().find(|slot| slot.key == key)
    }

    /// 按快捷键查找槽位(可变)
    pub fn find_slot_by_key_mut(&mut self, key: u8) -> Option<&mut SkillSlot> {
        self.slots.iter_mut().find(|slot| slot.key == key)
    }

    /// 按法术名查找槽位索引
    pub fn find_slot_by_spell(&self, spell: &str) -> Option<usize> {
        self.slots.iter().position(|slot| slot.spell == spell)
    }

    /// 检查是否有技能
    pub fn has_skills(&self) -> bool {
        self.slots.iter().any(|slot| !slot.is_empty())
    }

    /// 使用槽位中的技能
    pub fn use_skill(&mut self, slot_index: usize) -> Option<String> {
        if let Some(slot) = self.get_slot_mut(slot_index) {
            if !slot.is_empty() && !slot.is_on_cooldown() {
                slot.start_cooldown();
                return Some(slot.spell.clone());
            }
        }
        None
    }

    /// 获取快捷键名称
    pub fn get_key_name(&self, slot_index: usize) -> String {
        // 根据槽位索引返回快捷键名称
        // Bar1: F1-F8, Bar2: Ctrl+F1-F8
        if self.bar_index == 0 {
            format!("F{}", slot_index + 1)
        } else {
            format!("Ctrl+F{}", slot_index + 1)
        }
    }

    /// 设置位置
    pub fn set_position(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    /// 获取位置
    pub fn get_position(&self) -> (i32, i32) {
        (self.x, self.y)
    }
}

impl Dialog for SkillBarDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn update(&mut self, _delta_time: f32) {
        // 更新冷却时间等
        // 冷却状态会在is_on_cooldown()中实时检查
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // TODO: 实际渲染逻辑
        // 绘制技能栏背景、技能图标、冷却遮罩、快捷键文本等
    }

    fn is_visible(&self) -> bool {
        self.visible
    }
    
    fn name(&self) -> &str {
        "SkillBarDialog"
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
    fn test_skill_slot_creation() {
        let slot = SkillSlot::empty(1);
        assert_eq!(slot.key, 1);
        assert!(slot.is_empty());
        assert_eq!(slot.total_cost(), 0);
    }

    #[test]
    fn test_skill_slot_cost() {
        let mut slot = SkillSlot::empty(1);
        slot.base_cost = 10;
        slot.level_cost = 2;
        slot.level = 5;
        assert_eq!(slot.total_cost(), 20); // 10 + 2*5
    }

    #[test]
    fn test_skill_cooldown() {
        let mut slot = SkillSlot::empty(1);
        slot.delay = 1000; // 1秒冷却
        
        assert!(!slot.is_on_cooldown());
        
        slot.start_cooldown();
        assert!(slot.is_on_cooldown());
        
        let remaining = slot.get_cooldown_remaining();
        assert!(remaining > 0 && remaining <= 1000);
        
        slot.reset_cooldown();
        assert!(!slot.is_on_cooldown());
    }

    #[test]
    fn test_skillbar_creation() {
        let bar = SkillBarDialog::new(0);
        assert_eq!(bar.bar_index, 0);
        assert_eq!(bar.slots.len(), 8);
        assert_eq!(bar.slots[0].key, 1);
        assert_eq!(bar.slots[7].key, 8);
        assert!(!bar.has_skills());
    }

    #[test]
    fn test_skillbar_set_skill() {
        let mut bar = SkillBarDialog::new(0);
        
        bar.set_skill(
            0,
            "Fireball".to_string(),
            "火球术".to_string(),
            5,
            3,
            10,
            2,
            1000,
        );
        
        assert!(bar.has_skills());
        let slot = bar.get_slot(0).unwrap();
        assert_eq!(slot.spell, "Fireball");
        assert_eq!(slot.icon, 5);
        assert_eq!(slot.total_cost(), 16); // 10 + 2*3
    }

    #[test]
    fn test_skillbar_clear_slot() {
        let mut bar = SkillBarDialog::new(0);
        bar.set_skill(0, "Fireball".to_string(), "火球术".to_string(), 5, 3, 10, 2, 1000);
        
        assert!(bar.has_skills());
        bar.clear_slot(0);
        assert!(!bar.has_skills());
        assert!(bar.get_slot(0).unwrap().is_empty());
    }

    #[test]
    fn test_skillbar_find_by_key() {
        let mut bar = SkillBarDialog::new(0);
        bar.set_skill(2, "Lightning".to_string(), "闪电术".to_string(), 8, 2, 15, 3, 800);
        
        let slot = bar.find_slot_by_key(3); // Key 3 = slot index 2
        assert!(slot.is_some());
        assert_eq!(slot.unwrap().spell, "Lightning");
    }

    #[test]
    fn test_skillbar_find_by_spell() {
        let mut bar = SkillBarDialog::new(0);
        bar.set_skill(4, "Healing".to_string(), "治疗术".to_string(), 10, 5, 20, 4, 1500);
        
        let index = bar.find_slot_by_spell("Healing");
        assert_eq!(index, Some(4));
    }

    #[test]
    fn test_skillbar_use_skill() {
        let mut bar = SkillBarDialog::new(0);
        bar.set_skill(0, "Fireball".to_string(), "火球术".to_string(), 5, 3, 10, 2, 1000);
        
        // 第一次使用
        let result = bar.use_skill(0);
        assert_eq!(result, Some("Fireball".to_string()));
        
        // 冷却中，不能使用
        let result = bar.use_skill(0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_skillbar_key_names() {
        let bar1 = SkillBarDialog::new(0);
        assert_eq!(bar1.get_key_name(0), "F1");
        assert_eq!(bar1.get_key_name(7), "F8");
        
        let bar2 = SkillBarDialog::new(1);
        assert_eq!(bar2.get_key_name(0), "Ctrl+F1");
        assert_eq!(bar2.get_key_name(7), "Ctrl+F8");
    }

    #[test]
    fn test_skillbar_position() {
        let mut bar = SkillBarDialog::new(1);
        bar.set_position(100, 200);
        assert_eq!(bar.get_position(), (100, 200));
    }
}
