// ============================================================================
// 战斗相关组件
// ============================================================================

pub use mir2_shared::{MirClass, MirGender};
use std::time::Instant;

/// Buff类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuffType {
    /// 中毒
    Poison,
    /// 流血
    Bleeding,
    /// 攻击加成
    AttackBoost,
    /// 防御加成
    DefenseBoost,
    /// 速度加成
    SpeedBoost,
    /// 魔法护盾
    MagicShield,
}

impl BuffType {
    /// 获取Buff的默认持续时间(毫秒)
    pub fn default_duration(&self) -> u64 {
        match self {
            BuffType::Poison | BuffType::Bleeding => 10000, // 10秒
            BuffType::AttackBoost | BuffType::DefenseBoost | BuffType::SpeedBoost => 60000, // 60秒
            BuffType::MagicShield => 30000, // 30秒
        }
    }

    /// 是否是负面Buff
    pub fn is_debuff(&self) -> bool {
        matches!(self, BuffType::Poison | BuffType::Bleeding)
    }
}

/// Buff实例
#[derive(Debug, Clone)]
pub struct Buff {
    /// Buff类型
    pub buff_type: BuffType,
    /// 剩余持续时间(毫秒)
    pub remaining_duration: u64,
    /// 叠加层数
    pub stack_count: u8,
    /// 效果强度(可选)
    pub strength: Option<i32>,
    /// 开始时间
    pub start_time: Instant,
}

impl Buff {
    pub fn new(buff_type: BuffType) -> Self {
        Self {
            buff_type,
            remaining_duration: buff_type.default_duration(),
            stack_count: 1,
            strength: None,
            start_time: Instant::now(),
        }
    }

    pub fn with_duration(mut self, duration: u64) -> Self {
        self.remaining_duration = duration;
        self
    }

    pub fn with_strength(mut self, strength: i32) -> Self {
        self.strength = Some(strength);
        self
    }

    /// 更新Buff(返回是否已过期)
    pub fn update(&mut self, delta_ms: u64) -> bool {
        if self.remaining_duration > delta_ms {
            self.remaining_duration -= delta_ms;
            false
        } else {
            self.remaining_duration = 0;
            true // 已过期
        }
    }
}

/// Buff列表组件
#[derive(Debug, Clone)]
pub struct BuffList {
    /// 活动的Buff列表
    pub active_buffs: Vec<Buff>,
}

impl BuffList {
    pub fn new() -> Self {
        Self {
            active_buffs: Vec::new(),
        }
    }

    /// 添加Buff
    pub fn add_buff(&mut self, buff: Buff) {
        // 检查是否已存在相同类型的Buff
        if let Some(existing) = self.active_buffs.iter_mut().find(|b| b.buff_type == buff.buff_type) {
            // 刷新持续时间并增加层数
            existing.remaining_duration = buff.remaining_duration;
            existing.stack_count = (existing.stack_count + 1).min(99);
        } else {
            self.active_buffs.push(buff);
        }
    }

    /// 移除Buff
    pub fn remove_buff(&mut self, buff_type: BuffType) {
        self.active_buffs.retain(|b| b.buff_type != buff_type);
    }

    /// 检查是否有某个Buff
    pub fn has_buff(&self, buff_type: BuffType) -> bool {
        self.active_buffs.iter().any(|b| b.buff_type == buff_type)
    }

    /// 清理过期的Buff
    pub fn cleanup_expired(&mut self, delta_ms: u64) {
        self.active_buffs.retain_mut(|buff| !buff.update(delta_ms));
    }
}

impl Default for BuffList {
    fn default() -> Self {
        Self::new()
    }
}

/// 生命/魔法恢复计时器
#[derive(Debug, Clone)]
pub struct RegenTimer {
    /// HP恢复计时器(毫秒)
    pub hp_timer: u64,
    /// MP恢复计时器(毫秒)
    pub mp_timer: u64,
    /// HP恢复间隔(毫秒)
    pub hp_interval: u64,
    /// MP恢复间隔(毫秒)
    pub mp_interval: u64,
}

impl RegenTimer {
    pub fn new() -> Self {
        Self {
            hp_timer: 0,
            mp_timer: 0,
            hp_interval: 10000, // 默认10秒
            mp_interval: 10000, // 默认10秒
        }
    }

    /// 更新计时器
    pub fn update(&mut self, delta_ms: u64) {
        self.hp_timer += delta_ms;
        self.mp_timer += delta_ms;
    }

    /// 检查HP是否应该恢复
    pub fn should_regen_hp(&self) -> bool {
        self.hp_timer >= self.hp_interval
    }

    /// 检查MP是否应该恢复
    pub fn should_regen_mp(&self) -> bool {
        self.mp_timer >= self.mp_interval
    }

