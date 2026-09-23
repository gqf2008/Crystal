//! 跨图传送落图对象全量重同步（map resync）。
//!
//! 背景：客户端换图重建时会清掉旧图 NetObjectId 实体，其前提是「新图对象由服务端
//! 换图后全量重发」。该前提此前只对登录进图（session.rs StartGame）与过门
//! （session.rs WorldMoveRequest 移动传送点）成立；脚本传送（npc_script.rs
//! `teleport_player`：MOVE/ENTERMAP/INSTANCEMOVE/@MAP，任务传送常用入口）与
//! GM @GOTO（mod.rs GmGotoRequest）只发 MapChanged+MapInformation+UserLocation，
//! 不清旧图对象、不推新图 NPC/怪物/地面物、无玩家互见同步、无英雄召回——
//! 客户端落图即空图。
//!
//! 本模块把过门路径的对象同步块抽成 [`WorldActor::resync_map_objects_for_session`]，
//! 并提供统一传送入口 [`teleport_core`] / [`TeleportPlayerWithResync`] 消息。
//! session.rs 过门原路径本轮不动（行为零变化），去重留作跟进项。
//!
//! 已接入统一入口的路径：
//! - NPC 脚本 MOVE/ENTERMAP/INSTANCEMOVE/@MAP（npc_script.rs `teleport_player`）；
//! - GM @GOTO（mod.rs `GmGotoRequest`）、GM/NPC 指令 @MOVE 与 RECALL（mod.rs）；
//! - #935 组队不足强制送回绑定点（tick.rs）；
//! - social.rs 四处召回：配偶召回 / GROUPRECALL / RECALLMEMBER / NPC GROUPRECALL
//!   （SocialActor 经 world_ref ask `TeleportPlayerWithResync`；world_ref 未接线的
//!   测试栈降级旧行为并 warn!）。
//! 已知同图位移（不换图、只需 UserLocation/无下行）不在本入口范围：坐骑上下、
//! 被推/死亡推开等 map_index: None 路径保持原样。

use super::*;

