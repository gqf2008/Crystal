// SocialActor - 社交/组队/交易/好友/行会/婚姻/师徒
// 从 WorldActor 拆分出来，负责所有跨玩家社交逻辑

use std::collections::{HashMap, HashSet};

use kameo::actor::{Actor, ActorRef};
use kameo::message::Message;
use kameo::prelude::Context;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::actors::group::{Group, GroupMember};
use crate::actors::guild::{Guild, GuildRank};
use crate::actors::inventory::EquipmentSlot;
use crate::actors::player::{
    AddFriendToSelf, AddGold, AddItemToInventory, CanGainGold, DeductGold, GetItemInfo,
    GetPlayerState, PlayerActor, RemoveFriendFromSelf, RemoveItemFromInventory,
    SetAllowLoverRecall, SetAllowMarriage, SetAllowMentor, SetEnableGroupRecall, SetFriendMemo,
    SetGroupId, SetGuildInfo, SetLastRecallTime, SetMentor, SetMentorExp, SetPlayerPosition,
    SetPlayerState, SetSpouse,
};
use crate::actors::social_packets::*;
use crate::actors::trade::TradeSession;
use crate::actors::world::ai::{direction_towards, max_distance};
use crate::db::{self, DbPool};
use crate::gate::actor::{GateActor, SendToClient};
use crate::util::wire::build_packet_bytes;
use mir2_shared::enums::ServerPacketIds;

// ============================================================
// Message types (moved from WorldActor)
// ============================================================

/// #1329：WorldActor 启动后注入自身引用（SocialActor 查询地图标题 LoverUpdate.MapName 用）
pub struct SetWorldRef {
    pub world_ref: ActorRef<crate::actors::world::WorldActor>,
}

impl Message<SetWorldRef> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetWorldRef, _ctx: &mut Context<Self, Self::Reply>) {
        self.world_ref = Some(msg.world_ref);
    }
}

// --- Group ---

pub struct SwitchGroupRequest {
    pub session_id: u64,
    pub allow_group: bool,
}

pub struct GroupInviteRequest {
    pub session_id: u64,
    pub target_name: String,
}

pub struct GroupInviteReply {
    pub session_id: u64,
    pub inviter_id: u64,
    pub accept: bool,
}

pub struct DellMemberRequest {
    pub session_id: u64,
    pub member_name: String,
}

// --- Trade ---

pub struct TradeStartRequest {
    pub session_id: u64,
}

pub struct TradeStartReply {
    pub session_id: u64,
    pub accept: bool,
}

pub struct TradeAddGold {
    pub session_id: u64,
    pub amount: u32,
}

pub struct TradeConfirmLock {
    pub session_id: u64,
    pub locked: bool,
}

pub struct TradeCancel {
    pub session_id: u64,
}

pub struct TradeAddItem {
    pub session_id: u64,
    pub unique_id: u64,
    pub grid: u8,
    pub count: u16,
}

pub struct TradeRemoveItem {
    pub session_id: u64,
    pub unique_id: u64,
}

pub struct DepositTradeItemBySlot {
    pub session_id: u64,
    pub from_slot: i32,
    pub to_slot: i32,
}

pub struct RetrieveTradeItemBySlot {
    pub session_id: u64,
    pub from_slot: i32,
    pub to_slot: i32,
}

// --- Friend ---

pub struct AddFriendRequest {
    pub session_id: u64,
    pub friend_name: String,
    pub blocked: bool,
}

pub struct RemoveFriendRequest {
    pub session_id: u64,
    pub friend_object_id: u32,
}

pub struct RefreshFriendsRequest {
    pub session_id: u64,
}

pub struct AddMemoRequest {
    pub session_id: u64,
    pub friend_object_id: u32,
    pub memo: String,
}

// --- Guild ---

pub const GUILD_MAX_MEMBERS: usize = 200;

pub struct CreateGuildRequest {
    pub session_id: u64,
    pub guild_name: String,
}

pub struct GmCreateGuildRequest {
    pub session_id: u64,
    pub guild_name: String,
}

pub struct GuildInviteReply {
    pub session_id: u64,
    pub accept: bool,
}

pub struct RequestGuildInfo {
    pub session_id: u64,
    pub info_type: u8,
}

pub struct EditGuildMemberRequest {
    pub session_id: u64,
    pub change_type: u8,
    pub member_name: String,
    /// 职务索引（改名用，C# RankIndex）
    pub rank_index: u8,
    /// 新职务名（改名用，C# RankName）
    pub rank_name: String,
}

pub struct EditGuildNoticeRequest {
    pub session_id: u64,
    pub notice: Vec<String>,
}

pub struct LeaveGuildRequest {
    pub session_id: u64,
}

pub struct GuildStorageGoldChangeRequest {
    pub session_id: u64,
    pub change_type: u8,
    pub amount: u32,
}

pub struct GuildStorageItemChangeRequest {
    pub session_id: u64,
    pub change_type: u8,
    pub grid: u8,
    pub unique_id: u64,
    pub count: u32,
}

// --- Marriage ---

pub struct MarriageRequest {
    pub session_id: u64,
    pub target_name: String,
}

pub struct MarriageReply {
    pub session_id: u64,
    pub accept: bool,
}

pub struct SocialDivorceRequest {
    pub session_id: u64,
    pub partner_name: String,
}

pub struct SocialDivorceReply {
    pub session_id: u64,
    pub accept: bool,
}

pub struct SocialChangeMarriage {
    pub session_id: u64,
}

// --- Mentor ---

pub struct SocialAddMentor {
    pub session_id: u64,
    pub mentor_name: String,
}

pub struct SocialMentorReply {
    pub session_id: u64,
    pub accept: bool,
}

pub struct SocialAllowMentor {
    pub session_id: u64,
    pub allow: bool,
}

pub struct SocialCancelMentor {
    pub session_id: u64,
    /// C# MentorBreak(force)：true=手动解除（7 天冷却）；false=到期自动解除（无冷却，#2374）
    pub force: bool,
}

// --- Trade helper struct ---
// ============================================================
// WorldActor -> SocialActor: 玩家上线同步
// ============================================================

pub struct SocialPlayerJoined {
    pub session_id: u64,
    pub actor_ref: ActorRef<PlayerActor>,
    pub name: String,
}

/// WorldActor -> SocialActor: 玩家下线同步
pub struct SocialPlayerLeft {
    pub session_id: u64,
}

/// WorldActor(NPC 脚本) -> SocialActor: 查询玩家行会金币（对齐 C# CheckType.CheckGuildGold）
pub struct NpcGetGuildGold {
    pub session_id: u64,
}

impl Message<NpcGetGuildGold> for SocialActor {
    type Reply = u64;

    async fn handle(
        &mut self,
        msg: NpcGetGuildGold,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let state = match self.players.get(&msg.session_id) {
            Some(r) => match r.ask(GetPlayerState).await {
                Ok(Some(s)) => s,
                _ => return 0,
            },
            None => return 0,
        };
        let Some(guild_name) = state.guild_name else {
            return 0;
        };
        self.guilds.get(&guild_name).map(|g| g.gold).unwrap_or(0)
    }
}

/// WorldActor(NPC 脚本) -> SocialActor: 查询玩家行会职务权限（对齐 C# CheckType.CheckPermission）
pub struct NpcGetGuildMemberOptions {
    pub session_id: u64,
}

impl Message<NpcGetGuildMemberOptions> for SocialActor {
    type Reply = u8;

    async fn handle(
        &mut self,
        msg: NpcGetGuildMemberOptions,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let state = match self.players.get(&msg.session_id) {
            Some(r) => match r.ask(GetPlayerState).await {
                Ok(Some(s)) => s,
                _ => return 0,
            },
            None => return 0,
        };
        let Some(guild_name) = &state.guild_name else {
            return 0;
        };
        let Some(guild) = self.guilds.get(guild_name) else {
            return 0;
        };
        let Some(member) = guild.members.iter().find(|m| m.name == state.name) else {
            return 0;
        };
        member.rank.default_options()
    }
}

/// WorldActor -> SocialActor: 行会金币扣除（宣战费用 C# Guild_WarCost）
pub struct GuildDeductGold {
    pub guild_name: String,
    pub amount: u64,
}

impl Message<GuildDeductGold> for SocialActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        msg: GuildDeductGold,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let Some(guild) = self.guilds.get_mut(&msg.guild_name) else {
            return false;
        };
        if guild.gold < msg.amount {
            return false;
        }
        guild.gold -= msg.amount;
        self.save_guild_to_db(&msg.guild_name).await;
        true
    }
}

/// WorldActor -> SocialActor: 查询行会是否存在（宣战目标校验）
pub struct NpcGuildExists {
    pub guild_name: String,
}

impl Message<NpcGuildExists> for SocialActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        msg: NpcGuildExists,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.guilds.contains_key(&msg.guild_name)
    }
}

/// WorldActor -> SocialActor: 读取行会激活的 Buff 列表（C# GuildObject.BuffList）
pub struct NpcGetGuildBuffs {
    pub guild_name: String,
}

impl Message<NpcGetGuildBuffs> for SocialActor {
    type Reply = Vec<u32>;

    async fn handle(
        &mut self,
        msg: NpcGetGuildBuffs,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.guilds
            .get(&msg.guild_name)
            .map(|g| g.buffs.clone())
            .unwrap_or_default()
    }
}

/// WorldActor -> SocialActor: 读取行会领地旗标外观（C# GuildInfo.FlagImage/FlagColour）
pub struct NpcGetGuildFlagAppearance {
    pub guild_name: String,
}

impl Message<NpcGetGuildFlagAppearance> for SocialActor {
    type Reply = Option<(u16, i32)>;

    async fn handle(
        &mut self,
        msg: NpcGetGuildFlagAppearance,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.guilds
            .get(&msg.guild_name)
            .map(|g| (g.flag_image, g.flag_colour))
    }
}

/// WorldActor -> SocialActor: 设置行会领地旗标外观并持久化（C# @CHANGEFLAG/@CHANGEFLAGCOLOUR）
pub struct NpcSetGuildFlagAppearance {
    pub guild_name: String,
    pub flag_image: u16,
    pub flag_colour: i32,
}

impl Message<NpcSetGuildFlagAppearance> for SocialActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        msg: NpcSetGuildFlagAppearance,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let Some(guild) = self.guilds.get_mut(&msg.guild_name) else {
            return false;
        };
        guild.flag_image = msg.flag_image;
        guild.flag_colour = msg.flag_colour;
        self.save_guild_to_db(&msg.guild_name).await;
        true
    }
}

/// WorldActor -> SocialActor: 写入行会激活的 Buff 列表
pub struct NpcSetGuildBuffs {
    pub guild_name: String,
    pub buffs: Vec<u32>,
    /// #2571：随本次变更同步的时限（buff_id → unix 毫秒；None = 移除时限记录）。
    /// 激活时限 Buff 时置 Some，停用/到期时置 None；空表 = 不改动时限数据
    pub expiry_updates: Vec<(u32, Option<i64>)>,
}

impl Message<NpcSetGuildBuffs> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: NpcSetGuildBuffs, _ctx: &mut Context<Self, Self::Reply>) {
        if let Some(g) = self.guilds.get_mut(&msg.guild_name) {
            g.buffs = msg.buffs;
            for (buff_id, expire_at_ms) in msg.expiry_updates {
                match expire_at_ms {
                    Some(ms) => {
                        g.buff_expiries.insert(buff_id, ms);
                    }
                    None => {
                        g.buff_expiries.remove(&buff_id);
                    }
                }
            }
            self.save_guild_to_db(&msg.guild_name).await;
        }
    }
}

/// 行会战争镜像增删（#2138）：WorldActor 为准，SocialActor 仅存镜像。
/// at_war=true 双向添加；false 双向移除并清理空集合（C# IsAtWar = WarringGuilds.Count > 0）。
fn apply_guild_war_mirror(
    guild_wars: &mut HashMap<String, HashSet<String>>,
    guild_name: &str,
    other: &str,
    at_war: bool,
) {
    if at_war {
        guild_wars
            .entry(guild_name.to_string())
            .or_default()
            .insert(other.to_string());
        guild_wars
            .entry(other.to_string())
            .or_default()
            .insert(guild_name.to_string());
        return;
    }
    let remove_first = {
        let mut empty = false;
        if let Some(set) = guild_wars.get_mut(guild_name) {
            set.remove(other);
            empty = set.is_empty();
        }
        empty
    };
    if remove_first {
        guild_wars.remove(guild_name);
    }
    let remove_second = {
        let mut empty = false;
        if let Some(set) = guild_wars.get_mut(other) {
            set.remove(guild_name);
            empty = set.is_empty();
        }
        empty
    };
    if remove_second {
        guild_wars.remove(other);
    }
}

/// WorldActor -> SocialActor: 行会战争状态镜像（#2138，宣战/停战双向增删）
pub struct NpcSetGuildWar {
    pub guild_name: String,
    pub other: String,
    pub at_war: bool,
}

impl Message<NpcSetGuildWar> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: NpcSetGuildWar, _ctx: &mut Context<Self, Self::Reply>) {
        apply_guild_war_mirror(
            &mut self.guild_wars,
            &msg.guild_name,
            &msg.other,
            msg.at_war,
        );
    }
}

/// WorldActor -> SocialActor: 行会获得经验（C# GuildObject.GainExp）
pub struct GuildGainExp {
    pub guild_name: String,
    pub amount: i64,
}

impl Message<GuildGainExp> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: GuildGainExp, _ctx: &mut Context<Self, Self::Reply>) {
        // C# GuildObject.GainExp（644-690）：expAmount = amount * Guild_ExpRate；0 则忽略
        let exp_amount = (msg.amount as f64 * self.config.guild_exp_rate).floor() as u32;
        if exp_amount == 0 {
            return;
        }
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let (leveled, broadcast_gain, guild_level, guild_exp, guild_cap) = {
            let Some(guild) = self.guilds.get_mut(&msg.guild_name) else {
                return;
            };
            let leveled = guild.apply_gain_exp(
                msg.amount,
                self.config.guild_exp_rate,
                self.config.guild_point_per_level,
                &self.config.guild_experience_list,
                &self.config.guild_membercap_list,
            );
            // C#：升级/广播后 NextExpUpdate = now + 10000（节流 GuildExpGain）
            let broadcast_gain = if leveled {
                false
            } else if now_ms >= guild.next_exp_update {
                guild.next_exp_update = now_ms + 10_000;
                true
            } else {
                false
            };
            (
                leveled,
                broadcast_gain,
                guild.level,
                guild.experience,
                guild.member_cap,
            )
        };
        let sessions: Vec<u64> = {
            let Some(guild) = self.guilds.get(&msg.guild_name) else {
                return;
            };
            guild.members.iter().filter_map(|m| m.session_id).collect()
        };
        if leveled {
            debug!(
                "Guild '{}' leveled to {} (exp={} cap={})",
                msg.guild_name, guild_level, guild_exp, guild_cap
            );
            // C# 升级：广播 GuildStatus 给在线成员
            for sid in &sessions {
                send_guild_status_packet(&self.gate_ref, *sid, true);
            }
            self.save_guild_to_db(&msg.guild_name).await;
        } else if broadcast_gain {
            // #1344：C# 非升级 → 每 10s 广播 S.GuildExpGain{Amount=expAmount} 给在线成员
            let mut body = Vec::new();
            body.extend_from_slice(&exp_amount.to_le_bytes());
            let data = build_packet_bytes(ServerPacketIds::GuildExpGain as i16, &body);
            for sid in sessions {
                let _ = self
                    .gate_ref
                    .tell(SendToClient {
                        session_id: sid,
                        data: data.clone(),
                    })
                    .try_send();
            }
            debug!(
                "Guild '{}' gained {} exp (GuildExpGain broadcast)",
                msg.guild_name, exp_amount
            );
        }
    }
}

/// WorldActor(NPC 脚本) -> SocialActor: 查询玩家是否队长（对齐 C# CheckType.Groupleader）
pub struct NpcIsGroupLeader {
    pub session_id: u64,
}

impl Message<NpcIsGroupLeader> for SocialActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        msg: NpcIsGroupLeader,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let state = match self.players.get(&msg.session_id) {
            Some(r) => match r.ask(GetPlayerState).await {
                Ok(Some(s)) => s,
                _ => return false,
            },
            None => return false,
        };
        let Some(group_id) = state.group_id else {
            return false;
        };
        match self.groups.get(&group_id) {
            Some(g) => g.leader_session() == Some(msg.session_id),
            None => false,
        }
    }
}

/// WorldActor -> SocialActor: 查询是否允许登录（C# Settings.AllowLogin）
pub struct NpcGetAllowLogin;

impl Message<NpcGetAllowLogin> for SocialActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        _msg: NpcGetAllowLogin,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.config.allow_login
    }
}

/// WorldActor -> SocialActor: 查询是否允许注册新账号（C# Settings.AllowNewAccount）
pub struct NpcGetAllowNewAccount;

impl Message<NpcGetAllowNewAccount> for SocialActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        _msg: NpcGetAllowNewAccount,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.config.allow_new_account
    }
}

/// WorldActor -> SocialActor: 查询是否允许修改密码（C# Settings.AllowChangePassword）
pub struct NpcGetAllowChangePassword;

impl Message<NpcGetAllowChangePassword> for SocialActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        _msg: NpcGetAllowChangePassword,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.config.allow_change_password
    }
}

/// WorldActor -> SocialActor: 查询是否允许进入游戏（C# Settings.AllowStartGame）
pub struct NpcGetAllowStartGame;

impl Message<NpcGetAllowStartGame> for SocialActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        _msg: NpcGetAllowStartGame,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.config.allow_start_game
    }
}

/// WorldActor -> SocialActor: 查询邮件寄送费率（C# Settings.MailCostPer1KGold / MailItemInsurancePercentage / MailFreeWithStamp）
pub struct NpcGetMailSettings;

impl Message<NpcGetMailSettings> for SocialActor {
    type Reply = (u32, u32, bool, u32, bool, bool);

    async fn handle(
        &mut self,
        _msg: NpcGetMailSettings,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        (
            self.config.mail_cost_per_1k_gold,
            self.config.mail_item_insurance_percentage,
            self.config.mail_free_with_stamp,
            self.config.mail_capacity,
            self.config.mail_auto_send_gold,
            self.config.mail_auto_send_items,
        )
    }
}

/// WorldActor -> SocialActor: 查询英雄创建选项（C# Settings.AllowNewHero / Hero_CanCreateClass / Hero_RequiredLevel）
pub struct NpcGetHeroCreateOptions;

impl Message<NpcGetHeroCreateOptions> for SocialActor {
    type Reply = (bool, Vec<bool>, u8);

    async fn handle(
        &mut self,
        _msg: NpcGetHeroCreateOptions,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        (
            self.config.allow_new_hero,
            self.config.hero_can_create_class.clone(),
            self.config.hero_required_level,
        )
    }
}

/// WorldActor -> SocialActor: 查询允许创建职业（C# Settings.AllowCreateAssassin/AllowCreateArcher）
pub struct NpcGetCreateClassOptions;

impl Message<NpcGetCreateClassOptions> for SocialActor {
    type Reply = (bool, bool);

    async fn handle(
        &mut self,
        _msg: NpcGetCreateClassOptions,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        (
            self.config.allow_create_assassin,
            self.config.allow_create_archer,
        )
    }
}

/// WorldActor -> SocialActor: 查询是否允许删除角色（C# Settings.AllowDeleteCharacter）
pub struct NpcGetAllowDeleteCharacter;

impl Message<NpcGetAllowDeleteCharacter> for SocialActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        _msg: NpcGetAllowDeleteCharacter,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.config.allow_delete_character
    }
}

/// WorldActor -> SocialActor: 查询是否允许创建角色（C# Settings.AllowNewCharacter）
pub struct NpcGetAllowNewCharacter;

impl Message<NpcGetAllowNewCharacter> for SocialActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        _msg: NpcGetAllowNewCharacter,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.config.allow_new_character
    }
}

/// WorldActor(NPC 脚本) -> SocialActor: 查询行会宣战费用/时长（C# Settings.Guild_WarCost/Guild_WarTime，<$GUILDWARFEE>/<$GUILDWARTIME>）
/// 查询新手行会配置（名称/开关/加成%）
pub struct NpcGetNewbieGuildConfig;

impl Message<NpcGetNewbieGuildConfig> for SocialActor {
    type Reply = (String, bool, i32);

    async fn handle(
        &mut self,
        _msg: NpcGetNewbieGuildConfig,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        (
            self.config.newbie_guild.clone(),
            self.config.newbie_guild_buff_enabled,
            self.config.newbie_guild_exp_buff,
        )
    }
}

pub struct NpcGetGuildWarSettings;

impl Message<NpcGetGuildWarSettings> for SocialActor {
    type Reply = (u32, i64);

    async fn handle(
        &mut self,
        _msg: NpcGetGuildWarSettings,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        (self.config.guild_war_cost, self.config.guild_war_time)
    }
}

/// WorldActor(NPC 脚本) -> SocialActor: NPC 直接给/扣行会金币（对齐 C# ActionType.GiveGuildGold/TakeGuildGold）
pub struct NpcGuildGoldChange {
    pub session_id: u64,
    pub amount: u32,
    /// 2=减少（TakeGuildGold），3=增加（GiveGuildGold），对齐 C# S.GuildStorageGoldChange Type
    pub change_type: u8,
}

impl Message<NpcGuildGoldChange> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: NpcGuildGoldChange, _ctx: &mut Context<Self, Self::Reply>) {
        let state = match self.players.get(&msg.session_id) {
            Some(r) => match r.ask(GetPlayerState).await {
                Ok(Some(s)) => s,
                _ => return,
            },
            None => return,
        };
        let Some(guild_name) = state.guild_name.clone() else {
            return;
        };
        let Some(guild) = self.guilds.get_mut(&guild_name) else {
            return;
        };
        match msg.change_type {
            2 => {
                let amount = (msg.amount as u64).min(guild.gold);
                guild.gold -= amount;
                self.send_guild_storage_gold_change(msg.session_id, &state.name, amount as u32, 2)
                    .await;
                send_system_message(
                    &self.gate_ref,
                    msg.session_id,
                    &format!("行会仓库减少 {} 金币", amount),
                );
            }
            _ => {
                // C# GiveGuildGold：行会金币上限 uint.MaxValue
                let add = (msg.amount as u64)
                    .min((u32::MAX as u64).saturating_sub(guild.gold.min(u32::MAX as u64)));
                guild.gold += add;
                self.send_guild_storage_gold_change(msg.session_id, &state.name, add as u32, 3)
                    .await;
                send_system_message(
                    &self.gate_ref,
                    msg.session_id,
                    &format!("行会仓库增加 {} 金币", add),
                );
            }
        }
        self.save_guild_to_db(&guild_name).await;
        self.broadcast_guild_info(&guild_name).await;
        debug!(
            "NPC GuildGoldChange: {} {} (type {})",
            state.name, msg.amount, msg.change_type
        );
    }
}

/// WorldActor -> SocialActor: 查询玩家行会等级与剩余点数（C# GuildInfo.Level/SparePoints）
pub struct NpcGetGuildLevelSparePoints {
    pub session_id: u64,
}

impl Message<NpcGetGuildLevelSparePoints> for SocialActor {
    type Reply = (u8, u8);

    async fn handle(
        &mut self,
        msg: NpcGetGuildLevelSparePoints,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let state = match self.players.get(&msg.session_id) {
            Some(r) => match r.ask(GetPlayerState).await {
                Ok(Some(s)) => s,
                _ => return (0, 0),
            },
            None => return (0, 0),
        };
        let Some(guild_name) = state.guild_name else {
            return (0, 0);
        };
        self.guilds
            .get(&guild_name)
            .map(|g| (g.level, g.spare_points))
            .unwrap_or((0, 0))
    }
}

/// WorldActor -> SocialActor: 行会 Buff 购买扣费（C# AddBuff/ChargeForBuff：SparePoints -= PointsRequirement；Gold -= ActivationCost）
pub struct NpcGuildBuffCharge {
    pub session_id: u64,
    pub points: u32,
    pub gold: u32,
}

impl Message<NpcGuildBuffCharge> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: NpcGuildBuffCharge, _ctx: &mut Context<Self, Self::Reply>) {
        let state = match self.players.get(&msg.session_id) {
            Some(r) => match r.ask(GetPlayerState).await {
                Ok(Some(s)) => s,
                _ => return,
            },
            None => return,
        };
        let Some(guild_name) = state.guild_name.clone() else {
            return;
        };
        let Some(guild) = self.guilds.get_mut(&guild_name) else {
            return;
        };
        if msg.points > 0 {
            guild.spare_points = guild.spare_points.saturating_sub(msg.points.min(255) as u8);
        }
        if msg.gold > 0 {
            let amount = (msg.gold as u64).min(guild.gold);
            guild.gold -= amount;
            self.send_guild_storage_gold_change(msg.session_id, &state.name, amount as u32, 2)
                .await;
        }
        self.save_guild_to_db(&guild_name).await;
        self.broadcast_guild_info(&guild_name).await;
        debug!(
            "NPC GuildBuffCharge: {} points={} gold={}",
            guild_name, msg.points, msg.gold
        );
    }
}

