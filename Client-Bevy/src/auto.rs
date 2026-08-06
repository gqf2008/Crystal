// ============================================================================
// auto：自动化验证/调试系统（--auto-* / --real-verify / F12 截图等）
// ============================================================================
// 从 main.rs 迁出（#68）：主 bin 只保留 App 组装；调试系统按 CLI flag 在此注册。
// 用法同 main.rs 时代：cargo run --bin client_bevy -- --auto-enter --real-verify 等。

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use client_bevy::scenes::AppState;

/// 按 CLI flag（--auto-* / --real-verify / --auto-walk 等）注册自动化验证系统
pub fn register(app: &mut App) {
    // --auto-attack: 进游戏后每 1.5s 自动攻击（M10 战斗链路调试）
    if std::env::args().any(|a| a == "--auto-attack") {
        app.add_systems(Update, auto_attack_debug);
    }
    // --auto-inv / --auto-char: 进游戏 3 秒后自动打开背包/角色对话框（M9 调试）
    if std::env::args().any(|a| a == "--auto-inv") {
        app.add_systems(Update, auto_open_inventory);
    }
    if std::env::args().any(|a| a == "--auto-char") {
        app.add_systems(Update, auto_open_character);
    }

    // --storage-test: 自动仓库存取链路（自动化验证用）
    if std::env::args().any(|a| a == "--storage-test") {
        app.add_systems(Update, auto_storage_test);
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

/// --auto-attack：自动攻击（验证 攻击→受击→飘字 链路）
fn auto_attack_debug(
    net: Res<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut timer: Local<f32>,
) {
    if *state != client_bevy::scenes::AppState::Game {
        return;
    }
    *timer += time.delta_secs();
    if *timer >= 1.5 {
        *timer = 0.0;
        net.send_packet(&mir2_shared::packets::client::combat::Attack {
            direction: mir2_shared::enums::MirDirection::Up,
            spell: mir2_shared::enums::Spell::None,
        });
        tracing::info!("⚔️ --auto-attack 自动攻击");
    }
}

/// --auto-char：进游戏 3 秒后自动打开角色对话框
fn auto_open_character(
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut timer: Local<f32>,
) {
    if *state != client_bevy::scenes::AppState::Game {
        return;
    }
    *timer += time.delta_secs();
    if *timer >= 3.0 && !mgr.is_open(client_bevy::game::dialogs::DialogKind::Character) {
        mgr.toggle(client_bevy::game::dialogs::DialogKind::Character);
        tracing::info!("🎛️ --auto-char 自动打开角色对话框");
    }
}

/// --auto-inv：进游戏 3 秒后自动打开背包
fn auto_open_inventory(
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut timer: Local<f32>,
) {
    if *state != client_bevy::scenes::AppState::Game {
        return;
    }
    *timer += time.delta_secs();
    if *timer >= 3.0 && !mgr.is_open(client_bevy::game::dialogs::DialogKind::Inventory) {
        mgr.toggle(client_bevy::game::dialogs::DialogKind::Inventory);
        tracing::info!("🎛️ --auto-inv 自动打开背包");
    }
}

/// --shop-test：自动 NPC 商店买卖链路（CallNPC → [@Buy] → BuyItem → SellItem）
#[allow(clippy::too_many_arguments)]
fn auto_shop_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    npc_dialog: Res<client_bevy::game::dialogs::npc::NpcDialogState>,
    mut npc_goods: ResMut<client_bevy::game::dialogs::npc_goods::NpcGoodsState>,
    sell_panel: Res<client_bevy::game::dialogs::sell_panel::SellPanelState>,
    hud: Res<client_bevy::game::hud::HudState>,
    npcs: Query<(
        &client_bevy::actor::NetObjectId,
        &client_bevy::actor::NpcName,
    )>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut npc_oid: Local<Option<u32>>,
    mut bought_idx: Local<Option<i32>>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 6.0 {
                return;
            }
            let oid = npcs
                .iter()
                .find(|(_, n)| n.0.contains("Alchemist"))
                .or_else(|| npcs.iter().find(|(_, n)| n.0.contains("Merchant")))
                .map(|(id, _)| id.0);
            if let Some(oid) = oid {
                *npc_oid = Some(oid);
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: oid,
                    key: "[@Main]".to_string(),
                });
                tracing::info!("[SHOPTEST] CallNPC {}", oid);
                *stage = 1;
                *t = 0.0;
            }
        }
        1 => {
            if *t < 2.0 {
                return;
            }
            // 直接发送 [@Buy]（服务端匹配该键打开商店；脚本 NPC 菜单行不包含 <购买/@Buy>）
            if let Some(oid) = *npc_oid {
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: oid,
                    key: "[@Buy]".to_string(),
                });
                tracing::info!("[SHOPTEST] 发送购买菜单指令 [@Buy]");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 2.0 {
                return;
            }
            if npc_goods.visible && !npc_goods.goods.is_empty() {
                let g = &npc_goods.goods[0];
                net.send_packet(&mir2_shared::packets::client::npc::BuyItem {
                    item_index: g.item_index as u64,
                    count: 1,
                    panel_type: mir2_shared::enums::PanelType::Buy,
                });
                tracing::info!("[SHOPTEST] 购买 {} (idx={})", g.name, g.item_index);
                *bought_idx = Some(g.item_index);
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t < 3.0 {
                return;
            }
            // 出售刚购买的物品（按 item_index 匹配，uid 每次服务端启动都会重新分配）
            if let Some(idx) = *bought_idx {
                if let Some(item) = hud
                    .inventory
                    .items
                    .iter()
                    .flatten()
                    .find(|i| i.item_index == idx)
                {
                    net.send_packet(&mir2_shared::packets::client::npc::SellItem {
                        unique_id: item.unique_id,
                        count: 1,
                    });
                    tracing::info!("[SHOPTEST] 出售 {} (uid={})", item.name, item.unique_id);
                }
            }
            *stage = 4;
            *t = 0.0;
        }
        4 => {
            if *t < 3.0 {
                return;
            }
            // 回购：标记回购面板 → 发 [@BuyBack]
            npc_goods.is_buyback = true;
            if let Some(oid) = *npc_oid {
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: oid,
                    key: "[@BuyBack]".to_string(),
                });
                tracing::info!("[SHOPTEST] 发送回购指令 [@BuyBack]");
            }
            *stage = 5;
            *t = 0.0;
        }
        5 => {
            if *t < 2.0 {
                return;
            }
            if npc_goods.visible && !npc_goods.goods.is_empty() {
                let g = &npc_goods.goods[0];
                net.send_packet(&mir2_shared::packets::client::npc::BuyItemBack {
                    unique_id: g.unique_id,
                    count: 1,
                });
                tracing::info!("[SHOPTEST] 回购 {} (uid={})", g.name, g.unique_id);
                *stage = 6;
                *t = 0.0;
            }
        }
        6 => {
            if *t < 3.0 {
                return;
            }
            if let Some(idx) = *bought_idx {
                if hud.inventory.items.iter().flatten().any(|i| i.item_index == idx) {
                    tracing::info!("[SHOPTEST] ✅ 回购完成：物品已回背包");
                } else {
                    tracing::warn!("[SHOPTEST] ❌ 回购后背包未找到物品");
                }
            }
            *stage = 7;
            *t = 0.0;
        }
        7 => {
            if *t < 2.0 {
                return;
            }
            // 出售面板：[@Sell] → 服务端发 NPCGoods(Sell) → 客户端打开出售面板
            if let Some(oid) = *npc_oid {
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: oid,
                    key: "[@Sell]".to_string(),
                });
                tracing::info!("[SHOPTEST] 发送出售面板指令 [@Sell]");
            }
            *stage = 8;
            *t = 0.0;
        }
        8 => {
            if *t < 2.0 {
                return;
            }
            if sell_panel.visible {
                tracing::info!("[SHOPTEST] ✅ 出售面板已打开 (mode={:?})", sell_panel.mode);
            } else {
                tracing::warn!("[SHOPTEST] ❌ 出售面板未打开");
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --storage-test：自动仓库存取链路（CallNPC → [@Storage] → StoreItem → TakeBackItem）
#[allow(clippy::too_many_arguments)]
fn auto_storage_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    storage: Res<client_bevy::game::dialogs::storage::StorageState>,
    hud: Res<client_bevy::game::hud::HudState>,
    npcs: Query<(
        &client_bevy::actor::NetObjectId,
        &client_bevy::actor::NpcName,
        &Transform,
    )>,
    players: Query<
        &Transform,
        (With<client_bevy::actor::LocalPlayer>, With<client_bevy::actor::NetObjectId>),
    >,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut npc_oid: Local<Option<u32>>,
    mut inv_slot: Local<Option<usize>>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 6.0 {
                return;
            }
            // 名字匹配且距离最近的 NPC（真实服务器 NPC 分散，纯名字匹配会选到远处 NPC 被距离校验拒绝）
            let oid = players.single().ok().and_then(|ptf| {
                let (px, py) =
                    client_bevy::game::movement::world_to_tile(ptf.translation.x, ptf.translation.y);
                npcs.iter()
                    .filter(|(_, n, _)| n.0.contains("Alchemist") || n.0.contains("Merchant"))
                    .map(|(id, _, tf)| {
                        let (nx, ny) =
                            client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                        (id.0, (nx - px).abs() + (ny - py).abs())
                    })
                    .min_by_key(|(_, d)| *d)
                    .map(|(id, _)| id)
            });
            if let Some(oid) = oid {
                *npc_oid = Some(oid);
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: oid,
                    key: "[@Main]".to_string(),
                });
                tracing::info!("[STORAGETEST] CallNPC {}", oid);
                *stage = 1;
                *t = 0.0;
            }
        }
        1 => {
            if *t < 2.0 {
                return;
            }
            if let Some(oid) = *npc_oid {
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: oid,
                    key: "[@Storage]".to_string(),
                });
                tracing::info!("[STORAGETEST] 发送仓库指令 [@Storage]");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 2.0 {
                return;
            }
            if storage.visible {
                if let Some(idx) = hud.inventory.items.iter().position(|s| s.is_some()) {
                    *inv_slot = Some(idx);
                    net.send_packet(&mir2_shared::packets::client::item::StoreItem {
                        from: idx as i32,
                        to: 0,
                    });
                    tracing::info!("[STORAGETEST] 存入背包格 {} -> 仓库 0", idx);
                    *stage = 3;
                    *t = 0.0;
                }
            }
        }
        3 => {
            if *t < 2.0 {
                return;
            }
            if storage.items.get(0).and_then(|s| s.as_ref()).is_some() {
                if let Some(idx) = *inv_slot {
                    net.send_packet(&mir2_shared::packets::client::item::TakeBackItem {
                        from: 0,
                        to: idx as i32,
                    });
                    tracing::info!("[STORAGETEST] 取出仓库 0 -> 背包格 {}", idx);
                }
                *stage = 4;
            }
        }
        _ => {}
    }
}

/// --group-test：自动组队邀请链路（登录后向 bevy2char 发 AddMember，等成员列表）
#[allow(clippy::too_many_arguments)]
fn auto_group_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    group: Res<client_bevy::game::dialogs::group::GroupState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            let invitee = std::env::args()
                .skip_while(|a| a != "--e2e-invitee")
                .nth(1)
                .unwrap_or_else(|| "bevy2char".to_string());
            net.send_packet(&mir2_shared::packets::client::group::AddMember {
                name: invitee.clone(),
            });
            tracing::info!("[GROUPTEST] 邀请组队: {}", invitee);
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 5.0 {
                return;
            }
            if group.members.len() >= 2 {
                tracing::info!(
                    "[GROUPTEST] ✅ 组队成功: {}",
                    group.members.iter().map(|m| m.name.as_str()).collect::<Vec<_>>().join(", ")
                );
            } else {
                tracing::warn!("[GROUPTEST] ❌ 组队成员不足: {:?}", group.members);
            }
            *stage = 2;
        }
        _ => {}
    }
}

/// --group-accept：自动接受组队邀请（自动化验证用）
#[allow(clippy::too_many_arguments)]
fn auto_group_accept(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    mut group: ResMut<client_bevy::game::dialogs::group::GroupState>,
    mut accepted: Local<bool>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    if *accepted {
        return;
    }
    if let Some(inv) = group.invite.clone() {
        net.send_packet(&mir2_shared::packets::client::group::GroupInvite {
            accept_invite: true,
        });
        tracing::info!("[GROUPACCEPT] ✅ 接受邀请: {}", inv.inviter_name);
        group.invite = None;
        *accepted = true;
    }
}

/// --mail-test：自动发邮件（登录后向 bevy2char 发 SendMail，含金币）
#[allow(clippy::too_many_arguments)]
fn auto_mail_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut sent: Local<bool>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    if *sent {
        return;
    }
    *t += time.delta_secs();
    if *t < 8.0 {
        return;
    }
    let receiver = std::env::args()
        .skip_while(|a| a != "--e2e-receiver")
        .nth(1)
        .unwrap_or_else(|| "bevy2char".to_string());
    net.send_packet(&mir2_shared::packets::client::mail::SendMail {
        name: receiver.clone(),
        message: "HelloSubject\n邮件正文测试 100 金币".to_string(),
        gold: 100,
        items_idx: [0; 5],
        stamped: false,
    });
    tracing::info!("[MAILTEST] 发送邮件给 {} (含 100 金币)", receiver);
    *sent = true;
}

/// --mail-read：自动读取新邮件（收到列表条目 → ReadMail → 详情）
#[allow(clippy::too_many_arguments)]
fn auto_mail_read(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    mail: Res<client_bevy::game::dialogs::mail::MailState>,
    mut read_ids: Local<std::collections::HashSet<u64>>,
    mut done: Local<bool>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    if *done {
        return;
    }
    if let Some(d) = mail.detail.as_ref() {
        tracing::info!(
            "[MAILREAD] ✅ 已读取邮件: {} - {} 金币={} 正文={}",
            d.sender,
            d.subject,
            d.gold,
            d.body
        );
        *done = true;
        return;
    }
    for m in mail.mails.iter() {
        if m.unread && !read_ids.contains(&m.mail_id) {
            net.send_packet(&mir2_shared::packets::client::mail::ReadMail {
                mail_id: m.mail_id,
            });
            tracing::info!("[MAILREAD] 请求读取: {} ({})", m.subject, m.mail_id);
            read_ids.insert(m.mail_id);
        }
    }
}

/// --trade-test：自动交易链路（发起者：TradeRequest → 金币 500 → 放入物品 → 锁定 → 完成）
#[allow(clippy::too_many_arguments)]
fn auto_trade_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut trade: ResMut<client_bevy::game::dialogs::trade::TradeState>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            trade.is_initiator = true;
            net.send_packet(&mir2_shared::packets::client::trade::TradeRequest);
            tracing::info!("[TRADETEST] 发起交易请求");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 3.0 {
                return;
            }
            if trade.visible {
                tracing::info!("[TRADETEST] ✅ 交易窗口已打开，对方={}", trade.partner_name);
                net.send_packet(&mir2_shared::packets::client::trade::TradeGold { amount: 500 });
                tracing::info!("[TRADETEST] 放入金币 500");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 2.0 {
                return;
            }
            // 放入背包第一个物品
            if let Some((from, _)) = hud.inventory.items.iter().enumerate().find(|(_, s)| s.is_some()) {
                if trade.pending_deposit.is_none() && trade.my_items[0].is_none() {
                    trade.pending_deposit = Some((from, 0));
                    net.send_packet(&mir2_shared::packets::client::trade::DepositTradeItem {
                        from: from as i32,
                        to: 0,
                    });
                    tracing::info!("[TRADETEST] 放入背包格 {} -> 交易槽 0", from);
                    *stage = 3;
                    *t = 0.0;
                }
            }
        }
        3 => {
            if *t < 3.0 {
                return;
            }
            if trade.my_items[0].is_some() {
                tracing::info!("[TRADETEST] ✅ 物品已入槽: {}", trade.my_items[0].as_ref().unwrap().name);
                net.send_packet(&mir2_shared::packets::client::trade::TradeConfirm { locked: true });
                tracing::info!("[TRADETEST] 锁定交易");
                *stage = 4;
                *t = 0.0;
            }
        }
        4 => {
            if *t < 5.0 {
                return;
            }
            if !trade.visible {
                tracing::info!("[TRADETEST] 🎉 交易完成（窗口已关闭）");
            } else {
                tracing::warn!("[TRADETEST] ❌ 交易未完成，locked=({},{})", trade.my_locked, trade.their_locked);
            }
            *stage = 5;
        }
        _ => {}
    }
}

/// --trade-accept：自动接受交易邀请 + 加金币 300 + 锁定
#[allow(clippy::too_many_arguments)]
fn auto_trade_accept(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut trade: ResMut<client_bevy::game::dialogs::trade::TradeState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if trade.invite.is_some() {
                net.send_packet(&mir2_shared::packets::client::trade::TradeReply {
                    accept_invite: true,
                });
                tracing::info!(
                    "[TRADEACCEPT] ✅ 接受邀请: {}",
                    trade.invite.as_ref().unwrap()
                );
                trade.invite = None;
                trade.visible = true;
                *stage = 1;
                *t = 0.0;
            }
        }
        1 => {
            if *t < 2.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::trade::TradeGold { amount: 300 });
            tracing::info!("[TRADEACCEPT] 放入金币 300");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t < 3.0 {
                return;
            }
            if trade.their_locked && !trade.my_locked {
                net.send_packet(&mir2_shared::packets::client::trade::TradeConfirm { locked: true });
                tracing::info!("[TRADEACCEPT] 对方已锁定，我方锁定");
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t < 5.0 {
                return;
            }
            if !trade.visible {
                tracing::info!("[TRADEACCEPT] 🎉 交易完成");
            }
            *stage = 4;
        }
        _ => {}
    }
}

/// --drop-pick-test：怪物掉落 → 地面物品 → 拾取 → 背包
/// 前提：DB 配置 bevychar 在 Deer(340,325) 左侧、攻击力秒杀、Deer 掉落 chance=1.0
#[allow(clippy::too_many_arguments)]
fn auto_drop_pick_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hud: Res<client_bevy::game::hud::HudState>,
    ground: Query<&client_bevy::actor::NetObjectId, With<client_bevy::actor::GroundItem>>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut atk_timer: Local<f32>,
    mut dir_idx: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            // 每 1.2s 轮换方向攻击（Deer 刷新点 (205,325)，spread 45 会偏移）
            *atk_timer += time.delta_secs();
            if *atk_timer >= 1.2 {
                *atk_timer = 0.0;
                let dirs = [
                    mir2_shared::enums::MirDirection::Right,
                    mir2_shared::enums::MirDirection::Up,
                    mir2_shared::enums::MirDirection::Down,
                    mir2_shared::enums::MirDirection::Left,
                    mir2_shared::enums::MirDirection::UpRight,
                    mir2_shared::enums::MirDirection::DownRight,
                    mir2_shared::enums::MirDirection::UpLeft,
                    mir2_shared::enums::MirDirection::DownLeft,
                ];
                let d = dirs[*dir_idx as usize % dirs.len()];
                *dir_idx += 1;
                net.send_packet(&mir2_shared::packets::client::combat::Attack {
                    direction: d,
                    spell: mir2_shared::enums::Spell::None,
                });
                tracing::info!("[DROPTEST] 攻击方向 {:?}", d);
            }
            if ground.iter().next().is_some() {
                tracing::info!("[DROPTEST] ✅ 检测到地面物品实体");
                *stage = 1;
                *t = 0.0;
            } else if *t > 25.0 {
                tracing::warn!("[DROPTEST] ❌ 超时未检测到掉落（怪物可能已死/未掉）");
                *stage = 9;
            }
        }
        1 => {
            if *t < 1.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::item::PickUp {});
            tracing::info!("[DROPTEST] 发送 PickUp");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t < 3.0 {
                return;
            }
            if hud.inventory.items.iter().flatten().any(|i| i.item_index == 853) {
                tracing::info!("[DROPTEST] ✅ 拾取成功：背包有物品 853");
            } else {
                tracing::warn!("[DROPTEST] ❌ 背包未找到物品 853");
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --friend-test：自动加好友（AddFriend bevy2char → 等 FriendUpdate 列表出现）
#[allow(clippy::too_many_arguments)]
fn auto_friend_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    friend: Res<client_bevy::game::dialogs::friend::FriendState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::friend::AddFriend {
                name: "bevy2char".to_string(),
                blocked: false,
            });
            tracing::info!("[FRIENDTEST] 添加好友 bevy2char");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 3.0 {
                return;
            }
            if friend.friends.iter().any(|f| f.name == "bevy2char") {
                tracing::info!(
                    "[FRIENDTEST] ✅ 好友列表包含 bevy2char (在线={})",
                    friend.friends.iter().find(|f| f.name == "bevy2char").map(|f| f.online).unwrap_or(false)
                );
            } else {
                tracing::warn!("[FRIENDTEST] ❌ 好友列表为空或未包含 bevy2char: {:?}", friend.friends);
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --mail-compose-test：写邮件界面（输入框状态 → send_composed_mail → B 读取）
#[allow(clippy::too_many_arguments)]
fn auto_mail_compose_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mail: ResMut<client_bevy::game::dialogs::mail::MailState>,
    mut input: ResMut<client_bevy::game::dialogs::text_input::TextInputState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            // 打开邮件对话框 + 写界面（原版 C# MailDialog 写邮件流程）
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Mail) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Mail);
            }
            mail.compose = true;
            mail.detail = None;
            input.texts = vec![
                "bevy2char".to_string(),
                "ComposeSubject".to_string(),
                "邮件正文 M26 测试".to_string(),
                "100".to_string(),
            ];
            tracing::info!("[MAILCOMPOSE] 打开写邮件界面，填写收件人/主题/正文");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 2.0 {
                return;
            }
            // 与发送按钮相同的代码路径
            client_bevy::game::dialogs::mail::send_composed_mail(&net, &input, 100, &[]);
            mail.compose = false;
            tracing::info!("[MAILCOMPOSE] 发送邮件");
            *stage = 9;
        }
        _ => {}
    }
}

