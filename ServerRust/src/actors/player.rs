// PlayerActor - 玩家实例
// 持有单个玩家的完整状态：位置、方向、地图、背包等
// 移动由客户端驱动，服务端验证并广播

use kameo::actor::{Actor, ActorRef};
use kameo::message::Message;
use kameo::prelude::Context;
use tracing::{debug, info, warn};

use crate::actors::inventory::{PlayerInventory, EquipmentSlot};
use crate::actors::friend::FriendList;
use crate::actors::mail::Mailbox;
use crate::actors::guild::GuildRank;
use crate::actors::quest::QuestLog;
use crate::actors::creature::CreatureLog;
use crate::actors::refine::RefineLog;
use crate::gate::actor::{GateActor, SendToClient};
use crate::maps::loader::MapData;
use crate::util::wire::{build_packet_bytes, write_dotnet_string};
use mir2_shared::packets::Packet;

/// 玩家已学习的魔法/技能
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlayerMagic {
    pub spell: i32,
    pub level: u8,
    pub experience: u16,
    pub key: u8,
    pub toggled: bool,
    /// 上次施法时间（毫秒时间戳，用于 CD 检查）
    pub cast_time: i64,
    /// #937：装备临时技能（C# UserMagic.IsTempSpell；不持久化，卸装/换装时移除）
    pub temp_skill: bool,
}

impl PlayerMagic {
    pub fn new(spell: i32) -> Self {
        Self { spell, level: 0, experience: 0, key: 0, toggled: false, cast_time: 0, temp_skill: false }
    }
}

/// 方向增量 (MirDirection: Up=0, UpRight=1, Right=2, DownRight=3, Down=4, DownLeft=5, Left=6, UpLeft=7)
const DIR_DX: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
const DIR_DY: [i32; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];

/// 玩家状态
#[derive(Debug, Clone)]
pub struct PlayerState {
    /// 唯一对象 ID
    pub object_id: u32,
    /// 玩家名称
    pub name: String,
    /// 当前地图
    pub map_index: u16,
    /// 网格坐标 X
    pub x: i32,
    /// 网格坐标 Y
    pub y: i32,
    /// 朝向 (0..7)
    pub direction: u8,
    /// 攻击模式 (Peace/Group/Guild/EnemyGuild/RedBrown/All)
    pub attack_mode: mir2_shared::enums::AttackMode,
    /// 宠物模式 (Both/MoveOnly/AttackOnly/None/FocusMasterTarget)
    pub pet_mode: mir2_shared::enums::PetMode,
    /// 是否隐藏
    pub hidden: bool,
    /// 所属 session
    pub session_id: u64,
    /// 职业
    pub class: mir2_shared::enums::MirClass,
    /// 性别
    pub gender: mir2_shared::enums::MirGender,
    /// 发型
    pub hair: u8,
    /// 等级
    pub level: u16,
    /// 当前经验
    pub experience: i64,
    /// 升级所需经验
    pub max_experience: i64,
    /// 是否可获得经验（NPC 脚本 CANGAINEXP，对齐 C# CanGainExp）
    pub can_gain_exp: bool,
    /// 珍珠数（NPC 脚本 GIVEPEARLS/TAKEPEARLS，对齐 C# PearlCount）
    pub pearl_count: i32,
    /// 当前 HP
    pub hp: i32,
    /// 最大 HP（基础+装备加成后的总值）
    pub max_hp: i32,
    /// 当前 MP
    pub mp: i32,
    /// 最大 MP（基础+装备加成后的总值）
    pub max_mp: i32,
    /// 最小攻击力（基础+装备加成后的总值）
    pub min_attack: i32,
    /// 最大攻击力（基础+装备加成后的总值）
    pub max_attack: i32,
    /// 防御力（基础+装备加成后的总值）
    pub defence: i32,
    /// 最小魔法攻击力（基础+装备加成后的总值）
    pub min_mc: i32,
    /// 最大魔法攻击力（基础+装备加成后的总值）
    pub max_mc: i32,
    /// 最小道术攻击力（基础+装备加成后的总值）
    pub min_sc: i32,
    /// 最大道术攻击力（基础+装备加成后的总值）
    pub max_sc: i32,
    /// 装备加成：最小攻击力
    pub bonus_min_attack: i32,
    /// 装备加成：最大攻击力
    pub bonus_max_attack: i32,
    /// 装备加成：防御力
    pub bonus_defence: i32,
    /// 装备加成：最大 HP
    pub bonus_max_hp: i32,
    /// 装备加成：最大 MP
    pub bonus_max_mp: i32,
    /// 装备加成：最小魔法攻击力
    pub bonus_min_mc: i32,
    /// 装备加成：最大魔法攻击力
    pub bonus_max_mc: i32,
    /// 装备加成：最小道术攻击力
    pub bonus_min_sc: i32,
    /// 装备加成：最大道术攻击力
    pub bonus_max_sc: i32,
    /// 冰冻属性
    pub freezing: i32,
    /// 毒物攻击
    pub poison_attack: i32,
    /// 生命恢复（C# Stat.HealthRecovery，装备提供，用于基础回血公式）
    pub health_recovery: i32,
    /// 魔法恢复（C# Stat.SpellRecovery，装备提供，用于基础回蓝公式）
    pub spell_recovery: i32,
    /// 攻击速度（C# Stat.AttackSpeed，装备提供，UserInformation 展示）
    pub attack_speed: i32,
    /// 毒抗（C# Stat.PoisonResist，装备提供，UserInformation 展示）
    pub poison_resist: i32,
    /// 毒物恢复
    pub poison_recovery: i32,
    /// 神圣属性
    pub holy: i32,
    /// 准确
    pub accuracy: i32,
    /// 敏捷
    pub agility: i32,
    // ===== 战斗公式扩展字段（对齐 C# Stats，平铺模型）=====
    /// 最小物理防御 (AC)
    pub min_ac: i32,
    /// 最大物理防御 (AC)
    pub max_ac: i32,
    /// 最小魔法防御 (MAC)
    pub min_mac: i32,
    /// 最大魔法防御 (MAC)
    pub max_mac: i32,
    /// 装备加成：最小物理防御
    pub bonus_min_ac: i32,
    /// 装备加成：最大物理防御
    pub bonus_max_ac: i32,
    /// 装备加成：最小魔法防御
    pub bonus_min_mac: i32,
    /// 装备加成：最大魔法防御
    pub bonus_max_mac: i32,
    /// 幸运/诅咒（正=幸运倾向最大攻击，负=诅咒倾向最小攻击）
    pub luck: i32,
    /// 暴击率（C# CriticalRate，配合 CriticalRateWeight=5）
    pub critical_rate: i32,
    /// 暴击伤害（C# CriticalDamage，配合 CriticalDamageWeight=50）
    pub critical_damage: i32,
    /// 魔法抵抗（C# MagicResist，对抗 MAC 类攻击，权重 MagicResistWeight=10）
    pub magic_resist: i32,
    /// 反伤概率 %（C# Reflect，命中则反弹全额伤害）
    pub reflect: i32,
    /// 减伤 %（C# DamageReductionPercent，MagicShield/ElementalBarrier）
    pub damage_reduction_percent: i32,
    /// 攻击加成（C# AttackBonus，固定值加到伤害）
    pub attack_bonus: i32,
    /// 吸血 %（C# HPDrainRatePercent）
    pub hp_drain_rate_percent: i32,
    /// EnergyShield 触发概率 %
    pub energy_shield_percent: i32,
    /// EnergyShield 触发时回血量
    pub energy_shield_hp_gain: i32,
    /// 运行时中毒/负面状态列表（非持久化，每次上线清空）
    pub poison_list: Vec<crate::combat::poison::Poison>,
    /// 背包 + 装备 + 金币
    pub inventory: PlayerInventory,
    /// 所属组队 ID（None = 无组队）
    pub group_id: Option<u64>,
    /// 好友列表
    pub friend_list: FriendList,
    /// 收件箱
    pub mailbox: Mailbox,
    /// 所属行会名称
    pub guild_name: Option<String>,
    /// 行会 rank
    pub guild_rank: GuildRank,
    /// 任务日志
    pub quest_log: QuestLog,
    /// 配偶名称
    pub spouse_name: Option<String>,
    /// 结婚日期（unix 秒；0 = 无，C# CharacterInfo.MarriedDate）
    pub married_date: i64,
    /// 是否允许拜师
    pub allow_mentor: bool,
    /// 导师名称
    pub mentor_name: Option<String>,
    /// 宠物信息
    pub creature_log: CreatureLog,
    /// 英雄索引（0 = 无英雄）
    pub hero_index: u8,
    /// 英雄是否被收起（C# HeroSpawned 反义，@SUMMONHERO 切换；运行时状态不持久化）
    pub hero_despawned: bool,
    /// 英雄行为模式 (0=Attack, 1=Follow, etc.)
    pub hero_behaviour: u8,
    /// 自动药水 HP 阈值
    pub auto_pot_hp: u32,
    /// 自动药水 MP 阈值
    pub auto_pot_mp: u32,
    /// 自动药水 HP 物品索引
    pub auto_pot_hp_item: i32,
    /// 自动药水 MP 物品索引
    pub auto_pot_mp_item: i32,
    /// 英雄背包
    pub hero_inventory: crate::actors::inventory::PlayerInventory,
    /// 英雄已学习的魔法/技能（#218）
    pub hero_magics: Vec<PlayerMagic>,
    /// 精炼日志
    pub refine_log: RefineLog,
    /// 是否在钓鱼
    pub is_fishing: bool,
    /// 是否骑乘坐骑
    pub is_mounted: bool,
    /// 坐骑类型（0 = 无坐骑，>0 = 坐骑外观ID）
    pub mount_type: i16,
    /// 是否死亡（对应 C# Dead）
    pub is_dead: bool,
    /// 是否已解除诅咒锁定（C# UnlockCurse，神秘水使用后为 true，卸下诅咒装备后复位）
    pub unlock_curse: bool,
    /// 上次复活戒指触发时间（Unix 毫秒；C# LastRevivalTime，冷却 300000ms = 5 分钟）
    pub last_revival_time: i64,
    /// 最后下线时间（Unix 秒；C# CharacterInfo.LastLogoutDate，休息加成用）
    pub last_access: i64,
    /// 休息累积计数（C# _restedCounter，安全区每秒 +1；登录时按离线分钟 * 60 初始化）
    pub rested_counter: u32,
    /// 休息经验加成百分比（C# BuffType.Rested ExpRatePercent = Settings.RestedExpBonus）
    pub rested_exp_percent: u32,
    /// 休息加成到期时间（Unix 毫秒）
    pub rested_exp_end_tick: u64,
    /// 地图喊话卷轴标记（C# HasMapShout：! 喊话免费地图广播）
    pub has_map_shout: bool,
    /// 全服喊话卷轴标记（C# HasServerShout：! 喊话免费全服广播）
    pub has_server_shout: bool,
    /// 上次喊话时间（Unix 毫秒；C# ShoutTime，冷却 10 秒）
    pub last_shout_time: i64,
    /// PK 值（>0 = 红名，每杀1人+100，在线 tick 衰减）
    pub pk_points: i32,
    /// 累计击杀玩家数
    pub pk_kill_count: u32,
    /// 钓鱼自动释放
    pub fishing_autocast: bool,
    /// 轮回宿主（发起轮回的玩家 session_id）
    pub reincarnation_host: Option<u64>,
    /// 轮回是否已就绪
    pub reincarnation_ready: bool,
    /// 轮回过期时间（WorldActor tick count，过期则自动取消）
    pub reincarnation_expire_time: u64,
    /// 是否允许组队召回（对应 C# EnableGroupRecall）
    pub enable_group_recall: bool,
    /// 上次使用组队召回的时间戳（毫秒，对应 C# LastRecallTime）
    pub last_recall_time: u64,
    /// 是否允许配偶召回（对应 C# AllowLoverRecall）
    pub allow_lover_recall: bool,
    /// 是否为 GM（对应 C# IsGM / AccountInfo.AdminAccount）
    pub is_gm: bool,
    pub gm_never_die: bool,
    /// #1483：弓手特殊箭武装（0=无 1=Vampire 2=Poison；C# VampireShot/PoisonShot buff）
    pub special_shot_armed: u8,
    /// 是否已购买仓库扩容（C# AccountInfo.HasExpandedStorage；登录时从 accounts 表加载）
    pub has_expanded_storage: bool,
    /// 仓库扩容到期时间（unix 秒；0 = 无，C# AccountInfo.ExpandedStorageExpiryDate）
    pub expanded_storage_expiry_date: i64,
    /// 是否设置了仓库密码（C# AccountInfo.HasStoragePassword）
    pub has_storage_password: bool,
    /// 是否启用仓库密码保护（C# Settings.RequireStoragePassword，默认 true）
    pub require_storage_password: bool,
    /// 仓库密码最后设置时间（unix 秒；C# AccountInfo.StoragePasswordLastSet）
    pub storage_password_last_set: i64,
    /// 是否允许其他玩家观察（C# PlayerObject.AllowObserve，@ALLOWOBSERVE 切换）
    pub allow_observe: bool,
    /// 是否允许他人邀请加入行会（C# PlayerObject.EnableGuildInvite，@ALLOWGUILD 切换，默认 false）
    pub enable_guild_invite: bool,
    /// 是否允许交易（C# CharacterInfo.AllowTrade，@ALLOWTRADE 切换，默认 false）
    pub allow_trade: bool,
    /// 是否允许他人邀请组队（C# CharacterInfo.AllowGroup，SwitchGroup 切换，默认 false）
    pub allow_group: bool,
    /// 当前 Buff/Debuff 列表
    pub buffs: Vec<crate::combat::buff::BuffInstance>,
    /// 已学习的魔法/技能列表
    pub magics: Vec<PlayerMagic>,
    /// NPC Flag 列表（脚本进度追踪）
    pub flags: std::collections::HashMap<String, i32>,
    /// 经验倍率（1.0 = 正常，2.0 = 双倍）
    pub exp_multiplier: f64,
    /// 服务器全局经验倍率（C# Settings.ExpRate，登录时从 WorldActor 注入）
    pub exp_rate: f64,
    /// 经验倍率过期时间（WorldActor tick count）
    pub exp_multiplier_end_tick: u64,
    /// 掉落倍率（1.0 = 正常；Potion shape 5 Drop Buff，C# BuffType.Drop）
    pub drop_multiplier: f64,
    /// 掉落倍率过期时间（WorldActor tick count）
    pub drop_multiplier_end_tick: u64,
    /// 装备掉落率加成 %（C# Stat.ItemDropRatePercent）
    pub item_drop_rate_percent: i32,
    /// 装备金币掉落率加成 %（C# Stat.GoldDropRatePercent）
    pub gold_drop_rate_percent: i32,
    /// 元素等级（C# HumanObject.ElementsLevel，弓手元素球）
    pub elements_level: i32,
    /// 是否已有元素（C# HumanObject.HasElemental）
    pub has_elemental: bool,
    /// 专注是否被打断（C# Concentration buff Interrupted）
    pub concentration_interrupted: bool,
    /// 专注打断恢复时间（毫秒，C# Concentration buff InterruptTime）
    pub concentration_interrupt_time: i64,
    /// 绑定点地图（C# CharacterInfo.BindMapIndex）
    pub bind_map_index: i32,
    /// 绑定点 X（C# CharacterInfo.BindLocation.X）
    pub bind_x: i32,
    /// 绑定点 Y（C# CharacterInfo.BindLocation.Y）
    pub bind_y: i32,
    /// 等级特效（C# HumanObject.LevelEffects：flags 990-998 派生，外观特效位掩码）
    pub level_effects: u16,
    /// 是否为师徒关系中的导师（C# CharacterInfo.IsMentor；徒弟=false）
    pub is_mentor: bool,
    /// 徒弟经验积累（C# PlayerObject.MenteeEXP：GainExp 时 += amount * MenteeExpBank(1)/100）
    pub mentee_exp: i64,
    /// 导师伤害加成是否激活（C# HasBuff(Mentor)：徒弟近身同组时 true）
    pub mentor_damage_bonus: bool,
    /// 新手行会经验 buff（C# BuffType.Newbie：在 NewbieGuild 且开关开启时 true）
    pub newbie_exp_bonus: bool,
    /// 配偶经验加成百分比缓存（C# Settings.LoverEXPBonus；tick_partner_bonuses 刷新，避免 AddExperience 反向 ask WorldActor 死锁）
    pub exp_bonus_lover_percent: i32,
    /// 徒弟经验加成百分比缓存（C# Settings.MentorExpBoost）
    pub exp_bonus_mentee_percent: i32,
    /// 新手行会经验加成百分比缓存（C# Settings.NewbieGuildExpBuff）
    pub exp_bonus_newbie_percent: i32,
    /// 行会 Buff 经验加成百分比缓存（C# GuildBuffInfo.BuffExpRate；tick 刷新避免 AddExperience 反向 ask WorldActor 死锁）
    pub guild_buff_exp_percent: i32,
    /// 行会 Buff 钓鱼成功率加成百分比缓存（C# BuffFishRate）
    pub guild_buff_fish_rate_percent: i32,
    /// 当前地图是否无经验（C# MapInfo.NoExperience，#932；set_map_data 时从地图数据缓存，避免 AddExperience 反向 ask WorldActor 死锁）
    pub no_experience_map: bool,
    /// 灰名截止时间（毫秒；C# HumanObject.BrownTime，攻击低 PK 玩家后 1 分钟）
    pub brown_until_ms: i64,
    /// 坐骑忠诚度下降限速（毫秒；C# DecreaseLoyaltyTime，LoyaltyDelay=1000ms）
    pub mount_loyalty_decrease_time: i64,
    /// 坐骑忠诚度自动恢复时间（毫秒；C# IncreaseLoyaltyTime，每 LoyaltyDelay*60）
    pub mount_loyalty_increase_time: i64,
    /// 火把耐久消耗时间（毫秒；C# HumanObject.TorchTime，每 10s -5 耐久，归零卸下）
    pub torch_burn_time: i64,
    /// 上次受击时间（毫秒；C# Attacked 重置 RegenTime，受击后 RegenDelay=10s 内不自然回血）
    pub last_damage_ms: i64,
    /// 药水累计待回复 HP（C# PotHealthAmount，NormalPotion 累加）
    pub pot_hp_amount: u32,
    /// 药水累计待回复 MP（C# PotManaAmount，NormalPotion 累加）
    pub pot_mp_amount: u32,
    /// 药水池下次处理时间（毫秒；C# PotTime = now + PotDelay=200ms）
    pub pot_time_ms: i64,
}

impl PlayerState {
    /// 计算包含装备+Buff加成的最小攻击力
    pub fn effective_min_attack(&self) -> i32 {
        let base = self.min_attack + self.bonus_min_attack;
        let buff_bonus = crate::combat::buff::get_stat_bonus(
            &self.buffs,
            &crate::combat::buff::BuffType::AttackBoost { bonus: 0 },
        );
        (base + buff_bonus).max(0)
    }

    /// 计算包含装备+Buff加成的最大攻击力（#1508：Curse 按 C# MaxDCRatePercent 降低输出；#1517：Slaying 被动 MaxDC）
    pub fn effective_max_attack(&self) -> i32 {
        let base = self.max_attack + self.bonus_max_attack;
        let buff_bonus = crate::combat::buff::get_stat_bonus(
            &self.buffs,
            &crate::combat::buff::BuffType::AttackBoost { bonus: 0 },
        );
        let slaying_bonus = slaying_max_dc(&self.magics);
        let curse = crate::combat::buff::get_stat_bonus(
            &self.buffs,
            &crate::combat::buff::BuffType::Curse { percent: 0 },
        );
        // #1888：RhinoPriestDebuff 固定降低 MaxDC（C# RhinoPriest.cs:91）
        let rhino = crate::combat::buff::get_rhino_priest_debuff(&self.buffs);
        ((base + buff_bonus + slaying_bonus) * (100 - curse) / 100 + rhino.0).max(self.effective_min_attack())
    }

    pub fn effective_min_mc(&self) -> i32 {
        let base = self.min_mc + self.bonus_min_mc;
        (base).max(0)
    }

    /// #1508：Curse 按 C# MaxMCRatePercent 降低魔法攻击
    pub fn effective_max_mc(&self) -> i32 {
        let base = self.max_mc + self.bonus_max_mc;
        let buff_bonus = crate::combat::buff::get_stat_bonus(
            &self.buffs,
            &crate::combat::buff::BuffType::McBoost { bonus: 0 },
        );
        let curse = crate::combat::buff::get_stat_bonus(
            &self.buffs,
            &crate::combat::buff::BuffType::Curse { percent: 0 },
        );
        // #1888：RhinoPriestDebuff 固定降低 MaxMC（C# RhinoPriest.cs:91）
        let rhino = crate::combat::buff::get_rhino_priest_debuff(&self.buffs);
        ((base + buff_bonus) * (100 - curse) / 100 + rhino.1).max(self.effective_min_mc())
    }

    pub fn effective_min_sc(&self) -> i32 {
        let base = self.min_sc + self.bonus_min_sc;
        (base).max(0)
    }

    /// #1508：Curse 按 C# MaxSCRatePercent 降低道术
    pub fn effective_max_sc(&self) -> i32 {
        let base = self.max_sc + self.bonus_max_sc;
        let buff_bonus = crate::combat::buff::get_stat_bonus(
            &self.buffs,
            &crate::combat::buff::BuffType::ScBoost { bonus: 0 },
        );
        let curse = crate::combat::buff::get_stat_bonus(
            &self.buffs,
            &crate::combat::buff::BuffType::Curse { percent: 0 },
        );
        // #1888：RhinoPriestDebuff 固定降低 MaxSC（C# RhinoPriest.cs:91）
        let rhino = crate::combat::buff::get_rhino_priest_debuff(&self.buffs);
        ((base + buff_bonus) * (100 - curse) / 100 + rhino.2).max(self.effective_min_sc())
    }

    /// 计算包含装备+Buff加成的防御力
    pub fn effective_defence(&self) -> i32 {
        let base = self.defence + self.bonus_defence;
        let buff_bonus = crate::combat::buff::get_stat_bonus(
            &self.buffs,
            &crate::combat::buff::BuffType::DefenseBoost { bonus: 0 },
        );
        (base + buff_bonus).max(0)
    }

    // ===== 战斗公式扩展：AC/MAC 防御 =====

    pub fn effective_min_ac(&self) -> i32 {
        (self.min_ac + self.bonus_min_ac).max(0)
    }

    pub fn effective_max_ac(&self) -> i32 {
        (self.max_ac + self.bonus_max_ac).max(self.effective_min_ac())
    }

    pub fn effective_min_mac(&self) -> i32 {
        (self.min_mac + self.bonus_min_mac).max(0)
    }

    pub fn effective_max_mac(&self) -> i32 {
        (self.max_mac + self.bonus_max_mac).max(self.effective_min_mac())
    }

    /// 构建战斗公式用的属性快照（对齐 C# Stats 投影到 CombatStats）
    pub fn to_combat_stats(&self) -> crate::combat::attack::CombatStats {
        use crate::combat::attack::CombatStats;
        use crate::combat::buff::{get_stat_bonus, BuffType};
        // C# ProcessPoison：红毒降防（-0.10）/ 眩晕增伤（+0.20）
        let (mut armour_rate, mut damage_rate) = (1.0f32, 1.0f32);
        for p in &self.poison_list {
            if p.p_type.intersects(mir2_shared::enums::PoisonType::RED) {
                armour_rate -= 0.10;
            }
            if p.p_type.intersects(mir2_shared::enums::PoisonType::STUN) {
                damage_rate += 0.20;
            }
        }
        CombatStats {
            min_atk: self.effective_min_attack(),
            max_atk: self.effective_max_attack(),
            min_ac: self.effective_min_ac(),
            max_ac: self.effective_max_ac(),
            min_mac: self.effective_min_mac(),
            max_mac: self.effective_max_mac(),
            agility: self.agility + get_stat_bonus(&self.buffs, &BuffType::AgilityBoost { bonus: 0 }),
            accuracy: self.accuracy + spirit_sword_accuracy(&self.magics) + fencing_accuracy(&self.magics) + slaying_accuracy(&self.magics),
            luck: self.luck,
            critical_rate: self.critical_rate + get_stat_bonus(&self.buffs, &BuffType::CriticalRateBoost { bonus: 0 }),
            critical_damage: self.critical_damage,
            magic_resist: self.magic_resist,
            reflect: self.reflect + get_stat_bonus(&self.buffs, &BuffType::Reflect { percent: 0 }),
            damage_reduction_percent: self.damage_reduction_percent,
            attack_bonus: self.attack_bonus,
            hp_drain_rate_percent: self.hp_drain_rate_percent,
            energy_shield_percent: self.energy_shield_percent,
            energy_shield_hp_gain: self.energy_shield_hp_gain,
            armour_rate,
            damage_rate,
            // C# MentorDamageRatePercent：导师伤害加成（徒弟近身同组时 +10%）
            attacker_damage_rate: if self.mentor_damage_bonus { 1.1 } else { 1.0 },
            freezing: self.freezing,
            poison_attack: self.poison_attack,
            // C# SpecialItemMode.Paralize：任意装备带 Paralize 特殊模式（1/14 概率麻痹，Random.Next(1,15)==1）
            paralize: self.inventory.equipment.iter().flatten().any(|e| {
                e.info.as_ref().map(|i| i.unique.contains(mir2_shared::enums::SpecialItemMode::PARALIZE)).unwrap_or(false)
            }),
        }
    }
}

/// 战士 Fencing 被动：Accuracy + 3×Lv（C# HumanObject.RefreshStats：Stats[Stat.Accuracy] += magic.Level * 3）
pub fn fencing_accuracy(magics: &[PlayerMagic]) -> i32 {
    magics
        .iter()
        .find(|m| m.spell == 1) // Fencing C# 编号 = 1
        .map(|m| m.level as i32 * 3)
        .unwrap_or(0)
}

/// #427 道士 SpiritSword 被动：Accuracy +[0,3,5,8][Lv]（C# HumanObject.cs:2312 spiritSwordLvPlus）
pub fn spirit_sword_accuracy(magics: &[PlayerMagic]) -> i32 {
    const LV_PLUS: [i32; 4] = [0, 3, 5, 8];
    magics
        .iter()
        .find(|m| m.spell == (mir2_shared::enums::Spell::SpiritSword as i32 - 3))
        .map(|m| LV_PLUS[(m.level as usize).min(3)])
        .unwrap_or(0)
}