/// WorldActor -> SocialActor: 给指定行会增加金币（C# PurchaseGuildTerritory 卖家收款，:10502）
pub struct NpcGuildGoldGive {
    pub guild_name: String,
    pub amount: u32,
}

impl Message<NpcGuildGoldGive> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: NpcGuildGoldGive, _ctx: &mut Context<Self, Self::Reply>) {
        let Some(guild) = self.guilds.get_mut(&msg.guild_name) else {
            return;
        };
        let add = (msg.amount as u64)
            .min((u32::MAX as u64).saturating_sub(guild.gold.min(u32::MAX as u64)));
        guild.gold += add;
        self.save_guild_to_db(&msg.guild_name).await;
        self.broadcast_guild_info(&msg.guild_name).await;
        debug!("NPC GuildGoldGive: {} +{}", msg.guild_name, add);
    }
}

/// WorldActor(NPC 脚本) -> SocialActor: 强制离婚（对齐 C# ActionType.ForceDivorce：NPCDivorce）
pub struct NpcForceDivorce {
    pub session_id: u64,
}

impl Message<NpcForceDivorce> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: NpcForceDivorce, _ctx: &mut Context<Self, Self::Reply>) {
        let state = match self.players.get(&msg.session_id) {
            Some(r) => match r.ask(GetPlayerState).await {
                Ok(Some(s)) => s,
                _ => return,
            },
            None => return,
        };
        let Some(spouse_name) = state.spouse_name.clone() else {
            send_system_message(&self.gate_ref, msg.session_id, "你还没有结婚");
            return;
        };
        // 清除自己婚姻状态
        if let Some(record) = self.players.get(&msg.session_id) {
            let _ = record
                .ask(SetSpouse {
                    spouse_name: None,
                    married_date: 0,
                })
                .await;
        }
        // 配偶在线则同步清除
        let online: Vec<u64> = self.players.keys().copied().collect();
        for sid in online {
            if sid == msg.session_id {
                continue;
            }
            if let Some(record) = self.players.get(&sid) {
                if let Ok(Some(os)) = record.ask(GetPlayerState).await {
                    if os.name.eq_ignore_ascii_case(&spouse_name) {
                        let _ = record
                            .ask(SetSpouse {
                                spouse_name: None,
                                married_date: 0,
                            })
                            .await;
                        send_system_message(
                            &self.gate_ref,
                            sid,
                            &format!("你已与 {} 强制离婚", state.name),
                        );
                        send_lover_update_packet(&self.gate_ref, sid, "", 0, "", 0);
                        break;
                    }
                }
            }
        }
        send_system_message(
            &self.gate_ref,
            msg.session_id,
            &format!("你已与 {} 强制离婚", spouse_name),
        );
        send_lover_update_packet(&self.gate_ref, msg.session_id, "", 0, "", 0);
        debug!("NPC ForceDivorce: {} <- {}", state.name, spouse_name);
    }
}

/// WorldActor(NPC 脚本) -> SocialActor: NPC 直接加入行会（对齐 C# ActionType.AddToGuild：自动接受邀请）
pub struct NpcAddToGuild {
    pub session_id: u64,
    pub guild_name: String,
}

impl Message<NpcAddToGuild> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: NpcAddToGuild, _ctx: &mut Context<Self, Self::Reply>) {
        let guild_name = msg.guild_name.clone();
        let state = match self.players.get(&msg.session_id) {
            Some(r) => match r.ask(GetPlayerState).await {
                Ok(Some(s)) => s,
                _ => return,
            },
            None => return,
        };
        // 已有行会则忽略（对齐 C# player.MyGuild != null return）
        if state.guild_name.is_some() {
            send_system_message(&self.gate_ref, msg.session_id, "你已经有行会了");
            return;
        }
        let full = {
            let Some(guild) = self.guilds.get_mut(&guild_name) else {
                send_system_message(
                    &self.gate_ref,
                    msg.session_id,
                    &format!("行会 \"{}\" 不存在", guild_name),
                );
                return;
            };
            if guild.member_count() >= GUILD_MAX_MEMBERS {
                true
            } else {
                guild.add_member(state.name.clone(), Some(msg.session_id));
                false
            }
        };
        if full {
            send_system_message(&self.gate_ref, msg.session_id, "行会已满");
            return;
        }
        if let Some(record) = self.players.get(&msg.session_id) {
            let _ = record
                .ask(SetGuildInfo {
                    guild_name: Some(guild_name.clone()),
                    rank: GuildRank::Member,
                })
                .await;
        }
        send_guild_status_packet(&self.gate_ref, msg.session_id, true);
        if let Some(guild) = self.guilds.get(&guild_name) {
            send_guild_info_packet(&self.gate_ref, msg.session_id, guild);
        }
        send_system_message(
            &self.gate_ref,
            msg.session_id,
            &format!("你已加入行会 \"{}\"", guild_name),
        );
        // #1374：C# BroadcastInfo——加入行会后重发外观（同图玩家看到行会名/职位）
        if let Some(record) = self.players.get(&msg.session_id) {
            if let Ok(Some(fresh)) = record.ask(GetPlayerState).await {
                self.broadcast_ride_appearance(msg.session_id, &fresh).await;
            }
        }
        debug!("NPC AddToGuild: {} -> {}", state.name, guild_name);
    }
}

/// WorldActor(NPC 脚本) -> SocialActor: NPC 移除行会成员（对齐 C# ActionType.RemoveFromGuild）
pub struct NpcRemoveFromGuild {
    pub session_id: u64,
}

impl Message<NpcRemoveFromGuild> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: NpcRemoveFromGuild, _ctx: &mut Context<Self, Self::Reply>) {
        let state = match self.players.get(&msg.session_id) {
            Some(r) => match r.ask(GetPlayerState).await {
                Ok(Some(s)) => s,
                _ => return,
            },
            None => return,
        };
        let Some(guild_name) = state.guild_name.clone() else {
            send_system_message(&self.gate_ref, msg.session_id, "你不在任何行会中");
            return;
        };
        let removed = {
            let Some(guild) = self.guilds.get_mut(&guild_name) else {
                return;
            };
            guild.remove_member(&state.name)
        };
        if removed {
            if let Some(record) = self.players.get(&msg.session_id) {
                let _ = record
                    .ask(SetGuildInfo {
                        guild_name: None,
                        rank: GuildRank::Member,
                    })
                    .await;
            }
            send_guild_status_packet(&self.gate_ref, msg.session_id, false);
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                &format!("你已离开行会 \"{}\"", guild_name),
            );
            // #1374：C# BroadcastInfo——退出行会后重发外观（行会名清除）
            if let Some(record) = self.players.get(&msg.session_id) {
                if let Ok(Some(fresh)) = record.ask(GetPlayerState).await {
                    self.broadcast_ride_appearance(msg.session_id, &fresh).await;
                }
            }
            debug!("NPC RemoveFromGuild: {} <- {}", state.name, guild_name);
        }
    }
}

/// WorldActor(NPC 脚本) -> SocialActor: 组队召回（对齐 C# ActionType.GroupRecall）
/// NPC 版无冷却/套装/死亡检查限制，直接召回所有在线组员到该玩家位置
pub struct NpcGroupRecall {
    pub session_id: u64,
}

impl Message<NpcGroupRecall> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: NpcGroupRecall, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        let Some(group_id) = state.group_id else {
            return;
        };
        let Some(group) = self.groups.get(&group_id).cloned() else {
            return;
        };

        let target_map = state.map_index;
        let target_x = state.x;
        let target_y = state.y;
        for member in &group.members {
            if member.session_id == msg.session_id {
                continue;
            }
            if !member.online {
                continue;
            }
            if let Some(mem_record) = self.players.get(&member.session_id) {
                if let Ok(Some(mem_state)) = mem_record.ask(GetPlayerState).await {
                    let _ = mem_record
                        .ask(SetPlayerPosition {
                            x: target_x,
                            y: target_y,
                            direction: mem_state.direction,
                            map_index: Some(target_map),
                            is_mounted: None,
                        })
                        .await;
                    let mut body = Vec::new();
                    body.extend_from_slice(&target_x.to_le_bytes());
                    body.extend_from_slice(&target_y.to_le_bytes());
                    body.push(mem_state.direction);
                    let _ = self
                        .gate_ref
                        .tell(SendToClient {
                            session_id: member.session_id,
                            data: build_packet_bytes(ServerPacketIds::UserLocation as i16, &body),
                        })
                        .await;
                    debug!(
                        "NPC GROUPRECALL: {} recalled to ({},{}) map {}",
                        mem_state.name, target_x, target_y, target_map
                    );
                }
            }
        }
    }
}

/// WorldActor -> SocialActor: 聊天命令转发（社交类）
pub struct SocialChatCommand {
    pub session_id: u64,
    pub command: String,
    pub args: Vec<String>,
}

/// SocialActor 配置
pub struct SocialActorConfig {
    pub map_infos: Arc<RwLock<HashMap<i32, db::MapInfo>>>,
    pub item_infos: Arc<RwLock<HashMap<i32, db::ItemInfo>>>,
    /// 创建行会所需金币 (来自 cfg.server.toml 的 [social] section)
    /// 替代之前 hardcoded 1_000_000 常量。
    pub guild_creation_cost_gold: u64,
    /// 是否启用配偶（结婚戒指）召回（C# Settings.WeddingRingRecall）
    pub wedding_ring_recall_enabled: bool,
    /// 创建行会所需等级（C# Settings.Guild_RequiredLevel）
    pub guild_required_level: u16,
    /// 新手行会名称（C# Settings.NewbieGuild）
    pub newbie_guild: String,
    /// 新手行会经验 buff 开关（C# Settings.NewbieGuildBuffEnabled = true）
    pub newbie_guild_buff_enabled: bool,
    /// 新手行会经验加成 %（C# Settings.NewbieGuildExpBuff = 5）
    pub newbie_guild_exp_buff: i32,
    /// 行会经验倍率（C# Settings.Guild_ExpRate = 0.01）
    pub guild_exp_rate: f64,
    /// 行会每级分配点数（C# Settings.Guild_PointPerLevel = 0）
    pub guild_point_per_level: u8,
    /// 行会各级所需经验（C# Settings.Guild_ExperienceList，索引=等级）
    pub guild_experience_list: Vec<i64>,
    /// 行会各级成员上限（C# Settings.Guild_MembercapList，索引=等级）
    pub guild_membercap_list: Vec<i32>,
    /// 行会 Buff 定义（C# GuildBuffInfo：Configs/GuildSettings.ini [Buff-*]，TotalBuffs=16）
    pub guild_buff_infos: Vec<crate::util::ini::GuildBuffInfo>,
    /// 是否允许创建角色（C# Settings.AllowNewCharacter）
    pub allow_new_character: bool,
    /// 是否允许删除角色（C# Settings.AllowDeleteCharacter）
    pub allow_delete_character: bool,
    /// 是否允许创建刺客（C# Settings.AllowCreateAssassin）
    pub allow_create_assassin: bool,
    /// 是否允许创建弓箭手（C# Settings.AllowCreateArcher）
    pub allow_create_archer: bool,
    /// 是否允许创建英雄（C# Settings.AllowNewHero）
    pub allow_new_hero: bool,
    /// 英雄可创建职业（C# Settings.Hero_CanCreateClass[5]）
    pub hero_can_create_class: Vec<bool>,
    /// 创建英雄所需等级（C# Settings.Hero_RequiredLevel = 22）
    pub hero_required_level: u8,
    /// 邮件寄金币费用（每 1000 金币，C# Settings.MailCostPer1KGold）
    pub mail_cost_per_1k_gold: u32,
    /// 邮件寄物品保险百分比（C# Settings.MailItemInsurancePercentage）
    pub mail_item_insurance_percentage: u32,
    /// 邮票免费寄信（C# Settings.MailFreeWithStamp）
    pub mail_free_with_stamp: bool,
    /// 收件箱容量上限（C# Settings.MailCapacity = 100）
    pub mail_capacity: u32,
    /// 包裹金币自动收取（C# Settings.MailAutoSendGold）
    pub mail_auto_send_gold: bool,
    /// 包裹物品自动收取（C# Settings.MailAutoSendItems）
    pub mail_auto_send_items: bool,
    /// 是否允许进入游戏（C# Settings.AllowStartGame）
    pub allow_start_game: bool,
    /// 是否允许修改密码（C# Settings.AllowChangePassword）
    pub allow_change_password: bool,
    /// 是否允许注册新账号（C# Settings.AllowNewAccount）
    pub allow_new_account: bool,
    /// 是否允许登录（C# Settings.AllowLogin）
    pub allow_login: bool,
    /// 行会宣战费用（C# Settings.Guild_WarCost = 3000）
    pub guild_war_cost: u32,
    /// 行会战争时长（秒，C# Settings.Guild_WarTime = 180）
    pub guild_war_time: i64,
    /// 离婚后再次结婚等待天数（C# Settings.MarriageCooldown = 7）
    pub marriage_cooldown_days: i64,
    /// 结婚最低等级（C# Settings.MarriageLevelRequired = 10）
    pub marriage_level_required: u16,
    /// 师徒等级差下限（C# Settings.MentorLevelGap = 10）
    pub mentor_level_gap: u8,
    /// 师徒期限（天，C# Settings.MentorLength = 7）
    pub mentor_length_days: u8,
    /// 建会消耗列表（C# Guild_CreationCostList：[Required-i]；空 = 回退金币 guild_creation_cost_gold）
    pub guild_creation_costs: Vec<crate::util::ini::GuildCreationCost>,
    /// 组队邀请冷却（C# Settings.GroupInviteDelay = 2000ms，#2420 Setup.ini）
    pub group_invite_delay_ms: i64,
    /// 交易邀请冷却（C# Settings.TradeDelay = 2000ms，#2420 Setup.ini）
    pub trade_delay_ms: i64,
}

impl Default for SocialActorConfig {
    fn default() -> Self {
        Self {
            map_infos: Arc::new(RwLock::new(HashMap::new())),
            item_infos: Arc::new(RwLock::new(HashMap::new())),
            // default 与主流程 cfg 路径一致
            guild_creation_cost_gold: 1_000_000,
            wedding_ring_recall_enabled: true,
            guild_required_level: 22,
            newbie_guild: "NewbieGuild".to_string(),
            newbie_guild_buff_enabled: true,
            newbie_guild_exp_buff: 5,
            guild_exp_rate: 0.01,
            guild_point_per_level: 0,
            guild_experience_list: Vec::new(),
            guild_membercap_list: Vec::new(),
            guild_buff_infos: Vec::new(),
            allow_new_character: true,
            allow_delete_character: true,
            allow_create_assassin: true,
            allow_create_archer: true,
            allow_new_hero: true,
            hero_can_create_class: vec![true; 5],
            hero_required_level: 22,
            mail_cost_per_1k_gold: 100,
            mail_item_insurance_percentage: 5,
            mail_free_with_stamp: true,
            mail_capacity: 100,
            mail_auto_send_gold: false,
            mail_auto_send_items: false,
            allow_start_game: true,
            allow_change_password: true,
            allow_new_account: true,
            allow_login: true,
            guild_war_cost: 3000,
            guild_war_time: 180,
            marriage_cooldown_days: 7,
            marriage_level_required: 10,
            mentor_level_gap: 10,
            mentor_length_days: 7,
            guild_creation_costs: Vec::new(),
            group_invite_delay_ms: 2000,
            trade_delay_ms: 2000,
        }
    }
}

/// SocialActor 启动参数
pub struct SocialActorArgs {
    pub gate_ref: ActorRef<GateActor>,
    pub db_pool: DbPool,
    pub config: SocialActorConfig,
}

impl Actor for SocialActor {
    type Args = SocialActorArgs;
    type Error = anyhow::Error;

    async fn on_start(
        args: SocialActorArgs,
        _actor_ref: ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        // Load guilds from DB
        let guilds = match db::load_guilds(&args.db_pool).await {
            Ok(g) => {
                info!("SocialActor: loaded {} guilds from database", g.len());
                g
            }
            Err(e) => {
                warn!("SocialActor: failed to load guilds from DB: {}", e);
                HashMap::new()
            }
        };

        Ok(Self {
            players: HashMap::new(),
            groups: HashMap::new(),
            next_group_id: 1,
            pending_invites: HashMap::new(),
            active_trades: HashMap::new(),
            last_trade_request: HashMap::new(),
            last_group_invite: HashMap::new(),
            guilds,
            pending_guild_invites: HashMap::new(),
            guild_wars: HashMap::new(),
            pending_marriage_invites: HashMap::new(),
            pending_mentor_invites: HashMap::new(),
            gate_ref: args.gate_ref,
            world_ref: None,
            db_pool: args.db_pool,
            config: args.config,
        })
    }
}

/// SocialActor 状态
pub struct SocialActor {
    /// 在线玩家镜像（session_id -> ActorRef）
    players: HashMap<u64, ActorRef<PlayerActor>>,

    // === 组队状态 ===
    groups: HashMap<u64, Group>,
    next_group_id: u64,
    pending_invites: HashMap<u64, u64>,

    // === 交易状态 ===
    active_trades: HashMap<u64, TradeSession>,
    /// #919：交易邀请冷却（C# Settings.TradeDelay=2000ms，session -> 上次时间戳 ms）
    last_trade_request: HashMap<u64, i64>,
    /// #919：组队邀请冷却（C# Settings.GroupInviteDelay=2000ms，session -> 上次时间戳 ms）
    last_group_invite: HashMap<u64, i64>,

    // === 行会状态 ===
    guilds: HashMap<String, Guild>,
    pending_guild_invites: HashMap<u64, (u64, String)>,
    /// 行会战争镜像（guild_name -> 敌对行会集合；WorldActor 为准，#2138）
    guild_wars: HashMap<String, HashSet<String>>,

    // === 婚姻状态 ===
    pending_marriage_invites: HashMap<u64, u64>,

    // === 师徒状态 ===
    pending_mentor_invites: HashMap<u64, u64>,

    // === 依赖 ===
    gate_ref: ActorRef<GateActor>,
    /// #1329：WorldActor 引用（LoverUpdate 地图标题查询；WorldActor on_start 注入）
    world_ref: Option<ActorRef<crate::actors::world::WorldActor>>,
    db_pool: DbPool,
    config: SocialActorConfig,
}

/// C# Functions.FacingEachOther：双方朝向彼此
fn facing_each_other(dir_a: u8, ax: i32, ay: i32, dir_b: u8, bx: i32, by: i32) -> bool {
    dir_a == direction_towards(ax, ay, bx, by) && dir_b == direction_towards(bx, by, ax, ay)
}

/// C# Functions.PointMove(location, direction, 1)：前方一格
fn front_tile(x: i32, y: i32, dir: u8) -> (i32, i32) {
    let d = (dir % 8) as usize;
    (x + DIR_DX[d], y + DIR_DY[d])
}

// #2394：婚姻配置改用 SocialConfig（C# Settings.MarriageCooldown/MarriageLevelRequired）

/// C# MarriageRequest 目标侧校验输入（PlayerObject.cs:13174-13226 所需字段）
struct MarriageTargetCtx<'a> {
    requester_map: u16,
    requester_x: i32,
    requester_y: i32,
    requester_dir: u8,
    requester_dead: bool,
    target_name: &'a str,
    target_map: u16,
    target_x: i32,
    target_y: i32,
    target_dir: u8,
    target_level: u16,
    target_married_date: i64,
    target_allow_marriage: bool,
    target_dead: bool,
    target_spouse: bool,
    target_has_pending: bool,
}

/// C# MarriageRequest 目标侧校验（PlayerObject.cs:13174-13226）
/// 返回 None = 全部通过；Some(拒绝原因) = 校验失败（消息发给发起方）
fn marriage_target_check(
    ctx: &MarriageTargetCtx<'_>,
    now_unix: i64,
    marriage_level_required: u16,
    marriage_cooldown_days: i64,
) -> Option<String> {
    // C# :13174 双方面对面（FacingEachOther）
    if !facing_each_other(
        ctx.requester_dir,
        ctx.requester_x,
        ctx.requester_y,
        ctx.target_dir,
        ctx.target_x,
        ctx.target_y,
    ) {
        return Some("结婚需要双方面对面".to_string());
    }
    // C# :13180 目标等级（MarriageLevelRequired=10）
    if ctx.target_level < marriage_level_required {
        return Some(format!(
            "{} 需要达到 {} 级才能结婚",
            ctx.target_name, marriage_level_required
        ));
    }
    // C# :13186 目标离婚冷却（MarriageCooldown=7 天）
    if ctx.target_married_date > 0
        && now_unix < ctx.target_married_date + marriage_cooldown_days * 86_400
    {
        return Some(format!(
            "{} 离婚后 {} 天内无法再次结婚",
            ctx.target_name, marriage_cooldown_days
        ));
    }
    // C# :13192 目标是否允许求婚（AllowMarriage 开关）
    if !ctx.target_allow_marriage {
        return Some("目标玩家当前不允许接收求婚".to_string());
    }
    // C# :13204 双方死亡
    if ctx.requester_dead || ctx.target_dead {
        return Some("死亡状态下无法求婚".to_string());
    }
    // C# :13210 目标已有待处理求婚（MarriageProposal != null）
    if ctx.target_has_pending {
        return Some(format!("{} 已有待处理的求婚", ctx.target_name));
    }
    // C# :13216 同地图且距离 <= Globals.DataRange(16)
    if ctx.requester_map != ctx.target_map
        || max_distance(ctx.requester_x, ctx.requester_y, ctx.target_x, ctx.target_y) > 16
    {
        return Some(format!("{} 不在可求婚范围内", ctx.target_name));
    }
    // C# :13222 目标已婚
    if ctx.target_spouse {
        return Some("目标玩家已经结婚了".to_string());
    }
    None
}

// 行会创建费用：金币（对应 C# Settings.Guild_CreationCostList gold entry）
// 创建行会所需金币来自 cfg.server.toml (social.guild_creation_cost_gold),
// 不再需要 hardcoded 常量。config 在 SocialActor.config 字段里。

/// Mir 方向常量（对应 C# MirDirection 0..7）
const DIR_DX: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
const DIR_DY: [i32; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];

fn player_dir_x(dir: u8) -> i32 {
    DIR_DX[dir as usize & 7]
}
fn player_dir_y(dir: u8) -> i32 {
    DIR_DY[dir as usize & 7]
}

// ============================================================
// Helper methods
// ============================================================

impl SocialActor {
    /// 按名称查找在线玩家 session_id
    async fn find_player_by_name(&self, name: &str, exclude_session: u64) -> Option<u64> {
        for (sid, record) in &self.players {
            if *sid == exclude_session {
                continue;
            }
            if let Ok(Some(s)) = record.ask(GetPlayerState).await {
                if s.name == name {
                    return Some(*sid);
                }
            }
        }
        None
    }

    /// #1329：查询地图标题（LoverUpdate MapName；WorldActor GetMapTitle）
    async fn map_title(&self, map_index: u16) -> String {
        if let Some(world) = &self.world_ref {
            if let Ok(Some(title)) = world
                .ask(crate::actors::world::GetMapTitle { map_index })
                .await
            {
                return title;
            }
        }
        String::new()
    }

    /// #2012：查询玩家是否在安全区（C# InSafeZone；WorldActor IsInSafeZone）
    async fn is_in_safe_zone(&self, map_index: u16, x: i32, y: i32) -> bool {
        if let Some(world) = &self.world_ref {
            if let Ok(safe) = world
                .ask(crate::actors::world::IsInSafeZone { map_index, x, y })
                .await
            {
                return safe;
            }
        }
        false
    }

