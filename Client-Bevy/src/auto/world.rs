//! auto::world 自动化验证系统（从 auto.rs 拆分，#1146）

use bevy::prelude::*;
use super::*;

/// --fishing-test：打开钓鱼 → 抛竿 → 等 FishingUpdate → 等收获聊天消息
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_fishing_test(
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
                .find(|(text, _, _, _)| {
                    text.contains("钓到了") || text.contains("鱼跑了") || text.contains("需要装备鱼竿")
                })
                .map(|(text, _, _, _)| text.clone());
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

/// --quest-test：打开任务日志 → 接受任务1 → 等 ChangeQuest → 放弃
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_quest_test(
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
pub(crate) fn auto_buff_test(
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
pub(crate) fn auto_report_test(
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
        chat.lines.iter().rev().take(60).any(|(t, _, _, _)| t.contains(needle))
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
pub(crate) fn auto_inspect_test(
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
                        ranking: false,
                        name: String::new(),
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
pub(crate) fn auto_creature_test(
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
pub(crate) fn auto_hero_test(
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

/// --hero-exp-test：部署英雄 → 战斗击杀 → 等 GainHeroExperience 使 hero_exp 增长 → 切回（#1142/#1163）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_hero_exp_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hero: Res<client_bevy::game::dialogs::hero::HeroState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut initial_exp: Local<i64>,
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
            tracing::info!("[HEROEXP] 部署英雄 1");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 10.0 {
                tracing::warn!("[HEROEXP] ❌ 未切换到英雄");
                *stage = 9;
                return;
            }
            if hero.hero_index == 1 {
                *initial_exp = hero.hero_exp;
                tracing::info!("[HEROEXP] ✅ 英雄已部署 初始经验={}", hero.hero_exp);
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            // 英雄击杀怪物 → 主人得经验 → 英雄分得经验（GainHeroExperience → HeroState.hero_exp）
            if *t >= 40.0 {
                tracing::warn!("[HEROEXP] ❌ 经验未增长（initial={} now={}）", *initial_exp, hero.hero_exp);
                *stage = 9;
                return;
            }
            if hero.hero_exp > *initial_exp {
                tracing::info!("[HEROEXP] ✅ 英雄经验增长: {} -> {}", *initial_exp, hero.hero_exp);
                net.send_packet(&client_bevy::network::ChangeHeroWire { hero_index: 0 });
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t >= 8.0 {
                tracing::warn!("[HEROEXP] ⚠️ 切回主角色未确认");
                *stage = 9;
                return;
            }
            if hero.hero_index == 0 {
                tracing::info!("[HEROEXP] ✅ 切回主角色成功");
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --hero-battle-test：部署英雄 → 等待 HeroHealthChanged 使 hero_hp 下降 → 切回主角色（#1135）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_hero_battle_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hero: Res<client_bevy::game::dialogs::hero::HeroState>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut initial_hp: Local<i32>,
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
            tracing::info!("[HEROBATTLE] 部署英雄 1");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t >= 10.0 {
                tracing::warn!("[HEROBATTLE] ❌ 未切换到英雄");
                *stage = 9;
                return;
            }
            if hero.hero_index == 1 && hero.hero_hp > 0 {
                *initial_hp = hero.hero_hp;
                tracing::info!("[HEROBATTLE] ✅ 英雄已部署 初始 HP={}", hero.hero_hp);
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            // 等待 HP 下降（收到 HeroHealthChanged → HeroState.hero_hp 更新）
            if *t >= 30.0 {
                tracing::warn!("[HEROBATTLE] ❌ HP 未下降（initial={} now={}）", *initial_hp, hero.hero_hp);
                *stage = 9;
                return;
            }
            if hero.hero_hp < *initial_hp {
                tracing::info!("[HEROBATTLE] ✅ 英雄 HP 实时同步: {} -> {}", *initial_hp, hero.hero_hp);
                net.send_packet(&client_bevy::network::ChangeHeroWire { hero_index: 0 });
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            if *t >= 8.0 {
                tracing::warn!("[HEROBATTLE] ⚠️ 切回主角色未确认");
                *stage = 9;
                return;
            }
            if hero.hero_index == 0 {
                tracing::info!("[HEROBATTLE] ✅ 切回主角色成功");
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --mount-test：打开坐骑面板 → 骑乘/下马（@ride）→ 外观广播 → 坐骑层
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_mount_test(
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

/// --roll-test：触发 NPC 掷骰 → 服务端 Roll 包 → 客户端骰子对话框 → 自动回调
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_roll_test(
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

/// --auto-quest：任务闭环自动化（#44）
/// 阶段：0 接受任务1 → 1 等 ChangeQuest → 2 自动击杀怪物101 直到完成 → 3 交任务验证 CompleteQuest
pub(crate) fn auto_quest_system(
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

/// --reincarnation-test：死亡 → S.RequestReincarnation offer → 接受 → 复活（#222）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_reincarnation_test(
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

/// --object-state-test：施法 → mock 触发 隐藏/击退/坐下/显形/传送 状态机，逐项断言（#226）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_object_state_test(
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

/// --map-fx-test：施法 → mock 回发 MapEffect/PlaySound/SetTimer，4s 后 ExpireTimer，
/// 断言 特效生成 + 计时器激活 + 计时器关闭（#230）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_map_fx_test(
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
pub(crate) fn auto_mount_sync_test(
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

/// --npc-credit-test：施法 → mock 回发 NPCImageUpdate(110→2) + GainedCredit(50)，
/// 断言 NPC 形象变化 + 声望累积（#248）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_npc_credit_test(
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
pub(crate) fn auto_compass_test(
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
pub(crate) fn auto_sneak_test(
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

/// --quest-data-test：施法 → mock 回发 NewQuestInfo(消灭稻草人) + ShareQuest，
/// 断言 任务日志新增 + 共享提示（#260）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_quest_data_test(
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

/// --name-test：施法 → mock 回发 ObjectName(101) + UserName，断言 对象/玩家改名（#264）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_name_test(
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
    mut attempts: Local<u32>,
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
                    *stage = 9;
                } else if *attempts < 3 {
                    // 综合冒烟：怪物可能处于死亡/重生窗口，重试施法触发演示批次
                    *attempts += 1;
                    tracing::warn!("[NAME] ⚠️ 未通过（第 {} 次），重试施法", *attempts);
                    *stage = 1;
                    *t = 0.0;
                } else {
                    tracing::warn!("[NAME] ❌ 未通过（对象改名={} 玩家改名={}）", obj, player);
                    *stage = 9;
                }
            }
        }
        _ => {}
    }
}

/// --misc2-test：施法 → mock 回发 BaseStatsInfo([10,20,30]) 等，断言基础属性存储（#268）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_misc2_test(
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
pub(crate) fn auto_final_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    effects: Res<client_bevy::game::effects::EffectsState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    mut attempts: Local<u32>,
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
                let dash = actors_st
                    .iter()
                    .any(|(id, struck)| Some(id.0) == *target && struck);
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
                    *stage = 9;
                } else if *attempts < 3 {
                    // 综合冒烟：目标怪物可能处于死亡/重生窗口，重试施法
                    *attempts += 1;
                    tracing::warn!("[FINAL] ⚠️ 未通过（第 {} 次），重试施法", *attempts);
                    *flags = 0;
                    *stage = 1;
                    *t = 0.0;
                } else {
                    tracing::warn!("[FINAL] ❌ 未通过（冲刺攻击={} 特效={}）", dash, delta);
                    *stage = 9;
                }
            }
        }
        _ => {}
    }
}

/// --npc-input-test：施法 → mock 回发 NPCRequestInput(110, Amount)，断言输入状态激活（#272）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_npc_input_test(
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
pub(crate) fn auto_creature2_test(
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

/// --level-fx-test：升级表现链路（#283）
/// 流程：进游戏 → 连续攻击击杀怪物 → mock 回发 LevelChanged+ObjectLeveled → 断言 hud.level 提升
pub(crate) fn auto_level_fx_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut hits: Local<u32>,
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
            net.send_packet(&mir2_shared::packets::client::combat::Attack {
                direction: mir2_shared::enums::MirDirection::Down,
                spell: mir2_shared::enums::Spell::None,
            });
            tracing::info!("[LEVELFX] 开始攻击（第 {} 击）", *hits + 1);
            *hits = 1;
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 1.0 {
                return;
            }
            *hits += 1;
            net.send_packet(&mir2_shared::packets::client::combat::Attack {
                direction: mir2_shared::enums::MirDirection::Down,
                spell: mir2_shared::enums::Spell::None,
            });
            *t = 0.0;
            if *hits >= 9 {
                *stage = 2;
                *t = 0.0;
            }
        }
        2 => {
            if *t < 2.0 {
                return;
            }
            if hud.level >= 31 {
                tracing::info!("[LEVELFX] ✅ PASS 升级生效 level={}", hud.level);
            } else {
                tracing::error!("[LEVELFX] ❌ FAIL level={} 期望 >=31", hud.level);
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --chat-item-test：聊天物品链路（#285）
/// 流程：施法触发演示批次（含 NewChatItem 9005）→ 断言缓存；RequestChatItem(9999) → mock 回发 → 缓存增长
pub(crate) fn auto_chat_item_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    cache: Res<client_bevy::game::chat::ChatItemCache>,
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
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: mir2_shared::enums::MirDirection::Down,
                target_id: 101,
                location: mir2_shared::Point { x: 353, y: 352 },
            });
            tracing::info!("[CHATITEM] 🔥 施法触发演示批次");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 2.0 {
                return;
            }
            if cache.items.contains_key(&9005) {
                tracing::info!("[CHATITEM] ✅ 缓存包含 9005");
                net.send_packet(&mir2_shared::packets::client::misc::RequestChatItem {
                    chat_item_id: 9999,
                });
                *stage = 2;
                *t = 0.0;
            } else {
                tracing::warn!("[CHATITEM] ❌ 缓存缺少 9005（{} 条）", cache.items.len());
                *stage = 9;
            }
        }
        2 => {
            if *t < 1.5 {
                return;
            }
            if cache.items.contains_key(&9999) {
                tracing::info!("[CHATITEM] ✅ PASS 请求回发 9999 已缓存");
            } else {
                tracing::error!("[CHATITEM] ❌ FAIL 9999 未缓存（{} 条）", cache.items.len());
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --session-feedback-test：会话反馈链路（#289）
/// 流程：进游戏 → 聊天发送 @RETURNLOGIN → mock 回发 S.ReturnToLogin → 断言返回 Login
pub(crate) fn auto_session_feedback_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *state != AppState::Game {
                return;
            }
            if *t < 6.0 {
                return;
            }
            net.send_packet(&mir2_shared::packets::client::chat::Chat {
                message: "@RETURNLOGIN".to_string(),
                linked_items: Vec::new(),
            });
            tracing::info!("[SESSION] 发送 @RETURNLOGIN");
            *stage = 1;
            *t = 0.0;
        }
        1 => {
            if *t < 2.0 {
                return;
            }
            if *state == AppState::Login {
                tracing::info!("[SESSION] ✅ PASS 已返回登录界面");
            } else {
                tracing::error!("[SESSION] ❌ FAIL state={:?}", *state);
            }
            *stage = 9;
        }
        _ => {}
    }
}