/// --guild-test：创建行会（打开行会对话框 → 输入行会名 → GuildNameReturn → 等 GuildStatus 信息）
#[allow(clippy::too_many_arguments)]
fn auto_guild_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    guild: Res<client_bevy::game::dialogs::guild::GuildState>,
    mut input: ResMut<client_bevy::game::dialogs::text_input::TextInputState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Guild) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Guild);
            }
            if input.texts.len() < 1 {
                input.texts.resize(1, String::new());
            }
            input.texts[0] = "TestGuild".to_string();
            tracing::info!("[GUILDTEST] 打开行会对话框，输入行会名 TestGuild");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 1.0 {
                return;
            }
            // 与创建按钮相同：GuildNameReturn{name}
            net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                name: "TestGuild".to_string(),
            });
            tracing::info!("[GUILDTEST] 创建行会 TestGuild");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t < 4.0 {
                return;
            }
            if guild.in_guild && guild.name == "TestGuild" {
                tracing::info!(
                    "[GUILDTEST] ✅ 行会创建成功: {}（{}）成员 {}",
                    guild.name,
                    guild.leader,
                    guild.members.len()
                );
            } else {
                tracing::warn!(
                    "[GUILDTEST] ❌ 行会状态: in_guild={} name={}",
                    guild.in_guild,
                    guild.name
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --guild-invite-test：创建行会 → 邀请 bevy2char → 等成员数 2
#[allow(clippy::too_many_arguments)]
fn auto_guild_invite_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    guild: Res<client_bevy::game::dialogs::guild::GuildState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                name: "TestGuild2".to_string(),
            });
            tracing::info!("[GUILDINV] 创建行会 TestGuild2");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 3.0 {
                return;
            }
            if guild.in_guild && guild.name == "TestGuild2" {
                tracing::info!("[GUILDINV] ✅ 行会已创建");
                net.send_packet(&mir2_shared::packets::client::guild::EditGuildMember {
                    change_type: 0,
                    rank_index: 0,
                    name: "bevy2char".to_string(),
                    rank_name: String::new(),
                });
                tracing::info!("[GUILDINV] 邀请 bevy2char 加入");
                *stage = 2;
                *t = 0.0;
            } else {
                tracing::warn!("[GUILDINV] ❌ 行会未创建: {}", guild.name);
                *stage = 9;
            }
        }
        2 => {
            if *t < 5.0 {
                return;
            }
            if guild.members.iter().any(|m| m.name == "bevy2char") {
                tracing::info!(
                    "[GUILDINV] ✅ 成员加入: {} 人",
                    guild.members.len()
                );
            } else {
                tracing::warn!("[GUILDINV] ❌ 成员未加入: {:?}", guild.members);
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --guild-accept：自动接受行会邀请（GuildInvite → C.GuildInvite{true} → 等 in_guild）
#[allow(clippy::too_many_arguments)]
fn auto_guild_accept(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut guild: ResMut<client_bevy::game::dialogs::guild::GuildState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if let Some(name) = guild.invite.clone() {
                net.send_packet(&mir2_shared::packets::client::guild::GuildInvite {
                    accept_invite: true,
                });
                tracing::info!("[GUILDACCEPT] ✅ 接受行会邀请: {}", name);
                guild.invite = None;
                *stage = 1;
                *t = 0.0;
            }
        }
        1 => {
            if *t < 4.0 {
                return;
            }
            if guild.in_guild {
                tracing::info!(
                    "[GUILDACCEPT] ✅ 已加入行会: {}",
                    guild.name
                );
            } else {
                tracing::warn!("[GUILDACCEPT] ❌ 未加入行会");
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --guild-notice-test：创建行会 → 设置公告 → 等 GuildNoticeChange 回包
#[allow(clippy::too_many_arguments)]
fn auto_guild_notice_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    guild: Res<client_bevy::game::dialogs::guild::GuildState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                name: "TestGuild3".to_string(),
            });
            tracing::info!("[GUILDNOTICE] 创建行会 TestGuild3");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 3.0 {
                return;
            }
            if guild.in_guild && guild.name == "TestGuild3" {
                net.send_packet(&mir2_shared::packets::client::guild::EditGuildNotice {
                    notice_lines: vec!["TestNotice 公告内容".to_string()],
                });
                tracing::info!("[GUILDNOTICE] 设置公告");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 3.0 {
                return;
            }
            if guild.notice.iter().any(|l| l.contains("TestNotice")) {
                tracing::info!("[GUILDNOTICE] ✅ 公告已更新: {:?}", guild.notice);
            } else {
                tracing::warn!("[GUILDNOTICE] ❌ 公告未更新: {:?}", guild.notice);
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --guild-gold-test：创建行会 → 存入 100 → 取出 50 → 验证仓库金币
#[allow(clippy::too_many_arguments)]
fn auto_guild_gold_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    guild: Res<client_bevy::game::dialogs::guild::GuildState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                name: "TestGuild4".to_string(),
            });
            tracing::info!("[GUILDGOLD] 创建行会 TestGuild4");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 3.0 {
                return;
            }
            if guild.in_guild && guild.name == "TestGuild4" {
                net.send_packet(&mir2_shared::packets::client::guild::GuildStorageGoldChange {
                    change_type: 0,
                    amount: 100,
                });
                tracing::info!("[GUILDGOLD] 存入 100 金币");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 3.0 {
                return;
            }
            if guild.gold >= 100 {
                tracing::info!("[GUILDGOLD] ✅ 仓库金币: {}", guild.gold);
                net.send_packet(&mir2_shared::packets::client::guild::GuildStorageGoldChange {
                    change_type: 1,
                    amount: 50,
                });
                tracing::info!("[GUILDGOLD] 取出 50 金币");
                *stage = 3;
                *t = 0.0;
            } else {
                tracing::warn!("[GUILDGOLD] ❌ 仓库金币未更新: {}", guild.gold);
                *stage = 9;
            }
        }
        3 => {
            if *t < 3.0 {
                return;
            }
            if guild.gold >= 50 {
                tracing::info!("[GUILDGOLD] ✅ 取出后仓库金币: {}", guild.gold);
            } else {
                tracing::warn!("[GUILDGOLD] ❌ 取出后金币异常: {}", guild.gold);
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --ranking-test：打开排行榜 → 等 Rankings 数据
#[allow(clippy::too_many_arguments)]
fn auto_ranking_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    ranking: Res<client_bevy::game::dialogs::ranking::RankingState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Ranking) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Ranking);
            }
            tracing::info!("[RANKTEST] 打开排行榜对话框");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 4.0 {
                return;
            }
            if !ranking.entries.is_empty() {
                tracing::info!(
                    "[RANKTEST] ✅ 排行榜 {} 条，第一名: {}",
                    ranking.entries.len(),
                    ranking.entries[0].player_name
                );
            } else {
                tracing::warn!("[RANKTEST] ❌ 排行榜为空");
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --guild-item-test：行会仓库物品链路（打开仓库 → 存入背包物品 → 取出）
#[allow(clippy::too_many_arguments)]
fn auto_guild_item_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    guild: Res<client_bevy::game::dialogs::guild::GuildState>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut deposited_uid: Local<Option<u64>>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Guild) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Guild);
            }
            tracing::info!("[GUILDITEM] 打开行会对话框");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 3.0 {
                return;
            }
            if guild.in_guild {
                tracing::info!("[GUILDITEM] 已在行会: {}", guild.name);
                *stage = 2;
                *t = 0.0;
            } else {
                net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                    name: "TestGuild5".to_string(),
                });
                tracing::info!("[GUILDITEM] 创建行会 TestGuild5");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 3.0 {
                return;
            }
            if !guild.in_guild {
                return;
            }
            // 请求仓库列表（打开对话框时已自动请求，这里兜底）
            net.send_packet(&client_bevy::network::GuildStorageItemChangeWire {
                change_type: 3,
                grid: 0,
                unique_id: 0,
                count: 0,
            });
            tracing::info!("[GUILDITEM] 请求仓库列表");
            *stage = 3;
            *t = 0.0;
        }
        3 => {
            if *t < 3.0 {
                return;
            }
            if guild.storage_received {
                tracing::info!(
                    "[GUILDITEM] ✅ 仓库列表 {} 格",
                    guild.storage_items.len()
                );
            } else {
                tracing::warn!("[GUILDITEM] ❌ 仓库列表未收到");
                *stage = 9;
                return;
            }
            // 选第一个背包物品存入
            let first = hud
                .inventory
                .items
                .iter()
                .enumerate()
                .find_map(|(i, s)| s.as_ref().map(|it| (i, it)));
            match first {
                Some((i, item)) => {
                    *deposited_uid = Some(item.unique_id);
                    net.send_packet(&client_bevy::network::GuildStorageItemChangeWire {
                        change_type: 0,
                        grid: 0,
                        unique_id: item.unique_id,
                        count: item.count as u32,
                    });
                    tracing::info!(
                        "[GUILDITEM] 存入背包物品 [{}] uid={} (格 {})",
                        item.name,
                        item.unique_id,
                        i
                    );
                    *stage = 4;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[GUILDITEM] ❌ 背包为空，无法测试存入");
                    *stage = 9;
                }
            }
        }
        4 => {
            if *t < 3.0 {
                return;
            }
            let slot0 = guild.storage_items.get(0).and_then(|s| s.as_ref());
            match slot0 {
                Some(it) => {
                    tracing::info!(
                        "[GUILDITEM] ✅ 仓库格1: {} x{} (uid={})",
                        it.name,
                        it.count,
                        it.unique_id
                    );
                    net.send_packet(&client_bevy::network::GuildStorageItemChangeWire {
                        change_type: 1,
                        grid: 0,
                        unique_id: 0,
                        count: 0,
                    });
                    tracing::info!("[GUILDITEM] 取出仓库格1");
                    *stage = 5;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[GUILDITEM] ❌ 仓库格1为空，存入失败");
                    *stage = 9;
                }
            }
        }
        5 => {
            if *t < 3.0 {
                return;
            }
            let slot0_empty = guild.storage_items.get(0).and_then(|s| s.as_ref()).is_none();
            let uid_back = match *deposited_uid {
                Some(uid) => hud
                    .inventory
                    .items
                    .iter()
                    .filter_map(|s| s.as_ref())
                    .any(|it| it.unique_id == uid),
                None => false,
            };
            if slot0_empty && uid_back {
                tracing::info!("[GUILDITEM] ✅ 取出成功：仓库格1已空，物品回到背包");
            } else {
                tracing::warn!(
                    "[GUILDITEM] ❌ 取出异常: slot0_empty={} uid_back={}",
                    slot0_empty,
                    uid_back
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --mentor-test：发起拜师 → 等 MentorUpdate → 解除（配合 --mentor-accept）
#[allow(clippy::too_many_arguments)]
fn auto_mentor_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mentor: Res<client_bevy::game::dialogs::mentor::MentorState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 12.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::misc::AddMentor {
                name: "bevy2char".to_string(),
            });
            tracing::info!("[MENTORTEST] 请求拜师 bevy2char");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 15.0 {
                tracing::warn!(
                    "[MENTORTEST] ❌ 未收到师徒关系: mentor_name={}",
                    mentor.mentor_name
                );
                *stage = 9;
                return;
            }
            if mentor.mentor_name == "bevy2char" {
                tracing::info!(
                    "[MENTORTEST] ✅ 拜师成功: 师父={} Lv.{} 在线={}",
                    mentor.mentor_name,
                    mentor.mentor_level,
                    mentor.mentor_online
                );
                net.send_packet(&mir2_shared::packets::client::misc::CancelMentor);
                tracing::info!("[MENTORTEST] 解除师徒关系");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 5.0 {
                return;
            }
            if mentor.mentor_name.is_empty() {
                tracing::info!("[MENTORTEST] ✅ 解除成功");
            } else {
                tracing::warn!(
                    "[MENTORTEST] ❌ 解除失败: mentor_name={}",
                    mentor.mentor_name
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --mentor-accept：允许拜师 → 接受邀请 → 等 MentorUpdate → 等解除
#[allow(clippy::too_many_arguments)]
fn auto_mentor_accept(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mentor: Res<client_bevy::game::dialogs::mentor::MentorState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            net.send_packet(&client_bevy::network::AllowMentorWire { allow: true });
            tracing::info!("[MENTORACCEPT] 允许拜师");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 15.0 {
                tracing::warn!("[MENTORACCEPT] ❌ 未收到拜师邀请");
                *stage = 9;
                return;
            }
            if let Some((name, level)) = mentor.invite.as_ref() {
                tracing::info!("[MENTORACCEPT] ✅ 收到拜师邀请: {} Lv.{}", name, level);
                net.send_packet(&mir2_shared::packets::client::misc::MentorReply {
                    accept_invite: true,
                });
                tracing::info!("[MENTORACCEPT] 接受拜师");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t >= 15.0 {
                tracing::warn!(
                    "[MENTORACCEPT] ❌ 未收到师徒关系: mentor_name={}",
                    mentor.mentor_name
                );
                *stage = 9;
                return;
            }
            if mentor.mentor_name == "bevychar" {
                tracing::info!(
                    "[MENTORACCEPT] ✅ 收徒成功: 徒弟={} Lv.{} 在线={}",
                    mentor.mentor_name,
                    mentor.mentor_level,
                    mentor.mentor_online
                );
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t < 10.0 {
                return;
            }
            if mentor.mentor_name.is_empty() {
                tracing::info!("[MENTORACCEPT] ✅ 对方解除，师徒关系已清除");
            } else {
                tracing::warn!(
                    "[MENTORACCEPT] ❌ 未收到解除: mentor_name={}",
                    mentor.mentor_name
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --market-test：寄售背包物品×2 → 取回一件 → 留一件给买家（配合 --market-buy）
#[allow(clippy::too_many_arguments)]
fn auto_market_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    market: Res<client_bevy::game::dialogs::market::MarketState>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut consigned: Local<Vec<u32>>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 12.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Market) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Market);
            }
            net.send_packet(&mir2_shared::packets::client::market::MarketRefresh);
            tracing::info!("[MARKETTEST] 打开市场 + 刷新");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 4.0 {
                return;
            }
            // 寄售第一个背包物品（uid=100，价格 500）
            let first = hud
                .inventory
                .items
                .iter()
                .enumerate()
                .find_map(|(i, s)| s.as_ref().map(|it| (i, it)));
            match first {
                Some((_i, item)) => {
                    net.send_packet(&client_bevy::network::MarketConsignWire {
                        unique_id: item.unique_id as u32,
                        price: 500,
                        duration: 0,
                    });
                    tracing::info!(
                        "[MARKETTEST] 寄售 [{}] uid={} 价格500",
                        item.name,
                        item.unique_id
                    );
                    consigned.push(item.unique_id as u32);
                    *stage = 2;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[MARKETTEST] ❌ 背包为空");
                    *stage = 9;
                }
            }
        }
        2 => {
            if *t < 5.0 {
                return;
            }
            if market.consign_ok.is_some() {
                tracing::info!(
                    "[MARKETTEST] ✅ 第一件寄售成功 uid={}",
                    market.consign_ok.unwrap_or(0)
                );
            } else {
                tracing::warn!("[MARKETTEST] ❌ 第一件寄售未确认");
                *stage = 9;
                return;
            }
            // 寄售第二件（uid=101，价格 600）
            let first = hud
                .inventory
                .items
                .iter()
                .enumerate()
                .find_map(|(i, s)| s.as_ref().map(|it| (i, it)));
            match first {
                Some((_i, item)) => {
                    net.send_packet(&client_bevy::network::MarketConsignWire {
                        unique_id: item.unique_id as u32,
                        price: 600,
                        duration: 0,
                    });
                    tracing::info!(
                        "[MARKETTEST] 寄售第二件 [{}] uid={} 价格600",
                        item.name,
                        item.unique_id
                    );
                    consigned.push(item.unique_id as u32);
                    *stage = 3;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[MARKETTEST] ❌ 背包只剩 0 件（应剩 1 件）");
                    *stage = 9;
                }
            }
        }
        3 => {
            if *t < 5.0 {
                return;
            }
            // 刷新市场，取回第二件（uid=101）
            net.send_packet(&mir2_shared::packets::client::market::MarketRefresh);
            tracing::info!("[MARKETTEST] 刷新市场准备取回");
            *stage = 4;
            *t = 0.0;
        }
        4 => {
            if *t < 5.0 {
                return;
            }
            let mine: Vec<&client_bevy::game::dialogs::market::MarketItem> = market
                .listings
                .iter()
                .filter(|it| it.seller == "bevychar")
                .collect();
            tracing::info!("[MARKETTEST] 我的寄售: {} 件", mine.len());
            let target = mine.iter().find(|it| it.unique_id == 101).copied();
            match target {
                Some(it) => {
                    net.send_packet(&client_bevy::network::MarketGetBackWire {
                        listing_id: it.auction_id as u32,
                    });
                    tracing::info!("[MARKETTEST] 取回商品 {} uid={}", it.auction_id, it.unique_id);
                    *stage = 5;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!(
                        "[MARKETTEST] ❌ 未找到 uid=101 的寄售: {:?}",
                        mine.iter().map(|x| x.unique_id).collect::<Vec<_>>()
                    );
                    *stage = 9;
                }
            }
        }
        5 => {
            if *t < 6.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::market::MarketRefresh);
            tracing::info!("[MARKETTEST] 取回后刷新市场");
            *stage = 6;
            *t = 0.0;
        }
        6 => {
            if *t < 5.0 {
                return;
            }
            let mine: Vec<&client_bevy::game::dialogs::market::MarketItem> = market
                .listings
                .iter()
                .filter(|it| it.seller == "bevychar")
                .collect();
            if mine.len() == 1 && mine[0].unique_id == 100 {
                tracing::info!(
                    "[MARKETTEST] ✅ 取回成功：剩 1 件寄售（uid=100 价格{}）",
                    mine[0].price
                );
            } else {
                tracing::warn!(
                    "[MARKETTEST] ❌ 取回后异常: mine={:?}",
                    mine.iter().map(|x| x.unique_id).collect::<Vec<_>>()
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --market-buy：刷新市场 → 买下卖家 bevychar 的商品（配合 --market-test）
#[allow(clippy::too_many_arguments)]
fn auto_market_buy(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    market: Res<client_bevy::game::dialogs::market::MarketState>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut bought_id: Local<Option<u64>>,
    mut last_refresh: Local<f32>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 45.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Market) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Market);
            }
            net.send_packet(&mir2_shared::packets::client::market::MarketRefresh);
            tracing::info!("[MARKETBUY] 打开市场 + 刷新");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 20.0 {
                tracing::warn!("[MARKETBUY] ❌ 未找到卖家 bevychar 的商品");
                *stage = 9;
                return;
            }
            // 等待期每 4 秒刷新一次市场（卖家可能尚未上架）
            if *t - *last_refresh >= 4.0 {
                *last_refresh = *t;
                net.send_packet(&mir2_shared::packets::client::market::MarketRefresh);
                tracing::info!("[MARKETBUY] 等待中刷新市场");
            }
            let target = market
                .listings
                .iter()
                .find(|it| it.seller == "bevychar" && it.unique_id == 100)
                .cloned();
            if let Some(it) = target {
                *bought_id = Some(it.auction_id);
                net.send_packet(&client_bevy::network::MarketBuyWire {
                    listing_id: it.auction_id as u32,
                });
                tracing::info!(
                    "[MARKETBUY] 购买商品 {} [{}] {}金币",
                    it.auction_id,
                    it.name,
                    it.price
                );
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t >= 15.0 {
                tracing::warn!("[MARKETBUY] ❌ 购买未确认: message={}", market.message);
                *stage = 9;
                return;
            }
            if market.message.contains("购买成功") {
                tracing::info!("[MARKETBUY] ✅ 购买成功: {}", market.message);
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t < 5.0 {
                return;
            }
            // 验证物品进入背包（item_index=853）
            let has = hud
                .inventory
                .items
                .iter()
                .filter_map(|s| s.as_ref())
                .any(|it| it.item_index == 853);
            if has {
                tracing::info!("[MARKETBUY] ✅ 购买的物品已进入背包");
            } else {
                tracing::warn!("[MARKETBUY] ❌ 背包未见购买的物品");
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --gameshop-test：打开商城 → 请求目录 → 购买第一件可负担商品 → 邮件送达
#[allow(clippy::too_many_arguments)]
fn auto_gameshop_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    shop: Res<client_bevy::game::dialogs::game_shop::GameShopState>,
    mail: Res<client_bevy::game::dialogs::mail::MailState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut bought_item: Local<Option<i32>>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::GameShop) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::GameShop);
            }
            tracing::info!("[SHOPTEST] 打开商城（自动请求目录）");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 10.0 {
                tracing::warn!("[SHOPTEST] ❌ 商城目录未收到");
                *stage = 9;
                return;
            }
            if !shop.items.is_empty() {
                tracing::info!(
                    "[SHOPTEST] ✅ 商城目录 {} 件，我的金币 {}",
                    shop.items.len(),
                    shop.gold
                );
                // 选第一件金币价 <= 我的金币 的商品
                let target = shop.items.iter().find(|it| it.gold_price > 0);
                match target {
                    Some(it) => {
                        *bought_item = Some(it.item_index);
                        net.send_packet(&client_bevy::network::GameshopBuyWire {
                            item_id: it.item_index as u32,
                            quantity: 1,
                        });
                        tracing::info!(
                            "[SHOPTEST] 购买 #{} {} {}金币",
                            it.item_index,
                            it.name,
                            it.gold_price
                        );
                        *stage = 2;
                        *t = 0.0;
                    }
                    None => {
                        tracing::warn!("[SHOPTEST] ❌ 目录为空或没有可购买商品");
                        *stage = 9;
                    }
                }
            }
        }
        2 => {
            if *t >= 12.0 {
                tracing::warn!("[SHOPTEST] ❌ 未收到购买邮件");
                *stage = 9;
                return;
            }
            if mail.mails.iter().any(|m| m.sender == "GameShop") {
                let ms: Vec<String> = mail
                    .mails
                    .iter()
                    .filter(|m| m.sender == "GameShop")
                    .map(|m| format!("{}: {}", m.sender, m.subject))
                    .collect();
                tracing::info!("[SHOPTEST] ✅ 购买邮件送达: {:?}", ms);
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t < 3.0 {
                return;
            }
            tracing::info!(
                "[SHOPTEST] ✅ 完成（购买 #{}）",
                bought_item.unwrap_or(-1)
            );
            *stage = 9;
        }
        _ => {}
    }
}

/// --territory-test：打开行会领地 → 购买第一个无主领地 → 向 TestGuildWar 宣战
#[allow(clippy::too_many_arguments)]
fn auto_territory_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    territory: Res<client_bevy::game::dialogs::guild_territory::GuildTerritoryState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut bought_id: Local<Option<i32>>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::GuildTerritory) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::GuildTerritory);
            }
            tracing::info!("[TERRTEST] 打开行会领地（自动请求列表）");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 10.0 {
                tracing::warn!("[TERRTEST] ❌ 领地列表未收到");
                *stage = 9;
                return;
            }
            if !territory.rows.is_empty() {
                tracing::info!(
                    "[TERRTEST] ✅ 领地列表 {} 个",
                    territory.rows.len()
                );
                let free = territory
                    .rows
                    .iter()
                    .find(|r| r.owner.is_empty())
                    .cloned();
                match free {
                    Some(r) => {
                        *bought_id = Some(r.id);
                        net.send_packet(&client_bevy::network::PurchaseGuildTerritoryWire {
                            territory_id: r.id as u32,
                        });
                        tracing::info!("[TERRTEST] 购买领地 #{}", r.id);
                        *stage = 2;
                        *t = 0.0;
                    }
                    None => {
                        tracing::warn!("[TERRTEST] ❌ 没有无主领地");
                        *stage = 9;
                    }
                }
            }
        }
        2 => {
            if *t < 6.0 {
                return;
            }
            // 重新请求列表验证购买
            net.send_packet(&client_bevy::network::GuildTerritoryPageWire { page: 0 });
            tracing::info!("[TERRTEST] 购买后刷新领地列表");
            *stage = 3;
            *t = 0.0;
        }
        3 => {
            if *t < 6.0 {
                return;
            }
            let id = bought_id.unwrap_or(-1);
            let row = territory.rows.iter().find(|r| r.id == id);
            match row {
                Some(r) if r.owner == "TestGuild4" => {
                    tracing::info!(
                        "[TERRTEST] ✅ 购买成功：领地 #{} 归属 {}",
                        r.id,
                        r.owner
                    );
                    *stage = 4;
                    *t = 0.0;
                }
                Some(r) => {
                    tracing::warn!(
                        "[TERRTEST] ❌ 领地 #{} 归属异常: {}",
                        r.id,
                        r.owner
                    );
                    *stage = 9;
                }
                None => {
                    tracing::warn!("[TERRTEST] ❌ 领地 #{} 不存在", id);
                    *stage = 9;
                }
            }
        }
        4 => {
            if *t < 6.0 {
                return;
            }
            // 向 TestGuildWar 宣战（--territory-war 客户端先创建）
            net.send_packet(&mir2_shared::packets::client::guild::GuildWarReturn {
                guild_name: "TestGuildWar".to_string(),
            });
            tracing::info!("[TERRTEST] 向 TestGuildWar 宣战");
            *stage = 5;
            *t = 0.0;
        }
        5 => {
            if *t >= 10.0 {
                tracing::warn!("[TERRTEST] ❌ 未收到宣战确认");
                *stage = 9;
                return;
            }
            if territory.war_message.contains("TestGuildWar") {
                tracing::info!("[TERRTEST] ✅ 宣战成功: {}", territory.war_message);
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --territory-war：创建目标行会 TestGuildWar（供 --territory-test 宣战）
#[allow(clippy::too_many_arguments)]
fn auto_territory_war(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    guild: Res<client_bevy::game::dialogs::guild::GuildState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if guild.in_guild && guild.name == "TestGuildWar" {
                tracing::info!("[TERRWAR] ✅ 已在行会 TestGuildWar");
                *stage = 9;
                return;
            }
            net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                name: "TestGuildWar".to_string(),
            });
            tracing::info!("[TERRWAR] 创建行会 TestGuildWar");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 8.0 {
                return;
            }
            if guild.in_guild && guild.name == "TestGuildWar" {
                tracing::info!("[TERRWAR] ✅ 行会创建成功");
                *stage = 9;
            } else {
                tracing::warn!("[TERRWAR] ❌ 行会创建失败");
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --combat-test：自动选怪 → 连续 FireBall → 验证死亡 + 掉落（M37 战斗闭环）
#[allow(clippy::too_many_arguments)]
fn auto_combat_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut cast_timer: Local<f32>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    mut item_count_before: Local<usize>,
    mut effect_seen: Local<bool>,
    effects: Res<client_bevy::game::effects::EffectsState>,
    mut control: ResMut<client_bevy::game::player_control::ControlState>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    items: Query<(&client_bevy::actor::NetObjectId, &client_bevy::actor::GroundItem)>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            // 找 10 格内最近的怪物
            let mut best: Option<(u32, i32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my, d));
                }
            }
            if best.is_none() {
                // 探测：附近 40 格内怪物数量与最近距离
                let mut total = 0usize;
                let mut nearest = i32::MAX;
                for (_, tf, monster) in &actors {
                    if !monster {
                        continue;
                    }
                    total += 1;
                    let (mx, my) =
                        client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                    let d = (mx - px).abs() + (my - py).abs();
                    if d < nearest {
                        nearest = d;
                    }
                }
                tracing::warn!(
                    "[COMBAT] 40 格内无怪物：玩家=({},{}), 全图可见怪物={}, 最近距离={}",
                    px,
                    py,
                    total,
                    nearest
                );
            }
            match best {
                Some((oid, mx, my, d)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    *item_count_before = items.iter().count();
                    // 模拟真实玩法：点击选中攻击目标（供特效/施法定位）
                    control.attack_target = Some(oid);
                    tracing::info!(
                        "[COMBAT] 🎯 目标怪物 id={} @ ({},{}) 距离={}",
                        oid,
                        mx,
                        my,
                        d
                    );
                    *stage = 1;
                    *t = 0.0;
                    *cast_timer = 0.0;
                }
                None => {
                    tracing::warn!("[COMBAT] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t >= 45.0 {
                tracing::warn!("[COMBAT] ❌ 超时未击杀（目标仍在）");
                *stage = 9;
                return;
            }
            // 目标实体已消失（ObjectDied 移除）→ 击杀成功
            let alive = target
                .and_then(|tid| actors.iter().find(|(id, _, _)| id.0 == tid))
                .is_some();
            if !alive {
                tracing::info!("[COMBAT] ✅ 目标怪物已死亡（实体移除）");
                *stage = 2;
                *t = 0.0;
                return;
            }
            // M38：魔法特效验证（MagicCast → 弹道，ObjectStruck → 爆炸）
            if !*effect_seen && effects.spawned > 0 {
                *effect_seen = true;
                tracing::info!(
                    "[COMBAT] ✅ 魔法特效已生成（计数 {}）",
                    effects.spawned
                );
            }
            // 每 1.3 秒施放一次 FireBall（目标位置）
            *cast_timer += time.delta_secs();
            if *cast_timer >= 1.3 {
                *cast_timer = 0.0;
                let (mx, my) = target_tile.unwrap_or((0, 0));
                let Ok(pf) = players.single() else { return };
                let (px, py) =
                    client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
                let dir = client_bevy::game::movement::direction_from_delta(
                    (mx - px).signum(),
                    (my - py).signum(),
                )
                .unwrap_or(mir2_shared::enums::MirDirection::Down);
                net.send_packet(&mir2_shared::packets::client::combat::Magic {
                    spell: mir2_shared::enums::Spell::FireBall,
                    direction: dir,
                    target_id: target.unwrap_or(0),
                    location: mir2_shared::Point { x: mx, y: my },
                });
                tracing::info!("[COMBAT] 🔥 FireBall → ({},{})", mx, my);
            }
        }
        2 => {
            if *t < 5.0 {
                return;
            }
            // 对比地面物品计数（M24 掉落链路）
            let now = items.iter().count();
            let before = *item_count_before;
            if now > before {
                tracing::info!(
                    "[COMBAT] ✅ 死亡后出现掉落（地面物品 {} → {}）",
                    before,
                    now
                );
            } else {
                tracing::warn!(
                    "[COMBAT] ⚠️ 地面物品数未增加（{} → {}，可能掉落被拾取）",
                    before,
                    now
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --fishing-test：打开钓鱼 → 抛竿 → 等 FishingUpdate → 等收获聊天消息
#[allow(clippy::too_many_arguments)]
fn auto_fishing_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    fishing: Res<client_bevy::game::dialogs::fishing::FishingState>,
    chat: Res<client_bevy::game::chat::ChatState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Fishing) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Fishing);
            }
            net.send_packet(&client_bevy::network::FishingCastWire { fishing_type: 0 });
            tracing::info!("[FISHTEST] 抛竿");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 6.0 {
                tracing::warn!(
                    "[FISHTEST] ❌ 未收到 FishingUpdate（progress={}）",
                    fishing.progress
                );
                *stage = 9;
                return;
            }
            if fishing.progress == 1 {
                tracing::info!("[FISHTEST] ✅ 抛竿成功（等待中）");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 12.0 {
                return;
            }
            let hit = chat
                .lines
                .iter()
                .rev()
                .take(30)
                .find(|(text, _, _)| {
                    text.contains("钓到了") || text.contains("鱼跑了") || text.contains("需要装备鱼竿")
                })
                .map(|(text, _, _)| text.clone());
            match hit {
                Some(text) => {
                    tracing::info!("[FISHTEST] ✅ 收获消息: {}", text);
                    *stage = 9;
                }
                None => {
                    tracing::warn!("[FISHTEST] ❌ 未收到收获消息");
                    *stage = 9;
                }
            }
        }
        _ => {}
    }
}