    /// #1329：结婚天数（unix 秒 → 整天；C# MarriedDays）
    fn married_days(&self, date_secs: i64) -> i16 {
        if date_secs <= 0 {
            return 0;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        ((now - date_secs).max(0) / 86400).min(i16::MAX as i64) as i16
    }

    /// 查找交易（不可变）
    fn find_trade(&self, session_id: u64) -> Option<&TradeSession> {
        self.active_trades
            .values()
            .find(|t| t.side_a.session_id == session_id || t.side_b.session_id == session_id)
    }

    /// 查找交易（可变）
    fn find_trade_mut(&mut self, session_id: u64) -> Option<&mut TradeSession> {
        self.active_trades
            .values_mut()
            .find(|t| t.side_a.session_id == session_id || t.side_b.session_id == session_id)
    }

    /// 发送好友列表
    async fn send_friends_list(&self, session_id: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 收集所有在线 object_ids 与名字（离线添加的好友按名字判定在线）
        let mut online_object_ids: Vec<u32> = Vec::new();
        let mut online_names: Vec<String> = Vec::new();
        for r in self.players.values() {
            if let Ok(Some(s)) = r.ask(GetPlayerState).await {
                online_object_ids.push(s.object_id);
                online_names.push(s.name.clone());
            }
        }

        send_friends_list_packet(
            &self.gate_ref,
            session_id,
            &state.friend_list.friends,
            &state.friend_list.blocked,
            &online_object_ids,
            &online_names,
        );
    }

    /// 向所有在线行会成员广播完整行会信息
    async fn broadcast_guild_info(&self, guild_name: &str) {
        if let Some(guild) = self.guilds.get(guild_name) {
            for sid in guild.online_sessions(0) {
                send_guild_info_packet(&self.gate_ref, sid, guild);
            }
        }
    }

    /// 向所有在线行会成员广播仓库物品列表（M32）
    async fn broadcast_guild_storage_list(&self, guild_name: &str) {
        match self.guilds.get(guild_name) {
            Some(guild) => {
                let sids = guild.online_sessions(0);
                debug!(
                    "GuildStorageList broadcast '{}': sessions={:?}",
                    guild_name, sids
                );
                for sid in sids {
                    send_guild_storage_list_packet(&self.gate_ref, sid, guild);
                }
            }
            None => tracing::warn!("🏰 M32 broadcast: guild '{}' not found", guild_name),
        }
    }

    /// 保存行会到数据库（创建/金币/物品/公告等变更后调用）
    /// #295：下发行会仓库金币实时包（C# S.GuildStorageGoldChange）
    async fn send_guild_storage_gold_change(
        &self,
        session_id: u64,
        name: &str,
        amount: u32,
        change_type: u8,
    ) {
        let mut body = Vec::new();
        if mir2_shared::packets::base::serialize_packet(
            &mut std::io::Cursor::new(&mut body),
            &mir2_shared::packets::server::GuildStorageGoldChange {
                amount,
                change_type,
                name: name.to_string(),
            },
        )
        .is_ok()
        {
            let _ = self
                .gate_ref
                .tell(crate::gate::actor::SendToClient {
                    session_id,
                    data: body,
                })
                .await;
        }
    }

    /// #295：下发行会仓库物品实时包（C# S.GuildStorageItemChange）
    async fn send_guild_storage_item_change(
        &self,
        session_id: u64,
        change_type: u8,
        to: i32,
        from: i32,
        user: i32,
        item: Option<(i64, mir2_shared::data::item::UserItem)>,
    ) {
        let mut body = Vec::new();
        if mir2_shared::packets::base::serialize_packet(
            &mut std::io::Cursor::new(&mut body),
            &mir2_shared::packets::server::GuildStorageItemChange {
                change_type,
                to,
                from,
                user,
                item,
            },
        )
        .is_ok()
        {
            let _ = self
                .gate_ref
                .tell(crate::gate::actor::SendToClient {
                    session_id,
                    data: body,
                })
                .await;
        }
    }

    async fn save_guild_to_db(&self, guild_name: &str) {
        if let Some(guild) = self.guilds.get(guild_name) {
            if let Err(e) = db::save_guild(&self.db_pool, guild).await {
                warn!("Failed to save guild '{}' to DB: {}", guild.name, e);
            }
        }
    }

    // === 组队辅助方法 ===

    /// 加入或创建组队
    async fn join_or_create_group(
        &mut self,
        joiner_session: u64,
        target_session: u64,
        joiner_name: &str,
    ) {
        let joiner_name = joiner_name.to_string();

        // 获取加入者信息
        let joiner_member = {
            let record = match self.players.get(&joiner_session) {
                Some(r) => r,
                None => return,
            };
            let state = match record.ask(GetPlayerState).await {
                Ok(Some(s)) => s,
                _ => return,
            };
            GroupMember {
                session_id: state.session_id,
                name: state.name.clone(),
                is_leader: false,
                online: true,
            }
        };

        // 检查目标玩家是否在线
        let target_record = match self.players.get(&target_session) {
            Some(r) => r,
            None => {
                send_system_message(&self.gate_ref, joiner_session, "目标玩家不在线");
                return;
            }
        };

        let target_state = match target_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if let Some(target_group_id) = target_state.group_id {
            // 加入已有组队
            if let Some(group) = self.groups.get_mut(&target_group_id) {
                if !group.add_member(joiner_member) {
                    send_system_message(&self.gate_ref, joiner_session, "队伍已满或你已在队伍中");
                    return;
                }
                // 更新加入者的 group_id
                if let Some(record) = self.players.get(&joiner_session) {
                    let _ = record
                        .ask(SetGroupId {
                            group_id: Some(target_group_id),
                        })
                        .await;
                }
                send_system_message(
                    &self.gate_ref,
                    joiner_session,
                    &format!("已加入队伍 #{}", target_group_id),
                );
                self.broadcast_group_update(target_group_id);
                debug!("Player {} joined group #{}", joiner_name, target_group_id);
            } else {
                // #835：目标 group_id 指向已不存在的组（陈旧引用）——清掉并新建组队，
                // 否则旧成员永远无法再次组队
                debug!(
                    "Player {} has stale group_id {} (group gone); creating new group",
                    target_state.name, target_group_id
                );
                if let Some(record) = self.players.get(&target_session) {
                    let _ = record.ask(SetGroupId { group_id: None }).await;
                }
                self.create_new_group(
                    joiner_session,
                    &joiner_name,
                    joiner_member,
                    target_session,
                    target_state,
                )
                .await;
            }
        } else {
            self.create_new_group(
                joiner_session,
                &joiner_name,
                joiner_member,
                target_session,
                target_state,
            )
            .await;
        }
    }

    /// 创建新组队（C# Group 创建语义：目标为队长，加入者为队员）
    async fn create_new_group(
        &mut self,
        joiner_session: u64,
        joiner_name: &str,
        joiner_member: GroupMember,
        target_session: u64,
        target_state: crate::actors::player::PlayerState,
    ) {
        let group_id = self.next_group_id;
        self.next_group_id += 1;

        let target_member = GroupMember {
            session_id: target_session,
            name: target_state.name.clone(),
            is_leader: true,
            online: true,
        };

        let mut group = Group::new(group_id, target_member);
        group.add_member(joiner_member);

        // 更新两个玩家的 group_id
        if let Some(record) = self.players.get(&target_session) {
            let _ = record
                .ask(SetGroupId {
                    group_id: Some(group_id),
                })
                .await;
        }
        if let Some(record) = self.players.get(&joiner_session) {
            let _ = record
                .ask(SetGroupId {
                    group_id: Some(group_id),
                })
                .await;
        }

        self.groups.insert(group_id, group);
        send_system_message(
            &self.gate_ref,
            joiner_session,
            &format!("队伍 #{} 已创建", group_id),
        );
        send_system_message(
            &self.gate_ref,
            target_session,
            &format!("队伍 #{} 已创建", group_id),
        );
        // 创建后广播成员列表（C# 语义：双方立即看到组队面板）
        self.broadcast_group_update(group_id);
        debug!(
            "Created group #{} with {} and {}",
            group_id, target_state.name, joiner_name
        );
    }

    /// 离开组队
    async fn leave_group(&mut self, group_id: u64, session_id: u64, name: &str) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            if group.remove_member(session_id).is_some() {
                if let Some(record) = self.players.get(&session_id) {
                    let _ = record.ask(SetGroupId { group_id: None }).await;
                }
                send_system_message(&self.gate_ref, session_id, "已离开队伍");
                debug!("Player {} left group #{}", name, group_id);

                if group.member_count() == 0 {
                    self.groups.remove(&group_id);
                } else {
                    self.broadcast_group_update(group_id);
                }
            }
        }
    }

    /// 广播组队更新给所有成员
    fn broadcast_group_update(&self, group_id: u64) {
        if let Some(group) = self.groups.get(&group_id) {
            for member in &group.members {
                if member.online {
                    send_group_members_map(&self.gate_ref, member.session_id, &group.members);
                }
            }
        }
    }

    // === 交易执行 ===

    /// 执行交易：实际扣除并转移物品/金币
    async fn execute_trade(&mut self, trigger_session: u64) {
        // 先克隆交易数据（避免借用冲突）
        let trade_data = match self.find_trade(trigger_session) {
            Some(t) => t.clone(),
            None => return,
        };

        let (s1, s2) = trade_data.participant_sessions();
        let gold_a = trade_data.side_a.gold;
        let gold_b = trade_data.side_b.gold;
        let items_a: Vec<_> = trade_data.side_a.items.to_vec();
        let items_b: Vec<_> = trade_data.side_b.items.to_vec();

        // #924：锁定后复检（C# TradeConfirm——!InRange || 不同地图 || 死亡 → TradeCancel）
        let a_state = match self.players.get(&s1) {
            Some(r) => match r.ask(GetPlayerState).await {
                Ok(Some(st)) => Some(st),
                _ => None,
            },
            None => None,
        };
        let b_state = match self.players.get(&s2) {
            Some(r) => match r.ask(GetPlayerState).await {
                Ok(Some(st)) => Some(st),
                _ => None,
            },
            None => None,
        };
        let recheck_ok = match (a_state, b_state) {
            (Some(a), Some(b)) => {
                !a.is_dead
                    && !b.is_dead
                    && a.map_index == b.map_index
                    // C# 10803：InRange(TradePartner, Globals.DataRange=16) + FacingEachOther
                    && max_distance(a.x, a.y, b.x, b.y) <= 16
                    && facing_each_other(a.direction, a.x, a.y, b.direction, b.x, b.y)
            }
            _ => false,
        };
        if !recheck_ok {
            send_system_message(&self.gate_ref, s1, "距离过远或状态异常，交易已取消");
            send_system_message(&self.gate_ref, s2, "距离过远或状态异常，交易已取消");
            send_trade_cancel_packet(&self.gate_ref, s1);
            send_trade_cancel_packet(&self.gate_ref, s2);
            send_trade_close_packet(&self.gate_ref, s1);
            send_trade_close_packet(&self.gate_ref, s2);
            self.active_trades.remove(&trade_data.side_a.session_id);
            return;
        }

        // 容量检查（对应 C# CanGainItems / CanGainGold）
        // A 能否接收 B 的物品和金币
        let a_can_receive = match self.players.get(&s1) {
            Some(rec) => {
                let incoming_b: Vec<mir2_shared::data::item::UserItem> =
                    items_b.iter().filter_map(|t| t.item_data.clone()).collect();
                let items_ok = incoming_b.is_empty()
                    || rec
                        .ask(crate::actors::player::CanGainItemsFor { items: incoming_b })
                        .await
                        .unwrap_or(false);
                let gold_ok = gold_b == 0
                    || rec
                        .ask(CanGainGold {
                            amount: (gold_b as u32),
                        })
                        .await
                        .unwrap_or(false);
                items_ok && gold_ok
            }
            None => false,
        };
        // B 能否接收 A 的物品和金币
        let b_can_receive = match self.players.get(&s2) {
            Some(rec) => {
                let incoming_a: Vec<mir2_shared::data::item::UserItem> =
                    items_a.iter().filter_map(|t| t.item_data.clone()).collect();
                let items_ok = incoming_a.is_empty()
                    || rec
                        .ask(crate::actors::player::CanGainItemsFor { items: incoming_a })
                        .await
                        .unwrap_or(false);
                let gold_ok = gold_a == 0
                    || rec
                        .ask(CanGainGold {
                            amount: (gold_a as u32),
                        })
                        .await
                        .unwrap_or(false);
                items_ok && gold_ok
            }
            None => false,
        };

        if !a_can_receive {
            send_system_message(
                &self.gate_ref,
                s1,
                "你的背包已满或金币已达上限，无法完成交易",
            );
            send_trade_cancel_packet(&self.gate_ref, s1);
            send_trade_cancel_packet(&self.gate_ref, s2);
            send_trade_close_packet(&self.gate_ref, s1);
            send_trade_close_packet(&self.gate_ref, s2);
            self.active_trades.remove(&trade_data.side_a.session_id);
            return;
        }
        if !b_can_receive {
            send_system_message(
                &self.gate_ref,
                s2,
                "你的背包已满或金币已达上限，无法完成交易",
            );
            send_trade_cancel_packet(&self.gate_ref, s1);
            send_trade_cancel_packet(&self.gate_ref, s2);
            send_trade_close_packet(&self.gate_ref, s1);
            send_trade_close_packet(&self.gate_ref, s2);
            self.active_trades.remove(&trade_data.side_a.session_id);
            return;
        }

        // 从 A 扣除金币和物品
        if let Some(rec) = self.players.get(&s1) {
            if gold_a > 0 {
                let _ = rec.ask(DeductGold { amount: gold_a }).await;
            }
            for item in &items_a {
                let _ = rec
                    .ask(RemoveItemFromInventory {
                        unique_id: item.uid,
                    })
                    .await;
            }
        }

        // 从 B 扣除金币和物品
        if let Some(rec) = self.players.get(&s2) {
            if gold_b > 0 {
                let _ = rec.ask(DeductGold { amount: gold_b }).await;
            }
            for item in &items_b {
                let _ = rec
                    .ask(RemoveItemFromInventory {
                        unique_id: item.uid,
                    })
                    .await;
            }
        }

        // 将 B 的金币和物品给 A
        if let Some(rec) = self.players.get(&s1) {
            if gold_b > 0 {
                let _ = rec.ask(AddGold { amount: gold_b }).await;
            }
            for item in &items_b {
                // 优先使用交易侧缓存的完整物品（DepositTradeItemBySlot 已从背包移除，不能再查询）
                if let Some(data) = item.item_data.clone() {
                    let _ = rec.ask(AddItemToInventory { item: data }).await;
                } else if let Some(rec2) = self.players.get(&s2) {
                    if let Ok(Some(item_data)) = rec2
                        .ask(GetItemInfo {
                            unique_id: item.uid,
                        })
                        .await
                    {
                        let _ = rec.ask(AddItemToInventory { item: item_data }).await;
                    }
                }
            }
        }

        // 将 A 的金币和物品给 B
        if let Some(rec) = self.players.get(&s2) {
            if gold_a > 0 {
                let _ = rec.ask(AddGold { amount: gold_a }).await;
            }
            for item in &items_a {
                if let Some(data) = item.item_data.clone() {
                    let _ = rec.ask(AddItemToInventory { item: data }).await;
                } else if let Some(rec2) = self.players.get(&s1) {
                    if let Ok(Some(item_data)) = rec2
                        .ask(GetItemInfo {
                            unique_id: item.uid,
                        })
                        .await
                    {
                        let _ = rec.ask(AddItemToInventory { item: item_data }).await;
                    }
                }
            }
        }

        // 移除交易会话
        self.active_trades.remove(&trade_data.side_a.session_id);

        send_trade_success_packet(&self.gate_ref, s1);
        send_trade_success_packet(&self.gate_ref, s2);

        debug!(
            "Trade executed: {} gold + {} items <-> {} gold + {} items",
            gold_a,
            items_a.len(),
            gold_b,
            items_b.len()
        );

        send_trade_close_packet(&self.gate_ref, s1);
        send_trade_close_packet(&self.gate_ref, s2);
    }

    // === 召回命令 ===

    /// 检查召回套装是否完整（需要4种不同类型的Recall装备）
    async fn check_recall_set(&self, session_id: u64) -> bool {
        let record = match self.players.get(&session_id) {
            Some(r) => r,
            None => return false,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return false,
        };

        let item_infos = self.config.item_infos.read().await;
        let recall_types: HashSet<i32> = state.inventory.equipment.iter()
            .filter_map(|e| e.as_ref())
            .filter(|e| {
                item_infos.get(&e.item_index)
                    .map(|info| info.set_type == 2 /* ItemSet.Recall：C# 原始值，SharedRust 枚举 +3 不可用于 DB 比较 */)
                    .unwrap_or(false)
            })
            .filter_map(|e| item_infos.get(&e.item_index))
            .map(|info| info.item_type)
            .collect();
        recall_types.len() >= 4
    }

    /// GROUPRECALL - 召回所有组队成员
    async fn handle_group_recall(&mut self, leader_session: u64) {
        let record = match self.players.get(&leader_session) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // Must be group leader
        let Some(group_id) = state.group_id else {
            return;
        };
        let Some(group) = self.groups.get(&group_id) else {
            return;
        };
        if group.leader_session() != Some(leader_session) {
            send_system_message(&self.gate_ref, leader_session, "你不是队长");
            return;
        }

        // Check dead
        if state.is_dead {
            send_system_message(
                &self.gate_ref,
                leader_session,
                "你无法在死亡状态下使用组队召回",
            );
            return;
        }

        // Check no_recall
        {
            let map_infos = self.config.map_infos.read().await;
            if let Some(mi) = map_infos.get(&(state.map_index as i32)) {
                if mi.no_recall {
                    send_system_message(&self.gate_ref, leader_session, "该地图无法使用组队召回");
                    return;
                }
            }
        }

        // Check 180s cooldown
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        if now_ms < state.last_recall_time {
            let remaining = (state.last_recall_time - now_ms).div_ceil(1000);
            send_system_message(
                &self.gate_ref,
                leader_session,
                &format!("你还需要等待 {} 秒才能再次使用组队召回", remaining),
            );
            return;
        }

        // Check Recall item set
        if !self.check_recall_set(leader_session).await {
            send_system_message(
                &self.gate_ref,
                leader_session,
                "你需要装备完整的召回套装才能使用组队召回",
            );
            return;
        }

        let target_map = state.map_index;
        let target_x = state.x;
        let target_y = state.y;

        // Set cooldown BEFORE loop
        let new_recall_time = now_ms + 180_000;
        let _ = record
            .ask(SetLastRecallTime {
                last_recall_time: new_recall_time,
            })
            .await;

        // Teleport all group members (only those with EnableGroupRecall=true)
        // Clone group to avoid borrowing self while iterating + calling ask()
        let group = self.groups.get(&group_id).unwrap().clone();
        for member in &group.members {
            if member.session_id == leader_session {
                continue;
            }
            if !member.online {
                continue;
            }
            if let Some(mem_record) = self.players.get(&member.session_id) {
                if let Ok(Some(mem_state)) = mem_record.ask(GetPlayerState).await {
                    if !mem_state.enable_group_recall {
                        send_system_message(
                            &self.gate_ref,
                            mem_state.session_id,
                            "有人试图未经你允许进行组队召回",
                        );
                        send_system_message(
                            &self.gate_ref,
                            leader_session,
                            &format!("{} 拒绝了组队召回", mem_state.name),
                        );
                        continue;
                    }
                    let _ = mem_record
                        .ask(SetPlayerPosition {
                            x: target_x,
                            y: target_y,
                            direction: mem_state.direction,
                            map_index: Some(target_map),
                            is_mounted: None,
                        })
                        .await;
                    let mut body = Vec::new();
                    body.extend_from_slice(&target_x.to_le_bytes());
                    body.extend_from_slice(&target_y.to_le_bytes());
                    body.push(mem_state.direction);
                    let _ = self
                        .gate_ref
                        .tell(SendToClient {
                            session_id: member.session_id,
                            data: build_packet_bytes(ServerPacketIds::UserLocation as i16, &body),
                        })
                        .await;
                    debug!(
                        "GROUPRECALL: {} recalled to ({}, {}) on map {}",
                        mem_state.name, target_x, target_y, target_map
                    );
                }
            }
        }
    }

    /// RECALLMEMBER <name> - 召回指定成员
    async fn handle_recall_member(&mut self, leader_session: u64, member_name: &str) {
        let record = match self.players.get(&leader_session) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let Some(group_id) = state.group_id else {
            return;
        };
        let Some(group) = self.groups.get(&group_id) else {
            return;
        };
        if group.leader_session() != Some(leader_session) {
            send_system_message(&self.gate_ref, leader_session, "你不是队长");
            return;
        }

        if state.is_dead {
            send_system_message(
                &self.gate_ref,
                leader_session,
                "你无法在死亡状态下使用组队召回",
            );
            return;
        }

        {
            let map_infos = self.config.map_infos.read().await;
            if let Some(mi) = map_infos.get(&(state.map_index as i32)) {
                if mi.no_recall {
                    send_system_message(&self.gate_ref, leader_session, "该地图无法使用组队召回");
                    return;
                }
            }
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        if now_ms < state.last_recall_time {
            let remaining = (state.last_recall_time - now_ms).div_ceil(1000);
            send_system_message(
                &self.gate_ref,
                leader_session,
                &format!("你还需要等待 {} 秒才能再次使用组队召回", remaining),
            );
            return;
        }

        if !self.check_recall_set(leader_session).await {
            send_system_message(
                &self.gate_ref,
                leader_session,
                "你需要装备完整的召回套装才能使用组队召回",
            );
            return;
        }

        let target_map = state.map_index;
        let target_x = state.x;
        let target_y = state.y;

        // Find and teleport the named member
        // Clone group to avoid borrowing self while iterating + calling ask()
        let group = self.groups.get(&group_id).unwrap().clone();
        for member in &group.members {
            if member.session_id == leader_session {
                continue;
            }
            if !member.online {
                continue;
            }
            if let Some(mem_record) = self.players.get(&member.session_id) {
                if let Ok(Some(mem_state)) = mem_record.ask(GetPlayerState).await {
                    if mem_state.name.eq_ignore_ascii_case(member_name) {
                        if !mem_state.enable_group_recall {
                            send_system_message(
                                &self.gate_ref,
                                mem_state.session_id,
                                "有人试图未经你允许进行组队召回",
                            );
                            send_system_message(
                                &self.gate_ref,
                                leader_session,
                                &format!("{} 拒绝了组队召回", mem_state.name),
                            );
                            return;
                        }
                        let _ = record
                            .ask(SetLastRecallTime {
                                last_recall_time: now_ms + 60_000,
                            })
                            .await;
                        let _ = mem_record
                            .ask(SetPlayerPosition {
                                x: target_x,
                                y: target_y,
                                direction: mem_state.direction,
                                map_index: Some(target_map),
                                is_mounted: None,
                            })
                            .await;
                        let mut body = Vec::new();
                        body.extend_from_slice(&target_x.to_le_bytes());
                        body.extend_from_slice(&target_y.to_le_bytes());
                        body.push(mem_state.direction);
                        let _ = self
                            .gate_ref
                            .tell(SendToClient {
                                session_id: member.session_id,
                                data: build_packet_bytes(
                                    ServerPacketIds::UserLocation as i16,
                                    &body,
                                ),
                            })
                            .await;
                        debug!(
                            "RECALLMEMBER: {} recalled to ({}, {})",
                            mem_state.name, target_x, target_y
                        );
                        return;
                    }
                }
            }
        }
        send_system_message(&self.gate_ref, leader_session, "玩家未找到");
    }

    /// RECALL - 召回配偶（对应 C# PlayerObject.cs:2439 RECALLLOVER）
    async fn handle_recall_lover(&mut self, session_id: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let Some(spouse_name) = &state.spouse_name else {
            send_system_message(&self.gate_ref, session_id, "你需要有配偶才能使用召回");
            return;
        };

        if state.is_dead {
            send_system_message(&self.gate_ref, session_id, "你无法在死亡状态下使用配偶召回");
            return;
        }

        {
            let map_infos = self.config.map_infos.read().await;
            if let Some(mi) = map_infos.get(&(state.map_index as i32)) {
                if mi.no_recall {
                    send_system_message(&self.gate_ref, session_id, "该地图无法使用配偶召回");
                    return;
                }
            }
        }

        let ring_l = state.inventory.get_equipment(EquipmentSlot::RingL);
        if ring_l.is_none() {
            send_system_message(
                &self.gate_ref,
                session_id,
                "你需要佩戴结婚戒指才能使用配偶召回",
            );
            return;
        }

        let ring = ring_l.unwrap();
        if ring.wedding_ring == 0 {
            send_system_message(&self.gate_ref, session_id, "你的结婚戒指未绑定配偶");
            return;
        }

        // 全局开关（对应 C# Settings.WeddingRingRecall）
        if !self.config.wedding_ring_recall_enabled {
            send_system_message(&self.gate_ref, session_id, "结婚戒指召回功能已关闭");
            return;
        }

        let target_map = state.map_index;
        let target_x = state.x;
        let target_y = state.y;
        let spouse_name = spouse_name.clone();

        // Find spouse and teleport
        for (other_session, other_record) in &self.players {
            if *other_session == session_id {
                continue;
            }
            if let Ok(Some(other_state)) = other_record.ask(GetPlayerState).await {
                if other_state.name.eq_ignore_ascii_case(&spouse_name) {
                    // 检查配偶是否死亡（对应 C# player.Dead）
                    if other_state.is_dead {
                        send_system_message(&self.gate_ref, session_id, "配偶已死亡，无法召回");
                        return;
                    }

                    // 检查配偶是否也佩戴了结婚戒指（对应 C# player.Info.Equipment[RingL] == null）
                    let spouse_ring_l = other_state.inventory.get_equipment(EquipmentSlot::RingL);
                    if spouse_ring_l.is_none() {
                        send_system_message(
                            &self.gate_ref,
                            *other_session,
                            "你需要佩戴结婚戒指才能被召回",
                        );
                        send_system_message(
                            &self.gate_ref,
                            session_id,
                            &format!("{} 没有佩戴结婚戒指", other_state.name),
                        );
                        return;
                    }
                    let spouse_ring = spouse_ring_l.unwrap();
                    // 检查配偶戒指绑定是否正确（对应 C# player.Info.Equipment[RingL].WeddingRing != player.Info.Married）
                    if spouse_ring.wedding_ring == 0 {
                        send_system_message(
                            &self.gate_ref,
                            *other_session,
                            "你需要佩戴已绑定的结婚戒指才能被召回",
                        );
                        send_system_message(
                            &self.gate_ref,
                            session_id,
                            &format!("{} 没有佩戴已绑定的结婚戒指", other_state.name),
                        );
                        return;
                    }

                    // 检查配偶是否允许配偶召回（对应 C# player.AllowLoverRecall）
                    if !other_state.allow_lover_recall {
                        send_system_message(
                            &self.gate_ref,
                            *other_session,
                            "有人试图未经你允许进行配偶召回",
                        );
                        send_system_message(
                            &self.gate_ref,
                            session_id,
                            &format!("{} 拒绝了配偶召回", other_state.name),
                        );
                        return;
                    }

                    // 检查冷却时间（对应 C# Envir.Time < LastRecallTime && Envir.Time < player.LastRecallTime）
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    if now_ms < state.last_recall_time {
                        let remaining = (state.last_recall_time - now_ms).div_ceil(1000);
                        send_system_message(
                            &self.gate_ref,
                            session_id,
                            &format!("你还需要等待 {} 秒才能再次使用配偶召回", remaining),
                        );
                        return;
                    }
                    if now_ms < other_state.last_recall_time {
                        let remaining = (other_state.last_recall_time - now_ms).div_ceil(1000);
                        send_system_message(
                            &self.gate_ref,
                            session_id,
                            &format!("配偶还需要等待 {} 秒才能再次使用召回", remaining),
                        );
                        return;
                    }

                    // 设置冷却（60s，对应 C# LastRecallTime = Envir.Time + 60000; player.LastRecallTime = Envir.Time + 60000）
                    let new_recall_time = now_ms + 60_000;
                    let _ = record
                        .ask(SetLastRecallTime {
                            last_recall_time: new_recall_time,
                        })
                        .await;
                    let _ = other_record
                        .ask(SetLastRecallTime {
                            last_recall_time: new_recall_time,
                        })
                        .await;

                    // 尝试 Teleport（对应 C# player.Teleport(CurrentMap, Front)，失败则 CurrentLocation）
                    // Front = 发起者当前位置前方一格
                    let front_x = target_x + player_dir_x(state.direction);
                    let front_y = target_y + player_dir_y(state.direction);
                    let _ = other_record
                        .ask(SetPlayerPosition {
                            x: front_x,
                            y: front_y,
                            direction: other_state.direction,
                            map_index: Some(target_map),
                            is_mounted: None,
                        })
                        .await;
                    let mut body = Vec::new();
                    body.extend_from_slice(&front_x.to_le_bytes());
                    body.extend_from_slice(&front_y.to_le_bytes());
                    body.push(other_state.direction);
                    let _ = self
                        .gate_ref
                        .tell(SendToClient {
                            session_id: *other_session,
                            data: build_packet_bytes(ServerPacketIds::UserLocation as i16, &body),
                        })
                        .await;
                    debug!(
                        "RECALL: {} recalled {} to ({}, {})",
                        state.name, spouse_name, front_x, front_y
                    );
                    return;
                }
            }
        }
        send_system_message(&self.gate_ref, session_id, "配偶不在线");
    }

    /// RIDE - 切换骑乘状态
    async fn handle_toggle_ride(&mut self, session_id: u64) {
        debug!("RIDE: session={} toggle", session_id);
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => {
                warn!("RIDE: session {} not in social players", session_id);
                return;
            }
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let mount_item = match state.inventory.get_equipment(EquipmentSlot::Mount) {
            Some(m) => m.clone(),
            None => {
                debug!(
                    "RIDE: no mount equipped (slots={:?})",
                    state.inventory.equipment.len()
                );
                send_system_message(&self.gate_ref, session_id, "你没有装备坐骑");
                return;
            }
        };
        debug!(
            "RIDE: mount item idx={} slots={}",
            mount_item.item_index,
            mount_item.slots.len()
        );

        let has_saddle = mount_item.slots.get(2).and_then(|s| s.as_ref()).is_some();
        if !has_saddle {
            send_system_message(&self.gate_ref, session_id, "你必须给坐骑装备鞍才能骑乘");
            return;
        }

        let map_info;
        {
            let map_infos = self.config.map_infos.read().await;
            map_info = map_infos.get(&(state.map_index as i32)).cloned();
        }

        if state.is_mounted {
            let _ = record
                .ask(SetPlayerPosition {
                    x: state.x,
                    y: state.y,
                    direction: state.direction,
                    map_index: None,
                    is_mounted: Some(false),
                })
                .await;
            // M60：清坐骑类型 + 同步外观给自身与同图玩家
            if let Ok(Some(mut st)) = record.ask(GetPlayerState).await {
                st.mount_type = 0;
                let _ = record
                    .ask(crate::actors::player::SetPlayerState { state: st.clone() })
                    .await;
                self.broadcast_ride_appearance(session_id, &st).await;
            }
            send_system_message(&self.gate_ref, session_id, "你下了坐骑");
            debug!("RIDE: {} dismounted", state.name);
        } else {
            if let Some(ref mi) = map_info {
                if mi.no_mount {
                    send_system_message(&self.gate_ref, session_id, "该地图无法骑乘坐骑");
                    return;
                }
                if mi.need_bridle {
                    let has_reins = mount_item.slots.first().and_then(|s| s.as_ref()).is_some();
                    if !has_reins {
                        send_system_message(
                            &self.gate_ref,
                            session_id,
                            "该地图需要给坐骑装备缰绳才能骑乘",
                        );
                        return;
                    }
                }
            }

            let _ = record
                .ask(SetPlayerPosition {
                    x: state.x,
                    y: state.y,
                    direction: state.direction,
                    map_index: None,
                    is_mounted: Some(true),
                })
                .await;
            // M60：坐骑类型取装备坐骑物品 shape（C# Mount.MountType）
            let mount_type: i16 = {
                let infos = self.config.item_infos.read().await;
                infos
                    .get(&mount_item.item_index)
                    .map(|i| i.shape as i16)
                    .unwrap_or(0)
            };
            if let Ok(Some(mut st)) = record.ask(GetPlayerState).await {
                st.mount_type = mount_type;
                let _ = record
                    .ask(crate::actors::player::SetPlayerState { state: st.clone() })
                    .await;
                self.broadcast_ride_appearance(session_id, &st).await;
            }
            send_system_message(&self.gate_ref, session_id, "你骑上了坐骑");
            debug!("RIDE: {} mounted (mount_type={})", state.name, mount_type);
        }
    }
}