/// #1517：战士 Slaying 被动——MaxDC + [5,6,7,8][Lv]（C# HumanObject.cs:2297 slayingLvPlus）
pub fn slaying_max_dc(magics: &[PlayerMagic]) -> i32 {
    const LV_PLUS: [i32; 4] = [5, 6, 7, 8];
    magics
        .iter()
        .find(|m| m.spell == (mir2_shared::enums::Spell::Slaying as i32 - 3))
        .map(|m| LV_PLUS[(m.level as usize).min(3)])
        .unwrap_or(0)
}

/// #1517：战士 Slaying 被动——Accuracy + Lv（C# HumanObject.cs:2297）
pub fn slaying_accuracy(magics: &[PlayerMagic]) -> i32 {
    magics
        .iter()
        .find(|m| m.spell == (mir2_shared::enums::Spell::Slaying as i32 - 3))
        .map(|m| m.level as i32)
        .unwrap_or(0)
}

/// PlayerActor 状态
pub struct PlayerActor {
    pub state: PlayerState,
    /// GateActor 引用，用于发数据给客户端
    gate_ref: ActorRef<GateActor>,
    /// WorldActor 引用（#283：升级时通知广播 ObjectLeveled）
    world_ref: ActorRef<crate::actors::world::WorldActor>,
    /// 当前地图数据（用于边界+障碍物校验）
    map_data: Option<MapData>,
}


/// M44：Buff 类型 → 客户端 tag（与 Client-Bevy buff.rs 名称表对应）
fn buff_tag(t: &crate::combat::buff::BuffType) -> u8 {
    use crate::combat::buff::BuffType;
    match t {
        BuffType::HpRegen { .. } => 0,
        BuffType::MpRegen { .. } => 1,
        BuffType::AttackBoost { .. } => 2,
        BuffType::DefenseBoost { .. } => 3,
        BuffType::AcDefenseBoost { .. } => 4,
        BuffType::MacDefenseBoost { .. } => 5,
        BuffType::DamageReduction { .. } => 6,
        BuffType::Poison { .. } => 7,
        BuffType::Silence => 8,
        BuffType::Stun => 9,
        BuffType::Invisibility => 10,
        BuffType::AttackSpeedBoost { .. } => 11,
        BuffType::MoveSpeedBoost { .. } => 12,
        BuffType::AgilityBoost { .. } => 13,
        BuffType::CriticalRateBoost { .. } => 14,
        BuffType::MpRegenBoost { .. } => 15,
        BuffType::MaxMpBoost { .. } => 16,
        BuffType::Reflect { .. } => 17,
        BuffType::Taunt => 18,
        BuffType::Slow { .. } => 19,
        BuffType::Frozen => 20,
        BuffType::McBoost { .. } => 21,
        BuffType::ScBoost { .. } => 22,
        BuffType::Transform { .. } => 23,
        BuffType::TeleportManaPenalty { .. } => 24,
        BuffType::Curse { .. } => 25,
        BuffType::RhinoPriestDebuff { .. } => 26,
    }
}

impl PlayerActor {
    pub fn new(
        object_id: u32,
        name: String,
        session_id: u64,
        map_index: u16,
        gate_ref: ActorRef<GateActor>,
        world_ref: ActorRef<crate::actors::world::WorldActor>,
    ) -> Self {
        Self {
            state: PlayerState {
                object_id,
                name,
                map_index,
                x: 330,
                y: 330,
                direction: 4, // Down
                attack_mode: mir2_shared::enums::AttackMode::Peace,
                pet_mode: mir2_shared::enums::PetMode::Both,
                hidden: false,
                session_id,
                class: mir2_shared::enums::MirClass::Warrior,
                gender: mir2_shared::enums::MirGender::Male,
                hair: 0,
                level: 1,
                experience: 0,
                max_experience: 100,
        can_gain_exp: true,
        pearl_count: 0,
                hp: 120,
                max_hp: 120,
                mp: 60,
                max_mp: 60,
                min_attack: 5,
                max_attack: 10,
                defence: 2,
                min_mc: 0,
                max_mc: 0,
                min_sc: 0,
                max_sc: 0,
                bonus_min_attack: 0,
                bonus_max_attack: 0,
                bonus_defence: 0,
                bonus_max_hp: 0,
                bonus_max_mp: 0,
                bonus_min_mc: 0,
                bonus_max_mc: 0,
                bonus_min_sc: 0,
                bonus_max_sc: 0,
                freezing: 0,
                poison_attack: 0,
            health_recovery: 0,
            spell_recovery: 0,
            attack_speed: 0,
            poison_resist: 0,
                poison_recovery: 0,
                holy: 0,
                accuracy: 0,
                agility: 0,
                min_ac: 0,
                max_ac: 0,
                min_mac: 0,
                max_mac: 0,
                bonus_min_ac: 0,
                bonus_max_ac: 0,
                bonus_min_mac: 0,
                bonus_max_mac: 0,
                luck: 0,
                critical_rate: 0,
                critical_damage: 0,
                magic_resist: 0,
                reflect: 0,
                damage_reduction_percent: 0,
                attack_bonus: 0,
                hp_drain_rate_percent: 0,
                energy_shield_percent: 0,
                energy_shield_hp_gain: 0,
                poison_list: Vec::new(),
                inventory: PlayerInventory::new(),
                group_id: None,
                friend_list: FriendList::new(),
                mailbox: Mailbox::new(),
                guild_name: None,
                guild_rank: GuildRank::Member,
                quest_log: QuestLog::new(),
                spouse_name: None,
                married_date: 0,
                allow_mentor: false,
                mentor_name: None,
                creature_log: CreatureLog::new(),
                hero_index: 0,
                hero_behaviour: 0,
                hero_despawned: false,
                auto_pot_hp: 0,
                auto_pot_mp: 0,
                auto_pot_hp_item: 0,
                auto_pot_mp_item: 0,
                hero_inventory: PlayerInventory::new(),
                hero_magics: Vec::new(),
                refine_log: RefineLog::new(),
                is_fishing: false,
                is_mounted: false,
                mount_type: 0,
                is_dead: false,
            unlock_curse: false,
            last_revival_time: 0,
            last_access: 0,
            rested_counter: 0,
            rested_exp_percent: 0,
            rested_exp_end_tick: 0,
            has_map_shout: false,
            has_server_shout: false,
            last_shout_time: 0,
                pk_points: 0,
                pk_kill_count: 0,
                fishing_autocast: false,
                reincarnation_host: None,
                reincarnation_ready: false,
                reincarnation_expire_time: 0,
                enable_group_recall: false,
                last_recall_time: 0,
                allow_lover_recall: false,
                is_gm: false,
                gm_never_die: false, // #1480：GM 无敌模式（C# GMNeverDie）
                special_shot_armed: 0, // #1483：弓手特殊箭武装（0=无 1=Vampire 2=Poison）
                has_expanded_storage: false,
                expanded_storage_expiry_date: 0,
                has_storage_password: false,
                require_storage_password: false,
                storage_password_last_set: 0,
                allow_observe: false,
                enable_guild_invite: false,
allow_trade: false,

allow_group: false,
                buffs: Vec::new(),
                magics: Vec::new(),
                flags: std::collections::HashMap::new(),
                exp_multiplier: 1.0,
                exp_rate: 1.0,
                exp_multiplier_end_tick: 0,
            drop_multiplier: 1.0,
            drop_multiplier_end_tick: 0,
            item_drop_rate_percent: 0,
            gold_drop_rate_percent: 0,
            elements_level: 0,
            has_elemental: false,
            concentration_interrupted: false,
            concentration_interrupt_time: 0,
            bind_map_index: 0,
            bind_x: 0,
            bind_y: 0,
            level_effects: 0,
            is_mentor: false,
            mentee_exp: 0,
            mentor_damage_bonus: false,
            newbie_exp_bonus: false,
            exp_bonus_lover_percent: 0,
            exp_bonus_mentee_percent: 0,
            exp_bonus_newbie_percent: 0,
            guild_buff_exp_percent: 0,
            guild_buff_fish_rate_percent: 0,
            no_experience_map: false,
            brown_until_ms: 0,
            mount_loyalty_decrease_time: 0,
            mount_loyalty_increase_time: 0,
            torch_burn_time: 0,
            last_damage_ms: 0,
            pot_hp_amount: 0,
            pot_mp_amount: 0,
            pot_time_ms: 0,
            },
            gate_ref,
            world_ref,
            map_data: None,
        }
    }

    /// 设置地图数据
    pub fn set_map_data(&mut self, map: MapData) {
        self.state.no_experience_map = map.no_experience;
        self.map_data = Some(map);
    }

    /// 尝试移动（Walk=1格, Run=2格）
    pub fn try_move(&mut self, direction: u8, steps: i32) -> bool {
        if direction >= 8 {
            warn!("Invalid direction {}", direction);
            return false;
        }

        let dx = DIR_DX[direction as usize];
        let dy = DIR_DY[direction as usize];

        // #1428：C# Walk/Run 对每一格做 ValidPoint 校验（for j=1..=steps）
        for j in 1..=steps {
            let cx = self.state.x + dx * j;
            let cy = self.state.y + dy * j;
            if let Some(ref map) = self.map_data {
                if !map.is_walkable(cx, cy) {
                    debug!("Player {} blocked at ({}, {})", self.state.name, cx, cy);
                    return false;
                }
            }
        }

        // 更新朝向
        self.state.direction = direction;
        self.state.x = self.state.x + dx * steps;
        self.state.y = self.state.y + dy * steps;
        true
    }

    /// 转向（不移动）
    pub fn turn(&mut self, direction: u8) {
        if direction < 8 {
            self.state.direction = direction;
        }
    }

    /// 检查是否有指定类型的 Buff
    fn has_buff(&self, buff_type: crate::combat::buff::BuffType) -> bool {
        let tag = std::mem::discriminant(&buff_type);
        self.state.buffs.iter().any(|b| std::mem::discriminant(&b.buff_type) == tag)
    }

    /// #1319：死亡清理（对齐 C# Die()）——清 Buff 并逐 buff 下发 S.RemoveBuff、毒清空、灰名重置
    fn clear_death_state(&mut self) {
        let tags: Vec<u8> = self.state.buffs.iter().map(|b| buff_tag(&b.buff_type)).collect();
        self.state.buffs.clear();
        self.state.poison_list.clear();
        self.state.brown_until_ms = 0;
        for tag in tags {
            let mut body = Vec::new();
            body.push(tag);
            let _ = self.gate_ref.tell(SendToClient {
                session_id: self.state.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::RemoveBuff as i16, &body),
            }).try_send();
        }
    }

    /// 发送 UserLocation 给玩家
    fn send_user_location(&self) {
        let mut body = Vec::new();
        body.extend_from_slice(&self.state.x.to_le_bytes());
        body.extend_from_slice(&self.state.y.to_le_bytes());
        body.push(self.state.direction);
        let _ = self.gate_ref.tell(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UserLocation as i16, &body),
        }).try_send();
    }
}

impl Actor for PlayerActor {
    type Args = (u32, String, u64, u16, ActorRef<GateActor>, ActorRef<crate::actors::world::WorldActor>);
    type Error = anyhow::Error;

    async fn on_start(
        args: Self::Args,
        _actor_ref: ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        let (object_id, name, session_id, map_index, gate_ref, world_ref) = args;
        debug!("PlayerActor spawned: {} (object_id={}, session={})", name, object_id, session_id);
        Ok(Self::new(object_id, name, session_id, map_index, gate_ref, world_ref))
    }
}

// ============================================================
// 消息定义
// ============================================================

/// 移动类型
#[derive(Debug, Clone, Copy)]
pub enum MoveType {
    Walk,
    Run,
    Turn,
}

/// 移动请求（从 WorldActor 转发）
pub struct MoveRequest {
    pub session_id: u64,
    pub direction: u8,
    pub is_run: bool, // true = Run (2格), false = Walk (1格)
}

/// 转向请求
pub struct TurnRequest {
    pub session_id: u64,
    pub direction: u8,
}

/// 广播移动给其他玩家（其他 PlayerActor 收到此消息后发给自己的客户端）
pub struct BroadcastMovement {
    pub object_id: u32,
    pub x: i32,
    pub y: i32,
    pub direction: u8,
    pub move_type: MoveType,
    pub exclude_session: u64,
}

/// 获取玩家状态（用于广播/序列化）
pub struct GetPlayerState;

/// 设置地图数据
pub struct SetMapData {
    pub map: MapData,
}

/// 设置经验倍率
pub struct SetExpMultiplier {
    pub multiplier: f64,
    pub end_tick: u64,
}

impl Message<SetExpMultiplier> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetExpMultiplier,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.exp_multiplier = msg.multiplier.max(1.0);
        self.state.exp_multiplier_end_tick = msg.end_tick;
    }
}

/// 设置玩家掉落倍率（Potion shape 5 Drop Buff，C# BuffType.Drop）
pub struct SetDropMultiplier {
    pub multiplier: f64,
    pub end_tick: u64,
}

impl Message<SetDropMultiplier> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetDropMultiplier,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.drop_multiplier = msg.multiplier.max(1.0);
        self.state.drop_multiplier_end_tick = msg.end_tick;
    }
}

/// 设置诅咒解锁状态（C# UnlockCurse：神秘水解除诅咒装备卸装锁定）
pub struct SetUnlockCurse {
    pub unlock: bool,
}

impl Message<SetUnlockCurse> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetUnlockCurse,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.unlock_curse = msg.unlock;
    }
}

/// 设置物品 info（WorldActor 装备成功后补 ItemInfo，供复活戒指等逻辑读取）
pub struct SetItemInfo {
    pub unique_id: u64,
    pub info: Option<mir2_shared::data::item::ItemInfo>,
}

impl Message<SetItemInfo> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetItemInfo, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        for slot in self.state.inventory.equipment.iter_mut() {
            if let Some(item) = slot {
                if item.unique_id == msg.unique_id {
                    item.info = msg.info;
                    return;
                }
            }
        }
        for s in self.state.inventory.backpack.iter_mut().flatten() {
            if s.item.unique_id == msg.unique_id {
                s.item.info = msg.info;
                return;
            }
        }
    }
}

/// 设置休息累积计数（C# _restedCounter）
pub struct SetRestedCounter {
    pub counter: u32,
}

impl Message<SetRestedCounter> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetRestedCounter, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.rested_counter = msg.counter;
    }
}

/// 设置休息经验加成（C# GiveRestedBonus：BuffType.Rested + ExpRatePercent）
pub struct SetRestedExp {
    pub percent: u32,
    pub end_tick: u64,
}

impl Message<SetRestedExp> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetRestedExp, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.rested_exp_percent = msg.percent;
        self.state.rested_exp_end_tick = msg.end_tick;
    }
}

/// 休息加成到账（C# GiveRestedBonus(count)）：按 count 累加时长（分钟），上限 max_bonus 份
pub struct GiveRestedBonus {
    pub count: u32,
    pub buff_length_minutes: u32,
    pub exp_bonus_percent: u32,
    pub max_bonus: u32,
}

impl Message<GiveRestedBonus> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: GiveRestedBonus, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if msg.count == 0 {
            return;
        }
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let buff_ms = msg.buff_length_minutes.max(1) as i64 * 60_000;
        let existing = self.state.rested_exp_end_tick.saturating_sub(now_ms.max(0) as u64) as i64;
        let add = msg.count as i64 * buff_ms;
        let max_dur = msg.max_bonus.max(1) as i64 * buff_ms;
        let total = (existing + add).min(max_dur).max(0);
        self.state.rested_exp_percent = msg.exp_bonus_percent;
        self.state.rested_exp_end_tick = (now_ms + total).max(0) as u64;
        self.state.rested_counter = 0;
        crate::actors::world::send_system_message(
            &self.gate_ref,
            self.state.session_id,
            &format!("休息经验加成已生效：+{}% 经验（剩余 {} 分钟）", msg.exp_bonus_percent, total / 60_000),
        );
        debug!("Player {} rested bonus: +{}% for {} min", self.state.name, msg.exp_bonus_percent, total / 60_000);
    }
}

/// 设置喊话状态（C# HasMapShout/HasServerShout/ShoutTime）
pub struct SetShoutState {
    pub map_shout: bool,
    pub server_shout: bool,
    pub last_shout_time: i64,
}

impl Message<SetShoutState> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetShoutState, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.has_map_shout = msg.map_shout;
        self.state.has_server_shout = msg.server_shout;
        self.state.last_shout_time = msg.last_shout_time;
    }
}

/// 设置玩家状态（用于从数据库加载后初始化）
pub struct SetPlayerState {
    pub state: PlayerState,
}

impl Message<SetPlayerState> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetPlayerState,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state = msg.state;
    }
}

/// 复活玩家：重置 HP/MP 到最大值，设置位置
/// 转职（NPC 脚本 ChangeClass 用）
pub struct ChangeClass {
    pub class: mir2_shared::enums::MirClass,
}

impl Message<ChangeClass> for PlayerActor {
    type Reply = ();
    async fn handle(&mut self, msg: ChangeClass, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.class = msg.class;
    }
}

/// 改发型（NPC 脚本 ChangeHair 用）
pub struct SetHair {
    pub hair: u8,
}

impl Message<SetHair> for PlayerActor {
    type Reply = ();
    async fn handle(&mut self, msg: SetHair, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.hair = msg.hair;
    }
}

pub struct RevivePlayer {
    pub x: i32,
    pub y: i32,
    pub map_index: u16,
}

impl Message<RevivePlayer> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: RevivePlayer, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.is_dead = false;
        self.state.x = msg.x;
        self.state.y = msg.y;
        self.state.hp = self.state.max_hp;
        self.state.mp = self.state.max_mp;
        // 发送位置更新
        self.send_user_location();
        true
    }
}

// ============================================================
// Handler 实现
// ============================================================

/// #1614：C# HumanObject.CanWalk——麻痹/冰冻毒禁止移动（Paralysis/LRParalysis/Frozen）
pub(crate) fn movement_blocked_by_poison(
    poison_list: &[crate::combat::poison::Poison],
) -> bool {
    poison_list.iter().any(|p| {
        p.p_type.intersects(mir2_shared::enums::PoisonType::PARALYSIS)
            || p.p_type.intersects(mir2_shared::enums::PoisonType::LR_PARALYSIS)
            || p.p_type.intersects(mir2_shared::enums::PoisonType::FROZEN)
    })
}

impl Message<MoveRequest> for PlayerActor {
    type Reply = bool; // success

    async fn handle(
        &mut self,
        msg: MoveRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if self.has_buff(crate::combat::buff::BuffType::Stun) {
            return false;
        }
        // #1614：C# CanWalk——麻痹/冰冻毒禁止移动
        if movement_blocked_by_poison(&self.state.poison_list) {
            return false;
        }
        // #1428/#1502：C# HumanObject.Run steps = RidingMount || (ActiveSwiftFeet && !Sneaking) ? 3 : 2；Walk = 1
        let steps = if msg.is_run {
            if self.state.is_mounted
                || self.has_buff(crate::combat::buff::BuffType::MoveSpeedBoost { percent: 0 }) {
                3
            } else {
                2
            }
        } else {
            1
        };
        let success = self.try_move(msg.direction, steps);

        if success {
            debug!(
                "Player {} moved {} to ({}, {}) dir={}",
                self.state.name,
                if msg.is_run { "run" } else { "walk" },
                self.state.x,
                self.state.y,
                msg.direction
            );
            self.send_user_location();
        }

        success
    }
}

impl Message<TurnRequest> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: TurnRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if self.has_buff(crate::combat::buff::BuffType::Stun) {
            return;
        }
        self.turn(msg.direction);
        debug!("Player {} turned to dir={}", self.state.name, msg.direction);
        self.send_user_location();
    }
}

impl Message<BroadcastMovement> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: BroadcastMovement,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // 不给自己发
        if self.state.session_id == msg.exclude_session {
            return;
        }

        let opcode = match msg.move_type {
            MoveType::Walk => mir2_shared::enums::ServerPacketIds::ObjectWalk,
            MoveType::Run => mir2_shared::enums::ServerPacketIds::ObjectRun,
            MoveType::Turn => mir2_shared::enums::ServerPacketIds::ObjectTurn,
        };

        let mut body = Vec::new();
        body.extend_from_slice(&msg.object_id.to_le_bytes());
        body.extend_from_slice(&msg.x.to_le_bytes());
        body.extend_from_slice(&msg.y.to_le_bytes());
        body.push(msg.direction);

        let _ = self.gate_ref.tell(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(opcode as i16, &body),
        }).await;
    }
}

impl Message<GetPlayerState> for PlayerActor {
    type Reply = Option<PlayerState>;

    async fn handle(
        &mut self,
        _msg: GetPlayerState,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        Some(self.state.clone())
    }
}

impl Message<SetMapData> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetMapData,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.set_map_data(msg.map);
    }
}

/// 攻击请求（从 WorldActor 转发）
pub struct AttackRequest {
    pub session_id: u64,
    pub direction: u8,
    pub spell: u8,
}

/// 受到伤害（从 WorldActor 转发，其他玩家攻击到自己）
pub struct TakeDamage {
    pub attacker_id: u32,
    pub attacker_session: u64,
    pub damage: i32,
}

impl Message<AttackRequest> for PlayerActor {
    type Reply = Option<AttackResult>;

    async fn handle(
        &mut self,
        msg: AttackRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if self.has_buff(crate::combat::buff::BuffType::Stun) {
            return None;
        }
        if msg.spell != 0 && self.has_buff(crate::combat::buff::BuffType::Silence) {
            return None;
        }
        if msg.direction < 8 {
            self.state.direction = msg.direction;
        }

        // Spell::None = 基本近战攻击
        // Phase 1：只处理近战，无目标验证，纯视觉效果
        debug!(
            "Player {} attacks: dir={} spell={}",
            self.state.name, msg.direction, msg.spell
        );

        // 广播 ObjectAttack 给其他玩家
        Some(AttackResult {
            object_id: self.state.object_id,
            x: self.state.x,
            y: self.state.y,
            direction: self.state.direction,
            spell: msg.spell,
        })
    }
}

impl PlayerActor {
    /// 查找可用的复活戒指槽位（C# Die：RingL/RingR 且 SpecialItemMode.Revival && CurrentDura >= 1000）
    fn try_revival_ring(&self) -> Option<usize> {
        use mir2_shared::enums::SpecialItemMode;
        for idx in [
            crate::actors::inventory::EquipmentSlot::RingL as usize,
            crate::actors::inventory::EquipmentSlot::RingR as usize,
        ] {
            if let Some(ring) = self.state.inventory.equipment.get(idx).and_then(|s| s.as_ref()) {
                let has_revival = ring
                    .info
                    .as_ref()
                    .map(|i| i.unique.contains(SpecialItemMode::REVIVAL))
                    .unwrap_or(false);
                if has_revival && ring.current_dura >= 1000 {
                    return Some(idx);
                }
            }
        }
        None
    }
}

impl Message<TakeDamage> for PlayerActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        msg: TakeDamage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let damage = msg.damage.max(0);
        // #1283：C# Attacked——受击重置自然回血计时（RegenTime = now + RegenDelay(10s)）
        if damage > 0 {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            self.state.last_damage_ms = now_ms;
        }
        // #942：C# SpecialItemMode.Protection——装备含 Protection 且 MP>0 时伤害全部由 MP 吸收
        // （HumanObject.ChangeHP → ChangeMP(amount)，不致死；Struck 动画照常）
        if damage > 0
            && self.state.mp > 0
            && self.state.inventory.equipment.iter().flatten()
                .any(|it| it.info.as_ref().map(|i| i.unique.contains(mir2_shared::enums::SpecialItemMode::PROTECTION)).unwrap_or(false))
        {
            self.state.mp = (self.state.mp - damage).max(0);
            let mut struck_body = Vec::new();
            struck_body.extend_from_slice(&msg.attacker_id.to_le_bytes());
            let _ = self.gate_ref.tell(SendToClient {
                session_id: self.state.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Struck as i16, &struck_body),
            }).await;
            let mut hb = Vec::new();
            hb.extend_from_slice(&(self.state.hp as u32).to_le_bytes());
            hb.extend_from_slice(&(self.state.mp as u32).to_le_bytes());
            let _ = self.gate_ref.tell(SendToClient {
                session_id: self.state.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &hb),
            }).await;
            debug!("Player {} absorbed {} damage with MP (Protection)", self.state.name, damage);
            return false;
        }
        self.state.hp = (self.state.hp - damage).max(0);

        // C# HumanObject.Attacked：被玩家攻击时攻击者获得灰名（BrownTime，世界侧校验 PK/开战）
        if msg.attacker_session != 0 {
            let _ = self.world_ref
                .tell(crate::actors::world::partners::MarkBrown {
                    attacker_session: msg.attacker_session,
                    victim_session: self.state.session_id,
                })
                .try_send();
        }

        debug!(
            "Player {} took {} damage from object_id={} (hp: {}/{})",
            self.state.name, damage, msg.attacker_id, self.state.hp, self.state.max_hp
        );

        // 发送 Struck（自己被攻击的动画）
        let mut struck_body = Vec::new();
        struck_body.extend_from_slice(&msg.attacker_id.to_le_bytes());
        let _ = self.gate_ref.tell(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Struck as i16, &struck_body),
        }).await;

        // #1480：GM 无敌（C# GMNeverDie）——HP 钳到 ≥1，不进入死亡流程
        if self.state.gm_never_die {
            if self.state.hp <= 0 {
                self.state.hp = 1;
                let mut hb = Vec::new();
                hb.extend_from_slice(&(self.state.hp as u32).to_le_bytes());
                hb.extend_from_slice(&(self.state.mp as u32).to_le_bytes());
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: self.state.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &hb),
                }).await;
            }
            return false;
        }
        // 死亡处理
        if self.state.hp <= 0 && !self.state.is_dead {
            // C# Die()：复活戒指（SpecialItemMode.Revival）——回满血、扣 1000 耐久、5 分钟冷却
            if let Some(ring_idx) = self.try_revival_ring() {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);
                if now_ms >= self.state.last_revival_time {
                    let (ring_uid, ring_dura) = {
                        let ring = self.state.inventory.equipment[ring_idx].as_mut().unwrap();
                        ring.current_dura = ring.current_dura.saturating_sub(1000);
                        ring.dura_changed = true;
                        (ring.unique_id, ring.current_dura)
                    };
                    self.state.last_revival_time = now_ms + 300_000;
                    self.state.hp = self.state.max_hp;
                    // S.DuraChanged（C# Die：item.CurrentDura -= 1000）
                    let dc = mir2_shared::packets::server::experience::DuraChanged {
                        unique_id: ring_uid,
                        current_dura: ring_dura,
                    };
                    let mut dc_body = Vec::new();
                    if dc.write_body(&mut dc_body).is_ok() {
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: self.state.session_id,
                            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::DuraChanged as i16, &dc_body),
                        }).await;
                    }
                    // S.HealthChanged 回满血
                    let mut hb = Vec::new();
                    hb.extend_from_slice(&(self.state.hp as u32).to_le_bytes());
                    hb.extend_from_slice(&(self.state.mp as u32).to_le_bytes());
                    let _ = self.gate_ref.tell(SendToClient {
                        session_id: self.state.session_id,
                        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &hb),
                    }).await;
                    self.send_equipment_changed();
                    debug!("Player {} revived by ring (dura={})", self.state.name, ring_dura);
                    return false;
                }
            }
            self.state.is_dead = true;
            // #1319：C# Die()——清 Buff（逐 buff 下发 S.RemoveBuff）+ 毒清空 + 灰名重置
            self.clear_death_state();
            debug!("Player {} died (attacker={})", self.state.name, msg.attacker_id);

            // 发送 S.Death 包给死亡玩家（C# Shared/ServerPackets.cs Death: [Location Point][Direction u8]）
            // 之前误发空 body，客户端 read_body 解析失败 → 不进入死亡状态（#55 实测发现）
            let mut death_body = Vec::new();
            death_body.extend_from_slice(&self.state.x.to_le_bytes());
            death_body.extend_from_slice(&self.state.y.to_le_bytes());
            death_body.push(self.state.direction);
            let _ = self.gate_ref.tell(SendToClient {
                session_id: self.state.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Death as i16, &death_body),
            }).await;
            // S.ObjectDied 广播由 WorldActor 的 combat.rs 死亡分支处理（已实现）

            return true;
        }

        // 发送 HealthChanged
        if self.state.hp > 0 {
            let mut health_body = Vec::new();
            health_body.extend_from_slice(&(self.state.hp as u32).to_le_bytes());
            health_body.extend_from_slice(&(self.state.mp as u32).to_le_bytes());
            let _ = self.gate_ref.tell(SendToClient {
                session_id: self.state.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &health_body),
            }).await;
        }
        false
    }
}

