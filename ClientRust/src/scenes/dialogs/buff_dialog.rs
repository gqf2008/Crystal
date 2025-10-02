// Buff Dialog - Buff/Debuff显示对话框
// 显示角色当前的增益/减益效果

use std::collections::HashMap;

/// Buff类型 (部分，实际游戏中有更多类型)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuffType {
    // 技能Buff
    Fury,
    Rage,
    ImmortalSkin,
    CounterAttack,
    MagicBooster,
    MagicShield,
    Hiding,
    Haste,
    SoulShield,
    BlessedArmour,
    ProtectionField,
    UltimateEnhancer,
    Curse,
    EnergyShield,
    SwiftFeet,
    LightBody,
    MoonLight,
    DarkBody,
    Concentration,
    VampireShot,
    PoisonShot,
    MentalState,
    
    // 怪物Debuff
    RhinoPriestDebuff,
    Blindness,
    
    // 特殊Buff
    GameMaster,
    General,
    Exp,
    Drop,
    Gold,
    Knapsack,
    BagWeight,
    Transform,
    Mentor,
    Mentee,
    Lover,
    Guild,
    Rested,
    TemporalFlux,
    Skill,
    Newbie,
    
    // 属性Buff
    Impact,
    Magic,
    Taoist,
    Storm,
    HealthAid,
    ManaAid,
    Defence,
    MagicDefence,
    WonderDrug,
}

/// Buff数据
#[derive(Debug, Clone)]
pub struct ClientBuff {
    /// Buff类型
    pub buff_type: BuffType,
    /// 释放者名称
    pub caster: String,
    /// Buff数值 (如增加的攻击力等)
    pub values: Vec<i32>,
    /// Buff属性加成 (StatType -> value)
    pub stats: HashMap<String, i32>,
    /// 过期时间 (毫秒时间戳)
    pub expire_time: i64,
    /// 是否暂停
    pub paused: bool,
    /// 是否永久
    pub infinite: bool,
}

impl ClientBuff {
    /// 创建新Buff
    pub fn new(buff_type: BuffType, caster: String, expire_time: i64) -> Self {
        Self {
            buff_type,
            caster,
            values: Vec::new(),
            stats: HashMap::new(),
            expire_time,
            paused: false,
            infinite: false,
        }
    }

    /// 获取剩余时间 (秒)
    pub fn get_remaining_seconds(&self, current_time_ms: i64) -> i64 {
        if self.infinite || self.paused {
            return 0;
        }
        ((self.expire_time - current_time_ms) / 1000).max(0)
    }

    /// 检查是否已过期
    pub fn is_expired(&self, current_time_ms: i64) -> bool {
        if self.infinite || self.paused {
            return false;
        }
        current_time_ms >= self.expire_time
    }

    /// 检查是否即将过期 (<=5秒)
    pub fn is_expiring_soon(&self, current_time_ms: i64) -> bool {
        !self.paused && !self.infinite && self.get_remaining_seconds(current_time_ms) <= 5
    }
}

/// Buff对话框
/// 
/// 功能:
/// - 显示当前所有Buff
/// - 自动淡入淡出
/// - 展开/收起模式
/// - Buff过期闪烁提示
#[derive(Debug, Clone)]
pub struct BuffDialog {
    /// Buff列表
    pub buffs: Vec<ClientBuff>,
    /// 是否展开显示
    pub expanded: bool,
    /// 是否可见
    pub visible: bool,
    /// 透明度 (0.0-1.0)
    pub opacity: f32,
    /// 是否已淡出
    pub faded_out: bool,
    /// 是否已淡入
    pub faded_in: bool,
    /// 下次淡入淡出时间 (毫秒)
    pub next_fade_time: i64,
    /// 对话框位置
    pub position: (i32, i32),
    /// 对话框大小
    pub size: (i32, i32),
    /// 每行最多显示Buff数量
    pub buffs_per_row: usize,
}

impl Default for BuffDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl BuffDialog {
    /// 淡入淡出延迟 (毫秒)
    const FADE_DELAY: i64 = 55;
    /// 淡入淡出速率
    const FADE_RATE: f32 = 0.2;