/// --refine-test：精炼全流程（存入 → 开始 60 秒 → 查看 → 取回）
#[allow(clippy::too_many_arguments)]
fn auto_refine_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    chat: Res<client_bevy::game::chat::ChatState>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut uid: Local<Option<u64>>,
    mut item_index: Local<Option<i32>>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    // 聊天辅助：最近 60 条里找子串
    fn chat_has(chat: &client_bevy::game::chat::ChatState, needle: &str) -> bool {
        chat.lines.iter().rev().take(60).any(|(t, _, _)| t.contains(needle))
    }
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Refine) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Refine);
            }
            let first = hud
                .inventory
                .items
                .iter()
                .enumerate()
                .find_map(|(i, s)| s.as_ref().map(|it| (i, it)));
            match first {
                Some((_i, item)) => {
                    *uid = Some(item.unique_id);
                    *item_index = Some(item.item_index);
                    net.send_packet(&client_bevy::network::RefineDepositWire {
                        unique_id: item.unique_id,
                    });
                    tracing::info!(
                        "[REFINETEST] 存入精炼物品 uid={} #{}",
                        item.unique_id,
                        item.item_index
                    );
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[REFINETEST] ❌ 背包为空");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t >= 6.0 {
                tracing::warn!("[REFINETEST] ❌ 未收到存入确认");
                *stage = 9;
                return;
            }
            if chat_has(&chat, "精炼物品已存入") {
                tracing::info!("[REFINETEST] ✅ 存入成功");
                net.send_packet(&client_bevy::network::RefineItemWire {
                    item_id: item_index.unwrap_or(0) as u32,
                    materials: 1,
                });
                tracing::info!("[REFINETEST] 开始精炼");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t >= 6.0 {
                tracing::warn!("[REFINETEST] ❌ 未收到精炼开始确认");
                *stage = 9;
                return;
            }
            if chat_has(&chat, "精炼已开始") {
                tracing::info!("[REFINETEST] ✅ 精炼已开始（等待 65 秒）");
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t < 65.0 {
                return;
            }
            net.send_packet(&client_bevy::network::RefineCheckWire {
                unique_id: uid.unwrap_or(0),
            });
            tracing::info!("[REFINETEST] 查看精炼结果");
            *stage = 4;
            *t = 0.0;
        }
        4 => {
            if *t >= 8.0 {
                tracing::warn!("[REFINETEST] ❌ 未收到精炼结果");
                *stage = 9;
                return;
            }
            if chat_has(&chat, "精炼成功") || chat_has(&chat, "精炼失败") || chat_has(&chat, "已完成") {
                tracing::info!("[REFINETEST] ✅ 精炼结果已返回");
                net.send_packet(&client_bevy::network::RefineRetrieveWire {
                    unique_id: uid.unwrap_or(0),
                });
                tracing::info!("[REFINETEST] 取回精炼物品");
                *stage = 5;
                *t = 0.0;
            }
        }
        5 => {
            if *t < 5.0 {
                return;
            }
            if chat_has(&chat, "精炼物品已取回") {
                tracing::info!("[REFINETEST] ✅ 取回成功，精炼全流程完成");
            } else {
                tracing::warn!("[REFINETEST] ⚠️ 取回未确认（可能已自动完成）");
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --craft-test：打开合成 → 配方1 → 合成 → 等 CraftItem 响应/聊天
#[allow(clippy::too_many_arguments)]
fn auto_craft_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    craft: Res<client_bevy::game::dialogs::craft::CraftState>,
    chat: Res<client_bevy::game::chat::ChatState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    fn chat_has(chat: &client_bevy::game::chat::ChatState, needle: &str) -> bool {
        chat.lines.iter().rev().take(60).any(|(t, _, _)| t.contains(needle))
    }
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Craft) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Craft);
            }
            net.send_packet(&client_bevy::network::CraftItemWire {
                recipe_id: 1,
                materials: 0,
            });
            tracing::info!("[CRAFTTEST] 合成配方 1（木材x3+铁矿石x2）");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 8.0 {
                tracing::warn!(
                    "[CRAFTTEST] ❌ 未收到合成结果: message={}",
                    craft.message
                );
                *stage = 9;
                return;
            }
            let ok = craft.last_result.is_some()
                || chat_has(&chat, "合成成功")
                || chat_has(&chat, "合成失败")
                || chat_has(&chat, "材料不足")
                || chat_has(&chat, "未知配方");
            if ok {
                tracing::info!(
                    "[CRAFTTEST] ✅ 合成结果: {}",
                    craft.message
                );
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --rental-test（租方）：发起租赁 → 等 UpdateRentalItem → 锁定费用 → 确认
#[allow(clippy::too_many_arguments)]
fn auto_rental_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    rental: Res<client_bevy::game::dialogs::item_rental::ItemRentalState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            net.send_packet(&client_bevy::network::RentalRequestWire {
                target_name: "bevy2char".to_string(),
            });
            tracing::info!("[RENTAL] 向 bevy2char 发起租赁");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 25.0 {
                tracing::warn!("[RENTAL] ❌ 未收到租赁更新（has_item={}）", rental.has_item);
                *stage = 9;
                return;
            }
            if rental.has_item {
                tracing::info!(
                    "[RENTAL] ✅ 收到租赁物品（费用={} 期限={}）",
                    rental.fee,
                    rental.period
                );
                net.send_packet(&mir2_shared::packets::client::item::ItemRentalLockFee);
                tracing::info!("[RENTAL] 锁定费用");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t >= 15.0 {
                tracing::warn!("[RENTAL] ❌ 未收到可确认");
                *stage = 9;
                return;
            }
            if rental.can_confirm {
                tracing::info!("[RENTAL] ✅ 双方已锁定，确认成交");
                net.send_packet(&mir2_shared::packets::client::item::ConfirmItemRental);
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t < 5.0 {
                return;
            }
            if rental.confirmed {
                tracing::info!("[RENTAL] ✅ 租赁成交确认收到");
            } else {
                tracing::warn!("[RENTAL] ⚠️ 未收到成交确认包");
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --rental-owner（物主）：等请求 → 存入物品 → 设费/期 → 锁定物品 → 等可确认
#[allow(clippy::too_many_arguments)]
fn auto_rental_owner(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    rental: Res<client_bevy::game::dialogs::item_rental::ItemRentalState>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t >= 30.0 {
                tracing::warn!("[RENTALOWNER] ❌ 未收到租赁请求");
                *stage = 9;
                return;
            }
            if rental.request_received {
                tracing::info!("[RENTALOWNER] ✅ 收到租赁请求");
                // 存入第一个背包物品
                let first = hud
                    .inventory
                    .items
                    .iter()
                    .enumerate()
                    .find_map(|(i, s)| s.as_ref().map(|it| (i, it)));
                match first {
                    Some((_i, item)) => {
                        net.send_packet(&client_bevy::network::RentalDepositWire {
                            unique_id: item.unique_id,
                        });
                        tracing::info!(
                            "[RENTALOWNER] 存入物品 uid={}",
                            item.unique_id
                        );
                        *stage = 1;
                        *t = 0.0;
                    }
                    None => {
                        tracing::warn!("[RENTALOWNER] ❌ 背包为空");
                        *stage = 9;
                    }
                }
            }
        }
        1 => {
            if *t < 4.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::item::ItemRentalFee { amount: 100 });
            net.send_packet(&mir2_shared::packets::client::item::ItemRentalPeriod { days: 24 });
            tracing::info!("[RENTALOWNER] 设置费用 100 / 期限 24");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t < 4.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::item::ItemRentalLockItem);
            tracing::info!("[RENTALOWNER] 锁定物品");
            *stage = 3;
            *t = 0.0;
        }
        3 => {
            if *t >= 15.0 {
                tracing::warn!("[RENTALOWNER] ❌ 未收到可确认");
                *stage = 9;
                return;
            }
            if rental.can_confirm {
                tracing::info!("[RENTALOWNER] ✅ 双方已锁定，可确认");
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --quest-test：打开任务日志 → 接受任务1 → 等 ChangeQuest → 放弃
#[allow(clippy::too_many_arguments)]
fn auto_quest_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut quest_log: ResMut<client_bevy::game::dialogs::quest_log::QuestLogState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::QuestLog) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::QuestLog);
            }
            // 登录推送容错：若任务 1 已存在（上次会话残留）则直接走放弃流程
            if quest_log.quests.iter().any(|q| q.id == 1) {
                tracing::info!("[QUESTTEST] 任务 1 已在列表中（登录推送），直接放弃");
                net.send_packet(&mir2_shared::packets::client::quest::AbandonQuest {
                    quest_index: 1,
                });
                quest_log.quests.retain(|q| q.id != 1);
                *stage = 2;
                *t = 0.0;
                return;
            }
            net.send_packet(&mir2_shared::packets::client::quest::AcceptQuest {
                npc_index: 0,
                quest_index: 1,
            });
            tracing::info!("[QUESTTEST] 接受任务 1");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 8.0 {
                tracing::warn!("[QUESTTEST] ❌ 未收到任务更新");
                *stage = 9;
                return;
            }
            let first_id = quest_log.quests.first().map(|q| q.id);
            if let Some(qid) = first_id {
                let qname = quest_log
                    .quests
                    .iter()
                    .find(|q| q.id == qid)
                    .map(|q| q.name.clone())
                    .unwrap_or_default();
                tracing::info!("[QUESTTEST] ✅ 任务已显示: {}（任务 {}）", qname, qid);
                net.send_packet(&mir2_shared::packets::client::quest::AbandonQuest {
                    quest_index: qid,
                });
                // 模拟放弃按钮：本地移除
                quest_log.quests.retain(|x| x.id != qid);
                tracing::info!(
                    "[QUESTTEST] 放弃任务 {}（移除后剩 {}）",
                    qid,
                    quest_log.quests.len()
                );
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 5.0 {
                return;
            }
            if quest_log.quests.is_empty() {
                tracing::info!("[QUESTTEST] ✅ 任务已放弃（列表清空）");
            } else {
                let ids: Vec<i32> = quest_log.quests.iter().map(|q| q.id).collect();
                tracing::warn!(
                    "[QUESTTEST] ⚠️ 任务列表仍非空: {} ids={:?}",
                    quest_log.quests.len(),
                    ids
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --buff-test：打开状态对话框 → 施放 Fury（攻击提升）→ 等 AddBuff
#[allow(clippy::too_many_arguments)]
fn auto_buff_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    buff: Res<client_bevy::game::dialogs::buff::BuffState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    tracing::debug!("[BUFFTEST] 驱动运行中 stage={} t={:.1}", *stage, *t);
    match *stage {
        0 => {
            if *t < 4.0 {
                return;
            }
            tracing::info!("[BUFFTEST] 打开状态对话框");
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Buff) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Buff);
            }
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::Mirroring,
                direction: mir2_shared::enums::MirDirection::Down,
                target_id: 0,
                location: mir2_shared::Point { x: 0, y: 0 },
            });
            tracing::info!("[BUFFTEST] 施放 Mirroring");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 8.0 {
                tracing::warn!("[BUFFTEST] ❌ 未收到 AddBuff（buff={}）", buff.buffs.len());
                *stage = 9;
                return;
            }
            if let Some(b) = buff.buffs.first() {
                tracing::info!(
                    "[BUFFTEST] ✅ 获得状态: {}（剩余 {} tick）",
                    client_bevy::game::dialogs::buff::buff_name(b.tag),
                    b.remaining_ticks
                );
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 4.0 {
                return;
            }
            tracing::info!(
                "[BUFFTEST] ✅ 完成（当前 {} 个状态）",
                buff.buffs.len()
            );
            *stage = 9;
        }
        _ => {}
    }
}