/// 获得经验（从 WorldActor 转发）
pub struct AddExperience {
    pub amount: i32,
    /// C# Settings.ExperienceList（索引=Level-1）；空表时 PlayerActor 回退 ×1.5
    pub experience_list: Vec<i64>,
}

impl Message<AddExperience> for PlayerActor {
    /// 返回实际获得经验（扣除前基础量、含全部加成后的最终值；C# GainExp 宠物经验用）
    type Reply = i64;

    async fn handle(
        &mut self,
        msg: AddExperience,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // 对齐 C# CanGainExp：关闭时不给经验
        if !self.state.can_gain_exp {
            return 0;
        }
        // #932：C# MapInfo.NoExperience——无经验地图不给经验（GainExp/WinExp 入口拦截）。
        // 使用 set_map_data 时缓存的标志，避免 WorldActor tick 内反向 ask 死锁。
        if self.state.no_experience_map {
            return 0;
        }
        let base = msg.amount.max(0) as i64;
        // 休息经验加成（C# BuffType.Rested ExpRatePercent 累加到 ExpRatePercent）
        let rested_mul = 1.0 + self.state.rested_exp_percent as f64 / 100.0;
        // 配偶/徒弟/新手行会经验加成：使用 tick_partner_bonuses 缓存的百分比
        //（C# GainExp 语义；避免在 WorldActor tick 内反向 ask WorldActor 造成死锁）
        let lover_bonus = self.state.exp_bonus_lover_percent;
        let mentee_bonus = self.state.exp_bonus_mentee_percent;
        let newbie_bonus = if self.state.newbie_exp_bonus {
            self.state.exp_bonus_newbie_percent
        } else {
            0
        };
        // 行会 Buff 经验加成（C# GuildBuffInfo.BuffExpRate → Stat.ExpRatePercent）
        let guild_buff_bonus = self.state.guild_buff_exp_percent;
        let amount = (base as f64 * self.state.exp_multiplier * self.state.exp_rate * rested_mul
            * (1.0 + lover_bonus as f64 / 100.0)
            * (1.0 + mentee_bonus as f64 / 100.0)
            * (1.0 + newbie_bonus as f64 / 100.0)
            * (1.0 + guild_buff_bonus as f64 / 100.0)).round() as i64;
        self.state.experience += amount;

        // C# GainExp：徒弟经验积累 MenteeEXP += amount * Settings.MenteeExpBank(1) / 100
        if self.state.mentor_name.is_some() && !self.state.is_mentor {
            self.state.mentee_exp += (amount * 1) / 100;
        }

        // C# GainExp：行会获得经验（MyGuild.GainExp；新手行会由 WorldActor 侧过滤）
        if self.state.guild_name.is_some() {
            let _ = self.world_ref.tell(crate::actors::world::GuildExpEarned {
                session_id: self.state.session_id,
                amount,
            }).try_send();
        }

        debug!(
            "Player {} gained {} exp (base={} x{:.1}) (total={}/{})",
            self.state.name, amount, base, self.state.exp_multiplier, self.state.experience, self.state.max_experience
        );

        // 发送 GainExperience 给客户端
        let mut body = Vec::new();
        body.extend_from_slice(&(amount as u32).to_le_bytes());
        let _ = self.gate_ref.tell(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GainExperience as i16, &body),
        }).await;

        // 检查升级（用 SharedRust BaseStats 公式计算属性，对齐 C# RefreshLevelStats）
        const MAX_LEVEL: u16 = 200;
        while self.state.experience >= self.state.max_experience && self.state.level < MAX_LEVEL {
            self.state.experience -= self.state.max_experience;
            self.state.level += 1;
            // #283：通知 WorldActor 广播 ObjectLeveled 给同图其他玩家
            let _ = self.world_ref
                .tell(crate::actors::world::PlayerLeveled {
                    session_id: self.state.session_id,
                    object_id: self.state.object_id,
                    level: self.state.level,
                })
                .try_send();

            // 按 BaseStats 公式重算所有基础属性（对齐 C# Settings.ClassBaseStats[Class].Calculate）
            let base_stats = mir2_shared::data::stats::BaseStats::new(self.state.class);
            for bs in &base_stats.stats {
                let val = bs.calculate(self.state.class, self.state.level as i32);
                use mir2_shared::enums::Stat;
                match bs.stat {
                    Stat::HP => { self.state.max_hp = val; self.state.hp = val; }
                    Stat::MP => { self.state.max_mp = val; self.state.mp = val; }
                    Stat::MinDC => self.state.min_attack = val,
                    Stat::MaxDC => self.state.max_attack = val,
                    Stat::MinMC => self.state.min_mc = val,
                    Stat::MaxMC => self.state.max_mc = val,
                    Stat::MinSC => self.state.min_sc = val,
                    Stat::MaxSC => self.state.max_sc = val,
                    Stat::MinAC => self.state.min_ac = val,
                    Stat::MaxAC => { self.state.max_ac = val; self.state.defence = val; }
                    Stat::MinMAC => self.state.min_mac = val,
                    Stat::MaxMAC => self.state.max_mac = val,
                    Stat::Agility => self.state.agility = val,
                    Stat::Accuracy => self.state.accuracy = val,
                    _ => {}
                }
            }

            // 经验曲线（C# RefreshMaxExperience：MaxExperience = ExperienceList[Level-1]；空表回退 ×1.5）
            let li = (self.state.level as usize).saturating_sub(1);
            self.state.max_experience = if li < msg.experience_list.len() {
                msg.experience_list[li]
            } else if msg.experience_list.is_empty() {
                (self.state.max_experience as f64 * 1.5) as i64
            } else {
                0 // 超出经验表：不再升级（C# 语义）
            };

            info!("Player {} leveled up to {}! (hp={} mp={} atk={}-{} mc={}-{} sc={}-{})",
                  self.state.name, self.state.level, self.state.max_hp, self.state.max_mp,
                  self.state.min_attack, self.state.max_attack,
                  self.state.min_mc, self.state.max_mc,
                  self.state.min_sc, self.state.max_sc);

            // 发送 LevelChanged
            let mut lv_body = Vec::new();
            lv_body.extend_from_slice(&self.state.level.to_le_bytes());
            lv_body.extend_from_slice(&self.state.experience.to_le_bytes());
            lv_body.extend_from_slice(&self.state.max_experience.to_le_bytes());
            let _ = self.gate_ref.tell(SendToClient {
                session_id: self.state.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::LevelChanged as i16, &lv_body),
            }).await;
        }
        amount
    }
}

/// 扣除经验（死亡惩罚等）
pub struct DeductExperience {
    pub amount: i32,
}

impl Message<DeductExperience> for PlayerActor {
    type Reply = i64;

    async fn handle(
        &mut self,
        msg: DeductExperience,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let amount = msg.amount.max(0) as i64;
        let before = self.state.experience;
        self.state.experience = self.state.experience.saturating_sub(amount);
        let deducted = before - self.state.experience;

        debug!(
            "Player {} lost {} exp (total={}/{})",
            self.state.name, deducted, self.state.experience, self.state.max_experience
        );

        // 发送经验更新给客户端
        let mut body = Vec::new();
        body.extend_from_slice(&self.state.experience.to_le_bytes());
        body.extend_from_slice(&self.state.max_experience.to_le_bytes());
        let _ = self.gate_ref.tell(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GainExperience as i16, &body),
        }).await;

        deducted
    }
}

/// 治疗请求（来自 Healing/MassHealing 等魔法）
pub struct Heal {
    pub amount: i32,
}

impl Message<Heal> for PlayerActor {
    type Reply = i32; // 实际回复量

    async fn handle(
        &mut self,
        msg: Heal,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if self.state.is_dead || msg.amount <= 0 {
            return 0;
        }
        let before = self.state.hp;
        self.state.hp = (self.state.hp + msg.amount).min(self.state.max_hp);
        let healed = self.state.hp - before;

        // 发送 HealthChanged 给客户端
        let mut body = Vec::new();
        body.extend_from_slice(&(self.state.hp as u32).to_le_bytes());
        body.extend_from_slice(&(self.state.mp as u32).to_le_bytes());
        let _ = self.gate_ref.tell(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &body),
        }).await;

        debug!("Player {} healed for {} HP ({} -> {})", self.state.name, healed, before, self.state.hp);
        healed
    }
}

/// #1290：使用 NormalPotion 累计药水池（C# PotHealthAmount/PotManaAmount 累加）
pub struct AddPotionPool {
    pub hp: u32,
    pub mp: u32,
}

impl Message<AddPotionPool> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: AddPotionPool,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // #1667：C# PotHealthAmount/PotManaAmount = Min(ushort.MaxValue, ...)（PlayerObject.cs:5832）
        const POT_POOL_CAP: u32 = 65535;
        self.state.pot_hp_amount = self.state.pot_hp_amount.saturating_add(msg.hp).min(POT_POOL_CAP);
        self.state.pot_mp_amount = self.state.pot_mp_amount.saturating_add(msg.mp).min(POT_POOL_CAP);
    }
}

/// #1290：C# ProcessRegen PotTime——每次从药水池扣 min(池, PerTickRegen)，返回 (回复量, 剩余池)
fn potion_tick_regen(pool: u32, per_tick: u32) -> (u32, u32) {
    let heal = pool.min(per_tick);
    (heal, pool - heal)
}

/// #1290：药水池处理（每 PotDelay=200ms 由 WorldActor tick_potion_pools 调用）
pub struct TickPotionPool {
    pub per_tick: u32,
    pub now_ms: i64,
}

impl Message<TickPotionPool> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: TickPotionPool,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.pot_time_ms = msg.now_ms + 200; // C# PotDelay
        let mut changed = false;
        if self.state.pot_hp_amount > 0 {
            let (heal, rem) = potion_tick_regen(self.state.pot_hp_amount, msg.per_tick);
            self.state.pot_hp_amount = rem;
            self.state.hp = (self.state.hp + heal as i32).min(self.state.max_hp);
            changed = true;
        }
        if self.state.pot_mp_amount > 0 {
            let (add, rem) = potion_tick_regen(self.state.pot_mp_amount, msg.per_tick);
            self.state.pot_mp_amount = rem;
            self.state.mp = (self.state.mp + add as i32).min(self.state.max_mp);
            changed = true;
        }
        if changed {
            let mut body = Vec::new();
            body.extend_from_slice(&(self.state.hp as u32).to_le_bytes());
            body.extend_from_slice(&(self.state.mp as u32).to_le_bytes());
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: self.state.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::HealthChanged as i16,
                        &body,
                    ),
                })
                .await;
        }
    }
}

/// 复活请求（WorldActor 在死亡倒计时后调用）
pub struct Revive;

impl Message<Revive> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: Revive,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if !self.state.is_dead {
            return;
        }
        self.state.is_dead = false;
        self.state.hp = self.state.max_hp;
        self.state.mp = self.state.max_mp;

        // 发送 HealthChanged
        let mut body = Vec::new();
        body.extend_from_slice(&self.state.hp.to_le_bytes());
        body.extend_from_slice(&self.state.mp.to_le_bytes());
        let _ = self.gate_ref.tell(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &body),
        }).await;

        // 发送 S.Revived（空 body）：客户端靠它清除死亡状态恢复输入（#55 实测缺失会导致卡死）
        let _ = self.gate_ref.tell(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Revived as i16, &[]),
        }).await;

        debug!("Player {} revived (hp={} mp={})", self.state.name, self.state.hp, self.state.mp);
    }
}

/// 设置元素状态（C# HumanObject.ObtainElement 更新 ElementsLevel/HasElemental）
pub struct SetElements {
    pub level: i32,
    pub has_elemental: bool,
}

impl Message<SetElements> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetElements,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.elements_level = msg.level;
        self.state.has_elemental = msg.has_elemental;
    }
}

/// #1483：设置弓手特殊箭武装（0=无 1=Vampire 2=Poison；C# VampireShot/PoisonShot buff）
pub struct SetSpecialShotArmed {
    pub armed: u8,
}

impl Message<SetSpecialShotArmed> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetSpecialShotArmed, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.special_shot_armed = msg.armed.min(2);
    }
}

/// #1480：设置 GM 无敌模式（C# GMNeverDie，@superman 切换）
pub struct SetGmNeverDie {
    pub enabled: bool,
}

impl Message<SetGmNeverDie> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetGmNeverDie, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.gm_never_die = msg.enabled;
    }
}

/// 设置专注打断状态（C# Concentration buff Interrupted/InterruptTime）
pub struct SetConcentrationInterrupt {
    pub interrupted: bool,
    pub interrupt_time_ms: i64,
}

impl Message<SetConcentrationInterrupt> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetConcentrationInterrupt,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.concentration_interrupted = msg.interrupted;
        self.state.concentration_interrupt_time = msg.interrupt_time_ms;
    }
}

/// 设置绑定点（C# CharacterInfo.BindMapIndex/BindLocation）
pub struct SetBind {
    pub map_index: i32,
    pub x: i32,
    pub y: i32,
}

impl Message<SetBind> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetBind,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.bind_map_index = msg.map_index;
        self.state.bind_x = msg.x;
        self.state.bind_y = msg.y;
    }
}

/// 设置等级特效（C# HumanObject.SetLevelEffects：flags 990-998 派生）
pub struct SetLevelEffects {
    pub effects: u16,
}

impl Message<SetLevelEffects> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetLevelEffects,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.level_effects = msg.effects;
    }
}

/// Buff 应用请求
pub struct ApplyBuff {
    pub buff: crate::combat::buff::BuffInstance,
}

impl Message<ApplyBuff> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ApplyBuff,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        crate::combat::buff::apply_buff(&mut self.state.buffs, msg.buff.clone());
        // M44：推送 AddBuff 给客户端（简化 wire：[tag u8][remaining_ticks u32]）
        let mut body = Vec::new();
        body.push(buff_tag(&msg.buff.buff_type));
        body.extend_from_slice(&msg.buff.remaining_ticks.to_le_bytes());
        let _ = self.gate_ref.tell(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::AddBuff as i16, &body),
        }).try_send();
    }
}

/// 战斗触发的 Poison 施加（冰冻/毒攻等负面效果）
pub struct ApplyCombatPoisons {
    pub poisons: Vec<crate::combat::poison::Poison>,
}

impl Message<ApplyCombatPoisons> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ApplyCombatPoisons,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        for p in msg.poisons {
            crate::combat::poison::apply_poison(&mut self.state.poison_list, p);
        }
    }
}

/// 施加伤害减免（MagicShield/ElementalBarrier，C# Stat.DamageReductionPercent）
/// 直接设 PlayerState.damage_reduction_percent，并加 DamageReduction buff 记录时长
pub struct ApplyDamageReduction {
    pub percent: i32,
    pub duration_ticks: u32,
}

impl Message<ApplyDamageReduction> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ApplyDamageReduction,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.damage_reduction_percent = msg.percent;
        let buff = crate::combat::buff::BuffInstance::new(
            crate::combat::buff::BuffType::DamageReduction { percent: msg.percent },
            msg.duration_ticks,
            1,
        );
        crate::combat::buff::apply_buff(&mut self.state.buffs, buff);
    }
}

/// 解毒：清除自身所有 Poison（道士 Purification 用）
pub struct PurifyPoisons;

impl Message<PurifyPoisons> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: PurifyPoisons,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // #1906：C# PowerBead Effect==1 净化 = 移除全部 Debuff Buff + 清空毒列表
        let tags: Vec<u8> = self.state.buffs.iter()
            .filter(|b| crate::combat::buff::is_debuff(&b.buff_type))
            .map(|b| buff_tag(&b.buff_type))
            .collect();
        self.state.buffs.retain(|b| !crate::combat::buff::is_debuff(&b.buff_type));
        for tag in tags {
            let mut body = Vec::new();
            body.push(tag);
            let _ = self.gate_ref.tell(SendToClient {
                session_id: self.state.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::RemoveBuff as i16, &body),
            }).try_send();
        }
        self.state.poison_list.clear();
    }
}

/// 移除指定类型的 Buff
pub struct RemoveBuff {
    pub buff_type: crate::combat::buff::BuffType,
}

impl Message<RemoveBuff> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: RemoveBuff,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        crate::combat::buff::remove_buff_by_type(&mut self.state.buffs, &msg.buff_type);
        // M44：推送 RemoveBuff（[tag u8]）
        let mut body = Vec::new();
        body.push(buff_tag(&msg.buff_type));
        let _ = self.gate_ref.tell(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::RemoveBuff as i16, &body),
        }).try_send();
    }
}

/// #965：清除全部 Buff（@CLEARBUFFS，C# FlagForRemoval；逐 buff 下发 S.RemoveBuff）
pub struct ClearAllBuffs;

impl Message<ClearAllBuffs> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, _msg: ClearAllBuffs, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let tags: Vec<u8> = self.state.buffs.iter().map(|b| buff_tag(&b.buff_type)).collect();
        self.state.buffs.clear();
        for tag in tags {
            let mut body = Vec::new();
            body.push(tag);
            let _ = self.gate_ref.tell(SendToClient {
                session_id: self.state.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::RemoveBuff as i16, &body),
            }).try_send();
        }
    }
}

/// Buff tick（由 WorldActor 主循环每 tick 调用）
pub struct TickBuff;

impl Message<TickBuff> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: TickBuff,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // C# HumanObject.Process：Mount.HasMount 且每 LoyaltyDelay*60（60s）→ IncreaseMountLoyalty(1)
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        if now_ms >= self.state.mount_loyalty_increase_time {
            self.state.mount_loyalty_increase_time = now_ms + 60_000;
            let slot = crate::actors::inventory::EquipmentSlot::Mount as usize;
            if let Some(mount) = self.state.inventory.equipment[slot].as_mut() {
                if mount.current_dura < mount.max_dura {
                    mount.current_dura = mount.current_dura.saturating_add(1);
                    mount.dura_changed = true;
                    // S.ItemRepaired（C# IncreaseMountLoyalty）
                    let ir = mir2_shared::packets::server::ItemRepaired {
                        unique_id: mount.unique_id,
                        max_dura: mount.max_dura,
                        current_dura: mount.current_dura,
                    };
                    let mut body = Vec::new();
                    if ir.write_body(&mut body).is_ok() {
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: self.state.session_id,
                            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ItemRepaired as i16, &body),
                        }).await;
                    }
                }
            }
        }
        if self.state.buffs.is_empty() {
            return;
        }
        let results = crate::combat::buff::tick_buffs(&mut self.state.buffs, 1);
        let mut total_hp = 0i32;
        let mut total_mp = 0i32;
        for r in &results {
            total_hp += r.hp_change;
            total_mp += r.mp_change;
        }
        if total_hp != 0 {
            self.state.hp = (self.state.hp + total_hp).clamp(0, self.state.max_hp);
        }
        if total_mp != 0 {
            self.state.mp = (self.state.mp + total_mp).clamp(0, self.state.max_mp);
        }
        // 收集过期 buff 的 tag（C# RemoveBuff 客户端通知，格式与 M44 AddBuff 一致：[tag u8]）
        let expired_tags: Vec<u8> = self.state.buffs.iter()
            .filter(|b| b.remaining_ticks == 0)
            .map(|b| buff_tag(&b.buff_type))
            .collect();
        // 移除过期 buff
        crate::combat::buff::expire_buffs(&mut self.state.buffs,
        );
        for tag in expired_tags {
            let mut body = Vec::new();
            body.push(tag);
            let _ = self.gate_ref.tell(SendToClient {
                session_id: self.state.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::RemoveBuff as i16, &body),
            }).try_send();
        }

        // DamageReduction buff 过期后重置 damage_reduction_percent
        // （MagicShield/ElementalBarrier/ProtectionField 的减伤不应永久生效）
        let has_dr = self.state.buffs.iter().any(|b|
            matches!(b.buff_type, crate::combat::buff::BuffType::DamageReduction { .. }));
        if !has_dr {
            self.state.damage_reduction_percent = 0;
        }

        // Poison tick（每 5 ticks 触发一次，推进 1 秒的 duration/伤害，略快于真实时间但可接受）
        if !self.state.poison_list.is_empty() {
            let poison_dmg = crate::combat::poison::tick_poisons(&mut self.state.poison_list, 1);
            if poison_dmg > 0 {
                self.state.hp = (self.state.hp - poison_dmg).max(0);
                total_hp -= poison_dmg;
                // 中毒致死
                if self.state.hp == 0 && !self.state.is_dead {
                    self.state.is_dead = true;
                    // #1319：C# Die()——中毒致死同样清 Buff + PoisonList + 灰名，并逐 buff 下发 S.RemoveBuff
                    self.clear_death_state();
                }
            }
        }

        // 如有变化，同步客户端
        if total_hp != 0 || total_mp != 0 {
            let mut body = Vec::new();
            body.extend_from_slice(&self.state.hp.to_le_bytes());
            body.extend_from_slice(&self.state.mp.to_le_bytes());
            let _ = self.gate_ref.tell(SendToClient {
                session_id: self.state.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &body),
            }).await;
        }
    }
}

/// 设置装备属性加成（WorldActor 计算后下发）
pub struct SetStatBonuses {
    pub bonus_min_attack: i32,
    pub bonus_max_attack: i32,
    pub bonus_defence: i32,
    pub bonus_max_hp: i32,
    pub bonus_max_mp: i32,
    pub bonus_min_mc: i32,
    pub bonus_max_mc: i32,
    pub bonus_min_sc: i32,
    pub bonus_max_sc: i32,
    // 战斗公式扩展（装备提供的 AC/MAC/Luck/Crit 等）
    pub bonus_min_ac: i32,
    pub bonus_max_ac: i32,
    pub bonus_min_mac: i32,
    pub bonus_max_mac: i32,
    pub luck: i32,
    pub critical_rate: i32,
    pub critical_damage: i32,
    pub magic_resist: i32,
    pub reflect: i32,
    pub attack_bonus: i32,
    pub hp_drain_rate_percent: i32,
    pub agility: i32,
    pub accuracy: i32,
    pub freezing: i32,
    pub poison_attack: i32,
    pub health_recovery: i32,
    pub spell_recovery: i32,
    pub attack_speed: i32,
    pub poison_resist: i32,
    pub holy: i32,
    /// #1000：装备掉落率加成（C# Stat.ItemDropRatePercent/GoldDropRatePercent）
    pub item_drop_rate_percent: i32,
    pub gold_drop_rate_percent: i32,
}

