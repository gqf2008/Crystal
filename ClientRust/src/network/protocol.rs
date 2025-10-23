// Protocol module - Packet handling and dispatching
//
// 这个模块提供了基于 SharedRust 的数据包处理架构:
// - 直接使用 SharedRust 的 273 个服务器数据包和 146 个客户端数据包
// - 基于 opcode 的数据包分发系统
// - 类型安全的数据包处理接口
//
// 设计原则:
// 1. 不创建中间抽象层 (如 ServerMessage 枚举)
// 2. 直接使用 SharedRust 的 Packet trait
// 3. 通过 handler trait 实现多态处理

use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;

// ============================================================================
// 重新导出 SharedRust 的核心类型
// ============================================================================

// 重新导出 Packet trait
pub use mir2_shared::packets::base::Packet;

// 重新导出常用枚举
pub use mir2_shared::enums::ServerPacketIds;

// 重新导出常用数据类型
pub use mir2_shared::data::UserItem;

// 重新导出所有服务器数据包类型
pub mod packets {
    pub use mir2_shared::packets::server::*;
    
    // 明确导入有歧义的类型 - 优先使用 npc 模块的版本
    pub use mir2_shared::packets::server::npc::NPCRequestInput;
}

/// 序列化客户端数据包为字节流
///
/// 数据包格式:
/// - 2字节: 总长度 (包括这2字节)
/// - 2字节: Opcode (小端序)
/// - N字节: 数据包体
///
/// # 示例
/// ```ignore
/// use mir2_shared::packets::client::ClientVersion;
/// 
/// let packet = ClientVersion { version_hash: vec![1, 2, 3, 4] };
/// let bytes = serialize_client_packet(&packet)?;
/// // bytes 现在可以发送到服务器
/// ```
pub fn serialize_client_packet<P: Packet>(packet: &P) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    
    // 1. 写入长度占位符 (2字节)
    buffer.write_u16::<LittleEndian>(0)?;
    
    // 2. 写入opcode (2字节)
    buffer.write_i16::<LittleEndian>(P::OPCODE)?;
    
    // 3. 写入数据包体
    packet.write_body(&mut buffer)?;
    
    // 4. 回填实际长度
    let length = buffer.len() as u16;
    // 使用切片直接写入长度到开头2字节
    let mut cursor = Cursor::new(&mut buffer[0..2]);
    cursor.write_u16::<LittleEndian>(length)?;
    
    Ok(buffer)
}

// ============================================================================
// 服务器数据包解析辅助函数
// ============================================================================

/// 数据包头部信息
#[derive(Debug, Clone, Copy)]
pub struct PacketHeader {
    pub length: u16,
    pub opcode: i16,
}

/// 从字节流中解析数据包头部
///
/// # 参数
/// - `data`: 至少包含4字节的数据 (2字节长度 + 2字节opcode)
///
/// # 返回
/// - `Ok(PacketHeader)`: 成功解析的头部
/// - `Err`: 数据不足或解析失败
pub fn parse_packet_header(data: &[u8]) -> Result<PacketHeader> {
    if data.len() < 4 {
        return Err(anyhow!("数据不足: 需要至少4字节，实际{}字节", data.len()));
    }
    
    let mut cursor = Cursor::new(data);
    let length = cursor.read_u16::<LittleEndian>()?;
    let opcode = cursor.read_i16::<LittleEndian>()?;
    
    Ok(PacketHeader { length, opcode })
}

/// 获取数据包体的切片 (跳过4字节的头部)
///
/// # 参数
/// - `data`: 完整的数据包数据 (包括头部)
///
/// # 返回
/// - `Ok(&[u8])`: 数据包体的切片
/// - `Err`: 数据不足
/// 从完整的包数据中提取包体
/// 
/// # 参数
/// - `data`: 完整的包数据,格式为: [length(2字节)][opcode(2字节)][body...]
/// 
/// # 返回
/// 包体数据 (跳过前4字节的包头)
/// 
/// # 注意
/// 有些包可能没有包体(如Connected),这时会返回空slice,这是正常的
pub fn get_packet_body(data: &[u8]) -> Result<&[u8]> {
    if data.len() < 4 {
        return Err(anyhow!(
            "数据不足: 需要至少4字节 (包头), 实际收到 {} 字节",
            data.len()
        ));
    }
    
    // 跳过4字节包头 (2字节长度 + 2字节opcode)
    let body = &data[4..];
    
    Ok(body)
}

// ============================================================================
// 数据包处理器 Trait
// ============================================================================

/// 服务器数据包处理器接口
///
/// 实现这个 trait 来处理来自服务器的数据包。
/// 每个方法对应一个服务器数据包类型。
///
/// # 设计说明
/// 
/// 我们不使用一个大的 ServerMessage 枚举，而是通过这个 trait 来处理不同的数据包。
/// 好处:
/// 1. 类型安全 - 每个处理函数接收特定的数据包类型
/// 2. 可扩展 - 只需实现需要的方法
/// 3. 清晰 - 避免大量的 match 分支
///
/// # 示例
/// ```ignore
/// struct MyHandler;
/// 
/// impl PacketHandler for MyHandler {
///     fn on_connected(&mut self, packet: packets::Connected) {
///         println!("已连接到服务器");
///     }
///     
///     fn on_user_location(&mut self, packet: packets::UserLocation) {
///         println!("玩家位置: {:?}", packet.location);
///     }
/// }
/// ```
pub trait PacketHandler {
    // 连接相关
    fn on_connected(&mut self, _packet: packets::Connected) {}
    fn on_disconnect(&mut self, _packet: packets::Disconnect) {}
    
    // 用户信息
    fn on_user_information(&mut self, _packet: packets::UserInformation) {}
    fn on_user_location(&mut self, _packet: packets::UserLocation) {}
    
    // 地图相关
    fn on_map_information(&mut self, _packet: packets::MapInformation) {}
    fn on_new_map_info(&mut self, _packet: packets::NewMapInfo) {}
    
    // 对象相关
    fn on_object_player(&mut self, _packet: packets::ObjectPlayer) {}
    fn on_object_hero(&mut self, _packet: packets::ObjectHero) {}
    fn on_object_monster(&mut self, _packet: packets::ObjectMonster) {}
    fn on_object_npc(&mut self, _packet: packets::ObjectNpc) {}
    fn on_object_item(&mut self, _packet: packets::ObjectItem) {}
    fn on_object_remove(&mut self, _packet: packets::ObjectRemove) {}
    
    // 移动相关
    fn on_object_turn(&mut self, _packet: packets::ObjectTurn) {}
    fn on_object_walk(&mut self, _packet: packets::ObjectWalk) {}
    fn on_object_run(&mut self, _packet: packets::ObjectRun) {}
    
    // 聊天相关
    fn on_chat(&mut self, _packet: packets::Chat) {}
    fn on_object_chat(&mut self, _packet: packets::ObjectChat) {}
    
    // 登录相关
    fn on_login_success(&mut self, _packet: packets::LoginSuccess) {}
    fn on_login(&mut self, _packet: packets::Login) {}
    fn on_new_account(&mut self, _packet: packets::NewAccount) {}
    fn on_change_password(&mut self, _packet: packets::ChangePassword) {}
    fn on_change_password_banned(&mut self, _packet: packets::ChangePasswordBanned) {}
    
    // 角色管理
    fn on_new_character(&mut self, _packet: packets::NewCharacter) {}
    fn on_new_character_success(&mut self, _packet: packets::NewCharacterSuccess) {}
    fn on_delete_character(&mut self, _packet: packets::DeleteCharacter) {}
    fn on_delete_character_success(&mut self, _packet: packets::DeleteCharacterSuccess) {}
    
    // 心跳和时间
    fn on_keep_alive(&mut self, _packet: packets::KeepAlive) {}
    fn on_time_of_day(&mut self, _packet: packets::TimeOfDay) {}
    
    // 战斗相关
    fn on_object_attack(&mut self, _packet: packets::ObjectAttack) {}
    fn on_struck(&mut self, _packet: packets::Struck) {}
    fn on_object_struck(&mut self, _packet: packets::ObjectStruck) {}
    fn on_damage_indicator(&mut self, _packet: packets::DamageIndicator) {}
    fn on_dura_changed(&mut self, _packet: packets::DuraChanged) {}
    fn on_health_changed(&mut self, _packet: packets::HealthChanged) {}
    fn on_death(&mut self, _packet: packets::Death) {}
    fn on_object_died(&mut self, _packet: packets::ObjectDied) {}
    fn on_object_health(&mut self, _packet: packets::ObjectHealth) {}
    
    // 物品相关
    fn on_gained_item(&mut self, _packet: packets::GainedItem) {}
    fn on_gained_gold(&mut self, _packet: packets::GainedGold) {}
    fn on_lose_gold(&mut self, _packet: packets::LoseGold) {}
    fn on_refresh_item(&mut self, _packet: packets::RefreshItem) {}
    fn on_sell_item(&mut self, _packet: packets::SellItem) {}
    fn on_repair_item(&mut self, _packet: packets::RepairItem) {}
    fn on_item_repaired(&mut self, _packet: packets::ItemRepaired) {}
    fn on_split_item(&mut self, _packet: packets::SplitItem) {}
    fn on_split_item1(&mut self, _packet: packets::SplitItem1) {}
    fn on_merge_item(&mut self, _packet: packets::MergeItem) {}
    fn on_remove_item(&mut self, _packet: packets::RemoveItem) {}
    
    // 魔法和技能
    fn on_new_magic(&mut self, _packet: packets::NewMagic) {}
    fn on_magic_leveled(&mut self, _packet: packets::MagicLeveled) {}
    fn on_remove_magic(&mut self, _packet: packets::RemoveMagic) {}
    fn on_spell_toggle(&mut self, _packet: packets::SpellToggle) {}
    fn on_magic(&mut self, _packet: packets::Magic) {}
    fn on_magic_delay(&mut self, _packet: packets::MagicDelay) {}
    fn on_magic_cast(&mut self, _packet: packets::MagicCast) {}
    fn on_object_magic(&mut self, _packet: packets::ObjectMagic) {}
    fn on_object_effect(&mut self, _packet: packets::ObjectEffect) {}
    
    // NPC交互
    fn on_npc_response(&mut self, _packet: packets::NPCResponse) {}
    fn on_npc_goods(&mut self, _packet: packets::NPCGoods) {}
    fn on_npc_update(&mut self, _packet: packets::NPCUpdate) {}
    fn on_npc_image_update(&mut self, _packet: packets::NPCImageUpdate) {}
    fn on_default_npc(&mut self, _packet: packets::DefaultNPC) {}
    fn on_npc_request_input(&mut self, _packet: packets::NPCRequestInput) {}
    
    // 经验和等级
    fn on_gain_experience(&mut self, _packet: packets::GainExperience) {}
    fn on_level_changed(&mut self, _packet: packets::LevelChanged) {}
    