// ============================================================
// Message: SocialPlayerJoined
// ============================================================

impl Message<SocialPlayerJoined> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SocialPlayerJoined, _ctx: &mut Context<Self, Self::Reply>) {
        self.players.insert(msg.session_id, msg.actor_ref.clone());
        // C# MirConnection.cs:701：登录时主动下发好友列表（GetFriends → S.FriendUpdate）
        self.send_friends_list(msg.session_id).await;
        // 同步行会成员在线状态（服务端重启后行会从 DB 加载，成员 session 为 None；
        // 不更新则行会广播/在线显示失效）
        let state = match msg.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 离线添加的好友上线：校正对方列表里同名字条目的 object_id 为运行时 ID，并刷新其好友列表（在线状态翻转）
        let new_oid = state.object_id;
        let new_name = state.name.clone();
        for (sid, r) in &self.players {
            if *sid == msg.session_id {
                continue;
            }
            if let Ok(Some(mut os)) = r.ask(GetPlayerState).await {
                let mut changed = false;
                for f in os.friend_list.friends.iter_mut() {
                    if f.name.eq_ignore_ascii_case(&new_name) && f.object_id != new_oid {
                        f.object_id = new_oid;
                        changed = true;
                    }
                }
                for b in os.friend_list.blocked.iter_mut() {
                    if b.name.eq_ignore_ascii_case(&new_name) && b.object_id != new_oid {
                        b.object_id = new_oid;
                        changed = true;
                    }
                }
                if changed {
                    let _ = r.ask(SetPlayerState { state: os }).await;
                    self.send_friends_list(*sid).await;
                }
            }
        }
        if let Some(guild_name) = &state.guild_name {
            if let Some(guild) = self.guilds.get_mut(guild_name) {
                guild.set_online(&state.name, msg.session_id);
                debug!(
                    "SocialActor: guild member {} online (guild={})",
                    state.name, guild_name
                );
            }
        }

        // #2374：C# PlayerObject.cs:1194-1196——师徒到期（MentorDate.AddDays(7) < Now）登录时自动解除（force=false 无冷却）
        let now_secs = crate::actors::world::partners::now_unix_secs();
        if state.mentor_name.is_some()
            && mentor_relationship_expired(
                state.mentor_date,
                now_secs,
                self.config.mentor_length_days as i64,
            )
        {
            self.do_mentor_break(msg.session_id, false).await;
        }
        // 重新取状态（到期解除可能已清空师徒关系）
        let state = match msg.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 师徒状态同步（C# GetMentor 语义：上线时通知双方，双方各发 MentorUpdate）
        if let Some(partner_name) = &state.mentor_name {
            if let Some(partner_sid) = self.find_player_by_name(partner_name, msg.session_id).await
            {
                if let Some(partner_record) = self.players.get(&partner_sid) {
                    if let Ok(Some(partner_state)) = partner_record.ask(GetPlayerState).await {
                        // 上线者视角：对方（师父/徒弟）信息
                        send_mentor_update_packet(
                            &self.gate_ref,
                            msg.session_id,
                            partner_name,
                            partner_state.level as u32,
                            true,
                            // C# GetMentor：MenteeEXP = Info.MentorExp（接收者自己的导师银行）
                            state.mentor_exp,
                        );
                        // 对方视角：上线者信息
                        send_mentor_update_packet(
                            &self.gate_ref,
                            partner_sid,
                            &state.name,
                            state.level as u32,
                            true,
                            partner_state.mentor_exp,
                        );
                        let rel =
                            if partner_state.mentor_name.as_deref() == Some(state.name.as_str()) {
                                "徒弟"
                            } else {
                                "师父"
                            };
                        send_system_message(
                            &self.gate_ref,
                            partner_sid,
                            &format!("你的{} {} 上线了", rel, state.name),
                        );
                    }
                }
            } else {
                // 对方离线：显示名字 + 离线（等级未知给 0）
                send_mentor_update_packet(
                    &self.gate_ref,
                    msg.session_id,
                    partner_name,
                    0,
                    false,
                    state.mentor_exp,
                );
            }
        }
        // 配偶状态同步（C# GetRelationship 语义：上线时通知双方，双方各发 LoverUpdate）
        if let Some(spouse_name) = state.spouse_name.clone() {
            if let Some(spouse_sid) = self.find_player_by_name(&spouse_name, msg.session_id).await {
                if let Some(spouse_record) = self.players.get(&spouse_sid) {
                    if let Ok(Some(spouse_state)) = spouse_record.ask(GetPlayerState).await {
                        // 上线者视角：对方（配偶）名字/日期/地图/天数
                        let spouse_map = self.map_title(spouse_state.map_index).await;
                        send_lover_update_packet(
                            &self.gate_ref,
                            msg.session_id,
                            &spouse_name,
                            state.married_date,
                            &spouse_map,
                            self.married_days(state.married_date),
                        );
                        // 对方视角：上线者信息 + 系统提示（C# player.ReceiveChat(PlayerHasComeOnline)）
                        let self_map = self.map_title(state.map_index).await;
                        send_lover_update_packet(
                            &self.gate_ref,
                            spouse_sid,
                            &state.name,
                            spouse_state.married_date,
                            &self_map,
                            self.married_days(spouse_state.married_date),
                        );
                        send_system_message(
                            &self.gate_ref,
                            spouse_sid,
                            &format!("你的配偶 {} 上线了", state.name),
                        );
                    }
                }
            } else {
                // 对方离线：名字 + 日期，地图空（C# GetRelationship 离线分支）
                send_lover_update_packet(
                    &self.gate_ref,
                    msg.session_id,
                    &spouse_name,
                    state.married_date,
                    "",
                    self.married_days(state.married_date),
                );
            }
        }
        debug!("SocialActor: player {} joined", msg.name);
    }
}

// ============================================================
// Message: SocialPlayerLeft
// ============================================================

impl Message<SocialPlayerLeft> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SocialPlayerLeft, _ctx: &mut Context<Self, Self::Reply>) {
        // 提前取师徒/配偶信息（随后从 players 移除）
        let leaving_mentor = if let Some(rec) = self.players.get(&msg.session_id) {
            match rec.ask(GetPlayerState).await {
                Ok(Some(s)) => Some((
                    s.name.clone(),
                    s.level,
                    s.mentor_name.clone(),
                    s.is_mentor,
                    s.mentee_exp,
                )),
                _ => None,
            }
        } else {
            None
        };
        // #1329：配偶下线通知（C# LogoutRelationship）
        let leaving_spouse = if let Some(rec) = self.players.get(&msg.session_id) {
            match rec.ask(GetPlayerState).await {
                Ok(Some(s)) => s
                    .spouse_name
                    .clone()
                    .map(|n| (s.name.clone(), n, s.married_date)),
                _ => None,
            }
        } else {
            None
        };

        self.players.remove(&msg.session_id);

        // 师徒下线通知 + MentorExp 转移（C# LogoutMentor：徒弟下线 → mentor.MentorExp += MenteeEXP）
        if let Some((name, level, Some(partner_name), is_mentor, mentee_exp)) = leaving_mentor {
            if let Some(partner_sid) = self
                .find_player_by_name(&partner_name, msg.session_id)
                .await
            {
                if let Some(partner_record) = self.players.get(&partner_sid) {
                    if let Ok(Some(partner_state)) = partner_record.ask(GetPlayerState).await {
                        let partner_mentor_exp = if !is_mentor && mentee_exp > 0 {
                            let updated = partner_state.mentor_exp + mentee_exp;
                            let _ = partner_record.ask(SetMentorExp { amount: updated }).await;
                            updated
                        } else {
                            partner_state.mentor_exp
                        };
                        send_mentor_update_packet(
                            &self.gate_ref,
                            partner_sid,
                            &name,
                            level as u32,
                            false,
                            partner_mentor_exp,
                        );
                        send_system_message(
                            &self.gate_ref,
                            partner_sid,
                            &format!("{} 下线了", name),
                        );
                    }
                }
            } else if !is_mentor && mentee_exp > 0 {
                // 导师离线：直接写库（C# CharacterInfo.MentorExp 持久化）
                let _ = crate::db::add_mentor_exp(&self.db_pool, &partner_name, mentee_exp).await;
            }
        }

        // 配偶下线通知：配偶在线 → LoverUpdate MapName=""（离线刷新）
        if let Some((name, spouse_name, married_date)) = leaving_spouse {
            if let Some(spouse_sid) = self.find_player_by_name(&spouse_name, msg.session_id).await {
                send_lover_update_packet(
                    &self.gate_ref,
                    spouse_sid,
                    &name,
                    married_date,
                    "",
                    self.married_days(married_date),
                );
            }
        }

        // 行会成员离线标记（保持 session 为空，行会广播/在线显示正确）
        for guild in self.guilds.values_mut() {
            if let Some(member) = guild
                .members
                .iter_mut()
                .find(|m| m.session_id == Some(msg.session_id))
            {
                member.session_id = None;
                debug!("SocialActor: guild member {} offline", member.name);
            }
        }

        // 处理组队：断线即离队（C# PlayerObject.LeaveGroup —— 断线不再保留离线成员，
        // 避免旧成员 group_id 残留导致无法再次组队）
        let mut group_to_remove: Option<u64> = None;
        let mut group_to_notify: Option<u64> = None;
        for (gid, group) in self.groups.iter_mut() {
            if group.remove_member(msg.session_id).is_some() {
                if group.member_count() == 0 {
                    group_to_remove = Some(*gid);
                } else {
                    group_to_notify = Some(*gid);
                }
                break;
            }
        }
        if let Some(gid) = group_to_remove {
            self.groups.remove(&gid);
            debug!("SocialActor: group #{} removed (all members offline)", gid);
        }
        if let Some(gid) = group_to_notify {
            self.broadcast_group_update(gid);
        }

        // 清理交易
        self.active_trades.retain(|_, trade| {
            trade.side_a.session_id != msg.session_id && trade.side_b.session_id != msg.session_id
        });

        // 清理邀请
        self.pending_invites
            .retain(|&k, &mut v| k != msg.session_id && v != msg.session_id);
        self.pending_guild_invites.remove(&msg.session_id);
        self.pending_marriage_invites
            .retain(|&k, &mut v| k != msg.session_id && v != msg.session_id);
        self.pending_mentor_invites
            .retain(|&k, &mut v| k != msg.session_id && v != msg.session_id);

        debug!("SocialActor: player left (session={})", msg.session_id);
    }
}

// ============================================================
// Message: SocialChatCommand
// ============================================================

impl Message<SocialChatCommand> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SocialChatCommand, _ctx: &mut Context<Self, Self::Reply>) {
        match msg.command.as_str() {
            "GROUPRECALL" => {
                self.handle_group_recall(msg.session_id).await;
            }
            "RECALLMEMBER" => {
                if !msg.args.is_empty() {
                    self.handle_recall_member(msg.session_id, &msg.args[0])
                        .await;
                }
            }
            "RECALLLOVER" => {
                self.handle_recall_lover(msg.session_id).await;
            }
            "ENABLEGROUPRECALL" => {
                if let Some(record) = self.players.get(&msg.session_id) {
                    let _ = record.ask(SetEnableGroupRecall { enable: true }).await;
                }
                send_system_message(&self.gate_ref, msg.session_id, "Group Recall Enabled.");
            }
            "DISABLEGROUPRECALL" => {
                if let Some(record) = self.players.get(&msg.session_id) {
                    let _ = record.ask(SetEnableGroupRecall { enable: false }).await;
                }
                send_system_message(&self.gate_ref, msg.session_id, "Group Recall Disabled.");
            }
            "RIDE" => {
                self.handle_toggle_ride(msg.session_id).await;
            }
            _ => warn!("SocialActor: unknown social command: {}", msg.command),
        }
    }
}

// ============================================================
// 组队系统 Handler
// ============================================================

impl Message<SwitchGroupRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SwitchGroupRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // C# PlayerObject.SwitchGroup：保存开关（CharacterInfo.AllowGroup 持久化）
        let mut new_state = state.clone();
        new_state.allow_group = msg.allow_group;
        let _ = record.ask(SetPlayerState { state: new_state }).await;

        if !msg.allow_group {
            // 禁止组队 → 离开当前组队
            if let Some(group_id) = state.group_id {
                self.leave_group(group_id, msg.session_id, &state.name)
                    .await;
            }
            send_system_message(&self.gate_ref, msg.session_id, "已关闭组队");
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "已开启组队");
        }
    }
}

impl Message<GroupInviteRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: GroupInviteRequest, _ctx: &mut Context<Self, Self::Reply>) {
        // #919：C# AddMember——NextGroupInviteTime 防刷（#2420：GroupInviteDelay 配置化）
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        if let Some(last) = self.last_group_invite.get(&msg.session_id).copied() {
            if now_ms - last < self.config.group_invite_delay_ms {
                send_system_message(&self.gate_ref, msg.session_id, "操作过于频繁，请稍后再试");
                return;
            }
        }
        self.last_group_invite.insert(msg.session_id, now_ms);

        let inviter_record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let inviter_state = match inviter_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // C# AddMember（:9278-9282）：邀请者必须是队长
        if let Some(gid) = inviter_state.group_id {
            if let Some(group) = self.groups.get(&gid) {
                if group.leader_session() != Some(msg.session_id) {
                    send_system_message(&self.gate_ref, msg.session_id, "只有队长可以邀请组队");
                    return;
                }
            }
        }
        // C# AddMember（:9290-9295）：邀请者所在地图 NoGroup → 不能邀请
        {
            let map_infos = self.config.map_infos.read().await;
            if let Some(mi) = map_infos.get(&(inviter_state.map_index as i32)) {
                if mi.no_group {
                    send_system_message(&self.gate_ref, msg.session_id, "当前地图无法邀请组队");
                    return;
                }
            }
        }
        // C# AddMember（:9304-9308）：不能邀请自己
        if msg.target_name.eq_ignore_ascii_case(&inviter_state.name) {
            send_system_message(&self.gate_ref, msg.session_id, "不能邀请自己");
            return;
        }

        let Some(target_session) = self
            .find_player_by_name(&msg.target_name, msg.session_id)
            .await
        else {
            send_system_message(&self.gate_ref, msg.session_id, "目标玩家不在线");
            return;
        };

        let target_record = match self.players.get(&target_session) {
            Some(r) => r,
            None => return,
        };

        let target_state = match target_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // C# AddMember（PlayerObject.cs ~9310）：目标关闭组队 → 拒绝
        // C# AddMember（:9316-9320）：目标已在任意队伍 → 拒绝
        if target_state.group_id.is_some() {
            send_system_message(&self.gate_ref, msg.session_id, "对方已在其他队伍中");
            return;
        }
        // C# AddMember（:9322-9326）：目标已有待处理邀请
        if self.pending_invites.contains_key(&target_session) {
            send_system_message(&self.gate_ref, msg.session_id, "对方已有待处理的组队邀请");
            return;
        }
        // C# AddMember（:9328-9333）：目标所在地图 NoGroup
        {
            let map_infos = self.config.map_infos.read().await;
            if let Some(mi) = map_infos.get(&(target_state.map_index as i32)) {
                if mi.no_group {
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        &format!("对方所在地图无法组队（{}）", target_state.name),
                    );
                    send_system_message(
                        &self.gate_ref,
                        target_session,
                        "对方无法接受组队邀请（当前地图禁止组队）",
                    );
                    return;
                }
            }
        }

        if !target_state.allow_group {
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                "对方未开启组队（请对方先开启允许组队）",
            );
            return;
        }

        // 检查是否已在同一组队
        if let (Some(g1), Some(g2)) = (inviter_state.group_id, target_state.group_id) {
            if g1 == g2 {
                send_system_message(&self.gate_ref, msg.session_id, "你们已在同一组队中");
                return;
            }
        }

        // #2028：C# GroupMaxMembers——邀请者所在组已满时提前拒绝（避免对方接受后 add_member 失败）
        if let Some(gid) = inviter_state.group_id {
            if let Some(group) = self.groups.get(&gid) {
                if group.member_count() >= crate::actors::group::MAX_GROUP_SIZE {
                    send_system_message(&self.gate_ref, msg.session_id, "你的队伍已满，无法再邀请");
                    return;
                }
            }
        }

        // 发送邀请给目标玩家
        send_group_invite_packet(
            &self.gate_ref,
            target_session,
            &inviter_state.name,
            msg.session_id,
        );
        // 记录待处理邀请
        self.pending_invites.insert(target_session, msg.session_id);
        debug!(
            "Group invite: {} -> {}",
            inviter_state.name, target_state.name
        );
    }
}

impl Message<GroupInviteReply> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: GroupInviteReply, _ctx: &mut Context<Self, Self::Reply>) {
        // 解析邀请者 ID
        let inviter_id = self
            .pending_invites
            .remove(&msg.session_id)
            .unwrap_or(msg.inviter_id);

        if !msg.accept {
            send_system_message(&self.gate_ref, inviter_id, "对方拒绝了组队邀请");
            return;
        }