impl Message<SetStatBonuses> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetStatBonuses,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let d_min = msg.bonus_min_attack - self.state.bonus_min_attack;
        let d_max = msg.bonus_max_attack - self.state.bonus_max_attack;
        let d_def = msg.bonus_defence - self.state.bonus_defence;
        let d_hp = msg.bonus_max_hp - self.state.bonus_max_hp;
        let d_mp = msg.bonus_max_mp - self.state.bonus_max_mp;
        let d_min_mc = msg.bonus_min_mc - self.state.bonus_min_mc;
        let d_max_mc = msg.bonus_max_mc - self.state.bonus_max_mc;
        let d_min_sc = msg.bonus_min_sc - self.state.bonus_min_sc;
        let d_max_sc = msg.bonus_max_sc - self.state.bonus_max_sc;

        let changed = d_min != 0 || d_max != 0 || d_def != 0 || d_hp != 0 || d_mp != 0
            || d_min_mc != 0 || d_max_mc != 0 || d_min_sc != 0 || d_max_sc != 0
            || msg.bonus_min_ac != self.state.bonus_min_ac
            || msg.bonus_max_ac != self.state.bonus_max_ac
            || msg.bonus_min_mac != self.state.bonus_min_mac
            || msg.bonus_max_mac != self.state.bonus_max_mac
            || msg.luck != self.state.luck
            || msg.critical_rate != self.state.critical_rate
            || msg.critical_damage != self.state.critical_damage
            || msg.magic_resist != self.state.magic_resist
            || msg.reflect != self.state.reflect
            || msg.attack_bonus != self.state.attack_bonus
            || msg.hp_drain_rate_percent != self.state.hp_drain_rate_percent
            || msg.agility != self.state.agility
            || msg.accuracy != self.state.accuracy
            || msg.freezing != self.state.freezing
            || msg.poison_attack != self.state.poison_attack
            || msg.health_recovery != self.state.health_recovery
            || msg.spell_recovery != self.state.spell_recovery
            || msg.attack_speed != self.state.attack_speed
            || msg.poison_resist != self.state.poison_resist;

        if changed {
            self.state.min_attack += d_min;
            self.state.max_attack += d_max;
            self.state.defence += d_def;
            self.state.max_hp += d_hp;
            self.state.max_mp += d_mp;
            self.state.min_mc += d_min_mc;
            self.state.max_mc += d_max_mc;
            self.state.min_sc += d_min_sc;
            self.state.max_sc += d_max_sc;

            // Clamp HP/MP within new max
            self.state.hp = self.state.hp.min(self.state.max_hp);
            self.state.mp = self.state.mp.min(self.state.max_mp);

            self.state.bonus_min_attack = msg.bonus_min_attack;
            self.state.bonus_max_attack = msg.bonus_max_attack;
            self.state.bonus_defence = msg.bonus_defence;
            self.state.bonus_max_hp = msg.bonus_max_hp;
            self.state.bonus_max_mp = msg.bonus_max_mp;
            self.state.bonus_min_mc = msg.bonus_min_mc;
            self.state.bonus_max_mc = msg.bonus_max_mc;
            self.state.bonus_min_sc = msg.bonus_min_sc;
            self.state.bonus_max_sc = msg.bonus_max_sc;
        }

        // 战斗公式扩展字段：直接覆盖（装备提供的绝对值，非增量）
        self.state.bonus_min_ac = msg.bonus_min_ac;
        self.state.bonus_max_ac = msg.bonus_max_ac;
        self.state.bonus_min_mac = msg.bonus_min_mac;
        self.state.bonus_max_mac = msg.bonus_max_mac;
        self.state.luck = msg.luck;
        self.state.critical_rate = msg.critical_rate;
        self.state.critical_damage = msg.critical_damage;
        self.state.magic_resist = msg.magic_resist;
        self.state.reflect = msg.reflect;
        self.state.attack_bonus = msg.attack_bonus;
        self.state.hp_drain_rate_percent = msg.hp_drain_rate_percent;
        self.state.agility += msg.agility - self.state.agility; // 装备敏捷覆盖基础值
        self.state.accuracy += msg.accuracy - self.state.accuracy;
        self.state.freezing = msg.freezing;
        self.state.poison_attack = msg.poison_attack;
        self.state.health_recovery = msg.health_recovery;
        self.state.spell_recovery = msg.spell_recovery;
        self.state.attack_speed = msg.attack_speed;
        self.state.poison_resist = msg.poison_resist;
        self.state.holy = msg.holy;
        // #1000：掉落率加成（非战斗属性，直接覆盖）
        self.state.item_drop_rate_percent = msg.item_drop_rate_percent;
        self.state.gold_drop_rate_percent = msg.gold_drop_rate_percent;

        if changed {
            self.send_user_information_refresh();
        }
    }
}

/// 攻击结果（返回给 WorldActor 用于广播）
pub struct AttackResult {
    pub object_id: u32,
    pub x: i32,
    pub y: i32,
    pub direction: u8,
    pub spell: u8,
}

/// 损耗装备耐久
pub struct DamageEquipment {
    pub slot: crate::actors::inventory::EquipmentSlot,
    pub amount: u16,
}

/// 扣除单件装备耐久（C# HumanObject.DamageItem：NoDuraLoss 免疫 / Strong 减免 / 归零破损）。
/// 返回 (是否发生变化, 是否破损归零)。
fn apply_dura_damage(item: &mut mir2_shared::data::item::UserItem, amount: u16) -> (bool, bool) {
    // C# SpecialItemMode.NoDuraLoss = 0x400：装备不掉耐久
    let no_dura_loss = item
        .info
        .as_ref()
        .map(|i| {
            i.unique
                .contains(mir2_shared::enums::SpecialItemMode::NO_DURA_LOSS)
        })
        .unwrap_or(false);
    if no_dura_loss {
        return (false, false);
    }
    // C# DamageItem：Strong 属性减少耐久损耗（最少 1）
    let strong = item
        .info
        .as_ref()
        .map(|i| i.stats.get(mir2_shared::enums::Stat::Strong))
        .unwrap_or(0)
        .max(0) as u16;
    let amount = amount.saturating_sub(strong).max(1);
    if item.current_dura > amount {
        item.current_dura -= amount;
        item.dura_changed = true;
        (true, false)
    } else {
        item.current_dura = 0;
        item.dura_changed = true;
        (true, true)
    }
}

impl Message<DamageEquipment> for PlayerActor {
    type Reply = bool; // true = item broke

    async fn handle(
        &mut self,
        msg: DamageEquipment,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let (changed, broke) =
            if let Some(item) = self.state.inventory.equipment[msg.slot as usize].as_mut() {
                // #1246：C# DamageItem——CurrentDura == 0 已破损直接返回（不再扣/不再发 DuraChanged）。
                // 但 Torch 仍需返回 broke 供 tick 路径卸下删除（C# Process 对 0 耐久火把同样移除）。
                if item.current_dura == 0 {
                    (
                        false,
                        msg.slot == crate::actors::inventory::EquipmentSlot::Torch,
                    )
                } else {
                    apply_dura_damage(item, msg.amount)
                }
            } else {
                (false, false)
            };

        // C#：装备耐久变化 → S.DuraChanged（HumanObject Process 每 tick 冲刷 DuraChanged 标志）
        if changed {
            if let Some(item) = self.state.inventory.equipment[msg.slot as usize].as_ref() {
                let dc = mir2_shared::packets::server::experience::DuraChanged {
                    unique_id: item.unique_id,
                    current_dura: item.current_dura,
                };
                let mut body = Vec::new();
                if dc.write_body(&mut body).is_ok() {
                    let _ = self.gate_ref.tell(SendToClient {
                        session_id: self.state.session_id,
                        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::DuraChanged as i16, &body),
                    }).await;
                }
            }
        }

        if broke {
            // #1246：C# DamageItem——非 Torch 装备耐久归零仍保留在装备栏（属性失效由
            // RefreshStats 跳过，玩家可手动卸下后修理）；仅 Torch 归零卸下 + 删除（C# Process）
            if msg.slot == crate::actors::inventory::EquipmentSlot::Torch {
                self.state.inventory.equipment[msg.slot as usize] = None;
                self.send_equipment_changed();
                self.send_inventory_changed();
            }
        }

        broke
    }
}

/// 设置火把耐久消耗计时（毫秒；C# HumanObject.TorchTime）
pub struct SetTorchBurnTime {
    pub burn_time: i64,
}

impl Message<SetTorchBurnTime> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetTorchBurnTime,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.torch_burn_time = msg.burn_time;
    }
}

/// 修理所有装备（恢复耐久到最大值）
pub struct RepairAllEquipment;

impl Message<RepairAllEquipment> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: RepairAllEquipment,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let mut any_repaired = false;
        for slot_idx in 0..crate::actors::inventory::EquipmentSlot::COUNT {
            if let Some(ref mut item) = self.state.inventory.equipment[slot_idx] {
                // #926：C# BindMode.DontRepair(0x20)：不可修理
                let dont_repair = item.info.as_ref()
                    .map(|i| i.bind.contains(mir2_shared::enums::BindMode::DONT_REPAIR))
                    .unwrap_or(false);
                if dont_repair {
                    continue;
                }
                if item.current_dura < item.max_dura {
                    item.current_dura = item.max_dura;
                    item.dura_changed = true;
                    any_repaired = true;
                }
            }
        }
        if any_repaired {
            self.send_equipment_changed();
        }
    }
}

/// #926：设置物品灵魂绑定（C# BindOnEquip → SoulBoundId = 角色 Index；Rust 用 1 作已绑定哨兵）
pub struct SetItemSoulBound {
    pub unique_id: u64,
    pub bound: bool,
}

impl Message<SetItemSoulBound> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetItemSoulBound,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let value = if msg.bound { 1 } else { 0 };
        for slot in self.state.inventory.equipment.iter_mut().flatten() {
            if slot.unique_id == msg.unique_id {
                slot.soul_bound_id = value;
                return;
            }
        }
        for s in self.state.inventory.backpack.iter_mut().flatten() {
            if s.item.unique_id == msg.unique_id {
                s.item.soul_bound_id = value;
                return;
            }
        }
    }
}

/// #950：清空背包（GM @CLEARBAG；C# 逐格 S.DeleteItem + 清空 + RefreshStats）
pub struct ClearBackpack;

impl Message<ClearBackpack> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, _msg: ClearBackpack, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let mut removed: Vec<(u64, u32)> = Vec::new();
        for slot in self.state.inventory.backpack.iter_mut() {
            if let Some(s) = slot.take() {
                removed.push((s.item.unique_id, s.item.count as u32));
            }
        }
        for (uid, count) in removed {
            let pkt = mir2_shared::packets::server::experience::DeleteItem { unique_id: uid, count };
            let mut body = Vec::new();
            if pkt.write_body(&mut body).is_ok() {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: self.state.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::DeleteItem as i16, &body),
                }).await;
            }
        }
        self.send_inventory_changed();
    }
}

// ============================================================
// 背包操作消息 Handler
// ============================================================

/// #1753：英雄背包扩容（C# HeroInfo.ResizeInventory：上限 42，每次 +8；发 S.ResizeInventory）
pub struct ResizeHeroInventory;

impl Message<ResizeHeroInventory> for PlayerActor {
    type Reply = usize;

    async fn handle(&mut self, _msg: ResizeHeroInventory, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let len = self.state.hero_inventory.backpack.len();
        if len >= 42 {
            return len;
        }
        let new_len = (len + 8).min(42);
        self.state.hero_inventory.backpack.resize(new_len, None);
        // C# HeroObject.cs:490——S.ResizeInventory（客户端英雄背包刷新）
        let pkt = mir2_shared::packets::server::ui_events::ResizeInventory { size: new_len as i32 };
        let mut body = Vec::new();
        if pkt.write_body(&mut body).is_ok() {
            let _ = self.gate_ref.tell(SendToClient {
                session_id: self.state.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ResizeInventory as i16, &body),
            }).await;
        }
        debug!("Hero inventory resized to {}", new_len);
        new_len
    }
}

/// 添加物品到背包
pub struct AddItemToInventory {
    pub item: mir2_shared::data::item::UserItem,
}

impl Message<AddItemToInventory> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: AddItemToInventory, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        // 金币（item_index=0）：直接加 gold，不占背包（C# 金币独立于背包）
        if msg.item.item_index == 0 {
            self.state.inventory.gold = self.state.inventory.gold.saturating_add(msg.item.count as u64);
            // #1588：拾取金币应发 S.GainedGold（客户端 Gold += Gold）；
            // 原先错发 LoseGold 会让客户端先扣余额再靠 UserInformation 回刷。
            let mut body = Vec::new();
            body.extend_from_slice(&(msg.item.count as u32).to_le_bytes());
            let _ = self.gate_ref.tell(SendToClient {
                session_id: self.state.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GainedGold as i16, &body),
            }).await;
            return true;
        }
        match self.state.inventory.add_item(msg.item) {
            Some((_grid, _uid)) => {
                // 发送 ItemChanged 通知客户端更新背包
                self.send_inventory_changed();
                true
            }
            None => false,
        }
    }
}

/// 背包内移动
pub struct InventoryMoveItem {
    pub from_grid: u8,
    pub to_grid: u8,
}

impl Message<InventoryMoveItem> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: InventoryMoveItem, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let success = self.state.inventory.move_item(msg.from_grid, msg.to_grid);
        if success {
            self.send_inventory_changed();
        }
        success
    }
}

/// 获取物品信息
pub struct GetItemInfo {
    pub unique_id: u64,
}

impl Message<GetItemInfo> for PlayerActor {
    type Reply = Option<mir2_shared::data::item::UserItem>;

    async fn handle(&mut self, msg: GetItemInfo, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.inventory.get_item(msg.unique_id).cloned()
    }
}

/// 获取指定格子的物品信息
pub struct GetItemInfoByGrid {
    pub grid: u8,
}

impl Message<GetItemInfoByGrid> for PlayerActor {
    type Reply = Option<mir2_shared::data::item::UserItem>;

    async fn handle(&mut self, msg: GetItemInfoByGrid, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.inventory.get_item_by_grid(msg.grid).cloned()
    }
}

/// 消耗物品
pub struct ConsumeItem {
    pub unique_id: u64,
}

impl Message<ConsumeItem> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: ConsumeItem, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let removed = self.state.inventory.remove_item_by_uid(msg.unique_id);
        if removed.is_some() {
            self.send_inventory_changed();
            true
        } else {
            false
        }
    }
}

/// 装备物品
/// 获取英雄背包物品信息（#218）
pub struct GetHeroItemInfo {
    pub unique_id: u64,
}

impl Message<GetHeroItemInfo> for PlayerActor {
    type Reply = Option<mir2_shared::data::item::UserItem>;

    async fn handle(
        &mut self,
        msg: GetHeroItemInfo,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state
            .hero_inventory
            .backpack
            .iter()
            .flatten()
            .find(|s| s.item.unique_id == msg.unique_id)
            .map(|s| s.item.clone())
    }
}

/// 按 item_index 在英雄背包查找药水（#1182 自动药 TryAutoPot：找第一个同 index 的堆叠）
pub struct GetHeroPotionByItemIndex {
    pub item_index: i32,
}

impl Message<GetHeroPotionByItemIndex> for PlayerActor {
    type Reply = Option<mir2_shared::data::item::UserItem>;

    async fn handle(
        &mut self,
        msg: GetHeroPotionByItemIndex,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.find_hero_potion(msg.item_index)
    }
}

/// 消耗英雄背包物品（#218；#1182 起支持堆叠：count>1 时 -1，否则移除整格）
pub struct ConsumeHeroItem {
    pub unique_id: u64,
}

impl Message<ConsumeHeroItem> for PlayerActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        msg: ConsumeHeroItem,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.consume_hero_item(msg.unique_id)
    }
}

/// 英雄是否已学习技能（#218）
pub struct IsHeroMagicLearned {
    pub spell: i32,
}

impl Message<IsHeroMagicLearned> for PlayerActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        msg: IsHeroMagicLearned,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.hero_magics.iter().any(|m| m.spell == msg.spell)
    }
}

/// 英雄学习技能（#218）
pub struct LearnHeroMagic {
    pub spell: i32,
}

impl Message<LearnHeroMagic> for PlayerActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        msg: LearnHeroMagic,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.hero_learn_magic(msg.spell)
    }
}

/// NPC 脚本 HEROGIVESKILL：英雄学技能并设等级（对齐 C# HeroGiveSkill，Level<=3）
pub struct LearnHeroMagicWithLevel {
    pub spell: i32,
    pub level: u8,
}

impl Message<LearnHeroMagicWithLevel> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: LearnHeroMagicWithLevel, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if self.state.hero_magics.iter().any(|m| m.spell == msg.spell) {
            return false;
        }
        let mut magic = PlayerMagic::new(msg.spell);
        magic.level = msg.level.min(3);
        self.state.hero_magics.push(magic);
        true
    }
}

/// NPC 脚本 HEROREMOVESKILL：移除英雄技能（对齐 C# HeroRemoveSkill + S.RemoveMagic hero）
pub struct RemoveHeroMagicWithId {
    pub spell: i32,
}

impl Message<RemoveHeroMagicWithId> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: RemoveHeroMagicWithId, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let Some(idx) = self.state.hero_magics.iter().position(|m| m.spell == msg.spell) else {
            return false;
        };
        self.state.hero_magics.remove(idx);
        if let Ok(spell) = mir2_shared::enums::Spell::try_from(msg.spell as u8) {
            let pkt = mir2_shared::packets::server::magic::RemoveMagic { spell, hero: true };
            let mut body = Vec::new();
            if mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut body), &pkt).is_ok() {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: self.state.session_id,
                    data: body,
                }).try_send();
            }
        }
        true
    }
}


/// 装备物品
pub struct InventoryEquipItem {
    pub grid: u8,
    pub slot: crate::actors::inventory::EquipmentSlot,
}

impl Message<InventoryEquipItem> for PlayerActor {
    type Reply = Option<(Option<mir2_shared::data::item::UserItem>, u64)>;

    async fn handle(&mut self, msg: InventoryEquipItem, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let result = self.state.inventory.equip_item(msg.grid, msg.slot);
        if result.is_some() {
            self.send_inventory_changed();
            self.send_equipment_changed();
        }
        result
    }
}

/// #1546：从仓库格装备（C# EquipItem Grid=Storage；旧装备放回仓库原格）
pub struct StorageEquipItem {
    pub storage_idx: usize,
    pub slot: crate::actors::inventory::EquipmentSlot,
}

impl Message<StorageEquipItem> for PlayerActor {
    type Reply = Option<(Option<mir2_shared::data::item::UserItem>, u64)>;

    async fn handle(&mut self, msg: StorageEquipItem, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let result = self.state.inventory.equip_from_storage(msg.storage_idx, msg.slot);
        if result.is_some() {
            self.send_inventory_changed();
            self.send_equipment_changed();
        }
        result
    }
}
/// 获取装备信息
pub struct GetEquipmentInfo {
    pub slot: crate::actors::inventory::EquipmentSlot,
}

impl Message<GetEquipmentInfo> for PlayerActor {
    type Reply = Option<mir2_shared::data::item::UserItem>;

    async fn handle(&mut self, msg: GetEquipmentInfo, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.inventory.get_equipment(msg.slot).cloned()
    }
}

/// 卸下装备
pub struct InventoryUnequipItem {
    pub slot: crate::actors::inventory::EquipmentSlot,
}

impl Message<InventoryUnequipItem> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: InventoryUnequipItem, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let result = self.state.inventory.unequip_item(msg.slot);
        if result.is_some() {
            self.send_inventory_changed();
            self.send_equipment_changed();
            true
        } else {
            false
        }
    }
}

/// 喂坐骑：恢复坐骑耐久（C# UseItem Food；返回 (uid, max_dura, current_dura)）
pub struct FeedMount {
    pub amount: u16,
    /// 食物 shape（C#：shape 0 降低坐骑 MaxDura）
    pub shape: i32,
}

impl Message<FeedMount> for PlayerActor {
    type Reply = Option<(u64, u16, u16)>;

    async fn handle(&mut self, msg: FeedMount, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let result = {
            let slot = crate::actors::inventory::EquipmentSlot::Mount as usize;
            let Some(m) = self.state.inventory.equipment.get_mut(slot).and_then(|s| s.as_mut()) else {
                return None;
            };
            if m.current_dura >= m.max_dura {
                return None;
            }
            // C# Food shape 0：MaxDura -= min(1000, MaxDura - CurrentDura/30)
            if msg.shape == 0 {
                let reduce = 1000u32.min(m.max_dura as u32 - m.current_dura as u32 / 30);
                m.max_dura = ((m.max_dura as u32).saturating_sub(reduce)).max(0) as u16;
            }
            // C#：CurrentDura += item.CurrentDura（cap MaxDura）
            m.current_dura = (m.current_dura as u32 + msg.amount as u32).min(m.max_dura as u32) as u16;
            m.dura_changed = true;
            Some((m.unique_id, m.max_dura, m.current_dura))
        };
        self.send_equipment_changed();
        result
    }
}

/// 修改武器幸运（C# TryLuckWeapon：AddedStats[Stat.Luck] ±1，刷新装备）
pub struct AddWeaponLuck {
    pub delta: i32,
}

impl Message<AddWeaponLuck> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: AddWeaponLuck, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let slot = crate::actors::inventory::EquipmentSlot::Weapon as usize;
        let Some(w) = self.state.inventory.equipment.get_mut(slot).and_then(|s| s.as_mut()) else {
            return false;
        };
        use mir2_shared::enums::Stat;
        // C# 边界：TryLuckWeapon 上限 7；诅咒下限 -Settings.MaxLuck(-10)
        let new_luck = (w.added_stats.get(Stat::Luck) + msg.delta).clamp(-10, 7);
        w.added_stats.set(Stat::Luck, new_luck);
        self.send_equipment_changed();
        // #967：C# 诅咒/祝福油后 S.RefreshItem 即时刷新客户端武器显示
        if let Some(w) = self.state.inventory.equipment[slot].as_ref() {
            self.send_refresh_item(w);
        }
        true
    }
}

/// 修理武器（C# UseItem Scroll shape 4/5：RepairOil/WarGodOil；返回 (uid, max_dura, current_dura)）
pub struct RepairWeapon {
    pub full: bool,
}

impl Message<RepairWeapon> for PlayerActor {
    type Reply = Option<(u64, u16, u16)>;

    async fn handle(&mut self, msg: RepairWeapon, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let result = {
            let slot = crate::actors::inventory::EquipmentSlot::Weapon as usize;
            let Some(w) = self.state.inventory.equipment.get_mut(slot).and_then(|s| s.as_mut()) else {
                return None;
            };
            if w.max_dura == 0 || w.current_dura >= w.max_dura {
                return None;
            }
            if msg.full {
                // C# WarGodOil：CurrentDura = MaxDura
                w.current_dura = w.max_dura;
            } else {
                // C# RepairOil：MaxDura -= min(5000, MaxDura-CurrentDura)/30；CurrentDura += 5000（cap MaxDura）
                let missing = (w.max_dura as u32 - w.current_dura as u32).min(5000);
                w.max_dura = ((w.max_dura as u32).saturating_sub(missing / 30)).max(0) as u16;
                w.current_dura = (w.current_dura as u32 + 5000).min(w.max_dura as u32) as u16;
            }
            w.dura_changed = true;
            // #967：C# 修理油后 S.RefreshItem 即时刷新客户端武器耐久显示（先快照结束借用）
            let weapon_snapshot = w.clone();
            self.send_refresh_item(&weapon_snapshot);
            Some((weapon_snapshot.unique_id, weapon_snapshot.max_dura, weapon_snapshot.current_dura))
        };
        self.send_equipment_changed();
        result
    }
}

/// 标记物品已鉴定（C# NeedIdentify 使用/装备时自动鉴定 + S.RefreshItem）
pub struct SetItemIdentified {
    pub unique_id: u64,
}

impl Message<SetItemIdentified> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: SetItemIdentified, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        for s in self.state.inventory.backpack.iter_mut().flatten() {
            if s.item.unique_id == msg.unique_id {
                if !s.item.identified {
                    s.item.identified = true;
                    self.send_inventory_changed();
                }
                return true;
            }
        }
        false
    }
}

/// 从背包移除物品
pub struct RemoveItemFromInventory {
    pub unique_id: u64,
}

/// 按数量从背包移除物品（C# SellItem 堆叠拆分：原堆扣减 count，返回被移除部分）
pub struct RemoveItemFromInventoryCount {
    pub unique_id: u64,
    pub count: u16,
}

impl Message<RemoveItemFromInventoryCount> for PlayerActor {
    type Reply = Option<mir2_shared::data::item::UserItem>;

    async fn handle(&mut self, msg: RemoveItemFromInventoryCount, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let backpack = &mut self.state.inventory.backpack;
        for slot in backpack.iter_mut() {
            let Some(s) = slot else { continue };
            if s.item.unique_id != msg.unique_id {
                continue;
            }
            let take = msg.count.min(s.item.count);
            if take == 0 {
                return None;
            }
            if take >= s.item.count {
                let removed = s.item.clone();
                *slot = None;
                self.send_inventory_changed();
                return Some(removed);
            }
            let mut removed = s.item.clone();
            removed.count = take;
            s.item.count -= take;
            self.send_inventory_changed();
            return Some(removed);
        }
        None
    }
}


/// 客户端删除物品（C# C.DeleteItem）：按 uid 扣减背包/英雄背包数量，发 S.DeleteItem 确认
pub struct DeleteItemFromInventory {
    pub unique_id: u64,
    pub count: u16,
    pub hero: bool,
}

impl Message<DeleteItemFromInventory> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: DeleteItemFromInventory, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let backpack = if msg.hero { &mut self.state.hero_inventory.backpack } else { &mut self.state.inventory.backpack };
        let mut removed = 0u16;
        for slot in backpack.iter_mut() {
            if let Some(s) = slot {
                if s.item.unique_id == msg.unique_id {
                    let take = msg.count.saturating_sub(removed).min(s.item.count);
                    s.item.count -= take;
                    removed += take;
                    if s.item.count == 0 {
                        *slot = None;
                    }
                    if removed >= msg.count {
                        break;
                    }
                }
            }
        }
        if removed > 0 {
            // 简化：统一发背包变更包（英雄背包同步由 WorldActor.send_hero_information_packet 负责）
            self.send_inventory_changed();
            // S.DeleteItem 确认包（C# ServerPackets.DeleteItem：UniqueID + Count）
            let mut body = Vec::new();
            body.extend_from_slice(&msg.unique_id.to_le_bytes());
            body.extend_from_slice(&removed.to_le_bytes());
            let _ = self.gate_ref.tell(SendToClient {
                session_id: self.state.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::DeleteItem as i16, &body),
            }).try_send();
            return true;
        }
        false
    }
}

impl Message<RemoveItemFromInventory> for PlayerActor {
    type Reply = Option<mir2_shared::data::item::UserItem>;

    async fn handle(&mut self, msg: RemoveItemFromInventory, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let item = self.state.inventory.remove_item_by_uid(msg.unique_id);
        if item.is_some() {
            self.send_inventory_changed();
        }
        item
    }
}

/// 合并物品
pub struct InventoryMergeItem {
    pub from_grid: u8,
    pub to_grid: u8,
}

impl Message<InventoryMergeItem> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: InventoryMergeItem, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let success = self.state.inventory.merge_item(msg.from_grid, msg.to_grid);
        if success {
            self.send_inventory_changed();
        }
        success
    }
}

/// 拆分物品（按 unique_id 定位原格）
pub struct InventorySplitItem {
    pub unique_id: u64,
    pub count: u16,
}

impl Message<InventorySplitItem> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: InventorySplitItem, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let success = self.state.inventory.split_item_by_uid(msg.unique_id, msg.count);
        if success {
            self.send_inventory_changed();
        }
        success
    }
}