    /// 创建新的Buff对话框
    pub fn new() -> Self {
        // C#: Location = new Point(Settings.ScreenWidth - 170, 0)
        Self {
            buffs: Vec::new(),
            expanded: false,
            visible: true,
            opacity: 0.0,
            faded_out: true,
            faded_in: false,
            next_fade_time: 0,
            position: (1110, 0), // 假设1280宽屏幕
            size: (44, 34), // 收起状态大小
            buffs_per_row: 10,
        }
    }

    /// 添加Buff
    pub fn add_buff(&mut self, buff: ClientBuff) {
        self.buffs.push(buff);
        self.update_window();
    }

    /// 移除Buff
    pub fn remove_buff(&mut self, index: usize) -> Option<ClientBuff> {
        if index < self.buffs.len() {
            let buff = self.buffs.remove(index);
            self.update_window();
            Some(buff)
        } else {
            None
        }
    }

    /// 根据类型移除Buff
    pub fn remove_buff_by_type(&mut self, buff_type: BuffType) -> Option<ClientBuff> {
        if let Some(index) = self.buffs.iter().position(|b| b.buff_type == buff_type) {
            self.remove_buff(index)
        } else {
            None
        }
    }

    /// 清空所有Buff
    pub fn clear(&mut self) {
        self.buffs.clear();
        self.update_window();
    }

    /// 切换展开/收起
    pub fn toggle_expand(&mut self) {
        if self.buffs.len() == 1 {
            self.expanded = true;
        } else {
            self.expanded = !self.expanded;
        }
        self.update_window();
    }

    /// 更新对话框窗口大小
    fn update_window(&mut self) {
        let buff_count = self.buffs.len();

        if buff_count == 0 {
            self.size = (44, 34);
            return;
        }

        if self.expanded {
            // 展开模式: 显示所有Buff
            let cols = buff_count.min(self.buffs_per_row);
            let rows = (buff_count + self.buffs_per_row - 1) / self.buffs_per_row;
            
            let width = cols * 23;
            let height = 24 + (rows * 24);
            
            self.size = (width as i32, height as i32);
        } else {
            // 收起模式: 只显示一个Buff图标和数量
            self.size = (44, 34);
        }
    }

    /// 处理淡入淡出 (每帧调用)
    pub fn process(&mut self, current_time_ms: i64, mouse_over: bool) {
        // 移除已过期的Buff
        self.buffs.retain(|b| !b.is_expired(current_time_ms));
        
        if self.buffs.is_empty() {
            self.visible = false;
            return;
        }

        self.visible = true;

        if mouse_over {
            // 鼠标悬停: 淡入
            if self.buffs.is_empty() || (!self.faded_in && current_time_ms <= self.next_fade_time) {
                return;
            }

            self.opacity += Self::FADE_RATE;
            if self.opacity > 1.0 {
                self.opacity = 1.0;
                self.faded_in = true;
                self.faded_out = false;
            }

            self.next_fade_time = current_time_ms + Self::FADE_DELAY;
        } else {
            // 鼠标离开: 淡出
            if !self.faded_out && current_time_ms <= self.next_fade_time {
                return;
            }

            self.opacity -= Self::FADE_RATE;
            if self.opacity < 0.0 {
                self.opacity = 0.0;
                self.faded_out = true;
                self.faded_in = false;
            }

            self.next_fade_time = current_time_ms + Self::FADE_DELAY;
        }
    }

    /// 获取Buff在对话框中的位置
    pub fn get_buff_position(&self, index: usize) -> Option<(i32, i32)> {
        if index >= self.buffs.len() {
            return None;
        }

        if !self.expanded {
            // 收起模式: 所有Buff叠在一起，只显示第一个
            if index == 0 {
                return Some((self.position.0 + 10, self.position.1 + 6));
            } else {
                return None;
            }
        }

        // 展开模式: 计算网格位置
        let col = index % self.buffs_per_row;
        let row = index / self.buffs_per_row;
        
        let x = self.position.0 + self.size.0 - 10 - 23 - (col as i32 * 23) + ((self.buffs_per_row as i32 * 23) * (row as i32));
        let y = self.position.1 + 6 + (row as i32 * 24);
        
        Some((x, y))
    }

    /// 获取Buff数量
    pub fn buff_count(&self) -> usize {
        self.buffs.len()
    }

    /// 根据类型查找Buff
    pub fn find_buff(&self, buff_type: BuffType) -> Option<&ClientBuff> {
        self.buffs.iter().find(|b| b.buff_type == buff_type)
    }