        // 获取邀请者状态
        let inviter_record = match self.players.get(&inviter_id) {
            Some(r) => r,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "邀请者已离线");
                return;
            }
        };

        let inviter_state = match inviter_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // C# GroupInviteReply（PlayerObject.cs ~9407）：邀请方关闭组队 → 拒绝
        if !inviter_state.allow_group {
            send_system_message(&self.gate_ref, msg.session_id, "邀请者已关闭组队");
            return;
        }

        // C# GroupInvite（:9387-9392）：接受者已在队伍 → 拒绝
        {
            let record = match self.players.get(&msg.session_id) {
                Some(r) => r,
                None => return,
            };
            if let Ok(Some(rs)) = record.ask(GetPlayerState).await {
                if rs.group_id.is_some() {
                    send_system_message(&self.gate_ref, msg.session_id, "你已在队伍中");
                    return;
                }
            }
        }
        // C# GroupInvite（:9394-9399）：邀请者不再是队长 → 拒绝
        if let Some(gid) = inviter_state.group_id {
            if let Some(group) = self.groups.get(&gid) {
                if group.leader_session() != Some(inviter_id) {
                    send_system_message(&self.gate_ref, msg.session_id, "邀请者不再是队长");
                    return;
                }
            }
        }
        // C# GroupInvite（:9420-9426）：邀请者所在地图 NoGroup → 双方拒绝
        {
            let map_infos = self.config.map_infos.read().await;
            if let Some(mi) = map_infos.get(&(inviter_state.map_index as i32)) {
                if mi.no_group {
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        &format!("对方所在地图无法组队（{}）", inviter_state.name),
                    );
                    send_system_message(
                        &self.gate_ref,
                        inviter_id,
                        "对方无法接受组队邀请（当前地图禁止组队）",
                    );
                    return;
                }
            }
        }

        // 邀请者接受：将回复者加入邀请者的组队
        let reply_name = {
            let record = match self.players.get(&msg.session_id) {
                Some(r) => r,
                None => return,
            };
            match record.ask(GetPlayerState).await {
                Ok(Some(s)) => s.name.clone(),
                _ => return,
            }
        };

        self.join_or_create_group(msg.session_id, inviter_id, &reply_name)
            .await;
    }
}

impl Message<DellMemberRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: DellMemberRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let group_id = match state.group_id {
            Some(g) => g,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "你不在任何组队中");
                return;
            }
        };

        // 检查是否是队长
        let is_leader = {
            match self.groups.get(&group_id) {
                Some(g) => g.leader_session() == Some(msg.session_id),
                None => {
                    send_system_message(&self.gate_ref, msg.session_id, "组队不存在");
                    return;
                }
            }
        };

        if !is_leader {
            send_system_message(&self.gate_ref, msg.session_id, "只有队长可以踢出成员");
            return;
        }

        // 通过名称查找成员的 session_id
        let member_session = {
            let group = match self.groups.get(&group_id) {
                Some(g) => g,
                None => return,
            };
            group
                .members
                .iter()
                .find(|m| m.name == msg.member_name)
                .map(|m| m.session_id)
        };

        let Some(member_session) = member_session else {
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                &format!("找不到名为 '{}' 的队员", msg.member_name),
            );
            return;
        };

        // 踢出成员（C# DelMember：先给被踢者 S.DeleteGroup，再 LeaveGroup）
        if let Some(group) = self.groups.get_mut(&group_id) {
            if group.remove_member(member_session).is_some() {
                // 更新被踢出玩家的 group_id
                if let Some(target_record) = self.players.get(&member_session) {
                    let _ = target_record.ask(SetGroupId { group_id: None }).await;
                }
                // 被踢玩家清除组队 UI（C# S.DeleteGroup）
                send_delete_group_packet(&self.gate_ref, member_session);
                send_system_message(&self.gate_ref, member_session, "你已被移出队伍");

                debug!("Kicked {} from group #{}", msg.member_name, group_id);
                send_system_message(
                    &self.gate_ref,
                    msg.session_id,
                    &format!("{} 已被踢出队伍", msg.member_name),
                );

                // 组队空了 → 删除；只剩 1 人 → 解散（C# LeaveGroup 语义）
                if group.member_count() == 0 {
                    self.groups.remove(&group_id);
                } else if group.member_count() == 1 {
                    let last = group.members[0].session_id;
                    send_delete_group_packet(&self.gate_ref, last);
                    if let Some(record) = self.players.get(&last) {
                        let _ = record.ask(SetGroupId { group_id: None }).await;
                    }
                    self.groups.remove(&group_id);
                } else {
                    self.broadcast_group_update(group_id);
                }
            }
        }
    }
}

// ============================================================
// 交易系统 Handler
// ============================================================

impl Message<TradeStartRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: TradeStartRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // #919：C# StartTrade——NextTradeTime 防刷（#2420：TradeDelay 配置化）
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        if let Some(last) = self.last_trade_request.get(&msg.session_id).copied() {
            if now_ms - last < self.config.trade_delay_ms {
                send_system_message(&self.gate_ref, msg.session_id, "操作过于频繁，请稍后再试");
                return;
            }
        }
        self.last_trade_request.insert(msg.session_id, now_ms);

        // 检查是否已有交易
        if self.active_trades.contains_key(&msg.session_id) {
            send_system_message(&self.gate_ref, msg.session_id, "你已经在交易中");
            return;
        }

        // 检查是否死亡（对应 C# player.Dead || Dead）
        if state.is_dead {
            send_system_message(&self.gate_ref, msg.session_id, "死亡状态下无法交易");
            return;
        }

        // 发送交易请求给附近的玩家（距离校验）
        let player_pos = (state.x, state.y);

        let mut nearest_target: Option<(u64, i32)> = None;
        let mut found_trade_closed = false;
        for sid in self.players.keys() {
            if *sid == msg.session_id {
                continue;
            }
            if self.active_trades.contains_key(sid) {
                continue; // 对方已在交易中
            }
            if let Some(rec) = self.players.get(sid) {
                if let Ok(Some(other_state)) = rec.ask(GetPlayerState).await {
                    if other_state.is_dead {
                        continue;
                    } // 死亡状态下无法交易
                      // #911：C# PlayerObject（~10686）目标关闭交易（@ALLOWTRADE）→ 拒绝
                    if !other_state.allow_trade {
                        found_trade_closed = true;
                        continue;
                    }
                    // C# TradeRequest：目标必须在前方一格（PointMove 1）且双方面对面（10634-10666）
                    let (fx, fy) = front_tile(player_pos.0, player_pos.1, state.direction);
                    if other_state.x == fx
                        && other_state.y == fy
                        && facing_each_other(
                            state.direction,
                            player_pos.0,
                            player_pos.1,
                            other_state.direction,
                            other_state.x,
                            other_state.y,
                        )
                    {
                        nearest_target = Some((*sid, 1));
                        break;
                    }
                }
            }
        }

        if let Some((target, _dist)) = nearest_target {
            // 记录待处理交易请求
            // C# TradeRequest（:10680-10684）：目标已有待处理交易邀请 → 拒绝
            if self.pending_invites.contains_key(&target) {
                send_system_message(&self.gate_ref, msg.session_id, "对方已有待处理的交易请求");
                return;
            }
            self.pending_invites.insert(target, msg.session_id);
            send_trade_invite_packet(&self.gate_ref, target, &state.name);
            debug!(
                "Trade request: {} -> session {} (dist={})",
                state.name, target, _dist
            );
        } else if found_trade_closed {
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                "附近玩家关闭了交易（请对方先使用 @ALLOWTRADE 开启）",
            );
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "附近没有其他玩家");
        }
    }
}

impl Message<TradeStartReply> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: TradeStartReply, _ctx: &mut Context<Self, Self::Reply>) {
        // 解析发起者
        let initiator_id = self.pending_invites.remove(&msg.session_id).or_else(|| {
            self.active_trades.get(&msg.session_id).map(|t| {
                if t.side_a.session_id == msg.session_id {
                    t.side_b.session_id
                } else {
                    t.side_a.session_id
                }
            })
        });

        let Some(initiator_id) = initiator_id else {
            return;
        };

        if !msg.accept {
            send_system_message(&self.gate_ref, initiator_id, "对方拒绝了交易请求");
            return;
        }

        // 创建交易会话
        // C# TradeReply（:10723-10735）：接受者/邀请者任一已在交易 → 拒绝
        if self.active_trades.contains_key(&msg.session_id) {
            send_system_message(&self.gate_ref, msg.session_id, "你已经在交易中");
            return;
        }
        if self.active_trades.contains_key(&initiator_id) {
            send_system_message(&self.gate_ref, msg.session_id, "对方已在交易中");
            return;
        }

        let initiator_record = match self.players.get(&initiator_id) {
            Some(r) => r,
            None => return,
        };
        let initiator_name = match initiator_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s.name.clone(),
            _ => return,
        };
        let target_record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let target_name = match target_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s.name.clone(),
            _ => return,
        };

        let trade = TradeSession::new(
            initiator_id,
            initiator_name.clone(),
            msg.session_id,
            target_name.clone(),
        );
        self.active_trades.insert(initiator_id, trade);

        // 通知双方打开交易窗口
        send_trade_open_packet(&self.gate_ref, initiator_id, &target_name);
        send_trade_open_packet(&self.gate_ref, msg.session_id, &initiator_name);
        debug!(
            "Trade session created: {} <-> {}",
            initiator_name, target_name
        );
    }
}

impl Message<TradeAddGold> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: TradeAddGold, _ctx: &mut Context<Self, Self::Reply>) {
        // C# PlayerObject.TradeGold（PlayerObject.cs:10744-10760）：
        // - amount<1 拒绝（:10750）
        // - TradeGoldAmount **累计**（:10755 +=，客户端 my_gold += n 同语义）
        // - 余额按已押+本次校验（C# 押金即时扣款故按剩余额查；本服押金
        //   成交时统一结算，故改为已押总额+本次 ≤ 持有）
        if msg.amount < 1 {
            return;
        }

        // 先读已押金币与余额（ask 跨 await 不能持有交易借用，与原注释同理）
        let (committed, has_enough_gold) = {
            let committed = match self.find_trade_mut(msg.session_id) {
                Some(t) => match t.side_of_mut(msg.session_id) {
                    Some(s) => s.gold,
                    None => return,
                },
                None => {
                    send_system_message(&self.gate_ref, msg.session_id, "你不在交易中");
                    return;
                }
            };
            let record = match self.players.get(&msg.session_id) {
                Some(r) => r,
                None => return,
            };
            match record.ask(GetPlayerState).await {
                Ok(Some(s)) => (committed, s.inventory.gold >= committed + msg.amount as u64),
                _ => return,
            }
        };
        if !has_enough_gold {
            send_system_message(&self.gate_ref, msg.session_id, "金币不足");
            return;
        }

        let trade = match self.find_trade_mut(msg.session_id) {
            Some(t) => t,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "你不在交易中");
                return;
            }
        };

        let side = match trade.side_of_mut(msg.session_id) {
            Some(s) => s,
            None => return,
        };
        side.gold = committed + msg.amount as u64;
        side.unlock();

        // 对方看到**累计总额**（C# :10759 S.TradeGold{Amount=TradeGoldAmount}）
        let total = side.gold;
        let other_session = trade.other_session(msg.session_id);
        if let Some(other) = other_session {
            send_trade_gold_update_packet(&self.gate_ref, other, msg.session_id, total);
        }
    }
}

impl Message<TradeConfirmLock> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: TradeConfirmLock, _ctx: &mut Context<Self, Self::Reply>) {
        let trade = match self.find_trade_mut(msg.session_id) {
            Some(t) => t,
            None => return,
        };

        let side = match trade.side_of_mut(msg.session_id) {
            Some(s) => s,
            None => return,
        };
        side.locked = msg.locked;

        let both_locked = trade.both_locked();
        let (s1, s2) = trade.participant_sessions();
        let side_a = trade.side_a.clone();
        let side_b = trade.side_b.clone();

        // 通知双方确认状态
        send_trade_confirm_packet(&self.gate_ref, s1, &side_a, &side_b);
        send_trade_confirm_packet(&self.gate_ref, s2, &side_a, &side_b);

        // 双方都锁定 -> 执行交易
        if both_locked {
            self.execute_trade(msg.session_id).await;
        }
    }
}

impl Message<TradeCancel> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: TradeCancel, _ctx: &mut Context<Self, Self::Reply>) {
        let trade = match self.find_trade(msg.session_id) {
            Some(t) => t.clone(),
            None => return,
        };

        let (s1, s2) = trade.participant_sessions();
        self.active_trades.remove(&trade.side_a.session_id);

        // 通知双方
        send_trade_cancel_packet(&self.gate_ref, s1);
        send_trade_cancel_packet(&self.gate_ref, s2);
        debug!("Trade cancelled: session {}", s1);
    }
}

impl Message<TradeAddItem> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: TradeAddItem, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };

        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        // C# CanTradeItem：BindMode.DontTrade(0x10) 物品不可交易（含租赁绑定）
        {
            let infos = self.config.item_infos.read().await;
            let bind = state
                .inventory
                .get_item(msg.unique_id)
                .and_then(|it| infos.get(&it.item_index).map(|i| i.bind_mode))
                .unwrap_or(0);
            let rental_dont_trade = state
                .inventory
                .get_item(msg.unique_id)
                .map(|it| {
                    crate::actors::world::rental_has_flag(
                        it,
                        mir2_shared::enums::BindMode::DONT_TRADE.bits(),
                    )
                })
                .unwrap_or(false);
            if (bind & 0x0010) != 0 || rental_dont_trade {
                send_system_message(&self.gate_ref, msg.session_id, "该物品无法交易");
                return;
            }
        }

        // C# CharacterInfo.Trade = new UserItem[10]：交易槽位上限 10
        if msg.grid >= 10 {
            send_system_message(&self.gate_ref, msg.session_id, "无效的交易槽位");
            return;
        }
        {
            let trade_full = match self.find_trade(msg.session_id) {
                Some(t) => t
                    .side_of(msg.session_id)
                    .map(|side| !side.can_add_item(msg.unique_id))
                    .unwrap_or(false),
                None => false,
            };
            if trade_full {
                send_system_message(&self.gate_ref, msg.session_id, "交易物品已满（最多 10 件）");
                return;
            }
        }

        // #923：C# TradeItem——放入交易即从背包移除并锁定（防交易中消耗/重复放入）
        // #2006：部分堆叠按数量拆分（C# 原堆扣减 count、剩余保留）；全叠/非堆叠整件移除
        let full_count = state
            .inventory
            .get_item(msg.unique_id)
            .map(|it| it.count)
            .unwrap_or(0);
        let removed = if msg.count > 0 && msg.count < full_count {
            record
                .ask(crate::actors::player::RemoveItemFromInventoryCount {
                    unique_id: msg.unique_id,
                    count: msg.count,
                })
                .await
                .ok()
                .flatten()
        } else {
            record
                .ask(RemoveItemFromInventory {
                    unique_id: msg.unique_id,
                })
                .await
                .ok()
                .flatten()
        };
        let Some(item_data) = removed else {
            send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
            return;
        };
        // #2010：整堆/非堆叠放入时 count 取物品实际堆叠数（C# TradeItem 整件移动）
        let eff_count = if msg.count > 0 && msg.count < full_count {
            msg.count
        } else {
            item_data.count
        };

        // 交易不存在/已锁定 → 回滚归还
        let other_session = {
            let trade = match self.find_trade_mut(msg.session_id) {
                Some(t) => t,
                None => {
                    let _ = record.ask(AddItemToInventory { item: item_data }).await;
                    return;
                }
            };
            let side = match trade.side_of_mut(msg.session_id) {
                Some(s) => s,
                None => {
                    let _ = record.ask(AddItemToInventory { item: item_data }).await;
                    return;
                }
            };
            if side.locked {
                let _ = record.ask(AddItemToInventory { item: item_data }).await;
                return;
            }
            side.add_item(msg.unique_id, msg.grid, eff_count, Some(item_data));
            side.unlock();
            trade.other_session(msg.session_id)
        };

        // 通知对方
        if let Some(other) = other_session {
            send_trade_item_update_packet(
                &self.gate_ref,
                other,
                msg.unique_id,
                msg.grid,
                eff_count,
                true,
            );
        }
    }
}

impl Message<TradeRemoveItem> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: TradeRemoveItem, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };

        // #2010：C# RetrieveTradeItem 语义——移除即放回背包；锁定后不可改动（与 TradeAddItem 一致）
        let removed = {
            let trade = match self.find_trade_mut(msg.session_id) {
                Some(t) => t,
                None => return,
            };
            let side = match trade.side_of_mut(msg.session_id) {
                Some(s) => s,
                None => return,
            };
            if side.locked {
                return;
            }
            let removed = side.remove_item(msg.unique_id);
            side.unlock();
            removed
        };

        // 归还背包（整叠/拆分部分均回 item_data，避免物品销毁）
        if let Some(item_data) = removed.and_then(|ti| ti.item_data) {
            let ok = record
                .ask(AddItemToInventory { item: item_data })
                .await
                .unwrap_or(false);
            if !ok {
                debug!("TradeRemoveItem: 归还背包失败 session={}", msg.session_id);
            }
        }

        // 通知对方
        let trade = match self.find_trade_mut(msg.session_id) {
            Some(t) => t,
            None => return,
        };
        if let Some(other) = trade.other_session(msg.session_id) {
            send_trade_item_update_packet(&self.gate_ref, other, msg.unique_id, 0, 0, false);
        }
    }
}

impl Message<DepositTradeItemBySlot> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: DepositTradeItemBySlot, _ctx: &mut Context<Self, Self::Reply>) {
        // Resolve from_slot → unique_id from player inventory
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let slot = msg.from_slot as usize;
        let slot_data = state.inventory.backpack.get(slot).and_then(|s| s.as_ref());
        let uid = match slot_data {
            Some(s) => s.item.unique_id,
            None => {
                send_deposit_trade_item_packet(
                    &self.gate_ref,
                    msg.session_id,
                    msg.from_slot,
                    false,
                );
                return;
            }
        };

        // #2010：C# DepositTradeItem（10545-10553）——BindMode.DontTrade(0x10) 绑定物品不可放入交易（含租赁绑定）
        {
            let infos = self.config.item_infos.read().await;
            let bind = state
                .inventory
                .get_item(uid)
                .and_then(|it| infos.get(&it.item_index).map(|i| i.bind_mode))
                .unwrap_or(0);
            let rental_dont_trade = state
                .inventory
                .get_item(uid)
                .map(|it| {
                    crate::actors::world::rental_has_flag(
                        it,
                        mir2_shared::enums::BindMode::DONT_TRADE.bits(),
                    )
                })
                .unwrap_or(false);
            if (bind & 0x0010) != 0 || rental_dont_trade {
                send_system_message(&self.gate_ref, msg.session_id, "该物品无法交易");
                send_deposit_trade_item_packet(
                    &self.gate_ref,
                    msg.session_id,
                    msg.from_slot,
                    false,
                );
                return;
            }
        }

        // Check trade exists and not locked
        {
            let trade = match self.find_trade_mut(msg.session_id) {
                Some(t) => t,
                None => {
                    send_deposit_trade_item_packet(
                        &self.gate_ref,
                        msg.session_id,
                        msg.from_slot,
                        false,
                    );
                    return;
                }
            };
            let side = match trade.side_of_mut(msg.session_id) {
                Some(s) => s,
                None => return,
            };
            if side.locked {
                send_deposit_trade_item_packet(
                    &self.gate_ref,
                    msg.session_id,
                    msg.from_slot,
                    false,
                );
                return;
            }
        }

        // Remove item from player inventory
        let removed = record
            .ask(RemoveItemFromInventory { unique_id: uid })
            .await
            .ok()
            .flatten();
        let item_data = removed.clone();
        // #2010：C# 整格移动整叠（Info.Trade[to]=temp），count 取物品实际堆叠数
        let item_count = item_data.as_ref().map(|it| it.count).unwrap_or(1);

        // Add to trade side
        let other_session = {
            let trade = match self.find_trade_mut(msg.session_id) {
                Some(t) => t,
                None => {
                    // Rollback: return item to player
                    if let Some(item) = removed {
                        let _ = record.ask(AddItemToInventory { item }).await;
                    }
                    send_deposit_trade_item_packet(
                        &self.gate_ref,
                        msg.session_id,
                        msg.from_slot,
                        false,
                    );
                    return;
                }
            };
            let side = match trade.side_of_mut(msg.session_id) {
                Some(s) => s,
                None => {
                    if let Some(item) = removed {
                        let _ = record.ask(AddItemToInventory { item }).await;
                    }
                    return;
                }
            };
            side.add_item(uid, msg.to_slot as u8, item_count, item_data);
            side.unlock();
            trade.other_session(msg.session_id)
        };

        if let Some(other) = other_session {
            send_trade_item_update_packet(
                &self.gate_ref,
                other,
                uid,
                msg.to_slot as u8,
                item_count,
                true,
            );
        }
        send_deposit_trade_item_packet(&self.gate_ref, msg.session_id, msg.from_slot, true);
    }
}

impl Message<RetrieveTradeItemBySlot> for SocialActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: RetrieveTradeItemBySlot,
        _ctx: &mut Context<Self, Self::Reply>,
    ) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };

        // Find trade item by grid slot and extract
        let removed_trade_item = {
            let trade = match self.find_trade_mut(msg.session_id) {
                Some(t) => t,
                None => {
                    send_retrieve_trade_item_packet(
                        &self.gate_ref,
                        msg.session_id,
                        msg.from_slot,
                        false,
                    );
                    return;
                }
            };
            let side = match trade.side_of_mut(msg.session_id) {
                Some(s) => s,
                None => {
                    send_retrieve_trade_item_packet(
                        &self.gate_ref,
                        msg.session_id,
                        msg.from_slot,
                        false,
                    );
                    return;
                }
            };
            if side.locked {
                send_retrieve_trade_item_packet(
                    &self.gate_ref,
                    msg.session_id,
                    msg.from_slot,
                    false,
                );
                return;
            }
            let uid = side
                .items
                .iter()
                .find(|i| i.grid == msg.from_slot as u8)
                .map(|i| i.uid);
            match uid {
                Some(uid) => {
                    let removed = side.remove_item(uid);
                    side.unlock();
                    removed
                }
                None => {
                    send_retrieve_trade_item_packet(
                        &self.gate_ref,
                        msg.session_id,
                        msg.from_slot,
                        false,
                    );
                    return;
                }
            }
        };

        // Add item back to player inventory
        if let Some(trade_item) = &removed_trade_item {
            if let Some(item_data) = &trade_item.item_data {
                let _ = record
                    .ask(AddItemToInventory {
                        item: item_data.clone(),
                    })
                    .await;
            }
        }

        // Notify other party
        if let Some(trade_item) = &removed_trade_item {
            let trade = match self.find_trade(msg.session_id) {
                Some(t) => t,
                None => {
                    return;
                }
            };
            if let Some(other) = trade.other_session(msg.session_id) {
                send_trade_item_update_packet(&self.gate_ref, other, trade_item.uid, 0, 0, false);
            }
        }
        send_retrieve_trade_item_packet(&self.gate_ref, msg.session_id, msg.from_slot, true);
    }
}

// ============================================================
// 好友系统 Handler
// ============================================================

impl Message<AddFriendRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: AddFriendRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if msg.friend_name == state.name {
            send_system_message(&self.gate_ref, msg.session_id, "不能添加自己为好友");
            return;
        }

        // 遍历所有玩家查找匹配名称（忽略大小写，对齐 C# Envir.GetPlayer）
        let mut found: Option<(u64, u32, String)> = None;
        for (sid, r) in &self.players {
            if let Ok(Some(s)) = r.ask(GetPlayerState).await {
                if s.name.eq_ignore_ascii_case(&msg.friend_name) {
                    found = Some((*sid, s.object_id, s.name));
                    break;
                }
            }
        }

        // 在线未命中 → C# AddFriend（PlayerObject.cs:12428）：Envir.GetCharacterInfo(name) 全局角色信息，离线也可添加
        let (target_session, target_oid, target_name) = if let Some(f) = found {
            f
        } else {
            let canonical = match db::find_character_name(&self.db_pool, &msg.friend_name).await {
                Ok(Some(n)) => n,
                _ => {
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        &format!("找不到名为 '{}' 的玩家", msg.friend_name),
                    );
                    return;
                }
            };
            // 离线条目：用规范名 FNV-1a 哈希作稳定 object_id（friend_id_from_name），
            // 上线后由 SocialPlayerJoined 校正为运行时 ID
            let synthetic_id = crate::actors::friend::friend_id_from_name(&canonical);
            (0u64, synthetic_id, canonical)
        };

        // 检查是否已在黑名单（按名字忽略大小写，防离线/在线双份）
        if msg.blocked {
            let record = match self.players.get(&msg.session_id) {
                Some(r) => r,
                None => return,
            };
            let mut state = match record.ask(GetPlayerState).await {
                Ok(Some(s)) => s,
                _ => return,
            };
            if state.friend_list.is_blocked_name(&target_name)
                || state.friend_list.is_blocked(target_oid)
            {
                send_system_message(
                    &self.gate_ref,
                    msg.session_id,
                    &format!("{} 已在黑名单", target_name),
                );
                return;
            }
            state
                .friend_list
                .add_blocked(target_oid, target_name.clone());
            let _ = record.ask(SetPlayerState { state }).await;
            self.send_friends_list(msg.session_id).await;
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                &format!("已将 {} 加入黑名单", target_name),
            );
            return;
        }

        // 检查是否已是好友（按名字忽略大小写，防离线/在线双份）
        let is_already_friend = {
            let record = match self.players.get(&msg.session_id) {
                Some(r) => r,
                None => return,
            };
            match record.ask(GetPlayerState).await {
                Ok(Some(s)) => {
                    s.friend_list.is_friend_name(&target_name)
                        || s.friend_list.is_friend(target_oid)
                }
                _ => return,
            }
        };
        if is_already_friend {
            send_system_message(&self.gate_ref, msg.session_id, "已是你的好友");
            return;
        }

        // 添加好友（在线目标双方互相添加；离线目标仅添加自己——C# AddFriend 本身只加自己）
        {
            let record = match self.players.get(&msg.session_id) {
                Some(r) => r,
                None => return,
            };
            let _ = record
                .ask(AddFriendToSelf {
                    friend_oid: target_oid,
                    friend_name: target_name.clone(),
                })
                .await;
        }
        if target_session != 0 {
            let target_r = match self.players.get(&target_session) {
                Some(r) => r,
                None => return,
            };
            let _ = target_r
                .ask(AddFriendToSelf {
                    friend_oid: state.object_id,
                    friend_name: state.name.clone(),
                })
                .await;
        }

        // 通知（离线添加只通知自己）
        self.send_friends_list(msg.session_id).await;
        if target_session != 0 {
            self.send_friends_list(target_session).await;
        }

        send_system_message(
            &self.gate_ref,
            msg.session_id,
            &format!("已将 {} 添加为好友", target_name),
        );
    }
}