impl WorldActor {
    /// 跨图传送后的对象全量重同步（抽取自 session.rs WorldMoveRequest 过门路径，保持一致）：
    ///
    /// 1. 旧图 ObjectRemove 全清（怪物/其他玩家/地面物；旧图 NPC/装饰物由客户端换图重建清理）；
    /// 2. 新图 NPC/怪物生成下发（spawn_npcs_and_monsters）+ 征服旗 + 装饰物；
    /// 3. 新图地面物品/金币、已打开的门；
    /// 4. 玩家互见（旧图广播移除 mover、新图双向 ObjectPlayer）；
    /// 5. 英雄随主人跨图召回（C# HeroObject.OwnerRecall → Teleport(Owner.CurrentMap, Owner.Back)）。
    ///
    /// 调用方须已完成 SetPlayerPosition 并下发 MapChanged/MapInformation/UserLocation。
    /// old_map == dest_map_index（同图位移）时跳过：客户端不换图重建，既有对象全部有效。
    pub(crate) async fn resync_map_objects_for_session(
        &mut self,
        session_id: u64,
        old_map: u16,
        dest_map_index: u16,
        dest_file: &str,
    ) {
        if old_map == dest_map_index {
            // 同图位移：客户端无换图重建，对象无需重发（与传送前行为一致）
            return;
        }
        let dest_info_idx = dest_map_index as i32;

        // ---- 1. 旧图 ObjectRemove 全清（对齐过门路径：怪物 → 其他玩家 → 地面物）----
        let stale_monsters: Vec<u32> = self
            .monsters
            .values()
            .filter(|m| m.map_index == old_map)
            .map(|m| m.object_id)
            .collect();
        for oid in stale_monsters {
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id,
                    data: object_remove_packet(oid),
                })
                .try_send();
        }
        for (sid, rec) in &self.players {
            if *sid == session_id {
                continue;
            }
            if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                if s.map_index == old_map {
                    let _ = self
                        .gate_ref
                        .tell(SendToClient {
                            session_id,
                            data: object_remove_packet(s.object_id),
                        })
                        .try_send();
                }
            }
        }
        let stale_ground: Vec<u32> = self
            .ground_items
            .iter()
            .filter(|gi| gi.map_index == old_map)
            .map(|gi| gi.object_id)
            .collect();
        for oid in stale_ground {
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id,
                    data: object_remove_packet(oid),
                })
                .try_send();
        }

        // ---- 2. 新图 NPC/怪物生成下发（C# GetObjectsPassive）----
        // 地图级生成物单真源（对齐 C# Map.Objects）：目标图已物化时只按既有
        // object_id 重放给本会话（与 StartGame 路径同款理由与注释）。
        let reused_map_spawns = self.map_spawns_ready.contains(&dest_map_index);
        let new_monsters = if reused_map_spawns {
            send_map_spawns_to_session(
                &self.gate_ref,
                session_id,
                dest_map_index,
                &self.npcs,
                &self.monsters,
            );
            Vec::new()
        } else {
            let spawn_ctx = SpawnContext {
                map_info: self.map_infos.get(&dest_info_idx),
                monster_infos: &self.monster_infos,
                npc_infos: &self.npc_infos,
                dragon_info: self.dragon_info.as_ref(),
                rarity: self.rarity_cfg.clone(),
                routes: &self.routes,
            };
            let (npcs, monsters) = spawn_npcs_and_monsters(
                self.gate_ref.clone(),
                &self.spawn_dir,
                dest_file,
                dest_map_index,
                session_id,
                &mut self.next_object_id,
                &spawn_ctx,
                self.maps.get(&dest_map_index),
            )
            .await;
            if !(npcs.is_empty() && monsters.is_empty()) {
                self.map_spawns_ready.insert(dest_map_index);
            }
            for npc in npcs {
                self.npcs.insert(npc.object_id, npc);
            }
            monsters
        };
        // 征服旗子 NPC（C# ConquestGuildFlagInfo.Spawn；per-session 生成）
        let new_flags = spawn_conquest_flags(
            self.gate_ref.clone(),
            self.social_ref.clone(),
            &self.conquest_instances,
            dest_map_index,
            session_id,
            &mut self.next_object_id,
        )
        .await;
        for flag in new_flags {
            self.conquest_flags.insert(flag.object_id, flag);
        }
        // 装饰物同步（C# GetObjectsPassive 含 DecoObject）
        self.sync_decos_on_map(session_id, dest_map_index).await;
        // 先收集精英广播信息（move 前遍历）；复用分支下怪物是既有对象，不重复广播
        let elite_broadcasts: Vec<String> = if reused_map_spawns {
            Vec::new()
        } else {
            new_monsters
                .iter()
                .filter(|m| m.rarity > 0)
                .map(|m| m.name.clone())
                .collect()
        };
        if !reused_map_spawns {
            for monster in new_monsters {
                self.monsters.insert(monster.object_id, monster);
            }
        }
        // 初始生成精英广播
        for name in &elite_broadcasts {
            let map_name = self
                .map_infos
                .get(&dest_info_idx)
                .map(|m| m.title.clone())
                .unwrap_or_else(|| "未知地图".to_string());
            broadcast_system_message(
                &self.gate_ref,
                &self.players,
                &format!("一只 {} 出现在 {}！勇士们，前往讨伐！", name, map_name),
            );
        }

        // ---- 3. 新图地面物品/金币 + 已打开的门 ----
        let ground_sync: Vec<_> = self
            .ground_items
            .iter()
            .filter(|gi| gi.map_index == dest_map_index)
            .map(|gi| (gi.object_id, gi.item.clone(), gi.x, gi.y, gi.gold_amount))
            .collect();
        for (drop_oid, item, x, y, gold_amount) in ground_sync {
            if item.item_index == 0 {
                let object_gold = mir2_shared::packets::server::ObjectGold {
                    object_id: drop_oid,
                    gold: gold_amount,
                    location_x: x,
                    location_y: y,
                };
                let mut buf = Vec::new();
                if mir2_shared::packets::base::serialize_packet(
                    &mut std::io::Cursor::new(&mut buf),
                    &object_gold,
                )
                .is_ok()
                {
                    let _ = self
                        .gate_ref
                        .tell(SendToClient {
                            session_id,
                            data: buf,
                        })
                        .try_send();
                }
            } else {
                let object_item = mir2_shared::packets::server::ObjectItem {
                    object_id: drop_oid,
                    item,
                    location_x: x,
                    location_y: y,
                };
                let mut buf = Vec::new();
                if mir2_shared::packets::base::serialize_packet(
                    &mut std::io::Cursor::new(&mut buf),
                    &object_item,
                )
                .is_ok()
                {
                    let _ = self
                        .gate_ref
                        .tell(SendToClient {
                            session_id,
                            data: buf,
                        })
                        .try_send();
                }
            }
        }
        let open_doors_sync: Vec<u8> = self
            .open_doors
            .iter()
            .filter(|(map_idx, _)| *map_idx == dest_map_index)
            .map(|(_, door_idx)| *door_idx)
            .collect();
        for door_idx in open_doors_sync {
            send_opendoor(&self.gate_ref, session_id, door_idx, false).await;
        }

        // ---- 4. 玩家互见（#1653：C# PlayerObject.Teleport → GetObjectsPassive）----
        let mover_state = match self.players.get(&session_id) {
            Some(rec) => match rec.actor_ref.ask(GetPlayerState).await {
                Ok(Some(s)) => s,
                _ => return,
            },
            None => return,
        };
        // 通知旧地图其他玩家移除 mover
        let remove_packet = object_remove_packet(mover_state.object_id);
        broadcast_to_map_nb(&self.gate_ref, &self.players, old_map, &remove_packet).await;
        // 向 mover 发送新地图其他玩家
        self.resync_send_map_players_to(session_id, &mover_state, dest_map_index)
            .await;
        // 向新地图其他玩家发送 mover（隐身跳过）
        self.resync_send_player_to_map(session_id, &mover_state, dest_map_index)
            .await;

        // ---- 5. 英雄随主人跨图召回（#1661：C# HeroObject.OwnerRecall）----
        if self.hero_ai_states.contains_key(&session_id) {
            let hero_oid = mover_state.object_id.wrapping_add(hero::HERO_OID_OFFSET);
            let hero_remove = object_remove_packet(hero_oid);
            broadcast_to_map_nb(&self.gate_ref, &self.players, old_map, &hero_remove).await;
            if let Some(ai) = self.hero_ai_states.get_mut(&session_id) {
                let (hx, hy) = point_move(mover_state.x, mover_state.y, mover_state.direction, 1);
                ai.x = hx;
                ai.y = hy;
                ai.direction = mover_state.direction;
            }
            // 新地图广播英雄生成（C# CurrentMap.Broadcast）
            self.broadcast_hero_spawn(session_id).await;
        }
    }

    /// 构建玩家 ObjectPlayer 数据包（观察者相对色/行会战；C# GetNameColour）。
    ///
    /// 注意：本方法是 session.rs `build_player_object_packet` 的副本——该私有方法定义在
    /// world::session 内，本模块（world::map_sync）不可达；去重留作跟进项。
    async fn resync_player_object_packet(
        &self,
        target: &crate::actors::player::PlayerState,
        viewer: Option<&crate::actors::player::PlayerState>,
    ) -> Vec<u8> {
        let target_weapon = target
            .inventory
            .get_equipment(EquipmentSlot::Weapon)
            .and_then(|item| self.item_infos.get(&item.item_index))
            .map(|info| info.shape as i16)
            .unwrap_or(-1);
        let target_armor = target
            .inventory
            .get_equipment(EquipmentSlot::Armour)
            .and_then(|item| self.item_infos.get(&item.item_index))
            .map(|info| info.shape as i16)
            .unwrap_or(0);
        let target_weapon_effect = target
            .inventory
            .get_equipment(EquipmentSlot::Weapon)
            .and_then(|item| self.item_infos.get(&item.item_index))
            .map(|info| info.effect as i16)
            .unwrap_or(0);
        let (at_war, enemy) = super::guild_war_flags(
            viewer.and_then(|v| v.guild_name.as_deref()),
            target.guild_name.as_deref(),
            &self.guild_wars,
        );
        let colour = super::name_colour_for_viewer(
            target.pk_points,
            super::is_brown(target.brown_until_ms),
            self.is_conquest_map(target.map_index),
            target.guild_name.as_deref(),
            viewer.and_then(|v| v.guild_name.as_deref()),
            at_war,
            enemy,
        );
        // #934：C# MapInfo.NoNames——按目标所在地图掩码
        let display_name = if self
            .map_infos
            .get(&(target.map_index as i32))
            .map(|m| m.no_names)
            .unwrap_or(false)
        {
            "?????"
        } else {
            target.name.as_str()
        };
        build_object_player_packet(
            display_name,
            target.object_id,
            target.x,
            target.y,
            target.direction,
            target.level,
            colour,
            target.class,
            target.gender,
            target.hair,
            target_weapon,
            target_weapon_effect,
            target_armor,
            target.mount_type,
            target.is_mounted,
            target.level_effects,
            target.guild_name.as_deref().unwrap_or(""),
            crate::actors::world::guild_rank_name(target.guild_rank),
            // #2892：`Hidden` 随进视野包下发（C# `PlayerObject.cs:4799`）
            crate::actors::world::player_hidden(target),
        )
    }

    /// 把同图其他玩家的 ObjectPlayer 发给 viewer（进图同步；跳过隐身与跨图）。
    /// session.rs `send_map_players_to` 副本（私有不可达；去重留作跟进项）。
    async fn resync_send_map_players_to(
        &self,
        viewer_session: u64,
        viewer_state: &crate::actors::player::PlayerState,
        map_index: u16,
    ) {
        for (sid, rec) in &self.players {
            if *sid == viewer_session {
                continue;
            }
            let Ok(Some(ep_state)) = rec.actor_ref.ask(GetPlayerState).await else {
                continue;
            };
            if ep_state.map_index != map_index {
                continue;
            }
            // #2892：Observer 集（`Sneaking` 或 GM `@observer`）里的玩家不进他人视野；
            // GM 观战没有隐身 buff，只按 buff 判会漏
            let is_invisible = self.invisible_sessions.contains(sid)
                || ep_state
                    .buffs
                    .iter()
                    .any(|b| crate::combat::buff::is_sneaking_type(&b.buff_type));
            if is_invisible {
                continue;
            }
            let packet = self
                .resync_player_object_packet(&ep_state, Some(viewer_state))
                .await;
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: viewer_session,
                    data: packet,
                })
                .try_send();
        }
    }

    /// 把 mover 的 ObjectPlayer 发给同图其他玩家（跳过隐身；颜色按各观察者计算）。
    /// session.rs `send_player_to_map` 副本（私有不可达；去重留作跟进项）。
    async fn resync_send_player_to_map(
        &self,
        mover_session: u64,
        mover_state: &crate::actors::player::PlayerState,
        map_index: u16,
    ) {
        if self.invisible_sessions.contains(&mover_session)
            || mover_state
                .buffs
                .iter()
                .any(|b| crate::combat::buff::is_sneaking_type(&b.buff_type))
        {
            return;
        }
        for (sid, rec) in &self.players {
            if *sid == mover_session {
                continue;
            }
            let Ok(Some(viewer)) = rec.actor_ref.ask(GetPlayerState).await else {
                continue;
            };
            if viewer.map_index != map_index {
                continue;
            }
            let packet = self
                .resync_player_object_packet(mover_state, Some(&viewer))
                .await;
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: *sid,
                    data: packet,
                })
                .try_send();
        }
    }
}