    // Buff和状态
    fn on_add_buff(&mut self, _packet: packets::AddBuff) {}
    fn on_remove_buff(&mut self, _packet: packets::RemoveBuff) {}
    fn on_pause_buff(&mut self, _packet: packets::PauseBuff) {}
    
    // 任务系统
    fn on_change_quest(&mut self, _packet: packets::ChangeQuest) {}
    fn on_new_quest_info(&mut self, _packet: packets::NewQuestInfo) {}
    
    // 重生系统
    fn on_cancel_reincarnation(&mut self, _packet: packets::CancelReincarnation) {}
    fn on_request_reincarnation(&mut self, _packet: packets::RequestReincarnation) {}
    
    // 组队系统
    fn on_switch_group(&mut self, _packet: packets::SwitchGroup) {}
    fn on_delete_group(&mut self, _packet: packets::DeleteGroup) {}
    fn on_delete_member(&mut self, _packet: packets::DeleteMember) {}
    fn on_group_invite(&mut self, _packet: packets::GroupInvite) {}
    fn on_add_member(&mut self, _packet: packets::AddMember) {}
    fn on_group_members_map(&mut self, _packet: packets::GroupMembersMap) {}
    fn on_send_member_location(&mut self, _packet: packets::SendMemberLocation) {}
    
    // 行会系统
    fn on_guild_invite(&mut self, _packet: packets::GuildInvite) {}
    fn on_guild_member_change(&mut self, _packet: packets::GuildMemberChange) {}
    fn on_guild_status(&mut self, _packet: packets::GuildStatus) {}
    
    // 交易系统
    fn on_trade_request(&mut self, _packet: packets::TradeRequest) {}
    fn on_trade_accept(&mut self, _packet: packets::TradeAccept) {}
    fn on_trade_gold(&mut self, _packet: packets::TradeGold) {}
    fn on_trade_item(&mut self, _packet: packets::TradeItem) {}
    fn on_trade_confirm(&mut self, _packet: packets::TradeConfirm) {}
    fn on_trade_cancel(&mut self, _packet: packets::TradeCancel) {}
    
    // 好友系统
    fn on_friend_update(&mut self, _packet: packets::FriendUpdate) {}
    
    // 装备和背包操作
    fn on_move_item(&mut self, _packet: packets::MoveItem) {}
    fn on_equip_item(&mut self, _packet: packets::EquipItem) {}
    fn on_remove_slot_item(&mut self, _packet: packets::RemoveSlotItem) {}
    fn on_take_back_item(&mut self, _packet: packets::TakeBackItem) {}
    fn on_store_item(&mut self, _packet: packets::StoreItem) {}
    fn on_deposit_refine_item(&mut self, _packet: packets::DepositRefineItem) {}
    fn on_retrieve_refine_item(&mut self, _packet: packets::RetrieveRefineItem) {}
    fn on_refine_cancel(&mut self, _packet: packets::RefineCancel) {}
    fn on_refine_item(&mut self, _packet: packets::RefineItem) {}
    fn on_deposit_trade_item(&mut self, _packet: packets::DepositTradeItem) {}
    fn on_retrieve_trade_item(&mut self, _packet: packets::RetrieveTradeItem) {}
    fn on_use_item(&mut self, _packet: packets::UseItem) {}
    fn on_drop_item(&mut self, _packet: packets::DropItem) {}
    fn on_player_update(&mut self, _packet: packets::PlayerUpdate) {}
    fn on_player_inspect(&mut self, _packet: packets::PlayerInspect) {}
    fn on_logout_success(&mut self, _packet: packets::LogOutSuccess) {}
    fn on_logout_failed(&mut self, _packet: packets::LogOutFailed) {}
    fn on_change_a_mode(&mut self, _packet: packets::ChangeAMode) {}
    fn on_change_p_mode(&mut self, _packet: packets::ChangePMode) {}
    fn on_object_name(&mut self, _packet: packets::ObjectName) {}
    fn on_user_storage(&mut self, _packet: packets::UserStorage) {}
    
    // 高级移动
    fn on_user_back_step(&mut self, _packet: packets::UserBackStep) {}
    fn on_object_back_step(&mut self, _packet: packets::ObjectBackStep) {}
    fn on_user_dash_attack(&mut self, _packet: packets::UserDashAttack) {}
    fn on_object_dash_attack(&mut self, _packet: packets::ObjectDashAttack) {}
    fn on_user_attack_move(&mut self, _packet: packets::UserAttackMove) {}
    fn on_set_concentration(&mut self, _packet: packets::SetConcentration) {}
    fn on_object_range_attack(&mut self, _packet: packets::ObjectRangeAttack) {}
    fn on_range_attack(&mut self, _packet: packets::RangeAttack) {}
    fn on_pushed(&mut self, _packet: packets::Pushed) {}
    fn on_object_pushed(&mut self, _packet: packets::ObjectPushed) {}
    fn on_user_dash(&mut self, _packet: packets::UserDash) {}
    fn on_object_dash(&mut self, _packet: packets::ObjectDash) {}
    fn on_user_dash_fail(&mut self, _packet: packets::UserDashFail) {}
    fn on_object_dash_fail(&mut self, _packet: packets::ObjectDashFail) {}
    
    // 掉落物和拾取
    fn on_object_gold(&mut self, _packet: packets::ObjectGold) {}
    fn on_gained_quest_item(&mut self, _packet: packets::GainedQuestItem) {}
    fn on_delete_item(&mut self, _packet: packets::DeleteItem) {}
    
    // 状态和属性
    fn on_revived(&mut self, _packet: packets::Revived) {}
    fn on_object_revived(&mut self, _packet: packets::ObjectRevived) {}
    fn on_hero_health_changed(&mut self, _packet: packets::HeroHealthChanged) {}
    
    // 杂项功能
    fn on_user_slots_refresh(&mut self, _packet: packets::UserSlotsRefresh) {}
    fn on_complete_quest(&mut self, _packet: packets::CompleteQuest) {}
    fn on_share_quest(&mut self, _packet: packets::ShareQuest) {}
    fn on_delete_quest_item(&mut self, _packet: packets::DeleteQuestItem) {}
    fn on_mount_update(&mut self, _packet: packets::MountUpdate) {}
    fn on_fishing_update(&mut self, _packet: packets::FishingUpdate) {}
    fn on_object_sit_down(&mut self, _packet: packets::ObjectSitDown) {}
    fn on_in_trap_rock(&mut self, _packet: packets::InTrapRock) {}
    fn on_base_stats_info(&mut self, _packet: packets::BaseStatsInfo) {}
    fn on_hero_base_stats_info(&mut self, _packet: packets::HeroBaseStatsInfo) {}
    fn on_user_name(&mut self, _packet: packets::UserName) {}
    fn on_chat_item_stats(&mut self, _packet: packets::ChatItemStats) {}
    fn on_guild_exp_gain(&mut self, _packet: packets::GuildExpGain) {}
    fn on_guild_name_request(&mut self, _packet: packets::GuildNameRequest) {}
    fn on_guild_storage_gold_change(&mut self, _packet: packets::GuildStorageGoldChange) {}
    fn on_guild_storage_item_change(&mut self, _packet: packets::GuildStorageItemChange) {}
    fn on_guild_request_war(&mut self, _packet: packets::GuildRequestWar) {}
    
    // 战斗系统扩展
    fn on_object_mana(&mut self, _packet: packets::ObjectMana) {}
    fn on_poisoned(&mut self, _packet: packets::Poisoned) {}
    fn on_object_poisoned(&mut self, _packet: packets::ObjectPoisoned) {}
    fn on_colour_changed(&mut self, _packet: packets::ColourChanged) {}
    fn on_object_colour_changed(&mut self, _packet: packets::ObjectColourChanged) {}
    fn on_object_leveled(&mut self, _packet: packets::ObjectLeveled) {}
    fn on_object_harvest(&mut self, _packet: packets::ObjectHarvest) {}
    fn on_object_harvested(&mut self, _packet: packets::ObjectHarvested) {}
    fn on_object_spell(&mut self, _packet: packets::ObjectSpell) {}
    fn on_object_projectile(&mut self, _packet: packets::ObjectProjectile) {}
    fn on_map_effect(&mut self, _packet: packets::MapEffect) {}
    fn on_object_hidden(&mut self, _packet: packets::ObjectHidden) {}
    fn on_object_sneaking(&mut self, _packet: packets::ObjectSneaking) {}
    fn on_object_level_effects(&mut self, _packet: packets::ObjectLevelEffects) {}
    fn on_set_binding_shot(&mut self, _packet: packets::SetBindingShot) {}
    fn on_set_elemental(&mut self, _packet: packets::SetElemental) {}
    fn on_remove_delayed_explosion(&mut self, _packet: packets::RemoveDelayedExplosion) {}
    fn on_object_deco(&mut self, _packet: packets::ObjectDeco) {}
    
    // 地图和传送
    fn on_map_changed(&mut self, _packet: packets::MapChanged) {}
    fn on_object_teleport_out(&mut self, _packet: packets::ObjectTeleportOut) {}
    fn on_object_teleport_in(&mut self, _packet: packets::ObjectTeleportIn) {}
    fn on_teleport_in(&mut self, _packet: packets::TeleportIn) {}
    fn on_object_hide(&mut self, _packet: packets::ObjectHide) {}
    fn on_object_show(&mut self, _packet: packets::ObjectShow) {}
    fn on_world_map_setup_info(&mut self, _packet: packets::WorldMapSetupInfo) {}
    fn on_search_map_result(&mut self, _packet: packets::SearchMapResult) {}
    
    // NPC商店扩展
    fn on_npc_sell(&mut self, _packet: packets::NPCSell) {}
    fn on_npc_repair(&mut self, _packet: packets::NPCRepair) {}
    fn on_npc_s_repair(&mut self, _packet: packets::NPCSRepair) {}
    fn on_npc_refine(&mut self, _packet: packets::NPCRefine) {}
    fn on_npc_check_refine(&mut self, _packet: packets::NPCCheckRefine) {}
    fn on_npc_collect_refine(&mut self, _packet: packets::NPCCollectRefine) {}
    fn on_npc_replace_wed_ring(&mut self, _packet: packets::NPCReplaceWedRing) {}
    fn on_npc_storage(&mut self, _packet: packets::NPCStorage) {}
    fn on_craft_item(&mut self, _packet: packets::CraftItem) {}
    
    // 物品扩展
    fn on_new_item_info(&mut self, _packet: packets::NewItemInfo) {}
    fn on_new_chat_item(&mut self, _packet: packets::NewChatItem) {}
    fn on_item_slot_size_changed(&mut self, _packet: packets::ItemSlotSizeChanged) {}
    fn on_item_seal_changed(&mut self, _packet: packets::ItemSealChanged) {}
    fn on_combine_item(&mut self, _packet: packets::CombineItem) {}
    fn on_item_upgraded(&mut self, _packet: packets::ItemUpgraded) {}
    fn on_equip_slot_item(&mut self, _packet: packets::EquipSlotItem) {}
    fn on_gained_credit(&mut self, _packet: packets::GainedCredit) {}
    fn on_lose_credit(&mut self, _packet: packets::LoseCredit) {}
    