/// 丢弃物品（支持部分数量）
pub struct DropInventoryItem {
    pub unique_id: u64,
    pub count: u16,
}

impl Message<DropInventoryItem> for PlayerActor {
    type Reply = Option<mir2_shared::data::item::UserItem>;

    async fn handle(&mut self, msg: DropInventoryItem, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let item = self.state.inventory.remove_item_by_uid_partial(msg.unique_id, msg.count);
        if item.is_some() {
            self.send_inventory_changed();
        }
        item
    }
}

/// 按 unique_id 合并物品
pub struct MergeInventoryItemByUid {
    pub from_uid: u64,
    pub to_uid: u64,
}

impl Message<MergeInventoryItemByUid> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: MergeInventoryItemByUid, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let success = self.state.inventory.merge_item_by_uid(msg.from_uid, msg.to_uid);
        if success {
            self.send_inventory_changed();
        }
        success
    }
}

/// 修理物品
pub struct RepairItem {
    pub unique_id: u64,
    /// C# RepairItem(bool special)：SRepair 特殊修理（费用×3、不衰减 MaxDura）
    pub special: bool,
}

impl Message<RepairItem> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: RepairItem, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let success = self.state.inventory.repair_item(msg.unique_id, msg.special);
        if success {
            self.send_inventory_changed();
        }
        success
    }
}

/// 重置物品附加属性（洗点）
pub struct ResetItemAddedStats {
    pub unique_id: u64,
}

impl Message<ResetItemAddedStats> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: ResetItemAddedStats, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if let Some(item) = self.state.inventory.get_item_mut(msg.unique_id) {
            item.awake = Default::default();
            item.added_stats = Default::default();
            self.send_inventory_changed();
            true
        } else {
            false
        }
    }
}

/// 设置物品觉醒状态
pub struct SetItemAwake {
    pub unique_id: u64,
    pub awake: mir2_shared::data::item::Awake,
}

impl Message<SetItemAwake> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: SetItemAwake, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if let Some(item) = self.state.inventory.get_item_mut(msg.unique_id) {
            item.awake = msg.awake;
            self.send_inventory_changed();
            true
        } else {
            false
        }
    }
}

/// 镶嵌宝石：将背包中的宝石插入目标装备的空槽位
pub struct SocketGem {
    pub from_grid: u8,
    pub to_grid: u8,
    pub target_slot_count: usize,
}

impl Message<SocketGem> for PlayerActor {
    type Reply = Option<(u64, u64)>; // (source_uid, target_uid)

    async fn handle(&mut self, msg: SocketGem, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let result = self.state.inventory.socket_gem(msg.from_grid, msg.to_grid, msg.target_slot_count);
        if result.is_some() {
            self.send_inventory_changed();
        }
        result
    }
}

/// 按 item_index 计算背包中物品数量
pub struct CountItemsByIndex {
    pub item_index: i32,
}

impl Message<CountItemsByIndex> for PlayerActor {
    type Reply = u16;

    async fn handle(&mut self, msg: CountItemsByIndex, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.inventory.count_item_by_index(msg.item_index)
    }
}

/// 按 item_index 从背包中消耗指定数量物品
pub struct ConsumeItemsByIndex {
    pub item_index: i32,
    pub count: u16,
}

impl Message<ConsumeItemsByIndex> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: ConsumeItemsByIndex, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let success = self.state.inventory.remove_item_by_index(msg.item_index, msg.count);
        if success {
            self.send_inventory_changed();
        }
        success
    }
}

/// 召唤消耗护身符（C# HumanObject.GetAmulet + ConsumeItem）：
/// 装备槽 Pendant（C# Amulet）、ItemType::Amulet、shape==0、count>=amount；
/// 扣减 count，扣完移除装备并刷新客户端。
pub struct ConsumeAmuletForSummon {
    pub amount: u16,
}

impl Message<ConsumeAmuletForSummon> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: ConsumeAmuletForSummon, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let removed = self.state.consume_amulet_for_summon(msg.amount);
        if removed {
            self.send_equipment_changed();
        } else {
            // 仍有剩余：下发 RefreshItem 即时刷新护身符数量
            if let Some(item) = self.state.inventory.equipment[EquipmentSlot::Pendant as usize].as_ref() {
                self.send_refresh_item(item);
            }
        }
        removed
    }
}

/// #1453：消耗 1 个指定 shape 的毒护符（C# Plague GetPoison + ConsumeItem；shape 1=绿/2=红）
pub struct ConsumePoisonAmuletForPlague {
    pub shape: u16,
}

impl Message<ConsumePoisonAmuletForPlague> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: ConsumePoisonAmuletForPlague, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let removed = self.state.consume_poison_amulet(msg.shape);
        if removed {
            self.send_equipment_changed();
        } else if let Some(item) = self.state.inventory.equipment[EquipmentSlot::Pendant as usize].as_ref() {
            self.send_refresh_item(item);
        }
        removed
    }
}
/// 丢弃金币
pub struct DropGold {
    pub amount: u64,
}

impl Message<DropGold> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: DropGold, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if self.state.inventory.gold >= msg.amount {
            self.state.inventory.gold = self.state.inventory.gold.saturating_sub(msg.amount);
            self.send_gold_changed();
            true
        } else {
            false
        }
    }
}

/// 添加金币
pub struct AddGold {
    pub amount: u64,
}

impl Message<AddGold> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: AddGold, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        // #1747：C# GainGold（PlayerObject.cs:7677）——金币上限 uint.MaxValue，超出只加剩余额度
        let before = self.state.inventory.gold.min(u32::MAX as u64);
        let after = before.saturating_add(msg.amount).min(u32::MAX as u64);
        let gained = after - before;
        self.state.inventory.gold = after;
        self.send_gold_changed();
        // C# GainGold：S.GainedGold（客户端金币浮字，发实际增加量）
        if gained > 0 {
            let packet = mir2_shared::packets::server::drops::GainedGold {
                gold: gained as u32,
            };
            let mut body = Vec::new();
            if packet.write_body(&mut body).is_ok() {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: self.state.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GainedGold as i16, &body),
                }).try_send();
            }
        }
        true
    }
}

/// 扣减金币
pub struct DeductGold {
    pub amount: u64,
}

impl Message<DeductGold> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: DeductGold, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if self.state.inventory.gold >= msg.amount {
            self.state.inventory.gold = self.state.inventory.gold.saturating_sub(msg.amount);
            self.send_gold_changed();
            true
        } else {
            false
        }
    }
}

/// 检查金币是否足够
pub struct HasGold {
    pub amount: u64,
}

impl Message<HasGold> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: HasGold, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.inventory.gold >= msg.amount
    }
}

/// 扣减 MP
pub struct DeductMP {
    pub amount: i32,
}

impl Message<DeductMP> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: DeductMP, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if self.state.mp >= msg.amount {
            self.state.mp -= msg.amount;
            // 同步客户端
            let mut body = Vec::new();
            body.extend_from_slice(&(self.state.hp as u32).to_le_bytes());
            body.extend_from_slice(&(self.state.mp as u32).to_le_bytes());
            let _ = self.gate_ref.tell(SendToClient {
                session_id: self.state.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &body),
            }).await;
            true
        } else {
            false
        }
    }
}

/// 恢复 MP
pub struct AddMP {
    pub amount: i32,
}

impl Message<AddMP> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: AddMP, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if msg.amount <= 0 { return; }
        self.state.mp = (self.state.mp + msg.amount).min(self.state.max_mp);
        // 同步客户端
        let mut body = Vec::new();
        body.extend_from_slice(&(self.state.hp as u32).to_le_bytes());
        body.extend_from_slice(&(self.state.mp as u32).to_le_bytes());
        let _ = self.gate_ref.tell(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &body),
        }).await;
    }
}

/// 检查背包中是否有指定数量的物品（按 item_index）
pub struct HasItem {
    pub item_index: i32,
    pub count: u16,
}

impl Message<HasItem> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: HasItem, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.inventory.count_item_by_index(msg.item_index) >= msg.count
    }
}

/// 按 item_index 从背包中移除指定数量的物品
pub struct RemoveItemByIndex {
    pub item_index: i32,
    pub count: u16,
}

impl Message<RemoveItemByIndex> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: RemoveItemByIndex, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.inventory.remove_item_by_index(msg.item_index, msg.count)
    }
}

/// 按 item_index 从背包中移除指定数量的物品（带耐久下限过滤，C# TakeItem dura）
pub struct RemoveItemByIndexWithDura {
    pub item_index: i32,
    pub count: u16,
    pub min_dura: Option<u32>,
}

impl Message<RemoveItemByIndexWithDura> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: RemoveItemByIndexWithDura, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.inventory.remove_item_by_index_with_dura(msg.item_index, msg.count, msg.min_dura)
    }
}

/// 检查背包是否有空位
pub struct HasItemSpace;

impl Message<HasItemSpace> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, _msg: HasItemSpace, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.inventory.has_space()
    }
}

/// 增加 PK 值（击杀玩家时调用）
pub struct AddPkPoints {
    pub points: i32,
}

impl Message<AddPkPoints> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: AddPkPoints, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if msg.points > 0 {
            self.state.pk_points += msg.points;
            self.state.pk_kill_count += 1;
            debug!("Player {} PK points +{} (total={}, kills={})",
                   self.state.name, msg.points, self.state.pk_points, self.state.pk_kill_count);
        }
    }
}

/// NPC 脚本 CHANGEGENDER：修改角色性别（对齐 C# ActionType.ChangeGender）
pub struct SetGender {
    pub gender: mir2_shared::enums::MirGender,
}

impl Message<SetGender> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetGender, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.gender = msg.gender;
        debug!("Player {} gender changed to {:?}", self.state.name, msg.gender);
    }
}

/// NPC 脚本 REMOVESKILL：移除已学技能（对齐 C# ActionType.RemoveSkill + S.RemoveMagic）
pub struct RemoveMagicWithId {
    pub spell: i32,
}

impl Message<RemoveMagicWithId> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: RemoveMagicWithId, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let Some(idx) = self.state.magics.iter().position(|m| m.spell == msg.spell) else {
            return false;
        };
        self.state.magics.remove(idx);
        // S.RemoveMagic（opcode 118，C# RemoveSkill 语义）
        if let Ok(spell) = mir2_shared::enums::Spell::try_from(msg.spell as u8) {
            let pkt = mir2_shared::packets::server::magic::RemoveMagic { spell, hero: false };
            let mut body = Vec::new();
            if mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut body), &pkt).is_ok() {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: self.state.session_id,
                    data: body,
                }).try_send();
            }
        }
        debug!("Player {} removed magic spell={}", self.state.name, msg.spell);
        true
    }
}

/// PK 值衰减（每 tick 调用）
pub struct DecayPkPoints;

impl Message<DecayPkPoints> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, _msg: DecayPkPoints, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if self.state.pk_points > 0 {
            self.state.pk_points = (self.state.pk_points - 1).max(0);
        }
    }
}

/// NPC 脚本 SETPKPOINT：直接设置 PK 值（对齐 C# ActionType.SetPkPoint）
pub struct SetPkPoints {
    pub points: i32,
}

impl Message<SetPkPoints> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetPkPoints, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.pk_points = msg.points.max(0);
        debug!("Player {} PK points set to {}", self.state.name, self.state.pk_points);
    }
}

/// NPC 脚本 REDUCEPKPOINT：减少 PK 值（对齐 C# ActionType.ReducePkPoint）
pub struct ReducePkPoints {
    pub amount: i32,
}

impl Message<ReducePkPoints> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: ReducePkPoints, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.pk_points = (self.state.pk_points - msg.amount).max(0);
        debug!("Player {} PK points reduced by {} (total={})", self.state.name, msg.amount, self.state.pk_points);
    }
}

/// NPC 脚本 GIVEMP：恢复 MP（对齐 C# ActionType.GiveMP / ChangeMP）
pub struct RestoreMp {
    pub amount: i32,
}

impl Message<RestoreMp> for PlayerActor {
    type Reply = i32;

    async fn handle(&mut self, msg: RestoreMp, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if self.state.is_dead || msg.amount <= 0 {
            return 0;
        }
        let before = self.state.mp;
        self.state.mp = (self.state.mp + msg.amount).min(self.state.max_mp);
        let restored = self.state.mp - before;
        if restored > 0 {
            let mut body = Vec::new();
            body.extend_from_slice(&(self.state.hp as u32).to_le_bytes());
            body.extend_from_slice(&(self.state.mp as u32).to_le_bytes());
            let _ = self.gate_ref.tell(SendToClient {
                session_id: self.state.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &body),
            }).try_send();
        }
        debug!("Player {} MP +{} (restored={})", self.state.name, msg.amount, restored);
        restored
    }
}


/// 死亡时随机掉落背包物品（返回被掉落的物品列表）
pub struct DropRandomItemsOnDeath;

/// 死亡掉落：直接从装备槽位取走物品（不放回背包；C# DeathDrop Info.Equipment[i] = null）
pub struct TakeEquipmentOnDeath {
    pub slot: crate::actors::inventory::EquipmentSlot,
}

impl Message<TakeEquipmentOnDeath> for PlayerActor {
    type Reply = Option<mir2_shared::data::item::UserItem>;

    async fn handle(&mut self, msg: TakeEquipmentOnDeath, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let result = self.state.inventory.take_equipment(msg.slot);
        if result.is_some() {
            self.send_equipment_changed();
        }
        result
    }
}

impl Message<DropRandomItemsOnDeath> for PlayerActor {
    type Reply = Vec<mir2_shared::data::item::UserItem>;

    async fn handle(
        &mut self,
        _msg: DropRandomItemsOnDeath,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let mut dropped = Vec::new();
        // 红名玩家掉落更多（基础 0-2，红名 +1-3）
        let base_max = if self.state.pk_points > 0 { 5u8 } else { 2u8 };
        let max_drop = fastrand::u8(0..=base_max);
        for _ in 0..max_drop {
            if let Some(item) = self.state.inventory.random_drop_one() {
                dropped.push(item);
            }
        }
        dropped
    }
}

/// 设置组队 ID
pub struct SetGroupId {
    pub group_id: Option<u64>,
}

impl Message<SetGroupId> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetGroupId, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.group_id = msg.group_id;
    }
}

/// 添加好友到列表
pub struct AddFriendToSelf {
    pub friend_oid: u32,
    pub friend_name: String,
}

impl Message<AddFriendToSelf> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: AddFriendToSelf, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.friend_list.add_friend(msg.friend_oid, msg.friend_name);
    }
}

/// 从列表移除好友
pub struct RemoveFriendFromSelf {
    pub friend_oid: u32,
}

impl Message<RemoveFriendFromSelf> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: RemoveFriendFromSelf, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.friend_list.remove_friend(msg.friend_oid)
    }
}

/// 设置好友备注
pub struct SetFriendMemo {
    pub friend_oid: u32,
    pub memo: String,
}

impl Message<SetFriendMemo> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: SetFriendMemo, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.friend_list.set_memo(msg.friend_oid, msg.memo)
    }
}

// ============================================================
// 邮件系统 Handler
// ============================================================

/// 添加邮件到收件箱
pub struct AddMail {
    pub mail: crate::actors::mail::MailMessage,
}

impl Message<AddMail> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: AddMail, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.mailbox.add_mail(msg.mail);
    }
}

/// 获取邮件内容
pub struct GetMail {
    pub mail_id: u64,
}

impl Message<GetMail> for PlayerActor {
    type Reply = Option<crate::actors::mail::MailMessage>;

    async fn handle(&mut self, msg: GetMail, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.mailbox.get_mail(msg.mail_id).cloned()
    }
}

/// 标记邮件已读
pub struct MarkMailRead {
    pub mail_id: u64,
}

impl Message<MarkMailRead> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: MarkMailRead, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.mailbox.mark_read(msg.mail_id)
    }
}

/// 收取邮件附件（返回金币和物品）
pub struct CollectMailAttachment {
    pub mail_id: u64,
}

impl Message<CollectMailAttachment> for PlayerActor {
    type Reply = Option<(u64, Vec<mir2_shared::data::item::UserItem>)>;

    async fn handle(&mut self, msg: CollectMailAttachment, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.mailbox.collect_attachment(msg.mail_id)
    }
}

/// 删除邮件
pub struct DeleteMail {
    pub mail_id: u64,
}

impl Message<DeleteMail> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: DeleteMail, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.mailbox.delete_mail(msg.mail_id)
    }
}

// ============================================================
// 行会系统 Handler
// ============================================================

/// 设置玩家行会信息
pub struct SetGuildInfo {
    pub guild_name: Option<String>,
    pub rank: GuildRank,
}

impl Message<SetGuildInfo> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetGuildInfo, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.guild_name = msg.guild_name;
        self.state.guild_rank = msg.rank;
    }
}

// ============================================================
// 任务系统 Handler
// ============================================================

/// 更新任务日志
pub struct UpdateQuestLog {
    pub quest_log: crate::actors::quest::QuestLog,
}

impl Message<UpdateQuestLog> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: UpdateQuestLog, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.quest_log = msg.quest_log;
    }
}

/// 接受任务（在 PlayerActor 上执行）
pub struct AcceptQuest {
    pub quest: crate::actors::quest::QuestInstance,
}

impl Message<AcceptQuest> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: AcceptQuest, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.quest_log.accept_quest(msg.quest)
    }
}

/// #1489：GM 设置任务完成/取消（C# SETQUEST state 0=取消 1=完成）
pub struct GmSetQuest {
    pub quest_index: i32,
    pub complete: bool,
}

impl Message<GmSetQuest> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: GmSetQuest, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let ql = &mut self.state.quest_log;
        if let Some(idx) = ql.quests.iter().position(|q| q.quest_index == msg.quest_index) {
            ql.quests.remove(idx);
        }
        if msg.complete {
            if !ql.completed_indices.contains(&msg.quest_index) {
                ql.completed_indices.push(msg.quest_index);
            }
        } else {
            ql.completed_indices.retain(|i| *i != msg.quest_index);
        }
    }
}

/// #1490：GM 清空任务（C# CLEARQUESTS）
pub struct GmClearQuests;

impl Message<GmClearQuests> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, _msg: GmClearQuests, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.quest_log = crate::actors::quest::QuestLog::new();
    }
}

/// 完成任务（在 PlayerActor 上执行，返回完成的奖励信息）
pub struct CompleteQuest {
    pub quest_index: i32,
}

impl Message<CompleteQuest> for PlayerActor {
    type Reply = Option<crate::actors::quest::QuestInstance>;

    async fn handle(&mut self, msg: CompleteQuest, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let done = self.state.quest_log.complete_quest(msg.quest_index);
        if done.is_some() {
            // C# FinishQuest → RecalculateQuestBag：移除不再需要的任务物品
            self.recalculate_quest_bag();
        }
        done
    }
}

/// 放弃任务
pub struct AbandonQuest {
    pub quest_index: i32,
}

impl Message<AbandonQuest> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: AbandonQuest, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let done = self.state.quest_log.abandon_quest(msg.quest_index);
        if done {
            // C# AbandonQuest → RecalculateQuestBag
            self.recalculate_quest_bag();
        }
        done
    }
}

/// 任务失败（超时等）
pub struct FailQuest {
    pub quest_index: i32,
}

impl Message<FailQuest> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: FailQuest, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if let Some(quest) = self.state.quest_log.get_quest_mut(msg.quest_index) {
            quest.status = crate::actors::quest::QuestStatus::Failed;
            true
        } else {
            false
        }
    }
}

/// 获取任务
pub struct GetQuest {
    pub quest_index: i32,
}

impl Message<GetQuest> for PlayerActor {
    type Reply = Option<crate::actors::quest::QuestInstance>;

    async fn handle(&mut self, msg: GetQuest, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.quest_log.get_quest(msg.quest_index).cloned()
    }
}

/// 检查是否已完成过该任务
pub struct HasCompletedQuest {
    pub quest_index: i32,
}

impl Message<HasCompletedQuest> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: HasCompletedQuest, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.quest_log.completed_indices.contains(&msg.quest_index)
    }
}

/// 查询任务状态
/// 返回: 0=未接受/不存在, 1=已接受(进行中), 2=已完成
pub struct CheckQuestState {
    pub quest_index: i32,
}

impl Message<CheckQuestState> for PlayerActor {
    type Reply = u8;

    async fn handle(&mut self, msg: CheckQuestState, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if self.state.quest_log.completed_indices.contains(&msg.quest_index) {
            return 2;
        }
        if self.state.quest_log.get_quest(msg.quest_index).is_some() {
            return 1;
        }
        0
    }
}

/// 处理怪物击杀进度
pub struct ProcessMonsterKill {
    pub monster_index: i32,
}

impl Message<ProcessMonsterKill> for PlayerActor {
    type Reply = Vec<(i32, i32, bool)>;

    async fn handle(&mut self, msg: ProcessMonsterKill, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.quest_log.process_kill(msg.monster_index)
    }
}

/// 检查任务物品进度（在背包变化后调用）
/// 怪物击杀任务进度（C# QuestInfo Kill 任务；WorldActor 怪物死亡时调用）
/// 返回 (quest_index, progress_id, is_complete)
pub struct ProcessKillQuest {
    pub monster_index: i32,
}

impl Message<ProcessKillQuest> for PlayerActor {
    type Reply = Vec<(i32, i32, bool)>;

    async fn handle(&mut self, msg: ProcessKillQuest, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.quest_log.process_kill(msg.monster_index)
    }
}

/// 任务物品拾取（C# CheckNeedQuestItem）：活跃任务含该物品 ItemTask 且未完成 →
/// 入背包 + 更新任务进度。返回是否拾取（背包满返回 false，掉落仍落地）。
pub struct TryQuestItemPickup {
    pub item: mir2_shared::data::item::UserItem,
}

impl Message<TryQuestItemPickup> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: TryQuestItemPickup, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let item_index = msg.item.item_index;
        // C# CheckNeedQuestItem：活跃任务含该物品 ItemTask 且未完成
        let needed = self.state.quest_log.quests.iter().any(|q| {
            q.progress.iter().any(|p| p.progress_id == item_index && p.current < p.target)
        });
        if !needed {
            return false;
        }
        // #1342：任务物品入独立任务格（C# QuestInventory），不再占用普通背包
        if let Some(uid) = self.state.inventory.add_quest_item(msg.item.clone()) {
            // C# GainQuestItem：Enqueue(S.GainedQuestItem{Item})
            let mut item = msg.item;
            item.unique_id = uid;
            let pkt = mir2_shared::packets::server::miscellaneous::GainedQuestItem { item };
            let mut body = Vec::new();
            if pkt.write_body(&mut body).is_ok() {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: self.state.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GainedQuestItem as i16, &body),
                }).await;
            }
            // 更新物品任务进度（C# ProcessItem：按任务格数量对齐进度）
            for quest in &mut self.state.quest_log.quests {
                for p in &mut quest.progress {
                    let count = self.state.inventory.count_quest_item_by_index(p.progress_id) as i32;
                    if count > p.current && count <= p.target {
                        p.current = count;
                    } else if count >= p.target && p.current < p.target {
                        p.current = p.target;
                    }
                }
            }
            return true;
        }
        false
    }
}

pub struct CheckQuestItemProgress;

impl Message<CheckQuestItemProgress> for PlayerActor {
    type Reply = Vec<(i32, i32, bool)>;

    async fn handle(&mut self, _msg: CheckQuestItemProgress, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let mut updated = Vec::new();
        for quest in &mut self.state.quest_log.quests {
            let mut any_changed = false;
            for p in &mut quest.progress {
                let count = self.state.inventory.count_quest_item_by_index(p.progress_id);
                let count_i32 = count as i32;
                if count_i32 > p.current && count_i32 <= p.target {
                    p.current = count_i32;
                    any_changed = true;
                } else if count_i32 >= p.target && p.current < p.target {
                    p.current = p.target;
                    any_changed = true;
                }
            }
            if any_changed {
                let complete = quest.is_progress_complete();
                // 找到变化了的进度项（取第一个变化的作为代表）
                if let Some(p) = quest.progress.first() {
                    updated.push((quest.quest_index, p.progress_id, complete));
                }
            }
        }
        updated
    }
}

// ============================================================
// 婚姻/师徒系统 Handler
// ============================================================

/// 设置配偶名称（#1329：结婚时同时写入 married_date；离婚传 None 并清零日期）
pub struct SetSpouse {
    pub spouse_name: Option<String>,
    pub married_date: i64,
}

impl Message<SetSpouse> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetSpouse, _ctx: &mut Context<Self, Self::Reply>) {
        let married = msg.spouse_name.is_some();
        self.state.spouse_name = msg.spouse_name;
        self.state.married_date = if married { msg.married_date } else { 0 };
    }
}

/// NPC 脚本 MAKEWEDDINGRING：将左戒指标记为结婚戒指（对齐 C# PlayerObject.MakeWeddingRing）
pub struct MakeWeddingRing;

impl Message<MakeWeddingRing> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, _msg: MakeWeddingRing, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        // 对齐 C# CheckMakeWeddingRing：需已婚
        if self.state.spouse_name.is_none() {
            return false;
        }
        let Some(ring) = self.state.inventory.equipment
            .get_mut(crate::actors::inventory::EquipmentSlot::RingL as usize)
        else {
            return false;
        };
        let Some(ring) = ring.as_mut() else { return false };
        // Rust 约定：0 = 未绑定（社交召回检查 wedding_ring == 0）
        if ring.wedding_ring != 0 {
            return false;
        }
        ring.wedding_ring = 1;
        self.send_equipment_changed();
        debug!("Player {} wedding ring bound", self.state.name);
        true
    }
}

/// NPC 脚本 CHANGELEVEL：设置角色等级（对齐 C# ActionType.ChangeLevel：设等级 + 经验 0 + LevelUp）
pub struct ChangeLevel {
    pub level: u16,
}