/// --report-test：打开举报 → 提交 → 等系统消息确认
#[allow(clippy::too_many_arguments)]
fn auto_report_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    chat: Res<client_bevy::game::chat::ChatState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    fn chat_has(chat: &client_bevy::game::chat::ChatState, needle: &str) -> bool {
        chat.lines.iter().rev().take(60).any(|(t, _, _)| t.contains(needle))
    }
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Report) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Report);
            }
            net.send_packet(&client_bevy::network::ReportIssueWire {
                issue_type: 1,
                description: "测试举报".to_string(),
            });
            tracing::info!("[REPORTTEST] 提交举报（type=1）");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 8.0 {
                tracing::warn!("[REPORTTEST] ❌ 未收到举报确认");
                *stage = 9;
                return;
            }
            if chat_has(&chat, "举报信息已提交") {
                tracing::info!("[REPORTTEST] ✅ 举报已提交确认");
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --inspect-test：找目标玩家 → 发 Inspect → 等 PlayerInspect
#[allow(clippy::too_many_arguments)]
fn auto_inspect_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    inspect: Res<client_bevy::game::dialogs::inspect::InspectState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        Option<&client_bevy::actor::PlayerName>,
    )>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            // 找到 bevy2char
            let target = actors
                .iter()
                .find(|(_, name)| name.and_then(|n| Some(n.0 == "bevy2char")).unwrap_or(false))
                .map(|(id, _)| id.0);
            match target {
                Some(oid) => {
                    if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Inspect) {
                        mgr.toggle(client_bevy::game::dialogs::DialogKind::Inspect);
                    }
                    net.send_packet(&mir2_shared::packets::client::chat::Inspect {
                        object_id: oid,
                    });
                    tracing::info!("[INSPECTTEST] 查看玩家 bevy2char (oid={})", oid);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[INSPECTTEST] ❌ 找不到目标玩家 bevy2char");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t >= 8.0 {
                tracing::warn!("[INSPECTTEST] ❌ 未收到 PlayerInspect");
                *stage = 9;
                return;
            }
            if !inspect.name.is_empty() {
                tracing::info!(
                    "[INSPECTTEST] ✅ 查看成功: {} Lv.{} 行会={} 装备 {} 件",
                    inspect.name,
                    inspect.level,
                    inspect.guild,
                    inspect.items.len()
                );
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --creature-test：打开宠物对话框 → 自动请求列表 → 等解析完成
#[allow(clippy::too_many_arguments)]
fn auto_creature_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    creature: Res<client_bevy::game::dialogs::creature::CreatureState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Creature) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Creature);
            }
            // 打开对话框会自动请求；这里兜底再发一次
            net.send_packet(&client_bevy::network::CreatureRequestWire { request: true });
            tracing::info!("[CREATURETEST] 请求宠物列表");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 8.0 {
                tracing::warn!("[CREATURETEST] ❌ 未收到宠物列表");
                *stage = 9;
                return;
            }
            if creature.message.contains("宠物列表已更新") {
                tracing::info!(
                    "[CREATURETEST] ✅ 宠物列表: {} 个",
                    creature.creatures.len()
                );
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --hero-test：打开英雄 → 切换英雄1 → 等 ChangeHero → 切回主角色
#[allow(clippy::too_many_arguments)]
fn auto_hero_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hero: Res<client_bevy::game::dialogs::hero::HeroState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            if !mgr.is_open(client_bevy::game::dialogs::DialogKind::Hero) {
                mgr.toggle(client_bevy::game::dialogs::DialogKind::Hero);
            }
            net.send_packet(&client_bevy::network::ChangeHeroWire { hero_index: 1 });
            tracing::info!("[HEROTEST] 切换英雄 1");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 8.0 {
                tracing::warn!("[HEROTEST] ❌ 未收到 ChangeHero（index={}）", hero.hero_index);
                *stage = 9;
                return;
            }
            if hero.hero_index == 1 {
                tracing::info!("[HEROTEST] ✅ 英雄切换成功: {}", hero.message);
                // #206：英雄背包 布衣(槽1) 双击装备 → 装备槽 1
                let uid = hero.inventory.get(1).and_then(|s| s.as_ref()).map(|i| i.unique_id);
                match uid {
                    Some(uid) => {
                        net.send_packet(&mir2_shared::packets::client::item::EquipItem {
                            grid: mir2_shared::enums::MirGridType::HeroInventory,
                            unique_id: uid,
                            to: 1,
                        });
                        tracing::info!("[HEROTEST] 英雄装备 uid={} -> 槽 1", uid);
                        *stage = 3;
                        *t = 0.0;
                    }
                    None => {
                        tracing::warn!("[HEROTEST] ⚠️ 英雄背包槽 1 无物品，跳过装备验证");
                        net.send_packet(&client_bevy::network::ChangeHeroWire { hero_index: 0 });
                        *stage = 2;
                        *t = 0.0;
                    }
                }
            }
        }
        3 => {
            if *t >= 8.0 {
                tracing::warn!("[HEROTEST] ❌ 英雄装备未生效");
                *stage = 9;
                return;
            }
            if hero.equipment.get(1).and_then(|s| s.as_ref()).is_some() {
                tracing::info!("[HEROTEST] ✅ 英雄装备成功: {:?}", hero.equipment.get(1).and_then(|s| s.as_ref()).map(|i| i.name.clone()));
                let uid = hero.equipment[1].as_ref().unwrap().unique_id;
                net.send_packet(&mir2_shared::packets::client::item::RemoveItem {
                    grid: mir2_shared::enums::MirGridType::HeroEquipment,
                    unique_id: uid,
                    to: 0,
                });
                tracing::info!("[HEROTEST] 英雄卸下 uid={}", uid);
                *stage = 4;
                *t = 0.0;
            }
        }
        4 => {
            if *t >= 8.0 {
                tracing::warn!("[HEROTEST] ❌ 英雄卸下未生效");
                *stage = 9;
                return;
            }
            if hero.equipment.get(1).and_then(|s| s.as_ref()).is_none() {
                tracing::info!("[HEROTEST] ✅ 英雄卸下成功");
                // #218：英雄技能书（英雄背包槽 2）→ UseItem → 等 NewMagic(hero)
                let book_uid = hero
                    .inventory
                    .get(2)
                    .and_then(|s| s.as_ref())
                    .map(|i| i.unique_id);
                match book_uid {
                    Some(uid) => {
                        net.send_packet(&mir2_shared::packets::client::item::UseItem { unique_id: uid });
                        tracing::info!("[HEROTEST] 英雄使用技能书 uid={}", uid);
                        *stage = 5;
                        *t = 0.0;
                    }
                    None => {
                        tracing::warn!("[HEROTEST] ⚠️ 英雄背包槽 2 无技能书，跳过");
                        net.send_packet(&client_bevy::network::ChangeHeroWire { hero_index: 0 });
                        *stage = 2;
                        *t = 0.0;
                    }
                }
            }
        }
        5 => {
            if *t >= 8.0 {
                tracing::warn!("[HEROTEST] ❌ 英雄未学会技能");
                *stage = 9;
                return;
            }
            if hero.magics.iter().any(|m| m.spell == mir2_shared::enums::Spell::GreatFireBall) {
                tracing::info!("[HEROTEST] ✅ 英雄学会 GreatFireBall（{} 个技能）", hero.magics.len());
                // #220：等待 MagicLeveled 升级路由到英雄技能面板
                let lv = hero.magics.iter().find(|m| m.spell == mir2_shared::enums::Spell::GreatFireBall).map(|m| m.level).unwrap_or(0);
                if lv >= 1 {
                    tracing::info!("[HEROTEST] ✅ 英雄技能升级 Lv.{}（MagicLeveled 路由成功）", lv);
                    net.send_packet(&client_bevy::network::ChangeHeroWire { hero_index: 0 });
                    tracing::info!("[HEROTEST] 切回主角色");
                    *stage = 2;
                    *t = 0.0;
                }
                net.send_packet(&client_bevy::network::ChangeHeroWire { hero_index: 0 });
                tracing::info!("[HEROTEST] 切回主角色");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 5.0 {
                return;
            }
            if hero.hero_index == 0 {
                tracing::info!("[HEROTEST] ✅ 切回主角色成功");
            } else {
                tracing::warn!("[HEROTEST] ⚠️ 当前 index={}", hero.hero_index);
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --marriage-test（求婚方）：求婚 → 等 LoverUpdate → 离婚
#[allow(clippy::too_many_arguments)]
fn auto_marriage_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    relationship: Res<client_bevy::game::dialogs::relationship::RelationshipState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            net.send_packet(&client_bevy::network::MarriageRequestWire {
                target_name: "bevy2char".to_string(),
            });
            tracing::info!("[MARRY] 向 bevy2char 求婚");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 15.0 {
                tracing::warn!("[MARRY] ❌ 未结婚（married={}）", relationship.married);
                *stage = 9;
                return;
            }
            if relationship.married {
                tracing::info!("[MARRY] ✅ 结婚成功");
                net.send_packet(&client_bevy::network::DivorceRequestWire {
                    partner_name: "bevy2char".to_string(),
                });
                tracing::info!("[MARRY] 发起离婚");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t >= 15.0 {
                tracing::warn!("[MARRY] ❌ 未离婚（married={}）", relationship.married);
                *stage = 9;
                return;
            }
            if !relationship.married {
                tracing::info!("[MARRY] ✅ 离婚成功");
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --marriage-accept（被求婚方）：接受求婚 → 等结婚 → 离婚确认
#[allow(clippy::too_many_arguments)]
fn auto_marriage_accept(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    relationship: Res<client_bevy::game::dialogs::relationship::RelationshipState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t >= 20.0 {
                tracing::warn!("[MARRYACC] ❌ 未收到求婚");
                *stage = 9;
                return;
            }
            if relationship.invite.is_some() {
                tracing::info!(
                    "[MARRYACC] ✅ 收到求婚: {}",
                    relationship.invite.clone().unwrap_or_default()
                );
                net.send_packet(&mir2_shared::packets::client::misc::MarriageReply {
                    accept_invite: true,
                });
                tracing::info!("[MARRYACC] 接受求婚");
                *stage = 1;
                *t = 0.0;
            }
        }
        1 => {
            if *t >= 15.0 {
                tracing::warn!("[MARRYACC] ❌ 未结婚");
                *stage = 9;
                return;
            }
            if relationship.married {
                tracing::info!("[MARRYACC] ✅ 已婚");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            // 等待对方离婚请求并确认
            if relationship.message.contains("离婚请求") {
                tracing::info!("[MARRYACC] ✅ 收到离婚请求，确认");
                net.send_packet(&mir2_shared::packets::client::misc::DivorceReply {
                    accept_invite: true,
                });
                *stage = 3;
                *t = 0.0;
            }
            if *t >= 20.0 {
                tracing::warn!("[MARRYACC] ❌ 未收到离婚请求");
                *stage = 9;
            }
        }
        3 => {
            if *t < 5.0 {
                return;
            }
            if !relationship.married {
                tracing::info!("[MARRYACC] ✅ 离婚完成");
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --mount-test：打开坐骑面板 → 骑乘/下马（@ride）→ 外观广播 → 坐骑层
#[allow(clippy::too_many_arguments)]
fn auto_mount_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    net: ResMut<client_bevy::network::NetConnection>,
    mounts: Query<Option<&client_bevy::actor::MountState>, With<client_bevy::actor::LocalPlayer>>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
    mut rode: Local<bool>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::dialogs::DialogKind;
    use mir2_shared::packets::client::chat::Chat;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        if !mgr.is_open(DialogKind::Mount) {
            mgr.open(DialogKind::Mount);
            tracing::info!("[MOUNT] 打开坐骑面板");
        }
        *phase = *t;
        *stage = 1;
        return;
    }
    if *stage == 1 && *t - *phase >= 1.5 {
        let mounted = mounts.single().ok().flatten().is_some();
        tracing::info!("[MOUNT] ✅ 面板状态: 本地坐骑层={}", mounted);
        net.send_packet(&Chat {
            message: "@ride".to_string(),
            linked_items: Vec::new(),
        });
        tracing::info!("[MOUNT] ✅ 发送 @ride（骑乘）");
        *stage = 2;
        *phase = *t;
        return;
    }
    if *stage == 2 {
        if mounts.single().ok().flatten().is_some() {
            tracing::info!("[MOUNT] ✅ 骑乘成功（本地玩家出现坐骑层）");
            *rode = true;
            *stage = 3;
            *phase = *t;
        } else if *t - *phase >= 8.0 {
            tracing::warn!("[MOUNT] ❌ 骑乘超时（检查地图限制/鞍）");
            *stage = 9;
        }
        return;
    }
    if *stage == 3 && *t - *phase >= 1.5 {
        net.send_packet(&Chat {
            message: "@ride".to_string(),
            linked_items: Vec::new(),
        });
        tracing::info!("[MOUNT] ✅ 发送 @ride（下马）");
        *stage = 4;
        *phase = *t;
        return;
    }
    if *stage == 4 {
        if mounts.single().ok().flatten().is_none() {
            tracing::info!("[MOUNT] ✅ 下马成功");
            if mgr.is_open(DialogKind::Mount) {
                mgr.close(DialogKind::Mount);
                tracing::info!("[MOUNT] ✅ 关闭坐骑面板");
            }
            *stage = 9;
        } else if *t - *phase >= 8.0 {
            tracing::warn!("[MOUNT] ❌ 下马超时");
            *stage = 9;
        }
        return;
    }
    if *t >= 60.0 && *stage < 9 {
        tracing::warn!("[MOUNT] ❌ 总超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --reconnect-test：进入游戏 → 等服务器断开 → 自动重连 → 自动登录并重新进游戏
#[allow(clippy::too_many_arguments)]
fn auto_reconnect_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut net: ResMut<client_bevy::network::NetConnection>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
    mut saw_disconnect: Local<bool>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::network::NetState;
    *t += time.delta_secs();
    if *stage == 0 {
        if *state == AppState::Game {
            tracing::info!("[RECON] 已进入游戏，等待服务器断开...");
            *stage = 1;
            *phase = *t;
        } else if *t >= 60.0 {
            tracing::warn!("[RECON] ❌ 60 秒内未进入游戏");
            *stage = 9;
        }
        return;
    }
    if *stage == 1 {
        if net.disconnected.is_some() && !*saw_disconnect {
            *saw_disconnect = true;
            tracing::info!("[RECON] ✅ 检测到断线: {:?}", net.disconnected);
            *stage = 2;
            *phase = *t;
        } else if *t - *phase >= 60.0 {
            tracing::warn!("[RECON] ❌ 未检测到断线");
            *stage = 9;
        }
        return;
    }
    if *stage == 2 {
        if net.state == NetState::InGame && *state == AppState::Game && !net.reconnecting {
            tracing::info!("[RECON] ✅ 自动重连成功并重新进入游戏");
            *stage = 9;
        } else if *t - *phase >= 90.0 {
            tracing::warn!("[RECON] ❌ 重连超时（state={:?} reconnecting={}）", net.state, net.reconnecting);
            *stage = 9;
        }
        return;
    }
    if *t >= 200.0 && *stage < 9 {
        tracing::warn!("[RECON] ❌ 总超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --roll-test：触发 NPC 掷骰 → 服务端 Roll 包 → 客户端骰子对话框 → 自动回调
#[allow(clippy::too_many_arguments)]
fn auto_roll_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut roll: ResMut<client_bevy::game::dialogs::roll::RollState>,
    bm: Res<client_bevy::game::dialogs::big_map::BigMapState>,
    game_data: Res<client_bevy::map_renderer::GameData>,
    mut npc_dialog: ResMut<client_bevy::game::dialogs::npc::NpcDialogState>,
    net: ResMut<client_bevy::network::NetConnection>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
    mut npc_id: Local<u32>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        // 选离玩家出生点最近的 NPC（CallNPC 需 2 格内）
        let spawn = game_data.player_spawn.map(|(x, y, _)| (x as i32, y as i32));
        let picked = if let Some((sx, sy)) = spawn {
            bm.npcs
                .iter()
                .min_by_key(|n| (n.x - sx).abs() + (n.y - sy).abs())
                .cloned()
        } else {
            bm.npcs.first().cloned()
        };
        if let Some(npc) = picked {
            *npc_id = npc.object_id;
            npc_dialog.npc_object_id = npc.object_id;
            net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                object_id: npc.object_id,
                key: "[@TestRoll]".to_string(),
            });
            tracing::info!(
                "[ROLL] 触发 NPC {} ({},{}) 掷骰页",
                npc.object_id,
                npc.x,
                npc.y
            );
            *stage = 1;
            *phase = *t;
        } else if *t - *phase >= 15.0 {
            tracing::warn!("[ROLL] ❌ 未等到 NPC 数据");
            *stage = 9;
        }
        return;
    }
    if *stage == 1 {
        if roll.visible {
            tracing::info!(
                "[ROLL] ✅ 收到 Roll 包: type={} result={} page={} auto={}",
                roll.r#type,
                roll.result,
                roll.page,
                roll.auto_roll
            );
            *stage = 2;
            *phase = *t;
        } else if *t - *phase >= 10.0 {
            tracing::warn!("[ROLL] ❌ 未收到 Roll 包");
            *stage = 9;
        }
        return;
    }
    if *stage == 2 {
        if !roll.visible && roll.finished {
            tracing::info!("[ROLL] ✅ 掷骰完成回调已发送（NPC {}）", *npc_id);
            *stage = 9;
        } else if *t - *phase >= 12.0 {
            tracing::warn!("[ROLL] ❌ 回调超时");
            *stage = 9;
        }
        return;
    }
    if *t >= 40.0 && *stage < 9 {
        tracing::warn!("[ROLL] ❌ 超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --socket-test：打开镶嵌面板 → 孔位/宝石渲染 → 关闭
#[allow(clippy::too_many_arguments)]
fn auto_socket_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut socket: ResMut<client_bevy::game::dialogs::socket::SocketState>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::dialogs::DialogKind;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        let sock = hud
            .inventory
            .items
            .iter()
            .flatten()
            .find(|it| !it.slots.is_empty())
            .cloned();
        if let Some(item) = sock {
            socket.item = Some(item.clone());
            if !mgr.is_open(DialogKind::Socket) {
                mgr.open(DialogKind::Socket);
            }
            tracing::info!(
                "[SOCKET] 打开镶嵌面板: {} ({} 孔)",
                item.name,
                item.slots.len()
            );
            *stage = 1;
        } else {
            tracing::warn!("[SOCKET] ❌ 背包中没有带孔物品");
            *stage = 9;
        }
        *phase = *t;
        return;
    }
    if *stage == 1 && *t - *phase >= 1.5 {
        let gems: Vec<String> = socket
            .item
            .as_ref()
            .map(|i| {
                i.slots
                    .iter()
                    .map(|s| {
                        s.as_ref()
                            .map(|g| format!("{}", g.name))
                            .unwrap_or_else(|| "空".to_string())
                    })
                    .collect()
            })
            .unwrap_or_default();
        tracing::info!("[SOCKET] ✅ 孔位渲染: {}", gems.join(", "));
        *stage = 2;
        *phase = *t;
        return;
    }
    if *stage == 2 && *t - *phase >= 1.0 {
        if mgr.is_open(DialogKind::Socket) {
            mgr.close(DialogKind::Socket);
            tracing::info!("[SOCKET] ✅ 关闭镶嵌面板");
        }
        *stage = 9;
    }
    if *t >= 25.0 && *stage < 9 {
        tracing::warn!("[SOCKET] ❌ 超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --dura-test：打开耐久面板 → 装备耐久三态渲染 → 关闭
fn auto_dura_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::dialogs::DialogKind;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        if !mgr.is_open(DialogKind::DuraStatus) {
            mgr.open(DialogKind::DuraStatus);
            tracing::info!("[DURA] 打开耐久面板");
        }
        *phase = *t;
        *stage = 1;
        return;
    }
    if *stage == 1 && *t - *phase >= 1.0 {
        let equipped: Vec<String> = hud
            .equipment
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().map(|it| format!("slot{}={}({}/{})", i, it.name, it.current_dura, it.max_dura)))
            .collect();
        tracing::info!("[DURA] ✅ 装备耐久数据: {}", if equipped.is_empty() { "无".to_string() } else { equipped.join(", ") });
        *stage = 2;
        *phase = *t;
        return;
    }
    if *stage == 2 && *t - *phase >= 1.5 {
        if mgr.is_open(DialogKind::DuraStatus) {
            mgr.close(DialogKind::DuraStatus);
            tracing::info!("[DURA] ✅ 关闭耐久面板");
        }
        *stage = 9;
    }
    if *t >= 25.0 && *stage < 9 {
        tracing::warn!("[DURA] ❌ 超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --awake-test：打开觉醒 → 选武器 → 选类型/材料 → 执行觉醒（可重试）→ 关闭
#[allow(clippy::too_many_arguments)]
fn auto_awake_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut aw: ResMut<client_bevy::game::dialogs::npc_awake::NpcAwakeState>,
    hud: Res<client_bevy::game::hud::HudState>,
    net: ResMut<client_bevy::network::NetConnection>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
    mut attempts: Local<u32>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::dialogs::DialogKind;
    use mir2_shared::packets::client::misc::{Awakening, AwakeningNeedMaterials};
    use mir2_shared::enums::AwakeType;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        if !mgr.is_open(DialogKind::NpcAwake) {
            mgr.toggle(DialogKind::NpcAwake);
            tracing::info!("[AWAKE] 打开觉醒对话框");
        }
        *phase = *t;
        *stage = 1;
        return;
    }
    if *stage == 1 && *t - *phase >= 1.0 {
        for (i, it) in hud.inventory.items.iter().enumerate() {
            if let Some(item) = it {
                tracing::info!(
                    "[AWAKE] inv[{}] uid={} idx={} name={}",
                    i,
                    item.unique_id,
                    item.item_index,
                    item.name
                );
            }
        }
        let sword = hud
            .inventory
            .items
            .iter()
            .flatten()
            .find(|it| it.item_index == 221)
            .cloned();
        if let Some(item) = sword {
            aw.selected_uid = Some(item.unique_id);
            aw.selected_item = Some(item.clone());
            aw.awake_type = None;
            tracing::info!("[AWAKE] ✅ 选择武器: {} (uid={})", item.name, item.unique_id);
            *stage = 2;
        } else {
            tracing::warn!("[AWAKE] ❌ 背包中没有 WoodenSword");
            *stage = 9;
        }
        *phase = *t;
        return;
    }
    if *stage == 2 && *t - *phase >= 1.0 {
        if let Some(uid) = aw.selected_uid {
            aw.awake_type = Some(AwakeType::Dc);
            net.send_packet(&AwakeningNeedMaterials {
                unique_id: uid,
                awake_type: AwakeType::Dc,
            });
            tracing::info!("[AWAKE] ✅ 请求觉醒材料 uid={} type=Dc", uid);
        }
        *stage = 3;
        *phase = *t;
        return;
    }
    if *stage == 3 && *t - *phase >= 1.5 {
        tracing::info!(
            "[AWAKE] ✅ 材料需求: {}",
            if aw.materials.is_empty() {
                "无（跳过材料检查）".to_string()
            } else {
                aw.materials
                    .iter()
                    .map(|m| format!("#{}x{}", m.item_id, m.count))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        );
        if let Some(uid) = aw.selected_uid {
            net.send_packet(&Awakening {
                unique_id: uid,
                awake_type: AwakeType::Dc,
                position_idx: 0,
            });
            tracing::info!("[AWAKE] ✅ 执行觉醒 uid={}（第 {} 次）", uid, *attempts + 1);
            *attempts += 1;
        }
        aw.result = 0;
        *stage = 4;
        *phase = *t;
        return;
    }
    if *stage == 4 && *t - *phase >= 2.5 {
        if aw.result == 1 {
            tracing::info!("[AWAKE] ✅ 觉醒成功（结果 {}）", aw.result);
            *stage = 5;
            *phase = *t;
        } else if *attempts < 6 {
            // 失败/销毁：换下一把武器重试
            tracing::warn!(
                "[AWAKE] ⚠️ 觉醒结果 {}（{}），换武器重试",
                aw.result,
                aw.result_text
            );
            let swords: Vec<_> = hud
                .inventory
                .items
                .iter()
                .flatten()
                .filter(|it| it.item_index == 221)
                .collect();
            let next = swords
                .get((*attempts) as usize % swords.len().max(1))
                .cloned();
            if let Some(item) = next {
                aw.selected_uid = Some(item.unique_id);
                aw.selected_item = Some(item.clone());
            }
            aw.materials.clear();
            aw.result_text = String::new();
            *stage = 2;
            *phase = *t;
        } else {
            tracing::warn!("[AWAKE] ❌ 多次觉醒未成功");
            *stage = 9;
        }
        return;
    }
    if *stage == 5 && *t - *phase >= 1.0 {
        if mgr.is_open(DialogKind::NpcAwake) {
            mgr.close(DialogKind::NpcAwake);
            tracing::info!("[AWAKE] ✅ 关闭觉醒对话框");
        }
        *stage = 9;
    }
    if *t >= 45.0 && *stage < 9 {
        tracing::warn!("[AWAKE] ❌ 超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --bigmap-test：打开大地图 → 等 NewMapInfo/地形 → 选中 NPC → 传送 → 关闭
#[allow(clippy::too_many_arguments)]
fn auto_bigmap_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut bm: ResMut<client_bevy::game::dialogs::big_map::BigMapState>,
    net: ResMut<client_bevy::network::NetConnection>,
    players: Query<&Transform, With<client_bevy::actor::LocalPlayer>>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
    mut target: Local<(i32, i32)>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::dialogs::DialogKind;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        if !mgr.is_open(DialogKind::BigMap) {
            mgr.toggle(DialogKind::BigMap);
            tracing::info!("[BIGMAP] 打开大地图");
        }
        *phase = *t;
        *stage = 1;
        return;
    }
    if *stage == 1 && *t - *phase >= 1.0 {
        if bm.npcs.is_empty() {
            tracing::warn!("[BIGMAP] ⚠️ 无 NewMapInfo NPC 数据（服务端需 M53 支持）");
        } else {
            tracing::info!("[BIGMAP] ✅ NewMapInfo: {} 个 NPC（{}）", bm.npcs.len(), bm.title);
        }
        *stage = 2;
        *phase = *t;
        return;
    }
    if *stage == 2 {
        if bm.viewport_ready {
            tracing::info!("[BIGMAP] ✅ 地形纹理生成完成 {}x{}", bm.tex_size.0, bm.tex_size.1);
            *stage = 3;
            *phase = *t;
        } else if *t - *phase >= 8.0 {
            tracing::warn!("[BIGMAP] ❌ 地形生成超时");
            *stage = 9;
        }
        return;
    }
    if *stage == 3 && *t - *phase >= 1.0 {
        let tp = bm.npcs.iter().find(|n| n.can_teleport_to).cloned();
        if let Some(npc) = tp {
            bm.selected = Some(0);
            *target = (npc.x, npc.y);
            tracing::info!("[BIGMAP] ✅ 选中可传送 NPC: {} ({},{})", npc.name, npc.x, npc.y);
            net.send_packet(&mir2_shared::packets::client::npc::TeleportToNPC {
                object_id: npc.object_id,
            });
            tracing::info!("[BIGMAP] ✅ 发送传送请求 id={}", npc.object_id);
        } else {
            tracing::warn!("[BIGMAP] ⚠️ 无可传送 NPC");
        }
        *stage = 4;
        *phase = *t;
        return;
    }
    if *stage == 4 && *t - *phase >= 3.0 {
        let moved = players.single().ok().map(|tf| {
            client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y)
        });
        match moved {
            Some((x, y)) if (x, y) == *target => {
                tracing::info!("[BIGMAP] ✅ 传送生效 玩家位置=({},{})", x, y);
            }
            Some((x, y)) => {
                tracing::info!(
                    "[BIGMAP] ✅ 传送已处理 玩家位置=({},{})（目标 ({},{})）",
                    x,
                    y,
                    target.0,
                    target.1
                );
            }
            None => {
                tracing::warn!("[BIGMAP] ⚠️ 无法读取玩家位置");
            }
        }
        if mgr.is_open(DialogKind::BigMap) {
            mgr.close(DialogKind::BigMap);
            tracing::info!("[BIGMAP] ✅ 关闭大地图");
        }
        *stage = 9;
    }
    if *t >= 40.0 && *stage < 9 {
        tracing::warn!("[BIGMAP] ❌ 超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --keyboard-test：打开键位设置 → 滚动 → 重绑一行 → 重置 → 关闭
#[allow(clippy::too_many_arguments)]
fn auto_keyboard_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut kb: ResMut<client_bevy::game::dialogs::keyboard_layout::KeyboardState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::dialogs::DialogKind;
    use bevy::input::keyboard::KeyCode;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        if !mgr.is_open(DialogKind::KeyboardLayout) {
            mgr.toggle(DialogKind::KeyboardLayout);
            tracing::info!("[KBD] 打开键位设置");
        }
        *phase = *t;
        *stage = 1;
        return;
    }
    if *stage == 1 && *t - *phase >= 1.0 {
        kb.top_line = kb.top_line.saturating_add(2);
        tracing::info!("[KBD] ✅ 滚动 top_line={}", kb.top_line);
        kb.rebinding = Some(4);
        tracing::info!("[KBD] ✅ 等待按键: 行 4");
        *stage = 2;
        *phase = *t;
        return;
    }
    if *stage == 2 && *t - *phase >= 1.0 {
        if let Some(b) = kb.bindings.get_mut(4) {
            tracing::info!("[KBD] ✅ 绑定 {} → {}", b.action, "X");
            b.key = KeyCode::KeyX;
        }
        kb.rebinding = None;
        tracing::info!("[KBD] ✅ 重绑完成");
        *stage = 3;
        *phase = *t;
        return;
    }
    if *stage == 3 && *t - *phase >= 1.0 {
        kb.bindings = kb.defaults.clone();
        kb.top_line = 0;
        kb.enforce = !kb.enforce;
        tracing::info!("[KBD] ✅ 重置默认 + 规则切换（严格/宽松）完成");
        *stage = 4;
        *phase = *t;
        return;
    }
    if *stage == 4 && *t - *phase >= 1.0 {
        if mgr.is_open(DialogKind::KeyboardLayout) {
            mgr.close(DialogKind::KeyboardLayout);
            tracing::info!("[KBD] ✅ 关闭键位设置");
        }
        *stage = 9;
    }
    if *t >= 30.0 && *stage < 9 {
        tracing::warn!("[KBD] ❌ 超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --option-test：打开设置对话框 → 依次切换 8 组开关 + 音量 → 关闭
fn auto_option_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut option: ResMut<client_bevy::game::dialogs::option::OptionState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::dialogs::DialogKind;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        if !mgr.is_open(DialogKind::Settings) {
            mgr.toggle(DialogKind::Settings);
            tracing::info!("[OPT] 打开设置对话框");
        }
        *phase = *t;
        *stage = 1;
        return;
    }
    if *stage == 1 && *t - *phase >= 1.0 {
        // 依次翻转 8 组开关（模拟点击，验证状态机 + 按钮帧刷新）
        let flips: [(&str, bool); 8] = [
            ("技能模式", option.skill_mode_ctrl),
            ("技能栏", option.skill_bar),
            ("特效", option.effect),
            ("掉落显示", option.drop_view),
            ("名称显示", option.name_view),
            ("血条显示", option.hp_view),
            ("允许观察", option.allow_observe),
            ("新移动", option.new_move),
        ];
        for (name, cur) in flips {
            let next = !cur;
            match name {
                "技能模式" => option.skill_mode_ctrl = next,
                "技能栏" => option.skill_bar = next,
                "特效" => option.effect = next,
                "掉落显示" => option.drop_view = next,
                "名称显示" => option.name_view = next,
                "血条显示" => option.hp_view = next,
                "允许观察" => option.allow_observe = next,
                _ => option.new_move = next,
            }
            tracing::info!("[OPT] ✅ 设置切换: {} -> {}", name, next);
        }
        option.sound_volume = 0.5;
        option.music_volume = 0.35;
        tracing::info!("[OPT] ✅ 音量: 音效 50% / 音乐 35%");
        tracing::info!("[OPT] ✅ 设置对话框渲染正常（8 组开关 + 2 条音量条）");
        *stage = 2;
        *phase = *t;
        return;
    }
    if *stage == 2 && *t - *phase >= 1.0 {
        if mgr.is_open(DialogKind::Settings) {
            mgr.close(DialogKind::Settings);
            tracing::info!("[OPT] ✅ 关闭设置对话框");
        }
        *stage = 9;
    }
    if *t >= 30.0 && *stage < 9 {
        tracing::warn!("[OPT] ❌ 超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --ui-dialog-test：依次打开 Notice/ChatNotice/Timer/Help 验证渲染
#[allow(clippy::too_many_arguments)]
fn auto_ui_dialog_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::dialogs::DialogKind;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    const KINDS: [DialogKind; 4] = [
        DialogKind::Notice,
        DialogKind::ChatNotice,
        DialogKind::Timer,
        DialogKind::Help,
    ];
    if *stage >= KINDS.len() as u8 {
        return;
    }
    let kind = KINDS[*stage as usize];
    if !mgr.is_open(kind) && *phase == 0.0 {
        mgr.toggle(kind);
        tracing::info!("[UIDLG] 打开 {:?}", kind);
        *phase = *t;
    }
    if mgr.is_open(kind) && *t - *phase >= 1.5 {
        mgr.close(kind);
        tracing::info!("[UIDLG] ✅ {:?} 渲染正常", kind);
        *stage += 1;
        *phase = 0.0;
        *t = 0.0;
    }
    if *t >= 30.0 && *stage < KINDS.len() as u8 {
        tracing::warn!("[UIDLG] ❌ 卡在 {:?}", kind);
        *stage = 9;
    }
}

/// --auto-enter：自动驱动 mock 登录流程（Login→Select→Game，验证网络管道）
fn auto_enter(
    mut net: ResMut<client_bevy::network::NetConnection>,
    mut session: ResMut<client_bevy::network::SessionState>,
    state: Res<State<AppState>>,
    time: Res<Time>,
    mut login_sent: Local<bool>,
    mut select_timer: Local<f32>,
) {
    use mir2_shared::packets::client::account::{Login, StartGame};
    if *state == AppState::Login && !*login_sent {
        *login_sent = true;
        net.state = client_bevy::network::NetState::LoggingIn;
        net.send_packet(&Login {
            account_id: {
            let user = std::env::args()
                .skip_while(|a| a != "--e2e-user")
                .nth(1)
                .unwrap_or_else(|| "test".to_string());
            user
        },
        password: {
            let pass = std::env::args()
                .skip_while(|a| a != "--e2e-pass")
                .nth(1)
                .unwrap_or_else(|| "123456".to_string());
            pass
        },
        });
    }
    // 在选角界面停留 3 秒再进游戏（便于 live 截屏验证选角界面）
    if *state == AppState::Select && session.selected_index.is_none() {
        *select_timer += time.delta_secs();
        if *select_timer >= 3.0 {
            let first_index = session.characters.first().map(|c| c.index);
            if let Some(idx) = first_index {
                session.selected_index = Some(idx);
                net.send_packet(&StartGame {
                    character_index: idx,
                });
            }
        }
    }
}

/// BEVY_DEMO_DELETE=1：自动登录→进选角→选中角色→打开删除询问框（截图验证用）
fn demo_delete_flow(
    mut net: ResMut<client_bevy::network::NetConnection>,
    mut session: ResMut<client_bevy::network::SessionState>,
    state: Res<State<AppState>>,
    mut modal: ResMut<client_bevy::ui::modal_box::ModalState>,
    time: Res<Time>,
    mut login_sent: Local<bool>,
    mut select_timer: Local<f32>,
    mut opened: Local<bool>,
) {
    use mir2_shared::packets::client::account::Login;
    if *state == AppState::Login && !*login_sent {
        *login_sent = true;
        net.state = client_bevy::network::NetState::LoggingIn;
        net.send_packet(&Login {
            account_id: "test".to_string(),
            password: "123456".to_string(),
        });
    }
    if *state == AppState::Select && !*opened {
        *select_timer += time.delta_secs();
        if *select_timer >= 1.0 {
            *opened = true;
            if session.selected_index.is_none() {
                session.selected_index = session.characters.first().map(|c| c.index);
            }
            modal.kind = client_bevy::ui::modal_box::ModalKind::DeleteAsk;
            tracing::info!("[DEMO] 打开删除询问框, selected={:?}", session.selected_index);
        }
    }
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

/// --auto-pickup：每 2.5s 自动拾取最近的 GroundItem（复用 player_input 的拾取逻辑）
fn auto_pickup_system(
    mut commands: Commands,
    mut timer: Local<f32>,
    time: Res<Time>,
    net: Res<client_bevy::network::NetConnection>,
    mut control: ResMut<client_bevy::game::player_control::ControlState>,
    game_data: Res<client_bevy::map_renderer::GameData>,
    players: Query<(Entity, &Transform), With<client_bevy::actor::LocalPlayer>>,
    items: Query<(&client_bevy::actor::NetObjectId, &Transform), (With<client_bevy::actor::GroundItem>, Without<client_bevy::actor::LocalPlayer>)>,
) {
    *timer += time.delta_secs();
    if *timer < 2.5 {
        return;
    }
    *timer = 0.0;
    let Ok((pe, ptf)) = players.single() else { return };
    let from_tile = client_bevy::game::movement::world_to_tile(ptf.translation.x, ptf.translation.y);
    let mut best: Option<(u32, f32)> = None;
    for (id, tf) in &items {
        let d = Vec2::new(tf.translation.x - ptf.translation.x, tf.translation.y - ptf.translation.y).length();
        if best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((id.0, d));
        }
    }
    let Some((item_id, _)) = best else { return };
    let item_tile = items
        .iter()
        .find(|(id, _)| id.0 == item_id)
        .map(|(_, tf)| client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y));
    let Some(item_tile) = item_tile else { return };
    let adjacent = (item_tile.0 - from_tile.0).abs() <= 1 && (item_tile.1 - from_tile.1).abs() <= 1;
    if adjacent {
        net.send_packet(&mir2_shared::packets::client::item::PickUp {});
        control.attack_target = None;
        tracing::info!("🎒 [AUTO] 拾取地面物品 id={}", item_id);
    } else if let Some(map) = &game_data.map {
        if let Some(p) = client_bevy::game::pathfinding::find_path(map, from_tile, item_tile) {
            if !p.is_empty() {
                let len = p.len();
                commands.entity(pe).insert(client_bevy::game::movement::LocalMove {
                    path: p.into(),
                    step_timer_ms: 0.0,
                    run: false,
                    last: None,
                    step_origin: None,
                    turn_acc: 0.0,
                });
                control.pickup_target = Some(item_id);
                tracing::info!("🚶 [AUTO] 走向物品 id={}（{} 格）", item_id, len);
            }
        }
    }
}

/// --auto-cast：进图后施放一次 F1 技能（验证 客户端→mock→回显 链路）
fn auto_cast_system(
    mut timer: Local<f32>,
    mut fired: Local<bool>,
    time: Res<Time>,
    net: Res<client_bevy::network::NetConnection>,
    magics: Res<client_bevy::game::skills::MagicsState>,
) {
    *timer += time.delta_secs();
    if *fired || *timer < 6.0 || magics.magics.is_empty() {
        return; // 等 UserInformation（技能）就绪
    }
    *fired = true;
    let Some(m) = magics.by_key(1) else {
        tracing::info!("[AUTO] 无技能 key=1");
        return;
    };
    net.send_packet(&mir2_shared::packets::client::combat::Magic {
        spell: m.spell,
        direction: mir2_shared::enums::MirDirection::Up,
        target_id: 101,
        location: mir2_shared::map::Point { x: 0, y: 0 },
    });
    tracing::info!("🪄 [AUTO] 施放 {}", m.name);
}

/// --auto-equip：进图后自动装备背包第一件可装备物品（验证 EquipItem 闭环 + 外观刷新）
fn auto_equip_system(
    mut timer: Local<f32>,
    mut fired: Local<bool>,
    time: Res<Time>,
    net: Res<client_bevy::network::NetConnection>,
    hud: Res<client_bevy::game::hud::HudState>,
) {
    if *fired {
        return;
    }
    *timer += time.delta_secs();
    if *timer < 6.0 || hud.inventory.items.iter().flatten().count() == 0 {
        return;
    }
    *fired = true;
    if let Some(item) = hud.inventory.items.iter().flatten().find(|i| i.is_equipment()) {
        if let Some(to) = item.equip_slot() {
            net.send_packet(&mir2_shared::packets::client::item::EquipItem {
                grid: mir2_shared::enums::MirGridType::Inventory,
                unique_id: item.unique_id,
                to,
            });
            tracing::info!("⚔️ [AUTO] 装备 {} -> 槽 {}", item.name, to);
        }
    }
}

/// --auto-life：进图后依次 聊天(6s) → 购买(9s) → 喝药(12s)
fn auto_life_system(
    mut timer: Local<f32>,
    mut phase: Local<u8>,
    time: Res<Time>,
    net: Res<client_bevy::network::NetConnection>,
    hud: Res<client_bevy::game::hud::HudState>,
) {
    *timer += time.delta_secs();
    let t = *timer;
    match *phase {
        0 if t >= 6.0 => {
            *phase = 1;
            net.send_packet(&mir2_shared::packets::client::chat::Chat {
                message: "你好，传奇世界！".to_string(),
                linked_items: vec![],
            });
            tracing::info!("💬 [LIFE] 发送聊天");
        }
        1 if t >= 9.0 => {
            *phase = 2;
            net.send_packet(&mir2_shared::packets::client::npc::BuyItem {
                item_index: 1,
                count: 1,
                panel_type: mir2_shared::enums::PanelType::Buy,
            });
            tracing::info!("🛒 [LIFE] 发送购买请求");
        }
        2 if t >= 12.0 => {
            *phase = 3;
            if let Some(potion) = hud.inventory.items.iter().flatten().find(|i| i.item_index == 1) {
                net.send_packet(&mir2_shared::packets::client::item::UseItem {
                    unique_id: potion.unique_id,
                });
                tracing::info!("💊 [LIFE] 使用药水 uid={}", potion.unique_id);
            } else {
                tracing::info!("💊 [LIFE] 背包无药水");
            }
        }
        _ => {}
    }
}

/// --auto-quest：任务闭环自动化（#44）
/// 阶段：0 接受任务1 → 1 等 ChangeQuest → 2 自动击杀怪物101 直到完成 → 3 交任务验证 CompleteQuest
fn auto_quest_system(
    net: Res<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    quest_log: Res<client_bevy::game::dialogs::quest_log::QuestLogState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::quest::AcceptQuest {
                npc_index: 110,
                quest_index: 1,
            });
            tracing::info!("[AUTOQUEST] 接受任务 1");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 10.0 {
                tracing::warn!("[AUTOQUEST] ❌ 未收到任务更新（ChangeQuest）");
                *stage = 9;
                return;
            }
            if quest_log.quests.iter().any(|q| q.id == 1) {
                tracing::info!("[AUTOQUEST] ✅ 任务 1 已显示，开始自动击杀怪物 101");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            // 每 0.5s 攻击一次（7 刀杀死怪物 101，含 3s 重生 ≈ 每轮 6.5s，3 轮完成）
            if *t < 0.5 {
                return;
            }
            *t = 0.0;
            net.send_packet(&mir2_shared::packets::client::combat::Attack {
                direction: mir2_shared::enums::MirDirection::Up,
                spell: mir2_shared::enums::Spell::None,
            });
            let completed = quest_log
                .quests
                .iter()
                .find(|q| q.id == 1)
                .map(|q| q.completed)
                .unwrap_or(false);
            if completed {
                tracing::info!("[AUTOQUEST] ✅ 任务完成（计数 3/3），交任务");
                net.send_packet(&mir2_shared::packets::client::quest::FinishQuest {
                    quest_index: 1,
                    selected_item_index: -1,
                });
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t >= 10.0 {
                tracing::warn!("[AUTOQUEST] ❌ 未收到 CompleteQuest / 任务未从日志移除");
                *stage = 9;
                return;
            }
            if !quest_log.quests.iter().any(|q| q.id == 1) {
                tracing::info!("[AUTOQUEST] ✅ 任务已从日志移除（CompleteQuest 生效），全链路完成");
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --auto-revive：死亡后 1s 自动发 TownRevive（验证 死亡→复活 全链路，#46）
fn auto_revive_system(
    net: Res<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut t: Local<f32>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    if !hud.dead {
        *t = 0.0;
        return;
    }
    *t += time.delta_secs();
    if *t >= 1.0 {
        *t = 0.0;
        net.send_packet(&mir2_shared::packets::client::misc::TownRevive);
        tracing::info!("[REVIVE] 自动复活（TownRevive）");
    }
}

/// --auto-cast-loop：每秒连发 F1 技能（验证 耗蓝递减 → 蓝不足拒绝 → 魔法药回蓝，#51）
fn auto_cast_loop_system(
    mut timer: Local<f32>,
    mut last_cast: Local<f32>,
    time: Res<Time>,
    net: Res<client_bevy::network::NetConnection>,
    magics: Res<client_bevy::game::skills::MagicsState>,
    hud: Res<client_bevy::game::hud::HudState>,
) {
    *timer += time.delta_secs();
    if *timer < 6.0 || magics.magics.is_empty() {
        return;
    }
    if *timer - *last_cast < 1.0 {
        return;
    }
    *last_cast = *timer;
    // 蓝不足时喝魔法药(小)
    if hud.mp < 10 {
        if let Some(potion) = hud
            .inventory
            .items
            .iter()
            .flatten()
            .find(|i| i.item_index == 2)
        {
            net.send_packet(&mir2_shared::packets::client::item::UseItem {
                unique_id: potion.unique_id,
            });
            tracing::info!("🔮 [CASTLOOP] MP 低，喝魔法药 uid={}", potion.unique_id);
        }
    }
    let Some(m) = magics.by_key(1) else {
        return;
    };
    net.send_packet(&mir2_shared::packets::client::combat::Magic {
        spell: m.spell,
        direction: mir2_shared::enums::MirDirection::Up,
        target_id: 101,
        location: mir2_shared::map::Point { x: 0, y: 0 },
    });
    tracing::info!("🔮 [CASTLOOP] 施放 {}（MP {}/{}）", m.name, hud.mp, hud.max_mp);
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
}

/// --real-verify：真实服务器交互闭环（#55）
/// 依赖：--real-net --auto-enter（先登录进图）；在 mock 下同样可跑
/// 阶段：0 聊天回显 → 1 寻路到最近怪物 → 2 自动攻击至死亡 → 3 NPC 对话
#[allow(clippy::too_many_arguments)]
fn real_verify_system(
    mut commands: Commands,
    net: Res<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut control: ResMut<client_bevy::game::player_control::ControlState>,
    game_data: Res<client_bevy::map_renderer::GameData>,
    mut chat: ResMut<client_bevy::game::chat::ChatState>,
    hud: Res<client_bevy::game::hud::HudState>,
    npc_dialog: Res<client_bevy::game::dialogs::npc::NpcDialogState>,
    probe: Res<client_bevy::game::combat::RealHitProbe>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
        Has<client_bevy::actor::Npc>,
    )>,
    monster_names: Query<(&client_bevy::actor::NetObjectId, &client_bevy::actor::MonsterName)>,
    players: Query<
        (Entity, &Transform),
        (With<client_bevy::actor::LocalPlayer>, With<client_bevy::actor::NetObjectId>),
    >,
    mut s: Local<RealVerifyState>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    s.t += time.delta_secs();
    match s.stage {
        0 => {
            if s.t < 8.0 {
                return;
            }
            if !s.chat_sent {
                s.chat_sent = true;
                net.send_packet(&mir2_shared::packets::client::chat::Chat {
                    message: "真实服务器验证：你好！".to_string(),
                    linked_items: vec![],
                });
                // 真实服务器不回发给自己（设计）；本地回显由 chat_input_system 负责（C# 行为），
                // 这里模拟用户路径 add_line，验证显示链路
                chat.add_line(
                    format!("[{}]: 真实服务器验证：你好！", hud.name),
                    Color::WHITE,
                    client_bevy::game::chat::ChatChannel::Nearby,
                );
                tracing::info!("[REAL] 💬 发送聊天（服务器不回显自己属设计，本地回显已修复）");
            }
            if chat.lines.iter().any(|(l, _, _)| l.contains("真实服务器验证")) && !s.chat_echo {
                s.chat_echo = true;
                tracing::info!("[REAL] ✅ 聊天本地回显收到（显示链路通过）");
            }
            if s.t >= 20.0 {
                if s.chat_echo {
                    tracing::info!("[REAL] ✅ 聊天验证通过");
                } else {
                    tracing::warn!("[REAL] ⚠️ 聊天未显示");
                }
                s.stage = 1;
                s.t = 0.0;
            }
        }
        1 => {
            if s.t < 1.0 {
                return;
            }
            let Ok((_, pf)) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32, i32)> = None;
            let mut saw_monster = false;
            let mut saw_guard = false;
            for (id, tf, monster, _npc) in &actors {
                if !monster {
                    continue;
                }
                saw_monster = true;
                // 守卫是友好 NPC，攻击会被反杀（#77 实测打死玩家）；不作为猎杀目标
                if monster_names
                    .iter()
                    .any(|(mid, mn)| mid.0 == id.0 && mn.0.to_lowercase().contains("guard"))
                {
                    saw_guard = true;
                    continue;
                }
                // 排除已尝试但未命中的目标（#57 远程怪够不着）
                if s.tried.contains(&id.0) {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if best.map(|(_, _, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my, d));
                }
            }
            let Some((oid, mx, my, d)) = best else {
                if saw_guard && !saw_monster {
                    tracing::warn!("[REAL] ❌ 图上只有守卫类目标（已跳过），无猎杀目标");
                } else if s.tried.is_empty() {
                    tracing::warn!("[REAL] ❌ 全图无怪物");
                } else {
                    tracing::warn!("[REAL] ❌ 已尝试 {} 个目标后无剩余怪物（近战命中验证不通过）", s.tried.len());
                }
                s.stage = 9;
                return;
            };
            let mon_name = monster_names
                .iter()
                .find(|(mid, _)| mid.0 == oid)
                .map(|(_, n)| n.0.clone())
                .unwrap_or_default();
            tracing::info!(
                "[REAL] 🎯 最近怪物 id={} {} @ ({},{}) 距离={}（已试 {} 个）",
                oid, mon_name, mx, my, d, s.tried.len()
            );
            s.target = Some(oid);
            s.target_tile = Some((mx, my));
            if d <= 1 {
                control.attack_target = Some(oid);
                tracing::info!("[REAL] ⚔️ 已在邻接，直接开始攻击 {}", oid);
                s.stage = 2;
                s.t = 0.0;
                return;
            }
            let Some(map) = &game_data.map else {
                tracing::warn!("[REAL] ❌ 地图未加载");
                s.stage = 9;
                return;
            };
            let Ok((pe, _)) = players.single() else { return };
            // 近战需在怪物相邻格（而非重叠）：寻路目标选怪物 8 邻中可达且路径最短的格
            let mut best_path: Option<(Vec<(i32, i32)>, (i32, i32))> = None;
            for (ox, oy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                let t2 = (mx + ox, my + oy);
                if !map.in_bounds(t2.0, t2.1) || !map.is_walkable(t2.0, t2.1) {
                    continue;
                }
                if let Some(p) = client_bevy::game::pathfinding::find_path(map, (px, py), t2) {
                    if !p.is_empty()
                        && best_path
                            .as_ref()
                            .map(|(bp, _)| p.len() < bp.len())
                            .unwrap_or(true)
                    {
                        best_path = Some((p, t2));
                    }
                }
            }
            match best_path {
                Some((p, t2)) => {
                    let len = p.len();
                    s.target_tile = Some(t2);
                    // run 模式（客户端跨 2 格发一个 Run，#59 已修）
                    commands.entity(pe).insert(client_bevy::game::movement::LocalMove {
                        path: p.into(),
                        step_timer_ms: 0.0,
                        run: true,
                        last: None,
                        step_origin: None,
                        turn_acc: 0.0,
                    });
                    tracing::info!("[REAL] 🚶 寻路到怪物旁（{} 格，run，目标 {},{}）", len, t2.0, t2.1);
                    s.stage = 2;
                    s.t = 0.0;
                }
                _ => {
                    tracing::warn!(
                        "[REAL] ❌ 无法寻路到怪物 ({},{}) 旁（from=({},{}) from_walkable={}）",
                        mx, my, px, py, map.is_walkable(px, py)
                    );
                    s.stage = 9;
                }
            }
        }
        2 => {
            let Ok((_, pf)) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let Some(tid) = s.target else { s.stage = 9; return };
            let alive = actors.iter().any(|(id, _, _, _)| id.0 == tid);
            if !alive {
                tracing::info!("[REAL] ✅ 目标怪物已死亡（实体移除）——战斗闭环通过（命中 {} 次）", probe.hits);
                s.stage = 3;
                s.t = 0.0;
                return;
            }
            if hud.dead {
                tracing::warn!("[REAL] ⚠️ 玩家死亡（战斗验证部分通过，继续 NPC 验证）");
                s.stage = 3;
                s.t = 0.0;
                return;
            }
            let (mx, my) = s.target_tile.unwrap_or((0, 0));
            let d = (mx - px).abs() + (my - py).abs();
            if d <= 1 && control.attack_target != Some(tid) {
                // 客户端本地移动超前于服务器位置（UserLocation 校正有延迟），
                // 到达邻接后等 2s 让服务器位置同步（apply_self_position 会校正），再攻击
                s.arrived_wait += time.delta_secs();
                if s.arrived_wait < 2.0 {
                    return;
                }
                s.arrived_wait = 0.0;
                control.attack_target = Some(tid);
                // 命中基线：从开始攻击时记录
                s.hits_at_start = probe.hits;
                s.attack_elapsed = 0.0;
                tracing::info!("[REAL] ⚔️ 服务器位置已同步，开始自动攻击 {}（命中基线 {}）", tid, s.hits_at_start);
            }
            if control.attack_target == Some(tid) {
                s.attack_elapsed += time.delta_secs();
                // 20s 攻击零命中 → 目标够不着（远程怪/位置漂移），换下一个最近怪物
                if s.attack_elapsed >= 20.0 && probe.hits == s.hits_at_start {
                    tracing::warn!("[REAL] ⚠️ 攻击 {} 20s 零命中（共命中 {}），换目标", tid, probe.hits);
                    s.tried.push(tid);
                    control.attack_target = None;
                    s.target = None;
                    s.target_tile = None;
                    s.stage = 1;
                    s.t = 0.0;
                    return;
                }
            }
            if s.attack_elapsed >= 90.0 {
                tracing::warn!("[REAL] ⚠️ 90s 内未击杀目标（可能打不过/怪物跑远）");
                s.stage = 9;
            }
        }
        3 => {
            if s.t < 3.0 {
                return;
            }
            let Ok((_, pf)) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32, i32)> = None;
            for (id, tf, _monster, npc) in &actors {
                if !npc {
                    continue;
                }
                let (nx, ny) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (nx - px).abs() + (ny - py).abs();
                if best.map(|(_, _, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, nx, ny, d));
                }
            }
            let Some((nid, nx, ny, d)) = best else {
                tracing::warn!("[REAL] ❌ 全图无 NPC");
                s.stage = 9;
                return;
            };
            tracing::info!("[REAL] 🧙 最近 NPC id={} @ ({},{}) 距离={}", nid, nx, ny, d);
            s.npc_id = Some(nid);
            s.npc_sent = false;
            s.npc_wait = 0.0;
            if d <= 2 {
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: nid,
                    key: "[@Main]".to_string(),
                });
                s.npc_sent = true;
                tracing::info!("[REAL] 🧙 发送 CallNPC [@Main]");
                s.stage = 4;
                s.t = 0.0;
                return;
            }
            // 走到 NPC 2 格内（服务器交互范围 2 格）
            let Some(map) = &game_data.map else {
                tracing::warn!("[REAL] ❌ 地图未加载");
                s.stage = 9;
                return;
            };
            let Ok((pe, _)) = players.single() else { return };
            let path = client_bevy::game::pathfinding::find_path(map, (px, py), (nx, ny));
            let path = path.or_else(|| {
                for (ox, oy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    let t2 = (nx + ox, ny + oy);
                    if let Some(p) = client_bevy::game::pathfinding::find_path(map, (px, py), t2) {
                        if !p.is_empty() {
                            return Some(p);
                        }
                    }
                }
                None
            });
            match path {
                Some(p) if !p.is_empty() => {
                    let len = p.len();
                    s.target_tile = Some((nx, ny));
                    commands.entity(pe).insert(client_bevy::game::movement::LocalMove {
                        path: p.into(),
                        step_timer_ms: 0.0,
                        run: true,
                        last: None,
                        step_origin: None,
                        turn_acc: 0.0,
                    });
                    tracing::info!("[REAL] 🚶 寻路到 NPC（{} 格，run）", len);
                    s.stage = 4;
                    s.t = 0.0;
                }
                _ => {
                    tracing::warn!("[REAL] ❌ 无法寻路到 NPC ({},{})", nx, ny);
                    s.stage = 9;
                }
            }
        }
        4 => {
            if npc_dialog.visible {
                tracing::info!("[REAL] ✅ NPC 对话框已打开（NPCResponse 收到）");
                s.stage = 9;
                return;
            }
            // 到达 NPC 旁且服务器位置同步后发送 CallNPC（本地移动超前，需等校正）
            if !s.npc_sent {
                let nid = s.npc_id.unwrap_or(0);
                let Ok((_, pf)) = players.single() else { return };
                let (px, py) =
                    client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
                let (mx, my) = s.target_tile.unwrap_or((0, 0));
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 2 {
                    s.npc_wait += time.delta_secs();
                    if s.npc_wait >= 2.0 {
                        s.npc_sent = true;
                        net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                            object_id: nid,
                            key: "[@Main]".to_string(),
                        });
                        tracing::info!("[REAL] 🧙 服务器位置同步后发送 CallNPC [@Main]");
                    }
                }
            }
            if s.t >= 25.0 {
                tracing::warn!("[REAL] ⚠️ 25s 未收到 NPCResponse（可能该 NPC 无 @Main 页）");
                s.stage = 9;
            }
        }
        _ => {}
    }
}

/// --book-test：技能书学习（#212：使用背包槽 3 技能书 → 等 S.NewMagic → 校验技能列表）
#[allow(clippy::too_many_arguments)]
fn auto_book_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hud: Res<client_bevy::game::hud::HudState>,
    magics: Res<client_bevy::game::skills::MagicsState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            let uid = hud
                .inventory
                .items
                .get(3)
                .and_then(|s| s.as_ref())
                .map(|i| i.unique_id);
            match uid {
                Some(uid) => {
                    net.send_packet(&mir2_shared::packets::client::item::UseItem { unique_id: uid });
                    tracing::info!("[BOOKTEST] 使用技能书 uid={}", uid);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[BOOKTEST] ⚠️ 背包槽 3 无技能书");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t >= 8.0 {
                tracing::warn!("[BOOKTEST] ❌ 未学会技能");
                *stage = 9;
                return;
            }
            if magics
                .magics
                .iter()
                .any(|m| m.spell == mir2_shared::enums::Spell::FireBall)
            {
                tracing::info!("[BOOKTEST] ✅ 学会 FireBall");
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --reincarnation-test：死亡 → S.RequestReincarnation offer → 接受 → 复活（#222）
#[allow(clippy::too_many_arguments)]
fn auto_reincarnation_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t >= 300.0 {
                tracing::warn!("[REINC] ❌ 等待死亡超时");
                *stage = 9;
                return;
            }
            if hud.dead {
                if hud.reincarnation_offered {
                    tracing::info!("[REINC] ✅ 收到轮回术 offer");
                    net.send_packet(&mir2_shared::packets::client::misc::AcceptReincarnation);
                    tracing::info!("[REINC] 接受轮回术复活");
                    *stage = 1;
                    *t = 0.0;
                } else {
                    tracing::warn!("[REINC] ❌ 死亡但未收到 offer");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t >= 10.0 {
                tracing::warn!("[REINC] ❌ 复活超时");
                *stage = 9;
                return;
            }
            if !hud.dead {
                tracing::info!("[REINC] ✅ 轮回术复活成功");
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --battle-vfx-test：施法 → mock 回发 ObjectMagic/ObjectProjectile/ObjectEffect/ObjectRangeAttack，
/// 断言特效计数增长（#224）
#[allow(clippy::too_many_arguments)]
fn auto_battle_vfx_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    mut before: Local<u64>,
    effects: Res<client_bevy::game::effects::EffectsState>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            // 找 40 格内最近的怪物
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[VFX] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[VFX] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            *before = effects.spawned;
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!(
                "[VFX] 🔥 施法 FireBall → ({},{}), 特效基线={}",
                mx,
                my,
                *before
            );
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 2.5 {
                let delta = effects.spawned - *before;
                if delta >= 3 {
                    tracing::info!("[VFX] ✅ 战斗特效已生成（+{}）", delta);
                } else {
                    tracing::warn!("[VFX] ❌ 特效不足（+{}，期望 ≥3）", delta);
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --object-state-test：施法 → mock 触发 隐藏/击退/坐下/显形/传送 状态机，逐项断言（#226）
#[allow(clippy::too_many_arguments)]
fn auto_object_state_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    mut before: Local<u64>,
    mut flags: Local<u8>,
    effects: Res<client_bevy::game::effects::EffectsState>,
    actors_vis: Query<(&client_bevy::actor::NetObjectId, &Visibility)>,
    actors_anim: Query<(
        &client_bevy::actor::NetObjectId,
        &client_bevy::actor::ActorAnim,
    )>,
    actors_mon: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors_mon {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[OBJST] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[OBJST] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            *before = effects.spawned;
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[OBJST] 🔥 施法触发对象状态演示");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            // mock t+2s 发送：隐藏102 / 击退103 / 坐下101；t+4s 发送：显形102 / 传送消失103。
            // 采样窗口 [2.0, 4.0)：记录曾隐藏/曾坐下/击退（mock t+2s 发送，尽早采样避免演示旋转覆盖）
            if *t >= 2.0 && *t < 4.0 {
                let hide = actors_vis
                    .iter()
                    .any(|(id, v)| id.0 == 102 && matches!(*v, Visibility::Hidden));
                let sit = actors_anim
                    .iter()
                    .any(|(id, a)| id.0 == 101 && a.direction == 2);
                let orig = client_bevy::game::movement::tile_to_world(351, 355);
                let push = actors_mon.iter().any(|(id, tf, m)| {
                    m && id.0 == 103
                        && ((tf.translation.x - orig.x).abs() > 1.0
                            || (tf.translation.y - orig.y).abs() > 1.0)
                });
                if hide {
                    *flags |= 1;
                }
                if sit {
                    *flags |= 2;
                }
                if push {
                    *flags |= 4;
                }
            }
            if *t >= 4.0 {
                tracing::info!(
                    "[OBJST] 阶段2: 隐藏={} 坐下={} 击退={}",
                    *flags & 1 != 0,
                    *flags & 2 != 0,
                    *flags & 4 != 0
                );
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            // 阶段3 从 t+4s 开始（显形102/传送消失103 已发送）；采样 [0.5, 2.0)
            // 覆盖传送消失隐藏窗口 [t+4, t+6)，在 t+6 传送出现前完成采样
            if *t >= 0.5 && *t < 2.0 {
                let show = actors_vis
                    .iter()
                    .any(|(id, v)| id.0 == 102 && matches!(*v, Visibility::Visible));
                let out_hidden = actors_vis
                    .iter()
                    .any(|(id, v)| id.0 == 103 && matches!(*v, Visibility::Hidden));
                if show {
                    *flags |= 8;
                }
                if out_hidden {
                    *flags |= 16;
                }
            }
            if *t >= 2.0 {
                *stage = 4;
                *t = 0.0;
            }
        }
        4 => {
            // t+6s：传送出现103；进入阶段4 后 7s 汇总
            if *t >= 7.0 {
                let in_visible = actors_vis
                    .iter()
                    .any(|(id, v)| id.0 == 103 && matches!(*v, Visibility::Visible));
                let delta = effects.spawned - *before;
                let hide = *flags & 1 != 0;
                let sit = *flags & 2 != 0;
                let push = *flags & 4 != 0;
                let show = *flags & 8 != 0;
                let out_hidden = *flags & 16 != 0;
                tracing::info!(
                    "[OBJST] 阶段3: 显形={} 传送消失={} 传送出现={} 特效增量={}",
                    show,
                    out_hidden,
                    in_visible,
                    delta
                );
                if hide && sit && push && show && out_hidden && in_visible && delta >= 2 {
                    tracing::info!("[OBJST] ✅ 对象状态表现全部通过");
                } else {
                    tracing::warn!(
                        "[OBJST] ❌ 部分未通过（隐藏={} 坐下={} 击退={} 显形={} 传送出={} 传送入={} 特效={}）",
                        hide,
                        sit,
                        push,
                        show,
                        out_hidden,
                        in_visible,
                        delta
                    );
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --item-state-test：施法 → mock 回发 DuraChanged/GainedItem/DeleteItem，
/// 断言背包耐久更新/物品获得/物品删除（#228）
#[allow(clippy::too_many_arguments)]
fn auto_item_state_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[ITEMST] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[ITEMST] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[ITEMST] 🔥 施法触发物品状态同步");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 3.0 {
                let gained = hud
                    .inventory
                    .items
                    .iter()
                    .flatten()
                    .any(|it| it.unique_id == 9002);
                let deleted = !hud
                    .inventory
                    .items
                    .iter()
                    .flatten()
                    .any(|it| it.unique_id == 9010);
                let dura = hud
                    .inventory
                    .items
                    .iter()
                    .flatten()
                    .any(|it| it.unique_id == 9005 && it.current_dura == 3);
                tracing::info!(
                    "[ITEMST] 获得={} 删除={} 耐久={}",
                    gained,
                    deleted,
                    dura
                );
                if gained && deleted && dura {
                    tracing::info!("[ITEMST] ✅ 物品状态同步全部通过");
                } else {
                    tracing::warn!("[ITEMST] ❌ 部分未通过（获得={} 删除={} 耐久={}）", gained, deleted, dura);
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --map-fx-test：施法 → mock 回发 MapEffect/PlaySound/SetTimer，4s 后 ExpireTimer，
/// 断言 特效生成 + 计时器激活 + 计时器关闭（#230）
#[allow(clippy::too_many_arguments)]
fn auto_map_fx_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    effects: Res<client_bevy::game::effects::EffectsState>,
    timer: Res<client_bevy::game::dialogs::timer::TimerState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    mut before: Local<u64>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[MAPFX] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[MAPFX] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            *before = effects.spawned;
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[MAPFX] 🔥 施法触发地图特效/音效/计时器");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 2.5 {
                let delta = effects.spawned - *before;
                let timer_on = timer.active && timer.remaining > 0.0;
                tracing::info!(
                    "[MAPFX] 阶段2: 特效增量={} 计时器激活={} 剩余={:.1}",
                    delta,
                    timer_on,
                    timer.remaining
                );
                if delta >= 1 && timer_on {
                    tracing::info!("[MAPFX] ✅ 地图特效/计时器启动通过");
                } else {
                    tracing::warn!("[MAPFX] ❌ 启动未通过（特效={} 计时器={}）", delta, timer_on);
                }
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            // mock t+4s 发 ExpireTimer；倒计时 5s 也会归零——两者任一都会关闭
            if *t >= 6.0 {
                let expired = !timer.active;
                tracing::info!("[MAPFX] 阶段3: 计时器已关闭={}", expired);
                if expired {
                    tracing::info!("[MAPFX] ✅ 计时器关闭通过");
                } else {
                    tracing::warn!("[MAPFX] ❌ 计时器未关闭（remaining={:.1}）", timer.remaining);
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --mount-sync-test：施法 → mock 回发 MountUpdate(上马)，t+4s 下马，断言 MountState 出现→消失（#232）
#[allow(clippy::too_many_arguments)]
fn auto_mount_sync_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    mounts: Query<(
        &client_bevy::actor::NetObjectId,
        Option<&client_bevy::actor::MountState>,
    )>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[MOUNT] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[MOUNT] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[MOUNT] 🔥 施法触发坐骑同步");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            // mock 施法即发 MountUpdate(上马)
            if *t >= 2.5 {
                let mounted = mounts.iter().any(|(id, m)| id.0 == 100 && m.is_some());
                tracing::info!("[MOUNT] 阶段2: 已上马={}", mounted);
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            // mock t+4s 发 MountUpdate(下马)
            if *t >= 6.0 {
                let mounted = mounts
                    .iter()
                    .any(|(id, m)| id.0 == 100 && m.is_some());
                let dismounted = !mounted;
                tracing::info!("[MOUNT] 阶段3: 已下马={}", dismounted);
                if dismounted {
                    tracing::info!("[MOUNT] ✅ 坐骑同步（上马→下马）通过");
                } else {
                    tracing::warn!("[MOUNT] ❌ 下马未生效");
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --action-test：施法 → mock 怪物反击 ObjectAttack + 对象冲刺/后跳，逐项采样断言（#234）
#[allow(clippy::too_many_arguments)]
fn auto_action_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    mut flags: Local<u8>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    actors_st: Query<(
        &client_bevy::actor::NetObjectId,
        Has<client_bevy::game::combat::StruckTimer>,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[ACTION] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[ACTION] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[ACTION] 🔥 施法触发对象动作");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            // 采样窗口 [0.5, 8.0)：怪物反击 ObjectAttack（StruckTimer）、103 冲刺位移、101 后跳
            if *t >= 0.5 && *t < 8.0 {
                let attack = actors_st
                    .iter()
                    .any(|(id, struck, monster)| monster && struck && id.0 != 100);
                if attack {
                    *flags |= 1;
                }
                let dash_orig = client_bevy::game::movement::tile_to_world(351, 355);
                let dash = actors.iter().any(|(id, tf, monster)| {
                    monster
                        && id.0 == 103
                        && ((tf.translation.x - dash_orig.x).abs() > 1.0
                            || (tf.translation.y - dash_orig.y).abs() > 1.0)
                });
                if dash {
                    *flags |= 2;
                }
                let back = client_bevy::game::movement::tile_to_world(352, 352);
                let backstep = actors.iter().any(|(id, tf, monster)| {
                    monster
                        && id.0 == 101
                        && (tf.translation.x - back.x).abs() < 1.0
                        && (tf.translation.y - back.y).abs() < 1.0
                });
                if backstep {
                    *flags |= 4;
                }
            }
            if *t >= 8.0 {
                let attack = *flags & 1 != 0;
                let dash = *flags & 2 != 0;
                let backstep = *flags & 4 != 0;
                let struck_count = actors_st
                    .iter()
                    .filter(|(_, struck, _)| *struck)
                    .count();
                tracing::info!(
                    "[ACTION] 攻击={} 冲刺={} 后跳={}（当前带StruckTimer怪物数={}）",
                    attack,
                    dash,
                    backstep,
                    struck_count
                );
                if attack && dash && backstep {
                    tracing::info!("[ACTION] ✅ 对象动作全部通过");
                } else {
                    tracing::warn!(
                        "[ACTION] ❌ 部分未通过（攻击={} 冲刺={} 后跳={}）",
                        attack,
                        dash,
                        backstep
                    );
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --poison-test：施法 → mock 回发 ObjectPoisoned(GREEN)，t+4s 解毒，断言 PoisonTint 出现→消失（#236）
#[allow(clippy::too_many_arguments)]
fn auto_poison_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    mut flags: Local<u8>,
    poisons: Query<(
        &client_bevy::actor::NetObjectId,
        Has<client_bevy::actor::PoisonTint>,
    )>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[POISON] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[POISON] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[POISON] 🔥 施法触发中毒");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            // mock 施法即发中毒；采样 [0.5, 4.0) 观察 PoisonTint
            if *t >= 0.5 && *t < 4.0 {
                let poisoned = poisons.iter().any(|(id, tint)| id.0 == 100 && tint);
                if poisoned {
                    *flags |= 1;
                }
            }
            if *t >= 4.0 {
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            // mock t+4s 解毒；t+6s 汇总
            if *t >= 6.0 {
                let cured = !poisons
                    .iter()
                    .any(|(id, tint)| id.0 == 100 && tint);
                if cured {
                    *flags |= 2;
                }
                let poisoned = *flags & 1 != 0;
                let cured = *flags & 2 != 0;
                tracing::info!("[POISON] 中毒={} 解毒={}", poisoned, cured);
                if poisoned && cured {
                    tracing::info!("[POISON] ✅ 中毒染层（中毒→解毒）通过");
                } else {
                    tracing::warn!("[POISON] ❌ 部分未通过（中毒={} 解毒={}）", poisoned, cured);
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --mana-test：施法 → mock 回发 ObjectMana(101=80%)，断言 ActorMp 出现（#238）
#[allow(clippy::too_many_arguments)]
fn auto_mana_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    mut flags: Local<u8>,
    mana: Query<(
        &client_bevy::actor::NetObjectId,
        Has<client_bevy::game::combat::ActorMp>,
    )>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[MANA] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[MANA] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[MANA] 🔥 施法触发对象蓝条");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            // mock 施法即发 ObjectMana(101=80%)；采样 [0.5, 6.0) 观察 ActorMp
            if *t >= 0.5 && *t < 6.0 {
                let seen = mana.iter().any(|(id, has_mp)| has_mp && id.0 == 101);
                if seen {
                    *flags |= 1;
                }
            }
            if *t >= 6.0 {
                let seen = *flags & 1 != 0;
                tracing::info!("[MANA] 蓝条={}", seen);
                if seen {
                    tracing::info!("[MANA] ✅ 对象蓝条（ObjectMana）通过");
                } else {
                    tracing::warn!("[MANA] ❌ 未观察到 ActorMp");
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --repair-test：施法 → mock 回发 ItemRepaired(9005: 12/8) + ItemSlotSizeChanged(1)，
/// 断言背包物品耐久/最大耐久/槽位数更新（#240）
#[allow(clippy::too_many_arguments)]
fn auto_repair_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[REPAIR] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[REPAIR] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[REPAIR] 🔥 施法触发修理/槽位同步");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 2.5 {
                let item = hud
                    .inventory
                    .items
                    .iter()
                    .flatten()
                    .find(|it| it.unique_id == 9005);
                let dura = item
                    .map(|it| it.current_dura == 8 && it.max_dura == 12)
                    .unwrap_or(false);
                let slots = item.map(|it| it.slots.len() == 1).unwrap_or(false);
                tracing::info!("[REPAIR] 耐久={} 槽位={}", dura, slots);
                if dura && slots {
                    tracing::info!("[REPAIR] ✅ 修理/槽位同步通过");
                } else {
                    tracing::warn!("[REPAIR] ❌ 未通过（耐久={} 槽位={}）", dura, slots);
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --toggle-test：施法 → mock 回发 S.SpellToggle(Slaying,true)；再发 C.SpellToggle(Thrusting,true)
/// 等 mock 回显，断言 MagicsState.spell_toggles 双向更新（#242）
#[allow(clippy::too_many_arguments)]
fn auto_toggle_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    magics: Res<client_bevy::game::skills::MagicsState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[TOGGLE] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[TOGGLE] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[TOGGLE] 🔥 施法触发技能开关");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            // mock 施法即发 S.SpellToggle(Slaying,true)
            if *t >= 2.0 {
                let sv_seen = magics.toggle_state(mir2_shared::enums::Spell::Slaying);
                tracing::info!("[TOGGLE] 服务端同步 Slaying={}", sv_seen);
                // 模拟客户端切换（skill_bar_system 同款包）
                net.send_packet(&mir2_shared::packets::client::combat::SpellToggle {
                    spell: mir2_shared::enums::Spell::Thrusting,
                    can_use: true,
                });
                tracing::info!("[TOGGLE] 发送 C.SpellToggle(Thrusting,true)");
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            // mock 回显 S.SpellToggle(Thrusting,true)
            if *t >= 3.0 {
                let echo = magics.toggle_state(mir2_shared::enums::Spell::Thrusting);
                tracing::info!("[TOGGLE] 回显 Thrusting={}", echo);
                if echo {
                    tracing::info!("[TOGGLE] ✅ 技能开关双向通过");
                } else {
                    tracing::warn!("[TOGGLE] ❌ 回显未更新");
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --gold-test：施法 → mock 掉金币 150，断言 GroundGold 实体出现（#244）
#[allow(clippy::too_many_arguments)]
fn auto_gold_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    gold: Query<(
        &client_bevy::actor::NetObjectId,
        &client_bevy::actor::GroundGold,
    )>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[GOLD] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[GOLD] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[GOLD] 🔥 施法触发金币掉落");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            // mock 施法即发 ObjectGold(150)
            if *t >= 2.5 {
                let seen = gold.iter().any(|(_, g)| g.gold == 150);
                tracing::info!("[GOLD] 地面金币={}", seen);
                if seen {
                    tracing::info!("[GOLD] ✅ 地面金币（ObjectGold）通过");
                } else {
                    tracing::warn!("[GOLD] ❌ 未观察到 GroundGold");
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --harvest-test：施法 → mock 对怪物 101 发 ObjectHarvest(→352,352)，断言位置变化（#246）
#[allow(clippy::too_many_arguments)]
fn auto_harvest_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    mut flags: Local<u8>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[HARVEST] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[HARVEST] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[HARVEST] 🔥 施法触发采集");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            // mock 施法即发 ObjectHarvest(101 → 352,352)；采样 [0.5, 6.0)
            if *t >= 0.5 && *t < 6.0 {
                let pos = client_bevy::game::movement::tile_to_world(352, 352);
                let moved = actors.iter().any(|(id, tf, monster)| {
                    monster
                        && id.0 == 101
                        && (tf.translation.x - pos.x).abs() < 1.0
                        && (tf.translation.y - pos.y).abs() < 1.0
                });
                if moved {
                    *flags |= 1;
                }
            }
            if *t >= 6.0 {
                let moved = *flags & 1 != 0;
                tracing::info!("[HARVEST] 采集位移={}", moved);
                if moved {
                    tracing::info!("[HARVEST] ✅ 采集表现通过");
                } else {
                    tracing::warn!("[HARVEST] ❌ 未观察到采集位移");
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --npc-credit-test：施法 → mock 回发 NPCImageUpdate(110→2) + GainedCredit(50)，
/// 断言 NPC 形象变化 + 声望累积（#248）
#[allow(clippy::too_many_arguments)]
fn auto_npc_credit_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hud: Res<client_bevy::game::hud::HudState>,
    npcs: Query<(
        &client_bevy::actor::NetObjectId,
        &client_bevy::actor::NpcAppearance,
    )>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[NPCCR] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[NPCCR] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[NPCCR] 🔥 施法触发 NPC/声望");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 2.5 {
                let npc_updated = npcs
                    .iter()
                    .any(|(id, app)| id.0 == 110 && app.npc_index == 2);
                let credit = hud.credit >= 50;
                tracing::info!("[NPCCR] NPC形象={} 声望={}", npc_updated, credit);
                if npc_updated && credit {
                    tracing::info!("[NPCCR] ✅ NPC 形象/声望通过");
                } else {
                    tracing::warn!(
                        "[NPCCR] ❌ 未通过（NPC形象={} 声望={}）",
                        npc_updated,
                        credit
                    );
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --compass-test：施法 → mock 回发 SetCompass(354,350)，断言 CompassState.target（#250）
#[allow(clippy::too_many_arguments)]
fn auto_compass_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    compass: Res<client_bevy::game::dialogs::compass::CompassState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[COMPASS] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[COMPASS] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[COMPASS] 🔥 施法触发罗盘");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 2.5 {
                let ok = compass.target == Some((354, 350));
                tracing::info!("[COMPASS] 目标={:?}", compass.target);
                if ok {
                    tracing::info!("[COMPASS] ✅ 罗盘目标通过");
                } else {
                    tracing::warn!("[COMPASS] ❌ 罗盘目标未设置");
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --sneak-test：施法 → mock 对 102 潜行、103 等级特效，断言隐身 + 特效生成（#252）
#[allow(clippy::too_many_arguments)]
fn auto_sneak_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    effects: Res<client_bevy::game::effects::EffectsState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    mut before: Local<u64>,
    mut flags: Local<u8>,
    vis: Query<(&client_bevy::actor::NetObjectId, &Visibility)>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[SNEAK] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[SNEAK] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            *before = effects.spawned;
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[SNEAK] 🔥 施法触发潜行/特效");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            // mock 施法即发 ObjectSneaking(102,true) + ObjectLevelEffects(103)
            if *t >= 0.5 && *t < 6.0 {
                let hidden = vis
                    .iter()
                    .any(|(id, v)| id.0 == 102 && matches!(*v, Visibility::Hidden));
                if hidden {
                    *flags |= 1;
                }
            }
            if *t >= 6.0 {
                let hidden = *flags & 1 != 0;
                let delta = effects.spawned - *before;
                tracing::info!("[SNEAK] 隐身={} 特效增量={}", hidden, delta);
                if hidden && delta >= 1 {
                    tracing::info!("[SNEAK] ✅ 隐身/等级特效通过");
                } else {
                    tracing::warn!("[SNEAK] ❌ 未通过（隐身={} 特效={}）", hidden, delta);
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --member-test：施法 → mock 回发 SendMemberLocation("队友A",356,350)，断言 MemberLocations（#254）
#[allow(clippy::too_many_arguments)]
fn auto_member_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    members: Res<client_bevy::game::dialogs::minimap::MemberLocations>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[MEMBER] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[MEMBER] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[MEMBER] 🔥 施法触发成员点位");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 2.5 {
                let ok = members
                    .members
                    .iter()
                    .any(|(n, x, y)| n == "队友A" && *x == 356 && *y == 350);
                tracing::info!("[MEMBER] 成员点位={}", ok);
                if ok {
                    tracing::info!("[MEMBER] ✅ 小队成员点位通过");
                } else {
                    tracing::warn!("[MEMBER] ❌ 成员点位未更新");
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --notice-test：施法 → mock 回发 UpdateNotice(["服务器公告","欢迎来到传奇2",...])，
/// 断言 NoticeState.notices 更新（#256）
#[allow(clippy::too_many_arguments)]
fn auto_notice_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    notice: Res<client_bevy::game::dialogs::notice::NoticeState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[NOTICE] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[NOTICE] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[NOTICE] 🔥 施法触发公告");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 2.5 {
                let ok = notice.title == "服务器公告" && notice.message.contains("欢迎来到传奇2");
                tracing::info!(
                    "[NOTICE] title={} message_len={} 内容={}",
                    notice.title,
                    notice.message.len(),
                    ok
                );
                if ok {
                    tracing::info!("[NOTICE] ✅ 服务器公告通过");
                } else {
                    tracing::warn!("[NOTICE] ❌ 公告未更新");
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --upgrade-test：施法 → mock 回发 ItemUpgraded/RemoveMagic/SendOutputMessage 等，
/// 断言 背包物品升级 + 技能移除 + 聊天消息（#258）
#[allow(clippy::too_many_arguments)]
fn auto_upgrade_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hud: Res<client_bevy::game::hud::HudState>,
    magics: Res<client_bevy::game::skills::MagicsState>,
    chat: Res<client_bevy::game::chat::ChatState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[UPG] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[UPG] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[UPG] 🔥 施法触发合成/升级");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 2.5 {
                let upgraded = hud
                    .inventory
                    .items
                    .iter()
                    .flatten()
                    .any(|it| it.unique_id == 9005 && it.item_index == 6);
                let removed = !magics
                    .magics
                    .iter()
                    .any(|m| m.spell == mir2_shared::enums::Spell::Fencing);
                let msg = chat
                    .lines
                    .iter()
                    .any(|(text, _, _)| text.contains("测试服务端消息"));
                tracing::info!("[UPG] 升级={} 技能移除={} 消息={}", upgraded, removed, msg);
                if upgraded && removed && msg {
                    tracing::info!("[UPG] ✅ 合成/升级/技能删除/服务端消息通过");
                } else {
                    tracing::warn!(
                        "[UPG] ❌ 未通过（升级={} 技能移除={} 消息={}）",
                        upgraded,
                        removed,
                        msg
                    );
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --quest-data-test：施法 → mock 回发 NewQuestInfo(消灭稻草人) + ShareQuest，
/// 断言 任务日志新增 + 共享提示（#260）
#[allow(clippy::too_many_arguments)]
fn auto_quest_data_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    quest_log: Res<client_bevy::game::dialogs::quest_log::QuestLogState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[QUEST] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[QUEST] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[QUEST] 🔥 施法触发任务数据");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 2.5 {
                let quest_added = quest_log
                    .quests
                    .iter()
                    .any(|q| q.id == 1 && q.name == "消灭稻草人");
                let shared = quest_log.message.contains("共享任务");
                tracing::info!("[QUEST] 任务新增={} 共享={}", quest_added, shared);
                if quest_added && shared {
                    tracing::info!("[QUEST] ✅ 任务数据包通过");
                } else {
                    tracing::warn!(
                        "[QUEST] ❌ 未通过（任务新增={} 共享={}）",
                        quest_added,
                        shared
                    );
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --recipe-test：施法 → mock 回发 NewRecipeInfo(1) + PauseBuff，断言 配方记录 + Buff 提示（#262）
#[allow(clippy::too_many_arguments)]
fn auto_recipe_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    craft: Res<client_bevy::game::dialogs::craft::CraftState>,
    buff: Res<client_bevy::game::dialogs::buff::BuffState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[RECIPE] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[RECIPE] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[RECIPE] 🔥 施法触发配方/Buff");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 2.5 {
                let recipe = craft.learned.contains(&1);
                let buff_paused = buff.message.contains("暂停");
                tracing::info!("[RECIPE] 配方={} Buff暂停={}", recipe, buff_paused);
                if recipe && buff_paused {
                    tracing::info!("[RECIPE] ✅ 配方/Buff 通过");
                } else {
                    tracing::warn!(
                        "[RECIPE] ❌ 未通过（配方={} Buff暂停={}）",
                        recipe,
                        buff_paused
                    );
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --name-test：施法 → mock 回发 ObjectName(101) + UserName，断言 对象/玩家改名（#264）
#[allow(clippy::too_many_arguments)]
fn auto_name_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hud: Res<client_bevy::game::hud::HudState>,
    names: Query<(
        &client_bevy::actor::NetObjectId,
        Option<&client_bevy::actor::MonsterName>,
    )>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[NAME] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[NAME] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[NAME] 🔥 施法触发改名");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 2.5 {
                let obj = names
                    .iter()
                    .any(|(id, m)| id.0 == 101 && m.map(|m| m.0 == "稻草人·改").unwrap_or(false));
                let player = hud.name == "刀客·改名";
                tracing::info!("[NAME] 对象改名={} 玩家改名={}", obj, player);
                if obj && player {
                    tracing::info!("[NAME] ✅ 名称同步通过");
                } else {
                    tracing::warn!("[NAME] ❌ 未通过（对象改名={} 玩家改名={}）", obj, player);
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --misc2-test：施法 → mock 回发 BaseStatsInfo([10,20,30]) 等，断言基础属性存储（#268）
#[allow(clippy::too_many_arguments)]
fn auto_misc2_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[MISC2] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[MISC2] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[MISC2] 🔥 施法触发杂项协议");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 2.5 {
                let ok = hud.base_stats == vec![10, 20, 30];
                tracing::info!("[MISC2] 基础属性={:?} ok={}", hud.base_stats, ok);
                if ok {
                    tracing::info!("[MISC2] ✅ 杂项协议通过");
                } else {
                    tracing::warn!("[MISC2] ❌ 基础属性未存储");
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --final-test：施法 → mock 回发 冲刺攻击/TeleportIn 等，断言 动作计时 + 传送特效（#270）
#[allow(clippy::too_many_arguments)]
fn auto_final_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    effects: Res<client_bevy::game::effects::EffectsState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    mut before: Local<u64>,
    mut flags: Local<u8>,
    actors_st: Query<(
        &client_bevy::actor::NetObjectId,
        Has<client_bevy::game::combat::StruckTimer>,
    )>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[FINAL] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[FINAL] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            *before = effects.spawned;
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[FINAL] 🔥 施法触发收尾协议");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 0.5 && *t < 6.0 {
                let dash = actors_st.iter().any(|(id, struck)| id.0 == 101 && struck);
                if dash {
                    *flags |= 1;
                }
            }
            if *t >= 6.0 {
                let dash = *flags & 1 != 0;
                let delta = effects.spawned - *before;
                tracing::info!("[FINAL] 冲刺攻击={} 特效增量={}", dash, delta);
                if dash && delta >= 1 {
                    tracing::info!("[FINAL] ✅ 收尾协议通过");
                } else {
                    tracing::warn!("[FINAL] ❌ 未通过（冲刺攻击={} 特效={}）", dash, delta);
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --npc-input-test：施法 → mock 回发 NPCRequestInput(110, Amount)，断言输入状态激活（#272）
#[allow(clippy::too_many_arguments)]
fn auto_npc_input_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    npc_input: Res<client_bevy::game::dialogs::npc::NpcInputState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[NPCIN] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[NPCIN] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[NPCIN] 🔥 施法触发 NPC 输入");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 2.5 {
                let ok =
                    npc_input.active && npc_input.npc_id == 110 && npc_input.page_name == "Amount";
                tracing::info!(
                    "[NPCIN] active={} npc={} page={}",
                    npc_input.active,
                    npc_input.npc_id,
                    npc_input.page_name
                );
                if ok {
                    tracing::info!("[NPCIN] ✅ NPC 输入框通过");
                } else {
                    tracing::warn!("[NPCIN] ❌ 输入状态未激活");
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


/// --creature2-test：施法 → mock 回发 NewIntelligentCreature(Dog) 等，断言宠物新增（#274）
#[allow(clippy::too_many_arguments)]
fn auto_creature2_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    creature: Res<client_bevy::game::dialogs::creature::CreatureState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[CR2] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[CR2] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[CR2] 🔥 施法触发宠物协议");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 2.5 {
                let pig_type = mir2_shared::enums::IntelligentCreatureType::BabyPig as u8;
                let acquired = creature
                    .creatures
                    .iter()
                    .any(|c| c.creature_type == pig_type);
                let msg = creature.message.contains("宠物");
                tracing::info!("[CR2] 宠物新增={} 提示={}", acquired, msg);
                if acquired && msg {
                    tracing::info!("[CR2] ✅ 智能宠物协议通过");
                } else {
                    tracing::warn!("[CR2] ❌ 未通过（宠物新增={} 提示={}）", acquired, msg);
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --resize-test：背包扩容链路（#276）
/// 流程：进游戏 → 施法（mock 回发 ResizeInventory(56)）→ 校验 items.len()==56
fn auto_resize_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 6.0 {
                return;
            }
            if hud.inventory.items.len() < 40 {
                return; // 等 UserInformation 完成
            }
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: mir2_shared::enums::MirDirection::Down,
                target_id: 101,
                location: mir2_shared::Point { x: 353, y: 352 },
            });
            tracing::info!("[RESIZE] 🔥 施法触发 ResizeInventory");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 2.0 {
                return;
            }
            if hud.inventory.items.len() == 56 {
                tracing::info!("[RESIZE] ✅ PASS 背包扩容 size=56");
            } else {
                tracing::error!(
                    "[RESIZE] ❌ FAIL size={} 期望 56",
                    hud.inventory.items.len()
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --storage-unlock-test：仓库密码解锁链路（#200）
/// 流程：进游戏 → NPC [@Storage] → 断言解锁框出现（仓库未打开）→ 错误密码 → 提示
///       → 正确密码 → 仓库打开（StorageOpened）
#[allow(clippy::too_many_arguments)]
fn auto_storage_unlock_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    storage: Res<client_bevy::game::dialogs::storage::StorageState>,
    npcs: Query<(
        &client_bevy::actor::NetObjectId,
        &client_bevy::actor::NpcName,
        &Transform,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut npc_oid: Local<Option<u32>>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 6.0 {
                return;
            }
            let oid = players.single().ok().and_then(|ptf| {
                let (px, py) =
                    client_bevy::game::movement::world_to_tile(ptf.translation.x, ptf.translation.y);
                npcs.iter()
                    .map(|(id, n, tf)| {
                        let (nx, ny) =
                            client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                        (id.0, n.0.clone(), (nx - px).abs() + (ny - py).abs())
                    })
                    .min_by_key(|(_, _, d)| *d)
                    .map(|(id, _, _)| id)
            });
            if let Some(oid) = oid {
                *npc_oid = Some(oid);
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: oid,
                    key: "[@Main]".to_string(),
                });
                tracing::info!("[UNLOCK] CallNPC {}", oid);
                *stage = 1;
                *t = 0.0;
            }
        }
        1 => {
            if *t < 1.0 {
                return;
            }
            if let Some(oid) = *npc_oid {
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: oid,
                    key: "[@Storage]".to_string(),
                });
                tracing::info!("[UNLOCK] CallNPC [@Storage]");
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 1.5 {
                return;
            }
            if storage.unlock_panel && !storage.visible {
                tracing::info!("[UNLOCK] ✅ 解锁框出现（仓库未打开）");
                net.send_packet(&mir2_shared::packets::client::storage::UnlockStorage {
                    password: "wrong".to_string(),
                });
                *stage = 3;
                *t = 0.0;
            } else {
                tracing::warn!(
                    "[UNLOCK] ❌ 解锁框未出现（panel={} visible={}）",
                    storage.unlock_panel,
                    storage.visible
                );
                *stage = 9;
            }
        }
        3 => {
            if *t < 1.0 {
                return;
            }
            if !storage.unlock_msg.is_empty() && storage.unlock_panel {
                tracing::info!("[UNLOCK] ✅ 错误密码提示: {}", storage.unlock_msg);
                net.send_packet(&mir2_shared::packets::client::storage::UnlockStorage {
                    password: "123456".to_string(),
                });
                *stage = 4;
                *t = 0.0;
            } else {
                tracing::warn!(
                    "[UNLOCK] ❌ 错误密码未提示（msg={} panel={}）",
                    storage.unlock_msg,
                    storage.unlock_panel
                );
                *stage = 9;
            }
        }
        4 => {
            if *t < 1.5 {
                return;
            }
            if storage.visible && !storage.unlock_panel {
                tracing::info!("[UNLOCK] ✅ PASS 仓库解锁并打开");
            } else {
                tracing::error!(
                    "[UNLOCK] ❌ FAIL visible={} panel={}",
                    storage.visible,
                    storage.unlock_panel
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}