    // 英雄系统
    fn on_new_hero_info(&mut self, _packet: packets::NewHeroInfo) {}
    fn on_hero_create_request(&mut self, _packet: packets::HeroCreateRequest) {}
    fn on_new_hero(&mut self, _packet: packets::NewHero) {}
    fn on_hero_information(&mut self, _packet: packets::HeroInformation) {}
    fn on_update_hero_spawn_state(&mut self, _packet: packets::UpdateHeroSpawnState) {}
    fn on_unlock_hero_auto_pot(&mut self, _packet: packets::UnlockHeroAutoPot) {}
    fn on_set_auto_pot_value(&mut self, _packet: packets::SetAutoPotValue) {}
    fn on_set_auto_pot_item(&mut self, _packet: packets::SetAutoPotItem) {}
    fn on_set_hero_behaviour(&mut self, _packet: packets::SetHeroBehaviour) {}
    fn on_manage_heroes(&mut self, _packet: packets::ManageHeroes) {}
    fn on_change_hero(&mut self, _packet: packets::ChangeHero) {}
    fn on_take_back_hero_item(&mut self, _packet: packets::TakeBackHeroItem) {}
    fn on_transfer_hero_item(&mut self, _packet: packets::TransferHeroItem) {}
    fn on_gain_hero_experience(&mut self, _packet: packets::GainHeroExperience) {}
    fn on_hero_level_changed(&mut self, _packet: packets::HeroLevelChanged) {}
    
    // 组队和社交扩展
    fn on_allow_observe(&mut self, _packet: packets::AllowObserve) {}
    fn on_marriage_request(&mut self, _packet: packets::MarriageRequest) {}
    fn on_divorce_request(&mut self, _packet: packets::DivorceRequest) {}
    fn on_mentor_request(&mut self, _packet: packets::MentorRequest) {}
    fn on_lover_update(&mut self, _packet: packets::LoverUpdate) {}
    fn on_mentor_update(&mut self, _packet: packets::MentorUpdate) {}
    
    // 行会扩展
    fn on_guild_notice_change(&mut self, _packet: packets::GuildNoticeChange) {}
    fn on_guild_storage_list(&mut self, _packet: packets::GuildStorageList) {}
    fn on_guild_buff_list(&mut self, _packet: packets::GuildBuffList) {}
    fn on_object_guild_name_changed(&mut self, _packet: packets::ObjectGuildNameChanged) {}
    
    // 游戏开始相关
    fn on_start_game(&mut self, _packet: packets::StartGame) {}
    fn on_start_game_banned(&mut self, _packet: packets::StartGameBanned) {}
    fn on_start_game_delay(&mut self, _packet: packets::StartGameDelay) {}
    fn on_return_to_login(&mut self, _packet: packets::ReturnToLogin) {}
    fn on_client_version(&mut self, _packet: packets::ClientVersion) {}
    fn on_login_banned(&mut self, _packet: packets::LoginBanned) {}
    
    // 特殊系统
    fn on_send_output_message(&mut self, _packet: packets::SendOutputMessage) {}
    fn on_open_door(&mut self, _packet: packets::Opendoor) {}
    fn on_resize_inventory(&mut self, _packet: packets::ResizeInventory) {}
    fn on_resize_storage(&mut self, _packet: packets::ResizeStorage) {}
    fn on_transform_update(&mut self, _packet: packets::TransformUpdate) {}
    
    // 智能生物系统
    fn on_new_intelligent_creature(&mut self, _packet: packets::NewIntelligentCreature) {}
    fn on_update_intelligent_creature_list(&mut self, _packet: packets::UpdateIntelligentCreatureList) {}
    fn on_intelligent_creature_enable_rename(&mut self, _packet: packets::IntelligentCreatureEnableRename) {}
    fn on_intelligent_creature_pickup(&mut self, _packet: packets::IntelligentCreaturePickup) {}
    fn on_npc_pearl_goods(&mut self, _packet: packets::NPCPearlGoods) {}
    
    // 游戏商店系统
    fn on_game_shop_info(&mut self, _packet: packets::GameShopInfo) {}
    fn on_game_shop_stock(&mut self, _packet: packets::GameShopStock) {}
    
    // 排名系统
    fn on_rankings(&mut self, _packet: packets::Rankings) {}
    
    // 行会领地系统
    fn on_guild_territory_page(&mut self, _packet: packets::GuildTerritoryPage) {}
    fn on_purchase_guild_territory(&mut self, _packet: packets::PurchaseGuildTerritory) {}
    
    // 邮件系统
    fn on_receive_mail(&mut self, _packet: packets::ReceiveMail) {}
    fn on_mail_locked_item(&mut self, _packet: packets::MailLockedItem) {}
    fn on_mail_send_request(&mut self, _packet: packets::MailSendRequest) {}
    fn on_mail_sent(&mut self, _packet: packets::MailSent) {}
    fn on_parcel_collected(&mut self, _packet: packets::ParcelCollected) {}
    fn on_mail_cost(&mut self, _packet: packets::MailCost) {}
    
    // 市场系统
    fn on_npc_consign(&mut self, _packet: packets::NPCConsign) {}
    fn on_npc_market(&mut self, _packet: packets::NPCMarket) {}
    fn on_npc_market_page(&mut self, _packet: packets::NPCMarketPage) {}
    fn on_consign_item(&mut self, _packet: packets::ConsignItem) {}
    fn on_market_fail(&mut self, _packet: packets::MarketFail) {}
    fn on_market_success(&mut self, _packet: packets::MarketSuccess) {}
    
    // 觉醒系统
    fn on_npc_awakening(&mut self, _packet: packets::NPCAwakening) {}
    fn on_npc_disassemble(&mut self, _packet: packets::NPCDisassemble) {}
    fn on_npc_downgrade(&mut self, _packet: packets::NPCDowngrade) {}
    fn on_npc_reset(&mut self, _packet: packets::NPCReset) {}
    fn on_awakening_need_materials(&mut self, _packet: packets::AwakeningNeedMaterials) {}
    fn on_awakening_locked_item(&mut self, _packet: packets::AwakeningLockedItem) {}
    fn on_awakening(&mut self, _packet: packets::Awakening) {}
    
    // 租赁系统
    fn on_get_rented_items(&mut self, _packet: packets::GetRentedItems) {}
    fn on_item_rental_request(&mut self, _packet: packets::ItemRentalRequest) {}
    fn on_item_rental_fee(&mut self, _packet: packets::ItemRentalFee) {}
    fn on_item_rental_period(&mut self, _packet: packets::ItemRentalPeriod) {}
    fn on_deposit_rental_item(&mut self, _packet: packets::DepositRentalItem) {}
    fn on_retrieve_rental_item(&mut self, _packet: packets::RetrieveRentalItem) {}
    fn on_update_rental_item(&mut self, _packet: packets::UpdateRentalItem) {}
    fn on_cancel_item_rental(&mut self, _packet: packets::CancelItemRental) {}
    fn on_item_rental_lock(&mut self, _packet: packets::ItemRentalLock) {}
    fn on_item_rental_partner_lock(&mut self, _packet: packets::ItemRentalPartnerLock) {}
    fn on_can_confirm_item_rental(&mut self, _packet: packets::CanConfirmItemRental) {}
    fn on_confirm_item_rental(&mut self, _packet: packets::ConfirmItemRental) {}
    
    // UI事件系统
    fn on_new_recipe_info(&mut self, _packet: packets::NewRecipeInfo) {}
    fn on_open_browser(&mut self, _packet: packets::OpenBrowser) {}
    fn on_play_sound(&mut self, _packet: packets::PlaySound) {}
    fn on_set_timer(&mut self, _packet: packets::SetTimer) {}
    fn on_expire_timer(&mut self, _packet: packets::ExpireTimer) {}
    fn on_update_notice(&mut self, _packet: packets::UpdateNotice) {}
    fn on_roll(&mut self, _packet: packets::Roll) {}
    fn on_set_compass(&mut self, _packet: packets::SetCompass) {}
    
    // 默认处理 - 当收到未知或未实现的数据包时调用
    fn on_unknown_packet(&mut self, opcode: i16, _data: &[u8]) {
        tracing::warn!("收到未处理的数据包: opcode={}", opcode);
    }
}

// ============================================================================
// 数据包分发器
// ============================================================================