impl Message<RemoveFriendRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: RemoveFriendRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let success = match record
            .ask(RemoveFriendFromSelf {
                friend_oid: msg.friend_object_id,
            })
            .await
        {
            Ok(s) => s,
            _ => return,
        };

        // #1303：好友/黑名单通用删除（先试好友，再试黑名单）
        let success = if success {
            true
        } else {
            let mut st = match record.ask(GetPlayerState).await {
                Ok(Some(s)) => s,
                _ => return,
            };
            let ok = st.friend_list.remove_blocked(msg.friend_object_id);
            if ok {
                let _ = record.ask(SetPlayerState { state: st }).await;
            }
            ok
        };

        if success {
            send_system_message(&self.gate_ref, msg.session_id, "已移除");
            self.send_friends_list(msg.session_id).await;
        }
    }
}

impl Message<RefreshFriendsRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: RefreshFriendsRequest, _ctx: &mut Context<Self, Self::Reply>) {
        self.send_friends_list(msg.session_id).await;
    }
}

impl Message<AddMemoRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: AddMemoRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let success = match record
            .ask(SetFriendMemo {
                friend_oid: msg.friend_object_id,
                memo: msg.memo.clone(),
            })
            .await
        {
            Ok(s) => s,
            _ => return,
        };

        if success {
            send_system_message(&self.gate_ref, msg.session_id, "备注已更新");
            self.send_friends_list(msg.session_id).await;
        }
    }
}

// ============================================================
// 行会系统 Handler
// ============================================================

impl Message<CreateGuildRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: CreateGuildRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查是否已在行会
        if state.guild_name.is_some() {
            send_system_message(&self.gate_ref, msg.session_id, "你已经有行会了");
            return;
        }

        // 等级检查（对应 C# Info.Level < Settings.Guild_RequiredLevel）
        if state.level < self.config.guild_required_level {
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                &format!(
                    "等级不足，创建行会需要 {} 级",
                    self.config.guild_required_level
                ),
            );
            return;
        }

        // 名称唯一性检查
        if self.guilds.contains_key(&msg.guild_name) {
            send_system_message(&self.gate_ref, msg.session_id, "行会名称已存在");
            return;
        }

        // 新手行会名称限制（对应 C# !Info.AccountInfo.AdminAccount && guildName == Settings.NewbieGuild）
        if !state.is_gm
            && msg
                .guild_name
                .eq_ignore_ascii_case(&self.config.newbie_guild)
        {
            send_system_message(&self.gate_ref, msg.session_id, "不能创建该名称的行会");
            return;
        }

        // 名称为空检查
        if msg.guild_name.trim().is_empty() || msg.guild_name.len() > 20 {
            send_system_message(&self.gate_ref, msg.session_id, "行会名称无效");
            return;
        }

        // #2412：建会混合消耗（C# Guild_CreationCostList：[Required-i] 金币/物品；非 GM 才校验/消耗）
        if !state.is_gm {
            if !self.config.guild_creation_costs.is_empty() {
                // 逐项校验
                for cost in &self.config.guild_creation_costs {
                    match &cost.item_name {
                        None => {
                            if state.inventory.gold < cost.amount as u64 {
                                send_system_message(
                                    &self.gate_ref,
                                    msg.session_id,
                                    &format!("金币不足，创建行会需要 {} 金币", cost.amount),
                                );
                                return;
                            }
                        }
                        Some(name) => {
                            let idx = {
                                let infos = self.config.item_infos.read().await;
                                infos
                                    .values()
                                    .find(|i| i.name.eq_ignore_ascii_case(name))
                                    .map(|i| i.index)
                            };
                            let Some(idx) = idx else {
                                send_system_message(
                                    &self.gate_ref,
                                    msg.session_id,
                                    &format!("缺少建会材料：{}", name),
                                );
                                return;
                            };
                            let count = record
                                .ask(crate::actors::player::CountItemsByIndex { item_index: idx })
                                .await
                                .unwrap_or(0);
                            if (count as u32) < cost.amount {
                                send_system_message(
                                    &self.gate_ref,
                                    msg.session_id,
                                    &format!("缺少建会材料：{} ×{}", name, cost.amount),
                                );
                                return;
                            }
                        }
                    }
                }
                // 消耗
                for cost in &self.config.guild_creation_costs {
                    match &cost.item_name {
                        None => {
                            let _ = record
                                .ask(crate::actors::player::DeductGold {
                                    amount: cost.amount as u64,
                                })
                                .await;
                        }
                        Some(name) => {
                            let infos = self.config.item_infos.read().await;
                            if let Some(i) =
                                infos.values().find(|i| i.name.eq_ignore_ascii_case(name))
                            {
                                let _ = record
                                    .ask(crate::actors::player::ConsumeItemsByIndex {
                                        item_index: i.index,
                                        count: cost.amount.min(u16::MAX as u32) as u16,
                                    })
                                    .await;
                            }
                        }
                    }
                }
            } else {
                // 兼容：无 Required 列表时回退金币仅（guild_creation_cost_gold）
                if state.inventory.gold < self.config.guild_creation_cost_gold {
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        &format!(
                            "金币不足，创建行会需要 {} 金币",
                            self.config.guild_creation_cost_gold
                        ),
                    );
                    return;
                }
                let _ = record
                    .ask(DeductGold {
                        amount: self.config.guild_creation_cost_gold,
                    })
                    .await;
            }
        }

        let mut guild = Guild::new(msg.guild_name.clone(), state.name.clone(), msg.session_id);
        // #2406：C# GuildInfo 构造——MaxExperience = Guild_ExperienceList[Level]（Level=1 → index1）；
        // 上限从 MembercapList 读取（数据占位值 <5 时回退 50 保护）
        guild.max_experience = self
            .config
            .guild_experience_list
            .get(1)
            .copied()
            .unwrap_or(0);
        let cap = self
            .config
            .guild_membercap_list
            .get(1)
            .copied()
            .unwrap_or(50);
        guild.member_cap = if cap >= 5 { cap } else { 50 };
        self.guilds.insert(msg.guild_name.clone(), guild);

        // 保存行会到数据库
        if let Some(guild) = self.guilds.get(&msg.guild_name) {
            if let Err(e) = db::save_guild(&self.db_pool, guild).await {
                warn!("Failed to save guild '{}' to DB: {}", msg.guild_name, e);
            }
        }

        // 更新玩家行会信息
        let _ = record
            .ask(SetGuildInfo {
                guild_name: Some(msg.guild_name.clone()),
                rank: GuildRank::Leader,
            })
            .await;

        send_system_message(
            &self.gate_ref,
            msg.session_id,
            &format!("行会 \"{}\" 已创建", msg.guild_name),
        );
        // 发送完整行会信息（客户端据此显示行会对话框）
        if let Some(guild) = self.guilds.get(&msg.guild_name) {
            send_guild_info_packet(&self.gate_ref, msg.session_id, guild);
        } else {
            send_guild_status_packet(&self.gate_ref, msg.session_id, true);
        }
        debug!("Guild created: {} by {}", msg.guild_name, state.name);
    }
}

impl Message<GmCreateGuildRequest> for SocialActor {
    type Reply = ();
    async fn handle(&mut self, msg: GmCreateGuildRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 已在行会则拒绝（C# CREATEGUILD：PlayerAlreadyInGuild）
        if state.guild_name.is_some() {
            send_system_message(&self.gate_ref, msg.session_id, "该玩家已经有行会了");
            return;
        }
        // 名称长度限制（C#：gName.Length 3-20）
        if msg.guild_name.trim().is_empty() || msg.guild_name.len() < 3 || msg.guild_name.len() > 20
        {
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                "行会名称长度需为 3-20 个字符",
            );
            return;
        }
        // 名称唯一性检查
        if self.guilds.contains_key(&msg.guild_name) {
            send_system_message(&self.gate_ref, msg.session_id, "行会名称已存在");
            return;
        }

        // GM 直接建会：跳过等级/金币/新手行会限制（C# CREATEGUILD：仅 GM/TestServer 可用）
        let mut guild = Guild::new(msg.guild_name.clone(), state.name.clone(), msg.session_id);
        // #2406：C# GuildInfo 构造——MaxExperience = Guild_ExperienceList[Level]（Level=1 → index1）；
        // 上限从 MembercapList 读取（数据占位值 <5 时回退 50 保护）
        guild.max_experience = self
            .config
            .guild_experience_list
            .get(1)
            .copied()
            .unwrap_or(0);
        let cap = self
            .config
            .guild_membercap_list
            .get(1)
            .copied()
            .unwrap_or(50);
        guild.member_cap = if cap >= 5 { cap } else { 50 };
        self.guilds.insert(msg.guild_name.clone(), guild);
        if let Some(guild) = self.guilds.get(&msg.guild_name) {
            if let Err(e) = db::save_guild(&self.db_pool, guild).await {
                warn!("Failed to save guild '{}' to DB: {}", msg.guild_name, e);
            }
        }
        let _ = record
            .ask(SetGuildInfo {
                guild_name: Some(msg.guild_name.clone()),
                rank: GuildRank::Leader,
            })
            .await;

        send_system_message(
            &self.gate_ref,
            msg.session_id,
            &format!("行会 \"{}\" 已创建", msg.guild_name),
        );
        if let Some(guild) = self.guilds.get(&msg.guild_name) {
            send_guild_info_packet(&self.gate_ref, msg.session_id, guild);
        } else {
            send_guild_status_packet(&self.gate_ref, msg.session_id, true);
        }
        debug!("Guild created (GM): {} by {}", msg.guild_name, state.name);
    }
}

impl Message<GuildInviteReply> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: GuildInviteReply, _ctx: &mut Context<Self, Self::Reply>) {
        // 查找邀请
        let invite = match self.pending_guild_invites.remove(&msg.session_id) {
            Some(inv) => inv,
            None => return,
        };
        let (inviter_session, guild_name) = invite;

        if !msg.accept {
            send_system_message(&self.gate_ref, inviter_session, "对方拒绝了行会邀请");
            return;
        }

        // 获取行会
        let guild = match self.guilds.get_mut(&guild_name) {
            Some(g) => g,
            None => return,
        };

        // 检查行会人数上限
        if guild.member_count() >= GUILD_MAX_MEMBERS {
            send_system_message(&self.gate_ref, msg.session_id, "行会已满");
            return;
        }

        // 获取被邀请者名称
        let invitee_name = match self.players.get(&msg.session_id) {
            Some(r) => match r.ask(GetPlayerState).await {
                Ok(Some(s)) => s.name.clone(),
                _ => return,
            },
            None => return,
        };

        // 检查是否已在行会
        {
            let record = match self.players.get(&msg.session_id) {
                Some(r) => r,
                None => return,
            };
            let state = match record.ask(GetPlayerState).await {
                Ok(Some(s)) => s,
                _ => return,
            };
            if state.guild_name.is_some() {
                send_system_message(&self.gate_ref, msg.session_id, "你已经有行会了");
                return;
            }
        }

        // 添加到行会
        guild.add_member(invitee_name.clone(), Some(msg.session_id));

        // 更新玩家行会信息
        if let Some(record) = self.players.get(&msg.session_id) {
            let _ = record
                .ask(SetGuildInfo {
                    guild_name: Some(guild_name.clone()),
                    rank: GuildRank::Member,
                })
                .await;
            // #918：C# JoinGuild（~10005）——加入行会后 EnableGuildInvite 重置 false
            if let Ok(Some(mut st)) = record.ask(GetPlayerState).await {
                st.enable_guild_invite = false;
                // #2174：C# 加入新手行会 → Newbie buff 即时生效（tick_newbie_bonus 每 50 tick 兜底）
                if guild_name.eq_ignore_ascii_case(&self.config.newbie_guild)
                    && self.config.newbie_guild_buff_enabled
                {
                    st.newbie_exp_bonus = true;
                }
                let _ = record.ask(SetPlayerState { state: st }).await;
            }
            send_guild_status_packet(&self.gate_ref, msg.session_id, true);
        }

        // 通知行会成员
        for sid in guild.online_sessions(0) {
            send_guild_member_change_packet(&self.gate_ref, sid, &invitee_name, true);
        }

        send_system_message(
            &self.gate_ref,
            msg.session_id,
            &format!("已加入行会 \"{}\"", guild_name),
        );
        if let Some(inv_record) = self.players.get(&inviter_session) {
            if let Ok(Some(_inv_state)) = inv_record.ask(GetPlayerState).await {
                send_system_message(
                    &self.gate_ref,
                    inviter_session,
                    &format!("{} 加入了行会", invitee_name),
                );
                send_guild_member_change_packet(
                    &self.gate_ref,
                    inviter_session,
                    &invitee_name,
                    true,
                );
            }
        }

        // 发送完整行会信息给新成员（客户端据此显示行会对话框）
        if let Some(g) = self.guilds.get(&guild_name) {
            send_guild_info_packet(&self.gate_ref, msg.session_id, g);
        }

        debug!(
            "Guild invite accepted: {} joined {}",
            invitee_name, guild_name
        );
        // #2170：成员变更持久化（guild_members 表，重启后保持）
        self.save_guild_to_db(&guild_name).await;
    }
}

impl Message<RequestGuildInfo> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: RequestGuildInfo, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let guild_name = match &state.guild_name {
            Some(n) => n.clone(),
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "你没有行会");
                return;
            }
        };

        let guild = match self.guilds.get(&guild_name) {
            Some(g) => g,
            None => return,
        };

        // 发送行会信息
        send_guild_info_packet(&self.gate_ref, msg.session_id, guild);
        debug!("Guild info requested by {}", state.name);
    }
}

impl Message<EditGuildMemberRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: EditGuildMemberRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let guild_name = match &state.guild_name {
            Some(n) => n.clone(),
            None => return,
        };
        let my_rank = state.guild_rank;

        let guild = match self.guilds.get_mut(&guild_name) {
            Some(g) => g,
            None => return,
        };

        // #1463：C# 权限位——邀请 CanRecruit / 踢出 CanKick / 职务操作 CanChangeRank
        let my_options = guild.member_options(&state.name);
        let required = match msg.change_type {
            0 => crate::actors::guild::GuildRank::CAN_RECRUIT,
            1 => crate::actors::guild::GuildRank::CAN_KICK,
            _ => crate::actors::guild::GuildRank::CAN_CHANGE_RANK,
        };
        if my_options & required == 0 {
            send_system_message(&self.gate_ref, msg.session_id, "权限不足");
            return;
        }

        let mut guild_changed = false;
        match msg.change_type {
            0 => {
                // 邀请加入（C# EditGuildMember ChangeType=0 add member）
                // 查找目标玩家（在线）
                let mut target_session: Option<u64> = None;
                for (sid, r) in &self.players {
                    if let Ok(Some(s)) = r.ask(GetPlayerState).await {
                        if s.name == msg.member_name {
                            target_session = Some(*sid);
                            break;
                        }
                    }
                }
                let Some(target) = target_session else {
                    send_system_message(&self.gate_ref, msg.session_id, "玩家不在线");
                    return;
                };
                // 检查目标是否已在行会
                let target_in_guild = match self.players.get(&target) {
                    Some(r) => match r.ask(GetPlayerState).await {
                        Ok(Some(s)) => s.guild_name.is_some(),
                        _ => true,
                    },
                    None => true,
                };
                if target_in_guild {
                    send_system_message(&self.gate_ref, msg.session_id, "对方已有行会");
                    return;
                }
                // #910：C# PlayerObject（~9869）目标未开启 @ALLOWGUILD → 拒绝邀请
                let target_invite_disabled = match self.players.get(&target) {
                    Some(r) => match r.ask(GetPlayerState).await {
                        Ok(Some(s)) => !s.enable_guild_invite,
                        _ => true,
                    },
                    None => true,
                };
                if target_invite_disabled {
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        "对方关闭了行会邀请（请对方先使用 @ALLOWGUILD 开启）",
                    );
                    return;
                }
                // 已有待处理邀请
                if self.pending_guild_invites.contains_key(&target) {
                    send_system_message(&self.gate_ref, msg.session_id, "邀请已发送，等待对方回复");
                    return;
                }
                // C# PlayerObject.cs:9880-9884：MyGuild.IsAtWar() -> CannotRecruitDuringWar（仅拦截招募）
                if self
                    .guild_wars
                    .get(&guild_name)
                    .map(|s| !s.is_empty())
                    .unwrap_or(false)
                {
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        "行会处于战争状态，无法招募成员",
                    );
                    return;
                }
                self.pending_guild_invites
                    .insert(target, (msg.session_id, guild_name.clone()));
                send_guild_invite_packet(&self.gate_ref, target, &guild_name);
                send_system_message(
                    &self.gate_ref,
                    msg.session_id,
                    &format!("已向 {} 发送行会邀请", msg.member_name),
                );
            }
            1 => {
                // 踢出（C# ChangeType=1 delete member）
                // 不能踢会长
                if guild
                    .members
                    .iter()
                    .any(|m| m.name == msg.member_name && m.rank == GuildRank::Leader)
                {
                    send_system_message(&self.gate_ref, msg.session_id, "不能踢出会长");
                    return;
                }
                // 副会长不能踢出其他副会长/会长
                if my_rank == GuildRank::Officer {
                    if let Some(m) = guild.members.iter().find(|m| m.name == msg.member_name) {
                        if m.rank != GuildRank::Member {
                            send_system_message(&self.gate_ref, msg.session_id, "权限不足");
                            return;
                        }
                    }
                }

                let kicked_session = guild.members.iter().find_map(|m| {
                    if m.name == msg.member_name {
                        m.session_id
                    } else {
                        None
                    }
                });

                if guild.remove_member(&msg.member_name) {
                    // 更新被踢玩家
                    guild_changed = true;
                    if let Some(sid) = kicked_session {
                        if let Some(rec) = self.players.get(&sid) {
                            let _ = rec
                                .ask(SetGuildInfo {
                                    guild_name: None,
                                    rank: GuildRank::Member,
                                })
                                .await;
                            send_guild_status_packet(&self.gate_ref, sid, false);
                            // C# RefreshStats：行会关系结束后立即清除行会/新手 buff 加成缓存
                            if let Ok(Some(mut st)) = rec.ask(GetPlayerState).await {
                                st.guild_buff_exp_percent = 0;
                                st.guild_buff_fish_rate_percent = 0;
                                st.guild_buff_mine_rate_percent = 0;
                                st.newbie_exp_bonus = false;
                                let _ = rec.ask(SetPlayerState { state: st }).await;
                            }
                        }
                    }
                    // 通知行会成员
                    for sid in guild.online_sessions(0) {
                        send_guild_member_change_packet(
                            &self.gate_ref,
                            sid,
                            &msg.member_name,
                            false,
                        );
                    }
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        &format!("{} 已被踢出行会", msg.member_name),
                    );
                }
            }
            2 => {
                // #1395：按职务索引移动成员（C# EditGuildMember ChangeType=2 带 RankIndex）
                let target = msg.rank_index;
                if !guild.rank_defs.iter().any(|d| d.index == target) {
                    send_system_message(&self.gate_ref, msg.session_id, "职务不存在");
                    return;
                }
                if let Some(m) = guild.members.iter_mut().find(|m| m.name == msg.member_name) {
                    m.rank_index = target;
                    // 目标职务为 0/1 档时同步逻辑档（2+ 视为成员档）
                    m.rank = match target {
                        0 => GuildRank::Leader,
                        1 => GuildRank::Officer,
                        _ => GuildRank::Member,
                    };
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        &format!("{} 已调整职务", msg.member_name),
                    );
                }
                guild_changed = true;
            }
            3 => {
                // 降职
                if guild.set_rank(&msg.member_name, GuildRank::Member) {
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        &format!("{} 已降职为成员", msg.member_name),
                    );
                }
                guild_changed = true;
            }
            4 => {
                // #1395：添加职务（C# EditGuildMember ChangeType=4 add rank）
                if msg.rank_name.trim().is_empty() {
                    send_system_message(&self.gate_ref, msg.session_id, "职务名无效");
                    return;
                }
                let new_idx = guild.add_rank(msg.rank_name.trim());
                for sid in guild.online_sessions(0) {
                    send_guild_info_packet(&self.gate_ref, sid, guild);
                }
                guild_changed = true;
                send_system_message(
                    &self.gate_ref,
                    msg.session_id,
                    &format!("已添加职务：{}（#{})", msg.rank_name.trim(), new_idx),
                );
            }
            5 => {
                // #1395：切换职务权限位（C# EditGuildMember ChangeType=5；rank_name=选项位，name=true/false）
                let bit = msg.rank_name.trim().parse::<u8>().unwrap_or(0);
                if bit >= 8 {
                    return;
                }
                let on = msg.member_name.trim() == "true";
                if let Some(d) = guild
                    .rank_defs
                    .iter_mut()
                    .find(|d| d.index == msg.rank_index)
                {
                    if on {
                        d.options |= 1 << bit;
                    } else {
                        d.options &= !(1 << bit);
                    }
                }
                for sid in guild.online_sessions(0) {
                    send_guild_info_packet(&self.gate_ref, sid, guild);
                }
                guild_changed = true;
                send_system_message(&self.gate_ref, msg.session_id, "职务权限已更新");
            }
            6 => {
                // #1362：职务改名（C# EditGuildMember ChangeType=3 rename；Rust 用 6 避免与降职冲突）
                let idx = msg.rank_index;
                if !guild.rank_defs.iter().any(|d| d.index == idx)
                    || msg.rank_name.trim().is_empty()
                {
                    send_system_message(&self.gate_ref, msg.session_id, "职务名无效");
                    return;
                }
                if let Some(d) = guild.rank_defs.iter_mut().find(|d| d.index == idx) {
                    d.name = msg.rank_name.trim().to_string();
                }
                guild_changed = true;
                // 广播全量行会信息（职务名变化 → 成员列表刷新）
                for sid in guild.online_sessions(0) {
                    send_guild_info_packet(&self.gate_ref, sid, guild);
                }
                send_system_message(&self.gate_ref, msg.session_id, "职务名已更新");
            }
            _ => {}
        }
        // #1395：职务/权限变更后持久化
        if guild_changed {
            self.save_guild_to_db(&guild_name).await;
        }
    }
}

impl Message<EditGuildNoticeRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: EditGuildNoticeRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let guild_name = match &state.guild_name {
            Some(n) => n.clone(),
            None => return,
        };

        let guild = match self.guilds.get_mut(&guild_name) {
            Some(g) => g,
            None => return,
        };

        // #1461：C# CanChangeNotice
        if guild.member_options(&state.name) & crate::actors::guild::GuildRank::CAN_CHANGE_NOTICE
            == 0
        {
            send_system_message(&self.gate_ref, msg.session_id, "权限不足");
            return;
        }
        guild.notice = msg.notice.clone();

        // 通知所有在线行会成员
        for sid in guild.online_sessions(0) {
            send_guild_notice_change_packet(&self.gate_ref, sid, &guild.notice);
        }

        send_system_message(&self.gate_ref, msg.session_id, "行会公告已更新");
        // #2172：公告持久化（重启后保持）
        self.save_guild_to_db(&guild_name).await;
    }
}

