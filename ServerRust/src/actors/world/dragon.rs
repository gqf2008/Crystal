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
    /// 活跃状态
    pub active: bool,
}

impl DragonState {
    pub fn new(body_object_id: u32) -> Self {
        Self {
            body_object_id,
            level: 1,
            experience: 0,
            max_level_time: 0,
            last_delevel_check: 0,
            active: true,
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

    /// 加点经验，返回是否升级
    pub fn gain_exp(&mut self, amount: u64) -> bool {
        if self.level >= 12 {
            return false;
        }
        self.experience += amount;
        let needed = Self::xp_for_level(self.level);
        if self.experience >= needed {
            self.level += 1;
            self.experience = 0;
            if self.level >= 12 {
                self.max_level_time = chrono::Utc::now().timestamp();
            }
            return true;
        }
        false
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

/// 处理龙降级逻辑
pub async fn tick_dragon_delevel(
    dragon: &mut DragonState,
    _tick_count: u64,
    _gate_ref: &ActorRef<GateActor>,
) {
    if !dragon.active { return; }
    if dragon.level < 12 { return; }
    if dragon.max_level_time == 0 { return; }

    let now = chrono::Utc::now().timestamp();
    // 6 hours = 21600 seconds
    if now - dragon.max_level_time >= 21600 {
        dragon.level = 1;
        dragon.experience = 0;
        dragon.max_level_time = 0;
        // Broadcast would go here when gate_ref is wired
        tracing::info!("Dragon deleveled to {}", dragon.level);
    }
}
