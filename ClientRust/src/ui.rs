use std::time::Instant;

use anyhow::Result;

use crate::{
    audio::AudioEngine,
    keybinds::KeyBindSettings,
    net::{NetworkEvent, NetworkStack},
    protocol::{
        parse_server_message, CharacterSummary, ClientVersionResult, LoginResult, ServerMessage,
        StartGameResult,
    },
    settings::ClientSettings,
    state::{ClientState, GroundObject, GroundObjectRemoval},
};
use mir2_shared::{
    client_packets::{ClientVersion, Login, StartGame},
    enums::MirAction,
};

/// Temporary text-mode loop that will be replaced with the real rendering stack.
pub async fn launch(
    settings: &ClientSettings,
    keybinds: &KeyBindSettings,
    _audio: AudioEngine,
    mut network: NetworkStack,
    version_hash: Vec<u8>,
) -> Result<()> {
    let resolution = settings.resolution();
    let (server, port) = settings.server_address();
    let account_id = settings.game.account_id.trim().to_string();
    let password = settings.game.password.clone();
    let credentials_configured = !account_id.is_empty() && !password.is_empty();

    tracing::info!(
        width = resolution.width,
        height = resolution.height,
        server = %server,
        port,
        keybinds = keybinds.len(),
        auto_login = credentials_configured,
        "starting Rust client placeholder UI"
    );

    let mut handshake_sent = false;
    let mut verified = false;
    let mut login_attempted = false;
    let mut login_completed = false;
    let mut start_game_sent = false;
    let mut game_entered = false;
    let mut character_summaries: Vec<CharacterSummary> = Vec::new();
    let mut state = ClientState::default();
    let mut last_tick = Instant::now();

    while let Some(event) = network.next_event().await {
        let now = Instant::now();
        let elapsed = now.saturating_duration_since(last_tick);
        let delta_ms = elapsed.as_millis().min(u128::from(u32::MAX)) as u32;
        if delta_ms > 0 {
            let summary = state.advance_animations(delta_ms);
            if summary.objects_updated > 0 {
                tracing::trace!(
                    objects_updated = summary.objects_updated,
                    frames_advanced = summary.frames_advanced,
                    cycles_completed = summary.cycles_completed,
                    delta_ms,
                    "advanced map object animations"
                );
            }
        }
        last_tick = now;

        match event {
            NetworkEvent::Connected => tracing::info!("connected to server"),
            NetworkEvent::Disconnected => {
                tracing::warn!("server disconnected");
                break;
            }
            NetworkEvent::Packet { header, payload } => {
                match parse_server_message(header, payload) {
                    ServerMessage::Connected => {
                        tracing::info!("received server handshake");
                        if !handshake_sent {
                            let packet = ClientVersion {
                                version_hash: version_hash.clone(),
                            };
                            match network.send_packet(&packet).await {
                                Ok(_) => {
                                    handshake_sent = true;
                                    tracing::debug!("sent client version hash");
                                }
                                Err(err) => {
                                    tracing::error!(
                                        error = %err,
                                        "failed to send client version"
                                    );
                                    break;
                                }
                            }
                        }
                    }
                    ServerMessage::ClientVersion { result } => match result {
                        ClientVersionResult::CorrectVersion => {
                            tracing::info!("client version verified");
                            verified = true;
                            if !login_attempted {
                                if !credentials_configured {
                                    tracing::warn!(
                                        "account credentials missing in settings; skipping auto-login"
                                    );
                                    login_attempted = true;
                                } else {
                                    let packet = Login {
                                        account_id: account_id.clone(),
                                        password: password.clone(),
                                    };
                                    match network.send_packet(&packet).await {
                                        Ok(_) => {
                                            login_attempted = true;
                                            tracing::info!(
                                                account_len = account_id.len(),
                                                "sent login request"
                                            );
                                        }
                                        Err(err) => {
                                            tracing::error!(
                                                error = %err,
                                                "failed to send login request"
                                            );
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        ClientVersionResult::WrongVersion => {
                            tracing::error!(
                                "server rejected client version; please update the client"
                            );
                            break;
                        }
                        ClientVersionResult::Other(code) => {
                            tracing::warn!(code, "server returned unknown client version code");
                        }
                    },
                    ServerMessage::Login { result } => match result {
                        LoginResult::Disabled => {
                            tracing::error!("logging in is currently disabled on the server");
                            break;
                        }
                        LoginResult::BadAccountId => {
                            tracing::warn!("server rejected the account id");
                            break;
                        }
                        LoginResult::BadPassword => {
                            tracing::warn!("server rejected the password");
                            break;
                        }
                        LoginResult::AccountNotExist => {
                            tracing::warn!("account does not exist");
                            break;
                        }
                        LoginResult::WrongPassword => {
                            tracing::warn!("incorrect account password");
                            break;
                        }
                        LoginResult::PasswordMustChange => {
                            tracing::warn!(
                                "password must be changed before logging in; aborting auto-login"
                            );
                            break;
                        }
                        LoginResult::Unknown(code) => {
                            tracing::warn!(code, "server returned unknown login result code");
                        }
                    },
                    ServerMessage::LoginBanned {
                        reason,
                        expiry_ticks,
                    } => {
                        tracing::error!(
                            reason = %reason,
                            expiry_ticks,
                            "account is banned"
                        );
                        break;
                    }
                    ServerMessage::LoginSuccess { characters } => {
                        login_completed = true;
                        let character_count = characters.len();
                        if character_count == 0 {
                            tracing::warn!(
                                "login succeeded but no characters were returned by the server"
                            );
                            character_summaries = characters;
                            break;
                        } else {
                            tracing::info!(character_count, "login succeeded");
                            for summary in &characters {
                                tracing::info!(
                                    index = summary.index,
                                    name = %summary.name,
                                    level = summary.level,
                                    class = ?summary.class,
                                    gender = ?summary.gender,
                                    last_access_ticks = summary.last_access_ticks,
                                    "available character"
                                );
                            }
                        }
                        character_summaries = characters;

                        if !start_game_sent {
                            if let Some(first) = character_summaries.first() {
                                let packet = StartGame {
                                    character_index: first.index,
                                };
                                match network.send_packet(&packet).await {
                                    Ok(_) => {
                                        start_game_sent = true;
                                        tracing::info!(
                                            index = first.index,
                                            name = %first.name,
                                            "requested game start for character"
                                        );
                                    }
                                    Err(err) => {
                                        tracing::error!(
                                            error = %err,
                                            "failed to send start game packet"
                                        );
                                        break;
                                    }
                                }
                            } else {
                                tracing::warn!(
                                    "no character available to start; awaiting further input"
                                );
                            }
                        }
                    }
                    ServerMessage::StartGame { result, resolution } => match result {
                        StartGameResult::Success => {
                            game_entered = true;
                            tracing::info!(resolution, "game start acknowledged by server");
                        }
                        StartGameResult::Disabled => {
                            tracing::error!("game start is currently disabled on the server");
                            break;
                        }
                        StartGameResult::NotLoggedIn => {
                            tracing::error!("cannot start game before logging in");
                            break;
                        }
                        StartGameResult::CharacterNotFound => {
                            tracing::error!("selected character was not found on the server");
                            break;
                        }
                        StartGameResult::NoStartPoint => {
                            tracing::error!("server could not find a spawn location");
                            break;
                        }
                        StartGameResult::Unknown(code) => {
                            tracing::warn!(code, "server returned unknown start game result");
                        }
                    },
                    ServerMessage::StartGameBanned {
                        reason,
                        expiry_ticks,
                    } => {
                        tracing::error!(
                            reason = %reason,
                            expiry_ticks,
                            "start game blocked: account banned"
                        );
                        break;
                    }
                    ServerMessage::StartGameDelay { milliseconds } => {
                        tracing::info!(milliseconds, "start game delayed by server");
                    }
                    ServerMessage::NewMapInfo(info) => {
                        let movement_count = info.info.movements.len();
                        let npc_count = info.info.npcs.len();
                        tracing::info!(
                            map_index = info.map_index,
                            title = %info.info.title,
                            width = info.info.width,
                            height = info.info.height,
                            big_map = info.info.big_map,
                            movements = movement_count,
                            npcs = npc_count,
                            "received detailed map setup"
                        );
                        state.update_map_details(info);
                    }
                    ServerMessage::MapInformation(info) => {
                        tracing::info!(
                            map_index = info.map_index,
                            file = %info.file_name,
                            title = %info.title,
                            mini_map = info.mini_map,
                            big_map = info.big_map,
                            lights = info.lights,
                            lightning = info.lightning,
                            fire = info.fire,
                            map_dark_light = info.map_dark_light,
                            music = info.music,
                            weather = ?info.weather,
                            "received map information"
                        );
                        state.update_map_information(info);
                    }
                    ServerMessage::WorldMapSetup(info) => {
                        let icon_count = info.setup.icons.len();
                        tracing::info!(
                            enabled = info.setup.enabled,
                            icon_count,
                            teleport_to_npc_cost = info.teleport_to_npc_cost,
                            "received world map setup"
                        );
                        state.update_world_map(info);
                    }
                    ServerMessage::SearchMapResult(result) => {
                        tracing::info!(
                            map_index = result.map_index,
                            npc_index = result.npc_index,
                            "received search map result"
                        );
                        state.update_search_map_result(result);
                    }
                    ServerMessage::UserInformation(info) => {
                        let inventory_slots = info
                            .inventory
                            .as_ref()
                            .map(|slots| slots.len())
                            .unwrap_or(0);
                        let equipment_slots = info
                            .equipment
                            .as_ref()
                            .map(|slots| slots.len())
                            .unwrap_or(0);
                        tracing::info!(
                            object_id = info.object_id,
                            name = %info.name,
                            level = info.level,
                            class = ?info.class,
                            gender = ?info.gender,
                            gold = info.gold,
                            credit = info.credit,
                            inventory_slots,
                            equipment_slots,
                            magic_count = info.magics.len(),
                            guild = %info.guild_name,
                            "received user information"
                        );
                        state.update_from_user_information(info);
                    }
                    ServerMessage::UserLocation(info) => {
                        tracing::info!(
                            x = info.location.x,
                            y = info.location.y,
                            direction = ?info.direction,
                            "received user location"
                        );
                        state.update_location(info);
                    }
                    ServerMessage::UserSlotsRefresh(slots) => {
                        state.update_inventory_slots(slots);
                        let summary = state.summary();
                        tracing::info!(
                            inventory_slots = summary.inventory_slots,
                            equipment_slots = summary.equipment_slots,
                            gold = summary.gold,
                            credit = summary.credit,
                            character = summary.character_name.as_deref().unwrap_or("(unknown)"),
                            map_index = ?summary.map_index,
                            map_title = summary.map_title.as_deref().unwrap_or("(unknown)"),
                            map_object_count = summary.map_object_count,
                            hero_object_count = summary.hero_object_count,
                            visible_player_count = summary.visible_player_count,
                            visible_hero_count = summary.visible_hero_count,
                            visible_monster_count = summary.visible_monster_count,
                            ground_object_count = summary.ground_object_count,
                            npc_count = summary.npc_count,
                            "received slot refresh"
                        );
                    }
                    ServerMessage::ObjectPlayer(object) => {
                        tracing::info!(
                            object_id = object.object_id,
                            name = %object.name,
                            guild = %object.guild_name,
                            level = object.level,
                            class = ?object.class,
                            x = object.location.x,
                            y = object.location.y,
                            direction = ?object.direction,
                            dead = object.dead,
                            hidden = object.hidden,
                            buffs = object.buffs.len(),
                            "received object player"
                        );
                        let outcome = state.upsert_player_object(object);
                        let buffs_added = outcome.sync.buff_delta.added.len();
                        let buffs_removed = outcome.sync.buff_delta.removed.len();
                        tracing::debug!(
                            created = outcome.created,
                            object_type = ?outcome.object_type,
                            action_before = ?outcome.sync.action_before,
                            action_after = ?outcome.sync.action_after,
                            action_changed = outcome.sync.action_changed(),
                            buffs_added,
                            buffs_removed,
                            buffs_changed = !outcome.sync.buff_delta.is_empty(),
                            "synced player object"
                        );
                    }
                    ServerMessage::ObjectHero(object) => {
                        tracing::info!(
                            object_id = object.player.object_id,
                            name = %object.player.name,
                            owner = %object.owner_name,
                            level = object.player.level,
                            class = ?object.player.class,
                            x = object.player.location.x,
                            y = object.player.location.y,
                            direction = ?object.player.direction,
                            dead = object.player.dead,
                            hidden = object.player.hidden,
                            buffs = object.player.buffs.len(),
                            "received object hero"
                        );
                        let outcome = state.upsert_hero_object(object);
                        let buffs_added = outcome.sync.buff_delta.added.len();
                        let buffs_removed = outcome.sync.buff_delta.removed.len();
                        tracing::debug!(
                            created = outcome.created,
                            object_type = ?outcome.object_type,
                            action_before = ?outcome.sync.action_before,
                            action_after = ?outcome.sync.action_after,
                            action_changed = outcome.sync.action_changed(),
                            buffs_added,
                            buffs_removed,
                            buffs_changed = !outcome.sync.buff_delta.is_empty(),
                            "synced hero object"
                        );
                    }
                    ServerMessage::ObjectMonster(object) => {
                        tracing::info!(
                            object_id = object.object_id,
                            name = %object.name,
                            level = 0,
                            image = object.image,
                            x = object.location.x,
                            y = object.location.y,
                            direction = ?object.direction,
                            dead = object.dead,
                            skeleton = object.skeleton,
                            hidden = object.hidden,
                            poison = ?object.poison,
                            ai = object.ai,
                            effect = object.effect,
                            light = object.light,
                            buffs = object.buffs.len(),
                            "received object monster"
                        );
                        let outcome = state.upsert_monster_object(object);
                        let buffs_added = outcome.sync.buff_delta.added.len();
                        let buffs_removed = outcome.sync.buff_delta.removed.len();
                        tracing::debug!(
                            created = outcome.created,
                            object_type = ?outcome.object_type,
                            action_before = ?outcome.sync.action_before,
                            action_after = ?outcome.sync.action_after,
                            action_changed = outcome.sync.action_changed(),
                            buffs_added,
                            buffs_removed,
                            buffs_changed = !outcome.sync.buff_delta.is_empty(),
                            "synced monster object"
                        );
                    }
                    ServerMessage::ObjectRemove(packet) => {
                        if let Some(object) = state.remove_object(packet.object_id) {
                            tracing::info!(
                                object_id = packet.object_id,
                                object_type = ?object.object_type(),
                                location = ?object.location(),
                                direction = ?object.direction(),
                                action = ?object.current_action(),
                                hidden = object.is_hidden(),
                                dead = object.is_dead(),
                                "removed map object"
                            );
                        } else if let Some(npc) = state.remove_npc(packet.object_id) {
                            tracing::info!(
                                object_id = packet.object_id,
                                name = %npc.name,
                                name_colour = npc.name_colour_argb,
                                image = npc.image,
                                colour = npc.colour_argb,
                                location = ?npc.location,
                                direction = ?npc.direction,
                                quest_count = npc.quest_ids.len(),
                                "removed npc object"
                            );
                        } else if let Some(GroundObjectRemoval { object, .. }) =
                            state.remove_ground_object(packet.object_id)
                        {
                            match object {
                                GroundObject::Item(entry) => {
                                    tracing::info!(
                                        object_id = packet.object_id,
                                        name = %entry.name,
                                        location = ?entry.location,
                                        "removed ground item"
                                    );
                                }
                                GroundObject::Gold(entry) => {
                                    tracing::info!(
                                        object_id = packet.object_id,
                                        amount = entry.amount,
                                        location = ?entry.location,
                                        "removed ground gold"
                                    );
                                }
                            }
                        } else {
                            tracing::debug!(
                                object_id = packet.object_id,
                                "received object removal for unknown id"
                            );
                        }
                    }
                    ServerMessage::ObjectTurn(packet) => {
                        let fallback_object_id = packet.object_id;
                        let fallback_direction = packet.direction;
                        let fallback_location = packet.location;
                        match state.apply_object_action(packet, MirAction::Standing) {
                            Some(outcome) => {
                                let result = outcome.result;
                                tracing::debug!(
                                    object_id = outcome.object_id,
                                    object_type = ?outcome.object_type,
                                    action_before = ?result.action_before,
                                    action_after = ?result.action_after,
                                    direction_before = ?result.direction_before,
                                    direction_after = ?result.direction_after,
                                    location_before = ?result.location_before,
                                    location_after = ?result.location_after,
                                    action_changed = result.action_changed,
                                    "object turn"
                                );
                            }
                            None => {
                                tracing::debug!(
                                    object_id = fallback_object_id,
                                    direction = ?fallback_direction,
                                    location = ?fallback_location,
                                    "object turn for unknown id"
                                );
                            }
                        }
                    }
                    ServerMessage::ObjectWalk(packet) => {
                        let fallback_object_id = packet.object_id;
                        let fallback_direction = packet.direction;
                        let fallback_location = packet.location;
                        match state.apply_object_action(packet, MirAction::Walking) {
                            Some(outcome) => {
                                let result = outcome.result;
                                let moved = result.moved();
                                tracing::debug!(
                                    object_id = outcome.object_id,
                                    object_type = ?outcome.object_type,
                                    action_before = ?result.action_before,
                                    action_after = ?result.action_after,
                                    direction_before = ?result.direction_before,
                                    direction_after = ?result.direction_after,
                                    location_before = ?result.location_before,
                                    location_after = ?result.location_after,
                                    action_changed = result.action_changed,
                                    moved,
                                    "object walk"
                                );
                            }
                            None => {
                                tracing::debug!(
                                    object_id = fallback_object_id,
                                    direction = ?fallback_direction,
                                    location = ?fallback_location,
                                    "object walk for unknown id"
                                );
                            }
                        }
                    }
                    ServerMessage::ObjectRun(packet) => {
                        let fallback_object_id = packet.object_id;
                        let fallback_direction = packet.direction;
                        let fallback_location = packet.location;
                        match state.apply_object_action(packet, MirAction::Running) {
                            Some(outcome) => {
                                let result = outcome.result;
                                let moved = result.moved();
                                tracing::debug!(
                                    object_id = outcome.object_id,
                                    object_type = ?outcome.object_type,
                                    action_before = ?result.action_before,
                                    action_after = ?result.action_after,
                                    direction_before = ?result.direction_before,
                                    direction_after = ?result.direction_after,
                                    location_before = ?result.location_before,
                                    location_after = ?result.location_after,
                                    action_changed = result.action_changed,
                                    moved,
                                    "object run"
                                );
                            }
                            None => {
                                tracing::debug!(
                                    object_id = fallback_object_id,
                                    direction = ?fallback_direction,
                                    location = ?fallback_location,
                                    "object run for unknown id"
                                );
                            }
                        }
                    }
                    ServerMessage::ObjectAttack(packet) => {
                        let fallback_object_id = packet.object_id;
                        let fallback_direction = packet.direction;
                        let fallback_location = packet.location;
                        let fallback_spell = packet.spell;
                        let fallback_level = packet.level;
                        let fallback_attack_type = packet.attack_type;
                        match state.apply_object_attack(packet) {
                            Some(outcome) => {
                                let object_id = outcome.object_id;
                                let object_type = outcome.object_type;
                                let attack = outcome.attack;
                                let transition = attack.transition;
                                let moved = transition.moved();
                                tracing::debug!(
                                    object_id,
                                    object_type = ?object_type,
                                    spell = ?attack.spell,
                                    level = attack.level,
                                    attack_type = attack.attack_type,
                                    action_before = ?transition.action_before,
                                    action_after = ?transition.action_after,
                                    direction_before = ?transition.direction_before,
                                    direction_after = ?transition.direction_after,
                                    location_before = ?transition.location_before,
                                    location_after = ?transition.location_after,
                                    action_changed = transition.action_changed,
                                    moved,
                                    "object attack"
                                );
                            }
                            None => {
                                tracing::debug!(
                                    object_id = fallback_object_id,
                                    direction = ?fallback_direction,
                                    location = ?fallback_location,
                                    spell = ?fallback_spell,
                                    level = fallback_level,
                                    attack_type = fallback_attack_type,
                                    "object attack for unknown id"
                                );
                            }
                        }
                    }
                    ServerMessage::Struck(info) => {
                        tracing::debug!(attacker_id = info.attacker_id, "player struck");
                    }
                    ServerMessage::ObjectStruck(packet) => {
                        let fallback_object_id = packet.object_id;
                        let fallback_attacker_id = packet.attacker_id;
                        let fallback_direction = packet.direction;
                        let fallback_location = packet.location;
                        match state.apply_object_struck(packet) {
                            Some(outcome) => {
                                let object_id = outcome.object_id;
                                let object_type = outcome.object_type;
                                let struck = outcome.struck;
                                let transition = struck.transition;
                                let moved = transition.moved();
                                tracing::debug!(
                                    object_id,
                                    object_type = ?object_type,
                                    attacker_id = struck.attacker_id,
                                    action_before = ?transition.action_before,
                                    action_after = ?transition.action_after,
                                    direction_before = ?transition.direction_before,
                                    direction_after = ?transition.direction_after,
                                    location_before = ?transition.location_before,
                                    location_after = ?transition.location_after,
                                    action_changed = transition.action_changed,
                                    moved,
                                    "object struck"
                                );
                            }
                            None => {
                                tracing::debug!(
                                    object_id = fallback_object_id,
                                    attacker_id = fallback_attacker_id,
                                    direction = ?fallback_direction,
                                    location = ?fallback_location,
                                    "object struck for unknown id"
                                );
                            }
                        }
                    }
                    ServerMessage::ObjectItem(packet) => {
                        let outcome = state.spawn_object_item(packet);
                        match outcome.object {
                            GroundObject::Item(entry) => {
                                tracing::info!(
                                    object_id = outcome.object_id,
                                    name = %entry.name,
                                    name_colour_argb = entry.name_colour_argb,
                                    location = ?entry.location,
                                    image = entry.image,
                                    grade = ?entry.grade,
                                    "ground item spawned"
                                );
                            }
                            GroundObject::Gold(_) => {}
                        }
                    }
                    ServerMessage::ObjectGold(packet) => {
                        let outcome = state.spawn_object_gold(packet);
                        match outcome.object {
                            GroundObject::Gold(entry) => {
                                tracing::info!(
                                    object_id = outcome.object_id,
                                    amount = entry.amount,
                                    location = ?entry.location,
                                    "ground gold spawned"
                                );
                            }
                            GroundObject::Item(_) => {}
                        }
                    }
                    ServerMessage::ColourChanged(packet) => {
                        let event = state.apply_colour_changed(packet);
                        tracing::info!(
                            name_colour_argb = event.name_colour_argb,
                            "player name colour updated"
                        );
                    }
                    ServerMessage::ObjectColourChanged(packet) => {
                        let event = state.apply_object_colour_changed(packet);
                        tracing::info!(
                            object_id = event.object_id,
                            object_type = ?event.object_type,
                            previous_colour = ?event.previous_colour,
                            new_colour = event.new_colour,
                            "object name colour updated"
                        );
                    }
                    ServerMessage::ObjectGuildNameChanged(packet) => {
                        let event = state.apply_object_guild_name_changed(packet);
                        tracing::info!(
                            object_id = event.object_id,
                            object_type = ?event.object_type,
                            previous_guild = event.previous_guild_name.as_deref(),
                            new_guild = %event.new_guild_name,
                            "object guild name updated"
                        );
                    }
                    ServerMessage::GainExperience(packet) => {
                        let event = state.apply_gain_experience(packet);
                        tracing::info!(
                            amount = event.amount,
                            new_total = ?event.new_experience_total,
                            max_experience = ?event.max_experience,
                            "player experience gained"
                        );
                    }
                    ServerMessage::GainHeroExperience(packet) => {
                        let event = state.apply_gain_hero_experience(packet);
                        tracing::info!(
                            amount = event.amount,
                            new_total = ?event.new_experience_total,
                            max_experience = ?event.max_experience,
                            "hero experience gained"
                        );
                    }
                    ServerMessage::LevelChanged(packet) => {
                        let event = state.apply_level_changed(packet);
                        tracing::info!(
                            level = event.level,
                            experience = event.experience,
                            max_experience = event.max_experience,
                            "player level changed"
                        );
                    }
                    ServerMessage::HeroLevelChanged(packet) => {
                        let event = state.apply_hero_level_changed(packet);
                        tracing::info!(
                            level = event.level,
                            experience = event.experience,
                            max_experience = event.max_experience,
                            "hero level changed"
                        );
                    }
                    ServerMessage::ObjectLeveled(packet) => {
                        let event = state.apply_object_leveled(packet);
                        tracing::info!(
                            object_id = event.object_id,
                            object_type = ?event.object_type,
                            is_player = event.is_player,
                            is_hero = event.is_hero,
                            "map object leveled"
                        );
                    }
                    ServerMessage::ObjectHarvest(packet) => {
                        let fallback_object_id = packet.object_id;
                        let fallback_direction = packet.direction;
                        let fallback_location = packet.location;
                        match state.apply_object_harvest(packet) {
                            Some(outcome) => {
                                let result = outcome.result;
                                tracing::debug!(
                                    object_id = outcome.object_id,
                                    object_type = ?outcome.object_type,
                                    action_before = ?result.action_before,
                                    action_after = ?result.action_after,
                                    direction_before = ?result.direction_before,
                                    direction_after = ?result.direction_after,
                                    location_before = ?result.location_before,
                                    location_after = ?result.location_after,
                                    action_changed = result.action_changed,
                                    "object harvest action"
                                );
                            }
                            None => {
                                tracing::debug!(
                                    object_id = fallback_object_id,
                                    direction = ?fallback_direction,
                                    location = ?fallback_location,
                                    "object harvest for unknown id"
                                );
                            }
                        }
                    }
                    ServerMessage::ObjectHarvested(packet) => {
                        let fallback_object_id = packet.object_id;
                        let fallback_direction = packet.direction;
                        let fallback_location = packet.location;
                        match state.apply_object_harvested(packet) {
                            Some(outcome) => {
                                let result = outcome.result;
                                tracing::debug!(
                                    object_id = outcome.object_id,
                                    object_type = ?outcome.object_type,
                                    action_before = ?result.action_before,
                                    action_after = ?result.action_after,
                                    direction_before = ?result.direction_before,
                                    direction_after = ?result.direction_after,
                                    location_before = ?result.location_before,
                                    location_after = ?result.location_after,
                                    action_changed = result.action_changed,
                                    "object harvested animation"
                                );
                            }
                            None => {
                                tracing::debug!(
                                    object_id = fallback_object_id,
                                    direction = ?fallback_direction,
                                    location = ?fallback_location,
                                    "object harvested for unknown id"
                                );
                            }
                        }
                    }
                    ServerMessage::ObjectNpc(packet) => {
                        let outcome = state.upsert_npc(packet);
                        tracing::info!(
                            object_id = outcome.npc.object_id,
                            created = outcome.created,
                            name = %outcome.npc.name,
                            name_colour = outcome.npc.name_colour_argb,
                            image = outcome.npc.image,
                            colour = outcome.npc.colour_argb,
                            location = ?outcome.npc.location,
                            direction = ?outcome.npc.direction,
                            quest_count = outcome.npc.quest_ids.len(),
                            "received npc object"
                        );
                    }
                    ServerMessage::NpcResponse(packet) => {
                        let event = state.apply_npc_response(packet);
                        tracing::info!(line_count = event.line_count, "received npc dialogue page");
                        for (index, line) in event.page.iter().enumerate() {
                            tracing::debug!(index, text = %line, "npc dialogue line");
                        }
                    }
                    ServerMessage::Chat(packet) => {
                        tracing::info!(
                            chat_type = ?packet.chat_type,
                            message = %packet.message,
                            "received chat message"
                        );
                    }
                    ServerMessage::ObjectChat(packet) => {
                        if let Some(object) = state.objects.get(&packet.object_id) {
                            tracing::info!(
                                object_id = packet.object_id,
                                object_type = ?object.object_type(),
                                chat_type = ?packet.chat_type,
                                text = %packet.text,
                                "object chat message"
                            );
                        } else {
                            tracing::debug!(
                                object_id = packet.object_id,
                                chat_type = ?packet.chat_type,
                                text = %packet.text,
                                "chat from unknown object"
                            );
                        }
                    }
                    ServerMessage::Magic(packet) => {
                        tracing::info!(
                            spell = ?packet.spell,
                            target_id = packet.target_id,
                            target = ?packet.target,
                            cast = packet.cast,
                            level = packet.level,
                            secondary_targets = packet.secondary_target_ids.len(),
                            "player magic cast"
                        );
                    }
                    ServerMessage::MagicDelay(packet) => {
                        tracing::debug!(
                            object_id = packet.object_id,
                            spell = ?packet.spell,
                            delay = packet.delay,
                            "magic delay"
                        );
                    }
                    ServerMessage::MagicCast(packet) => {
                        tracing::info!(
                            spell = ?packet.spell,
                            "magic cast confirmed"
                        );
                    }
                    ServerMessage::ObjectMagic(packet) => {
                        if let Some(object) = state.objects.get(&packet.object_id) {
                            tracing::info!(
                                object_id = packet.object_id,
                                object_type = ?object.object_type(),
                                spell = ?packet.spell,
                                target_id = packet.target_id,
                                target = ?packet.target,
                                location = ?packet.location,
                                direction = ?packet.direction,
                                cast = packet.cast,
                                level = packet.level,
                                self_broadcast = packet.self_broadcast,
                                secondary_targets = packet.secondary_target_ids.len(),
                                "object magic cast"
                            );
                        } else {
                            tracing::debug!(
                                object_id = packet.object_id,
                                spell = ?packet.spell,
                                "magic from unknown object"
                            );
                        }
                    }
                    ServerMessage::ObjectEffect(packet) => {
                        if let Some(object) = state.objects.get(&packet.object_id) {
                            tracing::info!(
                                object_id = packet.object_id,
                                object_type = ?object.object_type(),
                                effect = ?packet.effect,
                                effect_type = packet.effect_type,
                                delay_time = packet.delay_time,
                                time = packet.time,
                                "object effect"
                            );
                        } else {
                            tracing::debug!(
                                object_id = packet.object_id,
                                effect = ?packet.effect,
                                "effect on unknown object"
                            );
                        }
                    }
                    ServerMessage::ObjectProjectile(packet) => {
                        tracing::info!(
                            spell = ?packet.spell,
                            source = packet.source,
                            destination = packet.destination,
                            "object projectile"
                        );
                    }
                    ServerMessage::RangeAttack(packet) => {
                        tracing::info!(
                            target_id = packet.target_id,
                            target = ?packet.target,
                            spell = ?packet.spell,
                            "player range attack"
                        );
                    }
                    ServerMessage::MoveItem(packet) => {
                        tracing::info!(
                            grid = ?packet.grid,
                            from = packet.from,
                            to = packet.to,
                            success = packet.success,
                            "move item"
                        );
                    }
                    ServerMessage::EquipItem(packet) => {
                        tracing::info!(
                            grid = ?packet.grid,
                            unique_id = packet.unique_id,
                            to = packet.to,
                            success = packet.success,
                            "equip item"
                        );
                    }
                    ServerMessage::MergeItem(packet) => {
                        tracing::info!(
                            grid_from = ?packet.grid_from,
                            grid_to = ?packet.grid_to,
                            id_from = packet.id_from,
                            id_to = packet.id_to,
                            success = packet.success,
                            "merge item"
                        );
                    }
                    ServerMessage::RemoveItem(packet) => {
                        tracing::info!(
                            grid = ?packet.grid,
                            unique_id = packet.unique_id,
                            to = packet.to,
                            success = packet.success,
                            "remove item"
                        );
                    }
                    ServerMessage::RemoveSlotItem(packet) => {
                        tracing::info!(
                            grid = ?packet.grid,
                            grid_to = ?packet.grid_to,
                            unique_id = packet.unique_id,
                            to = packet.to,
                            success = packet.success,
                            "remove slot item"
                        );
                    }
                    ServerMessage::TakeBackItem(packet) => {
                        tracing::info!(
                            from = packet.from,
                            to = packet.to,
                            success = packet.success,
                            "take back item"
                        );
                    }
                    ServerMessage::StoreItem(packet) => {
                        tracing::info!(
                            from = packet.from,
                            to = packet.to,
                            success = packet.success,
                            "store item"
                        );
                    }
                    ServerMessage::UseItem(packet) => {
                        tracing::info!(
                            unique_id = packet.unique_id,
                            success = packet.success,
                            grid = ?packet.grid,
                            "use item"
                        );
                    }
                    ServerMessage::DropItem(packet) => {
                        tracing::info!(
                            unique_id = packet.unique_id,
                            count = packet.count,
                            hero_item = packet.hero_item,
                            success = packet.success,
                            "drop item"
                        );
                    }
                    ServerMessage::Pushed(packet) => {
                        tracing::info!(
                            location = ?packet.location,
                            direction = ?packet.direction,
                            "player pushed"
                        );
                    }
                    ServerMessage::ObjectPushed(packet) => {
                        if state.objects.contains_key(&packet.object_id) {
                            tracing::info!(
                                object_id = packet.object_id,
                                location = ?packet.location,
                                direction = ?packet.direction,
                                "object pushed"
                            );
                        } else {
                            tracing::debug!(
                                object_id = packet.object_id,
                                "received ObjectPushed for unknown object"
                            );
                        }
                    }
                    ServerMessage::AddBuff(packet) => {
                        tracing::info!(
                            buff_type = ?packet.buff.buff_type,
                            object_id = packet.buff.object_id,
                            visible = packet.buff.visible,
                            expire_time = packet.buff.expire_time,
                            infinite = packet.buff.infinite,
                            paused = packet.buff.paused,
                            values_count = packet.buff.values.len(),
                            "add buff"
                        );
                    }
                    ServerMessage::RemoveBuff(packet) => {
                        tracing::info!(
                            buff_type = ?packet.buff_type,
                            object_id = packet.object_id,
                            "remove buff"
                        );
                    }
                    ServerMessage::PauseBuff(packet) => {
                        tracing::info!(
                            buff_type = ?packet.buff_type,
                            object_id = packet.object_id,
                            paused = packet.paused,
                            "pause buff"
                        );
                    }
                    ServerMessage::TradeRequest(packet) => {
                        tracing::info!(name = %packet.name, "trade request");
                    }
                    ServerMessage::TradeAccept(packet) => {
                        tracing::info!(name = %packet.name, "trade accept");
                    }
                    ServerMessage::TradeGold(packet) => {
                        tracing::info!(amount = packet.amount, "trade gold");
                    }
                    ServerMessage::TradeItem(packet) => {
                        tracing::info!(items_count = packet.trade_items.len(), "trade item");
                    }
                    ServerMessage::TradeConfirm(_) => {
                        tracing::info!("trade confirm");
                    }
                    ServerMessage::TradeCancel(packet) => {
                        tracing::info!(unlock = packet.unlock, "trade cancel");
                    }
                    ServerMessage::GroupInvite(packet) => {
                        tracing::info!(name = %packet.name, "group invite");
                    }
                    ServerMessage::AddMember(packet) => {
                        tracing::info!(name = %packet.name, "add member");
                    }
                    ServerMessage::DeleteGroup(_) => {
                        tracing::info!("delete group");
                    }
                    ServerMessage::DeleteMember(packet) => {
                        tracing::info!(name = %packet.name, "delete member");
                    }
                    ServerMessage::ShareQuest(packet) => {
                        tracing::info!(
                            quest_index = packet.quest_index,
                            sharer_name = %packet.sharer_name,
                            "share quest"
                        );
                    }
                    ServerMessage::CompleteQuest(packet) => {
                        tracing::info!(
                            completed_count = packet.completed_quests.len(),
                            "complete quest"
                        );
                    }
                    ServerMessage::NPCGoods(packet) => {
                        tracing::info!(
                            items_count = packet.list.len(),
                            rate = packet.rate,
                            panel_type = packet.panel_type,
                            hide_added_stats = packet.hide_added_stats,
                            "npc goods"
                        );
                    }
                    ServerMessage::ObjectHide(packet) => {
                        if state.objects.contains_key(&packet.object_id) {
                            tracing::info!(object_id = packet.object_id, "object hide");
                        } else {
                            tracing::debug!(
                                object_id = packet.object_id,
                                "received ObjectHide for unknown object"
                            );
                        }
                    }
                    ServerMessage::ObjectShow(packet) => {
                        if state.objects.contains_key(&packet.object_id) {
                            tracing::info!(object_id = packet.object_id, "object show");
                        } else {
                            tracing::debug!(
                                object_id = packet.object_id,
                                "received ObjectShow for unknown object"
                            );
                        }
                    }
                    ServerMessage::Poisoned(packet) => {
                        tracing::info!(poison = ?packet.poison, "player poisoned");
                    }
                    ServerMessage::ObjectPoisoned(packet) => {
                        if state.objects.contains_key(&packet.object_id) {
                            tracing::info!(
                                object_id = packet.object_id,
                                poison = ?packet.poison,
                                "object poisoned"
                            );
                        } else {
                            tracing::debug!(
                                object_id = packet.object_id,
                                "received ObjectPoisoned for unknown object"
                            );
                        }
                    }
                    ServerMessage::Revived(_) => {
                        tracing::info!("player revived");
                    }
                    ServerMessage::ObjectRevived(packet) => {
                        if state.objects.contains_key(&packet.object_id) {
                            tracing::info!(
                                object_id = packet.object_id,
                                effect = packet.effect,
                                "object revived"
                            );
                        } else {
                            tracing::debug!(
                                object_id = packet.object_id,
                                "received ObjectRevived for unknown object"
                            );
                        }
                    }
                    ServerMessage::GuildInvite(packet) => {
                        tracing::info!(
                            name = %packet.name,
                            "guild invite received"
                        );
                    }
                    ServerMessage::GuildStatus(packet) => {
                        tracing::info!(
                            guild_name = %packet.guild_name,
                            rank_name = %packet.guild_rank_name,
                            level = packet.level,
                            experience = packet.experience,
                            max_experience = packet.max_experience,
                            gold = packet.gold,
                            member_count = packet.member_count,
                            max_members = packet.max_members,
                            "guild status updated"
                        );
                    }
                    ServerMessage::GuildStorageGoldChange(packet) => {
                        tracing::info!(
                            amount = packet.amount,
                            change_type = packet.change_type,
                            name = %packet.name,
                            "guild storage gold changed"
                        );
                    }
                    ServerMessage::GuildStorageItemChange(packet) => {
                        tracing::info!(
                            change_type = packet.change_type,
                            to = packet.to,
                            from = packet.from,
                            user = packet.user,
                            user_id = ?packet.user_id,
                            has_item = packet.item.is_some(),
                            "guild storage item changed"
                        );
                    }
                    ServerMessage::NewHero(packet) => {
                        tracing::info!(result = packet.result, "new hero result");
                    }
                    ServerMessage::HeroInformation(packet) => {
                        let inventory_count =
                            packet.inventory.as_ref().map(|inv| inv.len()).unwrap_or(0);
                        let equipment_count =
                            packet.equipment.as_ref().map(|eq| eq.len()).unwrap_or(0);
                        tracing::info!(
                            object_id = packet.object_id,
                            name = %packet.name,
                            class = ?packet.class,
                            level = packet.level,
                            hp = packet.hp,
                            mp = packet.mp,
                            inventory_slots = inventory_count,
                            equipment_slots = equipment_count,
                            magic_count = packet.magics.len(),
                            auto_pot = packet.auto_pot,
                            "hero information received"
                        );
                    }
                    ServerMessage::MapChanged(packet) => {
                        tracing::info!(
                            map_index = packet.map_index,
                            file_name = %packet.file_name,
                            title = %packet.title,
                            mini_map = packet.mini_map,
                            big_map = packet.big_map,
                            lights = packet.lights,
                            location = ?packet.location,
                            direction = ?packet.direction,
                            map_dark_light = packet.map_dark_light,
                            music = packet.music,
                            "map changed"
                        );
                    }
                    ServerMessage::ObjectTeleportOut(packet) => {
                        if state.objects.contains_key(&packet.object_id) {
                            tracing::info!(
                                object_id = packet.object_id,
                                teleport_type = packet.teleport_type,
                                "object teleported out"
                            );
                        } else {
                            tracing::debug!(
                                object_id = packet.object_id,
                                "received ObjectTeleportOut for unknown object"
                            );
                        }
                    }
                    ServerMessage::ObjectTeleportIn(packet) => {
                        if state.objects.contains_key(&packet.object_id) {
                            tracing::info!(
                                object_id = packet.object_id,
                                teleport_type = packet.teleport_type,
                                "object teleported in"
                            );
                        } else {
                            tracing::debug!(
                                object_id = packet.object_id,
                                "received ObjectTeleportIn for unknown object"
                            );
                        }
                    }
                    ServerMessage::TeleportIn(_packet) => {
                        tracing::info!("player teleported in");
                    }
                    ServerMessage::ObjectRangeAttack(packet) => {
                        if state.objects.contains_key(&packet.object_id) {
                            tracing::info!(
                                object_id = packet.object_id,
                                location = ?packet.location,
                                direction = ?packet.direction,
                                target_id = packet.target_id,
                                target = ?packet.target,
                                spell = ?packet.spell,
                                level = packet.level,
                                "object range attack"
                            );
                        } else {
                            tracing::debug!(
                                object_id = packet.object_id,
                                "received ObjectRangeAttack for unknown object"
                            );
                        }
                    }
                    ServerMessage::UserDash(packet) => {
                        tracing::info!(
                            location = ?packet.location,
                            direction = ?packet.direction,
                            "user dash"
                        );
                    }
                    ServerMessage::ObjectDash(packet) => {
                        if state.objects.contains_key(&packet.object_id) {
                            tracing::info!(
                                object_id = packet.object_id,
                                location = ?packet.location,
                                direction = ?packet.direction,
                                "object dash"
                            );
                        } else {
                            tracing::debug!(
                                object_id = packet.object_id,
                                "received ObjectDash for unknown object"
                            );
                        }
                    }
                    ServerMessage::UserDashFail(packet) => {
                        tracing::info!(
                            location = ?packet.location,
                            direction = ?packet.direction,
                            "user dash failed"
                        );
                    }
                    ServerMessage::ObjectDashFail(packet) => {
                        if state.objects.contains_key(&packet.object_id) {
                            tracing::info!(
                                object_id = packet.object_id,
                                location = ?packet.location,
                                direction = ?packet.direction,
                                "object dash failed"
                            );
                        } else {
                            tracing::debug!(
                                object_id = packet.object_id,
                                "received ObjectDashFail for unknown object"
                            );
                        }
                    }
                    // NPC System - TODO: Implement full handlers in Step 4
                    ServerMessage::NPCSell(_) => tracing::trace!("NPC sell"),
                    ServerMessage::NPCRepair(p) => {
                        state.set_npc_rate(p.rate);
                        tracing::trace!(rate = p.rate, "NPC repair dialog opened");
                    }
                    ServerMessage::NPCSRepair(p) => {
                        tracing::trace!(rate = p.rate, "NPC special repair")
                    }
                    ServerMessage::NPCRefine(p) => {
                        tracing::trace!(rate = p.rate, refining = p.refining, "NPC refine")
                    }
                    ServerMessage::NPCCheckRefine(_) => tracing::trace!("NPC check refine"),
                    ServerMessage::NPCCollectRefine(p) => {
                        tracing::trace!(success = p.success, "NPC collect refine")
                    }
                    ServerMessage::NPCReplaceWedRing(p) => {
                        tracing::trace!(rate = p.rate, "NPC replace wedding ring")
                    }
                    ServerMessage::NPCStorage(_) => tracing::trace!("NPC storage"),
                    ServerMessage::NPCRequestInput(p) => {
                        tracing::info!(message = %p.message, max_length = p.max_length, "NPC request input")
                    }
                    // Item System - TODO: Implement full handlers in Step 4
                    ServerMessage::SellItem(p) => tracing::debug!(
                        unique_id = p.unique_id,
                        count = p.count,
                        success = p.success,
                        "sell item"
                    ),
                    ServerMessage::RepairItem(p) => {
                        tracing::debug!(unique_id = p.unique_id, "repair item")
                    }
                    ServerMessage::ItemRepaired(p) => tracing::debug!(
                        unique_id = p.unique_id,
                        max_dura = p.max_dura,
                        current_dura = p.current_dura,
                        "item repaired"
                    ),
                    ServerMessage::SplitItem(p) => {
                        tracing::debug!(grid = ?p.grid, unique_id = p.unique_id, count = p.count, "split item")
                    }
                    ServerMessage::SplitItem1(p) => {
                        tracing::debug!(grid = ?p.grid, unique_id = p.unique_id, count = p.count, "split item1")
                    }
                    ServerMessage::RefreshItem(p) => {
                        tracing::debug!(item_index = p.item.item_index, "refresh item")
                    }
                    ServerMessage::ItemSlotSizeChanged(p) => tracing::debug!(
                        unique_id = p.unique_id,
                        slot_size = p.slot_size,
                        "item slot size changed"
                    ),
                    ServerMessage::ItemSealChanged(p) => tracing::debug!(
                        unique_id = p.unique_id,
                        expiry_date = p.expiry_date,
                        "item seal changed"
                    ),
                    ServerMessage::CraftItem(p) => tracing::debug!(
                        unique_id = p.unique_id,
                        count = p.count,
                        success = p.success,
                        "craft item"
                    ),
                    ServerMessage::NewItemInfo(p) => {
                        tracing::debug!(item_index = p.info.index, name = %p.info.name, "new item info")
                    }
                    // Magic System
                    ServerMessage::NewMagic(p) => {
                        state.add_magic(p.magic.clone(), p.hero);
                        tracing::info!(
                            spell = ?p.magic.spell,
                            level = p.magic.level,
                            hero = p.hero,
                            "learned new magic"
                        );
                    }
                    ServerMessage::MagicLeveled(p) => {
                        state.level_magic(p.spell, p.level, 0, p.hero);
                        tracing::info!(
                            spell = ?p.spell,
                            level = p.level,
                            hero = p.hero,
                            "magic leveled up"
                        );
                    }
                    ServerMessage::RemoveMagic(p) => {
                        // Find and remove magic by spell type
                        let magics = if p.hero {
                            &state.hero_magics
                        } else {
                            &state.player_magics
                        };
                        if let Some(pos) = magics.iter().position(|m| m.spell == p.spell) {
                            state.remove_magic(pos as u8, p.hero);
                        }
                        tracing::info!(
                            spell = ?p.spell,
                            hero = p.hero,
                            "removed magic"
                        );
                    }
                    ServerMessage::SpellToggle(p) => {
                        state.toggle_spell(p.spell, p.hero);
                        tracing::debug!(
                            spell = ?p.spell,
                            can_use = p.can_use,
                            hero = p.hero,
                            "toggled spell"
                        );
                    }
                    // Player Status - TODO: Implement full handlers in Step 4
                    ServerMessage::PlayerUpdate(p) => tracing::trace!(
                        object_id = p.object_id,
                        light = p.light,
                        weapon = p.weapon,
                        "player update"
                    ),
                    ServerMessage::PlayerInspect(p) => {
                        tracing::info!(name = %p.name, guild = %p.guild_name, level = p.level, "player inspect")
                    }
                    ServerMessage::LogOutSuccess(p) => {
                        let char_count = p.characters.len();
                        state.store_logout_characters(p.characters);
                        tracing::info!(
                            characters = char_count,
                            "logout successful, returning to character selection"
                        );
                        // TODO: Transition to SelectScene
                    }
                    ServerMessage::TimeOfDay(p) => {
                        use mir2_shared::enums::LightSetting;
                        if let Ok(setting) = LightSetting::try_from(p.lights) {
                            state.set_light_setting(setting);
                            tracing::trace!(lights = ?setting, "time of day changed");
                        } else {
                            tracing::warn!(lights = p.lights, "unknown light setting value");
                        }
                    }
                    ServerMessage::ChangeAMode(p) => {
                        state.set_attack_mode(p.mode);
                        tracing::info!(mode = ?p.mode, "attack mode changed");
                    }
                    ServerMessage::ChangePMode(p) => {
                        state.set_pet_mode(p.mode);
                        tracing::info!(mode = ?p.mode, "pet mode changed");
                    }
                    ServerMessage::ObjectName(p) => {
                        state.log_object_name(p.object_id, &p.name);
                    }
                    ServerMessage::UserStorage(p) => {
                        let item_count = p.storage.len();
                        state.update_storage(p.storage);
                        tracing::info!(items = item_count, "storage updated");
                    }
                    // Object Status - TODO: Implement full handlers in Step 4
                    ServerMessage::ObjectHealth(p) => {
                        let hp = (p.percent as u32 * 100) / 100; // Convert percent to value
                        state.log_object_health(p.object_id, hp, 0);
                    }
                    ServerMessage::ObjectMana(p) => {
                        let mp = (p.percent as u32 * 100) / 100; // Convert percent to value
                        state.log_object_mana(p.object_id, mp);
                    }
                    ServerMessage::ObjectHidden(p) => {
                        state.log_object_hidden(p.object_id, p.hidden);
                    }
                    ServerMessage::MapEffect(p) => {
                        tracing::debug!(location = ?p.location, effect = ?p.effect, "map effect")
                    }
                    // Group System - TODO: Implement full handlers in Step 4
                    ServerMessage::SwitchGroup(p) => {
                        tracing::info!(allow_group = p.allow_group, "switch group")
                    }
                    ServerMessage::GroupMembersMap(p) => {
                        tracing::info!(members = p.members.len(), "group members map")
                    }
                    ServerMessage::SendMemberLocation(p) => {
                        tracing::trace!(member = %p.member_name, location = ?p.location, "send member location")
                    }
                    // Guild System - TODO: Implement full handlers in Step 4
                    ServerMessage::GuildStorageList(p) => {
                        tracing::info!(items = p.items.len(), "guild storage list")
                    }
                    ServerMessage::GuildNoticeChange(p) => tracing::info!(
                        lines = p.notice.len(),
                        update = p.update,
                        "guild notice change"
                    ),
                    ServerMessage::GuildMemberChange(p) => {
                        tracing::info!(name = %p.name, status = p.status, ranks = p.ranks.len(), "guild member change")
                    }
                    // Hero System - TODO: Implement full handlers in Step 4
                    ServerMessage::UpdateHeroSpawnState(p) => {
                        state.set_hero_spawn_state(p.state);
                        tracing::info!(state = ?p.state, "hero spawn state updated");
                    }
                    ServerMessage::SetAutoPotValue(p) => {
                        tracing::debug!(stat = p.stat, value = p.value, "set auto pot value")
                    }
                    ServerMessage::SetHeroBehaviour(p) => {
                        tracing::info!(attack_mode = ?p.attack_mode, pet_mode = ?p.pet_mode, "set hero behaviour")
                    }
                    ServerMessage::ManageHeroes(p) => {
                        tracing::info!(heroes = p.heroes.len(), "manage heroes")
                    }
                    ServerMessage::HeroCreateRequest(p) => {
                        tracing::info!(can_create = p.can_create_class.len(), "hero create request")
                    }
                    // Quest System - TODO: Implement full handlers in Step 4
                    ServerMessage::ChangeQuest(p) => {
                        state.update_quest(p.quest.clone());
                        tracing::info!(
                            quest_id = p.quest.id,
                            taken = p.quest.taken,
                            completed = p.quest.completed,
                            "quest status changed"
                        );
                    }
                    ServerMessage::NewQuestInfo(p) => {
                        tracing::info!(quest_id = p.quest.index, name = %p.quest.name, "new quest info")
                    }
                    // Account/Character - TODO: Implement full handlers in Step 4
                    ServerMessage::NewCharacter(p) => {
                        tracing::info!(result = p.result, "new character")
                    }
                    ServerMessage::NewCharacterSuccess(p) => {
                        tracing::info!(name = %p.character.name, level = p.character.level, "new character success")
                    }
                    ServerMessage::DeleteCharacter(p) => {
                        tracing::info!(result = p.result, "delete character")
                    }
                    ServerMessage::DeleteCharacterSuccess(p) => {
                        tracing::info!(character_id = p.character_id, "delete character success")
                    }
                    ServerMessage::GainedItem(packet) => {
                        let item_name = packet
                            .item
                            .info
                            .as_ref()
                            .map(|info| info.friendly_name())
                            .unwrap_or_else(|| format!("item({})", packet.item.item_index));
                        let event = state.apply_gained_item(packet.item);
                        tracing::info!(
                            container = ?event.container,
                            slot = event.slot_index,
                            item_index = event.item.item_index,
                            count = event.item.count,
                            name = %item_name,
                            "inventory item gained"
                        );
                    }
                    ServerMessage::GainedQuestItem(packet) => {
                        let item_name = packet
                            .item
                            .info
                            .as_ref()
                            .map(|info| info.friendly_name())
                            .unwrap_or_else(|| format!("item({})", packet.item.item_index));
                        let event = state.apply_gained_quest_item(packet.item);
                        tracing::info!(
                            container = ?event.container,
                            slot = event.slot_index,
                            item_index = event.item.item_index,
                            count = event.item.count,
                            name = %item_name,
                            "quest item gained"
                        );
                    }
                    ServerMessage::GainedGold(packet) => {
                        let event = state.apply_gained_gold(packet.gold);
                        tracing::info!(
                            amount = packet.gold,
                            change = event.change,
                            total = event.new_total,
                            "gold increased"
                        );
                    }
                    ServerMessage::LoseGold(packet) => {
                        let event = state.apply_lose_gold(packet.gold);
                        tracing::info!(
                            amount = packet.gold,
                            change = event.change,
                            total = event.new_total,
                            "gold decreased"
                        );
                    }
                    ServerMessage::GainedCredit(packet) => {
                        let event = state.apply_gained_credit(packet.credit);
                        tracing::info!(
                            amount = packet.credit,
                            change = event.change,
                            total = event.new_total,
                            "credit increased"
                        );
                    }
                    ServerMessage::LoseCredit(packet) => {
                        let event = state.apply_lose_credit(packet.credit);
                        tracing::info!(
                            amount = packet.credit,
                            change = event.change,
                            total = event.new_total,
                            "credit decreased"
                        );
                    }
                    ServerMessage::DamageIndicator(packet) => {
                        let outcome = state.apply_damage_indicator(packet);
                        let event = outcome.event;
                        if let Some(object_type) = event.object_type {
                            tracing::debug!(
                                object_id = event.object_id,
                                object_type = ?object_type,
                                damage = event.damage,
                                damage_type = ?event.damage_type,
                                "damage indicator"
                            );
                        } else {
                            tracing::debug!(
                                object_id = event.object_id,
                                damage = event.damage,
                                damage_type = ?event.damage_type,
                                "damage indicator for unknown id"
                            );
                        }
                    }
                    ServerMessage::DuraChanged(packet) => {
                        let event = state.apply_dura_changed(packet);
                        if let Some(container) = event.location {
                            tracing::info!(
                                unique_id = event.unique_id,
                                current_dura = event.current_dura,
                                container = ?container,
                                depleted = event.current_dura == 0,
                                "item durability updated"
                            );
                        } else {
                            tracing::info!(
                                unique_id = event.unique_id,
                                current_dura = event.current_dura,
                                depleted = event.current_dura == 0,
                                "item durability update for unknown item"
                            );
                        }
                    }
                    ServerMessage::DeleteItem(packet) => {
                        let event = state.apply_delete_item(packet);
                        tracing::info!(
                            unique_id = event.unique_id,
                            removed_count = event.removed_count,
                            remaining = ?event.remaining_count,
                            removed_completely = event.removed_completely,
                            container = ?event.location,
                            "item removed"
                        );
                    }
                    ServerMessage::DeleteQuestItem(packet) => {
                        let event = state.apply_delete_quest_item(packet);
                        tracing::info!(
                            unique_id = event.unique_id,
                            removed_count = event.removed_count,
                            remaining = ?event.remaining_count,
                            removed_completely = event.removed_completely,
                            container = ?event.location,
                            "quest item removed"
                        );
                    }
                    ServerMessage::Death(packet) => {
                        let event = state.apply_player_death(packet);
                        tracing::info!(
                            location = ?event.location,
                            direction = ?event.direction,
                            "player death"
                        );
                    }
                    ServerMessage::ObjectDied(packet) => {
                        let fallback_object_id = packet.object_id;
                        let fallback_location = packet.location;
                        let fallback_direction = packet.direction;
                        let fallback_death_type = packet.death_type;
                        match state.apply_object_died(packet) {
                            Some(outcome) => {
                                if outcome.removed {
                                    tracing::debug!(
                                        object_id = outcome.object_id,
                                        object_type = ?outcome.object_type,
                                        death_type = outcome.death_type,
                                        location = ?outcome.location,
                                        direction = ?outcome.direction,
                                        "object died and removed"
                                    );
                                } else if let Some(transition) = outcome.transition {
                                    let moved = transition.moved();
                                    tracing::debug!(
                                        object_id = outcome.object_id,
                                        object_type = ?outcome.object_type,
                                        death_type = outcome.death_type,
                                        action_before = ?transition.action_before,
                                        action_after = ?transition.action_after,
                                        direction_before = ?transition.direction_before,
                                        direction_after = ?transition.direction_after,
                                        location_before = ?transition.location_before,
                                        location_after = ?transition.location_after,
                                        action_changed = transition.action_changed,
                                        moved,
                                        "object death transition"
                                    );
                                }
                            }
                            None => {
                                tracing::debug!(
                                    object_id = fallback_object_id,
                                    location = ?fallback_location,
                                    direction = ?fallback_direction,
                                    death_type = fallback_death_type,
                                    "object death for unknown id"
                                );
                            }
                        }
                    }
                    ServerMessage::HealthChanged(packet) => {
                        let event = state.apply_health_changed(packet);
                        tracing::info!(
                            hp = event.hp,
                            mp = event.mp,
                            dead = event.hp <= 0,
                            "player health updated"
                        );
                    }
                    ServerMessage::HeroHealthChanged(packet) => {
                        let event = state.apply_hero_health_changed(packet);
                        tracing::info!(hp = event.hp, mp = event.mp, "hero health updated");
                    }
                    ServerMessage::Disconnect { reason } => {
                        tracing::warn!(reason, "server requested disconnect");
                        break;
                    }
                    ServerMessage::KeepAlive { time } => {
                        if verified {
                            tracing::trace!(time, "keep-alive");
                        } else {
                            tracing::debug!(time, "keep-alive (pre-auth)");
                        }
                    }
                    ServerMessage::Unknown { opcode, payload } => {
                        tracing::debug!(opcode, size = payload.len(), "unhandled server packet");
                    }
                    ServerMessage::ParseError { opcode, message } => {
                        tracing::warn!(opcode, "failed to parse server packet: {message}");
                    }
                }

                if game_entered
                    && state.map_details.is_some()
                    && state.character.is_some()
                    && state.location.is_some()
                {
                    tracing::info!(
                        "received core bootstrap packets (map, character, location); exiting placeholder loop"
                    );
                    break;
                }
            }
            NetworkEvent::Error(err) => {
                tracing::error!(error = %err, "network error");
                break;
            }
        }
    }

    if let (Some(map), Some(user), Some(position)) =
        (&state.map_details, &state.character, &state.location)
    {
        tracing::info!(
            map_index = map.map_index,
            map_title = %map.info.title,
            character = %user.name,
            level = user.level,
            class = ?user.class,
            x = position.location.x,
            y = position.location.y,
            "terminating after collecting user bootstrap data (game scene stub not implemented)"
        );
    } else if let Some(info) = &state.map_details {
        tracing::info!(
            map_index = info.map_index,
            title = %info.info.title,
            width = info.info.width,
            height = info.info.height,
            "terminating after receiving full map setup (game scene stub not implemented)"
        );
    } else if let Some(info) = &state.map_information {
        tracing::info!(
            map_index = info.map_index,
            title = %info.title,
            "terminating after receiving initial map metadata (game scene stub not implemented)"
        );
    } else if game_entered {
        tracing::info!(
            "game start acknowledged but in-game scene is not yet implemented; exiting placeholder UI"
        );
    } else if login_completed {
        tracing::info!(
            character_count = character_summaries.len(),
            "character selection UI not yet implemented; terminating after successful login"
        );
    }

    let summary = state.summary();
    tracing::debug!(
        character = summary.character_name.as_deref().unwrap_or("(unknown)"),
        map_index = ?summary.map_index,
        map_title = summary.map_title.as_deref().unwrap_or("(unknown)"),
        location = ?summary.location,
        inventory_slots = summary.inventory_slots,
        equipment_slots = summary.equipment_slots,
        level = ?summary.level,
        experience = ?summary.experience,
        max_experience = ?summary.max_experience,
        world_map_enabled = ?summary.world_map_enabled,
        world_map_icon_count = summary.world_map_icon_count,
        teleport_to_npc_cost = ?summary.teleport_to_npc_cost,
        last_search_map = ?summary.last_search_map,
        last_search_npc = ?summary.last_search_npc,
        gold = summary.gold,
        credit = summary.credit,
        hero_level = ?summary.hero_level,
        hero_experience = ?summary.hero_experience,
        hero_max_experience = ?summary.hero_max_experience,
        map_object_count = summary.map_object_count,
        hero_object_count = summary.hero_object_count,
        visible_player_count = summary.visible_player_count,
        visible_hero_count = summary.visible_hero_count,
        visible_monster_count = summary.visible_monster_count,
        ground_object_count = summary.ground_object_count,
        npc_count = summary.npc_count,
        "final client summary"
    );

    Ok(())
}
