/// 龙系统，对应 C# MirEnvir/Dragon.cs
/// 龙身多部件Boss，经验累积→升级→掉落→定时降级

use kameo::actor::ActorRef;
use crate::gate::actor::GateActor;

/// 龙的状态数据
#[derive(Debug, Clone)]
pub struct DragonState {
    /// 龙身主体怪物 object_id
    pub body_object_id: u32,
    /// 当前等级 (1-12)
    pub level: u8,
    /// 当前经验
    pub experience: u64,
    /// 升至满级的时间戳（Unix秒，满级后6小时自动降级回1级）
    pub max_level_time: i64,
    /// 上次降级检查时间（tick count）
    pub last_delevel_check: u64,
    /// 上次 spawn 检查时间（tick count，用于 EvilMir 生成节流）
    pub last_spawn_check: u64,
    /// 活跃状态
    pub active: bool,
    /// 当前 EvilMir 怪物 object_id（None = 未生成/已被击杀）
    pub evil_mir_oid: Option<u32>,
}

impl DragonState {
    pub fn new(body_object_id: u32) -> Self {
        Self {
            body_object_id,
            level: 1,
            experience: 0,
            max_level_time: 0,
            last_delevel_check: 0,
            last_spawn_check: 0,
            active: true,
            evil_mir_oid: None,
        }
    }

    /// 升级所需经验表（C# Exps[12]）
    pub fn xp_for_level(level: u8) -> u64 {
        match level {
            1 => 5000,
            2 => 10000,
            3 => 20000,
            4 => 40000,
            5 => 80000,
            6 => 160000,
            7 => 320000,
            8 => 640000,
            9 => 1280000,
            10 => 2560000,
            11 => 5120000,
            _ => 0, // Level 12 is max
        }
    }

    /// 加点经验，返回升级的次数（可能连升多级）。对应 C# Dragon.GainExp。
    pub fn gain_exp(&mut self, amount: u64) -> u32 {
        let mut levelled = 0u32;
        if self.level >= 12 {
            return 0;
        }
        self.experience += amount;
        loop {
            if self.level >= 12 { break; }
            let needed = Self::xp_for_level(self.level);
            if needed == 0 || self.experience < needed { break; }
            self.experience -= needed;
            self.level += 1;
            levelled += 1;
            if self.level >= 12 {
                self.experience = 0;
                self.max_level_time = chrono::Utc::now().timestamp();
            }
        }
        levelled
    }

    /// 生成龙身 24 个部件的位置偏移（C# BodyLocations）
    pub fn body_part_offsets() -> Vec<(i32, i32)> {
        vec![
            (0, -2), (1, -1), (2, 0), (1, 1), (0, 2), (-1, 1), (-2, 0), (-1, -1), // 外圈8个
            (0, -1), (1, 0), (0, 1), (-1, 0), // 内圈4个
            (0, -3), (2, -2), (3, 0), (2, 2), (0, 3), (-2, 2), (-3, 0), (-2, -2), // 更外圈8个
            (1, -2), (2, -1), (2, 1), (1, 2), (-1, 2), (-2, 1), (-2, -1), (-1, -2), // 交错8个
        ].into_iter().take(24).collect()
    }
}

/// 处理龙降级逻辑（C# Dragon.Process 的降级分支）+ spawn 检查。
///
/// 返回 Some(SpawnEvilMirRequest) 当需要生成新 EvilMir（level 提升且当前无活跃 EvilMir）。
pub async fn tick_dragon_delevel(
    dragon: &mut DragonState,
    _tick_count: u64,
    _gate_ref: &ActorRef<GateActor>,
) {
    if !dragon.active { return; }
    if dragon.level < 12 { return; }
    if dragon.max_level_time == 0 { return; }

    let now = chrono::Utc::now().timestamp();
    // 6 hours = 21600 seconds（C# DeLevelDelay = 60 * 60 * 1000 ms = 1 小时；C# Process 用 6 * DeLevelDelay = 6 小时）
    if now - dragon.max_level_time >= 21600 {
        dragon.level = 1;
        dragon.experience = 0;
        dragon.max_level_time = 0;
        tracing::info!("Dragon deleveled to {}", dragon.level);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gain_exp_level_up() {
        let mut d = DragonState::new(1);
        // Level 1 -> 2 needs 5000 xp
        let n = d.gain_exp(5000);
        assert_eq!(n, 1);
        assert_eq!(d.level, 2);
        assert_eq!(d.experience, 0);
    }

    #[test]
    fn test_gain_exp_multi_level() {
        let mut d = DragonState::new(1);
        // 5000 + 10000 = 15000 → level 1->2 (cost 5000), 2->3 (cost 10000)
        let n = d.gain_exp(15000);
        assert_eq!(n, 2);
        assert_eq!(d.level, 3);
    }

    #[test]
    fn test_max_level_delevel_trigger() {
        let mut d = DragonState::new(1);
        d.level = 12;
        d.max_level_time = 0;
        d.gain_exp(0); // no-op at max
        assert_eq!(d.level, 12);
    }
}