/// 统一传送核心（跨图/同图）：SetPlayerPosition + 下行包 + 跨图 resync。
///
/// 下行包规则：跨图 → MapChanged + MapInformation + UserLocation 后接
/// [`WorldActor::resync_map_objects_for_session`] 全量重发；同图 → 仅 UserLocation
/// （客户端无换图重建，既有对象全部有效，MapChanged 反而触发多余重建）。
/// 返回 false = 玩家不在线（调用方自行决定降级）。
///
/// 调用方：npc_script.rs `teleport_player`（脚本 MOVE/@MAP，前置 required_group
/// 校验、后置 apply_map_entry_rules）、mod.rs GM 指令、tick.rs #935 强制送回、
/// [`TeleportPlayerWithResync`] 消息（social 召回等外部 actor 入口）。
pub(crate) async fn teleport_core(
    world: &mut WorldActor,
    session_id: u64,
    map_index: u16,
    x: i32,
    y: i32,
    direction: u8,
) -> bool {
    let dest = world.map_infos.get(&(map_index as i32)).cloned();
    let Some(dest_mi) = dest else {
        // 无地图配置：仍尝试改坐标（teleport_player 既有兜底语义）
        if let Some(record) = world.players.get(&session_id) {
            let _ = record
                .actor_ref
                .ask(SetPlayerPosition {
                    x,
                    y,
                    direction,
                    map_index: Some(map_index),
                    is_mounted: None,
                })
                .await;
            return true;
        }
        return false;
    };
    let dest_file = dest_mi.file_name.clone();
    let dest_title = dest_mi.title.clone();
    let _ = world.get_or_load_map(&dest_file, map_index);

    let Some(record) = world.players.get(&session_id) else {
        return false;
    };
    // 换图前记录旧图（跨图传送后需全量重同步新图对象）
    let old_map = record
        .actor_ref
        .ask(GetPlayerState)
        .await
        .ok()
        .flatten()
        .map(|s| s.map_index);
    let _ = record
        .actor_ref
        .ask(SetPlayerPosition {
            x,
            y,
            direction,
            map_index: Some(map_index),
            is_mounted: None,
        })
        .await;

    let cross_map = old_map != Some(map_index);
    if cross_map {
        let map_pkt = build_map_changed_packet(
            map_index,
            &dest_file,
            &dest_title,
            x,
            y,
            direction,
            Some(&dest_mi),
        );
        if let Err(e) = world
            .gate_ref
            .tell(SendToClient {
                session_id,
                data: map_pkt,
            })
            .try_send()
        {
            warn!(
                "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                session_id,
                dropped_send_opcode(&e),
                e
            );
        }
        // C# GetMapInfo：换图补发 MapInformation
        let map_info =
            build_map_information_packet(map_index, &dest_file, &dest_title, Some(&dest_mi));
        if let Err(e) = world
            .gate_ref
            .tell(SendToClient {
                session_id,
                data: map_info,
            })
            .try_send()
        {
            warn!(
                "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                session_id,
                dropped_send_opcode(&e),
                e
            );
        }
    }
    // UserLocation：跨图/同图都发（客户端落位确认，对齐 C# Teleport 语义）
    let mut body = Vec::new();
    body.extend_from_slice(&x.to_le_bytes());
    body.extend_from_slice(&y.to_le_bytes());
    body.push(direction);
    if let Err(e) = world
        .gate_ref
        .tell(SendToClient {
            session_id,
            data: build_packet_bytes(
                mir2_shared::enums::ServerPacketIds::UserLocation as i16,
                &body,
            ),
        })
        .try_send()
    {
        warn!(
            "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
            session_id,
            dropped_send_opcode(&e),
            e
        );
    }
    // 跨图落图同步：旧图 ObjectRemove 全清 + 新图 NPC/怪物/地面物/玩家互见/英雄召回全量重发
    if cross_map {
        if let Some(old_map) = old_map {
            world
                .resync_map_objects_for_session(session_id, old_map, map_index, &dest_file)
                .await;
        }
    }
    true
}