/// C# GuildObject.DeleteMember（:455-510）：会长离开结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeaderLeaveOutcome {
    /// 最后一名成员离开 → 解散行会（C# LeaderOk）
    Disband,
    /// 多名会长 → 正常退会（C# AllOk）
    Leave,
    /// 唯一会长且还有其他成员 → 阻止（YouNeedLastLeaderToDisbandGuild）
    Blocked,
}

/// C# GuildObject.DeleteMember：member_total<2 → Disband；leader_count>1 → Leave；否则 Blocked
pub(crate) fn leader_leave_outcome(member_total: usize, leader_count: usize) -> LeaderLeaveOutcome {
    if member_total < 2 {
        LeaderLeaveOutcome::Disband
    } else if leader_count > 1 {
        LeaderLeaveOutcome::Leave
    } else {
        LeaderLeaveOutcome::Blocked
    }
}

impl Message<LeaveGuildRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: LeaveGuildRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let guild_name = match &state.guild_name {
            Some(n) => n.clone(),
            None => return,
        };

        // C# LEAVEGUILD（PlayerObject.cs:3251-3259）：MyGuild.IsAtWar() -> CannotLeaveGuildAtWar
        // 置于会长/解散逻辑之前：普通退会与最后会长解散两条路径统一拦截
        if self
            .guild_wars
            .get(&guild_name)
            .map(|s| !s.is_empty())
            .unwrap_or(false)
        {
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                "行会处于战争状态，无法退出行会",
            );
            return;
        }

        // C# GuildObject.DeleteMember（:455-510）：会长离开 → 最后成员解散 / 多会长正常退 / 唯一会长有成员阻止
        if state.guild_rank == GuildRank::Leader {
            let Some(guild) = self.guilds.get_mut(&guild_name) else {
                return;
            };
            let leader_count = guild
                .members
                .iter()
                .filter(|m| m.rank == GuildRank::Leader)
                .count();
            let member_total = guild.members.len();
            match leader_leave_outcome(member_total, leader_count) {
                LeaderLeaveOutcome::Disband => {
                    // LeaderOk：最后会长离开 → Envir.DeleteGuild（:505-506）
                    let online: Vec<u64> = guild.online_sessions(0);
                    self.guilds.remove(&guild_name);
                    let _ = db::delete_guild(&self.db_pool, &guild_name).await;
                    for sid in online {
                        if let Some(rec) = self.players.get(&sid) {
                            let _ = rec
                                .ask(SetGuildInfo {
                                    guild_name: None,
                                    rank: GuildRank::Member,
                                })
                                .await;
                            send_guild_status_packet(&self.gate_ref, sid, false);
                            // C# RefreshStats：行会关系结束后立即清除行会/新手 buff 加成缓存
                            if let Ok(Some(mut st)) = rec.ask(GetPlayerState).await {
                                st.guild_buff_exp_percent = 0;
                                st.guild_buff_fish_rate_percent = 0;
                                st.guild_buff_mine_rate_percent = 0;
                                st.newbie_exp_bonus = false;
                                let _ = rec.ask(SetPlayerState { state: st }).await;
                            }
                            if let Ok(Some(fresh)) = rec.ask(GetPlayerState).await {
                                self.broadcast_ride_appearance(sid, &fresh).await;
                            }
                            send_system_message(
                                &self.gate_ref,
                                sid,
                                &format!("行会 \"{}\" 已解散", guild_name),
                            );
                        }
                    }
                    return;
                }
                LeaderLeaveOutcome::Blocked => {
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        "你是唯一会长且行会还有其他成员，无法离开（需要最后一名会长才能解散行会）",
                    );
                    return;
                }
                LeaderLeaveOutcome::Leave => {
                    // AllOk：多名会长 → 继续正常退会
                }
            }
        }

        let guild = match self.guilds.get_mut(&guild_name) {
            Some(g) => g,
            None => return,
        };

        guild.remove_member(&state.name);
        let _ = record
            .ask(SetGuildInfo {
                guild_name: None,
                rank: GuildRank::Member,
            })
            .await;
        send_guild_status_packet(&self.gate_ref, msg.session_id, false);

        // 通知其他行会成员
        // C# LEAVEGUILD（:3259-3260）：退会立即清除行会/新手 buff 加成缓存（RemoveBuff(Guild)/RemoveBuff(Newbie)）
        if let Ok(Some(mut st)) = record.ask(GetPlayerState).await {
            st.guild_buff_exp_percent = 0;
            st.guild_buff_fish_rate_percent = 0;
            st.guild_buff_mine_rate_percent = 0;
            st.newbie_exp_bonus = false;
            let _ = record.ask(SetPlayerState { state: st }).await;
        }

        for sid in guild.online_sessions(0) {
            send_guild_member_change_packet(&self.gate_ref, sid, &state.name, false);
        }

        send_system_message(
            &self.gate_ref,
            msg.session_id,
            &format!("已离开行会 \"{}\"", guild_name),
        );
        // #1374：C# BroadcastInfo——退出行会后重发外观（行会名清除）
        if let Ok(Some(fresh)) = record.ask(GetPlayerState).await {
            self.broadcast_ride_appearance(msg.session_id, &fresh).await;
        }
        // #2170：成员变更持久化（guild_members 表，重启后保持）
        self.save_guild_to_db(&guild_name).await;
    }
}

impl Message<GuildStorageGoldChangeRequest> for SocialActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: GuildStorageGoldChangeRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let guild_name = match &state.guild_name {
            Some(n) => n.clone(),
            None => return,
        };

        let guild = match self.guilds.get_mut(&guild_name) {
            Some(g) => g,
            None => return,
        };

        match msg.change_type {
            0 => {
                // 存入
                // #1462：C# CanStoreItem
                if guild.member_options(&state.name)
                    & crate::actors::guild::GuildRank::CAN_STORE_ITEM
                    == 0
                {
                    send_system_message(&self.gate_ref, msg.session_id, "权限不足");
                    return;
                }
                let has_gold = { state.inventory.gold >= msg.amount as u64 };
                if !has_gold {
                    send_system_message(&self.gate_ref, msg.session_id, "金币不足");
                    return;
                }
                let _ = record
                    .ask(DeductGold {
                        amount: msg.amount as u64,
                    })
                    .await;
                guild.gold += msg.amount as u64;
                send_system_message(
                    &self.gate_ref,
                    msg.session_id,
                    &format!("已存入 {} 金币到行会仓库", msg.amount),
                );
                self.send_guild_storage_gold_change(msg.session_id, &state.name, msg.amount, 0)
                    .await;
                self.save_guild_to_db(&guild_name).await;
                self.broadcast_guild_info(&guild_name).await;
            }
            1 => {
                // 取出
                // 只有会长和副会长可以取出
                // #1462：C# CanRetrieveItem
                if guild.member_options(&state.name)
                    & crate::actors::guild::GuildRank::CAN_RETRIEVE_ITEM
                    == 0
                {
                    send_system_message(&self.gate_ref, msg.session_id, "权限不足");
                    return;
                }
                if guild.gold < msg.amount as u64 {
                    send_system_message(&self.gate_ref, msg.session_id, "行会仓库金币不足");
                    return;
                }
                guild.gold -= msg.amount as u64;
                let _ = record
                    .ask(AddGold {
                        amount: msg.amount as u64,
                    })
                    .await;
                send_system_message(
                    &self.gate_ref,
                    msg.session_id,
                    &format!("已从行会仓库取出 {} 金币", msg.amount),
                );
                self.send_guild_storage_gold_change(msg.session_id, &state.name, msg.amount, 1)
                    .await;
                self.save_guild_to_db(&guild_name).await;
                self.broadcast_guild_info(&guild_name).await;
            }
            _ => {}
        }
    }
}

impl Message<GuildStorageItemChangeRequest> for SocialActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: GuildStorageItemChangeRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // #2012：C# GuildStorageItemChange（10134）——type != 3 需在安全区
        if msg.change_type != 3
            && !self
                .is_in_safe_zone(state.map_index, state.x, state.y)
                .await
        {
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                "必须在安全区才能使用行会仓库",
            );
            return;
        }

        let guild_name = match &state.guild_name {
            Some(n) => n.clone(),
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "你还没有加入行会");
                return;
            }
        };

        let guild = match self.guilds.get_mut(&guild_name) {
            Some(g) => g,
            None => return,
        };

        match msg.change_type {
            0 => {
                // 存入物品
                // #1462：C# CanStoreItem
                if guild.member_options(&state.name)
                    & crate::actors::guild::GuildRank::CAN_STORE_ITEM
                    == 0
                {
                    send_system_message(&self.gate_ref, msg.session_id, "权限不足");
                    return;
                }
                if !guild.storage_has_space() {
                    send_system_message(&self.gate_ref, msg.session_id, "行会仓库已满");
                    return;
                }

                if state.inventory.get_item(msg.unique_id).is_none() {
                    send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
                    return;
                }
                // C#：BindMode.DontStore(0x8) 物品不可存入行会仓库（含租赁绑定，:10167）
                let infos = self.config.item_infos.read().await;
                let bind = state
                    .inventory
                    .get_item(msg.unique_id)
                    .and_then(|it| infos.get(&it.item_index).map(|i| i.bind_mode))
                    .unwrap_or(0);
                let rental_dont_store = state
                    .inventory
                    .get_item(msg.unique_id)
                    .map(|it| {
                        crate::actors::world::rental_has_flag(
                            it,
                            mir2_shared::enums::BindMode::DONT_STORE.bits(),
                        )
                    })
                    .unwrap_or(false);
                if (bind & 0x0008) != 0 || rental_dont_store {
                    send_system_message(&self.gate_ref, msg.session_id, "该物品无法存入仓库");
                    return;
                }

                let removed = record
                    .ask(RemoveItemFromInventory {
                        unique_id: msg.unique_id,
                    })
                    .await
                    .unwrap_or(None);
                let mut deposited = false;
                if let Some(removed_item) = removed {
                    let item_index = removed_item.item_index;
                    // #2012：C# 整件移动——存储数量服务端权威为整叠，忽略客户端 count（防部分存入数量不一致）
                    let stored_qty = removed_item.count as u32;
                    // C# GuildStorageItemChange type=0（:10178）：存入 to 槽（msg.grid = 客户端点选的目标槽）
                    let slot_val = msg.grid as usize;
                    if guild.deposit_item_at(removed_item.clone(), stored_qty, slot_val) {
                        send_system_message(&self.gate_ref, msg.session_id, "物品已存入行会仓库");
                        debug!(
                            "GuildStorageItem: {} deposited item={} slot={}",
                            state.name, item_index, slot_val
                        );
                        // #295：实时通知（C# S.GuildStorageItemChange type=0 存入）
                        self.send_guild_storage_item_change(
                            msg.session_id,
                            0,
                            slot_val as i32,
                            0,
                            state.object_id as i32,
                            Some((state.object_id as i64, removed_item)),
                        )
                        .await;
                        deposited = true;
                    } else {
                        let _ = record.ask(AddItemToInventory { item: removed_item }).await;
                        send_system_message(
                            &self.gate_ref,
                            msg.session_id,
                            "该仓库格子已有物品或无效",
                        );
                    }
                }
                if deposited {
                    debug!("GuildStorageItem: saved + broadcast storage list");
                    self.save_guild_to_db(&guild_name).await;
                    self.broadcast_guild_storage_list(&guild_name).await;
                } else {
                    debug!("GuildStorageItem: deposit failed");
                }
            }
            1 => {
                // 取出物品
                // #1462：C# CanRetrieveItem
                if guild.member_options(&state.name)
                    & crate::actors::guild::GuildRank::CAN_RETRIEVE_ITEM
                    == 0
                {
                    send_system_message(&self.gate_ref, msg.session_id, "权限不足");
                    return;
                }

                if !state.inventory.has_space() {
                    send_system_message(&self.gate_ref, msg.session_id, "背包已满");
                    return;
                }

                // #2012：历史脏数据自愈——存储 qty 与物品实际堆叠数对齐（C# 整件语义）
                if let Some((item, qty)) = &guild.storage_items[msg.grid as usize] {
                    if *qty != item.count as u32 {
                        warn!(
                            "GuildStorageItem: 修正脏数据 slot={} qty={} -> {}",
                            msg.grid, qty, item.count
                        );
                        guild.storage_items[msg.grid as usize] =
                            Some((item.clone(), item.count as u32));
                    }
                }
                let result = guild.withdraw_item(msg.grid);
                let mut withdrew = false;
                match result {
                    Some((item_data, qty, _slot)) => {
                        let added = record
                            .ask(AddItemToInventory {
                                item: item_data.clone(),
                            })
                            .await
                            .unwrap_or(false);
                        if added {
                            send_system_message(&self.gate_ref, msg.session_id, "物品已取出");
                            // #295：实时通知（C# S.GuildStorageItemChange type=1 取出）
                            self.send_guild_storage_item_change(
                                msg.session_id,
                                1,
                                0,
                                msg.grid as i32,
                                state.object_id as i32,
                                None,
                            )
                            .await;
                            withdrew = true;
                        } else {
                            guild.storage_items[msg.grid as usize] = Some((item_data, qty));
                            send_system_message(&self.gate_ref, msg.session_id, "背包已满");
                        }
                    }
                    None => {
                        send_system_message(&self.gate_ref, msg.session_id, "该仓库格子没有物品");
                    }
                }
                if withdrew {
                    debug!("GuildStorageItem: saved + broadcast storage list");
                    self.save_guild_to_db(&guild_name).await;
                    self.broadcast_guild_storage_list(&guild_name).await;
                } else {
                    debug!("GuildStorageItem: withdraw failed");
                }
            }
            3 => {
                // 请求仓库列表（C# GuildStorageItemChange type=3 语义）
                send_guild_storage_list_packet(&self.gate_ref, msg.session_id, guild);
            }
            _ => {}
        }
    }
}

// ============================================================
// 婚姻系统 Handler
// ============================================================

impl Message<MarriageRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: MarriageRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let requester_record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let requester_state = match requester_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查是否已有配偶
        if requester_state.spouse_name.is_some() {
            send_system_message(&self.gate_ref, msg.session_id, "你已经结婚了");
            return;
        }

        // C# MarriageRequest（:13140-13144）：离婚冷却（MarriedDate.AddDays(MarriageCooldown=7) > Now）
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        if requester_state.married_date > 0
            && now_unix < requester_state.married_date + self.config.marriage_cooldown_days * 86_400
        {
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                &format!(
                    "离婚后 {} 天内无法再次结婚",
                    self.config.marriage_cooldown_days
                ),
            );
            return;
        }
        // C# MarriageRequest（:13146-13150）：等级要求（Settings.MarriageLevelRequired=10）
        if requester_state.level < self.config.marriage_level_required {
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                &format!(
                    "需要达到 {} 级才能结婚",
                    self.config.marriage_level_required
                ),
            );
            return;
        }

        // C# MarriageRequest（:13198）：不能向自己求婚（CantMarryYourself）
        if requester_state.name.eq_ignore_ascii_case(&msg.target_name) {
            send_system_message(&self.gate_ref, msg.session_id, "不能向自己求婚");
            return;
        }

        // 查找目标玩家
        let target_session = match self
            .find_player_by_name(&msg.target_name, msg.session_id)
            .await
        {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "目标玩家不在线");
                return;
            }
        };

        let target_record = match self.players.get(&target_session) {
            Some(r) => r,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "目标玩家已下线");
                return;
            }
        };
        let target_state = match target_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // C# MarriageRequest（:13174-13226）：目标侧校验（面对面/等级/冷却/AllowMarriage/死亡/待处理求婚/范围/已婚）
        let marriage_ctx = MarriageTargetCtx {
            requester_map: requester_state.map_index,
            requester_x: requester_state.x,
            requester_y: requester_state.y,
            requester_dir: requester_state.direction,
            requester_dead: requester_state.is_dead,
            target_name: &target_state.name,
            target_map: target_state.map_index,
            target_x: target_state.x,
            target_y: target_state.y,
            target_dir: target_state.direction,
            target_level: target_state.level,
            target_married_date: target_state.married_date,
            target_allow_marriage: target_state.allow_marriage,
            target_dead: target_state.is_dead,
            target_spouse: target_state.spouse_name.is_some(),
            target_has_pending: self.pending_marriage_invites.contains_key(&target_session),
        };
        if let Some(reason) = marriage_target_check(
            &marriage_ctx,
            now_unix,
            self.config.marriage_level_required,
            self.config.marriage_cooldown_days,
        ) {
            send_system_message(&self.gate_ref, msg.session_id, &reason);
            return;
        }

        // 发送结婚请求给目标
        self.pending_marriage_invites
            .insert(target_session, msg.session_id);
        send_marriage_invite_packet(&self.gate_ref, target_session, &requester_state.name);
        debug!(
            "MarriageRequest: {} -> {} (session {})",
            requester_state.name, msg.target_name, target_session
        );
    }
}

impl Message<MarriageReply> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: MarriageReply, _ctx: &mut Context<Self, Self::Reply>) {
        let replier_session = msg.session_id;

        // 查找谁发送了邀请
        let requester_session = match self.pending_marriage_invites.remove(&replier_session) {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, replier_session, "没有待处理的结婚请求");
                return;
            }
        };

        let replier_record = match self.players.get(&replier_session) {
            Some(r) => r,
            None => return,
        };
        let replier_state = match replier_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if !msg.accept {
            send_system_message(&self.gate_ref, replier_session, "你拒绝了结婚请求");
            // 通知邀请方被拒绝
            if let Some(req_record) = self.players.get(&requester_session) {
                let req_state = match req_record.ask(GetPlayerState).await {
                    Ok(Some(s)) => s,
                    _ => return,
                };
                send_system_message(
                    &self.gate_ref,
                    requester_session,
                    &format!("{} 拒绝了结婚请求", replier_state.name),
                );
                debug!(
                    "MarriageReply: {} rejected {}'s proposal",
                    replier_state.name, req_state.name
                );
            }
            return;
        }

        // 双方确认婚姻关系
        let requester_record = match self.players.get(&requester_session) {
            Some(r) => r,
            None => {
                send_system_message(&self.gate_ref, replier_session, "对方已不在线");
                return;
            }
        };
        let requester_state = match requester_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // #1329：结婚写入同一时刻（C# Info.MarriedDate = Envir.Now，unix 秒）
        // C# MarriageReply（:13253-13265）：双方已婚复检（邀请等待期间可能已结婚）
        if replier_state.spouse_name.is_some() {
            send_system_message(&self.gate_ref, replier_session, "你已经结婚了");
            return;
        }
        if requester_state.spouse_name.is_some() {
            send_system_message(
                &self.gate_ref,
                replier_session,
                &format!("{} 已经结婚了", requester_state.name),
            );
            return;
        }

        let married_date = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let _ = replier_record
            .ask(SetSpouse {
                spouse_name: Some(requester_state.name.clone()),
                married_date,
            })
            .await;
        let _ = requester_record
            .ask(SetSpouse {
                spouse_name: Some(replier_state.name.clone()),
                married_date,
            })
            .await;

        send_system_message(
            &self.gate_ref,
            replier_session,
            &format!("结婚成功，你的配偶是: {}", requester_state.name),
        );
        send_system_message(
            &self.gate_ref,
            requester_session,
            &format!("结婚成功，你的配偶是: {}", replier_state.name),
        );
        // #1329：全量 LoverUpdate（双方视角：对方名字/结婚日期/当前地图/结婚天数）
        let requester_map = self.map_title(requester_state.map_index).await;
        let replier_map = self.map_title(replier_state.map_index).await;
        send_lover_update_packet(
            &self.gate_ref,
            replier_session,
            &requester_state.name,
            married_date,
            &requester_map,
            0,
        );
        send_lover_update_packet(
            &self.gate_ref,
            requester_session,
            &replier_state.name,
            married_date,
            &replier_map,
            0,
        );
        debug!(
            "Marriage: {} <-> {} married",
            requester_state.name, replier_state.name
        );
    }
}

impl Message<SocialDivorceRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SocialDivorceRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let requester_record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let requester_state = match requester_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查是否已婚
        if requester_state.spouse_name.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "你还没有结婚");
            return;
        }

        // 查找配偶是否在线
        let target_session = match self
            .find_player_by_name(&msg.partner_name, msg.session_id)
            .await
        {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "目标玩家不在线");
                return;
            }
        };

        let target_record = match self.players.get(&target_session) {
            Some(r) => r,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "目标玩家已下线");
                return;
            }
        };
        let target_state = match target_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 确认确实是配偶关系
        if target_state.spouse_name.as_deref() != Some(&requester_state.name) {
            send_system_message(&self.gate_ref, msg.session_id, "对方不是你的配偶");
            return;
        }

        // 发送离婚请求
        send_divorce_request_packet(&self.gate_ref, target_session, &requester_state.name);
        debug!(
            "DivorceRequest: {} -> {}",
            requester_state.name, msg.partner_name
        );
    }
}

impl Message<SocialDivorceReply> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SocialDivorceReply, _ctx: &mut Context<Self, Self::Reply>) {
        let replier_record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let replier_state = match replier_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if !msg.accept {
            send_system_message(&self.gate_ref, msg.session_id, "你拒绝了离婚请求");
            return;
        }

        // 双方解除婚姻关系（C# DivorceReply :13378-13387：MarriedDate = Envir.Now = 离婚冷却起点）
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let spouse_name = replier_state.spouse_name.clone();
        let _ = replier_record
            .ask(SetSpouse {
                spouse_name: None,
                married_date: now_unix,
            })
            .await;
        // C# DivorceReply（:13380-13392）：离婚清除左戒婚戒标记 + RefreshItem
        let _ = replier_record
            .ask(crate::actors::player::ClearWeddingRing)
            .await;

        // 通知前配偶
        if let Some(ref name) = spouse_name {
            if let Some(target_session) = self.find_player_by_name(name, msg.session_id).await {
                if let Some(target_record) = self.players.get(&target_session) {
                    let _ = target_record
                        .ask(SetSpouse {
                            spouse_name: None,
                            married_date: now_unix,
                        })
                        .await;
                    let _ = target_record
                        .ask(crate::actors::player::ClearWeddingRing)
                        .await;
                    send_system_message(&self.gate_ref, target_session, "你已离婚");
                    // M49：前配偶状态同步（原实现只更新确认方）
                    send_lover_update_packet(&self.gate_ref, target_session, "", now_unix, "", 0);
                }
            }
        }

        send_system_message(&self.gate_ref, msg.session_id, "离婚成功");
        send_lover_update_packet(&self.gate_ref, msg.session_id, "", now_unix, "", 0);
        debug!("DivorceReply: {} divorced", replier_state.name);
    }
}

impl Message<SocialChangeMarriage> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SocialChangeMarriage, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // C# ChangeMarriage（MirConnection.cs:1803-1823）：未婚切换 AllowMarriage，已婚切换 AllowLoverRecall
        let in_marriage = state.spouse_name.is_some();
        if in_marriage {
            let new_allow = !state.allow_lover_recall;
            let _ = record.ask(SetAllowLoverRecall { allow: new_allow }).await;
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                if new_allow {
                    "你现在允许配偶召回"
                } else {
                    "你现在禁止配偶召回"
                },
            );
            debug!(
                "ChangeMarriage: session={} married, allow_lover_recall={}",
                msg.session_id, new_allow
            );
        } else {
            let new_allow = !state.allow_marriage;
            let _ = record.ask(SetAllowMarriage { allow: new_allow }).await;
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                if new_allow {
                    "你现在允许接收求婚"
                } else {
                    "你现在禁止接收求婚"
                },
            );
            debug!(
                "ChangeMarriage: session={} unmarried, allow_marriage={}",
                msg.session_id, new_allow
            );
        }

        // 刷新 LoverUpdate 显示（原有行为保留）
        if in_marriage {
            let spouse_name = state.spouse_name.clone().unwrap_or_default();
            let spouse_map = if let Some(spouse_sid) =
                self.find_player_by_name(&spouse_name, msg.session_id).await
            {
                if let Some(spouse_record) = self.players.get(&spouse_sid) {
                    if let Ok(Some(spouse_state)) = spouse_record.ask(GetPlayerState).await {
                        self.map_title(spouse_state.map_index).await
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            send_lover_update_packet(
                &self.gate_ref,
                msg.session_id,
                &spouse_name,
                state.married_date,
                &spouse_map,
                self.married_days(state.married_date),
            );
        } else {
            send_lover_update_packet(&self.gate_ref, msg.session_id, "", 0, "", 0);
        }
    }
}

// ============================================================
// 师徒系统 Handler
// ============================================================

impl Message<SocialAddMentor> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SocialAddMentor, _ctx: &mut Context<Self, Self::Reply>) {
        let requester_record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let requester_state = match requester_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查是否已有导师
        if requester_state.mentor_name.is_some() {
            send_system_message(&self.gate_ref, msg.session_id, "你已经有导师了");
            return;
        }