    /// 重置HP计时器
    pub fn reset_hp_timer(&mut self) {
        self.hp_timer = 0;
    }

    /// 重置MP计时器
    pub fn reset_mp_timer(&mut self) {
        self.mp_timer = 0;
    }
}

impl Default for RegenTimer {
    fn default() -> Self {
        Self::new()
    }
}

/// 生命值组件
#[derive(Debug, Clone, Copy)]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

impl Health {
    pub fn new(max: i32) -> Self {
        Self { current: max, max }
    }

    pub fn is_alive(&self) -> bool {
        self.current > 0
    }

    pub fn take_damage(&mut self, damage: i32) {
        if damage <= 0 {
            return;
        }
        self.current = self.current.saturating_sub(damage).max(0);
    }

    pub fn heal(&mut self, amount: i32) {
        if amount <= 0 {
            return;
        }
        self.current = self.current.saturating_add(amount).min(self.max);
    }
}

/// 魔法值组件
#[derive(Debug, Clone, Copy)]
pub struct Mana {
    pub current: i32,
    pub max: i32,
}

impl Mana {
    pub fn new(max: i32) -> Self {
        Self { current: max, max }
    }

    pub fn has_enough(&self, cost: i32) -> bool {
        self.current >= cost
    }

    pub fn consume(&mut self, cost: i32) -> bool {
        if cost < 0 {
            return false;
        }
        if self.current >= cost {
            self.current -= cost;
            true
        } else {
            false
        }
    }

    pub fn restore(&mut self, amount: i32) {
        if amount <= 0 {
            return;
        }
        self.current = self.current.saturating_add(amount).min(self.max);
    }

    pub fn percent(&self) -> f32 {
        if self.max > 0 {
            self.current as f32 / self.max as f32
        } else {
            0.0
        }
    }
}

/// 战斗属性组件 (玩家/怪物)
#[derive(Debug, Clone)]
pub struct CombatStats {
    pub level: u16,
    pub attack_min: i32,
    pub attack_max: i32,
    pub defense: i32,
    pub magic_defense: i32,
    pub accuracy: u8,
    pub agility: u8,
}

impl Default for CombatStats {
    fn default() -> Self {
        Self {
            level: 1,
            attack_min: 0,
            attack_max: 0,
            defense: 0,
            magic_defense: 0,
            accuracy: 0,
            agility: 0,
        }
    }
}

impl CombatStats {
    pub fn new() -> Self {
        Self::default()
    }
}

/// 经验值组件 (升级系统 - Level System)
#[derive(Debug, Clone, Copy)]
pub struct Experience {
    pub current: i64,
    pub required: i64,  // 下一级所需
}

impl Experience {
    pub fn new(level: u16) -> Self {
        Self {
            current: 0,
            required: Self::calculate_required(level),
        }
    }
    
    pub fn add(&mut self, amount: i64) -> bool {
        self.current += amount;
        self.current >= self.required
    }
    
    pub fn level_up(&mut self, new_level: u16) {
        self.current -= self.required;
        self.required = Self::calculate_required(new_level);
    }
    
    pub fn percent(&self) -> f32 {
        (self.current as f32 / self.required as f32).clamp(0.0, 1.0)
    }
    
    /// 传奇升级经验公式
    fn calculate_required(level: u16) -> i64 {
        ((level as i64 + 1) * (level as i64 + 1) * 100) as i64
    }
}

impl Default for Experience {
    fn default() -> Self {
        Self::new(1)
    }
}

/// 货币组件 (经济系统 - Economy System)
#[derive(Debug, Clone, Copy)]
pub struct Currency {
    pub gold: u32,      // 金币
    pub credit: u32,    // 元宝/点券
}

impl Currency {
    pub fn new() -> Self {
        Self { gold: 0, credit: 0 }
    }
    
    pub fn can_afford_gold(&self, cost: u32) -> bool {
        self.gold >= cost
    }
    
    pub fn spend_gold(&mut self, cost: u32) -> bool {
        if self.can_afford_gold(cost) {
            self.gold -= cost;
            true
        } else {
            false
        }
    }
    
    pub fn add_gold(&mut self, amount: u32) {
        self.gold = self.gold.saturating_add(amount);
    }
}

impl Default for Currency {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 武器系统组件
// ============================================================================

/// 武器运行时状态组件
/// 存储武器的动态状态信息，如当前攻击帧、冷却等
#[derive(Debug, Clone)]
pub struct WeaponState {
    /// 当前攻击类型 (Attack1/Attack2/Attack3)
    pub current_attack: u8,
    /// 攻击动画当前帧
    pub current_frame: u8,
    /// 攻击冷却剩余时间(毫秒)
    pub cooldown_remaining: u64,
    /// 攻击速度倍率 (1.0=正常)
    pub attack_speed: f32,
    /// 是否正在攻击
    pub is_attacking: bool,
    /// 攻击开始时间
    pub attack_start_time: Option<Instant>,
}

impl WeaponState {
    pub fn new() -> Self {
        Self {
            current_attack: 1,
            current_frame: 0,
            cooldown_remaining: 0,
            attack_speed: 1.0,
            is_attacking: false,
            attack_start_time: None,
        }
    }