/// 数据包分发器 - 将原始字节流分发到对应的处理函数
///
/// 这个函数根据 opcode 解析数据包并调用 handler 的相应方法。
///
/// # 参数
/// - `header`: 数据包头部信息
/// - `data`: 完整的数据包数据 (包括头部)
/// - `handler`: 实现了 PacketHandler trait 的处理器
///
/// # 返回
/// - `Ok(())`: 成功处理
/// - `Err`: 解析或处理失败
///
/// # 示例
/// ```ignore
/// let header = parse_packet_header(&data)?;
/// dispatch_packet(header, &data, &mut my_handler)?;
/// ```
pub fn dispatch_packet<H: PacketHandler>(
    header: PacketHeader,
    data: &[u8],
    handler: &mut H,
) -> Result<()> {
    // 获取数据包体
    let body = get_packet_body(data)?;
    let mut cursor = Cursor::new(body);
    
    // 根据 opcode 分发到对应的处理函数
    match header.opcode as u16 {
        // 连接相关
        x if x == ServerPacketIds::Connected as u16 => {
            let packet = packets::Connected::read_body(&mut cursor)?;
            handler.on_connected(packet);
        }
        x if x == ServerPacketIds::Disconnect as u16 => {
            let packet = packets::Disconnect::read_body(&mut cursor)?;
            handler.on_disconnect(packet);
        }
        
        // 用户信息
        x if x == ServerPacketIds::UserInformation as u16 => {
            let packet = packets::UserInformation::read_body(&mut cursor)?;
            handler.on_user_information(packet);
        }
        x if x == ServerPacketIds::UserLocation as u16 => {
            let packet = packets::UserLocation::read_body(&mut cursor)?;
            handler.on_user_location(packet);
        }
        
        // 地图相关
        x if x == ServerPacketIds::MapInformation as u16 => {
            tracing::debug!("📥 Received MapInformation packet (opcode=17)");
            let packet = packets::MapInformation::read_body(&mut cursor)?;
            tracing::debug!("📥 MapInformation parsed: {} ({})", packet.title, packet.file_name);
            handler.on_map_information(packet);
        }
        x if x == ServerPacketIds::NewMapInfo as u16 => {
            let packet = packets::NewMapInfo::read_body(&mut cursor)?;
            handler.on_new_map_info(packet);
        }
        
        // 对象相关
        x if x == ServerPacketIds::ObjectPlayer as u16 => {
            let packet = packets::ObjectPlayer::read_body(&mut cursor)?;
            handler.on_object_player(packet);
        }
        x if x == ServerPacketIds::ObjectHero as u16 => {
            let packet = packets::ObjectHero::read_body(&mut cursor)?;
            handler.on_object_hero(packet);
        }
        x if x == ServerPacketIds::ObjectMonster as u16 => {
            let packet = packets::ObjectMonster::read_body(&mut cursor)?;
            handler.on_object_monster(packet);
        }
        x if x == ServerPacketIds::ObjectNpc as u16 => {
            let packet = packets::ObjectNpc::read_body(&mut cursor)?;
            handler.on_object_npc(packet);
        }
        x if x == ServerPacketIds::ObjectItem as u16 => {
            let packet = packets::ObjectItem::read_body(&mut cursor)?;
            handler.on_object_item(packet);
        }
        
        // 对象移除
        x if x == ServerPacketIds::ObjectRemove as u16 => {
            let packet = packets::ObjectRemove::read_body(&mut cursor)?;
            handler.on_object_remove(packet);
        }
        
        // 对象转向
        x if x == ServerPacketIds::ObjectTurn as u16 => {
            let packet = packets::ObjectTurn::read_body(&mut cursor)?;
            handler.on_object_turn(packet);
        }
        x if x == ServerPacketIds::ObjectWalk as u16 => {
            let packet = packets::ObjectWalk::read_body(&mut cursor)?;
            handler.on_object_walk(packet);
        }
        x if x == ServerPacketIds::ObjectRun as u16 => {
            let packet = packets::ObjectRun::read_body(&mut cursor)?;
            handler.on_object_run(packet);
        }
        
        // 聊天消息
        x if x == ServerPacketIds::Chat as u16 => {
            let packet = packets::Chat::read_body(&mut cursor)?;
            handler.on_chat(packet);
        }
        
        // 对象聊天
        x if x == ServerPacketIds::ObjectChat as u16 => {
            let packet = packets::ObjectChat::read_body(&mut cursor)?;
            handler.on_object_chat(packet);
        }
        
        // 登录相关
        x if x == ServerPacketIds::LoginSuccess as u16 => {
            let packet = packets::LoginSuccess::read_body(&mut cursor)?;
            handler.on_login_success(packet);
        }
        x if x == ServerPacketIds::Login as u16 => {
            let packet = packets::Login::read_body(&mut cursor)?;
            handler.on_login(packet);
        }
        x if x == ServerPacketIds::NewAccount as u16 => {
            let packet = packets::NewAccount::read_body(&mut cursor)?;
            tracing::info!("📦 收到NewAccount响应: result={}", packet.result);
            handler.on_new_account(packet);
        }
        x if x == ServerPacketIds::ChangePassword as u16 => {
            let packet = packets::ChangePassword::read_body(&mut cursor)?;
            handler.on_change_password(packet);
        }
        x if x == ServerPacketIds::ChangePasswordBanned as u16 => {
            let packet = packets::ChangePasswordBanned::read_body(&mut cursor)?;
            handler.on_change_password_banned(packet);
        }
        
        // 角色选择
        x if x == ServerPacketIds::NewCharacter as u16 => {
            let packet = packets::NewCharacter::read_body(&mut cursor)?;
            handler.on_new_character(packet);
        }
        x if x == ServerPacketIds::NewCharacterSuccess as u16 => {
            let packet = packets::NewCharacterSuccess::read_body(&mut cursor)?;
            handler.on_new_character_success(packet);
        }
        x if x == ServerPacketIds::DeleteCharacter as u16 => {
            let packet = packets::DeleteCharacter::read_body(&mut cursor)?;
            handler.on_delete_character(packet);
        }
        x if x == ServerPacketIds::DeleteCharacterSuccess as u16 => {
            let packet = packets::DeleteCharacterSuccess::read_body(&mut cursor)?;
            handler.on_delete_character_success(packet);
        }
        
        // 心跳
        x if x == ServerPacketIds::KeepAlive as u16 => {
            let packet = packets::KeepAlive::read_body(&mut cursor)?;
            handler.on_keep_alive(packet);
        }
        x if x == ServerPacketIds::TimeOfDay as u16 => {
            let packet = packets::TimeOfDay::read_body(&mut cursor)?;
            handler.on_time_of_day(packet);
        }
        
        // 战斗相关
        x if x == ServerPacketIds::ObjectAttack as u16 => {
            let packet = packets::ObjectAttack::read_body(&mut cursor)?;
            handler.on_object_attack(packet);
        }
        x if x == ServerPacketIds::Struck as u16 => {
            let packet = packets::Struck::read_body(&mut cursor)?;
            handler.on_struck(packet);
        }
        x if x == ServerPacketIds::ObjectStruck as u16 => {
            let packet = packets::ObjectStruck::read_body(&mut cursor)?;
            handler.on_object_struck(packet);
        }
        x if x == ServerPacketIds::DamageIndicator as u16 => {
            let packet = packets::DamageIndicator::read_body(&mut cursor)?;
            handler.on_damage_indicator(packet);
        }
        x if x == ServerPacketIds::DuraChanged as u16 => {
            let packet = packets::DuraChanged::read_body(&mut cursor)?;
            handler.on_dura_changed(packet);
        }
        x if x == ServerPacketIds::HealthChanged as u16 => {
            let packet = packets::HealthChanged::read_body(&mut cursor)?;
            handler.on_health_changed(packet);
        }
        x if x == ServerPacketIds::Death as u16 => {
            let packet = packets::Death::read_body(&mut cursor)?;
            handler.on_death(packet);
        }
        x if x == ServerPacketIds::ObjectDied as u16 => {
            let packet = packets::ObjectDied::read_body(&mut cursor)?;
            handler.on_object_died(packet);
        }
        x if x == ServerPacketIds::ObjectHealth as u16 => {
            let packet = packets::ObjectHealth::read_body(&mut cursor)?;
            handler.on_object_health(packet);
        }
        
        // 物品和背包
        x if x == ServerPacketIds::GainedItem as u16 => {
            let packet = packets::GainedItem::read_body(&mut cursor)?;
            handler.on_gained_item(packet);
        }
        x if x == ServerPacketIds::GainedGold as u16 => {
            let packet = packets::GainedGold::read_body(&mut cursor)?;
            handler.on_gained_gold(packet);
        }
        x if x == ServerPacketIds::LoseGold as u16 => {
            let packet = packets::LoseGold::read_body(&mut cursor)?;
            handler.on_lose_gold(packet);
        }
        x if x == ServerPacketIds::RefreshItem as u16 => {
            let packet = packets::RefreshItem::read_body(&mut cursor)?;
            handler.on_refresh_item(packet);
        }
        x if x == ServerPacketIds::SellItem as u16 => {
            let packet = packets::SellItem::read_body(&mut cursor)?;
            handler.on_sell_item(packet);
        }
        x if x == ServerPacketIds::RepairItem as u16 => {
            let packet = packets::RepairItem::read_body(&mut cursor)?;
            handler.on_repair_item(packet);
        }
        x if x == ServerPacketIds::ItemRepaired as u16 => {
            let packet = packets::ItemRepaired::read_body(&mut cursor)?;
            handler.on_item_repaired(packet);
        }
        x if x == ServerPacketIds::SplitItem as u16 => {
            let packet = packets::SplitItem::read_body(&mut cursor)?;
            handler.on_split_item(packet);
        }
        x if x == ServerPacketIds::SplitItem1 as u16 => {
            let packet = packets::SplitItem1::read_body(&mut cursor)?;
            handler.on_split_item1(packet);
        }
        x if x == ServerPacketIds::MergeItem as u16 => {
            let packet = packets::MergeItem::read_body(&mut cursor)?;
            handler.on_merge_item(packet);
        }
        x if x == ServerPacketIds::RemoveItem as u16 => {
            let packet = packets::RemoveItem::read_body(&mut cursor)?;
            handler.on_remove_item(packet);
        }
        
        // 魔法和技能
        x if x == ServerPacketIds::NewMagic as u16 => {
            let packet = packets::NewMagic::read_body(&mut cursor)?;
            handler.on_new_magic(packet);
        }
        x if x == ServerPacketIds::MagicLeveled as u16 => {
            let packet = packets::MagicLeveled::read_body(&mut cursor)?;
            handler.on_magic_leveled(packet);
        }
        x if x == ServerPacketIds::RemoveMagic as u16 => {
            let packet = packets::RemoveMagic::read_body(&mut cursor)?;
            handler.on_remove_magic(packet);
        }
        x if x == ServerPacketIds::SpellToggle as u16 => {
            let packet = packets::SpellToggle::read_body(&mut cursor)?;
            handler.on_spell_toggle(packet);
        }
        x if x == ServerPacketIds::Magic as u16 => {
            let packet = packets::Magic::read_body(&mut cursor)?;
            handler.on_magic(packet);
        }
        x if x == ServerPacketIds::MagicDelay as u16 => {
            let packet = packets::MagicDelay::read_body(&mut cursor)?;
            handler.on_magic_delay(packet);
        }
        x if x == ServerPacketIds::MagicCast as u16 => {
            let packet = packets::MagicCast::read_body(&mut cursor)?;
            handler.on_magic_cast(packet);
        }
        x if x == ServerPacketIds::ObjectMagic as u16 => {
            let packet = packets::ObjectMagic::read_body(&mut cursor)?;
            handler.on_object_magic(packet);
        }
        x if x == ServerPacketIds::ObjectEffect as u16 => {
            let packet = packets::ObjectEffect::read_body(&mut cursor)?;
            handler.on_object_effect(packet);
        }
        
        // NPC交互
        x if x == ServerPacketIds::NPCResponse as u16 => {
            let packet = packets::NPCResponse::read_body(&mut cursor)?;
            handler.on_npc_response(packet);
        }
        x if x == ServerPacketIds::NPCGoods as u16 => {
            let packet = packets::NPCGoods::read_body(&mut cursor)?;
            handler.on_npc_goods(packet);
        }
        x if x == ServerPacketIds::NPCUpdate as u16 => {
            let packet = packets::NPCUpdate::read_body(&mut cursor)?;
            handler.on_npc_update(packet);
        }
        x if x == ServerPacketIds::NPCImageUpdate as u16 => {
            let packet = packets::NPCImageUpdate::read_body(&mut cursor)?;
            handler.on_npc_image_update(packet);
        }
        x if x == ServerPacketIds::DefaultNPC as u16 => {
            let packet = packets::DefaultNPC::read_body(&mut cursor)?;
            handler.on_default_npc(packet);
        }
        x if x == ServerPacketIds::NPCRequestInput as u16 => {
            let packet = packets::NPCRequestInput::read_body(&mut cursor)?;
            handler.on_npc_request_input(packet);
        }
        
        // 经验和等级
        x if x == ServerPacketIds::GainExperience as u16 => {
            let packet = packets::GainExperience::read_body(&mut cursor)?;
            handler.on_gain_experience(packet);
        }
        x if x == ServerPacketIds::LevelChanged as u16 => {
            let packet = packets::LevelChanged::read_body(&mut cursor)?;
            handler.on_level_changed(packet);
        }
        
        // Buff和状态
        x if x == ServerPacketIds::AddBuff as u16 => {
            let packet = packets::AddBuff::read_body(&mut cursor)?;
            handler.on_add_buff(packet);
        }
        x if x == ServerPacketIds::RemoveBuff as u16 => {
            let packet = packets::RemoveBuff::read_body(&mut cursor)?;
            handler.on_remove_buff(packet);
        }
        x if x == ServerPacketIds::PauseBuff as u16 => {
            let packet = packets::PauseBuff::read_body(&mut cursor)?;
            handler.on_pause_buff(packet);
        }
        
        // 任务系统
        x if x == ServerPacketIds::ChangeQuest as u16 => {
            let packet = packets::ChangeQuest::read_body(&mut cursor)?;
            handler.on_change_quest(packet);
        }
        x if x == ServerPacketIds::NewQuestInfo as u16 => {
            let packet = packets::NewQuestInfo::read_body(&mut cursor)?;
            handler.on_new_quest_info(packet);
        }
        
        // 重生系统
        x if x == ServerPacketIds::CancelReincarnation as u16 => {
            let packet = packets::CancelReincarnation::read_body(&mut cursor)?;
            handler.on_cancel_reincarnation(packet);
        }
        x if x == ServerPacketIds::RequestReincarnation as u16 => {
            let packet = packets::RequestReincarnation::read_body(&mut cursor)?;
            handler.on_request_reincarnation(packet);
        }
        
        // 组队系统
        x if x == ServerPacketIds::SwitchGroup as u16 => {
            let packet = packets::SwitchGroup::read_body(&mut cursor)?;
            handler.on_switch_group(packet);
        }
        x if x == ServerPacketIds::DeleteGroup as u16 => {
            let packet = packets::DeleteGroup::read_body(&mut cursor)?;
            handler.on_delete_group(packet);
        }
        x if x == ServerPacketIds::DeleteMember as u16 => {
            let packet = packets::DeleteMember::read_body(&mut cursor)?;
            handler.on_delete_member(packet);
        }
        x if x == ServerPacketIds::GroupInvite as u16 => {
            let packet = packets::GroupInvite::read_body(&mut cursor)?;
            handler.on_group_invite(packet);
        }
        x if x == ServerPacketIds::AddMember as u16 => {
            let packet = packets::AddMember::read_body(&mut cursor)?;
            handler.on_add_member(packet);
        }
        x if x == ServerPacketIds::GroupMembersMap as u16 => {
            let packet = packets::GroupMembersMap::read_body(&mut cursor)?;
            handler.on_group_members_map(packet);
        }
        x if x == ServerPacketIds::SendMemberLocation as u16 => {
            let packet = packets::SendMemberLocation::read_body(&mut cursor)?;
            handler.on_send_member_location(packet);
        }
        
        // 行会系统
        x if x == ServerPacketIds::GuildInvite as u16 => {
            let packet = packets::GuildInvite::read_body(&mut cursor)?;
            handler.on_guild_invite(packet);
        }
        x if x == ServerPacketIds::GuildMemberChange as u16 => {
            let packet = packets::GuildMemberChange::read_body(&mut cursor)?;
            handler.on_guild_member_change(packet);
        }
        x if x == ServerPacketIds::GuildStatus as u16 => {
            let packet = packets::GuildStatus::read_body(&mut cursor)?;
            handler.on_guild_status(packet);
        }
        
        // 交易系统
        x if x == ServerPacketIds::TradeRequest as u16 => {
            let packet = packets::TradeRequest::read_body(&mut cursor)?;
            handler.on_trade_request(packet);
        }
        x if x == ServerPacketIds::TradeAccept as u16 => {
            let packet = packets::TradeAccept::read_body(&mut cursor)?;
            handler.on_trade_accept(packet);
        }
        x if x == ServerPacketIds::TradeGold as u16 => {
            let packet = packets::TradeGold::read_body(&mut cursor)?;
            handler.on_trade_gold(packet);
        }
        x if x == ServerPacketIds::TradeItem as u16 => {
            let packet = packets::TradeItem::read_body(&mut cursor)?;
            handler.on_trade_item(packet);
        }
        x if x == ServerPacketIds::TradeConfirm as u16 => {
            let packet = packets::TradeConfirm::read_body(&mut cursor)?;
            handler.on_trade_confirm(packet);
        }
        x if x == ServerPacketIds::TradeCancel as u16 => {
            let packet = packets::TradeCancel::read_body(&mut cursor)?;
            handler.on_trade_cancel(packet);
        }
        
        // 好友系统
        x if x == ServerPacketIds::FriendUpdate as u16 => {
            let packet = packets::FriendUpdate::read_body(&mut cursor)?;
            handler.on_friend_update(packet);
        }
        
        // 装备和背包操作
        x if x == ServerPacketIds::MoveItem as u16 => {
            let packet = packets::MoveItem::read_body(&mut cursor)?;
            handler.on_move_item(packet);
        }
        x if x == ServerPacketIds::EquipItem as u16 => {
            let packet = packets::EquipItem::read_body(&mut cursor)?;
            handler.on_equip_item(packet);
        }
        x if x == ServerPacketIds::RemoveSlotItem as u16 => {
            let packet = packets::RemoveSlotItem::read_body(&mut cursor)?;
            handler.on_remove_slot_item(packet);
        }
        x if x == ServerPacketIds::TakeBackItem as u16 => {
            let packet = packets::TakeBackItem::read_body(&mut cursor)?;
            handler.on_take_back_item(packet);
        }
        x if x == ServerPacketIds::StoreItem as u16 => {
            let packet = packets::StoreItem::read_body(&mut cursor)?;
            handler.on_store_item(packet);
        }
        x if x == ServerPacketIds::DepositRefineItem as u16 => {
            let packet = packets::DepositRefineItem::read_body(&mut cursor)?;
            handler.on_deposit_refine_item(packet);
        }
        x if x == ServerPacketIds::RetrieveRefineItem as u16 => {
            let packet = packets::RetrieveRefineItem::read_body(&mut cursor)?;
            handler.on_retrieve_refine_item(packet);
        }
        x if x == ServerPacketIds::RefineCancel as u16 => {
            let packet = packets::RefineCancel::read_body(&mut cursor)?;
            handler.on_refine_cancel(packet);
        }
        x if x == ServerPacketIds::RefineItem as u16 => {
            let packet = packets::RefineItem::read_body(&mut cursor)?;
            handler.on_refine_item(packet);
        }
        x if x == ServerPacketIds::DepositTradeItem as u16 => {
            let packet = packets::DepositTradeItem::read_body(&mut cursor)?;
            handler.on_deposit_trade_item(packet);
        }
        x if x == ServerPacketIds::RetrieveTradeItem as u16 => {
            let packet = packets::RetrieveTradeItem::read_body(&mut cursor)?;
            handler.on_retrieve_trade_item(packet);
        }
        x if x == ServerPacketIds::UseItem as u16 => {
            let packet = packets::UseItem::read_body(&mut cursor)?;
            handler.on_use_item(packet);
        }
        x if x == ServerPacketIds::DropItem as u16 => {
            let packet = packets::DropItem::read_body(&mut cursor)?;
            handler.on_drop_item(packet);
        }
        
        // 玩家状态
        x if x == ServerPacketIds::PlayerUpdate as u16 => {
            let packet = packets::PlayerUpdate::read_body(&mut cursor)?;
            handler.on_player_update(packet);
        }
        x if x == ServerPacketIds::PlayerInspect as u16 => {
            let packet = packets::PlayerInspect::read_body(&mut cursor)?;
            handler.on_player_inspect(packet);
        }
        x if x == ServerPacketIds::LogOutSuccess as u16 => {
            let packet = packets::LogOutSuccess::read_body(&mut cursor)?;
            handler.on_logout_success(packet);
        }
        x if x == ServerPacketIds::LogOutFailed as u16 => {
            let packet = packets::LogOutFailed::read_body(&mut cursor)?;
            handler.on_logout_failed(packet);
        }
        x if x == ServerPacketIds::ChangeAMode as u16 => {
            let packet = packets::ChangeAMode::read_body(&mut cursor)?;
            handler.on_change_a_mode(packet);
        }
        x if x == ServerPacketIds::ChangePMode as u16 => {
            let packet = packets::ChangePMode::read_body(&mut cursor)?;
            handler.on_change_p_mode(packet);
        }
        x if x == ServerPacketIds::ObjectName as u16 => {
            let packet = packets::ObjectName::read_body(&mut cursor)?;
            handler.on_object_name(packet);
        }
        x if x == ServerPacketIds::UserStorage as u16 => {
            let packet = packets::UserStorage::read_body(&mut cursor)?;
            handler.on_user_storage(packet);
        }
        
        // 高级移动
        x if x == ServerPacketIds::UserBackStep as u16 => {
            let packet = packets::UserBackStep::read_body(&mut cursor)?;
            handler.on_user_back_step(packet);
        }
        x if x == ServerPacketIds::ObjectBackStep as u16 => {
            let packet = packets::ObjectBackStep::read_body(&mut cursor)?;
            handler.on_object_back_step(packet);
        }
        x if x == ServerPacketIds::UserDashAttack as u16 => {
            let packet = packets::UserDashAttack::read_body(&mut cursor)?;
            handler.on_user_dash_attack(packet);
        }
        x if x == ServerPacketIds::ObjectDashAttack as u16 => {
            let packet = packets::ObjectDashAttack::read_body(&mut cursor)?;
            handler.on_object_dash_attack(packet);
        }
        x if x == ServerPacketIds::UserAttackMove as u16 => {
            let packet = packets::UserAttackMove::read_body(&mut cursor)?;
            handler.on_user_attack_move(packet);
        }
        x if x == ServerPacketIds::SetConcentration as u16 => {
            let packet = packets::SetConcentration::read_body(&mut cursor)?;
            handler.on_set_concentration(packet);
        }
        x if x == ServerPacketIds::ObjectRangeAttack as u16 => {
            let packet = packets::ObjectRangeAttack::read_body(&mut cursor)?;
            handler.on_object_range_attack(packet);
        }
        x if x == ServerPacketIds::RangeAttack as u16 => {
            let packet = packets::RangeAttack::read_body(&mut cursor)?;
            handler.on_range_attack(packet);
        }
        x if x == ServerPacketIds::Pushed as u16 => {
            let packet = packets::Pushed::read_body(&mut cursor)?;
            handler.on_pushed(packet);
        }
        x if x == ServerPacketIds::ObjectPushed as u16 => {
            let packet = packets::ObjectPushed::read_body(&mut cursor)?;
            handler.on_object_pushed(packet);
        }
        x if x == ServerPacketIds::UserDash as u16 => {
            let packet = packets::UserDash::read_body(&mut cursor)?;
            handler.on_user_dash(packet);
        }
        x if x == ServerPacketIds::ObjectDash as u16 => {
            let packet = packets::ObjectDash::read_body(&mut cursor)?;
            handler.on_object_dash(packet);
        }
        x if x == ServerPacketIds::UserDashFail as u16 => {
            let packet = packets::UserDashFail::read_body(&mut cursor)?;
            handler.on_user_dash_fail(packet);
        }
        x if x == ServerPacketIds::ObjectDashFail as u16 => {
            let packet = packets::ObjectDashFail::read_body(&mut cursor)?;
            handler.on_object_dash_fail(packet);
        }
        
        // 掉落物和拾取
        x if x == ServerPacketIds::ObjectGold as u16 => {
            let packet = packets::ObjectGold::read_body(&mut cursor)?;
            handler.on_object_gold(packet);
        }
        x if x == ServerPacketIds::GainedQuestItem as u16 => {
            let packet = packets::GainedQuestItem::read_body(&mut cursor)?;
            handler.on_gained_quest_item(packet);
        }
        x if x == ServerPacketIds::DeleteItem as u16 => {
            let packet = packets::DeleteItem::read_body(&mut cursor)?;
            handler.on_delete_item(packet);
        }
        
        // 状态和属性
        x if x == ServerPacketIds::Revived as u16 => {
            let packet = packets::Revived::read_body(&mut cursor)?;
            handler.on_revived(packet);
        }
        x if x == ServerPacketIds::ObjectRevived as u16 => {
            let packet = packets::ObjectRevived::read_body(&mut cursor)?;
            handler.on_object_revived(packet);
        }
        x if x == ServerPacketIds::HeroHealthChanged as u16 => {
            let packet = packets::HeroHealthChanged::read_body(&mut cursor)?;
            handler.on_hero_health_changed(packet);
        }
        
        // 杂项功能
        x if x == ServerPacketIds::UserSlotsRefresh as u16 => {
            let packet = packets::UserSlotsRefresh::read_body(&mut cursor)?;
            handler.on_user_slots_refresh(packet);
        }
        x if x == ServerPacketIds::CompleteQuest as u16 => {
            let packet = packets::CompleteQuest::read_body(&mut cursor)?;
            handler.on_complete_quest(packet);
        }
        x if x == ServerPacketIds::ShareQuest as u16 => {
            let packet = packets::ShareQuest::read_body(&mut cursor)?;
            handler.on_share_quest(packet);
        }
        x if x == ServerPacketIds::DeleteQuestItem as u16 => {
            let packet = packets::DeleteQuestItem::read_body(&mut cursor)?;
            handler.on_delete_quest_item(packet);
        }
        x if x == ServerPacketIds::MountUpdate as u16 => {
            let packet = packets::MountUpdate::read_body(&mut cursor)?;
            handler.on_mount_update(packet);
        }
        x if x == ServerPacketIds::FishingUpdate as u16 => {
            let packet = packets::FishingUpdate::read_body(&mut cursor)?;
            handler.on_fishing_update(packet);
        }
        x if x == ServerPacketIds::ObjectSitDown as u16 => {
            let packet = packets::ObjectSitDown::read_body(&mut cursor)?;
            handler.on_object_sit_down(packet);
        }
        x if x == ServerPacketIds::InTrapRock as u16 => {
            let packet = packets::InTrapRock::read_body(&mut cursor)?;
            handler.on_in_trap_rock(packet);
        }
        x if x == ServerPacketIds::BaseStatsInfo as u16 => {
            let packet = packets::BaseStatsInfo::read_body(&mut cursor)?;
            handler.on_base_stats_info(packet);
        }
        x if x == ServerPacketIds::HeroBaseStatsInfo as u16 => {
            let packet = packets::HeroBaseStatsInfo::read_body(&mut cursor)?;
            handler.on_hero_base_stats_info(packet);
        }
        x if x == ServerPacketIds::UserName as u16 => {
            let packet = packets::UserName::read_body(&mut cursor)?;
            handler.on_user_name(packet);
        }
        x if x == ServerPacketIds::ChatItemStats as u16 => {
            let packet = packets::ChatItemStats::read_body(&mut cursor)?;
            handler.on_chat_item_stats(packet);
        }
        x if x == ServerPacketIds::GuildExpGain as u16 => {
            let packet = packets::GuildExpGain::read_body(&mut cursor)?;
            handler.on_guild_exp_gain(packet);
        }
        x if x == ServerPacketIds::GuildNameRequest as u16 => {
            let packet = packets::GuildNameRequest::read_body(&mut cursor)?;
            handler.on_guild_name_request(packet);
        }
        x if x == ServerPacketIds::GuildStorageGoldChange as u16 => {
            let packet = packets::GuildStorageGoldChange::read_body(&mut cursor)?;
            handler.on_guild_storage_gold_change(packet);
        }
        x if x == ServerPacketIds::GuildStorageItemChange as u16 => {
            let packet = packets::GuildStorageItemChange::read_body(&mut cursor)?;
            handler.on_guild_storage_item_change(packet);
        }
        x if x == ServerPacketIds::GuildRequestWar as u16 => {
            let packet = packets::GuildRequestWar::read_body(&mut cursor)?;
            handler.on_guild_request_war(packet);
        }
        
        // 战斗系统扩展
        x if x == ServerPacketIds::ObjectMana as u16 => {
            let packet = packets::ObjectMana::read_body(&mut cursor)?;
            handler.on_object_mana(packet);
        }
        x if x == ServerPacketIds::Poisoned as u16 => {
            let packet = packets::Poisoned::read_body(&mut cursor)?;
            handler.on_poisoned(packet);
        }
        x if x == ServerPacketIds::ObjectPoisoned as u16 => {
            let packet = packets::ObjectPoisoned::read_body(&mut cursor)?;
            handler.on_object_poisoned(packet);
        }
        x if x == ServerPacketIds::ColourChanged as u16 => {
            let packet = packets::ColourChanged::read_body(&mut cursor)?;
            handler.on_colour_changed(packet);
        }
        x if x == ServerPacketIds::ObjectColourChanged as u16 => {
            let packet = packets::ObjectColourChanged::read_body(&mut cursor)?;
            handler.on_object_colour_changed(packet);
        }
        x if x == ServerPacketIds::ObjectLeveled as u16 => {
            let packet = packets::ObjectLeveled::read_body(&mut cursor)?;
            handler.on_object_leveled(packet);
        }
        x if x == ServerPacketIds::ObjectHarvest as u16 => {
            let packet = packets::ObjectHarvest::read_body(&mut cursor)?;
            handler.on_object_harvest(packet);
        }
        x if x == ServerPacketIds::ObjectHarvested as u16 => {
            let packet = packets::ObjectHarvested::read_body(&mut cursor)?;
            handler.on_object_harvested(packet);
        }
        x if x == ServerPacketIds::ObjectSpell as u16 => {
            let packet = packets::ObjectSpell::read_body(&mut cursor)?;
            handler.on_object_spell(packet);
        }
        x if x == ServerPacketIds::ObjectProjectile as u16 => {
            let packet = packets::ObjectProjectile::read_body(&mut cursor)?;
            handler.on_object_projectile(packet);
        }
        x if x == ServerPacketIds::MapEffect as u16 => {
            let packet = packets::MapEffect::read_body(&mut cursor)?;
            handler.on_map_effect(packet);
        }
        x if x == ServerPacketIds::ObjectHidden as u16 => {
            let packet = packets::ObjectHidden::read_body(&mut cursor)?;
            handler.on_object_hidden(packet);
        }
        x if x == ServerPacketIds::ObjectSneaking as u16 => {
            let packet = packets::ObjectSneaking::read_body(&mut cursor)?;
            handler.on_object_sneaking(packet);
        }
        x if x == ServerPacketIds::ObjectLevelEffects as u16 => {
            let packet = packets::ObjectLevelEffects::read_body(&mut cursor)?;
            handler.on_object_level_effects(packet);
        }
        x if x == ServerPacketIds::SetBindingShot as u16 => {
            let packet = packets::SetBindingShot::read_body(&mut cursor)?;
            handler.on_set_binding_shot(packet);
        }
        x if x == ServerPacketIds::SetElemental as u16 => {
            let packet = packets::SetElemental::read_body(&mut cursor)?;
            handler.on_set_elemental(packet);
        }
        x if x == ServerPacketIds::RemoveDelayedExplosion as u16 => {
            let packet = packets::RemoveDelayedExplosion::read_body(&mut cursor)?;
            handler.on_remove_delayed_explosion(packet);
        }
        x if x == ServerPacketIds::ObjectDeco as u16 => {
            let packet = packets::ObjectDeco::read_body(&mut cursor)?;
            handler.on_object_deco(packet);
        }
        
        // 地图和传送
        x if x == ServerPacketIds::MapChanged as u16 => {
            let packet = packets::MapChanged::read_body(&mut cursor)?;
            handler.on_map_changed(packet);
        }
        x if x == ServerPacketIds::ObjectTeleportOut as u16 => {
            let packet = packets::ObjectTeleportOut::read_body(&mut cursor)?;
            handler.on_object_teleport_out(packet);
        }
        x if x == ServerPacketIds::ObjectTeleportIn as u16 => {
            let packet = packets::ObjectTeleportIn::read_body(&mut cursor)?;
            handler.on_object_teleport_in(packet);
        }
        x if x == ServerPacketIds::TeleportIn as u16 => {
            let packet = packets::TeleportIn::read_body(&mut cursor)?;
            handler.on_teleport_in(packet);
        }
        x if x == ServerPacketIds::ObjectHide as u16 => {
            let packet = packets::ObjectHide::read_body(&mut cursor)?;
            handler.on_object_hide(packet);
        }
        x if x == ServerPacketIds::ObjectShow as u16 => {
            let packet = packets::ObjectShow::read_body(&mut cursor)?;
            handler.on_object_show(packet);
        }
        x if x == ServerPacketIds::WorldMapSetup as u16 => {
            let packet = packets::WorldMapSetupInfo::read_body(&mut cursor)?;
            handler.on_world_map_setup_info(packet);
        }
        x if x == ServerPacketIds::SearchMapResult as u16 => {
            let packet = packets::SearchMapResult::read_body(&mut cursor)?;
            handler.on_search_map_result(packet);
        }
        
        // NPC商店扩展
        x if x == ServerPacketIds::NPCSell as u16 => {
            let packet = packets::NPCSell::read_body(&mut cursor)?;
            handler.on_npc_sell(packet);
        }
        x if x == ServerPacketIds::NPCRepair as u16 => {
            let packet = packets::NPCRepair::read_body(&mut cursor)?;
            handler.on_npc_repair(packet);
        }
        x if x == ServerPacketIds::NPCSRepair as u16 => {
            let packet = packets::NPCSRepair::read_body(&mut cursor)?;
            handler.on_npc_s_repair(packet);
        }
        x if x == ServerPacketIds::NPCRefine as u16 => {
            let packet = packets::NPCRefine::read_body(&mut cursor)?;
            handler.on_npc_refine(packet);
        }
        x if x == ServerPacketIds::NPCCheckRefine as u16 => {
            let packet = packets::NPCCheckRefine::read_body(&mut cursor)?;
            handler.on_npc_check_refine(packet);
        }
        x if x == ServerPacketIds::NPCCollectRefine as u16 => {
            let packet = packets::NPCCollectRefine::read_body(&mut cursor)?;
            handler.on_npc_collect_refine(packet);
        }
        x if x == ServerPacketIds::NPCReplaceWedRing as u16 => {
            let packet = packets::NPCReplaceWedRing::read_body(&mut cursor)?;
            handler.on_npc_replace_wed_ring(packet);
        }
        x if x == ServerPacketIds::NPCStorage as u16 => {
            let packet = packets::NPCStorage::read_body(&mut cursor)?;
            handler.on_npc_storage(packet);
        }
        x if x == ServerPacketIds::CraftItem as u16 => {
            let packet = packets::CraftItem::read_body(&mut cursor)?;
            handler.on_craft_item(packet);
        }
        
        // 物品扩展
        x if x == ServerPacketIds::NewItemInfo as u16 => {
            let packet = packets::NewItemInfo::read_body(&mut cursor)?;
            handler.on_new_item_info(packet);
        }
        x if x == ServerPacketIds::NewChatItem as u16 => {
            let packet = packets::NewChatItem::read_body(&mut cursor)?;
            handler.on_new_chat_item(packet);
        }
        x if x == ServerPacketIds::ItemSlotSizeChanged as u16 => {
            let packet = packets::ItemSlotSizeChanged::read_body(&mut cursor)?;
            handler.on_item_slot_size_changed(packet);
        }
        x if x == ServerPacketIds::ItemSealChanged as u16 => {
            let packet = packets::ItemSealChanged::read_body(&mut cursor)?;
            handler.on_item_seal_changed(packet);
        }
        x if x == ServerPacketIds::CombineItem as u16 => {
            let packet = packets::CombineItem::read_body(&mut cursor)?;
            handler.on_combine_item(packet);
        }
        x if x == ServerPacketIds::ItemUpgraded as u16 => {
            let packet = packets::ItemUpgraded::read_body(&mut cursor)?;
            handler.on_item_upgraded(packet);
        }
        x if x == ServerPacketIds::EquipSlotItem as u16 => {
            let packet = packets::EquipSlotItem::read_body(&mut cursor)?;
            handler.on_equip_slot_item(packet);
        }
        x if x == ServerPacketIds::GainedCredit as u16 => {
            let packet = packets::GainedCredit::read_body(&mut cursor)?;
            handler.on_gained_credit(packet);
        }
        x if x == ServerPacketIds::LoseCredit as u16 => {
            let packet = packets::LoseCredit::read_body(&mut cursor)?;
            handler.on_lose_credit(packet);
        }
        
        // 英雄系统
        x if x == ServerPacketIds::NewHeroInfo as u16 => {
            let packet = packets::NewHeroInfo::read_body(&mut cursor)?;
            handler.on_new_hero_info(packet);
        }
        x if x == ServerPacketIds::HeroCreateRequest as u16 => {
            let packet = packets::HeroCreateRequest::read_body(&mut cursor)?;
            handler.on_hero_create_request(packet);
        }
        x if x == ServerPacketIds::NewHero as u16 => {
            let packet = packets::NewHero::read_body(&mut cursor)?;
            handler.on_new_hero(packet);
        }
        x if x == ServerPacketIds::HeroInformation as u16 => {
            let packet = packets::HeroInformation::read_body(&mut cursor)?;
            handler.on_hero_information(packet);
        }
        x if x == ServerPacketIds::UpdateHeroSpawnState as u16 => {
            let packet = packets::UpdateHeroSpawnState::read_body(&mut cursor)?;
            handler.on_update_hero_spawn_state(packet);
        }
        x if x == ServerPacketIds::UnlockHeroAutoPot as u16 => {
            let packet = packets::UnlockHeroAutoPot::read_body(&mut cursor)?;
            handler.on_unlock_hero_auto_pot(packet);
        }
        x if x == ServerPacketIds::SetAutoPotValue as u16 => {
            let packet = packets::SetAutoPotValue::read_body(&mut cursor)?;
            handler.on_set_auto_pot_value(packet);
        }
        x if x == ServerPacketIds::SetAutoPotItem as u16 => {
            let packet = packets::SetAutoPotItem::read_body(&mut cursor)?;
            handler.on_set_auto_pot_item(packet);
        }
        x if x == ServerPacketIds::SetHeroBehaviour as u16 => {
            let packet = packets::SetHeroBehaviour::read_body(&mut cursor)?;
            handler.on_set_hero_behaviour(packet);
        }
        x if x == ServerPacketIds::ManageHeroes as u16 => {
            let packet = packets::ManageHeroes::read_body(&mut cursor)?;
            handler.on_manage_heroes(packet);
        }
        x if x == ServerPacketIds::ChangeHero as u16 => {
            let packet = packets::ChangeHero::read_body(&mut cursor)?;
            handler.on_change_hero(packet);
        }
        x if x == ServerPacketIds::TakeBackHeroItem as u16 => {
            let packet = packets::TakeBackHeroItem::read_body(&mut cursor)?;
            handler.on_take_back_hero_item(packet);
        }
        x if x == ServerPacketIds::TransferHeroItem as u16 => {
            let packet = packets::TransferHeroItem::read_body(&mut cursor)?;
            handler.on_transfer_hero_item(packet);
        }
        x if x == ServerPacketIds::GainHeroExperience as u16 => {
            let packet = packets::GainHeroExperience::read_body(&mut cursor)?;
            handler.on_gain_hero_experience(packet);
        }
        x if x == ServerPacketIds::HeroLevelChanged as u16 => {
            let packet = packets::HeroLevelChanged::read_body(&mut cursor)?;
            handler.on_hero_level_changed(packet);
        }
        
        // 组队和社交扩展
        x if x == ServerPacketIds::AllowObserve as u16 => {
            let packet = packets::AllowObserve::read_body(&mut cursor)?;
            handler.on_allow_observe(packet);
        }
        x if x == ServerPacketIds::MarriageRequest as u16 => {
            let packet = packets::MarriageRequest::read_body(&mut cursor)?;
            handler.on_marriage_request(packet);
        }
        x if x == ServerPacketIds::DivorceRequest as u16 => {
            let packet = packets::DivorceRequest::read_body(&mut cursor)?;
            handler.on_divorce_request(packet);
        }
        x if x == ServerPacketIds::MentorRequest as u16 => {
            let packet = packets::MentorRequest::read_body(&mut cursor)?;
            handler.on_mentor_request(packet);
        }
        x if x == ServerPacketIds::LoverUpdate as u16 => {
            let packet = packets::LoverUpdate::read_body(&mut cursor)?;
            handler.on_lover_update(packet);
        }
        x if x == ServerPacketIds::MentorUpdate as u16 => {
            let packet = packets::MentorUpdate::read_body(&mut cursor)?;
            handler.on_mentor_update(packet);
        }
        
        // 行会扩展
        x if x == ServerPacketIds::GuildNoticeChange as u16 => {
            let packet = packets::GuildNoticeChange::read_body(&mut cursor)?;
            handler.on_guild_notice_change(packet);
        }
        x if x == ServerPacketIds::GuildStorageList as u16 => {
            let packet = packets::GuildStorageList::read_body(&mut cursor)?;
            handler.on_guild_storage_list(packet);
        }
        x if x == ServerPacketIds::GuildBuffList as u16 => {
            let packet = packets::GuildBuffList::read_body(&mut cursor)?;
            handler.on_guild_buff_list(packet);
        }
        x if x == ServerPacketIds::ObjectGuildNameChanged as u16 => {
            let packet = packets::ObjectGuildNameChanged::read_body(&mut cursor)?;
            handler.on_object_guild_name_changed(packet);
        }
        
        // 游戏开始相关
        x if x == ServerPacketIds::StartGame as u16 => {
            let packet = packets::StartGame::read_body(&mut cursor)?;
            handler.on_start_game(packet);
        }
        x if x == ServerPacketIds::StartGameBanned as u16 => {
            let packet = packets::StartGameBanned::read_body(&mut cursor)?;
            handler.on_start_game_banned(packet);
        }
        x if x == ServerPacketIds::StartGameDelay as u16 => {
            let packet = packets::StartGameDelay::read_body(&mut cursor)?;
            handler.on_start_game_delay(packet);
        }
        x if x == ServerPacketIds::ReturnToLogin as u16 => {
            let packet = packets::ReturnToLogin::read_body(&mut cursor)?;
            handler.on_return_to_login(packet);
        }
        x if x == ServerPacketIds::ClientVersion as u16 => {
            let packet = packets::ClientVersion::read_body(&mut cursor)?;
            handler.on_client_version(packet);
        }
        x if x == ServerPacketIds::LoginBanned as u16 => {
            let packet = packets::LoginBanned::read_body(&mut cursor)?;
            handler.on_login_banned(packet);
        }
        
        // 特殊系统
        x if x == ServerPacketIds::SendOutputMessage as u16 => {
            let packet = packets::SendOutputMessage::read_body(&mut cursor)?;
            handler.on_send_output_message(packet);
        }
        x if x == ServerPacketIds::Opendoor as u16 => {
            let packet = packets::Opendoor::read_body(&mut cursor)?;
            handler.on_open_door(packet);
        }
        x if x == ServerPacketIds::ResizeInventory as u16 => {
            let packet = packets::ResizeInventory::read_body(&mut cursor)?;
            handler.on_resize_inventory(packet);
        }
        x if x == ServerPacketIds::ResizeStorage as u16 => {
            let packet = packets::ResizeStorage::read_body(&mut cursor)?;
            handler.on_resize_storage(packet);
        }
        x if x == ServerPacketIds::TransformUpdate as u16 => {
            let packet = packets::TransformUpdate::read_body(&mut cursor)?;
            handler.on_transform_update(packet);
        }
        
        // 智能生物系统
        x if x == ServerPacketIds::NewIntelligentCreature as u16 => {
            let packet = packets::NewIntelligentCreature::read_body(&mut cursor)?;
            handler.on_new_intelligent_creature(packet);
        }
        x if x == ServerPacketIds::UpdateIntelligentCreatureList as u16 => {
            let packet = packets::UpdateIntelligentCreatureList::read_body(&mut cursor)?;
            handler.on_update_intelligent_creature_list(packet);
        }
        x if x == ServerPacketIds::IntelligentCreatureEnableRename as u16 => {
            let packet = packets::IntelligentCreatureEnableRename::read_body(&mut cursor)?;
            handler.on_intelligent_creature_enable_rename(packet);
        }
        x if x == ServerPacketIds::IntelligentCreaturePickup as u16 => {
            let packet = packets::IntelligentCreaturePickup::read_body(&mut cursor)?;
            handler.on_intelligent_creature_pickup(packet);
        }
        x if x == ServerPacketIds::NPCPearlGoods as u16 => {
            let packet = packets::NPCPearlGoods::read_body(&mut cursor)?;
            handler.on_npc_pearl_goods(packet);
        }
        
        // 游戏商店系统
        x if x == ServerPacketIds::GameShopInfo as u16 => {
            let packet = packets::GameShopInfo::read_body(&mut cursor)?;
            handler.on_game_shop_info(packet);
        }
        x if x == ServerPacketIds::GameShopStock as u16 => {
            let packet = packets::GameShopStock::read_body(&mut cursor)?;
            handler.on_game_shop_stock(packet);
        }
        
        // 排名系统
        x if x == ServerPacketIds::Rankings as u16 => {
            let packet = packets::Rankings::read_body(&mut cursor)?;
            handler.on_rankings(packet);
        }
        
        // 行会领地系统
        x if x == ServerPacketIds::GuildTerritoryPage as u16 => {
            let packet = packets::GuildTerritoryPage::read_body(&mut cursor)?;
            handler.on_guild_territory_page(packet);
        }
        x if x == ServerPacketIds::PurchaseGuildTerritory as u16 => {
            let packet = packets::PurchaseGuildTerritory::read_body(&mut cursor)?;
            handler.on_purchase_guild_territory(packet);
        }
        
        // 邮件系统
        x if x == ServerPacketIds::ReceiveMail as u16 => {
            let packet = packets::ReceiveMail::read_body(&mut cursor)?;
            handler.on_receive_mail(packet);
        }
        x if x == ServerPacketIds::MailLockedItem as u16 => {
            let packet = packets::MailLockedItem::read_body(&mut cursor)?;
            handler.on_mail_locked_item(packet);
        }
        x if x == ServerPacketIds::MailSendRequest as u16 => {
            let packet = packets::MailSendRequest::read_body(&mut cursor)?;
            handler.on_mail_send_request(packet);
        }
        x if x == ServerPacketIds::MailSent as u16 => {
            let packet = packets::MailSent::read_body(&mut cursor)?;
            handler.on_mail_sent(packet);
        }
        x if x == ServerPacketIds::ParcelCollected as u16 => {
            let packet = packets::ParcelCollected::read_body(&mut cursor)?;
            handler.on_parcel_collected(packet);
        }
        x if x == ServerPacketIds::MailCost as u16 => {
            let packet = packets::MailCost::read_body(&mut cursor)?;
            handler.on_mail_cost(packet);
        }
        
        // 市场系统
        x if x == ServerPacketIds::NPCConsign as u16 => {
            let packet = packets::NPCConsign::read_body(&mut cursor)?;
            handler.on_npc_consign(packet);
        }
        x if x == ServerPacketIds::NPCMarket as u16 => {
            let packet = packets::NPCMarket::read_body(&mut cursor)?;
            handler.on_npc_market(packet);
        }
        x if x == ServerPacketIds::NPCMarketPage as u16 => {
            let packet = packets::NPCMarketPage::read_body(&mut cursor)?;
            handler.on_npc_market_page(packet);
        }
        x if x == ServerPacketIds::ConsignItem as u16 => {
            let packet = packets::ConsignItem::read_body(&mut cursor)?;
            handler.on_consign_item(packet);
        }
        x if x == ServerPacketIds::MarketFail as u16 => {
            let packet = packets::MarketFail::read_body(&mut cursor)?;
            handler.on_market_fail(packet);
        }
        x if x == ServerPacketIds::MarketSuccess as u16 => {
            let packet = packets::MarketSuccess::read_body(&mut cursor)?;
            handler.on_market_success(packet);
        }
        
        // 觉醒系统
        x if x == ServerPacketIds::NPCAwakening as u16 => {
            let packet = packets::NPCAwakening::read_body(&mut cursor)?;
            handler.on_npc_awakening(packet);
        }
        x if x == ServerPacketIds::NPCDisassemble as u16 => {
            let packet = packets::NPCDisassemble::read_body(&mut cursor)?;
            handler.on_npc_disassemble(packet);
        }
        x if x == ServerPacketIds::NPCDowngrade as u16 => {
            let packet = packets::NPCDowngrade::read_body(&mut cursor)?;
            handler.on_npc_downgrade(packet);
        }
        x if x == ServerPacketIds::NPCReset as u16 => {
            let packet = packets::NPCReset::read_body(&mut cursor)?;
            handler.on_npc_reset(packet);
        }
        x if x == ServerPacketIds::AwakeningNeedMaterials as u16 => {
            let packet = packets::AwakeningNeedMaterials::read_body(&mut cursor)?;
            handler.on_awakening_need_materials(packet);
        }
        x if x == ServerPacketIds::AwakeningLockedItem as u16 => {
            let packet = packets::AwakeningLockedItem::read_body(&mut cursor)?;
            handler.on_awakening_locked_item(packet);
        }
        x if x == ServerPacketIds::Awakening as u16 => {
            let packet = packets::Awakening::read_body(&mut cursor)?;
            handler.on_awakening(packet);
        }
        
        // 租赁系统
        x if x == ServerPacketIds::GetRentedItems as u16 => {
            let packet = packets::GetRentedItems::read_body(&mut cursor)?;
            handler.on_get_rented_items(packet);
        }
        x if x == ServerPacketIds::ItemRentalRequest as u16 => {
            let packet = packets::ItemRentalRequest::read_body(&mut cursor)?;
            handler.on_item_rental_request(packet);
        }
        x if x == ServerPacketIds::ItemRentalFee as u16 => {
            let packet = packets::ItemRentalFee::read_body(&mut cursor)?;
            handler.on_item_rental_fee(packet);
        }
        x if x == ServerPacketIds::ItemRentalPeriod as u16 => {
            let packet = packets::ItemRentalPeriod::read_body(&mut cursor)?;
            handler.on_item_rental_period(packet);
        }
        x if x == ServerPacketIds::DepositRentalItem as u16 => {
            let packet = packets::DepositRentalItem::read_body(&mut cursor)?;
            handler.on_deposit_rental_item(packet);
        }
        x if x == ServerPacketIds::RetrieveRentalItem as u16 => {
            let packet = packets::RetrieveRentalItem::read_body(&mut cursor)?;
            handler.on_retrieve_rental_item(packet);
        }
        x if x == ServerPacketIds::UpdateRentalItem as u16 => {
            let packet = packets::UpdateRentalItem::read_body(&mut cursor)?;
            handler.on_update_rental_item(packet);
        }
        x if x == ServerPacketIds::CancelItemRental as u16 => {
            let packet = packets::CancelItemRental::read_body(&mut cursor)?;
            handler.on_cancel_item_rental(packet);
        }
        x if x == ServerPacketIds::ItemRentalLock as u16 => {
            let packet = packets::ItemRentalLock::read_body(&mut cursor)?;
            handler.on_item_rental_lock(packet);
        }
        x if x == ServerPacketIds::ItemRentalPartnerLock as u16 => {
            let packet = packets::ItemRentalPartnerLock::read_body(&mut cursor)?;
            handler.on_item_rental_partner_lock(packet);
        }
        x if x == ServerPacketIds::CanConfirmItemRental as u16 => {
            let packet = packets::CanConfirmItemRental::read_body(&mut cursor)?;
            handler.on_can_confirm_item_rental(packet);
        }
        x if x == ServerPacketIds::ConfirmItemRental as u16 => {
            let packet = packets::ConfirmItemRental::read_body(&mut cursor)?;
            handler.on_confirm_item_rental(packet);
        }
        
        // UI事件系统
        x if x == ServerPacketIds::NewRecipeInfo as u16 => {
            let packet = packets::NewRecipeInfo::read_body(&mut cursor)?;
            handler.on_new_recipe_info(packet);
        }
        x if x == ServerPacketIds::OpenBrowser as u16 => {
            let packet = packets::OpenBrowser::read_body(&mut cursor)?;
            handler.on_open_browser(packet);
        }
        x if x == ServerPacketIds::PlaySound as u16 => {
            let packet = packets::PlaySound::read_body(&mut cursor)?;
            handler.on_play_sound(packet);
        }
        x if x == ServerPacketIds::SetTimer as u16 => {
            let packet = packets::SetTimer::read_body(&mut cursor)?;
            handler.on_set_timer(packet);
        }
        x if x == ServerPacketIds::ExpireTimer as u16 => {
            let packet = packets::ExpireTimer::read_body(&mut cursor)?;
            handler.on_expire_timer(packet);
        }
        x if x == ServerPacketIds::UpdateNotice as u16 => {
            let packet = packets::UpdateNotice::read_body(&mut cursor)?;
            handler.on_update_notice(packet);
        }
        x if x == ServerPacketIds::Roll as u16 => {
            let packet = packets::Roll::read_body(&mut cursor)?;
            handler.on_roll(packet);
        }
        x if x == ServerPacketIds::SetCompass as u16 => {
            let packet = packets::SetCompass::read_body(&mut cursor)?;
            handler.on_set_compass(packet);
        }
        
        // 未知数据包
        _ => {
            handler.on_unknown_packet(header.opcode, body);
        }
    }
    
    Ok(())
}

// ============================================================================
// 测试和示例
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    struct TestHandler {
        connected_called: bool,
    }
    
    impl PacketHandler for TestHandler {
        fn on_connected(&mut self, _packet: packets::Connected) {
            self.connected_called = true;
        }
    }
    
    #[test]
    fn test_packet_header_parsing() {
        let data = vec![0x10, 0x00, 0x01, 0x00]; // length=16, opcode=1
        let header = parse_packet_header(&data).unwrap();
        assert_eq!(header.length, 16);
        assert_eq!(header.opcode, 1);
    }
    
    #[test]
    fn test_serialize_client_packet() {
        use mir2_shared::packets::client::ClientVersion;
        
        let packet = ClientVersion {
            version_hash: vec![1, 2, 3, 4],
        };
        
        let bytes = serialize_client_packet(&packet).unwrap();
        
        // 检查长度和opcode
        assert!(bytes.len() >= 4);
        let header = parse_packet_header(&bytes).unwrap();
        assert_eq!(header.length, bytes.len() as u16);
        assert_eq!(header.opcode, ClientVersion::OPCODE);
    }
}