        // 查找目标玩家
        let target_session = match self
            .find_player_by_name(&msg.mentor_name, msg.session_id)
            .await
        {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "目标玩家不在线");
                return;
            }
        };

        let target_record = match self.players.get(&target_session) {
            Some(r) => r,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "目标玩家已下线");
                return;
            }
        };
        let target_state = match target_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查目标是否允许拜师
        if !target_state.allow_mentor {
            send_system_message(&self.gate_ref, msg.session_id, "目标玩家当前不允许拜师");
            return;
        }

        // C# PlayerObject.AddMentor 规则（同职业 + 等级差 + 双方无师徒关系）
        if requester_state.name == msg.mentor_name {
            send_system_message(&self.gate_ref, msg.session_id, "不能拜自己为师");
            return;
        }
        if target_state.mentor_name.is_some() {
            send_system_message(&self.gate_ref, msg.session_id, "对方已有师徒关系");
            return;
        }
        if requester_state.class != target_state.class {
            send_system_message(&self.gate_ref, msg.session_id, "只能拜同职业的师父");
            return;
        }
        if (requester_state.level as u32 + self.config.mentor_level_gap as u32)
            > target_state.level as u32
        {
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                &format!("师父等级需高于徒弟至少 {} 级", self.config.mentor_level_gap),
            );
            return;
        }

        // #2374：C# AddMentor 冷却门（PlayerObject.cs:13536/13542：MentorDate > Now 拒绝）
        let now_secs = crate::actors::world::partners::now_unix_secs();
        if requester_state.mentor_date > now_secs {
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                "你正处于解除师徒的冷却期，暂时不能拜师",
            );
            return;
        }
        if target_state.mentor_date > now_secs {
            send_system_message(&self.gate_ref, msg.session_id, "对方正处于解除师徒的冷却期");
            return;
        }

        // 发送拜师请求给目标（C# S.MentorRequest：Name + Level）
        self.pending_mentor_invites
            .insert(target_session, msg.session_id);
        send_mentor_invite_packet(
            &self.gate_ref,
            target_session,
            &requester_state.name,
            requester_state.level,
        );
        debug!("AddMentor: {} -> {}", requester_state.name, msg.mentor_name);
    }
}

impl Message<SocialMentorReply> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SocialMentorReply, _ctx: &mut Context<Self, Self::Reply>) {
        let replier_session = msg.session_id;

        // 查找谁发送了邀请
        let requester_session = match self.pending_mentor_invites.remove(&replier_session) {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, replier_session, "没有待处理的拜师请求");
                return;
            }
        };

        let replier_record = match self.players.get(&replier_session) {
            Some(r) => r,
            None => return,
        };
        let replier_state = match replier_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if !msg.accept {
            send_system_message(&self.gate_ref, replier_session, "你拒绝了拜师请求");
            if let Some(req_record) = self.players.get(&requester_session) {
                let req_state = match req_record.ask(GetPlayerState).await {
                    Ok(Some(s)) => s,
                    _ => return,
                };
                send_system_message(
                    &self.gate_ref,
                    requester_session,
                    &format!("{} 拒绝了拜师请求", replier_state.name),
                );
                debug!(
                    "MentorReply: {} rejected {}'s request",
                    replier_state.name, req_state.name
                );
            }
            return;
        }

        // 确认师徒关系
        let requester_record = match self.players.get(&requester_session) {
            Some(r) => r,
            None => {
                send_system_message(&self.gate_ref, replier_session, "对方已不在线");
                return;
            }
        };
        let requester_state = match requester_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 双方互相记录（C#：student.Info.Mentor = mentor；mentor.Info.Mentor = student）
        // C# MentorReply（:13605-13635）：接受侧复检（等待期间双方可能已建立师徒关系/换职业/升级）
        if replier_state.mentor_name.is_some() {
            send_system_message(&self.gate_ref, replier_session, "你已有师徒关系");
            return;
        }
        if requester_state.mentor_name.is_some() {
            send_system_message(
                &self.gate_ref,
                replier_session,
                &format!("{} 已有师徒关系", requester_state.name),
            );
            return;
        }
        if replier_state.class != requester_state.class {
            send_system_message(&self.gate_ref, replier_session, "只能收同职业的徒弟");
            return;
        }
        // C# :13631-13635：徒弟等级需低于师父至少 10 级（Settings.MentorLevelGap=10，与 AddMentor 一致）
        if (requester_state.level as u32 + self.config.mentor_level_gap as u32)
            > replier_state.level as u32
        {
            send_system_message(
                &self.gate_ref,
                replier_session,
                &format!("徒弟等级需低于师父至少 {} 级", self.config.mentor_level_gap),
            );
            return;
        }

        // C#：student.Info.Mentor = 导师；导师 Info.IsMentor = true（PlayerObject.cs:13637-13640）
        let _ = replier_record
            .ask(SetMentor {
                mentor_name: Some(requester_state.name.clone()),
                is_mentor: true,
            })
            .await;
        let _ = requester_record
            .ask(SetMentor {
                mentor_name: Some(replier_state.name.clone()),
                is_mentor: false,
            })
            .await;
        // #2374：C# 双方 MentorDate = Envir.Now（:13641-13642）——拜师成功即开始 7 天期限
        let now_secs = crate::actors::world::partners::now_unix_secs();
        let _ = replier_record
            .ask(crate::actors::player::SetMentorDate { date: now_secs })
            .await;
        let _ = requester_record
            .ask(crate::actors::player::SetMentorDate { date: now_secs })
            .await;

        send_system_message(
            &self.gate_ref,
            replier_session,
            &format!("收徒成功，你的徒弟是: {}", requester_state.name),
        );
        send_system_message(
            &self.gate_ref,
            requester_session,
            &format!("拜师成功，你的导师是: {}", replier_state.name),
        );

        // 双方 MentorUpdate 同步（C# GetMentor 语义：Name = 对方）
        send_mentor_update_packet(
            &self.gate_ref,
            replier_session,
            &requester_state.name,
            requester_state.level as u32,
            true,
            replier_state.mentor_exp,
        );
        send_mentor_update_packet(
            &self.gate_ref,
            requester_session,
            &replier_state.name,
            replier_state.level as u32,
            true,
            requester_state.mentor_exp,
        );
        debug!(
            "Mentor: {} is mentor of {}",
            replier_state.name, requester_state.name
        );
    }
}

impl Message<SocialAllowMentor> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SocialAllowMentor, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let _ = record.ask(SetAllowMentor { allow: msg.allow }).await;
        send_system_message(
            &self.gate_ref,
            msg.session_id,
            if msg.allow {
                "已允许拜师"
            } else {
                "已禁止拜师"
            },
        );
        debug!(
            "AllowMentor: session={} allow={}",
            msg.session_id, msg.allow
        );
    }
}

impl Message<SocialCancelMentor> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SocialCancelMentor, _ctx: &mut Context<Self, Self::Reply>) {
        self.do_mentor_break(msg.session_id, msg.force).await;
    }
}

impl SocialActor {
    /// C# PlayerObject.MentorBreak（:13450-13475）：解除师徒关系（force=true 设 7 天冷却；#2374）
    async fn do_mentor_break(&mut self, session_id: u64, force: bool) {
        let record = match self.players.get(&session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if state.mentor_name.is_none() {
            send_system_message(&self.gate_ref, session_id, "你没有师徒关系");
            return;
        }

        let partner_name = state.mentor_name.clone().unwrap_or_default();
        let self_is_mentor = state.is_mentor;

        // C# MentorBreak：取对方在线状态（partnerP）
        let partner_online: Option<(u64, crate::actors::player::PlayerState)> =
            match self.find_player_by_name(&partner_name, session_id).await {
                Some(sid) => match self.players.get(&sid) {
                    Some(prec) => match prec.ask(GetPlayerState).await {
                        Ok(Some(ps)) => Some((sid, ps)),
                        _ => None,
                    },
                    None => None,
                },
                None => None,
            };

        // C# MentorBreak 转移（仅对方在线：Info.MentorExp += partnerP.MenteeEXP / partner.MentorExp += MenteeEXP）
        let (
            new_self_mentor_exp,
            new_self_mentee_exp,
            new_partner_mentor_exp,
            new_partner_mentee_exp,
        ) = mentor_break_transfer(
            self_is_mentor,
            state.mentor_exp,
            state.mentee_exp,
            partner_online
                .as_ref()
                .map(|(_, s)| s.mentor_exp)
                .unwrap_or(0),
            partner_online
                .as_ref()
                .map(|(_, s)| s.mentee_exp)
                .unwrap_or(0),
            partner_online.is_some(),
        );

        // 清除自身师徒关系 + 写入新银行（C# Info.Mentor=0 → GetMentor(false)）
        let mut new_self_state = state.clone();
        new_self_state.mentor_name = None;
        new_self_state.is_mentor = false;
        new_self_state.mentor_exp = new_self_mentor_exp;
        new_self_state.mentee_exp = new_self_mentee_exp;
        // #2374：手动解除（force=true）→ 7 天冷却（C# :13463 MentorDate = Now.AddDays(7)）
        if force {
            new_self_state.mentor_date = mentor_cooldown_until(
                crate::actors::world::partners::now_unix_secs(),
                self.config.mentor_length_days as i64,
            );
        }
        let _ = record
            .ask(SetPlayerState {
                state: new_self_state,
            })
            .await;
        // C#：自身结算 IsMentor && MentorExp > 0 → GainExp + 清零
        if mentor_settle_amount(self_is_mentor, new_self_mentor_exp) > 0 {
            if let Some(world) = &self.world_ref {
                let _ = world
                    .ask(crate::actors::world::partners::SettleMentorExp { session_id })
                    .await;
            }
        }
        send_mentor_cancel_packet(&self.gate_ref, session_id);
        send_system_message(&self.gate_ref, session_id, "已解除师徒关系");

        // 对方同步（C# 双方 Info.Mentor 同时清空 + partner 结算）
        if let Some((partner_sid, partner_state)) = partner_online {
            if let Some(partner_record) = self.players.get(&partner_sid) {
                let mut ps = partner_state.clone();
                ps.mentor_name = None;
                ps.is_mentor = false;
                ps.mentor_exp = new_partner_mentor_exp;
                ps.mentee_exp = new_partner_mentee_exp;
                let _ = partner_record.ask(SetPlayerState { state: ps }).await;
                if mentor_settle_amount(!self_is_mentor, new_partner_mentor_exp) > 0 {
                    if let Some(world) = &self.world_ref {
                        let _ = world
                            .ask(crate::actors::world::partners::SettleMentorExp {
                                session_id: partner_sid,
                            })
                            .await;
                    }
                }
                send_mentor_cancel_packet(&self.gate_ref, partner_sid);
                send_system_message(
                    &self.gate_ref,
                    partner_sid,
                    &format!("{} 解除了师徒关系", state.name),
                );
            }
        } else if !self_is_mentor {
            // 对方（导师）离线：C# partner.Experience += partner.MentorExp 直接入账 + 清零
            if let Ok(Some(partner_db)) =
                crate::db::load_character(&self.db_pool, &partner_name).await
            {
                if partner_db.is_mentor && partner_db.mentor_exp > 0 {
                    let _ = crate::db::add_character_experience(
                        &self.db_pool,
                        &partner_name,
                        partner_db.mentor_exp,
                    )
                    .await;
                    let _ = crate::db::reset_mentor_exp(&self.db_pool, &partner_name).await;
                }
            }
        }
        debug!("CancelMentor: {} removed mentor", state.name);
    }
}

/// #2374：师徒到期判定（C# PlayerObject.cs:1194：MentorDate.AddDays(MentorLength) < Now；MentorLength=7 天）
pub(crate) fn mentor_relationship_expired(mentor_date: i64, now: i64, length_days: i64) -> bool {
    mentor_date > 0 && mentor_date + length_days * 86400 < now
}

/// #2374：解除冷却截止（C# MentorBreak force：MentorDate = Now.AddDays(MentorLength)）
pub(crate) fn mentor_cooldown_until(now: i64, length_days: i64) -> i64 {
    now + length_days * 86400
}

/// C# MentorBreak 经验转移计算（纯函数）：仅对方在线时转移——
/// 自己是导师则收徒弟 MenteeEXP；自己是徒弟则把 MenteeEXP 转给导师 MentorExp。
/// 返回 (self_mentor_exp, self_mentee_exp, partner_mentor_exp, partner_mentee_exp)
fn mentor_break_transfer(
    self_is_mentor: bool,
    self_mentor_exp: i64,
    self_mentee_exp: i64,
    partner_mentor_exp: i64,
    partner_mentee_exp: i64,
    partner_online: bool,
) -> (i64, i64, i64, i64) {
    if !partner_online {
        // C#：离线不转移；关系结束自身 MenteeEXP 清零
        return (self_mentor_exp, 0, partner_mentor_exp, partner_mentee_exp);
    }
    if self_is_mentor {
        // C#：Info.MentorExp += partnerP.MenteeEXP; partnerP.MenteeEXP = 0
        (
            self_mentor_exp + partner_mentee_exp,
            0,
            partner_mentor_exp,
            0,
        )
    } else {
        // C#：partner.MentorExp += MenteeEXP; MenteeEXP = 0
        (
            self_mentor_exp,
            0,
            partner_mentor_exp + self_mentee_exp,
            partner_mentee_exp,
        )
    }
}

/// C# MentorBreak 结算：IsMentor && MentorExp > 0 → GainExp(MentorExp)（返回应入账经验）
fn mentor_settle_amount(is_mentor: bool, mentor_exp: i64) -> i64 {
    if is_mentor && mentor_exp > 0 {
        mentor_exp
    } else {
        0
    }
}

impl SocialActor {
    /// M60：骑乘/下马后同步外观（自身 + 同地图其他玩家）
    async fn broadcast_ride_appearance(
        &self,
        session_id: u64,
        state: &crate::actors::player::PlayerState,
    ) {
        use crate::actors::inventory::EquipmentSlot;
        let infos = self.config.item_infos.read().await;
        let weapon = state
            .inventory
            .get_equipment(EquipmentSlot::Weapon)
            .and_then(|item| infos.get(&item.item_index).cloned())
            .map(|info| info.shape as i16)
            .unwrap_or(-1);
        let armor = state
            .inventory
            .get_equipment(EquipmentSlot::Armour)
            .and_then(|item| infos.get(&item.item_index).cloned())
            .map(|info| info.shape as i16)
            .unwrap_or(0);
        let weapon_effect = state
            .inventory
            .get_equipment(EquipmentSlot::Weapon)
            .and_then(|item| infos.get(&item.item_index).cloned())
            .map(|info| info.effect as i16)
            .unwrap_or(0);
        let packet = crate::actors::world::build_object_player_packet(
            &state.name,
            state.object_id,
            state.x,
            state.y,
            state.direction,
            state.level,
            crate::actors::world::name_colour_for_pk(
                state.pk_points,
                crate::actors::world::is_brown(state.brown_until_ms),
            ),
            state.class,
            state.gender,
            state.hair,
            weapon,
            weapon_effect,
            armor,
            state.mount_type,
            state.is_mounted,
            state.level_effects,
            state.guild_name.as_deref().unwrap_or(""),
            crate::actors::world::guild_rank_name(state.guild_rank),
        );
        let _ = self
            .gate_ref
            .tell(SendToClient {
                session_id,
                data: packet.clone(),
            })
            .await;
        for (sid, other) in &self.players {
            if *sid == session_id {
                continue;
            }
            if let Ok(Some(os)) = other.ask(GetPlayerState).await {
                if os.map_index == state.map_index {
                    let _ = self
                        .gate_ref
                        .tell(SendToClient {
                            session_id: *sid,
                            data: packet.clone(),
                        })
                        .await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_guild_war_mirror, facing_each_other, front_tile, leader_leave_outcome,
        LeaderLeaveOutcome,
    };

    #[test]
    fn leader_leave_outcome_matches_csharp_deletemember() {
        // C# GuildObject.DeleteMember：最后成员解散 / 多会长正常退 / 唯一会长有成员阻止
        assert_eq!(leader_leave_outcome(1, 1), LeaderLeaveOutcome::Disband);
        assert_eq!(leader_leave_outcome(2, 2), LeaderLeaveOutcome::Leave);
        assert_eq!(leader_leave_outcome(3, 2), LeaderLeaveOutcome::Leave);
        assert_eq!(leader_leave_outcome(5, 1), LeaderLeaveOutcome::Blocked);
        assert_eq!(leader_leave_outcome(2, 1), LeaderLeaveOutcome::Blocked);
    }

    /// #1159：C# Functions.FacingEachOther——双方朝向彼此才为 true
    #[test]
    fn test_facing_each_other() {
        // A(0,0) 朝右(2)，B(1,0) 朝左(6)：面对面
        assert!(facing_each_other(2, 0, 0, 6, 1, 0));
        // A 朝上(0)，B 朝下(4)：面对面
        assert!(facing_each_other(0, 5, 5, 4, 5, 4));
        // 同向（都朝右）不是面对面
        assert!(!facing_each_other(2, 0, 0, 2, 1, 0));
        // 背对不是面对面
        assert!(!facing_each_other(6, 0, 0, 6, 1, 0));
    }

    /// #1159：C# Functions.PointMove(location, direction, 1)——前方一格
    #[test]
    fn test_front_tile() {
        assert_eq!(front_tile(5, 5, 0), (5, 4)); // Up
        assert_eq!(front_tile(5, 5, 2), (6, 5)); // Right
        assert_eq!(front_tile(5, 5, 4), (5, 6)); // Down
        assert_eq!(front_tile(5, 5, 6), (4, 5)); // Left
        assert_eq!(front_tile(5, 5, 1), (6, 4)); // UpRight
    }

    /// #2180：C# MarriageRequest 目标侧校验（PlayerObject.cs:13174-13226）每项拒绝路径
    #[test]
    fn marriage_target_check_rejects_like_csharp() {
        use super::{marriage_target_check, MarriageTargetCtx};
        let now = 1_700_000_000i64;
        let base = MarriageTargetCtx {
            requester_map: 1,
            requester_x: 100,
            requester_y: 100,
            requester_dir: 2,
            requester_dead: false,
            target_name: "Target",
            target_map: 1,
            target_x: 101,
            target_y: 100,
            target_dir: 6,
            target_level: 10,
            target_married_date: 0,
            target_allow_marriage: true,
            target_dead: false,
            target_spouse: false,
            target_has_pending: false,
        };
        // 全部通过
        assert_eq!(marriage_target_check(&base, now, 10, 7), None);

        // 1. 双方面对面
        let c = MarriageTargetCtx {
            requester_dir: 6,
            ..base
        };
        assert_eq!(
            marriage_target_check(&c, now, 10, 7).unwrap(),
            "结婚需要双方面对面"
        );

        // 2. 目标等级不足
        let c = MarriageTargetCtx {
            target_level: 9,
            ..base
        };
        assert_eq!(
            marriage_target_check(&c, now, 10, 7).unwrap(),
            "Target 需要达到 10 级才能结婚"
        );

        // 3. 目标离婚冷却
        let c = MarriageTargetCtx {
            target_married_date: now - 1,
            ..base
        };
        assert_eq!(
            marriage_target_check(&c, now, 10, 7).unwrap(),
            "Target 离婚后 7 天内无法再次结婚"
        );
        // 冷却已过
        let c = MarriageTargetCtx {
            target_married_date: now - 8 * 86_400,
            ..base
        };
        assert_eq!(marriage_target_check(&c, now, 10, 7), None);

        // 4. 目标未允许求婚
        let c = MarriageTargetCtx {
            target_allow_marriage: false,
            ..base
        };
        assert_eq!(
            marriage_target_check(&c, now, 10, 7).unwrap(),
            "目标玩家当前不允许接收求婚"
        );

        // 5. 双方死亡
        let c = MarriageTargetCtx {
            requester_dead: true,
            ..base
        };
        assert_eq!(
            marriage_target_check(&c, now, 10, 7).unwrap(),
            "死亡状态下无法求婚"
        );
        let c = MarriageTargetCtx {
            target_dead: true,
            ..base
        };
        assert_eq!(
            marriage_target_check(&c, now, 10, 7).unwrap(),
            "死亡状态下无法求婚"
        );

        // 6. 目标已有待处理求婚
        let c = MarriageTargetCtx {
            target_has_pending: true,
            ..base
        };
        assert_eq!(
            marriage_target_check(&c, now, 10, 7).unwrap(),
            "Target 已有待处理的求婚"
        );

        // 7. 距离 > DataRange(16) 或不同地图
        let c = MarriageTargetCtx {
            target_x: 120,
            ..base
        };
        assert_eq!(
            marriage_target_check(&c, now, 10, 7).unwrap(),
            "Target 不在可求婚范围内"
        );
        let c = MarriageTargetCtx {
            target_map: 2,
            ..base
        };
        assert_eq!(
            marriage_target_check(&c, now, 10, 7).unwrap(),
            "Target 不在可求婚范围内"
        );

        // 8. 目标已婚
        let c = MarriageTargetCtx {
            target_spouse: true,
            ..base
        };
        assert_eq!(
            marriage_target_check(&c, now, 10, 7).unwrap(),
            "目标玩家已经结婚了"
        );
    }

    /// #2138：行会战争镜像双向增删（宣战/停战），与 C# WarringGuilds 语义一致
    #[test]
    fn guild_war_mirror_add_remove_bidirectional() {
        use std::collections::{HashMap, HashSet};
        let mut wars: HashMap<String, HashSet<String>> = HashMap::new();
        // 宣战：双向添加
        apply_guild_war_mirror(&mut wars, "A", "B", true);
        assert!(wars.get("A").unwrap().contains("B"));
        assert!(wars.get("B").unwrap().contains("A"));
        // 第二个敌对行会：A 集合增长
        apply_guild_war_mirror(&mut wars, "A", "C", true);
        assert_eq!(wars.get("A").unwrap().len(), 2);
        // 停战 B：A 仍与 C 交战，键保留；B 键清空删除
        apply_guild_war_mirror(&mut wars, "A", "B", false);
        assert!(!wars.get("A").unwrap().contains("B"));
        assert!(!wars.contains_key("B"));
        assert_eq!(wars.get("A").unwrap().len(), 1);
        // 停战 C：集合清空后删除键
        apply_guild_war_mirror(&mut wars, "A", "C", false);
        assert!(!wars.contains_key("A"));
        assert!(!wars.contains_key("C"));
        // 停战不存在的对：无副作用
        apply_guild_war_mirror(&mut wars, "X", "Y", false);
        assert!(wars.is_empty());
    }

    /// #2374：师徒到期/冷却（C# MentorDate + MentorLength(7 天)）
    #[test]
    fn mentor_term_and_cooldown_match_csharp() {
        let now = 1_700_000_000i64;
        // 未到期：MentorDate + 7d >= Now
        assert!(!super::mentor_relationship_expired(now - 6 * 86400, now, 7));
        assert!(!super::mentor_relationship_expired(now - 7 * 86400, now, 7));
        // 到期：MentorDate + 7d < Now
        assert!(super::mentor_relationship_expired(now - 8 * 86400, now, 7));
        // 无日期（0）不判到期
        assert!(!super::mentor_relationship_expired(0, now, 7));
        // 解除冷却截止 = Now + 7d（C# MentorDate = Now.AddDays(7)）
        assert_eq!(super::mentor_cooldown_until(now, 7), now + 7 * 86400);
    }

    /// #2142：C# MentorBreak 转移（仅对方在线）+ 结算金额
    #[test]
    fn mentor_break_transfer_and_settle_matches_csharp() {
        use super::{mentor_break_transfer, mentor_settle_amount};
        // 自己是导师、对方（徒弟）在线：收徒弟 MenteeEXP，徒弟清零
        assert_eq!(
            mentor_break_transfer(true, 100, 5, 0, 30, true),
            (130, 0, 0, 0)
        );
        // 自己是徒弟、对方（导师）在线：MenteeEXP 转导师，自身清零
        assert_eq!(
            mentor_break_transfer(false, 0, 30, 100, 0, true),
            (0, 0, 130, 0)
        );
        // 对方离线：不转移，自身 MenteeEXP 清零
        assert_eq!(
            mentor_break_transfer(false, 0, 30, 100, 0, false),
            (0, 0, 100, 0)
        );
        // 结算：仅导师且银行 > 0
        assert_eq!(mentor_settle_amount(true, 130), 130);
        assert_eq!(mentor_settle_amount(true, 0), 0);
        assert_eq!(mentor_settle_amount(false, 130), 0);
    }
}