    /// 开始攻击
    pub fn start_attack(&mut self, attack_type: u8) {
        self.is_attacking = true;
        self.current_attack = attack_type.clamp(1, 3);
        self.current_frame = 0;
        self.attack_start_time = Some(Instant::now());
    }

    /// 结束攻击
    pub fn end_attack(&mut self, cooldown_ms: u64) {
        self.is_attacking = false;
        self.cooldown_remaining = cooldown_ms;
        self.attack_start_time = None;
    }

    /// 更新冷却时间
    pub fn update_cooldown(&mut self, delta_ms: u64) {
        if self.cooldown_remaining > delta_ms {
            self.cooldown_remaining -= delta_ms;
        } else {
            self.cooldown_remaining = 0;
        }
    }

    /// 是否可以攻击
    pub fn can_attack(&self) -> bool {
        !self.is_attacking && self.cooldown_remaining == 0
    }
}

impl Default for WeaponState {
    fn default() -> Self {
        Self::new()
    }
}

/// 武器动画配置组件
/// 存储武器动画的静态配置数据
#[derive(Debug, Clone)]
pub struct WeaponAnimation {
    /// 武器库索引 (CWeapons数组索引)
    pub weapon_library_index: u16,
    /// Attack1 动画帧数
    pub attack1_frames: u8,
    /// Attack2 动画帧数
    pub attack2_frames: u8,
    /// Attack3 动画帧数
    pub attack3_frames: u8,
    /// 每帧持续时间(毫秒)
    pub frame_duration: u64,
    /// 武器特效库索引 (CWeaponEffect数组索引)
    pub effect_library_index: Option<u16>,
    /// 特效触发帧 (在哪一帧显示攻击特效)
    pub effect_trigger_frame: u8,
}

impl WeaponAnimation {
    pub fn new(weapon_library_index: u16) -> Self {
        Self {
            weapon_library_index,
            attack1_frames: 6, // 默认6帧
            attack2_frames: 6,
            attack3_frames: 6,
            frame_duration: 100, // 默认100ms/帧
            effect_library_index: None,
            effect_trigger_frame: 3, // 默认第3帧触发特效
        }
    }

    /// 获取指定攻击类型的总帧数
    pub fn get_attack_frames(&self, attack_type: u8) -> u8 {
        match attack_type {
            1 => self.attack1_frames,
            2 => self.attack2_frames,
            3 => self.attack3_frames,
            _ => self.attack1_frames,
        }
    }

    /// 获取攻击动画总时长(毫秒)
    pub fn get_attack_duration(&self, attack_type: u8) -> u64 {
        self.get_attack_frames(attack_type) as u64 * self.frame_duration
    }

    /// 是否应该触发特效
    pub fn should_trigger_effect(&self, current_frame: u8) -> bool {
        self.effect_library_index.is_some() && current_frame == self.effect_trigger_frame
    }
}

/// 攻击判定区域组件
/// 用于近战武器的攻击范围判定
#[derive(Debug, Clone)]
pub struct AttackHitbox {
    /// 攻击范围(格子数)
    pub range: u8,
    /// 攻击扇形角度(度)
    pub angle: f32,
    /// 是否启用判定
    pub enabled: bool,
    /// 判定开始帧
    pub hit_start_frame: u8,
    /// 判定结束帧
    pub hit_end_frame: u8,
}

impl AttackHitbox {
    pub fn new(range: u8) -> Self {
        Self {
            range,
            angle: 120.0, // 默认120度扇形
            enabled: false,
            hit_start_frame: 2, // 默认第2帧开始判定
            hit_end_frame: 4,   // 默认第4帧结束判定
        }
    }

    /// 启用判定
    pub fn activate(&mut self) {
        self.enabled = true;
    }

    /// 禁用判定
    pub fn deactivate(&mut self) {
        self.enabled = false;
    }

    /// 检查当前帧是否在判定范围内
    pub fn is_active_frame(&self, current_frame: u8) -> bool {
        self.enabled && current_frame >= self.hit_start_frame && current_frame <= self.hit_end_frame
    }

    /// 设置判定区间
    pub fn set_hit_frames(&mut self, start: u8, end: u8) {
        self.hit_start_frame = start;
        self.hit_end_frame = end;
    }
}

impl Default for AttackHitbox {
    fn default() -> Self {
        Self::new(1)
    }
}