impl Message<ChangeLevel> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: ChangeLevel, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        const MAX_LEVEL: u16 = 200;
        let new_level = msg.level.min(MAX_LEVEL);
        self.state.level = new_level;
        self.state.experience = 0;
        // 按 BaseStats 公式重算基础属性（对齐 C# Settings.ClassBaseStats[Class].Calculate）
        let base_stats = mir2_shared::data::stats::BaseStats::new(self.state.class);
        for bs in &base_stats.stats {
            let val = bs.calculate(self.state.class, self.state.level as i32);
            use mir2_shared::enums::Stat;
            match bs.stat {
                Stat::HP => { self.state.max_hp = val; self.state.hp = val; }
                Stat::MP => { self.state.max_mp = val; self.state.mp = val; }
                Stat::MinDC => self.state.min_attack = val,
                Stat::MaxDC => self.state.max_attack = val,
                Stat::MinMC => self.state.min_mc = val,
                Stat::MaxMC => self.state.max_mc = val,
                Stat::MinSC => self.state.min_sc = val,
                Stat::MaxSC => self.state.max_sc = val,
                Stat::MinAC => self.state.min_ac = val,
                Stat::MaxAC => { self.state.max_ac = val; self.state.defence = val; }
                Stat::MinMAC => self.state.min_mac = val,
                Stat::MaxMAC => self.state.max_mac = val,
                Stat::Agility => self.state.agility = val,
                Stat::Accuracy => self.state.accuracy = val,
                _ => {}
            }
        }
        // 发 LevelChanged
        let mut lv_body = Vec::new();
        lv_body.extend_from_slice(&self.state.level.to_le_bytes());
        lv_body.extend_from_slice(&self.state.experience.to_le_bytes());
        lv_body.extend_from_slice(&self.state.max_experience.to_le_bytes());
        let _ = self.gate_ref.tell(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::LevelChanged as i16, &lv_body),
        }).await;
        // #283：通知 WorldActor 广播 ObjectLeveled
        let _ = self.world_ref
            .tell(crate::actors::world::PlayerLeveled {
                session_id: self.state.session_id,
                object_id: self.state.object_id,
                level: self.state.level,
            })
            .try_send();
        info!("Player {} level changed to {}", self.state.name, self.state.level);
    }
}

/// NPC 脚本 CANGAINEXP：设置是否可获得经验（对齐 C# ActionType.CanGainExp）
pub struct SetCanGainExp {
    pub can: bool,
}

impl Message<SetCanGainExp> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetCanGainExp, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.can_gain_exp = msg.can;
        debug!("Player {} can_gain_exp={}", self.state.name, msg.can);
    }
}

/// NPC 脚本 GIVEPEARLS：增加珍珠（对齐 C# ActionType.GivePearls / IntelligentCreatureGainPearls）
pub struct GainPearls {
    pub amount: u32,
}

impl Message<GainPearls> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: GainPearls, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let capped = (msg.amount as i64 + self.state.pearl_count as i64).min(i32::MAX as i64) as i32;
        self.state.pearl_count = capped;
        debug!("Player {} pearls +{} (total={})", self.state.name, msg.amount, self.state.pearl_count);
    }
}

/// NPC 脚本 TAKEPEARLS：减少珍珠（对齐 C# ActionType.TakePearls / IntelligentCreatureLosePearls，下限 0）
pub struct LosePearls {
    pub amount: u32,
}

impl Message<LosePearls> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: LosePearls, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.pearl_count = (self.state.pearl_count - msg.amount as i32).max(0);
        debug!("Player {} pearls -{} (total={})", self.state.name, msg.amount, self.state.pearl_count);
    }
}





/// 设置是否允许拜师
pub struct SetAllowMentor {
    pub allow: bool,
}

impl Message<SetAllowMentor> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetAllowMentor, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.allow_mentor = msg.allow;
    }
}

/// 设置导师名称
pub struct SetMentor {
    pub mentor_name: Option<String>,
    /// 是否导师（C# CharacterInfo.IsMentor；拜师接受方=true）
    pub is_mentor: bool,
}

impl Message<SetMentor> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetMentor, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.mentor_name = msg.mentor_name;
        self.state.is_mentor = msg.is_mentor;
    }
}

/// 设置导师伤害加成激活状态（C# BuffType.Mentor 存在性，WorldActor 近身检查后设置）
pub struct SetMentorDamageBonus {
    pub active: bool,
}

impl Message<SetMentorDamageBonus> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetMentorDamageBonus,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.mentor_damage_bonus = msg.active;
    }
}

/// 设置新手行会经验 buff 激活状态（C# BuffType.Newbie 存在性）
pub struct SetNewbieExpBonus {
    pub active: bool,
}

impl Message<SetNewbieExpBonus> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetNewbieExpBonus,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.newbie_exp_bonus = msg.active;
    }
}

/// 设置灰名截止时间（C# BrownTime；WorldActor MarkBrown 设置）
pub struct SetBrownTime {
    pub until_ms: i64,
}

impl Message<SetBrownTime> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetBrownTime,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.brown_until_ms = msg.until_ms;
    }
}

/// 骑乘移动时扣坐骑忠诚度（C# HumanObject.DecreaseMountLoyalty：LoyaltyDelay=1000ms 限速）
pub struct DecreaseMountLoyalty {
    pub amount: u16,
}

impl Message<DecreaseMountLoyalty> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: DecreaseMountLoyalty,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        if now_ms < self.state.mount_loyalty_decrease_time {
            return;
        }
        self.state.mount_loyalty_decrease_time = now_ms + 1000; // C# LoyaltyDelay
        let slot = crate::actors::inventory::EquipmentSlot::Mount as usize;
        if !self.state.is_mounted {
            return;
        }
        if let Some(mount) = self.state.inventory.equipment[slot].as_mut() {
            if mount.current_dura == 0 {
                return;
            }
            // C# DamageItem(mount, amount)：NoDuraLoss 免疫、Strong 减免
            let no_dura_loss = mount.info.as_ref()
                .map(|i| i.unique.contains(mir2_shared::enums::SpecialItemMode::NO_DURA_LOSS))
                .unwrap_or(false);
            if no_dura_loss {
                return;
            }
            let strong = mount.info.as_ref()
                .map(|i| i.stats.get(mir2_shared::enums::Stat::Strong))
                .unwrap_or(0)
                .max(0) as u16;
            let amount = msg.amount.saturating_sub(strong).max(1);
            mount.current_dura = mount.current_dura.saturating_sub(amount);
            mount.dura_changed = true;
            // S.DuraChanged
            let dc = mir2_shared::packets::server::experience::DuraChanged {
                unique_id: mount.unique_id,
                current_dura: mount.current_dura,
            };
            let mut body = Vec::new();
            if dc.write_body(&mut body).is_ok() {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: self.state.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::DuraChanged as i16, &body),
                }).await;
            }
            if mount.current_dura == 0 {
                // C# RefreshMount：耐久归零自动下坐骑
                self.state.inventory.equipment[slot] = None;
                self.state.is_mounted = false;
                self.state.mount_type = 0;
                self.send_equipment_changed();
                crate::actors::world::send_system_message(&self.gate_ref, self.state.session_id, "坐骑忠诚度耗尽，已自动下马");
            }
        }
    }
}

/// 设置宠物信息
pub struct SetCreature {
    pub creature_log: CreatureLog,
}

impl Message<SetCreature> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetCreature, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.creature_log = msg.creature_log;
    }
}

/// 宠物饥饿计时
pub struct TickCreatureHunger {
    pub dt_seconds: u32,
}

impl Message<TickCreatureHunger> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: TickCreatureHunger, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.creature_log.tick(msg.dt_seconds);
    }
}

/// 恢复宠物饥饿值
pub struct RestoreCreatureHunger {
    pub amount: u8,
}

impl Message<RestoreCreatureHunger> for PlayerActor {
    type Reply = bool; // true = restored, false = no active pet

    async fn handle(&mut self, msg: RestoreCreatureHunger, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if let Some(ref mut creature) = self.state.creature_log.active_creature {
            if creature.enabled {
                creature.restore_hunger(msg.amount);
                return true;
            }
        }
        false
    }
}

/// 设置坐骑骑乘状态（C# RidingMount；装备坐骑即骑乘）
pub struct SetMountState {
    pub mounted: bool,
    pub mount_type: i16,
}

impl Message<SetMountState> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetMountState, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.is_mounted = msg.mounted;
        if msg.mounted {
            self.state.mount_type = msg.mount_type;
        }
        debug!("Player {} mounted={} type={}", self.state.name, msg.mounted, msg.mount_type);
    }
}

/// 设置攻击模式
pub struct SetAttackMode {
    pub mode: mir2_shared::enums::AttackMode,
}

impl Message<SetAttackMode> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetAttackMode, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.attack_mode = msg.mode;
        debug!("Player {} attack mode -> {:?}", self.state.name, msg.mode);
    }
}

/// 设置宠物模式
pub struct SetPetMode {
    pub mode: mir2_shared::enums::PetMode,
}

impl Message<SetPetMode> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetPetMode, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.pet_mode = msg.mode;
        debug!("Player {} pet mode -> {:?}", self.state.name, msg.mode);
    }
}

/// 设置技能快捷键
pub struct SetSpellKey {
    pub spell: i32,
    pub key: u8,
    pub old_key: u8,
}

impl Message<SetSpellKey> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetSpellKey, _ctx: &mut Context<Self, Self::Reply>) {
        // 客户端协议编号 = C# 编号 + 3（与 combat.rs MagicRequest 的 spell_cs 转换一致）
        let spell_cs = msg.spell.saturating_sub(3);
        let mut target_found = false;
        for magic in &mut self.state.magics {
            if magic.spell == spell_cs {
                magic.key = msg.key;
                target_found = true;
            } else if msg.key > 0 && magic.key == msg.key {
                magic.key = 0;
            }
        }
        if target_found {
            debug!("Player {} spell {} key -> {}", self.state.name, spell_cs, msg.key);
        }
    }
}

/// 技能开关切换
pub struct ToggleSpell {
    pub spell: i32,
    pub toggled: bool,
}

impl Message<ToggleSpell> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: ToggleSpell, _ctx: &mut Context<Self, Self::Reply>) {
        // 客户端协议编号 = C# 编号 + 3
        let spell_cs = msg.spell.saturating_sub(3);
        for magic in &mut self.state.magics {
            if magic.spell == spell_cs {
                magic.toggled = msg.toggled;
                debug!("Player {} spell {} toggled -> {}", self.state.name, spell_cs, msg.toggled);
                break;
            }
        }
    }
}

/// 获得法术经验 + 更新施法冷却时间
pub struct GainSpellExp {
    pub spell: u8,
    pub amount: u16,
    pub cast_time: i64,
    /// #1230：DB magic_infos（等级门控/阈值/延迟用；None 时回退旧公式）
    pub info: Option<crate::db::MagicInfo>,
}

impl Message<GainSpellExp> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: GainSpellExp, _ctx: &mut Context<Self, Self::Reply>) {
        // #214：combat 传入 SharedRust +3 spell；先更新 cast_time（C# 编号匹配）
        let spell_cs = msg.spell.saturating_sub(3) as i32;
        for magic in &mut self.state.magics {
            if magic.spell == spell_cs {
                magic.cast_time = msg.cast_time;
                break;
            }
        }
        if let Some((spell, level, experience)) = self.state.gain_spell_exp(msg.spell, msg.amount, msg.info.as_ref()) {
            // #1230：C# LevelMagic——升级时补发 S.MagicDelay（新等级延迟 DelayBase - level*DelayReduction）
            if let Some(info) = msg.info.as_ref() {
                let delay = crate::combat::magic::magic_delay(info, level) as i64;
                let md = mir2_shared::packets::server::magic_combat::MagicDelay {
                    object_id: self.state.object_id,
                    spell,
                    delay,
                };
                let mut md_body = Vec::new();
                if md.write_body(&mut md_body).is_ok() {
                    let _ = self.gate_ref.tell(crate::gate::actor::SendToClient {
                        session_id: self.state.session_id,
                        data: crate::util::wire::build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::MagicDelay as i16, &md_body,
                        ),
                    }).await;
                }
            }
            // Send MagicLeveled packet (C# S.MagicLeveled: ObjectID u32 + Spell byte + Level byte + Experience u16)
            let packet = mir2_shared::packets::server::magic::MagicLeveled {
                object_id: self.state.object_id,
                spell,
                level,
                experience,
            };
            let mut body = Vec::new();
            if let Err(e) = packet.write_body(&mut body) {
                warn!("Failed to serialize MagicLeveled: {}", e);
                return;
            }
            let _ = self.gate_ref.tell(crate::gate::actor::SendToClient {
                session_id: self.state.session_id,
                data: crate::util::wire::build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::MagicLeveled as i16, &body,
                ),
            }).await;
            debug!("GainSpellExp: {} leveled spell={:?} -> {}", self.state.name, spell, level);
        }
    }
}

/// 设置英雄索引
pub struct SetHeroIndex {
    pub hero_index: u8,
}

impl Message<SetHeroIndex> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetHeroIndex, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.hero_index = msg.hero_index;
    }
}

/// 设置英雄行为模式
pub struct SetHeroBehaviour {
    pub behaviour: u8,
}

impl Message<SetHeroBehaviour> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetHeroBehaviour, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.hero_behaviour = msg.behaviour;
        debug!("Player {} hero behaviour -> {}", self.state.name, msg.behaviour);
    }
}

/// 设置自动药水阈值
pub struct SetAutoPotValue {
    pub stat: u8,
    pub value: u32,
}

// C# Stat enum values: HP=12, MP=13
const STAT_HP: u8 = 12;
const STAT_MP: u8 = 13;

impl Message<SetAutoPotValue> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetAutoPotValue, _ctx: &mut Context<Self, Self::Reply>) {
        // C# SetAutoPotValue：value = Math.Min(99, value)（英雄自动喝药阈值）
        let value = msg.value.min(99);
        match msg.stat {
            STAT_HP => { self.state.auto_pot_hp = value; debug!("Player {} auto_pot_hp -> {}", self.state.name, value); }
            STAT_MP => { self.state.auto_pot_mp = value; debug!("Player {} auto_pot_mp -> {}", self.state.name, value); }
            _ => {}
        }
    }
}

/// 设置自动药水物品
pub struct SetAutoPotItem {
    pub grid: u8,
    pub item_index: i32,
}

// #1576：线协议 grid 用 SharedRust MirGridType 枚举值（客户端 S/C 包均 MirGridType::try_from(u8) 解析）：
// HeroHpItem=26 / HeroMpItem=27。旧值 23/24 对应 Socket/HeroEquipment，导致自动药物品格永不匹配。
const GRID_HERO_HP_ITEM: u8 = mir2_shared::enums::MirGridType::HeroHpItem as u8;
const GRID_HERO_MP_ITEM: u8 = mir2_shared::enums::MirGridType::HeroMpItem as u8;

impl Message<SetAutoPotItem> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetAutoPotItem, _ctx: &mut Context<Self, Self::Reply>) {
        match msg.grid {
            GRID_HERO_HP_ITEM => { self.state.auto_pot_hp_item = msg.item_index; debug!("Player {} auto_pot_hp_item -> {}", self.state.name, msg.item_index); }
            GRID_HERO_MP_ITEM => { self.state.auto_pot_mp_item = msg.item_index; debug!("Player {} auto_pot_mp_item -> {}", self.state.name, msg.item_index); }
            _ => {}
        }
    }
}

/// 从装备插槽中移除物品（RemoveSlotItem）
/// #1313：钓具穿戴（C# EquipSlotItem GridTo=Fishing → 鱼竿 slots[FishingSlot]）
pub struct EquipFishingGear {
    pub rod_uid: u64,
    pub slot: usize,
    pub gear_uid: u64,
}

pub struct RemoveSlotItemMsg {
    pub grid: u8,
    pub grid_to: u8,
    pub unique_id: u64,
    pub to: i32,
    pub from_unique_id: u64,
}

// MirGridType values
const GRID_MOUNT: u8 = 11;
const GRID_FISHING: u8 = 12;
const GRID_SOCKET: u8 = 14;
const GRID_INVENTORY: u8 = 1;

impl Message<EquipFishingGear> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: EquipFishingGear, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if crate::actors::inventory::equip_fishing_gear(
            &mut self.state.inventory,
            msg.rod_uid,
            msg.slot,
            msg.gear_uid,
        )
        .is_ok()
        {
            self.send_inventory_changed();
            self.send_equipment_changed();
            true
        } else {
            false
        }
    }
}


/// #1313：抛竿消耗鱼饵（C# ConsumeItem Bait）
pub struct FishingConsumeBait {
    pub amount: u16,
}

impl Message<FishingConsumeBait> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: FishingConsumeBait, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let ok = self.state.inventory.fishing_consume_bait(msg.amount);
        if ok {
            self.send_equipment_changed();
        }
        ok
    }
}

/// #1313：钓具耐久 -amount（C# DamagedFishingItem）
pub struct FishingGearDamageMsg {
    pub slot: usize,
    pub amount: u16,
}

impl Message<FishingGearDamageMsg> for PlayerActor {
    type Reply = u8; // 0=无钓具 1=正常 2=损坏移除

    async fn handle(&mut self, msg: FishingGearDamageMsg, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        use crate::actors::inventory::FishingGearDamageResult;{
            let r = self.state.inventory.fishing_gear_damage(msg.slot, msg.amount);
            if r != FishingGearDamageResult::NoGear {
                self.send_equipment_changed();
            }
            match r {
                FishingGearDamageResult::NoGear => 0,
                FishingGearDamageResult::Ok => 1,
                FishingGearDamageResult::Broken => 2,
            }
        }
    }
}

/// #1313：鱼竿耐久 -amount（C# DamageItem(rod,1)）
pub struct FishingRodDurability {
    pub amount: u16,
}

impl Message<FishingRodDurability> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: FishingRodDurability, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.inventory.fishing_rod_durability_loss(msg.amount);
        self.send_equipment_changed();
    }
}

const GRID_STORAGE: u8 = 4;

impl Message<RemoveSlotItemMsg> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: RemoveSlotItemMsg, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        // Find the parent equipment item based on grid type
        let equip_index = match msg.grid {
            GRID_MOUNT => Some(EquipmentSlot::Mount as usize),
            GRID_FISHING => Some(EquipmentSlot::Weapon as usize),
            GRID_SOCKET => {
                self.state.inventory.equipment.iter()
                    .position(|e| e.as_ref().map_or(false, |i| i.unique_id == msg.from_unique_id))
            }
            _ => None,
        };

        let equip_idx = match equip_index {
            Some(i) => i,
            None => return false,
        };

        // Find and extract the slotted item from parent's slots array
        let removed = if msg.grid == GRID_SOCKET {
            // For Socket, parent might be in equipment or backpack
            if let Some(Some(item)) = self.state.inventory.equipment.get_mut(equip_idx) {
                let pos = item.slots.iter().position(|s| s.as_ref().map_or(false, |i| i.unique_id == msg.unique_id));
                pos.and_then(|p| item.slots.get_mut(p).and_then(|s| s.take()))
            } else if let Some(Some(slot)) = self.state.inventory.backpack.get_mut(equip_idx) {
                let pos = slot.item.slots.iter().position(|s| s.as_ref().map_or(false, |i| i.unique_id == msg.unique_id));
                pos.and_then(|p| slot.item.slots.get_mut(p).and_then(|s| s.take()))
            } else {
                None
            }
        } else {
            match self.state.inventory.equipment.get_mut(equip_idx) {
                Some(Some(item)) => {
                    let pos = item.slots.iter().position(|s| s.as_ref().map_or(false, |i| i.unique_id == msg.unique_id));
                    pos.and_then(|p| item.slots.get_mut(p).and_then(|s| s.take()))
                }
                _ => None,
            }
        };

        let removed_item = match removed {
            Some(item) => item,
            None => return false,
        };
        // C# RemoveSlotItem：slotTemp.Cursed && !UnlockCurse → 拒绝
        if removed_item.cursed && !self.state.unlock_curse {
            return false;
        }
        let was_cursed = removed_item.cursed;
        // Place into destination grid
        let success = match msg.grid_to {
            GRID_INVENTORY | GRID_STORAGE => {
                let to_idx = msg.to as usize;
                self.state.inventory.try_place_item_at(removed_item, to_idx)
            }
            _ => false,
        };

        if success {
            self.send_inventory_changed();
            self.send_equipment_changed();
            // C#：卸下诅咒物品后 UnlockCurse 复位
            if was_cursed {
                self.state.unlock_curse = false;
            }
            debug!("Player {} removed slot item uid={} -> grid_to={} to={}", self.state.name, msg.unique_id, msg.grid_to, msg.to);
        }
        success
    }
}

/// 设置玩家位置
pub struct SetPlayerPosition {
    pub x: i32,
    pub y: i32,
    pub direction: u8,
    pub map_index: Option<u16>,
    pub is_mounted: Option<bool>,
}

impl Message<SetPlayerPosition> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetPlayerPosition, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.x = msg.x;
        self.state.y = msg.y;
        self.state.direction = msg.direction;
        if let Some(mi) = msg.map_index {
            self.state.map_index = mi;
        }
        if let Some(mounted) = msg.is_mounted {
            self.state.is_mounted = mounted;
        }
    }
}

/// 设置组队召回冷却时间
pub struct SetLastRecallTime {
    pub last_recall_time: u64,
}

impl Message<SetLastRecallTime> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetLastRecallTime, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.last_recall_time = msg.last_recall_time;
    }
}

/// 设置是否允许组队召回
pub struct SetEnableGroupRecall {
    pub enable: bool,
}

impl Message<SetEnableGroupRecall> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetEnableGroupRecall, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.enable_group_recall = msg.enable;
    }
}

/// 设置是否允许配偶召回（对应 C# AllowLoverRecall）
pub struct SetAllowLoverRecall {
    pub allow: bool,
}

impl Message<SetAllowLoverRecall> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetAllowLoverRecall, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.allow_lover_recall = msg.allow;
    }
}

/// 检查能否获得物品（背包是否有空间）
pub struct CanGainItems;

impl Message<CanGainItems> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, _msg: CanGainItems, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.inventory.can_gain_items()
    }
}

/// 检查能否获得金币（是否超过上限）
pub struct CanGainGold {
    pub amount: u32,
}

impl Message<CanGainGold> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: CanGainGold, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        (msg.amount as u64) + self.state.inventory.gold <= u32::MAX as u64
    }
}

/// 设置钓鱼状态
pub struct SetFishing {
    pub is_fishing: bool,
    pub autocast: bool,
}

impl Message<SetFishing> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetFishing, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.is_fishing = msg.is_fishing;
        self.state.fishing_autocast = msg.autocast;
    }
}

/// 召唤护身符消耗（纯逻辑，#973）
impl PlayerState {
    /// C# HumanObject.GetAmulet + ConsumeItem：装备槽 Pendant、ItemType::Amulet、shape==0、count>=amount。
    /// 扣减 count；count 归零移除装备。返回是否消耗成功（移除时需由调用方刷新装备）。
    pub fn consume_amulet_for_summon(&mut self, amount: u16) -> bool {
        use crate::actors::inventory::EquipmentSlot;
        let slot = EquipmentSlot::Pendant as usize;
        let Some(item) = self.inventory.equipment.get_mut(slot) else {
            return false;
        };
        let Some(item) = item.as_mut() else {
            return false;
        };
        let Some(info) = item.info.as_ref() else {
            return false;
        };
        if info.item_type != mir2_shared::enums::ItemType::Amulet || info.shape != 0 || item.count < amount {
            return false;
        }
        item.count -= amount;
        if item.count == 0 {
            self.inventory.equipment[slot] = None;
        }
        true
    }

    /// 装备毒护符 shape（#1453：C# GetPoison(1,1)=绿 / (1,2)=红；Pendant 槽 Amulet shape 1/2；0=无）
    pub fn equipped_poison_shape(&self) -> i32 {
        use crate::actors::inventory::EquipmentSlot;
        let Some(Some(item)) = self.inventory.equipment.get(EquipmentSlot::Pendant as usize) else {
            return 0;
        };
        let Some(info) = item.info.as_ref() else {
            return 0;
        };
        if info.item_type != mir2_shared::enums::ItemType::Amulet {
            return 0;
        }
        if info.shape == 1i16 || info.shape == 2i16 { info.shape as i32 } else { 0 }
    }

    /// C# HumanObject.GetPoison + ConsumeItem：消耗 1 个指定 shape 的毒护符（shape 1=绿/2=红）
    pub fn consume_poison_amulet(&mut self, shape: u16) -> bool {
        use crate::actors::inventory::EquipmentSlot;
        let slot = EquipmentSlot::Pendant as usize;
        let Some(item) = self.inventory.equipment.get_mut(slot) else {
            return false;
        };
        let Some(item) = item.as_mut() else {
            return false;
        };
        let Some(info) = item.info.as_ref() else {
            return false;
        };
        if info.item_type != mir2_shared::enums::ItemType::Amulet || info.shape != shape as i16 || item.count < 1 {
            return false;
        }
        item.count -= 1;
        if item.count == 0 {
            self.inventory.equipment[slot] = None;
        }
        true
    }
}

/// 英雄↔主背包物品转移（纯逻辑，#203）
impl PlayerState {
    /// 英雄背包格 → 主背包格（C# TakeBackHeroItem 语义）
    pub fn take_back_hero_item(&mut self, from: i32, to: i32) -> bool {
        let from = from as usize;
        let to = to as usize;
        if from >= self.hero_inventory.backpack.len() || to >= self.inventory.backpack.len() {
            return false;
        }
        // 目标格空闲则直放，否则走合并/找空位；失败放回英雄背包
        if let Some(mut slot) = self.hero_inventory.backpack[from].take() {
            slot.grid = to as u8;
            if self.inventory.backpack[to].is_none() {
                self.inventory.backpack[to] = Some(slot);
            } else if self.inventory.add_item(slot.item.clone()).is_none() {
                self.hero_inventory.backpack[from] = Some(slot);
                return false;
            }
            true
        } else {
            false
        }
    }

    /// 主背包格 → 英雄背包格（C# TransferHeroItem 语义）
    pub fn transfer_hero_item(&mut self, from: i32, to: i32) -> bool {
        let from = from as usize;
        let to = to as usize;
        if from >= self.inventory.backpack.len() || to >= self.hero_inventory.backpack.len() {
            return false;
        }
        if let Some(mut slot) = self.inventory.backpack[from].take() {
            slot.grid = to as u8;
            if self.hero_inventory.backpack[to].is_none() {
                self.hero_inventory.backpack[to] = Some(slot);
            } else if self.hero_inventory.add_item(slot.item.clone()).is_none() {
                self.inventory.backpack[from] = Some(slot);
                return false;
            }
            true
        } else {
            false
        }
    }

    /// 按 item_index 查找英雄背包里的药水（#1182 自动药 TryAutoPot：找第一个同 index 的堆叠）
    pub fn find_hero_potion(&self, item_index: i32) -> Option<mir2_shared::data::item::UserItem> {
        self.hero_inventory
            .backpack
            .iter()
            .flatten()
            .find(|s| s.item.item_index == item_index && s.item.count > 0)
            .map(|s| s.item.clone())
    }

