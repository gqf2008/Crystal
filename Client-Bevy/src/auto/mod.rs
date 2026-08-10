// ============================================================================
// auto：自动化验证/调试系统（--auto-* / --real-verify / F12 截图等）
// ============================================================================
// 从 auto.rs 拆分（#1146）：register() 按 CLI flag 分发；各域系统按领域模块化。
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use client_bevy::scenes::AppState;
mod combat;
mod dialogs;
mod inventory;
mod navigation;
mod social;
mod world;
pub(crate) use combat::*;
pub(crate) use dialogs::*;
pub(crate) use inventory::*;
pub(crate) use navigation::*;
pub(crate) use social::*;
pub(crate) use world::*;
/// 按 CLI flag（--auto-* / --real-verify / --auto-walk 等）注册自动化验证系统
pub fn register(app: &mut App) {
    // --auto-attack: 进游戏后每 1.5s 自动攻击（M10 战斗链路调试）
    // --attack-range-test: 攻击距离校验（#1554，近战1格/弓手9格/范围外提示）
    if std::env::args().any(|a| a == "--attack-range-test") {
        app.add_systems(Update, auto_attack_range_test);
    }
    if std::env::args().any(|a| a == "--auto-attack") {
        app.add_systems(Update, auto_attack_debug);
    }
    // --auto-pmode: 宠物模式切换链路（#1562，C.ChangePMode → S.ChangePMode 确认）
    if std::env::args().any(|a| a == "--auto-pmode") {
        app.add_systems(Update, auto_pmode_test);
    }
    // --auto-pet-pickup: 宠物拾取指令链路（#1558，C.IntelligentCreaturePickup → 拾取入包）
    if std::env::args().any(|a| a == "--auto-pet-pickup") {
        app.add_systems(Update, auto_pet_pickup_test);
    }
    // --auto-ranged-attack: 弓手远程攻击链路（#1556，C.RangeAttack → 弹道 → 受击反馈）
    if std::env::args().any(|a| a == "--auto-ranged-attack") {
        app.add_systems(Update, auto_ranged_attack_test);
    }
    // --auto-inv / --auto-char: 进游戏 3 秒后自动打开背包/角色对话框（M9 调试）
    if std::env::args().any(|a| a == "--auto-inv") {
        app.add_systems(Update, auto_open_inventory);
    }
    if std::env::args().any(|a| a == "--auto-char") {
        app.add_systems(Update, auto_open_character);
    }
    // --storage-test: 自动仓库存取链路（自动化验证用）
    // --storage-equip-test: 仓库双击装备链路（#1546，Storage EquipItem）
    if std::env::args().any(|a| a == "--storage-equip-test") {
        app.add_systems(Update, auto_storage_equip_test);
    }
    if std::env::args().any(|a| a == "--storage-test") {
        app.add_systems(Update, auto_storage_test);
    }
    // --storage-unlock-test: 仓库密码解锁链路（#200，[@Storage] → 解锁框 → UnlockStorage → UserStorage）
    if std::env::args().any(|a| a == "--storage-unlock-test") {
        app.add_systems(Update, auto_storage_unlock_test);
    }
    // --storage-resize-test: 仓库扩容链路（#281，进图 mock 回发 ResizeStorage(80) → 校验 items.len()==80）
    if std::env::args().any(|a| a == "--storage-resize-test") {
        app.add_systems(Update, auto_storage_resize_test);
    }
    // --level-fx-test: 升级表现链路（#283，击杀 → mock 回发 LevelChanged+ObjectLeveled → 校验）
    if std::env::args().any(|a| a == "--level-fx-test") {
        app.add_systems(Update, auto_level_fx_test);
    }
    // --chat-item-test: 聊天物品链路（#285，进图缓存 9005 + RequestChatItem(9999) → 缓存增长）
    if std::env::args().any(|a| a == "--chat-item-test") {
        app.add_systems(Update, auto_chat_item_test);
    }
    // --session-feedback-test: 会话反馈链路（#289，@RETURNLOGIN → 返回登录界面）
    if std::env::args().any(|a| a == "--session-feedback-test") {
        app.add_systems(Update, auto_session_feedback_test);
    }
    // --guild-storage-realtime-test: 行会仓库实时同步（#295，施法 → mock 回发 Gold/ItemChange → 校验状态）
    if std::env::args().any(|a| a == "--guild-storage-realtime-test") {
        app.add_systems(Update, auto_guild_storage_realtime_test);
    }
    // --group-test: 自动组队邀请链路（自动化验证用，配合 --group-accept）
    if std::env::args().any(|a| a == "--group-test") {
        app.add_systems(Update, auto_group_test);
    }
    // --group-accept: 自动接受组队邀请（自动化验证用）
    if std::env::args().any(|a| a == "--group-accept") {
        app.add_systems(Update, auto_group_accept);
    }
    // --mail-test: 自动发邮件链路（自动化验证用，配合 --mail-read）
    if std::env::args().any(|a| a == "--mail-test") {
        app.add_systems(Update, auto_mail_test);
    }
    // --mail-read: 自动读取新邮件（自动化验证用）
    if std::env::args().any(|a| a == "--mail-read") {
        app.add_systems(Update, auto_mail_read);
    }
    // --whisper-send: 真实服私聊发送（配合 --whisper-check 双开验证 /w 链路）
    if std::env::args().any(|a| a == "--whisper-send") {
        app.add_systems(Update, auto_whisper_send);
    }
    // --whisper-check: 真实服私聊接收校验（WhisperIn 显示 + last_pm 记录，#813）
    if std::env::args().any(|a| a == "--whisper-check") {
        app.add_systems(Update, auto_whisper_check);
    }
    // --trade-test: 自动交易链路（发起者，配合 --trade-accept）
    if std::env::args().any(|a| a == "--trade-test") {
        app.add_systems(Update, auto_trade_test);
    }
    // --trade-accept: 自动接受交易邀请（配合 --trade-test）
    if std::env::args().any(|a| a == "--trade-accept") {
        app.add_systems(Update, auto_trade_accept);
    }
    // --drop-pick-test: 怪物掉落 → 地面物品 → 拾取 → 背包（自动化验证用）
    if std::env::args().any(|a| a == "--drop-pick-test") {
        app.add_systems(Update, auto_drop_pick_test);
    }
    // --friend-test: 自动加好友链路（配合 B 在线）
    if std::env::args().any(|a| a == "--friend-test") {
        app.add_systems(Update, auto_friend_test);
    }
    // --mail-compose-test: 写邮件界面 → 发送（配合 B --mail-read）
    if std::env::args().any(|a| a == "--mail-compose-test") {
        app.add_systems(Update, auto_mail_compose_test);
    }
    // --guild-test: 创建行会链路（GuildNameReturn → GuildStatus 信息）
    if std::env::args().any(|a| a == "--guild-test") {
        app.add_systems(Update, auto_guild_test);
    }
    // --guild-invite-test: 行会邀请链路（创建→邀请，配合 --guild-accept）
    if std::env::args().any(|a| a == "--guild-invite-test") {
        app.add_systems(Update, auto_guild_invite_test);
    }
    // --guild-accept: 自动接受行会邀请
    if std::env::args().any(|a| a == "--guild-accept") {
        app.add_systems(Update, auto_guild_accept);
    }
    // --guild-notice-test: 行会公告链路（创建→设置公告→等 GuildNoticeChange）
    if std::env::args().any(|a| a == "--guild-notice-test") {
        app.add_systems(Update, auto_guild_notice_test);
    }
    // --guild-gold-test: 行会仓库金币链路（创建→存入→取出）
    if std::env::args().any(|a| a == "--guild-gold-test") {
        app.add_systems(Update, auto_guild_gold_test);
    }
    // --ranking-test: 排行榜链路（打开对话框 → GetRanking → 显示）
    if std::env::args().any(|a| a == "--ranking-test") {
        app.add_systems(Update, auto_ranking_test);
    }
    // --shop-test: 自动 NPC 商店买卖链路（自动化验证用）
    if std::env::args().any(|a| a == "--shop-test") {
        app.add_systems(Update, auto_shop_test);
    }
    // --guild-item-test: 行会仓库物品链路（打开仓库 → 存入背包物品 → 取出）
    if std::env::args().any(|a| a == "--guild-item-test") {
        app.add_systems(Update, auto_guild_item_test);
    }
    // --mentor-test: 师徒链路（发起拜师，配合 --mentor-accept）
    if std::env::args().any(|a| a == "--mentor-test") {
        app.add_systems(Update, auto_mentor_test);
    }
    // --mentor-accept: 师徒链路（允许拜师 + 接受邀请，配合 --mentor-test）
    if std::env::args().any(|a| a == "--mentor-accept") {
        app.add_systems(Update, auto_mentor_accept);
    }
    // --market-test: 市场链路（寄售×2 → 取回一件 → 留一件给 --market-buy 买）
    if std::env::args().any(|a| a == "--market-test") {
        app.add_systems(Update, auto_market_test);
    }
    // --market-buy: 市场购买链路（配合 --market-test）
    if std::env::args().any(|a| a == "--market-buy") {
        app.add_systems(Update, auto_market_buy);
    }
    // --gameshop-test: 商城链路（打开商城 → 目录 → 购买 → 邮件送达）
    if std::env::args().any(|a| a == "--gameshop-test") {
        app.add_systems(Update, auto_gameshop_test);
    }
    // --territory-test: 行会领地链路（打开领地 → 购买无主领地 → 宣战，配合 --territory-war）
    if std::env::args().any(|a| a == "--territory-test") {
        app.add_systems(Update, auto_territory_test);
    }
    // --territory-war: 创建目标行会供宣战（配合 --territory-test）
    if std::env::args().any(|a| a == "--territory-war") {
        app.add_systems(Update, auto_territory_war);
    }
    // --combat-test: 战斗闭环（选中怪物 → 连续 FireBall → 死亡 → 掉落）
    if std::env::args().any(|a| a == "--combat-test") {
        app.add_systems(Update, auto_combat_test);
    }
    // --fishing-test: 钓鱼链路（装备鱼竿 → 抛竿 → 等收获消息）
    if std::env::args().any(|a| a == "--fishing-test") {
        app.add_systems(Update, auto_fishing_test);
    }
    // --refine-test: 精炼链路（存入 → 开始 → 等待 → 查看 → 取回）
    if std::env::args().any(|a| a == "--refine-test") {
        app.add_systems(Update, auto_refine_test);
    }
    // --craft-test: 合成链路（配方1：木材x3+铁矿石x2 → 铁剑）
    if std::env::args().any(|a| a == "--craft-test") {
        app.add_systems(Update, auto_craft_test);
    }
    // --rental-test: 物品租赁链路（租方，配合 --rental-owner）
    if std::env::args().any(|a| a == "--rental-test") {
        app.add_systems(Update, auto_rental_test);
    }
    // --rental-owner: 物品租赁链路（物主，配合 --rental-test）
    if std::env::args().any(|a| a == "--rental-owner") {
        app.add_systems(Update, auto_rental_owner);
    }
    // --quest-test: 任务日志链路（接受任务 → ChangeQuest 显示 → 放弃）
    if std::env::args().any(|a| a == "--quest-test") {
        app.add_systems(Update, auto_quest_test);
    }
    // --buff-test: 状态/Buff 链路（施放 Fury → AddBuff 显示）
    if std::env::args().any(|a| a == "--buff-test") {
        app.add_systems(Update, auto_buff_test);
    }
    // --report-test: 举报链路（提交举报 → 系统消息确认）
    if std::env::args().any(|a| a == "--report-test") {
        app.add_systems(Update, auto_report_test);
    }
    // --inspect-test: 查看玩家链路（找到 bevy2char → Inspect → PlayerInspect 显示）
    if std::env::args().any(|a| a == "--inspect-test") {
        app.add_systems(Update, auto_inspect_test);
    }
    // --creature-test: 宠物链路（打开宠物 → 请求列表 → 解析）
    if std::env::args().any(|a| a == "--creature-test") {
        app.add_systems(Update, auto_creature_test);
    }
    // --hero-test: 英雄链路（切换英雄1 → ChangeHero 包 → 切回主角色）
    if std::env::args().any(|a| a == "--hero-test") {
        app.add_systems(Update, auto_hero_test);
    }
    // --hero-battle-test: 英雄战斗 HP 实时同步（#1135：部署英雄 → 等 HeroHealthChanged 使 HP 下降）
    if std::env::args().any(|a| a == "--hero-battle-test") {
        app.add_systems(Update, auto_hero_battle_test);
    }
    // --hero-exp-test: 英雄经验增长（#1142/#1163：部署英雄 → 等 GainHeroExperience 使 hero_exp>0）
    if std::env::args().any(|a| a == "--hero-exp-test") {
        app.add_systems(Update, auto_hero_exp_test);
    }
    // --reincarnation-test: 轮回术确认链路（死亡 → 收到 offer → 接受 → 复活）
    if std::env::args().any(|a| a == "--reincarnation-test") {
        app.add_systems(Update, auto_reincarnation_test);
    }
    // --battle-vfx-test: 战斗表现层（施法 → ObjectMagic/ObjectProjectile/ObjectEffect/ObjectRangeAttack 特效）
    if std::env::args().any(|a| a == "--battle-vfx-test") {
        app.add_systems(Update, auto_battle_vfx_test);
    }
    // --object-state-test: 对象状态表现（隐藏/显形/坐下/击退/传送进出，#226）
    if std::env::args().any(|a| a == "--object-state-test") {
        app.add_systems(Update, auto_object_state_test);
    }
    // --item-state-test: 物品状态同步（DuraChanged/DeleteItem/GainedItem，#228）
    if std::env::args().any(|a| a == "--item-state-test") {
        app.add_systems(Update, auto_item_state_test);
    }
    // --map-fx-test: 地图特效/音效/计时器（MapEffect/PlaySound/SetTimer/ExpireTimer，#230）
    if std::env::args().any(|a| a == "--map-fx-test") {
        app.add_systems(Update, auto_map_fx_test);
    }
    // --mount-sync-test: 坐骑同步（MountUpdate 上马/下马，#232）
    if std::env::args().any(|a| a == "--mount-sync-test") {
        app.add_systems(Update, auto_mount_sync_test);
    }
    // --action-test: 对象动作（ObjectAttack/冲刺/后跳，#234）
    if std::env::args().any(|a| a == "--action-test") {
        app.add_systems(Update, auto_action_test);
    }
    // --poison-test: 中毒染层（Poisoned/ObjectPoisoned，#236）
    if std::env::args().any(|a| a == "--poison-test") {
        app.add_systems(Update, auto_poison_test);
    }
    // --mana-test: 对象蓝条（ObjectMana，#238）
    if std::env::args().any(|a| a == "--mana-test") {
        app.add_systems(Update, auto_mana_test);
    }
    // --repair-test: 物品修理/槽位同步（ItemRepaired/ItemSlotSizeChanged，#240）
    if std::env::args().any(|a| a == "--repair-test") {
        app.add_systems(Update, auto_repair_test);
    }
    // --toggle-test: 技能开关（SpellToggle 双向，#242）
    if std::env::args().any(|a| a == "--toggle-test") {
        app.add_systems(Update, auto_toggle_test);
    }
    // --gold-test: 地面金币（ObjectGold，#244）
    if std::env::args().any(|a| a == "--gold-test") {
        app.add_systems(Update, auto_gold_test);
    }
    // --harvest-test: 采集表现（ObjectHarvest/ObjectHarvested，#246）
    if std::env::args().any(|a| a == "--harvest-test") {
        app.add_systems(Update, auto_harvest_test);
    }
    // --npc-credit-test: NPC 形象更新 + 声望（NPCImageUpdate/GainedCredit，#248）
    if std::env::args().any(|a| a == "--npc-credit-test") {
        app.add_systems(Update, auto_npc_credit_test);
    }
    // --compass-test: 罗盘目标（SetCompass，#250）
    if std::env::args().any(|a| a == "--compass-test") {
        app.add_systems(Update, auto_compass_test);
    }
    // --sneak-test: 隐身/等级特效（ObjectSneaking/ObjectLevelEffects，#252）
    if std::env::args().any(|a| a == "--sneak-test") {
        app.add_systems(Update, auto_sneak_test);
    }
    // --member-test: 小队成员小地图点位（SendMemberLocation，#254）
    if std::env::args().any(|a| a == "--member-test") {
        app.add_systems(Update, auto_member_test);
    }
    // --notice-test: 服务器公告（UpdateNotice，#256）
    if std::env::args().any(|a| a == "--notice-test") {
        app.add_systems(Update, auto_notice_test);
    }
    // --upgrade-test: 合成/升级/技能删除/服务端消息（#258）
    if std::env::args().any(|a| a == "--upgrade-test") {
        app.add_systems(Update, auto_upgrade_test);
    }
    // --quest-data-test: 任务数据包（NewQuestInfo/ShareQuest，#260）
    if std::env::args().any(|a| a == "--quest-data-test") {
        app.add_systems(Update, auto_quest_data_test);
    }
    // --recipe-test: 配方学习 + Buff 暂停（#262）
    if std::env::args().any(|a| a == "--recipe-test") {
        app.add_systems(Update, auto_recipe_test);
    }
    // --name-test: 名称同步（ObjectName/UserName，#264）
    if std::env::args().any(|a| a == "--name-test") {
        app.add_systems(Update, auto_name_test);
    }
    // --misc2-test: 杂项协议（BaseStatsInfo 等，#268）
    if std::env::args().any(|a| a == "--misc2-test") {
        app.add_systems(Update, auto_misc2_test);
    }
    // --final-test: 冲刺攻击/传送/杂项收尾（#270）
    if std::env::args().any(|a| a == "--final-test") {
        app.add_systems(Update, auto_final_test);
    }
    // --npc-input-test: NPC 输入框（NPCRequestInput，#272）
    if std::env::args().any(|a| a == "--npc-input-test") {
        app.add_systems(Update, auto_npc_input_test);
    }
    // --creature2-test: 智能宠物协议（#274）
    if std::env::args().any(|a| a == "--creature2-test") {
        app.add_systems(Update, auto_creature2_test);
    }
    // --resize-test: 背包扩容链路（#276，施法 → mock 回发 ResizeInventory(56) → 校验）
    if std::env::args().any(|a| a == "--resize-test") {
        app.add_systems(Update, auto_resize_test);
    }
    // --book-test: 技能书学习链路（使用技能书 → 等 NewMagic → 校验 MagicsState）
    if std::env::args().any(|a| a == "--book-test") {
        app.add_systems(Update, auto_book_test);
    }
    // --marriage-test: 婚姻链路（求婚 → 结婚 → 离婚，配合 --marriage-accept）
    if std::env::args().any(|a| a == "--marriage-test") {
        app.add_systems(Update, auto_marriage_test);
    }
    // --marriage-accept: 婚姻链路（接受求婚 → 离婚确认，配合 --marriage-test）
    if std::env::args().any(|a| a == "--marriage-accept") {
        app.add_systems(Update, auto_marriage_accept);
    }
    // --ui-dialog-test: 纯客户端对话框批量验证（公告/聊天公告/计时器/帮助）
    if std::env::args().any(|a| a == "--ui-dialog-test") {
        app.add_systems(Update, auto_ui_dialog_test);
    }
    // --option-test: 设置对话框验证（打开 → 切换 8 组开关 → 音量 → 关闭）
    if std::env::args().any(|a| a == "--option-test") {
        app.add_systems(Update, auto_option_test);
    }
    // --keyboard-test: 键位设置对话框验证（打开 → 滚动 → 重绑 → 重置 → 关闭）
    if std::env::args().any(|a| a == "--keyboard-test") {
        app.add_systems(Update, auto_keyboard_test);
    }
    // --bigmap-test: 大地图对话框验证（NewMapInfo → 地形 → NPC 列表 → 传送）
    if std::env::args().any(|a| a == "--bigmap-test") {
        app.add_systems(Update, auto_bigmap_test);
    }
    // --worldmap-test: 世界地图（#300）WorldMapSetup → 图标 → RequestMapInfo → NewMapInfo 切换
    if std::env::args().any(|a| a == "--worldmap-test") {
        app.add_systems(Update, auto_worldmap_test);
    }
    // --real-worldmap-test: 真实服务器世界地图联调（#302）WorldMapSetup → RequestMapInfo → NewMapInfo
    if std::env::args().any(|a| a == "--real-worldmap-test") {
        app.add_systems(Update, auto_real_worldmap_test);
    }
    // --awake-test: 觉醒对话框验证（选武器 → 类型 → 材料 → 觉醒 → 结果）
    if std::env::args().any(|a| a == "--awake-test") {
        app.add_systems(Update, auto_awake_test);
    }
    // --dura-test: 耐久面板验证（打开 → 装备耐久三态渲染 → 关闭）
    if std::env::args().any(|a| a == "--dura-test") {
        app.add_systems(Update, auto_dura_test);
    }
    // --socket-test: 镶嵌面板验证（带宝石槽物品 → 孔位/宝石渲染 → 关闭）
    if std::env::args().any(|a| a == "--socket-test") {
        app.add_systems(Update, auto_socket_test);
    }
    // --roll-test: 掷骰链路（CallNPC TestRoll → 服务端 Roll 包 → 动画/回调）
    if std::env::args().any(|a| a == "--roll-test") {
        app.add_systems(Update, auto_roll_test);
    }
    // --reconnect-test: 断线自动重连验证（等断线 → 重连 → 自动登录进游戏）
    if std::env::args().any(|a| a == "--reconnect-test") {
        app.add_systems(Update, auto_reconnect_test);
    }
    // --mount-test: 坐骑链路（装备坐骑 → 面板 → @ride 骑乘/下马 → 外观广播）
    if std::env::args().any(|a| a == "--mount-test") {
        app.add_systems(Update, auto_mount_test);
    }
    // --auto-enter: 自动从登录界面进入游戏（自动化验证用）
    if std::env::args().any(|a| a == "--auto-enter") {
        // auto_enter 需要覆盖 Login 和 Select 两个状态（内部自行判断）
        app.add_systems(Update, auto_enter);
    }
    // BEVY_DEMO_DELETE=1: 自动登录→进选角→打开删除询问框（截图验证用）
    if std::env::var("BEVY_DEMO_DELETE").as_deref() == Ok("1") {
        app.add_systems(Update, demo_delete_flow);
    }
    // F12: 保存当前帧截图到 ../../tools/bevy_shot_N.png（开发调试用）
    app.add_systems(Update, debug_screenshot);
    if std::env::var("UI_DUMP").is_ok() {
        app.add_systems(Update, ui_dump_system);
    }
    // --auto-pickup: 进图后自动拾取最近的 GroundItem（验证拾取闭环，无需鼠标）
    if std::env::args().any(|a| a == "--auto-pickup") {
        app.add_systems(Update, auto_pickup_system);
    }
    // --auto-cast: 进图后自动施放 F1 技能（验证技能链路）
    if std::env::args().any(|a| a == "--auto-cast") {
        app.add_systems(Update, auto_cast_system);
    }
    // --auto-equip: 进图后自动装备背包第一件可装备物品（验证穿戴→外观）
    if std::env::args().any(|a| a == "--auto-equip") {
        app.add_systems(Update, auto_equip_system);
    }
    // --auto-life: 依次验证 聊天回显 → 商店购买 → 使用药水
    if std::env::args().any(|a| a == "--auto-life") {
        app.add_systems(Update, auto_life_system);
    }
    // --auto-quest: 任务闭环（接受任务1 → 自动击杀 → 完成 → 交任务 → 验证奖励）
    if std::env::args().any(|a| a == "--auto-quest") {
        app.add_systems(Update, auto_quest_system);
    }
    // --auto-revive: 死亡后 1s 自动 TownRevive（验证 死亡→复活 全链路，#46）
    if std::env::args().any(|a| a == "--auto-revive") {
        app.add_systems(Update, auto_revive_system);
    }
    // --auto-cast-loop: 每秒连发 F1 技能（验证耗蓝/蓝不足拒绝，#51）
    if std::env::args().any(|a| a == "--auto-cast-loop") {
        app.add_systems(Update, auto_cast_loop_system);
    }
    // --real-verify: 真实服务器交互闭环（聊天/移动/战斗/NPC，#55）
    if std::env::args().any(|a| a == "--real-verify") {
        app.add_systems(Update, real_verify_system);
    }
    // --spell-verify: 真实服务器法术冒烟（#306）HellFire/IceThrust/Curse/EnergyRepulsor
    if std::env::args().any(|a| a == "--spell-verify") {
        app.add_systems(Update, auto_spell_verify);
    }
    // --auto-walk-diag: 对角线移动方向稳定性验证（#1145）——真实寻路 + LocalMove + 记录方向序列
    // --hold-move-test: 按住移动方向驱动稳定性（#1548，45° 扇区容差）
    if std::env::args().any(|a| a == "--hold-move-test") {
        app.init_resource::<HoldMoveTest>();
        app.add_systems(Update, hold_move_test_system);
    }
    if std::env::args().any(|a| a == "--auto-walk-diag") {
        app.init_resource::<AutoWalkDiag>();
        app.add_systems(Update, auto_walk_diag_system);
    }
    // --auto-walk <up|down|left|right>: 调试 chunk 流式（每帧驱动玩家平移）
    {
        let dir = std::env::args()
            .position(|a| a == "--auto-walk")
            .and_then(|i| std::env::args().nth(i + 1))
            .unwrap_or_default();
        let dir_copy = dir.clone();
        app.insert_resource(AutoWalkDir(dir));
        if !dir_copy.is_empty() {
            app.add_systems(Update, auto_walk_system);
        }
    }
}
fn debug_screenshot(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut counter: Local<u32>,
    time: Res<Time>,
    mut acc: Local<f32>,
) {
    if std::env::var("BEVY_AUTO_SHOT").is_ok() {
        let interval: f32 = std::env::var("BEVY_SHOT_INTERVAL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2.0);
        *acc += time.delta_secs();
        if *acc >= interval {
            *acc = 0.0;
            capture_shot(&mut commands, &mut counter);
        }
    }
    if keys.just_pressed(KeyCode::F12) {
        capture_shot(&mut commands, &mut counter);
    }
}
fn capture_shot(commands: &mut Commands, counter: &mut u32) {
    // #71：截图目录不存在时自动创建（tools/ 位于仓库根，随 CWD 变化）
    if let Ok(dir) = std::path::Path::new("../tools").canonicalize() {
        let _ = std::fs::create_dir_all(dir);
    } else {
        let _ = std::fs::create_dir_all("../tools");
    }
    let path = format!("../tools/bevy_shot_{}.png", *counter);
    *counter += 1;
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
}
#[derive(Resource)]
struct AutoWalkDir(String);
/// 调试：把本地玩家按方向持续平移（--auto-walk down），用于验证 chunk 流式加载
fn auto_walk_system(
    mut timer: Local<f32>,
    time: Res<Time>,
    dir: Res<AutoWalkDir>,
    mut players: Query<&mut Transform, (With<client_bevy::actor::LocalPlayer>, With<client_bevy::actor::NetObjectId>)>,
) {
    if dir.0.is_empty() {
        return;
    }
    *timer += time.delta_secs();
    if *timer < 0.06 {
        return;
    }
    *timer = 0.0;
    let step = match dir.0.as_str() {
        "down" => Vec3::new(0.0, -client_bevy::map_renderer::TILE_HEIGHT, 0.0),
        "up" => Vec3::new(0.0, client_bevy::map_renderer::TILE_HEIGHT, 0.0),
        "left" => Vec3::new(-client_bevy::map_renderer::TILE_WIDTH, 0.0, 0.0),
        "right" => Vec3::new(client_bevy::map_renderer::TILE_WIDTH, 0.0, 0.0),
        _ => return,
    };
    for mut tf in players.iter_mut() {
        // 调试边界：向下最多走到 tile 430（n0 村庄下方），避免走出地图
        let tile_y = ((-tf.translation.y - 32.0) / 32.0).round() as i32;
        if dir.0 == "down" && tile_y >= 430 {
            return;
        }
        if dir.0 == "up" && tile_y <= 270 {
            return;
        }
        tf.translation += step;
    }
}
/// --auto-walk-diag 状态（#1145）：记录对角线移动过程中的方向序列
#[derive(Resource, Default)]
struct AutoWalkDiag {
    started: bool,
    seq: Vec<u8>,
}
/// 对角线移动方向稳定性验证（#1145）：
/// 进图 8s 后从玩家位置向 +20/+15 对角寻路，插入真实 LocalMove；
/// 记录每步 anim.direction 变化，结束时打印序列（应稳定无来回跳）。
fn auto_walk_diag_system(
    mut t: Local<f32>,
    mut state: ResMut<AutoWalkDiag>,
    time: Res<Time>,
    game_data: Res<client_bevy::map_renderer::GameData>,
    mut commands: Commands,
    // 本地玩家（LocalMove 可能不存在，移动时才插入）：用 Option 避免查询冲突（B0001）
    mut players: Query<
        (
            Entity,
            &Transform,
            &mut client_bevy::actor::ActorAnim,
            Option<&client_bevy::game::movement::LocalMove>,
        ),
        (With<client_bevy::actor::LocalPlayer>, With<client_bevy::actor::NetObjectId>),
    >,
) {
    use client_bevy::game::movement::{world_to_tile, LocalMove as LM};
    use client_bevy::map_renderer::GameData as GD;
    if !state.started {
        *t += time.delta_secs();
        if *t < 8.0 {
            return;
        }
        tracing::info!("[DIAGWALK] 开始（t={} map={}）", *t, game_data.map.is_some());
        let Ok((pe, ptf, mut anim, _)) = players.single_mut() else {
            tracing::warn!("[DIAGWALK] ❌ 本地玩家查询失败");
            return;
        };
        let Some(map) = &game_data.map else {
            tracing::warn!("[DIAGWALK] ❌ 地图未就绪");
            return;
        };
        let from = world_to_tile(ptf.translation.x, ptf.translation.y);
        let to = (from.0 + 20, from.1 + 15); // 纯 45° 对角
        if let Some(p) = client_bevy::game::pathfinding::find_path(map, from, to) {
            if p.is_empty() {
                tracing::warn!("[DIAGWALK] ❌ 对角路径不可达 {:?} -> {:?}", from, to);
                return;
            }
            let first = p[0];
            if let Some(d) =
                client_bevy::game::movement::direction_from_delta(first.0 - from.0, first.1 - from.1)
            {
                anim.direction = d as u8;
            }
            let path_len = p.len();
            commands.entity(pe).insert(LM {
                path: p.into(),
                step_timer_ms: 0.0,
                run: false,
                last: None,
                step_origin: None,
                turn_acc: 0.0,
            });
            state.seq.push(anim.direction);
            tracing::info!("[DIAGWALK] 对角寻路 {:?} -> {:?} 路径 {} 步", from, to, path_len);
        }
        state.started = true;
        return;
    }
    let Ok((_, _, anim, lm_opt)) = players.single() else { return };
    let Some(lm) = lm_opt else { return };
    if lm.path.is_empty() && anim.action == mir2_shared::enums::MirAction::Standing {
        // 结束：输出方向序列 + 抖动检查（相邻方向差 4 = 反向抖动）
        let seq = std::mem::take(&mut state.seq);
        let jitter = seq.windows(2).filter(|w| {
            let d = (w[1] as i32 - w[0] as i32).rem_euclid(8);
            d == 4
        }).count();
        let verdict = if jitter == 0 { "✅ 稳定" } else { "❌ 抖动" };
        tracing::info!("[DIAGWALK] {} 方向序列 {:?}（反向抖动 {} 次）", verdict, seq, jitter);
        state.started = false;
        return;
    }
    if anim.action != mir2_shared::enums::MirAction::Standing
        && state.seq.last() != Some(&anim.direction)
    {
        state.seq.push(anim.direction);
    }
}
/// --real-verify 状态机（合并 Local 参数，避免超 Bevy 16 参数上限）
#[derive(Default)]
struct RealVerifyState {
    t: f32,
    stage: u8,
    target: Option<u32>,
    target_tile: Option<(i32, i32)>,
    chat_sent: bool,
    chat_echo: bool,
    /// 已尝试但未命中的目标（远程怪够不着时换目标）
    tried: Vec<u32>,
    /// 进入攻击阶段时的命中计数基线
    hits_at_start: u32,
    /// 当前目标累计攻击时间（到邻接后才计时）
    attack_elapsed: f32,
    /// 到达邻接后等待服务器位置同步的计时（客户端本地移动超前，需等 UserLocation 校正）
    arrived_wait: f32,
    /// NPC 对话目标（stage 3 记录，stage 4 使用）
    npc_id: Option<u32>,
    /// 是否已发送 CallNPC（防止重复）
    npc_sent: bool,
    /// NPC 到达后等待服务器位置同步的计时
    npc_wait: f32,
    /// #304：是否已发送城镇复活（死亡处理）
    revive_sent: bool,
    /// #304：连续死亡次数（超过 3 次判定冒烟失败）
    revive_count: u8,
}
/// #304：被动弱怪名单（优先猎杀，避免守卫/高血量目标导致冒烟卡死）
fn is_passive_prey(name: &str) -> bool {
    let n = name.to_lowercase();
    [
        "deer", "doe", "chicken", "hen", "pig", "sheep", "cow", "duck", "goose", "rabbit",
        "football", "鹿", "鸡", "猪", "羊", "鸭", "鹅", "兔",
    ]
    .iter()
    .any(|k| n.contains(k))
}
/// --hold-move-test：按住移动方向驱动稳定性（#1548）
/// 进图后模拟鼠标固定在玩家 45° 方向连续 40 帧：
///   - mouse_direction 输出必须稳定（扇区容差）
///   - 方向驱动选择（原/Next/Previous 退避）在真实地图上应有可达方向
///   - 记录方向序列，断言无来回抖动（相邻帧方向差 <= 1 或退避后稳定）
#[derive(Resource, Default)]
struct HoldMoveTest {
    started: bool,
    frame: u32,
    /// 0=稳定 1=陷阱 2=冲刺
    phase: u8,
    dirs: Vec<u8>,
}
fn hold_move_test_system(
    mut t: Local<f32>,
    mut st: ResMut<HoldMoveTest>,
    time: Res<Time>,
    game_data: Res<client_bevy::map_renderer::GameData>,
    mut hud: ResMut<client_bevy::game::hud::HudState>,
    players: Query<&Transform, (With<client_bevy::actor::LocalPlayer>, With<client_bevy::actor::NetObjectId>)>,
) {
    use client_bevy::game::movement::{mouse_direction, next_direction, point_move, previous_direction};
    use mir2_shared::enums::MirDirection;
    if !st.started {
        *t += time.delta_secs();
        if *t < 8.0 {
            return;
        }
        let Ok(ptf) = players.single() else { return };
        let Some(map) = &game_data.map else { return };
        tracing::info!("[HOLDMOVE] 开始：玩家=({},{}) map={}", ptf.translation.x, ptf.translation.y, map.name);
        st.started = true;
        return;
    }
    st.frame += 1;
    st.frame += 1;
    // 阶段结束判定
    if st.frame > 40 {
        match st.phase {
            0 => {
                // 阶段0（稳定）结束：输出方向序列
                let dirs = std::mem::take(&mut st.dirs);
                let jitter = dirs.windows(2).filter(|w| {
                    let d = (w[1] as i32 - w[0] as i32).rem_euclid(8);
                    d == 4
                }).count();
                let stable = dirs.windows(2).all(|w| {
                    let d = (w[1] as i32 - w[0] as i32).rem_euclid(8);
                    d <= 2 || d >= 6
                });
                let verdict = if jitter == 0 && stable { "✅ 方向稳定无抖动" } else { "❌ 方向抖动" };
                tracing::info!("[HOLDMOVE] {} 方向序列 {:?}", verdict, dirs);
                // 进入陷阱阶段
                hud.in_trap_rock = true;
                st.frame = 0;
                st.phase = 1;
                return;
            }
            1 => {
                // 阶段1（陷阱）结束：验证方向不再前进
                let dirs = std::mem::take(&mut st.dirs);
                let verdict = if dirs.len() <= 1 {
                    "✅ 陷阱禁止移动（原地转向）"
                } else {
                    "❌ 陷阱中仍在移动"
                };
                tracing::info!("[HOLDMOVE] {} 陷阱阶段方向 {:?}", verdict, dirs);
                // 进入冲刺阶段
                hud.in_trap_rock = false;
                hud.sprint = true;
                st.frame = 0;
                st.phase = 2;
                return;
            }
            _ => {
                // 阶段2（冲刺）结束：验证可移动（3 格跑）
                let dirs = std::mem::take(&mut st.dirs);
                let moving = !dirs.is_empty();
                let verdict = if moving { "✅ 冲刺可移动（3 格跑）" } else { "❌ 冲刺未移动" };
                tracing::info!("[HOLDMOVE] {} 冲刺阶段方向 {:?}", verdict, dirs);
                hud.sprint = false;
                st.started = false;
                st.frame = 0;
                st.phase = 0;
                return;
            }
        }
    }
    let Ok(ptf) = players.single() else { return };
    let Some(map) = &game_data.map else { return };
    let player_world = Vec2::new(ptf.translation.x, ptf.translation.y);
    let mouse_world = player_world + Vec2::new(400.0, 400.0);
    let dir = mouse_direction(player_world, mouse_world);
    let from = client_bevy::game::movement::world_to_tile(player_world.x, player_world.y);
    let chosen = if hud.in_trap_rock {
        None // 陷阱禁止移动（C# CanWalk 12094 直接 false）
    } else {
        [dir, next_direction(dir), previous_direction(dir)].iter().copied().find(|d| {
            let p = point_move(from.0, from.1, *d, 1);
            map.is_walkable(p.0, p.1)
        })
    };
    let d = chosen.unwrap_or(dir);
    if st.dirs.last() != Some(&(d as u8)) {
        st.dirs.push(d as u8);
    }
}

/// UI_DUMP=1：输出全部 UI 实体（按钮 rect / 文本 / 可见性 / 所属对话框）
fn ui_dump_system(
    time: Res<Time>,
    mut acc: Local<f32>,
    mut done: Local<bool>,
    mgr: Res<client_bevy::game::dialogs::DialogManager>,
    q: Query<(
        Entity,
        &Transform,
        &Visibility,
        Option<&client_bevy::ui::sprite_ui::UiButton>,
        Option<&client_bevy::ui::sprite_ui::ButtonFrames>,
        Option<&Text2d>,
        Option<&Sprite>,
        Option<&client_bevy::game::dialogs::DialogRoot>,
    )>,
) {
    if *done {
        return;
    }
    *acc += time.delta_secs();
    if *acc < 10.0 {
        return;
    }
    *done = true;
    tracing::info!("=== UI_DUMP begin; open={:?} ===", mgr.open);
    for (e, tf, vis, btn, frames, txt, sp, root) in q.iter() {
        let vis_s = match *vis {
            Visibility::Visible => "V",
            Visibility::Hidden => "H",
            _ => "I",
        };
        let mut parts: Vec<String> = Vec::new();
        if let Some(b) = btn {
            parts.push(format!("btn=({:.0},{:.0},{:.0},{:.0})", b.rect.0, b.rect.1, b.rect.2, b.rect.3));
        }
        if frames.is_some() {
            parts.push("frames".to_string());
        }
        if let Some(t) = txt {
            parts.push(format!("txt={:?}", t.0));
        }
        if let Some(s) = sp {
            if let Some(cs) = s.custom_size {
                parts.push(format!("size=({:.0},{:.0})", cs.x, cs.y));
            }
            if let Some(r) = s.rect {
                parts.push(format!("rect=({:.0},{:.0},{:.0},{:.0})", r.min.x, r.min.y, r.max.x, r.max.y));
            }
        }
        if let Some(r) = root {
            parts.push(format!("root={:?}", r.0));
        }
        tracing::info!(
            "UI e={:?} vis={} pos=({:.0},{:.0}) z={:.1} {}",
            e,
            vis_s,
            tf.translation.x,
            -tf.translation.y,
            tf.translation.z,
            parts.join(" ")
        );
    }
    tracing::info!("=== UI_DUMP end count={} ===", q.iter().count());
}