    /// 根据类型查找Buff (可变)
    pub fn find_buff_mut(&mut self, buff_type: BuffType) -> Option<&mut ClientBuff> {
        self.buffs.iter_mut().find(|b| b.buff_type == buff_type)
    }

    /// 检查是否有指定Buff
    pub fn has_buff(&self, buff_type: BuffType) -> bool {
        self.find_buff(buff_type).is_some()
    }

    /// 获取Buff剩余时间文本
    pub fn get_buff_time_text(&self, buff: &ClientBuff, current_time_ms: i64) -> String {
        if buff.paused {
            "Paused".to_string()
        } else if buff.infinite {
            "Permanent".to_string()
        } else {
            let seconds = buff.get_remaining_seconds(current_time_ms);
            format_duration(seconds)
        }
    }
}

/// 格式化持续时间 (秒 -> "HH:MM:SS")
fn format_duration(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{:02}:{:02}", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_buff(buff_type: BuffType, expire_time: i64) -> ClientBuff {
        ClientBuff::new(buff_type, "TestCaster".to_string(), expire_time)
    }

    #[test]
    fn test_client_buff_new() {
        let buff = create_test_buff(BuffType::Haste, 10000);
        assert_eq!(buff.buff_type, BuffType::Haste);
        assert_eq!(buff.caster, "TestCaster");
        assert!(!buff.infinite);
        assert!(!buff.paused);
    }

    #[test]
    fn test_buff_remaining_time() {
        let buff = create_test_buff(BuffType::Haste, 10000);
        assert_eq!(buff.get_remaining_seconds(5000), 5);
        assert_eq!(buff.get_remaining_seconds(9000), 1);
        assert_eq!(buff.get_remaining_seconds(10000), 0);
    }

    #[test]
    fn test_buff_expiration() {
        let buff = create_test_buff(BuffType::Haste, 10000);
        assert!(!buff.is_expired(5000));
        assert!(!buff.is_expired(9999));
        assert!(buff.is_expired(10000));
        assert!(buff.is_expired(15000));
    }

    #[test]
    fn test_buff_expiring_soon() {
        let buff = create_test_buff(BuffType::Haste, 10000);
        assert!(!buff.is_expiring_soon(1000)); // 9秒剩余
        assert!(buff.is_expiring_soon(5000)); // 5秒剩余
        assert!(buff.is_expiring_soon(9000)); // 1秒剩余
    }

    #[test]
    fn test_infinite_buff() {
        let mut buff = create_test_buff(BuffType::Guild, 10000);
        buff.infinite = true;

        assert!(!buff.is_expired(20000));
        assert_eq!(buff.get_remaining_seconds(20000), 0);
        assert!(!buff.is_expiring_soon(20000));
    }

    #[test]
    fn test_buff_dialog_new() {
        let dialog = BuffDialog::new();
        assert!(dialog.visible);
        assert_eq!(dialog.buff_count(), 0);
        assert!(!dialog.expanded);
        assert!(dialog.faded_out);
    }

    #[test]
    fn test_add_buff() {
        let mut dialog = BuffDialog::new();
        let buff = create_test_buff(BuffType::Haste, 10000);

        dialog.add_buff(buff);
        assert_eq!(dialog.buff_count(), 1);
    }

    #[test]
    fn test_remove_buff() {
        let mut dialog = BuffDialog::new();
        dialog.add_buff(create_test_buff(BuffType::Haste, 10000));
        dialog.add_buff(create_test_buff(BuffType::Fury, 10000));
        assert_eq!(dialog.buff_count(), 2);

        let removed = dialog.remove_buff(0);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().buff_type, BuffType::Haste);
        assert_eq!(dialog.buff_count(), 1);
    }

    #[test]
    fn test_remove_buff_by_type() {
        let mut dialog = BuffDialog::new();
        dialog.add_buff(create_test_buff(BuffType::Haste, 10000));
        dialog.add_buff(create_test_buff(BuffType::Fury, 10000));

        let removed = dialog.remove_buff_by_type(BuffType::Haste);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().buff_type, BuffType::Haste);
        assert_eq!(dialog.buff_count(), 1);
        assert!(dialog.has_buff(BuffType::Fury));
    }

    #[test]
    fn test_clear() {
        let mut dialog = BuffDialog::new();
        dialog.add_buff(create_test_buff(BuffType::Haste, 10000));
        dialog.add_buff(create_test_buff(BuffType::Fury, 10000));
        assert_eq!(dialog.buff_count(), 2);

        dialog.clear();
        assert_eq!(dialog.buff_count(), 0);
    }

    #[test]
    fn test_toggle_expand() {
        let mut dialog = BuffDialog::new();
        assert!(!dialog.expanded);

        dialog.toggle_expand();
        assert!(dialog.expanded);

        dialog.toggle_expand();
        assert!(!dialog.expanded);
    }

    #[test]
    fn test_update_window_size() {
        let mut dialog = BuffDialog::new();
        assert_eq!(dialog.size, (44, 34)); // 收起状态

        // 添加1个Buff
        dialog.add_buff(create_test_buff(BuffType::Haste, 10000));
        assert_eq!(dialog.size, (44, 34)); // 收起状态不变

        // 展开
        dialog.expanded = true;
        dialog.update_window();
        assert!(dialog.size.0 > 44); // 展开后变大

        // 添加更多Buff
        for i in 0..15 {
            dialog.add_buff(create_test_buff(BuffType::Fury, 10000));
        }
        let old_height = dialog.size.1;
        dialog.update_window();
        assert!(dialog.size.1 > old_height); // 多行后高度增加
    }

    #[test]
    fn test_process_removes_expired() {
        let mut dialog = BuffDialog::new();
        dialog.add_buff(create_test_buff(BuffType::Haste, 5000));
        dialog.add_buff(create_test_buff(BuffType::Fury, 10000));
        assert_eq!(dialog.buff_count(), 2);

        // 第一个Buff过期
        dialog.process(5000, false);
        assert_eq!(dialog.buff_count(), 1);
        assert!(dialog.has_buff(BuffType::Fury));
        assert!(!dialog.has_buff(BuffType::Haste));
    }

    #[test]
    fn test_fade_in_on_mouse_over() {
        let mut dialog = BuffDialog::new();
        dialog.add_buff(create_test_buff(BuffType::Haste, 10000));
        assert_eq!(dialog.opacity, 0.0);

        // 鼠标悬停
        for _ in 0..10 {
            dialog.process(0, true);
        }
        assert!(dialog.opacity > 0.0);
        assert!(dialog.faded_in || dialog.opacity > 0.5);
    }

    #[test]
    fn test_fade_out_on_mouse_leave() {
        let mut dialog = BuffDialog::new();
        dialog.add_buff(create_test_buff(BuffType::Haste, 10000));
        dialog.opacity = 1.0;
        dialog.faded_in = true;
        dialog.faded_out = false;

        // 鼠标离开
        for _ in 0..10 {
            dialog.process(0, false);
        }
        assert!(dialog.opacity < 1.0);
    }

    #[test]
    fn test_find_buff() {
        let mut dialog = BuffDialog::new();
        dialog.add_buff(create_test_buff(BuffType::Haste, 10000));
        dialog.add_buff(create_test_buff(BuffType::Fury, 10000));

        assert!(dialog.find_buff(BuffType::Haste).is_some());
        assert!(dialog.find_buff(BuffType::Rage).is_none());
    }

    #[test]
    fn test_has_buff() {
        let mut dialog = BuffDialog::new();
        dialog.add_buff(create_test_buff(BuffType::Haste, 10000));

        assert!(dialog.has_buff(BuffType::Haste));
        assert!(!dialog.has_buff(BuffType::Fury));
    }

    #[test]
    fn test_get_buff_time_text() {
        let dialog = BuffDialog::new();
        
        let buff = create_test_buff(BuffType::Haste, 10000);
        assert_eq!(dialog.get_buff_time_text(&buff, 5000), "5s");

        let mut buff_paused = create_test_buff(BuffType::Haste, 10000);
        buff_paused.paused = true;
        assert_eq!(dialog.get_buff_time_text(&buff_paused, 5000), "Paused");

        let mut buff_infinite = create_test_buff(BuffType::Guild, 10000);
        buff_infinite.infinite = true;
        assert_eq!(dialog.get_buff_time_text(&buff_infinite, 5000), "Permanent");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(5), "5s");
        assert_eq!(format_duration(65), "01:05");
        assert_eq!(format_duration(3665), "01:01:05");
    }
}