    /// 消耗英雄背包物品（#218；#1182 起支持堆叠：count>1 时 -1，否则移除整格）
    pub fn consume_hero_item(&mut self, unique_id: u64) -> bool {
        for slot in self.hero_inventory.backpack.iter_mut() {
            if let Some(s) = slot {
                if s.item.unique_id == unique_id {
                    // 堆叠物品（药水）只消耗 1 个；非堆叠（书/卷轴）移除整格
                    if s.item.count > 1 {
                        s.item.count -= 1;
                    } else {
                        *slot = None;
                    }
                    return true;
                }
            }
        }
        false
    }
}
/// 技能经验/升级（#214）
impl PlayerState {
    /// 施法获得技能经验（入参为 SharedRust +3 spell，内部转 C# 编号匹配）
    /// 返回 (SharedRust Spell, 新等级, 经验)——仅升级时返回 Some
    /// #1230：对齐 C# LevelMagic——玩家等级门控（Lv0 需 Info.Level1，Lv1 需 Level2，Lv2 需 Level3）
    /// 与阈值 DB magic_infos.need1/need2/need3（info=None 时回退旧公式 (level+1)*1000）
    pub fn gain_spell_exp(
        &mut self,
        spell_shared: u8,
        amount: u16,
        info: Option<&crate::db::MagicInfo>,
    ) -> Option<(mir2_shared::enums::Spell, u8, u16)> {
        let spell_cs = spell_shared.saturating_sub(3) as i32;
        let magic = self.magics.iter_mut().find(|m| m.spell == spell_cs)?;
        if magic.level >= 3 {
            return None;
        }
        if let Some(i) = info {
            let gate = match magic.level {
                0 => i.level1,
                1 => i.level2,
                2 => i.level3,
                _ => return None,
            };
            if gate > 0 && (self.level as i32) < gate {
                return None;
            }
        }
        // #942：C# SpecialItemMode.Skill——技能经验 ×3（Stats[SkillGainMultiplier]=3）
        // #1246：破损装备特殊模式失效（C# RefreshStats continue）
        let mut amount = amount;
        let skill_multiplier = self.inventory.equipment.iter().flatten().any(|it| {
            let broken =
                it.current_dura == 0 && it.info.as_ref().map(|i| i.durability > 0).unwrap_or(false);
            !broken
                && it
                    .info
                    .as_ref()
                    .map(|i| {
                        i.unique
                            .contains(mir2_shared::enums::SpecialItemMode::SKILL)
                    })
                    .unwrap_or(false)
        });
        if skill_multiplier {
            amount = amount.saturating_mul(3);
        } else if self.is_mentor && self.mentor_damage_bonus {
            // #1305：C# LevelMagic MentorSkillBoost——导师且徒弟同组近身时技能经验 ×2
            //（C#：仅当 Stats[SkillGainMultiplier]==1 时 ×2，随后再 ×SkillGainMultiplier）
            amount = amount.saturating_mul(2);
        }
        magic.experience = magic.experience.saturating_add(amount);
        let xp_needed = match info {
            Some(i) => match magic.level {
                0 => i.need1.max(1) as u16,
                1 => i.need2.max(1) as u16,
                2 => i.need3.max(1) as u16,
                _ => return None,
            },
            None => (magic.level as u16 + 1) * 1000,
        };
        if magic.experience >= xp_needed && magic.level < 3 {
            magic.level += 1;
            // C# LevelMagic：case 2 满级清零；case 0/1 余数结转（Experience -= NeedX）
            if magic.level >= 3 {
                magic.experience = 0;
            } else {
                magic.experience = magic.experience.saturating_sub(xp_needed);
            }
            let spell = mir2_shared::enums::Spell::try_from(spell_shared)
                .unwrap_or(mir2_shared::enums::Spell::None);
            return Some((spell, magic.level, magic.experience));
        }
        None
    }
}
/// 技能学习（#212 / #218 / #220）
impl PlayerState {
    /// 学习技能：未学习则加入（C# PlayerObject.UseItem Book 语义）
    pub fn learn_magic(&mut self, spell: i32) -> bool {
        if self.magics.iter().any(|m| m.spell == spell) {
            return false;
        }
        self.magics.push(PlayerMagic::new(spell));
        true
    }

    /// 英雄学习技能（#218）：未学习则加入英雄魔法列表
    pub fn hero_learn_magic(&mut self, spell: i32) -> bool {
        if self.hero_magics.iter().any(|m| m.spell == spell) {
            return false;
        }
        self.hero_magics.push(PlayerMagic::new(spell));
        true
    }

    /// 英雄施法获得技能经验（#220）：入参为 SharedRust +3 spell，内部转 C# 编号匹配
    /// 返回 (SharedRust Spell, 新等级, 经验)——仅升级时返回 Some
    /// #1230：对齐 C# LevelMagic（英雄继承 HumanObject）——等级门控 + DB need 阈值 + Skill ×3
    pub fn gain_hero_spell_exp(
        &mut self,
        spell_shared: u8,
        amount: u16,
        info: Option<&crate::db::MagicInfo>,
    ) -> Option<(mir2_shared::enums::Spell, u8, u16)> {
        let spell_cs = spell_shared.saturating_sub(3) as i32;
        let magic = self.hero_magics.iter_mut().find(|m| m.spell == spell_cs)?;
        if magic.level >= 3 {
            return None;
        }
        if let Some(i) = info {
            let gate = match magic.level {
                0 => i.level1,
                1 => i.level2,
                2 => i.level3,
                _ => return None,
            };
            if gate > 0 && (self.level as i32) < gate {
                return None;
            }
        }
        // #942：C# SpecialItemMode.Skill——技能经验 ×3（英雄装备同样生效）
        // #1246：破损装备特殊模式失效（C# RefreshStats continue）
        let mut amount = amount;
        let skill_multiplier = self.hero_inventory.equipment.iter().flatten().any(|it| {
            let broken =
                it.current_dura == 0 && it.info.as_ref().map(|i| i.durability > 0).unwrap_or(false);
            !broken
                && it
                    .info
                    .as_ref()
                    .map(|i| {
                        i.unique
                            .contains(mir2_shared::enums::SpecialItemMode::SKILL)
                    })
                    .unwrap_or(false)
        });
        if skill_multiplier {
            amount = amount.saturating_mul(3);
        } else if self.is_mentor && self.mentor_damage_bonus {
            // #1305：C# LevelMagic MentorSkillBoost——导师且徒弟同组近身时技能经验 ×2
            amount = amount.saturating_mul(2);
        }
        magic.experience = magic.experience.saturating_add(amount);
        let xp_needed = match info {
            Some(i) => match magic.level {
                0 => i.need1.max(1) as u16,
                1 => i.need2.max(1) as u16,
                2 => i.need3.max(1) as u16,
                _ => return None,
            },
            None => (magic.level as u16 + 1) * 1000,
        };
        if magic.experience >= xp_needed && magic.level < 3 {
            magic.level += 1;
            // C# LevelMagic：case 2 满级清零；case 0/1 余数结转（Experience -= NeedX）
            if magic.level >= 3 {
                magic.experience = 0;
            } else {
                magic.experience = magic.experience.saturating_sub(xp_needed);
            }
            let spell = mir2_shared::enums::Spell::try_from(spell_shared)
                .unwrap_or(mir2_shared::enums::Spell::None);
            return Some((spell, magic.level, magic.experience));
        }
        None
    }
}

/// 英雄施法技能经验（#220：返回升级信息，由 WorldActor 发送 S.MagicLeveled）
pub struct GainHeroSpellExp {
    pub spell_shared: u8,
    pub amount: u16,
    /// #1230：DB magic_infos（等级门控/阈值用；None 时回退旧公式）
    pub info: Option<crate::db::MagicInfo>,
}

impl Message<GainHeroSpellExp> for PlayerActor {
    type Reply = Option<(mir2_shared::enums::Spell, u8, u16)>;

    async fn handle(
        &mut self,
        msg: GainHeroSpellExp,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.gain_hero_spell_exp(msg.spell_shared, msg.amount, msg.info.as_ref())
    }
}

/// 是否已学习技能
pub struct IsMagicLearned {
    pub spell: i32,
}

impl Message<IsMagicLearned> for PlayerActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        msg: IsMagicLearned,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.magics.iter().any(|m| m.spell == msg.spell)
    }
}

/// 学习技能（#212）
pub struct LearnMagic {
    pub spell: i32,
}

impl Message<LearnMagic> for PlayerActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        msg: LearnMagic,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.learn_magic(msg.spell)
    }
}

/// 学习技能并设置等级（NPC 脚本 GIVESKILL 用，对齐 C# GiveSkill 设 Level，最多 3 级）
pub struct LearnMagicWithLevel {
    pub spell: i32,
    pub level: u8,
}

impl Message<LearnMagicWithLevel> for PlayerActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        msg: LearnMagicWithLevel,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if self.state.magics.iter().any(|m| m.spell == msg.spell) {
            // 已学会：不重复添加（对齐 C# 已学直接 break）
            return false;
        }
        let mut magic = PlayerMagic::new(msg.spell);
        magic.level = msg.level.min(3);
        self.state.magics.push(magic);
        true
    }
}
/// 英雄装备/卸下（#206）
impl PlayerState {
    /// 英雄背包格 → 英雄装备槽（C# C.EquipItem Grid=HeroInventory）
    pub fn hero_equip_item(
        &mut self,
        slot: crate::actors::inventory::EquipmentSlot,
        uid: u64,
    ) -> bool {
        let Some(idx) = self
            .hero_inventory
            .backpack
            .iter()
            .position(|s| s.as_ref().is_some_and(|s| s.item.unique_id == uid))
        else {
            return false;
        };
        let Some(slot_item) = self.hero_inventory.backpack[idx].take() else {
            return false;
        };
        let slot_i = slot as usize;
        if slot_i >= self.hero_inventory.equipment.len() {
            self.hero_inventory.backpack[idx] = Some(slot_item);
            return false;
        }
        // 旧装备放回英雄背包（找空位；背包满则换回）
        if let Some(old) = self.hero_inventory.equipment[slot_i].take() {
            if let Some((gi, empty)) = self
                .hero_inventory
                .backpack
                .iter_mut()
                .enumerate()
                .find(|(_, s)| s.is_none())
            {
                *empty = Some(crate::actors::inventory::InventorySlot {
                    grid: gi as u8,
                    item: old,
                });
            } else {
                self.hero_inventory.equipment[slot_i] = Some(old);
                self.hero_inventory.backpack[idx] = Some(slot_item);
                return false;
            }
        }
        self.hero_inventory.equipment[slot_i] = Some(slot_item.item);
        true
    }

    /// 英雄装备槽 → 英雄背包（C# C.RemoveItem Grid=HeroEquipment）
    pub fn hero_remove_item(&mut self, uid: u64) -> bool {
        let Some(slot_i) = self
            .hero_inventory
            .equipment
            .iter()
            .position(|s| s.as_ref().is_some_and(|i| i.unique_id == uid))
        else {
            return false;
        };
        let Some(item) = self.hero_inventory.equipment[slot_i].take() else {
            return false;
        };
        if let Some((gi, empty)) = self
            .hero_inventory
            .backpack
            .iter_mut()
            .enumerate()
            .find(|(_, s)| s.is_none())
        {
            *empty = Some(crate::actors::inventory::InventorySlot {
                grid: gi as u8,
                item,
            });
            true
        } else {
            self.hero_inventory.equipment[slot_i] = Some(item);
            false
        }
    }
}

/// 英雄装备（#206：背包 → 装备槽）
pub struct HeroEquipItem {
    pub slot: crate::actors::inventory::EquipmentSlot,
    pub unique_id: u64,
}

impl Message<HeroEquipItem> for PlayerActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        msg: HeroEquipItem,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.hero_equip_item(msg.slot, msg.unique_id)
    }
}

/// 英雄卸下（#206：装备槽 → 背包）
pub struct HeroRemoveItem {
    pub unique_id: u64,
}

impl Message<HeroRemoveItem> for PlayerActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        msg: HeroRemoveItem,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state.hero_remove_item(msg.unique_id)
    }
}
/// 从英雄背包取回物品到主背包（C# C.TakeBackHeroItem: From=英雄格 To=主背包格，#203）
pub struct TakeBackHeroItem {
    pub from: i32,
    pub to: i32,
}

impl Message<TakeBackHeroItem> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: TakeBackHeroItem, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.take_back_hero_item(msg.from, msg.to);
    }
}

/// 从主背包转移物品到英雄背包（C# C.TransferHeroItem: From=主背包格 To=英雄格，#203）
pub struct TransferHeroItem {
    pub from: i32,
    pub to: i32,
}

impl Message<TransferHeroItem> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: TransferHeroItem, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.transfer_hero_item(msg.from, msg.to);
    }
}

/// 设置精炼日志
pub struct SetRefineLog {
    pub refine_log: crate::actors::refine::RefineLog,
}

impl Message<SetRefineLog> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetRefineLog, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.refine_log = msg.refine_log;
    }
}

/// 存入仓库（C# StoreItem{From=背包格, To=仓库格}）
pub struct StoreItemTo {
    pub from: i32,
    pub to: i32,
}

impl Message<StoreItemTo> for PlayerActor {
    type Reply = Option<(mir2_shared::data::item::UserItem, usize)>;

    async fn handle(&mut self, msg: StoreItemTo, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.inventory.store_item_to(msg.from, msg.to)
    }
}

/// 从仓库取出（C# TakeBackItem{From=仓库格, To=背包格}）
pub struct TakeBackItemTo {
    pub from: i32,
    pub to: i32,
}

impl Message<TakeBackItemTo> for PlayerActor {
    type Reply = Option<(mir2_shared::data::item::UserItem, u8)>;

    async fn handle(&mut self, msg: TakeBackItemTo, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.inventory.take_back_item_to(msg.from, msg.to)
    }
}

// ============================================================
// 轮回系统消息
// ============================================================

/// 轮回术 offer（#222：施法者请求复活死亡玩家）
pub struct OfferReincarnation {
    pub host_session: u64,
    pub expire_tick: u64,
}

impl Message<OfferReincarnation> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: OfferReincarnation, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.reincarnation_host = Some(msg.host_session);
        self.state.reincarnation_ready = true;
        self.state.reincarnation_expire_time = msg.expire_tick;
    }
}
/// 清除当前玩家的轮回状态（被施法者/死亡玩家使用）
pub struct ClearReincarnation;

impl Message<ClearReincarnation> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, _msg: ClearReincarnation, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.reincarnation_host = None;
        self.state.reincarnation_ready = false;
        self.state.reincarnation_expire_time = 0;
    }
}

/// 清除宿主的轮回状态（施法者使用）
pub struct ClearReincarnationHost;

impl Message<ClearReincarnationHost> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, _msg: ClearReincarnationHost, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.reincarnation_ready = false;
        self.state.reincarnation_expire_time = 0;
    }
}

/// 以一半 HP 复活
pub struct ReviveAtHalfHp;

impl Message<ReviveAtHalfHp> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, _msg: ReviveAtHalfHp, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.is_dead = false;
        self.state.hp = (self.state.max_hp / 2).max(1);

        let mut body = Vec::new();
        body.extend_from_slice(&self.state.hp.to_le_bytes());
        body.extend_from_slice(&self.state.mp.to_le_bytes());
        let _ = self.gate_ref.tell(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &body),
        }).await;

        debug!("ReviveAtHalfHp: {} hp={}/{}", self.state.name, self.state.hp, self.state.max_hp);
    }
}

// ============================================================
// 背包通知辅助函数
// ============================================================

impl PlayerActor {
    fn send_inventory_changed(&self) {
        // 发送 UserInformation 刷新（不含背包数据，客户端需主动查询）
        self.send_user_information_refresh();
    }

