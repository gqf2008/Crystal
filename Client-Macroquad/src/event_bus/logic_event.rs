// ============================================================================
// GameLogicEvent - 游戏逻辑事件定义
// ============================================================================
//
// 职责：
// - 定义客户端系统间的业务逻辑事件
// - 不包含网络协议事件（那些在 NetworkEvent 中）
//
// 设计原则：
// - 业务导向：描述游戏中发生的业务事件
// - 系统解耦：系统间通过事件通信，避免直接依赖
// - 单帧有效：事件在当前帧结束后清空

use hecs::Entity;
use mir2_shared::enums::MirDirection;

// ============================================================================
// 游戏逻辑事件枚举
// ============================================================================

#[derive(Debug, Clone)]
pub enum GameLogicEvent {
    // ========================================================================
    // 战斗事件
    // ========================================================================
    /// 造成伤害
    DamageDealt {
        attacker: Entity,
        target: Entity,
        damage: i32,
        damage_type: DamageType,
    },

    /// 实体死亡
    EntityDied {
        entity: Entity,
        killer: Option<Entity>,
    },

    /// 实体复活
    EntityRevived {
        entity: Entity,
        position: (i32, i32),
    },

    // ========================================================================
    // 移动事件
    // ========================================================================
    /// 实体移动
    EntityMoved {
        entity: Entity,
        from: (i32, i32),
        to: (i32, i32),
        direction: MirDirection,
    },

    /// 实体传送
    EntityTeleported {
        entity: Entity,
        from: (i32, i32),
        to: (i32, i32),
    },

    /// 寻路失败
    PathfindingFailed {
        entity: Entity,
        target: (i32, i32),
        reason: String,
    },

    /// 碰撞发生
    CollisionOccurred {
        entity: Entity,
        collider: ColliderType,
        position: (i32, i32),
    },

    // ========================================================================
    // 物品事件
    // ========================================================================
    /// 拾取物品
    ItemPickedUp {
        entity: Entity,
        item_id: u64,
        item_type: u8,
    },

    /// 丢弃物品
    ItemDropped {
        entity: Entity,
        item_id: u64,
        location: (i32, i32),
    },

    /// 使用物品
    ItemUsed {
        entity: Entity,
        item_id: u64,
        item_type: u8,
    },

    /// 装备物品
    ItemEquipped {
        entity: Entity,
        item_id: u64,
        slot: u8,
    },

    /// 卸下装备
    ItemUnequipped {
        entity: Entity,
        item_id: u64,
        slot: u8,
    },

    // ========================================================================
    // 技能事件
    // ========================================================================
    /// 施放技能
    SpellCast {
        caster: Entity,
        spell: SpellType,
        target: Option<Entity>,
        target_position: Option<(i32, i32)>,
    },

    /// 技能命中
    SpellHit {
        spell: SpellType,
        caster: Entity,
        target: Entity,
    },

    /// 技能学习
    SpellLearned {
        entity: Entity,
        spell: SpellType,
        level: u8,
    },

    // ========================================================================
    // Buff/Debuff 事件
    // ========================================================================
    /// 添加 Buff
    BuffAdded {
        entity: Entity,
        buff_type: BuffType,
        duration: f32,
        source: Option<Entity>,
    },

    /// 移除 Buff
    BuffRemoved { entity: Entity, buff_type: BuffType },

    /// Buff 刷新（重新计时）
    BuffRefreshed {
        entity: Entity,
        buff_type: BuffType,
        new_duration: f32,
    },

    // ========================================================================
    // 等级/经验事件
    // ========================================================================
    /// 获得经验
    ExperienceGained {
        entity: Entity,
        amount: i64,
        source: ExperienceSource,
    },

    /// 等级提升
    LevelUp {
        entity: Entity,
        old_level: u16,
        new_level: u16,
    },

    // ========================================================================
    // 地图事件
    // ========================================================================
    /// 进入地图
    MapEntered {
        entity: Entity,
        map_index: i32,
        position: (i32, i32),
    },

    /// 离开地图
    MapLeft { entity: Entity, map_index: i32 },

    // ========================================================================
    // 交互事件
    // ========================================================================
    /// 与NPC对话
    NpcInteraction { entity: Entity, npc_id: u32 },

    /// 打开商店
    ShopOpened { entity: Entity, shop_id: u32 },

    /// 打开仓库
    StorageOpened { entity: Entity },

    // ========================================================================
    // 组队/公会事件
    // ========================================================================
    /// 加入队伍
    PartyJoined { entity: Entity, party_id: u32 },

    /// 离开队伍
    PartyLeft { entity: Entity, party_id: u32 },

    /// 加入公会
    GuildJoined { entity: Entity, guild_name: String },

    /// 离开公会
    GuildLeft { entity: Entity, guild_name: String },
}

// ============================================================================
// 辅助枚举
// ============================================================================

/// 伤害类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageType {
    Physical,  // 物理伤害
    Magic,     // 魔法伤害
    Poison,    // 毒素伤害
    Holy,      // 神圣伤害
    Fire,      // 火焰伤害
    Ice,       // 冰霜伤害
    Lightning, // 雷电伤害
}

/// 碰撞体类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColliderType {
    Wall,           // 墙壁
    Entity(Entity), // 其他实体
    Boundary,       // 地图边界
}

/// 技能类型（临时定义，应该从 Shared 导入）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellType {
    // 战士技能
    BasicSword,
    Thrust,
    Assassination,

    // 法师技能
    FireBall,
    Lightning,
    Teleport,

    // 道士技能
    Healing,
    PoisonCloud,
    SummonSkeleton,
}

/// Buff 类型（临时定义，应该从 Shared 导入）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuffType {
    // 增益
    AttackBoost,
    DefenseBoost,
    SpeedBoost,
    MagicShield,

    // 减益
    Poison,
    Slow,
    Stun,
    Curse,
}

/// 经验来源
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExperienceSource {
    MonsterKill,   // 击杀怪物
    QuestComplete, // 完成任务
    Discovery,     // 探索发现
    Other,         // 其他
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_damage_types() {
        let physical = DamageType::Physical;
        let magic = DamageType::Magic;

        assert_ne!(physical, magic);
        assert_eq!(physical, DamageType::Physical);
    }
}