/// 统一传送消息：SocialActor 等无法持有 &mut WorldActor 的 actor 的跨图传送入口
/// （配偶/组队/成员召回等）。语义同 [`teleport_core`]。
pub struct TeleportPlayerWithResync {
    pub session_id: u64,
    pub map_index: u16,
    pub x: i32,
    pub y: i32,
    pub direction: u8,
}

impl Message<TeleportPlayerWithResync> for WorldActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        msg: TeleportPlayerWithResync,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        teleport_core(
            self,
            msg.session_id,
            msg.map_index,
            msg.x,
            msg.y,
            msg.direction,
        )
        .await
    }
}

/// 构建 ObjectRemove 数据包（[ObjectID u32]）
fn object_remove_packet(object_id: u32) -> Vec<u8> {
    build_packet_bytes(
        mir2_shared::enums::ServerPacketIds::ObjectRemove as i16,
        &object_id.to_le_bytes(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::account::AccountActor;
    use crate::actors::social::{SocialActorArgs, SocialActorConfig};
    use crate::actors::world::{GmGotoRequest, WorldActorArgs};
    use crate::gate::actor::{ClientData, SessionCreated, SetAccountRef, SetWorldRef};
    use tokio::sync::mpsc;

    type RxChannel = tokio::sync::mpsc::Receiver<Vec<u8>>;

    async fn wait_opcode_body(rx: &mut RxChannel, opcode: i16, secs: u64) -> Option<Vec<u8>> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(secs);
        while tokio::time::Instant::now() < deadline {
            let remaining = deadline - tokio::time::Instant::now();
            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Some(data)) if data.len() >= 4 => {
                    if i16::from_le_bytes([data[2], data[3]]) == opcode {
                        return Some(data[4..].to_vec());
                    }
                }
                Ok(Some(_)) => continue,
                _ => return None,
            }
        }
        None
    }

    async fn login(
        gate_ref: &ActorRef<GateActor>,
        session_id: u64,
        rx: &mut RxChannel,
        user: &str,
    ) {
        let cv_body = {
            let mut b = Vec::new();
            let hash = b"test";
            b.extend_from_slice(&(hash.len() as i32).to_le_bytes());
            b.extend_from_slice(hash);
            b
        };
        let _ = gate_ref
            .ask(ClientData {
                session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ClientPacketIds::ClientVersion as i16,
                    &cv_body,
                ),
            })
            .await;
        let _ = gate_ref
            .ask(ClientData {
                session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ClientPacketIds::NewAccount as i16,
                    &[],
                ),
            })
            .await;
        let mut lb = Vec::new();
        let _ = mir2_shared::binary::write_dotnet_string(&mut lb, user);
        let _ = mir2_shared::binary::write_dotnet_string(&mut lb, "testpass");
        let _ = gate_ref
            .ask(ClientData {
                session_id,
                data: build_packet_bytes(mir2_shared::enums::ClientPacketIds::Login as i16, &lb),
            })
            .await;
        let ok = mir2_shared::enums::ServerPacketIds::LoginSuccess as i16;
        loop {
            let data = tokio::time::timeout(Duration::from_secs(2), rx.recv())
                .await
                .expect("login timeout")
                .expect("channel closed");
            if data.len() >= 4 && i16::from_le_bytes([data[2], data[3]]) == ok {
                break;
            }
        }
    }

    async fn new_character(
        gate_ref: &ActorRef<GateActor>,
        session_id: u64,
        rx: &mut RxChannel,
        name: &str,
    ) {
        let mut body = Vec::new();
        let _ = mir2_shared::binary::write_dotnet_string(&mut body, name);
        body.push(0u8); // gender = Male
        body.push(0u8); // class = Warrior
        let _ = gate_ref
            .ask(ClientData {
                session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ClientPacketIds::NewCharacter as i16,
                    &body,
                ),
            })
            .await;
        assert!(
            wait_opcode_body(
                rx,
                mir2_shared::enums::ServerPacketIds::NewCharacterSuccess as i16,
                3
            )
            .await
            .is_some(),
            "NewCharacterSuccess"
        );
    }

    async fn start_game(gate_ref: &ActorRef<GateActor>, session_id: u64, rx: &mut RxChannel) {
        let _ = gate_ref
            .ask(ClientData {
                session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ClientPacketIds::StartGame as i16,
                    &0i32.to_le_bytes().to_vec(),
                ),
            })
            .await;
        assert!(
            wait_opcode_body(rx, mir2_shared::enums::ServerPacketIds::StartGame as i16, 5)
                .await
                .is_some(),
            "StartGame"
        );
    }

    async fn spawn_world(gate_ref: &ActorRef<GateActor>, db_pool: &DbPool) -> ActorRef<WorldActor> {
        let social_ref = SocialActor::spawn(SocialActorArgs {
            gate_ref: gate_ref.clone(),
            db_pool: db_pool.clone(),
            config: SocialActorConfig::default(),
        });
        let world_ref = WorldActor::spawn(WorldActorArgs {
            tick_interval_ms: 1000,
            gate_ref: gate_ref.clone(),
            map_dir: std::path::PathBuf::from("."),
            spawn_dir: None,
            quest_dir: std::path::PathBuf::from("."),
            npc_script_dir: std::path::PathBuf::from("."),
            db_pool: db_pool.clone(),
            social_ref,
            conquest_cfg: crate::util::config::ConquestConfig::default(),
            rested_cfg: crate::util::config::RestedConfig::default(),
            pvp_cfg: crate::util::config::PvpConfig::default(),
            health_regen_weight: 10,
            mana_regen_weight: 10,
            goods_hide_added_stats: true,
            goods_on: true,
            goods_max_stored: 15,
            goods_buy_back_time_minutes: 60,
            goods_buy_back_max_stored: 20,
            safe_zone_healing: false,
            archive_inactive_after_months: 12,
            monster_recall_enabled: true,
            monster_recall_range: 12,
            monster_recall_cooldown_ms: 5000,
            exp_mob_level_difference: true,
            refine_cfg: crate::util::config::RefineConfig::default(),
            replace_wedring_cost: 125,
            lover_exp_bonus: 5,
            mentor_exp_boost: 10,
            mentor_damage_boost: 10,
            mentor_skill_boost: true,
            mentee_exp_bank: 1,
            orbs_exp_list: Vec::new(),
            orbs_dmg_list: Vec::new(),
            orbs_def_list: Vec::new(),
            awakening_cfg: Default::default(),
            gem_cfg: Default::default(),
            hero_exp_list: Vec::new(),
            setup_cfg: Default::default(),
            drop_rate: 1.0,
            exp_rate: 1.0,
            experience_list: Vec::new(),
            item_timeout_ticks: 300,
            max_drop_gold: 2000,
            drop_gold: true,
            rarity_cfg: crate::util::config::RarityConfig::default(),
            notice_path: "Notice.txt".to_string(),
            death_exp_penalty_percent: 0,
            movement_pacing_ms: 0,
            fishing_cfg: crate::util::ini::FishingConfig::default(),
            random_item_stats: Vec::new(),
            guild_buff_infos: Vec::new(),
        });
        let _ = gate_ref
            .ask(SetWorldRef {
                world_ref: world_ref.clone(),
            })
            .await;
        world_ref
    }

    /// 两张图：0 = 起点（脚本 NPC GateNpc，[@MAIN] 里 MOVE 1 20 20）；
    /// 1 = 目标（NPC DstNpc + 怪物 DstMob）
    async fn seed_two_maps(db_pool: &DbPool) {
        sqlx::query(
            "INSERT INTO map_infos (idx, file_name, title) VALUES (0, '0', 'SrcMap'), (1, '1', 'DstMap')",
        )
        .execute(db_pool)
        .await
        .expect("insert map_infos");
        sqlx::query(
            "INSERT INTO npc_infos (idx, map_index, file_name, name, x, y) \
             VALUES (1, 0, 'GateNpc', 'GateNpc', 10, 10), (2, 1, 'DstNpc', 'DstNpc', 5, 5)",
        )
        .execute(db_pool)
        .await
        .expect("insert npc_infos");
        sqlx::query(
            "INSERT INTO npc_scripts (npc_index, page_name, lines_json) \
             VALUES (1, '[@MAIN]', '[\"[@MAIN]\",\"#ACT\",\"MOVE 1 20 20\"]')",
        )
        .execute(db_pool)
        .await
        .expect("insert npc_scripts");
        sqlx::query("INSERT INTO monster_infos (idx, name, image) VALUES (1, 'DstMob', 1)")
            .execute(db_pool)
            .await
            .expect("insert monster_infos");
        sqlx::query(
            "INSERT INTO map_respawns (map_index, monster_index, x, y, count, spread) \
             VALUES (1, 1, 8, 8, 1, 0)",
        )
        .execute(db_pool)
        .await
        .expect("insert map_respawns");
    }

    /// 脚本传送（NPC 脚本 MOVE → npc_script::teleport_player）跨图后，
    /// 目标图 NPC/怪物 ObjectAdd 包必须全量重发——客户端换图重建已清旧图实体，
    /// 不重发落图即空图。
    ///
    /// 红检：删掉 npc_script.rs `teleport_player` 里的
    /// `world.resync_map_objects_for_session(...)` 调用 → MapChanged 之后等不到
    /// ObjectNpc/ObjectMonster，最后两个断言 FAILED（已实测）。
    #[test]
    fn e2e_script_move_teleport_resyncs_map_objects() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .thread_stack_size(8 * 1024 * 1024)
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async {
            let gate_ref = GateActor::spawn(());
            let session_id = 81u64;
            let (tx, mut rx) = mpsc::channel::<Vec<u8>>(1024);
            let _ = gate_ref
                .ask(SessionCreated {
                    session_id,
                    sender: tx.clone(),
                    ip: "127.0.0.1".to_string(),
                })
                .await;
            let _tx = tx;
            let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");
            let account_ref = AccountActor::spawn((gate_ref.clone(), db_pool.clone()));
            let _ = gate_ref.ask(SetAccountRef { account_ref }).await;
            login(&gate_ref, session_id, &mut rx, "mapsyncmove").await;
            seed_two_maps(&db_pool).await;
            let _world_ref = spawn_world(&gate_ref, &db_pool).await;

            new_character(&gate_ref, session_id, &mut rx, "MoveChar").await;
            // 站到脚本 NPC 旁（CallNPC 有 2 格距离校验；NPC 在 (10,10)）
            sqlx::query(
                "UPDATE characters SET map_index = 0, x = 11, y = 10 WHERE name = 'MoveChar'",
            )
            .execute(&db_pool)
            .await
            .expect("place character");
            start_game(&gate_ref, session_id, &mut rx).await;

            // 起点图 NPC（拿 object_id 供 CallNPC）
            let npc_body = wait_opcode_body(
                &mut rx,
                mir2_shared::enums::ServerPacketIds::ObjectNpc as i16,
                5,
            )
            .await
            .expect("起点图 ObjectNpc 未下发");
            let npc_oid = u32::from_le_bytes(npc_body[0..4].try_into().unwrap());

            // CallNPC [@MAIN] → 脚本 #ACT MOVE 1 20 20（teleport_player 跨图传送）
            let mut call_body = Vec::new();
            call_body.extend_from_slice(&npc_oid.to_le_bytes());
            let _ = mir2_shared::binary::write_dotnet_string(&mut call_body, "[@MAIN]");
            let _ = gate_ref
                .ask(ClientData {
                    session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ClientPacketIds::CallNPC as i16,
                        &call_body,
                    ),
                })
                .await;

            // 换图确认（teleport_player 自身就发，红/绿都应有）
            assert!(
                wait_opcode_body(
                    &mut rx,
                    mir2_shared::enums::ServerPacketIds::MapChanged as i16,
                    5
                )
                .await
                .is_some(),
                "MapChanged"
            );
            // 红检核心断言：目标图 NPC/怪物 ObjectAdd 必须在换图后全量重发
            assert!(
                wait_opcode_body(
                    &mut rx,
                    mir2_shared::enums::ServerPacketIds::ObjectNpc as i16,
                    5
                )
                .await
                .is_some(),
                "脚本传送后目标图 NPC 未重发（落图空图）"
            );
            assert!(
                wait_opcode_body(
                    &mut rx,
                    mir2_shared::enums::ServerPacketIds::ObjectMonster as i16,
                    5
                )
                .await
                .is_some(),
                "脚本传送后目标图怪物未重发（落图空图）"
            );
        });
    }

    /// GM @GOTO（mod.rs GmGotoRequest）跨图后同样必须全量重发新图对象，
    /// 且玩家互见（mover 看到目标图玩家、目标图玩家看到 mover）。
    ///
    /// 红检：删掉 mod.rs `GmGotoRequest` handler 里的
    /// `self.resync_map_objects_for_session(...)` 调用 → mover 收不到
    /// ObjectNpc/ObjectMonster/ObjectPlayer，断言 FAILED（已实测）。
    #[test]
    fn e2e_gm_goto_resyncs_map_objects() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .thread_stack_size(8 * 1024 * 1024)
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async {
            let gate_ref = GateActor::spawn(());
            let (tx_a, mut rx_a) = mpsc::channel::<Vec<u8>>(1024);
            let (tx_b, mut rx_b) = mpsc::channel::<Vec<u8>>(1024);
            let _ = gate_ref
                .ask(SessionCreated {
                    session_id: 82,
                    sender: tx_a.clone(),
                    ip: "127.0.0.1".to_string(),
                })
                .await;
            let _ = gate_ref
                .ask(SessionCreated {
                    session_id: 83,
                    sender: tx_b.clone(),
                    ip: "127.0.0.1".to_string(),
                })
                .await;
            let _tx_a = tx_a;
            let _tx_b = tx_b;
            let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");
            let account_ref = AccountActor::spawn((gate_ref.clone(), db_pool.clone()));
            let _ = gate_ref.ask(SetAccountRef { account_ref }).await;
            login(&gate_ref, 82, &mut rx_a, "mapsyncgm").await;
            login(&gate_ref, 83, &mut rx_b, "mapsyncanchor").await;
            seed_two_maps(&db_pool).await;
            let world_ref = spawn_world(&gate_ref, &db_pool).await;

            // A 在图 0，B（锚点）在图 1
            new_character(&gate_ref, 82, &mut rx_a, "GmMover").await;
            new_character(&gate_ref, 83, &mut rx_b, "GotoAnchor").await;
            sqlx::query(
                "UPDATE characters SET map_index = 0, x = 30, y = 30 WHERE name = 'GmMover'",
            )
            .execute(&db_pool)
            .await
            .expect("place A");
            sqlx::query(
                "UPDATE characters SET map_index = 1, x = 20, y = 21 WHERE name = 'GotoAnchor'",
            )
            .execute(&db_pool)
            .await
            .expect("place B");
            start_game(&gate_ref, 82, &mut rx_a).await;
            start_game(&gate_ref, 83, &mut rx_b).await;

            // GM @GOTO 到 B 身边（跨图 0 → 1）
            let _ = world_ref
                .ask(GmGotoRequest {
                    session_id: 82,
                    target_name: "GotoAnchor".to_string(),
                })
                .await;

            // mover 视角：换图 + 目标图 NPC/怪物/玩家全量重发
            assert!(
                wait_opcode_body(
                    &mut rx_a,
                    mir2_shared::enums::ServerPacketIds::MapChanged as i16,
                    5
                )
                .await
                .is_some(),
                "MapChanged"
            );
            assert!(
                wait_opcode_body(
                    &mut rx_a,
                    mir2_shared::enums::ServerPacketIds::ObjectNpc as i16,
                    5
                )
                .await
                .is_some(),
                "@GOTO 后目标图 NPC 未重发"
            );
            assert!(
                wait_opcode_body(
                    &mut rx_a,
                    mir2_shared::enums::ServerPacketIds::ObjectMonster as i16,
                    5
                )
                .await
                .is_some(),
                "@GOTO 后目标图怪物未重发"
            );
            assert!(
                wait_opcode_body(
                    &mut rx_a,
                    mir2_shared::enums::ServerPacketIds::ObjectPlayer as i16,
                    5
                )
                .await
                .is_some(),
                "@GOTO 后 mover 未收到目标图玩家 ObjectPlayer"
            );
            // 锚点视角：mover 出现在目标图
            assert!(
                wait_opcode_body(
                    &mut rx_b,
                    mir2_shared::enums::ServerPacketIds::ObjectPlayer as i16,
                    5
                )
                .await
                .is_some(),
                "@GOTO 后目标图玩家未收到 mover 的 ObjectPlayer"
            );
        });
    }

    /// 统一传送消息入口（TeleportPlayerWithResync，social 召回/外部 actor 路径）：
    /// 跨图必须 MapChanged + UserLocation + 目标图 NPC/怪物全量重发。
    ///
    /// 红检：摘掉 handler 里的 resync 调用 → ObjectNpc/ObjectMonster 等不到，断言 FAILED。
    #[test]
    fn e2e_teleport_message_cross_map_resyncs_map_objects() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .thread_stack_size(8 * 1024 * 1024)
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async {
            let gate_ref = GateActor::spawn(());
            let session_id = 84u64;
            let (tx, mut rx) = mpsc::channel::<Vec<u8>>(1024);
            let _ = gate_ref
                .ask(SessionCreated {
                    session_id,
                    sender: tx.clone(),
                    ip: "127.0.0.1".to_string(),
                })
                .await;
            let _tx = tx;
            let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");
            let account_ref = AccountActor::spawn((gate_ref.clone(), db_pool.clone()));
            let _ = gate_ref.ask(SetAccountRef { account_ref }).await;
            login(&gate_ref, session_id, &mut rx, "mapsyncmsg").await;
            seed_two_maps(&db_pool).await;
            let world_ref = spawn_world(&gate_ref, &db_pool).await;

            new_character(&gate_ref, session_id, &mut rx, "TpMsgChar").await;
            sqlx::query(
                "UPDATE characters SET map_index = 0, x = 15, y = 15 WHERE name = 'TpMsgChar'",
            )
            .execute(&db_pool)
            .await
            .expect("place character");
            start_game(&gate_ref, session_id, &mut rx).await;

            // 统一入口跨图传送 0 → 1
            let ok = world_ref
                .ask(TeleportPlayerWithResync {
                    session_id,
                    map_index: 1,
                    x: 20,
                    y: 20,
                    direction: 4,
                })
                .await
                .expect("TeleportPlayerWithResync ask");
            assert!(ok, "传送必须成功（玩家在线）");

            assert!(
                wait_opcode_body(
                    &mut rx,
                    mir2_shared::enums::ServerPacketIds::MapChanged as i16,
                    5
                )
                .await
                .is_some(),
                "跨图传送必须发 MapChanged"
            );
            assert!(
                wait_opcode_body(
                    &mut rx,
                    mir2_shared::enums::ServerPacketIds::UserLocation as i16,
                    5
                )
                .await
                .is_some(),
                "跨图传送必须发 UserLocation"
            );
            assert!(
                wait_opcode_body(
                    &mut rx,
                    mir2_shared::enums::ServerPacketIds::ObjectNpc as i16,
                    5
                )
                .await
                .is_some(),
                "统一入口跨图后目标图 NPC 未重发（落图空图）"
            );
            assert!(
                wait_opcode_body(
                    &mut rx,
                    mir2_shared::enums::ServerPacketIds::ObjectMonster as i16,
                    5
                )
                .await
                .is_some(),
                "统一入口跨图后目标图怪物未重发（落图空图）"
            );
        });
    }

    /// @move 聊天指令（session.rs MOVE 分支）同图传送也必须走 teleport_core
    /// 下发 UserLocation——否则服务端坐标已改、客户端不知情，位置脱同步且
    /// 断线存档会把传送后坐标落库（2026-09-17 实机冒烟：GM @move 100 100 后
    /// 收到"已传送至"系统消息但客户端坐标不变）。
    /// 红检：MOVE 分支改回裸 SetPlayerPosition → 无 UserLocation → FAILED。
    #[test]
    fn e2e_at_move_chat_same_map_sends_user_location() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .thread_stack_size(8 * 1024 * 1024)
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async {
            let gate_ref = GateActor::spawn(());
            let session_id = 86u64;
            let (tx, mut rx) = mpsc::channel::<Vec<u8>>(1024);
            let _ = gate_ref
                .ask(SessionCreated {
                    session_id,
                    sender: tx.clone(),
                    ip: "127.0.0.1".to_string(),
                })
                .await;
            let _tx = tx;
            let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");
            let account_ref = AccountActor::spawn((gate_ref.clone(), db_pool.clone()));
            let _ = gate_ref.ask(SetAccountRef { account_ref }).await;
            login(&gate_ref, session_id, &mut rx, "mapsyncmv").await;
            seed_two_maps(&db_pool).await;
            let world_ref = spawn_world(&gate_ref, &db_pool).await;

            new_character(&gate_ref, session_id, &mut rx, "MvChatChar").await;
            // GM 权限（@move 门槛）在账号列 admin_account；StartGame 加载时读
            sqlx::query("UPDATE accounts SET admin_account = 1 WHERE username = 'mapsyncmv'")
                .execute(&db_pool)
                .await
                .expect("grant gm");
            sqlx::query(
                "UPDATE characters SET map_index = 0, x = 15, y = 15 WHERE name = 'MvChatChar'",
            )
            .execute(&db_pool)
            .await
            .expect("place character");
            start_game(&gate_ref, session_id, &mut rx).await;

            // 排空登录/进图期间的存量包（含可能的 MapChanged/UserLocation）
            let drain_deadline = tokio::time::Instant::now() + Duration::from_millis(1500);
            while tokio::time::Instant::now() < drain_deadline {
                let remaining = drain_deadline - tokio::time::Instant::now();
                let _ = tokio::time::timeout(remaining, rx.recv()).await;
            }

            // 聊天发 @move 18 18（同图）：dotnet string + linked_items 计数 0
            let mut body = Vec::new();
            let _ = mir2_shared::binary::write_dotnet_string(&mut body, "@move 18 18");
            body.extend_from_slice(&0i32.to_le_bytes());
            let _ = gate_ref
                .ask(ClientData {
                    session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ClientPacketIds::Chat as i16,
                        &body,
                    ),
                })
                .await;

            let loc = wait_opcode_body(
                &mut rx,
                mir2_shared::enums::ServerPacketIds::UserLocation as i16,
                5,
            )
            .await
            .expect("@move 同图传送必须发 UserLocation（否则客户端坐标脱同步）");
            // 包体坐标必须是传送目标 (18,18)——堵"发了包但坐标错"的口子
            //（UserLocation body：x i32 LE + y i32 LE + dir u8）
            assert!(loc.len() >= 8, "UserLocation 包体过短: {}", loc.len());
            let lx = i32::from_le_bytes([loc[0], loc[1], loc[2], loc[3]]);
            let ly = i32::from_le_bytes([loc[4], loc[5], loc[6], loc[7]]);
            assert_eq!((lx, ly), (18, 18), "UserLocation 坐标须为传送目标");
            let _ = world_ref;
        });
    }

    /// 同图位移只发 UserLocation，不得发 MapChanged（客户端无换图重建，
    /// MapChanged 会触发多余的全图重建）。
    ///
    /// 红检：teleport_core 若退化为无条件发 MapChanged（teleport_player 旧行为）
    /// → 同图传送后 2s 内收到 MapChanged，断言 FAILED。
    #[test]
    fn e2e_teleport_message_same_map_sends_only_user_location() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .thread_stack_size(8 * 1024 * 1024)
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async {
            let gate_ref = GateActor::spawn(());
            let session_id = 85u64;
            let (tx, mut rx) = mpsc::channel::<Vec<u8>>(1024);
            let _ = gate_ref
                .ask(SessionCreated {
                    session_id,
                    sender: tx.clone(),
                    ip: "127.0.0.1".to_string(),
                })
                .await;
            let _tx = tx;
            let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");
            let account_ref = AccountActor::spawn((gate_ref.clone(), db_pool.clone()));
            let _ = gate_ref.ask(SetAccountRef { account_ref }).await;
            login(&gate_ref, session_id, &mut rx, "mapsyncsame").await;
            seed_two_maps(&db_pool).await;
            let world_ref = spawn_world(&gate_ref, &db_pool).await;

            new_character(&gate_ref, session_id, &mut rx, "TpSameChar").await;
            sqlx::query(
                "UPDATE characters SET map_index = 0, x = 15, y = 15 WHERE name = 'TpSameChar'",
            )
            .execute(&db_pool)
            .await
            .expect("place character");
            start_game(&gate_ref, session_id, &mut rx).await;

            // 排空登录/进图期间的存量包（含可能的 MapChanged/UserLocation）
            let drain_deadline = tokio::time::Instant::now() + Duration::from_millis(1500);
            while tokio::time::Instant::now() < drain_deadline {
                let remaining = drain_deadline - tokio::time::Instant::now();
                let _ = tokio::time::timeout(remaining, rx.recv()).await;
            }

            // 同图位移 0 → 0
            let ok = world_ref
                .ask(TeleportPlayerWithResync {
                    session_id,
                    map_index: 0,
                    x: 18,
                    y: 18,
                    direction: 2,
                })
                .await
                .expect("TeleportPlayerWithResync ask");
            assert!(ok);

            assert!(
                wait_opcode_body(
                    &mut rx,
                    mir2_shared::enums::ServerPacketIds::UserLocation as i16,
                    5
                )
                .await
                .is_some(),
                "同图位移必须发 UserLocation"
            );
            assert!(
                wait_opcode_body(
                    &mut rx,
                    mir2_shared::enums::ServerPacketIds::MapChanged as i16,
                    2
                )
                .await
                .is_none(),
                "同图位移不得发 MapChanged（客户端无换图重建，会触发多余重建）"
            );
        });
    }
}