    /// C# RecalculateQuestBag：清除任务格中不再被任何活跃任务需要的任务物品，并逐个发 S.DeleteQuestItem
    fn recalculate_quest_bag(&mut self) {
        // 统计所有活跃任务仍需的任务物品数量（target - current，未完成项）
        let needed: Vec<(i32, u16)> = {
            let mut map: Vec<(i32, u32)> = Vec::new();
            for quest in &self.state.quest_log.quests {
                for p in &quest.progress {
                    if p.current >= p.target { continue; }
                    if let Some(e) = map.iter_mut().find(|(idx, _)| *idx == p.progress_id) {
                        e.1 += (p.target - p.current) as u32;
                    } else {
                        map.push((p.progress_id, (p.target - p.current) as u32));
                    }
                }
            }
            map.into_iter().map(|(idx, need)| (idx, need.min(u16::MAX as u32) as u16)).collect()
        };
        // 每个任务物品保留 needed 数量，移除超出部分（复用 remove_quest_item_by_index）
        let mut deletions: Vec<(u64, u16)> = Vec::new();
        let present: Vec<(i32, u16)> = {
            let mut m: Vec<(i32, u16)> = Vec::new();
            for item in self.state.inventory.quest_inventory.iter().flatten() {
                if let Some(e) = m.iter_mut().find(|(idx, _)| *idx == item.item_index) {
                    e.1 = e.1.saturating_add(item.count);
                } else {
                    m.push((item.item_index, item.count));
                }
            }
            m
        };
        for (idx, current) in present {
            let need = needed.iter().find(|(nidx, _)| *nidx == idx).map(|(_, n)| *n).unwrap_or(0);
            let keep = need.min(current);
            let excess = current - keep;
            if excess > 0 {
                deletions.extend(self.state.inventory.remove_quest_item_by_index(idx, excess));
            }
        }
        for (unique_id, count) in deletions {
            let pkt = mir2_shared::packets::server::miscellaneous::DeleteQuestItem { unique_id, count };
            let mut body = Vec::new();
            if pkt.write_body(&mut body).is_ok() {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: self.state.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::DeleteQuestItem as i16, &body),
                }).try_send();
            }
        }
    }

    fn send_equipment_changed(&self) {
        // 发送 UserInformation 刷新装备状态
        self.send_user_information_refresh();
    }

    /// #967：下发 S.RefreshItem（C# 幸运/耐久变化后客户端即时刷新物品显示）
    fn send_refresh_item(&self, item: &mir2_shared::data::item::UserItem) {
        let pkt = mir2_shared::packets::server::item::RefreshItem { item: item.clone() };
        let mut body = Vec::new();
        if pkt.write_body(&mut body).is_ok() {
            let _ = self.gate_ref.tell(SendToClient {
                session_id: self.state.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::RefreshItem as i16, &body),
            }).try_send();
        }
    }

    fn send_gold_changed(&self) {
        // 发送 UserInformation 刷新金币
        self.send_user_information_refresh();
    }

    /// 发送 UserInformation 刷新（不含完整背包数据）
    fn send_user_information_refresh(&self) {
        use mir2_shared::enums::ServerPacketIds;
        let mut body = Vec::new();

        body.extend_from_slice(&self.state.object_id.to_le_bytes());   // object_id
        body.extend_from_slice(&1u32.to_le_bytes());                    // real_id
        write_dotnet_string(&mut body, &self.state.name);               // name
        write_dotnet_string(&mut body, self.state.guild_name.as_deref().unwrap_or("")); // guild_name
        write_dotnet_string(&mut body, match self.state.guild_rank {
            crate::actors::guild::GuildRank::Leader => "掌门",
            crate::actors::guild::GuildRank::Officer => "副掌门",
            crate::actors::guild::GuildRank::Member => "成员",
        }); // guild_rank
        body.extend_from_slice(&0i32.to_le_bytes());                    // name_colour
        body.push(self.state.class as u8);                              // class
        body.push(self.state.gender as u8);                             // gender
        body.extend_from_slice(&self.state.level.to_le_bytes());        // level
        body.extend_from_slice(&self.state.x.to_le_bytes());            // location_x
        body.extend_from_slice(&self.state.y.to_le_bytes());            // location_y
        body.push(self.state.direction);                                // direction
        body.push(self.state.hair);                                     // hair
        body.extend_from_slice(&self.state.hp.to_le_bytes());  // hp
        body.extend_from_slice(&self.state.mp.to_le_bytes());  // mp
        body.extend_from_slice(&self.state.experience.to_le_bytes()); // experience
        body.extend_from_slice(&self.state.max_experience.to_le_bytes()); // max_experience
        body.extend_from_slice(&0u16.to_le_bytes());                    // level_effects
        body.push(if self.state.hero_index > 0 { 1u8 } else { 0u8 });  // has_hero
        // hero_behaviour（C# 值 0..3，与 SharedRust HeroBehaviour 一致）
        body.push(self.state.hero_behaviour);                     // hero_behaviour (C# 0..3)

        // 背包/装备数据（简化版：不发送完整物品，客户端通过 ItemChanged 等增量包更新）
        body.push(0u8);                                                 // has_inventory=false
        body.push(0u8);                                                 // has_equipment=false
        body.push(0u8);                                                 // has_quest_inventory=false
        body.extend_from_slice(&(self.state.inventory.gold as u32).to_le_bytes()); // gold
        body.extend_from_slice(&0u32.to_le_bytes());                    // credit=0
        // 仓库扩容/仓库密码（C# UserInformation：HasExpandedStorage/HasStoragePassword/
        // RequireStoragePassword/StoragePasswordLastSet/ExpandedStorageExpiryTime）
        body.push(if self.state.has_expanded_storage { 1u8 } else { 0u8 }); // has_expanded_storage
        body.push(if self.state.has_storage_password { 1u8 } else { 0u8 }); // has_storage_password
        body.push(if self.state.require_storage_password { 1u8 } else { 0u8 }); // require_storage_password
        body.extend_from_slice(&self.state.storage_password_last_set.to_le_bytes()); // storage_password_last_set
        body.extend_from_slice(&self.state.expanded_storage_expiry_date.to_le_bytes()); // expanded_storage_expiry_time
        body.extend_from_slice(&0i32.to_le_bytes());                    // magic_count=0
        body.extend_from_slice(&0i32.to_le_bytes());                    // creature_count=0
        body.push(0u8);                                                 // summoned_creature_type
        body.push(0u8);                                                 // creature_summoned=false
        body.push(if self.state.allow_observe { 1u8 } else { 0u8 }); // allow_observe
        body.push(0u8);                                                 // observer=false

        // #208：角色面板属性段（18 x i32；最终值 = 基础 + 装备加成）
        body.extend_from_slice(&(self.state.max_hp + self.state.bonus_max_hp).to_le_bytes());
        body.extend_from_slice(&(self.state.max_mp + self.state.bonus_max_mp).to_le_bytes());
        for v in [
            self.state.min_ac + self.state.bonus_min_ac,
            self.state.max_ac + self.state.bonus_max_ac,
        ] {
            body.extend_from_slice(&v.to_le_bytes());
        }
        for v in [
            self.state.min_mac + self.state.bonus_min_mac,
            self.state.max_mac + self.state.bonus_max_mac,
        ] {
            body.extend_from_slice(&v.to_le_bytes());
        }
        for v in [
            self.state.min_attack + self.state.bonus_min_attack,
            self.state.max_attack + self.state.bonus_max_attack,
        ] {
            body.extend_from_slice(&v.to_le_bytes());
        }
        for v in [
            self.state.min_mc + self.state.bonus_min_mc,
            self.state.max_mc + self.state.bonus_max_mc,
        ] {
            body.extend_from_slice(&v.to_le_bytes());
        }
        for v in [
            self.state.min_sc + self.state.bonus_min_sc,
            self.state.max_sc + self.state.bonus_max_sc,
        ] {
            body.extend_from_slice(&v.to_le_bytes());
        }
        body.extend_from_slice(&self.state.critical_rate.to_le_bytes());
        body.extend_from_slice(&self.state.critical_damage.to_le_bytes());
        body.extend_from_slice(&self.state.attack_speed.to_le_bytes()); // attack_speed（装备加成 Stat::AttackSpeed）
        body.extend_from_slice(&self.state.accuracy.to_le_bytes());
        body.extend_from_slice(&self.state.agility.to_le_bytes());
        body.extend_from_slice(&self.state.luck.to_le_bytes());

        // #210：State 页段（轻量刷新无 item_infos，负重暂填 0；全量包由 build_user_information_packet 下发）
        for v in [
            0i32, // bag_weight
            0i32, // wear_weight
            0i32, // hand_weight
            self.state.magic_resist,
            self.state.poison_resist,
            self.state.health_recovery,
            self.state.spell_recovery,
            self.state.poison_recovery,
            self.state.holy,
            self.state.freezing,
            self.state.poison_attack,
        ] {
            body.extend_from_slice(&v.to_le_bytes());
        }

        let _ = self.gate_ref.tell(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(ServerPacketIds::UserInformation as i16, &body),
        }).try_send();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn movement_blocked_by_poison_matches_csharp_canwalk() {
        // #1614：C# HumanObject.CanWalk——Paralysis/LRParalysis/Frozen 禁止移动
        use mir2_shared::enums::PoisonType;
        use crate::combat::poison::Poison;
        let list = |t: PoisonType| vec![Poison::new(t, 5, 0, 1000)];
        assert!(movement_blocked_by_poison(&list(PoisonType::PARALYSIS)));
        assert!(movement_blocked_by_poison(&list(PoisonType::LR_PARALYSIS)));
        assert!(movement_blocked_by_poison(&list(PoisonType::FROZEN)));
        assert!(!movement_blocked_by_poison(&list(PoisonType::GREEN)));
        assert!(!movement_blocked_by_poison(&[]));
    }

    #[test]
    fn hero_auto_pot_grid_constants_match_protocol() {
        // #1576：SetAutoPotItem 线协议 grid 值必须与 MirGridType 一致（HeroHpItem=26 / HeroMpItem=27）
        assert_eq!(
            GRID_HERO_HP_ITEM,
            mir2_shared::enums::MirGridType::HeroHpItem as u8,
            "英雄自动药 HP 格常量与协议错位"
        );
        assert_eq!(
            GRID_HERO_MP_ITEM,
            mir2_shared::enums::MirGridType::HeroMpItem as u8,
            "英雄自动药 MP 格常量与协议错位"
        );
    }


    use super::*;

    fn make_state() -> PlayerState {
        PlayerState {
            object_id: 1000,
            name: "TestPlayer".to_string(),
            map_index: 0,
            x: 330,
            y: 330,
            direction: 4,
            attack_mode: mir2_shared::enums::AttackMode::Peace,
            pet_mode: mir2_shared::enums::PetMode::Both,
            hidden: false,
            session_id: 1,
            class: mir2_shared::enums::MirClass::Warrior,
            gender: mir2_shared::enums::MirGender::Male,
            hair: 0,
            level: 1,
            experience: 0,
            max_experience: 100,
        can_gain_exp: true,
        pearl_count: 0,
            hp: 120,
            max_hp: 120,
            mp: 60,
            max_mp: 60,
            min_attack: 5,
            max_attack: 10,
            defence: 2,
            min_mc: 0,
            max_mc: 0,
            min_sc: 0,
            max_sc: 0,
            bonus_min_attack: 0,
            bonus_max_attack: 0,
            bonus_defence: 0,
            bonus_max_hp: 0,
            bonus_max_mp: 0,
            bonus_min_mc: 0,
            bonus_max_mc: 0,
            bonus_min_sc: 0,
            bonus_max_sc: 0,
            freezing: 0,
            poison_attack: 0,
            health_recovery: 0,
            spell_recovery: 0,
            attack_speed: 0,
            poison_resist: 0,
            poison_recovery: 0,
            holy: 0,
            accuracy: 0,
            agility: 0,
            min_ac: 0,
            max_ac: 0,
            min_mac: 0,
            max_mac: 0,
            bonus_min_ac: 0,
            bonus_max_ac: 0,
            bonus_min_mac: 0,
            bonus_max_mac: 0,
            luck: 0,
            critical_rate: 0,
            critical_damage: 0,
            magic_resist: 0,
            reflect: 0,
            damage_reduction_percent: 0,
            attack_bonus: 0,
            hp_drain_rate_percent: 0,
            energy_shield_percent: 0,
            energy_shield_hp_gain: 0,
            poison_list: Vec::new(),
            inventory: PlayerInventory::new(),
            group_id: None,
            friend_list: FriendList::new(),
            mailbox: Mailbox::new(),
            guild_name: None,
            guild_rank: GuildRank::Member,
            quest_log: QuestLog::new(),
            spouse_name: None,
            married_date: 0,
            allow_mentor: false,
            mentor_name: None,
            creature_log: CreatureLog::new(),
            hero_index: 0,
            hero_despawned: false,
            hero_inventory: PlayerInventory::new(),
            hero_magics: Vec::new(),
            refine_log: RefineLog::new(),
            is_fishing: false,
            is_mounted: false,
            mount_type: 0,
            fishing_autocast: false,
            reincarnation_host: None,
            reincarnation_ready: false,
            reincarnation_expire_time: 0,
            enable_group_recall: false,
            last_recall_time: 0,
            allow_lover_recall: false,
            is_gm: false,
            gm_never_die: false, // #1480：GM 无敌模式（C# GMNeverDie）
            special_shot_armed: 0, // #1483：弓手特殊箭武装（0=无 1=Vampire 2=Poison）
            has_expanded_storage: false,
            expanded_storage_expiry_date: 0,
            has_storage_password: false,
            require_storage_password: false,
            storage_password_last_set: 0,
            allow_observe: false,
            enable_guild_invite: false,
allow_trade: false,

allow_group: false,
            is_dead: false,
            unlock_curse: false,
            last_revival_time: 0,
            last_access: 0,
            rested_counter: 0,
            rested_exp_percent: 0,
            rested_exp_end_tick: 0,
            has_map_shout: false,
            has_server_shout: false,
            last_shout_time: 0,
            pk_points: 0,
            pk_kill_count: 0,
            buffs: Vec::new(),
            magics: Vec::new(),
            flags: std::collections::HashMap::new(),
            exp_multiplier: 1.0,
            exp_rate: 1.0,
            exp_multiplier_end_tick: 0,
            drop_multiplier: 1.0,
            drop_multiplier_end_tick: 0,
            item_drop_rate_percent: 0,
            gold_drop_rate_percent: 0,
            auto_pot_hp: 0,
            auto_pot_mp: 0,
            auto_pot_hp_item: -1,
            auto_pot_mp_item: -1,
            hero_behaviour: 0,
            elements_level: 0,
            has_elemental: false,
            concentration_interrupted: false,
            concentration_interrupt_time: 0,
            bind_map_index: 0,
            bind_x: 0,
            bind_y: 0,
            level_effects: 0,
            is_mentor: false,
            mentee_exp: 0,
            mentor_damage_bonus: false,
            newbie_exp_bonus: false,
            exp_bonus_lover_percent: 0,
            exp_bonus_mentee_percent: 0,
            exp_bonus_newbie_percent: 0,
            guild_buff_exp_percent: 0,
            guild_buff_fish_rate_percent: 0,
            no_experience_map: false,
            brown_until_ms: 0,
            mount_loyalty_decrease_time: 0,
            mount_loyalty_increase_time: 0,
            torch_burn_time: 0,
            last_damage_ms: 0,
            pot_hp_amount: 0,
            pot_mp_amount: 0,
            pot_time_ms: 0,
        }
    }

    #[test]
    fn test_spouse_initial() {
        assert!(make_state().spouse_name.is_none());
    }

    #[test]
    fn test_set_spouse() {
        let mut s = make_state();
        s.spouse_name = Some("Partner".to_string());
        assert_eq!(s.spouse_name, Some("Partner".to_string()));
        s.spouse_name = None;
        assert!(s.spouse_name.is_none());
    }

    #[test]
    fn test_allow_mentor_toggle() {
        let mut s = make_state();
        assert!(!s.allow_mentor);
        s.allow_mentor = true;
        assert!(s.allow_mentor);
        s.allow_mentor = false;
        assert!(!s.allow_mentor);
    }

    #[test]
    fn test_set_mentor() {
        let mut s = make_state();
        assert!(s.mentor_name.is_none());
        s.mentor_name = Some("Master".to_string());
        assert_eq!(s.mentor_name, Some("Master".to_string()));
        s.mentor_name = None;
        assert!(s.mentor_name.is_none());
    }

    #[test]
    fn test_married_can_have_mentor() {
        // A married player can still have a mentor
        let mut s = make_state();
        s.spouse_name = Some("Spouse".to_string());
        s.mentor_name = Some("Master".to_string());
        assert!(s.spouse_name.is_some());
        assert!(s.mentor_name.is_some());
    }

    // ---- #203 英雄↔主背包转移 ----
    fn put_item(inv: &mut PlayerInventory, grid: usize, uid: u64) {
        let mut item = mir2_shared::data::item::UserItem::new(1001);
        item.unique_id = uid;
        inv.backpack[grid] = Some(crate::actors::inventory::InventorySlot {
            grid: grid as u8,
            item,
        });
    }

    #[test]
    fn test_take_back_hero_item_to_empty_slot() {
        let mut s = make_state();
        put_item(&mut s.hero_inventory, 3, 9001);
        assert!(s.hero_inventory.backpack[3].is_some());
        assert!(s.take_back_hero_item(3, 5));
        assert!(s.hero_inventory.backpack[3].is_none());
        assert!(s.inventory.backpack[5].is_some());
        assert_eq!(s.inventory.backpack[5].as_ref().unwrap().item.unique_id, 9001);
        assert_eq!(s.inventory.backpack[5].as_ref().unwrap().grid, 5);
    }

    #[test]
    fn test_transfer_hero_item_to_empty_slot() {
        let mut s = make_state();
        put_item(&mut s.inventory, 2, 9002);
        assert!(s.transfer_hero_item(2, 7));
        assert!(s.inventory.backpack[2].is_none());
        assert!(s.hero_inventory.backpack[7].is_some());
        assert_eq!(s.hero_inventory.backpack[7].as_ref().unwrap().item.unique_id, 9002);
    }

    #[test]
    fn test_transfer_out_of_range_fails() {
        let mut s = make_state();
        put_item(&mut s.inventory, 0, 9003);
        assert!(!s.transfer_hero_item(0, 999));
        assert!(s.inventory.backpack[0].is_some());
        assert!(!s.take_back_hero_item(999, 0));
    }

    // ---- #206 英雄装备/卸下 ----
    #[test]
    fn test_hero_equip_item_moves_to_slot() {
        let mut s = make_state();
        put_item(&mut s.hero_inventory, 2, 9101);
        assert!(s.hero_equip_item(crate::actors::inventory::EquipmentSlot::Weapon, 9101));
        assert!(s.hero_inventory.backpack[2].is_none());
        let eq = s.hero_inventory.equipment
            [crate::actors::inventory::EquipmentSlot::Weapon as usize]
            .as_ref()
            .unwrap();
        assert_eq!(eq.unique_id, 9101);
    }

    #[test]
    fn test_hero_equip_unknown_uid_fails() {
        let mut s = make_state();
        assert!(!s.hero_equip_item(crate::actors::inventory::EquipmentSlot::Weapon, 9999));
    }

    #[test]
    fn test_hero_remove_item_returns_to_backpack() {
        let mut s = make_state();
        put_item(&mut s.hero_inventory, 4, 9102);
        assert!(s.hero_equip_item(crate::actors::inventory::EquipmentSlot::Armour, 9102));
        assert!(s.hero_remove_item(9102));
        assert!(s.hero_inventory.equipment
            [crate::actors::inventory::EquipmentSlot::Armour as usize]
            .is_none());
        assert!(s
            .hero_inventory
            .backpack
            .iter()
            .any(|s| s.as_ref().is_some_and(|s| s.item.unique_id == 9102)));
    }

    // ---- #1182 英雄自动药（背包药水查找/消耗） ----
    #[test]
    fn test_find_hero_potion_by_item_index() {
        let mut s = make_state();
        let mut potion = mir2_shared::data::item::UserItem::new(13); // item_index=13
        potion.unique_id = 9201;
        potion.count = 5;
        s.hero_inventory.backpack[0] = Some(crate::actors::inventory::InventorySlot {
            grid: 0,
            item: potion,
        });
        put_item(&mut s.hero_inventory, 2, 9202);
        // 按 item_index 找到药水（与格位无关）
        let found = s.find_hero_potion(13).expect("potion should be found");
        assert_eq!(found.unique_id, 9201);
        assert_eq!(found.count, 5);
        // 不存在的 index 返回 None
        assert!(s.find_hero_potion(999).is_none());
        // count=0 的堆叠不返回
        s.hero_inventory.backpack[0].as_mut().unwrap().item.count = 0;
        assert!(s.find_hero_potion(13).is_none());
    }

    #[test]
    fn test_consume_hero_item_stack_and_single() {
        let mut s = make_state();
        let mut potion = mir2_shared::data::item::UserItem::new(13);
        potion.unique_id = 9203;
        potion.count = 3;
        s.hero_inventory.backpack[0] = Some(crate::actors::inventory::InventorySlot {
            grid: 0,
            item: potion,
        });
        // 堆叠：每次消耗 1 个，格子保留
        assert!(s.consume_hero_item(9203));
        assert_eq!(s.hero_inventory.backpack[0].as_ref().unwrap().item.count, 2);
        assert!(s.consume_hero_item(9203));
        assert_eq!(s.hero_inventory.backpack[0].as_ref().unwrap().item.count, 1);
        assert!(s.consume_hero_item(9203));
        assert!(s.hero_inventory.backpack[0].is_none());
        // 未知 uid 失败
        assert!(!s.consume_hero_item(9999));
    }

    // ---- #218 英雄技能 ----
    #[test]
    fn test_hero_learn_magic_adds_once() {
        let mut s = make_state();
        assert!(s.hero_magics.is_empty());
        assert!(s.hero_learn_magic(31)); // FireBall C#
        assert_eq!(s.hero_magics.len(), 1);
        assert!(!s.hero_learn_magic(31));
    }

    // ---- #220 英雄技能升级 ----
    #[test]
    fn test_gain_hero_spell_exp_levels_up() {
        let mut s = make_state();
        assert!(s.hero_learn_magic(31)); // FireBall C#
        let r = s.gain_hero_spell_exp(34, 1000, None);
        assert!(r.is_some());
        let (spell, level, exp) = r.unwrap();
        assert_eq!(spell, mir2_shared::enums::Spell::FireBall);
        assert_eq!(level, 1);
        assert_eq!(exp, 0);
        // 3 级封顶后不再给经验
        let _ = s.gain_hero_spell_exp(34, 3000, None);
        let _ = s.gain_hero_spell_exp(34, 10000, None);
        assert!(s.gain_hero_spell_exp(34, 10000, None).is_none());
        assert_eq!(s.hero_magics[0].level, 3);
    }

    #[test]
    fn test_gain_hero_spell_exp_unlearned_ignored() {
        let mut s = make_state();
        assert!(s.gain_hero_spell_exp(34, 1000, None).is_none());
    }
    // ---- #214 技能升级 ----
    #[test]
    fn test_gain_spell_exp_levels_up() {
        let mut s = make_state();
        assert!(s.learn_magic(31)); // FireBall C#
        let r = s.gain_spell_exp(34, 1000, None);
        assert!(r.is_some());
        let (spell, level, exp) = r.unwrap();
        assert_eq!(spell, mir2_shared::enums::Spell::FireBall);
        assert_eq!(level, 1);
        assert_eq!(exp, 0);
        // 再升 2 级需 2000
        let r2 = s.gain_spell_exp(34, 2000, None);
        assert!(r2.is_some());
        assert_eq!(r2.unwrap().1, 2);
        // 3 级封顶后不再给经验
        let r3 = s.gain_spell_exp(34, 10000, None);
        assert!(r3.is_some());
        assert_eq!(r3.unwrap().1, 3);
        assert!(s.gain_spell_exp(34, 10000, None).is_none());
    }

    #[test]
    fn test_gain_spell_exp_unlearned_ignored() {
        let mut s = make_state();
        assert!(s.gain_spell_exp(34, 1000, None).is_none());
        assert!(s.gain_spell_exp(0, 1000, None).is_none()); // 基础攻击（未学）忽略
    }
    // ---- #212 技能书学习 ----
    #[test]
    fn test_learn_magic_adds_once() {
        let mut s = make_state();
        assert!(s.magics.is_empty());
        assert!(s.learn_magic(31)); // FireBall（C# 编号 = 31，SharedRust = 34）
        assert_eq!(s.magics.len(), 1);
        assert!(!s.learn_magic(31)); // 重复学习失败
        assert!(s.learn_magic(1)); // Fencing（C# 编号 = 1，SharedRust = 4）
        assert_eq!(s.magics.len(), 2);
    }

    // ---- #427 精神力剑被动 accuracy ----
    #[test]
    fn test_spirit_sword_accuracy() {
        let mut s = make_state();
        assert_eq!(spirit_sword_accuracy(&s.magics), 0);
        // SpiritSword C# 编号 = 62（SharedRust 65）
        s.magics.push(PlayerMagic::new(62));
        assert_eq!(spirit_sword_accuracy(&s.magics), 0); // 未学
        // SpiritSword C# 编号 = 62（SharedRust 65）；spiritSwordLvPlus = {0,3,5,8}
        s.magics.push(PlayerMagic::new(62));
        assert_eq!(spirit_sword_accuracy(&s.magics), 0); // Lv0 -> 0
        s.magics[0].level = 1;
        assert_eq!(spirit_sword_accuracy(&s.magics), 3);
        s.magics[0].level = 2;
        assert_eq!(spirit_sword_accuracy(&s.magics), 5);
        s.magics[0].level = 3;
        assert_eq!(spirit_sword_accuracy(&s.magics), 8);
    }

    // ---- #942 特殊装备 Skill：技能经验 ×3 ----
    #[test]
    fn test_skill_special_exp_multiplier() {
        let mut s = make_state();
        use crate::actors::inventory::EquipmentSlot;
        use mir2_shared::data::item::UserItem;
        // 学习火球（C# 编号 31；gain_spell_exp 入参为 SharedRust +3 = 34）
        assert!(s.learn_magic(31));
        // 无 Skill 特殊：经验 100
        let _ = s.gain_spell_exp(34, 100, None);
        assert_eq!(s.magics.iter().find(|m| m.spell == 31).unwrap().experience, 100);
        // 装备 Skill 特殊装备：经验 ×3
        let mut info = mir2_shared::data::item::ItemInfo::default();
        info.unique = mir2_shared::enums::SpecialItemMode::SKILL;
        s.inventory.equipment[EquipmentSlot::Armour as usize] = Some(UserItem {
            info: Some(info),
            ..Default::default()
        });
        let _ = s.gain_spell_exp(34, 100, None);
        assert_eq!(s.magics.iter().find(|m| m.spell == 31).unwrap().experience, 400); // 100 + 300
    }

    // ---- #1230 技能经验对齐 C# LevelMagic：DB 阈值（need1/2/3）+ 玩家等级门控（Level1/2/3） ----
    #[test]
    fn test_gain_spell_exp_db_threshold_and_gate() {
        let mut s = make_state();
        assert!(s.learn_magic(31)); // FireBall C# 编号 31（SharedRust 34）
        let info = crate::db::MagicInfo {
            name: "FireBall".into(),
            spell: 31,
            base_cost: 2,
            level_cost: 1,
            icon: 0,
            level1: 7,
            level2: 12,
            level3: 17,
            need1: 100,
            need2: 300,
            need3: 600,
            delay_base: 1800,
            delay_reduction: 100,
            power_base: 5,
            power_bonus: 1,
            mpower_base: 5,
            mpower_bonus: 1,
            range: 9,
            multiplier_base: 1.0,
            multiplier_bonus: 0.0,
        };
        // 等级 1 < Level1(7)：等级门控拦截，不加经验
        assert!(s.gain_spell_exp(34, 500, Some(&info)).is_none());
        assert_eq!(s.magics.iter().find(|m| m.spell == 31).unwrap().experience, 0);
        // 升到 7 级后：need1=100，500 经验升级并结转 400（C# case 0）
        s.level = 7;
        let r = s.gain_spell_exp(34, 500, Some(&info)).unwrap();
        assert_eq!(r.1, 1);
        assert_eq!(s.magics.iter().find(|m| m.spell == 31).unwrap().experience, 400);
        // 1 级阈值 need2=300 且门控 Level2=12：当前等级 7 < 12 被拦
        assert!(s.gain_spell_exp(34, 200, Some(&info)).is_none());
        assert_eq!(s.magics.iter().find(|m| m.spell == 31).unwrap().level, 1);
        s.level = 12;
        // 已有结转 400 >= need2(300)，+200 → 升 2 级并结转 600-300=300（C# case 1）
        let r2 = s.gain_spell_exp(34, 200, Some(&info)).unwrap();
        assert_eq!(r2.1, 2);
        assert_eq!(s.magics.iter().find(|m| m.spell == 31).unwrap().experience, 300);
        // 2 级门控 Level3=17：当前等级 12 < 17 被拦（C# case 2 需 Level >= Level3）
        assert!(s.gain_spell_exp(34, 1000, Some(&info)).is_none());
    }

    // ---- #1246 破损装备特殊模式失效：Skill ×3 不生效 ----
    #[test]
    fn test_skill_special_exp_broken_item_no_multiplier() {
        let mut s = make_state();
        use crate::actors::inventory::EquipmentSlot;
        use mir2_shared::data::item::UserItem;
        assert!(s.learn_magic(31)); // FireBall C# 编号 31（SharedRust 34）
        let mut info = mir2_shared::data::item::ItemInfo::default();
        info.unique = mir2_shared::enums::SpecialItemMode::SKILL;
        info.durability = 1000;
        s.inventory.equipment[EquipmentSlot::Armour as usize] = Some(UserItem {
            info: Some(info),
            current_dura: 500,
            max_dura: 1000,
            ..Default::default()
        });
        let _ = s.gain_spell_exp(34, 100, None);
        assert_eq!(
            s.magics.iter().find(|m| m.spell == 31).unwrap().experience,
            300
        );
        // 破损：×3 失效（+100 → 400）
        s.inventory.equipment[EquipmentSlot::Armour as usize]
            .as_mut()
            .unwrap()
            .current_dura = 0;
        let _ = s.gain_spell_exp(34, 100, None);
        assert_eq!(
            s.magics.iter().find(|m| m.spell == 31).unwrap().experience,
            400
        );
    }

    // ---- #1305 导师技能经验双倍（C# LevelMagic MentorSkillBoost） ----
    #[test]
    fn test_mentor_skill_boost() {
        let mut s = make_state();
        assert!(s.learn_magic(31)); // FireBall C# 编号 31（SharedRust 34）
        let _ = s.gain_spell_exp(34, 100, None); // 非导师：无加成
        assert_eq!(
            s.magics.iter().find(|m| m.spell == 31).unwrap().experience,
            100
        );
        // 导师且徒弟同组近身（mentor_damage_bonus）：×2（+200 → 300）
        s.is_mentor = true;
        s.mentor_damage_bonus = true;
        let _ = s.gain_spell_exp(34, 100, None);
        assert_eq!(
            s.magics.iter().find(|m| m.spell == 31).unwrap().experience,
            300
        );
        // 徒弟不在近身（bonus=false）：无 ×2（+100 → 400）
        s.mentor_damage_bonus = false;
        let _ = s.gain_spell_exp(34, 100, None);
        assert_eq!(
            s.magics.iter().find(|m| m.spell == 31).unwrap().experience,
            400
        );
        // 有 Skill ×3 装备时：×3 优先，不叠加 ×2（+300 → 700）
        use crate::actors::inventory::EquipmentSlot;
        use mir2_shared::data::item::UserItem;
        let mut info = mir2_shared::data::item::ItemInfo::default();
        info.unique = mir2_shared::enums::SpecialItemMode::SKILL;
        info.durability = 1000;
        s.inventory.equipment[EquipmentSlot::Armour as usize] = Some(UserItem {
            info: Some(info),
            current_dura: 500,
            max_dura: 1000,
            ..Default::default()
        });
        s.mentor_damage_bonus = true;
        let _ = s.gain_spell_exp(34, 100, None);
        assert_eq!(
            s.magics.iter().find(|m| m.spell == 31).unwrap().experience,
            700
        );
    }

    // ---- #1290 药水池 tick 计算（C# ProcessRegen：min(池, PerTickRegen)） ----
    #[test]
    fn test_potion_tick_regen() {
        // 池 > per_tick：扣满 per_tick
        assert_eq!(potion_tick_regen(100, 8), (8, 92));
        // 池 < per_tick：取余清零
        assert_eq!(potion_tick_regen(5, 8), (5, 0));
        // 池 == per_tick
        assert_eq!(potion_tick_regen(8, 8), (8, 0));
        // 池 0
        assert_eq!(potion_tick_regen(0, 8), (0, 0));
    }

    /// #973：召唤护身符消耗（C# GetAmulet + ConsumeItem）
    #[test]
    fn test_consume_amulet_for_summon() {
        use crate::actors::inventory::EquipmentSlot;
        use mir2_shared::data::item::UserItem;

        let amulet = |shape: i16, count: u16| {
            let mut info = mir2_shared::data::item::ItemInfo::default();
            info.item_type = mir2_shared::enums::ItemType::Amulet;
            info.shape = shape;
            Some(UserItem { info: Some(info), count, ..Default::default() })
        };

        // 正常消耗：count 5 扣 2 → 剩 3
        let mut s = make_state();
        s.inventory.equipment[EquipmentSlot::Pendant as usize] = amulet(0, 5);
        assert!(s.consume_amulet_for_summon(2));
        assert_eq!(s.inventory.equipment[EquipmentSlot::Pendant as usize].as_ref().unwrap().count, 3);

        // 扣到 0 → 移除装备
        assert!(s.consume_amulet_for_summon(3));
        assert!(s.inventory.equipment[EquipmentSlot::Pendant as usize].is_none());

        // 数量不足 → 失败
        let mut s = make_state();
        s.inventory.equipment[EquipmentSlot::Pendant as usize] = amulet(0, 2);
        assert!(!s.consume_amulet_for_summon(3));

        // shape != 0 → 失败（C# GetAmulet(count, shape=0)）
        let mut s = make_state();
        s.inventory.equipment[EquipmentSlot::Pendant as usize] = amulet(1, 5);
        assert!(!s.consume_amulet_for_summon(1));

        // 非护身符 → 失败
        let mut s = make_state();
        let mut info = mir2_shared::data::item::ItemInfo::default();
        info.item_type = mir2_shared::enums::ItemType::Necklace;
        s.inventory.equipment[EquipmentSlot::Pendant as usize] = Some(UserItem {
            info: Some(info),
            count: 5,
            ..Default::default()
        });
        assert!(!s.consume_amulet_for_summon(1));

        // 未装备 → 失败
        let mut s = make_state();
        assert!(!s.consume_amulet_for_summon(1));
    }
    #[test]
    fn test_poison_amulet_detect_and_consume() {
        // #1453：C# GetPoison(1,1)=绿(shape1) / (1,2)=红(shape2)，Plague 消耗 1
        use crate::actors::inventory::EquipmentSlot;
        use mir2_shared::data::item::UserItem;

        let poison_amulet = |shape: i16, count: u16| {
            let mut info = mir2_shared::data::item::ItemInfo::default();
            info.item_type = mir2_shared::enums::ItemType::Amulet;
            info.shape = shape;
            Some(UserItem { info: Some(info), count, ..Default::default() })
        };

        // 绿毒护符：detect=1，消耗 1 后 count-1
        let mut s = make_state();
        s.inventory.equipment[EquipmentSlot::Pendant as usize] = poison_amulet(1, 3);
        assert_eq!(s.equipped_poison_shape(), 1);
        assert!(s.consume_poison_amulet(1));
        assert_eq!(s.inventory.equipment[EquipmentSlot::Pendant as usize].as_ref().unwrap().count, 2);

        // 红毒护符：detect=2；错误 shape 消耗失败
        let mut s = make_state();
        s.inventory.equipment[EquipmentSlot::Pendant as usize] = poison_amulet(2, 1);
        assert_eq!(s.equipped_poison_shape(), 2);
        assert!(!s.consume_poison_amulet(1));

        // 普通护符 shape0：detect=0；消耗 1 失败（GetPoison 要 shape 1/2）
        let mut s = make_state();
        s.inventory.equipment[EquipmentSlot::Pendant as usize] = poison_amulet(0, 5);
        assert_eq!(s.equipped_poison_shape(), 0);
        assert!(!s.consume_poison_amulet(1));

        // 未装备：detect=0
        let s = make_state();
        assert_eq!(s.equipped_poison_shape(), 0);
    }
}


