// SQLite persistence layer
// Saves/loads accounts, characters, inventory, guilds, friends, mail, quests, creatures, refine

use std::path::Path;

use sqlx::{SqlitePool, Row};
use tracing::info;

use crate::actors::account::AccountInfo;
use crate::actors::player::PlayerState;
use crate::actors::inventory::PlayerInventory;
use crate::actors::friend::{FriendList, FriendEntry, BlockedEntry};
use crate::actors::mail::{Mailbox, MailMessage};
use crate::actors::quest::{QuestLog, QuestInstance, QuestProgress, QuestStatus};
use crate::actors::guild::{Guild, GuildMember, GuildRank};
use crate::actors::creature::{CreatureLog, IntelligentCreature, CreatureType, PickupMode};
use crate::actors::refine::{RefineLog, RefiningItem, RefineStatus};

pub type DbPool = SqlitePool;

/// Initialize the SQLite database and run migrations
pub async fn init_db(db_path: &Path) -> anyhow::Result<DbPool> {
    let db_url = format!("sqlite://{}", db_path.display());
    let pool = SqlitePool::connect(&db_url).await?;

    // Create tables if not exists
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS accounts (
            username TEXT PRIMARY KEY,
            password_hash TEXT NOT NULL,
            is_online INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS characters (
            name TEXT PRIMARY KEY,
            account_username TEXT NOT NULL,
            schema_version INTEGER NOT NULL DEFAULT 1,
            class INTEGER NOT NULL DEFAULT 0,
            gender INTEGER NOT NULL DEFAULT 0,
            hair INTEGER NOT NULL DEFAULT 0,
            map_index INTEGER NOT NULL DEFAULT 0,
            x INTEGER NOT NULL DEFAULT 0,
            y INTEGER NOT NULL DEFAULT 0,
            direction INTEGER NOT NULL DEFAULT 0,
            attack_mode TEXT NOT NULL DEFAULT 'Peace',
            pet_mode TEXT NOT NULL DEFAULT 'Both',
            level INTEGER NOT NULL DEFAULT 1,
            experience INTEGER NOT NULL DEFAULT 0,
            max_experience INTEGER NOT NULL DEFAULT 100,
            hp INTEGER NOT NULL DEFAULT 120,
            max_hp INTEGER NOT NULL DEFAULT 120,
            mp INTEGER NOT NULL DEFAULT 60,
            max_mp INTEGER NOT NULL DEFAULT 60,
            min_attack INTEGER NOT NULL DEFAULT 5,
            max_attack INTEGER NOT NULL DEFAULT 10,
            defence INTEGER NOT NULL DEFAULT 2,
            gold INTEGER NOT NULL DEFAULT 0,
            group_id INTEGER,
            guild_name TEXT,
            guild_rank INTEGER DEFAULT 2,
            spouse_name TEXT,
            allow_mentor INTEGER NOT NULL DEFAULT 0,
            mentor_name TEXT,
            hero_index INTEGER NOT NULL DEFAULT 0,
            is_fishing INTEGER NOT NULL DEFAULT 0,
            fishing_autocast INTEGER NOT NULL DEFAULT 0,
            is_dead INTEGER NOT NULL DEFAULT 0,
            pk_points INTEGER NOT NULL DEFAULT 0,
            pk_kill_count INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (account_username) REFERENCES accounts(username),
            FOREIGN KEY (guild_name) REFERENCES guilds(name)
        );
        CREATE TABLE IF NOT EXISTS inventory_backpack (
            character_name TEXT NOT NULL,
            grid INTEGER NOT NULL,
            item_json TEXT NOT NULL,
            PRIMARY KEY (character_name, grid),
            FOREIGN KEY (character_name) REFERENCES characters(name)
        );
        CREATE TABLE IF NOT EXISTS inventory_equipment (
            character_name TEXT NOT NULL,
            slot INTEGER NOT NULL,
            item_json TEXT NOT NULL,
            PRIMARY KEY (character_name, slot),
            FOREIGN KEY (character_name) REFERENCES characters(name)
        );
        CREATE TABLE IF NOT EXISTS inventory_storage (
            character_name TEXT NOT NULL,
            grid INTEGER NOT NULL,
            item_json TEXT NOT NULL,
            PRIMARY KEY (character_name, grid),
            FOREIGN KEY (character_name) REFERENCES characters(name)
        );
        CREATE TABLE IF NOT EXISTS hero_inventory_backpack (
            character_name TEXT NOT NULL,
            grid INTEGER NOT NULL,
            item_json TEXT NOT NULL,
            PRIMARY KEY (character_name, grid),
            FOREIGN KEY (character_name) REFERENCES characters(name)
        );
        CREATE TABLE IF NOT EXISTS friends (
            character_name TEXT NOT NULL,
            friend_object_id INTEGER NOT NULL,
            friend_name TEXT NOT NULL,
            memo TEXT NOT NULL DEFAULT '',
            PRIMARY KEY (character_name, friend_object_id),
            FOREIGN KEY (character_name) REFERENCES characters(name)
        );
        CREATE TABLE IF NOT EXISTS blocked_list (
            character_name TEXT NOT NULL,
            blocked_object_id INTEGER NOT NULL,
            blocked_name TEXT NOT NULL,
            PRIMARY KEY (character_name, blocked_object_id),
            FOREIGN KEY (character_name) REFERENCES characters(name)
        );
        CREATE TABLE IF NOT EXISTS mail (
            character_name TEXT NOT NULL,
            mail_id INTEGER PRIMARY KEY,
            sender_name TEXT NOT NULL,
            subject TEXT NOT NULL,
            body TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            read_flag INTEGER NOT NULL DEFAULT 0,
            collected INTEGER NOT NULL DEFAULT 0,
            locked INTEGER NOT NULL DEFAULT 0,
            gold INTEGER NOT NULL DEFAULT 0,
            items_json TEXT NOT NULL DEFAULT '[]',
            FOREIGN KEY (character_name) REFERENCES characters(name)
        );
        CREATE TABLE IF NOT EXISTS quests (
            character_name TEXT NOT NULL,
            quest_index INTEGER NOT NULL,
            title TEXT NOT NULL,
            status TEXT NOT NULL,
            progress_json TEXT NOT NULL DEFAULT '[]',
            exp_reward INTEGER NOT NULL DEFAULT 0,
            gold_reward INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (character_name, quest_index),
            FOREIGN KEY (character_name) REFERENCES characters(name)
        );
        CREATE TABLE IF NOT EXISTS completed_quests (
            character_name TEXT NOT NULL,
            quest_index INTEGER NOT NULL,
            PRIMARY KEY (character_name, quest_index),
            FOREIGN KEY (character_name) REFERENCES characters(name)
        );
        CREATE TABLE IF NOT EXISTS guilds (
            name TEXT PRIMARY KEY,
            notice_json TEXT NOT NULL DEFAULT '[]',
            gold INTEGER NOT NULL DEFAULT 0,
            storage_items_json TEXT NOT NULL DEFAULT '[]'
        );
        CREATE TABLE IF NOT EXISTS guild_members (
            guild_name TEXT NOT NULL,
            member_name TEXT NOT NULL,
            rank INTEGER NOT NULL DEFAULT 2,
            PRIMARY KEY (guild_name, member_name),
            FOREIGN KEY (guild_name) REFERENCES guilds(name)
        );
        CREATE TABLE IF NOT EXISTS creatures (
            character_name TEXT PRIMARY KEY,
            active_type INTEGER NOT NULL DEFAULT 0,
            active_custom_name TEXT,
            active_pickup_mode INTEGER NOT NULL DEFAULT 0,
            active_hunger INTEGER NOT NULL DEFAULT 100,
            active_enabled INTEGER NOT NULL DEFAULT 0,
            owned_json TEXT NOT NULL DEFAULT '[]',
            request_updates INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (character_name) REFERENCES characters(name)
        );
        CREATE TABLE IF NOT EXISTS refine_log (
            character_name TEXT PRIMARY KEY,
            active_original_uid INTEGER,
            active_item_index INTEGER NOT NULL DEFAULT 0,
            active_start_time INTEGER NOT NULL DEFAULT 0,
            active_finish_time INTEGER NOT NULL DEFAULT 0,
            active_status INTEGER NOT NULL DEFAULT 0,
            active_success_chance INTEGER NOT NULL DEFAULT 0,
            total_refines INTEGER NOT NULL DEFAULT 0,
            successful_refines INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (character_name) REFERENCES characters(name)
        );
        -- Game config tables (migrated from Server.MirDB)
        CREATE TABLE IF NOT EXISTS map_infos (
            index INTEGER PRIMARY KEY,
            file_name TEXT NOT NULL,
            title TEXT NOT NULL,
            mini_map INTEGER NOT NULL DEFAULT 0,
            light INTEGER NOT NULL DEFAULT 0,
            big_map INTEGER NOT NULL DEFAULT 0,
            no_teleport INTEGER NOT NULL DEFAULT 0,
            no_reconnect INTEGER NOT NULL DEFAULT 0,
            no_reconnect_map TEXT,
            no_random INTEGER NOT NULL DEFAULT 0,
            no_escape INTEGER NOT NULL DEFAULT 0,
            no_recall INTEGER NOT NULL DEFAULT 0,
            no_drug INTEGER NOT NULL DEFAULT 0,
            no_position INTEGER NOT NULL DEFAULT 0,
            no_throw_item INTEGER NOT NULL DEFAULT 0,
            no_drop_player INTEGER NOT NULL DEFAULT 0,
            no_drop_monster INTEGER NOT NULL DEFAULT 0,
            no_names INTEGER NOT NULL DEFAULT 0,
            fight INTEGER NOT NULL DEFAULT 0,
            fire INTEGER NOT NULL DEFAULT 0,
            fire_damage INTEGER NOT NULL DEFAULT 0,
            lightning INTEGER NOT NULL DEFAULT 0,
            lightning_damage INTEGER NOT NULL DEFAULT 0,
            map_dark_light INTEGER NOT NULL DEFAULT 0,
            mine_index INTEGER NOT NULL DEFAULT 0,
            no_mount INTEGER NOT NULL DEFAULT 0,
            need_bridle INTEGER NOT NULL DEFAULT 0,
            no_fight INTEGER NOT NULL DEFAULT 0,
            music INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS safe_zones (
            map_index INTEGER NOT NULL,
            x INTEGER NOT NULL,
            y INTEGER NOT NULL,
            size INTEGER NOT NULL DEFAULT 0,
            start_point INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (map_index, x, y),
            FOREIGN KEY (map_index) REFERENCES map_infos(index)
        );
        CREATE TABLE IF NOT EXISTS map_respawns (
            map_index INTEGER NOT NULL,
            monster_index INTEGER NOT NULL,
            x INTEGER NOT NULL,
            y INTEGER NOT NULL,
            count INTEGER NOT NULL DEFAULT 0,
            spread INTEGER NOT NULL DEFAULT 0,
            delay INTEGER NOT NULL DEFAULT 0,
            direction INTEGER NOT NULL DEFAULT 0,
            route_path TEXT,
            random_delay INTEGER NOT NULL DEFAULT 0,
            respawn_index INTEGER NOT NULL DEFAULT 0,
            save_respawn_time INTEGER NOT NULL DEFAULT 0,
            respawn_ticks INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (map_index) REFERENCES map_infos(index)
        );
        CREATE TABLE IF NOT EXISTS map_movements (
            map_index INTEGER NOT NULL,
            source_x INTEGER NOT NULL,
            source_y INTEGER NOT NULL,
            dest_x INTEGER NOT NULL,
            dest_y INTEGER NOT NULL,
            need_hole INTEGER NOT NULL DEFAULT 0,
            need_move INTEGER NOT NULL DEFAULT 0,
            conquest_index INTEGER NOT NULL DEFAULT 0,
            show_on_big_map INTEGER NOT NULL DEFAULT 0,
            icon INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (map_index) REFERENCES map_infos(index)
        );
        CREATE TABLE IF NOT EXISTS mine_zones (
            map_index INTEGER NOT NULL,
            x INTEGER NOT NULL,
            y INTEGER NOT NULL,
            size INTEGER NOT NULL DEFAULT 0,
            mine INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (map_index) REFERENCES map_infos(index)
        );
        CREATE TABLE IF NOT EXISTS item_infos (
            index INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            type INTEGER NOT NULL DEFAULT 0,
            grade INTEGER NOT NULL DEFAULT 0,
            required_type INTEGER NOT NULL DEFAULT 0,
            required_class INTEGER NOT NULL DEFAULT 0,
            required_gender INTEGER NOT NULL DEFAULT 0,
            set_type INTEGER NOT NULL DEFAULT 0,
            shape INTEGER NOT NULL DEFAULT 0,
            weight INTEGER NOT NULL DEFAULT 0,
            light INTEGER NOT NULL DEFAULT 0,
            required_amount INTEGER NOT NULL DEFAULT 0,
            image INTEGER NOT NULL DEFAULT 0,
            durability INTEGER NOT NULL DEFAULT 0,
            stack_size INTEGER NOT NULL DEFAULT 0,
            price INTEGER NOT NULL DEFAULT 0,
            start_item INTEGER NOT NULL DEFAULT 0,
            effect INTEGER NOT NULL DEFAULT 0,
            bool_flags INTEGER NOT NULL DEFAULT 0,
            bind_mode INTEGER NOT NULL DEFAULT 0,
            special_mode INTEGER NOT NULL DEFAULT 0,
            random_stats_id INTEGER NOT NULL DEFAULT 0,
            can_fast_run INTEGER NOT NULL DEFAULT 0,
            can_awakening INTEGER NOT NULL DEFAULT 0,
            slots INTEGER NOT NULL DEFAULT 0,
            stats_json TEXT NOT NULL DEFAULT '{}',
            has_tool_tip INTEGER NOT NULL DEFAULT 0,
            tool_tip TEXT
        );
        CREATE TABLE IF NOT EXISTS monster_infos (
            index INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            image INTEGER NOT NULL DEFAULT 0,
            ai INTEGER NOT NULL DEFAULT 0,
            effect INTEGER NOT NULL DEFAULT 0,
            level INTEGER NOT NULL DEFAULT 0,
            view_range INTEGER NOT NULL DEFAULT 0,
            cool_eye INTEGER NOT NULL DEFAULT 0,
            stats_json TEXT NOT NULL DEFAULT '{}',
            light INTEGER NOT NULL DEFAULT 0,
            attack_speed INTEGER NOT NULL DEFAULT 0,
            move_speed INTEGER NOT NULL DEFAULT 0,
            experience INTEGER NOT NULL DEFAULT 0,
            can_push INTEGER NOT NULL DEFAULT 0,
            can_tame INTEGER NOT NULL DEFAULT 0,
            auto_rev INTEGER NOT NULL DEFAULT 0,
            undead INTEGER NOT NULL DEFAULT 0,
            drop_path TEXT
        );
        CREATE TABLE IF NOT EXISTS npc_infos (
            index INTEGER PRIMARY KEY,
            map_index INTEGER NOT NULL,
            file_name TEXT NOT NULL,
            name TEXT NOT NULL,
            x INTEGER NOT NULL,
            y INTEGER NOT NULL,
            image INTEGER NOT NULL DEFAULT 0,
            rate INTEGER NOT NULL DEFAULT 0,
            time_visible INTEGER NOT NULL DEFAULT 0,
            hour_start INTEGER NOT NULL DEFAULT 0,
            minute_start INTEGER NOT NULL DEFAULT 0,
            hour_end INTEGER NOT NULL DEFAULT 0,
            minute_end INTEGER NOT NULL DEFAULT 0,
            min_lev INTEGER NOT NULL DEFAULT 0,
            max_lev INTEGER NOT NULL DEFAULT 0,
            day_of_week TEXT,
            class_required TEXT,
            conquest INTEGER NOT NULL DEFAULT 0,
            flag_needed INTEGER NOT NULL DEFAULT 0,
            show_on_big_map INTEGER NOT NULL DEFAULT 0,
            big_map_icon INTEGER NOT NULL DEFAULT 0,
            can_teleport_to INTEGER NOT NULL DEFAULT 0,
            conquest_visible INTEGER NOT NULL DEFAULT 0,
            collect_quest_indexes TEXT NOT NULL DEFAULT '[]',
            finish_quest_indexes TEXT NOT NULL DEFAULT '[]'
        );
        CREATE TABLE IF NOT EXISTS quest_infos (
            index INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            group_name TEXT NOT NULL,
            file_name TEXT NOT NULL,
            required_min_level INTEGER NOT NULL DEFAULT 0,
            required_max_level INTEGER NOT NULL DEFAULT 0,
            required_quest INTEGER NOT NULL DEFAULT 0,
            required_class INTEGER NOT NULL DEFAULT 0,
            quest_type INTEGER NOT NULL DEFAULT 0,
            exp_reward INTEGER NOT NULL DEFAULT 0,
            gold_reward INTEGER NOT NULL DEFAULT 0,
            goto_message TEXT,
            kill_message TEXT,
            item_message TEXT,
            flag_message TEXT,
            time_limit_seconds INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS dragon_info (
            id INTEGER PRIMARY KEY DEFAULT 0,
            enabled INTEGER NOT NULL DEFAULT 0,
            map_file_name TEXT NOT NULL,
            monster_name TEXT NOT NULL,
            body_name TEXT NOT NULL,
            location_x INTEGER NOT NULL,
            location_y INTEGER NOT NULL,
            drop_area_top_x INTEGER NOT NULL,
            drop_area_top_y INTEGER NOT NULL,
            drop_area_bottom_x INTEGER NOT NULL,
            drop_area_bottom_y INTEGER NOT NULL,
            exps_json TEXT NOT NULL DEFAULT '[]'
        );
        CREATE TABLE IF NOT EXISTS magic_infos (
            name TEXT PRIMARY KEY,
            spell INTEGER NOT NULL DEFAULT 0,
            base_cost INTEGER NOT NULL DEFAULT 0,
            level_cost INTEGER NOT NULL DEFAULT 0,
            icon INTEGER NOT NULL DEFAULT 0,
            level1 INTEGER NOT NULL DEFAULT 0,
            level2 INTEGER NOT NULL DEFAULT 0,
            level3 INTEGER NOT NULL DEFAULT 0,
            need1 INTEGER NOT NULL DEFAULT 0,
            need2 INTEGER NOT NULL DEFAULT 0,
            need3 INTEGER NOT NULL DEFAULT 0,
            delay_base INTEGER NOT NULL DEFAULT 0,
            delay_reduction INTEGER NOT NULL DEFAULT 0,
            power_base INTEGER NOT NULL DEFAULT 0,
            power_bonus INTEGER NOT NULL DEFAULT 0,
            mpower_base INTEGER NOT NULL DEFAULT 0,
            mpower_bonus INTEGER NOT NULL DEFAULT 0,
            range INTEGER NOT NULL DEFAULT 0,
            multiplier_base REAL NOT NULL DEFAULT 0.0,
            multiplier_bonus REAL NOT NULL DEFAULT 0.0
        );
        CREATE TABLE IF NOT EXISTS game_shop_items (
            item_index INTEGER PRIMARY KEY,
            gindex INTEGER NOT NULL DEFAULT 0,
            gold_price INTEGER NOT NULL DEFAULT 0,
            credit_price INTEGER NOT NULL DEFAULT 0,
            count INTEGER NOT NULL DEFAULT 0,
            class TEXT NOT NULL,
            category TEXT NOT NULL,
            stock INTEGER NOT NULL DEFAULT 0,
            infinite_stock INTEGER NOT NULL DEFAULT 0,
            deal INTEGER NOT NULL DEFAULT 0,
            top_item INTEGER NOT NULL DEFAULT 0,
            date INTEGER NOT NULL DEFAULT 0,
            can_buy_credit INTEGER NOT NULL DEFAULT 0,
            can_buy_gold INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS conquest_infos (
            index INTEGER PRIMARY KEY,
            full_map INTEGER NOT NULL DEFAULT 0,
            location_x INTEGER NOT NULL,
            location_y INTEGER NOT NULL,
            size INTEGER NOT NULL DEFAULT 0,
            name TEXT NOT NULL,
            map_index INTEGER NOT NULL,
            palace_index INTEGER NOT NULL,
            guard_index INTEGER NOT NULL,
            gate_index INTEGER NOT NULL,
            wall_index INTEGER NOT NULL,
            siege_index INTEGER NOT NULL,
            flag_index INTEGER NOT NULL DEFAULT 0,
            extra_maps_json TEXT NOT NULL DEFAULT '[]',
            start_hour INTEGER NOT NULL DEFAULT 0,
            war_length INTEGER NOT NULL DEFAULT 0,
            conquest_type INTEGER NOT NULL DEFAULT 0,
            conquest_game INTEGER NOT NULL DEFAULT 0,
            monday INTEGER NOT NULL DEFAULT 0,
            tuesday INTEGER NOT NULL DEFAULT 0,
            wednesday INTEGER NOT NULL DEFAULT 0,
            thursday INTEGER NOT NULL DEFAULT 0,
            friday INTEGER NOT NULL DEFAULT 0,
            saturday INTEGER NOT NULL DEFAULT 0,
            sunday INTEGER NOT NULL DEFAULT 0,
            king_x INTEGER NOT NULL,
            king_y INTEGER NOT NULL,
            king_size INTEGER NOT NULL DEFAULT 0,
            control_point_index INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS monster_drops (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            monster_index INTEGER NOT NULL,
            item_index INTEGER NOT NULL,
            min_count INTEGER NOT NULL DEFAULT 1,
            max_count INTEGER NOT NULL DEFAULT 1,
            chance REAL NOT NULL DEFAULT 1.0,
            FOREIGN KEY (monster_index) REFERENCES monster_infos(index)
        );
        CREATE INDEX IF NOT EXISTS idx_monster_drops_monster ON monster_drops(monster_index);
        CREATE TABLE IF NOT EXISTS npc_goods (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            npc_index INTEGER NOT NULL,
            item_index INTEGER NOT NULL,
            count INTEGER NOT NULL DEFAULT 1,
            price INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (npc_index) REFERENCES npc_infos(index)
        );
        CREATE INDEX IF NOT EXISTS idx_npc_goods_npc ON npc_goods(npc_index);
        CREATE TABLE IF NOT EXISTS npc_scripts (
            npc_index INTEGER NOT NULL,
            page_name TEXT NOT NULL,
            lines_json TEXT NOT NULL DEFAULT '[]',
            PRIMARY KEY (npc_index, page_name)
        );
        "#
    ).execute(&pool).await?;

    info!("SQLite database initialized: {}", db_path.display());
    Ok(pool)
}

// ============================================================
// Account save/load
// ============================================================

pub async fn save_account(pool: &DbPool, account: &AccountInfo) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT OR REPLACE INTO accounts (username, password_hash, is_online)
           VALUES (?, ?, ?)"#
    )
    .bind(&account.username)
    .bind(&account.password_hash)
    .bind(if account.is_online { 1 } else { 0 })
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_characters_by_account(pool: &DbPool, account_username: &str) -> anyhow::Result<Vec<(String, u16, i32, i32)>> {
    let rows = sqlx::query(
        "SELECT name, map_index, x, y FROM characters WHERE account_username = ?"
    )
    .bind(account_username)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| (
        r.get::<String, _>("name"),
        r.get::<i32, _>("map_index") as u16,
        r.get::<i32, _>("x"),
        r.get::<i32, _>("y"),
    )).collect())
}

pub async fn load_account(pool: &DbPool, username: &str) -> anyhow::Result<Option<AccountInfo>> {
    let row = sqlx::query(
        "SELECT username, password_hash, is_online FROM accounts WHERE username = ?"
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| AccountInfo {
        username: r.get::<String, _>("username"),
        password_hash: r.get::<String, _>("password_hash"),
        is_online: r.get::<i32, _>("is_online") != 0,
    }))
}

pub async fn load_all_accounts(pool: &DbPool) -> anyhow::Result<Vec<AccountInfo>> {
    let rows = sqlx::query("SELECT username, password_hash, is_online FROM accounts")
        .fetch_all(pool)
        .await?;

    Ok(rows.into_iter().map(|r| AccountInfo {
        username: r.get::<String, _>("username"),
        password_hash: r.get::<String, _>("password_hash"),
        is_online: r.get::<i32, _>("is_online") != 0,
    }).collect())
}

pub async fn set_account_offline(pool: &DbPool, username: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE accounts SET is_online = 0 WHERE username = ?")
        .bind(username)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn change_password(pool: &DbPool, username: &str, new_hash: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE accounts SET password_hash = ? WHERE username = ?")
        .bind(new_hash)
        .bind(username)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_character(pool: &DbPool, character_name: &str) -> anyhow::Result<()> {
    // Delete all related data
    sqlx::query("DELETE FROM inventory_backpack WHERE character_name = ?")
        .bind(character_name).execute(pool).await?;
    sqlx::query("DELETE FROM inventory_equipment WHERE character_name = ?")
        .bind(character_name).execute(pool).await?;
    sqlx::query("DELETE FROM inventory_storage WHERE character_name = ?")
        .bind(character_name).execute(pool).await?;
    sqlx::query("DELETE FROM hero_inventory_backpack WHERE character_name = ?")
        .bind(character_name).execute(pool).await?;
    sqlx::query("DELETE FROM friends WHERE character_name = ?")
        .bind(character_name).execute(pool).await?;
    sqlx::query("DELETE FROM blocked_list WHERE character_name = ?")
        .bind(character_name).execute(pool).await?;
    sqlx::query("DELETE FROM mail WHERE character_name = ?")
        .bind(character_name).execute(pool).await?;
    sqlx::query("DELETE FROM quests WHERE character_name = ?")
        .bind(character_name).execute(pool).await?;
    sqlx::query("DELETE FROM completed_quests WHERE character_name = ?")
        .bind(character_name).execute(pool).await?;
    sqlx::query("DELETE FROM creatures WHERE character_name = ?")
        .bind(character_name).execute(pool).await?;
    sqlx::query("DELETE FROM refine_log WHERE character_name = ?")
        .bind(character_name).execute(pool).await?;
    sqlx::query("DELETE FROM guild_members WHERE member_name = ?")
        .bind(character_name).execute(pool).await?;
    sqlx::query("DELETE FROM characters WHERE name = ?")
        .bind(character_name).execute(pool).await?;
    Ok(())
}

// ============================================================
// Character save/load
// ============================================================

pub async fn save_character(pool: &DbPool, state: &PlayerState, account_username: &str) -> anyhow::Result<()> {
    // Save character
    sqlx::query(
        r#"INSERT OR REPLACE INTO characters (
            name, account_username, schema_version, class, gender, hair,
            map_index, x, y, direction,
            attack_mode, pet_mode, level, experience, max_experience,
            hp, max_hp, mp, max_mp, min_attack, max_attack, defence,
            gold, group_id, guild_name, guild_rank,
            spouse_name, allow_mentor, mentor_name, hero_index,
            is_fishing, fishing_autocast, is_dead, pk_points, pk_kill_count
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
    .bind(&state.name)
    .bind(account_username)
    .bind(1i32)
    .bind(state.class as i32)
    .bind(state.gender as i32)
    .bind(state.hair as i32)
    .bind(state.map_index as i32)
    .bind(state.x)
    .bind(state.y)
    .bind(state.direction as i32)
    .bind(format!("{:?}", state.attack_mode))
    .bind(format!("{:?}", state.pet_mode))
    .bind(state.level as i32)
    .bind(state.experience)
    .bind(state.max_experience)
    .bind(state.hp)
    .bind(state.max_hp)
    .bind(state.mp)
    .bind(state.max_mp)
    .bind(state.min_attack)
    .bind(state.max_attack)
    .bind(state.defence)
    .bind(state.inventory.gold as i64)
    .bind(state.group_id.map(|v| v as i64))
    .bind(&state.guild_name)
    .bind(state.guild_rank as i32)
    .bind(&state.spouse_name)
    .bind(if state.allow_mentor { 1 } else { 0 })
    .bind(&state.mentor_name)
    .bind(state.hero_index as i32)
    .bind(if state.is_fishing { 1 } else { 0 })
    .bind(if state.fishing_autocast { 1 } else { 0 })
    .bind(if state.is_dead { 1 } else { 0 })
    .bind(state.pk_points)
    .bind(state.pk_kill_count as i32)
    .execute(pool)
    .await?;

    // Save backpack
    save_inventory(pool, &state.name, &state.inventory).await?;

    // Save hero inventory
    save_hero_inventory(pool, &state.name, &state.hero_inventory).await?;

    // Save friends
    save_friends(pool, &state.name, &state.friend_list).await?;

    // Save mail
    save_mail(pool, &state.name, &state.mailbox).await?;

    // Save quests
    save_quests(pool, &state.name, &state.quest_log).await?;

    // Save creatures
    save_creatures(pool, &state.name, &state.creature_log).await?;

    // Save refine
    save_refine(pool, &state.name, &state.refine_log).await?;

    Ok(())
}

pub async fn load_character(pool: &DbPool, character_name: &str) -> anyhow::Result<Option<PlayerState>> {
    let row = sqlx::query(
        "SELECT * FROM characters WHERE name = ?"
    )
    .bind(character_name)
    .fetch_optional(pool)
    .await?;

    let row = match row {
        Some(r) => r,
        None => return Ok(None),
    };

    let inventory = load_inventory(pool, character_name).await?;
    let friend_list = load_friends(pool, character_name).await?;
    let mailbox = load_mail(pool, character_name).await?;
    let quest_log = load_quests(pool, character_name).await?;
    let creature_log = load_creatures(pool, character_name).await?;
    let refine_log = load_refine(pool, character_name).await?;
    let hero_inventory = load_hero_inventory(pool, character_name).await?;

    let attack_mode = parse_attack_mode(&row.get::<String, _>("attack_mode"));
    let pet_mode = parse_pet_mode(&row.get::<String, _>("pet_mode"));
    let guild_rank = GuildRank::from_u8(row.get::<i32, _>("guild_rank") as u8);

    let class_val = row.get::<i32, _>("class") as u8;
    let class = mir2_shared::enums::MirClass::try_from(class_val).unwrap_or(mir2_shared::enums::MirClass::Warrior);
    let gender_val = row.get::<i32, _>("gender") as u8;
    let gender = mir2_shared::enums::MirGender::try_from(gender_val).unwrap_or(mir2_shared::enums::MirGender::Male);

    let state = PlayerState {
        object_id: 0, // Will be assigned by WorldActor
        name: row.get::<String, _>("name"),
        map_index: row.get::<i32, _>("map_index") as u16,
        x: row.get("x"),
        y: row.get("y"),
        direction: row.get::<i32, _>("direction") as u8,
        attack_mode,
        pet_mode,
        hidden: false,
        session_id: 0, // Will be set on connect
        class,
        gender,
        hair: row.get::<i32, _>("hair") as u8,
        level: row.get::<i32, _>("level") as u16,
        experience: row.get("experience"),
        max_experience: row.get("max_experience"),
        hp: row.get("hp"),
        max_hp: row.get("max_hp"),
        mp: row.get("mp"),
        max_mp: row.get("max_mp"),
        min_attack: row.get("min_attack"),
        max_attack: row.get("max_attack"),
        defence: row.get("defence"),
        bonus_min_attack: 0,
        bonus_max_attack: 0,
        bonus_defence: 0,
        bonus_max_hp: 0,
        bonus_max_mp: 0,
        inventory,
        group_id: row.get::<Option<i64>, _>("group_id").map(|v| v as u64),
        friend_list,
        mailbox,
        guild_name: row.get::<Option<String>, _>("guild_name"),
        guild_rank,
        quest_log,
        spouse_name: row.get::<Option<String>, _>("spouse_name"),
        allow_mentor: row.get::<i32, _>("allow_mentor") != 0,
        mentor_name: row.get::<Option<String>, _>("mentor_name"),
        creature_log,
        hero_index: row.get::<i32, _>("hero_index") as u8,
        hero_inventory,
        refine_log,
        is_fishing: row.get::<i32, _>("is_fishing") != 0,
        is_mounted: false,
        is_dead: row.get::<i32, _>("is_dead") != 0,
        fishing_autocast: row.get::<i32, _>("fishing_autocast") != 0,
        reincarnation_host: None,
        reincarnation_ready: false,
        reincarnation_expire_time: 0,
        enable_group_recall: false,
        last_recall_time: 0,
        allow_lover_recall: row.get::<Option<i32>, _>("allow_lover_recall").map(|v| v != 0).unwrap_or(false),
        is_gm: false, // TODO: load from accounts.admin_account column
        pk_points: row.get::<i32, _>("pk_points"),
        pk_kill_count: row.get::<i32, _>("pk_kill_count") as u32,
        buffs: Vec::new(),
    };

    Ok(Some(state))
}

fn parse_attack_mode(s: &str) -> mir2_shared::enums::AttackMode {
    match s {
        "Group" => mir2_shared::enums::AttackMode::Group,
        "Guild" => mir2_shared::enums::AttackMode::Guild,
        "EnemyGuild" => mir2_shared::enums::AttackMode::EnemyGuild,
        "RedBrown" => mir2_shared::enums::AttackMode::RedBrown,
        "All" => mir2_shared::enums::AttackMode::All,
        _ => mir2_shared::enums::AttackMode::Peace,
    }
}

fn parse_pet_mode(s: &str) -> mir2_shared::enums::PetMode {
    match s {
        "MoveOnly" => mir2_shared::enums::PetMode::MoveOnly,
        "AttackOnly" => mir2_shared::enums::PetMode::AttackOnly,
        "None" => mir2_shared::enums::PetMode::None,
        "FocusMasterTarget" => mir2_shared::enums::PetMode::FocusMasterTarget,
        _ => mir2_shared::enums::PetMode::Both,
    }
}

// ============================================================
// Inventory save/load
// ============================================================

async fn save_inventory(pool: &DbPool, character_name: &str, inv: &PlayerInventory) -> anyhow::Result<()> {
    // Clear existing
    sqlx::query("DELETE FROM inventory_backpack WHERE character_name = ?")
        .bind(character_name).execute(pool).await?;
    sqlx::query("DELETE FROM inventory_equipment WHERE character_name = ?")
        .bind(character_name).execute(pool).await?;
    sqlx::query("DELETE FROM inventory_storage WHERE character_name = ?")
        .bind(character_name).execute(pool).await?;

    // Backpack
    for (grid, slot) in inv.backpack.iter().enumerate() {
        if let Some(s) = slot {
            let item_json = serde_json::to_string(&s.item)?;
            sqlx::query("INSERT INTO inventory_backpack (character_name, grid, item_json) VALUES (?, ?, ?)")
                .bind(character_name).bind(grid as i32).bind(&item_json)
                .execute(pool).await?;
        }
    }

    // Equipment
    for (slot, item) in inv.equipment.iter().enumerate() {
        if let Some(item) = item {
            let item_json = serde_json::to_string(item)?;
            sqlx::query("INSERT INTO inventory_equipment (character_name, slot, item_json) VALUES (?, ?, ?)")
                .bind(character_name).bind(slot as i32).bind(&item_json)
                .execute(pool).await?;
        }
    }

    // Storage
    for (grid, slot) in inv.storage.iter().enumerate() {
        if let Some(s) = slot {
            let item_json = serde_json::to_string(&s.item)?;
            sqlx::query("INSERT INTO inventory_storage (character_name, grid, item_json) VALUES (?, ?, ?)")
                .bind(character_name).bind(grid as i32).bind(&item_json)
                .execute(pool).await?;
        }
    }

    Ok(())
}

async fn load_inventory(pool: &DbPool, character_name: &str) -> anyhow::Result<PlayerInventory> {
    let mut inv = PlayerInventory::new();

    // Backpack
    let backpack_rows = sqlx::query(
        "SELECT grid, item_json FROM inventory_backpack WHERE character_name = ?"
    )
    .bind(character_name)
    .fetch_all(pool)
    .await?;

    for row in backpack_rows {
        let grid: i32 = row.get("grid");
        let item_json: String = row.get("item_json");
        if let Ok(item) = serde_json::from_str::<mir2_shared::data::item::UserItem>(&item_json) {
            inv.backpack[grid as usize] = Some(crate::actors::inventory::InventorySlot {
                grid: grid as u8,
                item,
            });
        }
    }

    // Equipment
    let equip_rows = sqlx::query(
        "SELECT slot, item_json FROM inventory_equipment WHERE character_name = ?"
    )
    .bind(character_name)
    .fetch_all(pool)
    .await?;

    for row in equip_rows {
        let slot: i32 = row.get("slot");
        let item_json: String = row.get("item_json");
        if let Ok(item) = serde_json::from_str::<mir2_shared::data::item::UserItem>(&item_json) {
            inv.equipment[slot as usize] = Some(item);
        }
    }

    // Storage
    let storage_rows = sqlx::query(
        "SELECT grid, item_json FROM inventory_storage WHERE character_name = ?"
    )
    .bind(character_name)
    .fetch_all(pool)
    .await?;

    for row in storage_rows {
        let grid: i32 = row.get("grid");
        let item_json: String = row.get("item_json");
        if let Ok(item) = serde_json::from_str::<mir2_shared::data::item::UserItem>(&item_json) {
            inv.storage[grid as usize] = Some(crate::actors::inventory::InventorySlot {
                grid: grid as u8,
                item,
            });
        }
    }

    Ok(inv)
}

// ============================================================
// Hero inventory save/load
// ============================================================

async fn save_hero_inventory(pool: &DbPool, character_name: &str, inv: &PlayerInventory) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM hero_inventory_backpack WHERE character_name = ?")
        .bind(character_name).execute(pool).await?;

    for (grid, slot) in inv.backpack.iter().enumerate() {
        if let Some(s) = slot {
            let item_json = serde_json::to_string(&s.item)?;
            sqlx::query("INSERT INTO hero_inventory_backpack (character_name, grid, item_json) VALUES (?, ?, ?)")
                .bind(character_name).bind(grid as i32).bind(&item_json)
                .execute(pool).await?;
        }
    }

    Ok(())
}

async fn load_hero_inventory(pool: &DbPool, character_name: &str) -> anyhow::Result<PlayerInventory> {
    let mut inv = PlayerInventory::new();

    let rows = sqlx::query(
        "SELECT grid, item_json FROM hero_inventory_backpack WHERE character_name = ?"
    )
    .bind(character_name)
    .fetch_all(pool)
    .await?;

    for row in rows {
        let grid: i32 = row.get("grid");
        let item_json: String = row.get("item_json");
        if let Ok(item) = serde_json::from_str::<mir2_shared::data::item::UserItem>(&item_json) {
            inv.backpack[grid as usize] = Some(crate::actors::inventory::InventorySlot {
                grid: grid as u8,
                item,
            });
        }
    }

    Ok(inv)
}

// ============================================================
// Friends save/load
// ============================================================

async fn save_friends(pool: &DbPool, character_name: &str, list: &FriendList) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM friends WHERE character_name = ?")
        .bind(character_name).execute(pool).await?;
    sqlx::query("DELETE FROM blocked_list WHERE character_name = ?")
        .bind(character_name).execute(pool).await?;

    for f in &list.friends {
        sqlx::query(
            "INSERT INTO friends (character_name, friend_object_id, friend_name, memo) VALUES (?, ?, ?, ?)"
        )
        .bind(character_name).bind(f.object_id as i64).bind(&f.name).bind(&f.memo)
        .execute(pool).await?;
    }

    for b in &list.blocked {
        sqlx::query(
            "INSERT INTO blocked_list (character_name, blocked_object_id, blocked_name) VALUES (?, ?, ?)"
        )
        .bind(character_name).bind(b.object_id as i64).bind(&b.name)
        .execute(pool).await?;
    }

    Ok(())
}

async fn load_friends(pool: &DbPool, character_name: &str) -> anyhow::Result<FriendList> {
    let mut list = FriendList::new();

    let friend_rows = sqlx::query(
        "SELECT friend_object_id, friend_name, memo FROM friends WHERE character_name = ?"
    )
    .bind(character_name)
    .fetch_all(pool)
    .await?;

    for row in friend_rows {
        list.friends.push(FriendEntry {
            object_id: row.get::<i64, _>("friend_object_id") as u32,
            name: row.get("friend_name"),
            memo: row.get("memo"),
        });
    }

    let blocked_rows = sqlx::query(
        "SELECT blocked_object_id, blocked_name FROM blocked_list WHERE character_name = ?"
    )
    .bind(character_name)
    .fetch_all(pool)
    .await?;

    for row in blocked_rows {
        list.blocked.push(BlockedEntry {
            object_id: row.get::<i64, _>("blocked_object_id") as u32,
            name: row.get("blocked_name"),
        });
    }

    Ok(list)
}

// ============================================================
// Mail save/load
// ============================================================

async fn save_mail(pool: &DbPool, character_name: &str, mailbox: &Mailbox) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM mail WHERE character_name = ?")
        .bind(character_name).execute(pool).await?;

    for m in &mailbox.inbox {
        let items_json = serde_json::to_string(&m.items)?;
        sqlx::query(
            r#"INSERT INTO mail (character_name, mail_id, sender_name, subject, body, timestamp,
                read_flag, collected, locked, gold, items_json)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(character_name)
        .bind(m.mail_id as i64)
        .bind(&m.sender_name)
        .bind(&m.subject)
        .bind(&m.body)
        .bind(m.timestamp)
        .bind(if m.read { 1 } else { 0 })
        .bind(if m.collected { 1 } else { 0 })
        .bind(if m.locked { 1 } else { 0 })
        .bind(m.gold as i64)
        .bind(&items_json)
        .execute(pool).await?;
    }

    Ok(())
}

async fn load_mail(pool: &DbPool, character_name: &str) -> anyhow::Result<Mailbox> {
    let mut mailbox = Mailbox::new();

    let rows = sqlx::query(
        "SELECT mail_id, sender_name, subject, body, timestamp, read_flag, collected, locked, gold, items_json
         FROM mail WHERE character_name = ?"
    )
    .bind(character_name)
    .fetch_all(pool)
    .await?;

    for row in rows {
        let items: Vec<mir2_shared::data::item::UserItem> =
            serde_json::from_str(&row.get::<String, _>("items_json")).unwrap_or_default();

        mailbox.add_mail(MailMessage {
            mail_id: row.get::<i64, _>("mail_id") as u64,
            sender_name: row.get("sender_name"),
            receiver_name: character_name.to_string(),
            subject: row.get("subject"),
            body: row.get("body"),
            timestamp: row.get("timestamp"),
            read: row.get::<i32, _>("read_flag") != 0,
            collected: row.get::<i32, _>("collected") != 0,
            locked: row.get::<i32, _>("locked") != 0,
            gold: row.get::<i64, _>("gold") as u64,
            items,
        });
    }

    Ok(mailbox)
}

// ============================================================
// Quests save/load
// ============================================================

async fn save_quests(pool: &DbPool, character_name: &str, log: &QuestLog) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM quests WHERE character_name = ?")
        .bind(character_name).execute(pool).await?;
    sqlx::query("DELETE FROM completed_quests WHERE character_name = ?")
        .bind(character_name).execute(pool).await?;

    for q in &log.quests {
        let progress_json = serde_json::to_string(&q.progress)?;
        let status_str = match q.status {
            QuestStatus::Accepted => "Accepted",
            QuestStatus::InProgress => "InProgress",
            QuestStatus::Completed => "Completed",
        };
        sqlx::query(
            "INSERT INTO quests (character_name, quest_index, title, status, progress_json, exp_reward, gold_reward)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(character_name)
        .bind(q.quest_index)
        .bind(&q.title)
        .bind(status_str)
        .bind(&progress_json)
        .bind(q.exp_reward)
        .bind(q.gold_reward as i64)
        .execute(pool).await?;
    }

    for qi in &log.completed_indices {
        sqlx::query("INSERT INTO completed_quests (character_name, quest_index) VALUES (?, ?)")
            .bind(character_name).bind(qi)
            .execute(pool).await?;
    }

    Ok(())
}

async fn load_quests(pool: &DbPool, character_name: &str) -> anyhow::Result<QuestLog> {
    let mut log = QuestLog::new();

    let rows = sqlx::query(
        "SELECT quest_index, title, status, progress_json, exp_reward, gold_reward
         FROM quests WHERE character_name = ?"
    )
    .bind(character_name)
    .fetch_all(pool)
    .await?;

    for row in rows {
        let status_str: String = row.get("status");
        let status = match status_str.as_str() {
            "Accepted" => QuestStatus::Accepted,
            "Completed" => QuestStatus::Completed,
            _ => QuestStatus::InProgress,
        };

        let progress: Vec<QuestProgress> =
            serde_json::from_str(&row.get::<String, _>("progress_json")).unwrap_or_default();

        log.quests.push(QuestInstance {
            quest_index: row.get("quest_index"),
            title: row.get("title"),
            status,
            progress,
            exp_reward: row.get("exp_reward"),
            gold_reward: row.get::<i64, _>("gold_reward") as u64,
        });
    }

    let completed_rows = sqlx::query(
        "SELECT quest_index FROM completed_quests WHERE character_name = ?"
    )
    .bind(character_name)
    .fetch_all(pool)
    .await?;

    for row in completed_rows {
        log.completed_indices.push(row.get("quest_index"));
    }

    Ok(log)
}

// ============================================================
// Guild save/load
// ============================================================

pub async fn save_guild(pool: &DbPool, guild: &Guild) -> anyhow::Result<()> {
    let notice_json = serde_json::to_string(&guild.notice)?;
    let storage_items_json = serde_json::to_string(&guild.storage_items)?;
    sqlx::query("INSERT OR REPLACE INTO guilds (name, notice_json, gold, storage_items_json) VALUES (?, ?, ?, ?)")
        .bind(&guild.name)
        .bind(&notice_json)
        .bind(guild.gold as i64)
        .bind(&storage_items_json)
        .execute(pool)
        .await?;

    // Clear existing members
    sqlx::query("DELETE FROM guild_members WHERE guild_name = ?")
        .bind(&guild.name).execute(pool).await?;

    for m in &guild.members {
        sqlx::query("INSERT INTO guild_members (guild_name, member_name, rank) VALUES (?, ?, ?)")
            .bind(&guild.name)
            .bind(&m.name)
            .bind(m.rank as i32)
            .execute(pool).await?;
    }

    Ok(())
}

pub async fn load_guilds(pool: &DbPool) -> anyhow::Result<HashMap<String, Guild>> {
    let mut guilds = HashMap::new();

    let guild_rows = sqlx::query("SELECT name, notice_json, gold, storage_items_json FROM guilds")
        .fetch_all(pool)
        .await?;

    for row in guild_rows {
        let name: String = row.get("name");
        let notice: Vec<String> = serde_json::from_str(&row.get::<String, _>("notice_json")).unwrap_or_default();
        let gold: i64 = row.get("gold");
        let storage_items: Vec<Option<(mir2_shared::data::item::UserItem, u32)>> =
            serde_json::from_str(&row.get::<String, _>("storage_items_json")).unwrap_or_else(|_| vec![None; 100]);

        let member_rows = sqlx::query(
            "SELECT member_name, rank FROM guild_members WHERE guild_name = ?"
        )
        .bind(&name)
        .fetch_all(pool)
        .await?;

        let members: Vec<GuildMember> = member_rows.into_iter().map(|r| GuildMember {
            name: r.get("member_name"),
            session_id: None, // Loaded as offline
            rank: GuildRank::from_u8(r.get::<i32, _>("rank") as u8),
        }).collect();

        guilds.insert(name.clone(), Guild {
            name,
            notice,
            members,
            gold: gold as u64,
            storage_items,
        });
    }

    Ok(guilds)
}

// ============================================================
// Creatures save/load
// ============================================================

async fn save_creatures(pool: &DbPool, character_name: &str, log: &CreatureLog) -> anyhow::Result<()> {
    let owned_json = serde_json::to_string(&log.owned_creatures)?;

    let (active_type, active_custom_name, active_pickup_mode, active_hunger, active_enabled) =
        if let Some(c) = &log.active_creature {
            (c.creature_type as i32, c.custom_name.clone(), c.pickup_mode as i32, c.hunger as i32, if c.enabled { 1 } else { 0 })
        } else {
            (0, None, 0, 100, 0)
        };

    sqlx::query(
        r#"INSERT OR REPLACE INTO creatures (
            character_name, active_type, active_custom_name, active_pickup_mode,
            active_hunger, active_enabled, owned_json, request_updates
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#
    )
    .bind(character_name)
    .bind(active_type)
    .bind(active_custom_name)
    .bind(active_pickup_mode)
    .bind(active_hunger)
    .bind(active_enabled)
    .bind(&owned_json)
    .bind(if log.request_updates { 1 } else { 0 })
    .execute(pool).await?;

    Ok(())
}

async fn load_creatures(pool: &DbPool, character_name: &str) -> anyhow::Result<CreatureLog> {
    let row = sqlx::query(
        "SELECT active_type, active_custom_name, active_pickup_mode, active_hunger,
                active_enabled, owned_json, request_updates
         FROM creatures WHERE character_name = ?"
    )
    .bind(character_name)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let active_type: i32 = r.get("active_type");
            let active_enabled: i32 = r.get("active_enabled");

            let active_creature = if active_type > 0 && active_enabled != 0 {
                Some(IntelligentCreature {
                    creature_type: CreatureType::from(active_type as u8),
                    custom_name: r.get::<Option<String>, _>("active_custom_name"),
                    pickup_mode: PickupMode::from(r.get::<i32, _>("active_pickup_mode") as u8),
                    hunger: r.get::<i32, _>("active_hunger") as u8,
                    enabled: active_enabled != 0,
                })
            } else {
                None
            };

            let owned_creatures: Vec<IntelligentCreature> =
                serde_json::from_str(&r.get::<String, _>("owned_json")).unwrap_or_default();

            Ok(CreatureLog {
                active_creature,
                owned_creatures,
                request_updates: r.get::<i32, _>("request_updates") != 0,
            })
        }
        None => Ok(CreatureLog::new()),
    }
}

// ============================================================
// Refine save/load
// ============================================================

async fn save_refine(pool: &DbPool, character_name: &str, log: &RefineLog) -> anyhow::Result<()> {
    let (uid, item_index, start_time, finish_time, status, success_chance) =
        if let Some(item) = &log.active_refine {
            (
                Some(item.original_uid as i64),
                item.item_index as i32,
                item.start_time as i64,
                item.finish_time as i64,
                item.status as i32,
                item.success_chance as i32,
            )
        } else {
            (None, 0, 0, 0, 0, 0)
        };

    sqlx::query(
        r#"INSERT OR REPLACE INTO refine_log (
            character_name, active_original_uid, active_item_index,
            active_start_time, active_finish_time, active_status, active_success_chance,
            total_refines, successful_refines
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
    .bind(character_name)
    .bind(uid)
    .bind(item_index)
    .bind(start_time)
    .bind(finish_time)
    .bind(status)
    .bind(success_chance)
    .bind(log.total_refines as i32)
    .bind(log.successful_refines as i32)
    .execute(pool).await?;

    Ok(())
}

async fn load_refine(pool: &DbPool, character_name: &str) -> anyhow::Result<RefineLog> {
    let row = sqlx::query(
        "SELECT active_original_uid, active_item_index, active_start_time, active_finish_time,
                active_status, active_success_chance, total_refines, successful_refines
         FROM refine_log WHERE character_name = ?"
    )
    .bind(character_name)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let uid: Option<i64> = r.get("active_original_uid");
            let status: i32 = r.get("active_status");

            let active_refine = if uid.is_some() && uid != Some(0) {
                Some(RefiningItem {
                    original_uid: uid.unwrap() as u64,
                    item_index: r.get::<i32, _>("active_item_index") as u32,
                    start_time: r.get::<i64, _>("active_start_time") as u64,
                    finish_time: r.get::<i64, _>("active_finish_time") as u64,
                    status: RefineStatus::from_i32(status).unwrap_or(RefineStatus::None),
                    success_chance: r.get::<i32, _>("active_success_chance") as u8,
                })
            } else {
                None
            };

            Ok(RefineLog {
                active_refine,
                total_refines: r.get::<i32, _>("total_refines") as u32,
                successful_refines: r.get::<i32, _>("successful_refines") as u32,
            })
        }
        None => Ok(RefineLog::new()),
    }
}

// Add From impl for RefineStatus since it doesn't have one
impl RefineStatus {
    fn from_i32(val: i32) -> Option<Self> {
        match val {
            0 => Some(Self::None),
            1 => Some(Self::Pending),
            2 => Some(Self::Ready),
            3 => Some(Self::Failed),
            _ => None,
        }
    }
}

use std::collections::HashMap;

// ============================================================
// Game config loading (migrated from Server.MirDB)
// ============================================================

/// Map safe zone
#[derive(Debug, Clone)]
pub struct SafeZoneInfo {
    pub map_index: i32,
    pub x: i32,
    pub y: i32,
    pub size: i32,
    pub start_point: bool,
}

/// Map respawn
#[derive(Debug, Clone)]
pub struct MapRespawnInfo {
    pub map_index: i32,
    pub monster_index: i32,
    pub x: i32,
    pub y: i32,
    pub count: i32,
    pub spread: i32,
    pub delay: i32,
    pub direction: i32,
    pub route_path: Option<String>,
    pub random_delay: i32,
    pub respawn_index: i32,
    pub save_respawn_time: bool,
    pub respawn_ticks: i32,
}

/// Map movement (teleport)
#[derive(Debug, Clone)]
pub struct MapMovementInfo {
    pub map_index: i32,
    pub source_x: i32,
    pub source_y: i32,
    pub dest_x: i32,
    pub dest_y: i32,
    pub need_hole: bool,
    pub need_move: bool,
    pub conquest_index: i32,
    pub show_on_big_map: bool,
    pub icon: i32,
}

/// Map info with nested data
#[derive(Debug, Clone)]
pub struct MapInfo {
    pub index: i32,
    pub file_name: String,
    pub title: String,
    pub mini_map: i32,
    pub light: i32,
    pub big_map: bool,
    pub no_teleport: bool,
    pub no_reconnect: bool,
    pub no_reconnect_map: String,
    pub no_random: bool,
    pub no_escape: bool,
    pub no_recall: bool,
    pub no_drug: bool,
    pub no_position: bool,
    pub no_throw_item: bool,
    pub no_drop_player: bool,
    pub no_drop_monster: bool,
    pub no_names: bool,
    pub fight: bool,
    pub fire: bool,
    pub fire_damage: i32,
    pub lightning: bool,
    pub lightning_damage: i32,
    pub map_dark_light: i32,
    pub mine_index: i32,
    pub no_mount: bool,
    pub need_bridle: bool,
    pub no_fight: bool,
    pub music: bool,
    pub no_town_teleport: bool,
    pub no_reincarnation: bool,
    pub weather_particles: bool,
    pub gt: bool,
    pub gt_index: i32,
    pub safe_zones: Vec<SafeZoneInfo>,
    pub respawns: Vec<MapRespawnInfo>,
    pub movements: Vec<MapMovementInfo>,
}

/// Item info (flat from DB, stats parsed from JSON)
#[derive(Debug, Clone)]
pub struct ItemInfo {
    pub index: i32,
    pub name: String,
    pub item_type: i32,
    pub grade: i32,
    pub required_type: i32,
    pub required_class: i32,
    pub required_gender: i32,
    pub set_type: i32,
    pub shape: i32,
    pub weight: i32,
    pub light: i32,
    pub required_amount: i32,
    pub image: i32,
    pub durability: i32,
    pub stack_size: i32,
    pub price: u32,
    pub start_item: bool,
    pub effect: i32,
    pub bool_flags: i64,
    pub bind_mode: i32,
    pub special_mode: i32,
    pub random_stats_id: i32,
    pub can_fast_run: bool,
    pub can_awakening: bool,
    pub slots: i32,
    pub stats_json: String,
    pub stats: HashMap<u8, i32>,
    pub has_tool_tip: bool,
    pub tool_tip: Option<String>,
}

/// Monster info (flat from DB, stats parsed from JSON)
#[derive(Debug, Clone)]
pub struct MonsterInfo {
    pub index: i32,
    pub name: String,
    pub image: i32,
    pub ai: i32,
    pub effect: i32,
    pub level: i32,
    pub view_range: i32,
    pub cool_eye: i32,
    pub stats_json: String,
    pub stats: HashMap<u8, i32>,
    pub light: i32,
    pub attack_speed: i32,
    pub move_speed: i32,
    pub experience: i32,
    pub can_push: bool,
    pub can_tame: bool,
    pub auto_rev: bool,
    pub undead: bool,
    pub drop_path: Option<String>,
}

/// Monster drop entry (from DB)
#[derive(Debug, Clone)]
pub struct MonsterDropInfo {
    pub monster_index: i32,
    pub item_index: i32,
    pub min_count: u16,
    pub max_count: u16,
    pub chance: f64,
}

/// NPC info
#[derive(Debug, Clone)]
pub struct NPCInfo {
    pub index: i32,
    pub map_index: i32,
    pub file_name: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub image: i32,
    pub rate: i32,
    pub time_visible: i32,
    pub hour_start: i32,
    pub minute_start: i32,
    pub hour_end: i32,
    pub minute_end: i32,
    pub min_lev: i32,
    pub max_lev: i32,
    pub day_of_week: Option<String>,
    pub class_required: Option<String>,
    pub conquest: i32,
    pub flag_needed: i32,
    pub show_on_big_map: bool,
    pub big_map_icon: i32,
    pub can_teleport_to: bool,
    pub conquest_visible: bool,
    pub collect_quest_indexes: Vec<i32>,
    pub finish_quest_indexes: Vec<i32>,
}

/// Quest info
#[derive(Debug, Clone)]
pub struct QuestKillTask {
    pub monster_index: i32,
    pub count: i32,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct QuestItemTask {
    pub item_index: i32,
    pub count: i32,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct QuestFlagTask {
    pub number: i32,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct QuestInfo {
    pub index: i32,
    pub name: String,
    pub group_name: String,
    pub file_name: String,
    pub required_min_level: i32,
    pub required_max_level: i32,
    pub required_quest: i32,
    pub required_class: i32,
    pub quest_type: i32,
    pub exp_reward: i32,
    pub gold_reward: i32,
    pub goto_message: Option<String>,
    pub kill_message: Option<String>,
    pub item_message: Option<String>,
    pub flag_message: Option<String>,
    pub time_limit_seconds: i32,
    /// Parsed from quest .txt files
    pub kill_tasks: Vec<QuestKillTask>,
    pub item_tasks: Vec<QuestItemTask>,
    pub flag_tasks: Vec<QuestFlagTask>,
}

/// NPC 商品信息
#[derive(Debug, Clone)]
pub struct NpcGoodsInfo {
    pub npc_index: i32,
    pub item_index: i32,
    pub count: i32,
    pub price: i32,
}

/// NPC 脚本信息
#[derive(Debug, Clone)]
pub struct NpcScriptInfo {
    pub npc_index: i32,
    pub page_name: String,
    pub lines: Vec<String>,
}

/// Game shop item
#[derive(Debug, Clone)]
pub struct GameShopItem {
    pub item_index: i32,
    pub gindex: i32,
    pub gold_price: u32,
    pub credit_price: u32,
    pub count: u16,
    pub class_name: String,
    pub category: String,
    pub stock: i32,
    pub infinite_stock: bool,
    pub deal: bool,
    pub top_item: bool,
    pub date: i64,
    pub can_buy_credit: bool,
    pub can_buy_gold: bool,
}

/// Magic info
#[derive(Debug, Clone)]
pub struct MagicInfo {
    pub name: String,
    pub spell: i32,
    pub base_cost: i32,
    pub level_cost: i32,
    pub icon: i32,
    pub level1: i32,
    pub level2: i32,
    pub level3: i32,
    pub need1: i32,
    pub need2: i32,
    pub need3: i32,
    pub delay_base: i32,
    pub delay_reduction: i32,
    pub power_base: i32,
    pub power_bonus: i32,
    pub mpower_base: i32,
    pub mpower_bonus: i32,
    pub range: i32,
    pub multiplier_base: f64,
    pub multiplier_bonus: f64,
}

/// Dragon info
#[derive(Debug, Clone)]
pub struct DragonInfo {
    pub id: i32,
    pub enabled: bool,
    pub map_file_name: String,
    pub monster_name: String,
    /// Resolved from monster_name at load time; None if monster not found
    pub monster_index: Option<i32>,
    pub body_name: String,
    pub location_x: i32,
    pub location_y: i32,
    pub drop_area_top_x: i32,
    pub drop_area_top_y: i32,
    pub drop_area_bottom_x: i32,
    pub drop_area_bottom_y: i32,
    pub exps: Vec<i64>,
}

/// Load all map infos from DB with nested safe_zones, respawns, movements
pub async fn load_map_infos(pool: &DbPool) -> anyhow::Result<Vec<MapInfo>> {
    // 4 queries total instead of 1 + N*3
    let rows = sqlx::query("SELECT * FROM map_infos").fetch_all(pool).await?;
    let sz_rows = sqlx::query("SELECT * FROM safe_zones").fetch_all(pool).await?;
    let rs_rows = sqlx::query("SELECT * FROM map_respawns").fetch_all(pool).await?;
    let mv_rows = sqlx::query("SELECT * FROM map_movements").fetch_all(pool).await?;

    // Index child rows by map_index
    let mut sz_by_map: HashMap<i32, Vec<SafeZoneInfo>> = HashMap::new();
    for r in sz_rows {
        let mi: i32 = r.get("map_index");
        sz_by_map.entry(mi).or_default().push(SafeZoneInfo {
            map_index: mi,
            x: r.get("x"),
            y: r.get("y"),
            size: r.get("size"),
            start_point: r.get::<i32, _>("start_point") != 0,
        });
    }

    let mut rs_by_map: HashMap<i32, Vec<MapRespawnInfo>> = HashMap::new();
    for r in rs_rows {
        let mi: i32 = r.get("map_index");
        rs_by_map.entry(mi).or_default().push(MapRespawnInfo {
            map_index: mi,
            monster_index: r.get("monster_index"),
            x: r.get("x"),
            y: r.get("y"),
            count: r.get("count"),
            spread: r.get("spread"),
            delay: r.get("delay"),
            direction: r.get("direction"),
            route_path: r.get::<Option<String>, _>("route_path"),
            random_delay: r.get("random_delay"),
            respawn_index: r.get("respawn_index"),
            save_respawn_time: r.get::<i32, _>("save_respawn_time") != 0,
            respawn_ticks: r.get("respawn_ticks"),
        });
    }

    let mut mv_by_map: HashMap<i32, Vec<MapMovementInfo>> = HashMap::new();
    for r in mv_rows {
        let mi: i32 = r.get("map_index");
        mv_by_map.entry(mi).or_default().push(MapMovementInfo {
            map_index: mi,
            source_x: r.get("source_x"),
            source_y: r.get("source_y"),
            dest_x: r.get("dest_x"),
            dest_y: r.get("dest_y"),
            need_hole: r.get::<i32, _>("need_hole") != 0,
            need_move: r.get::<i32, _>("need_move") != 0,
            conquest_index: r.get("conquest_index"),
            show_on_big_map: r.get::<i32, _>("show_on_big_map") != 0,
            icon: r.get("icon"),
        });
    }

    let mut maps = Vec::with_capacity(rows.len());
    for row in rows {
        let index: i32 = row.get("index");
        maps.push(MapInfo {
            index,
            file_name: row.get("file_name"),
            title: row.get("title"),
            mini_map: row.get("mini_map"),
            light: row.get("light"),
            big_map: row.get::<i32, _>("big_map") != 0,
            no_teleport: row.get::<i32, _>("no_teleport") != 0,
            no_reconnect: row.get::<i32, _>("no_reconnect") != 0,
            no_reconnect_map: row.get::<Option<String>, _>("no_reconnect_map").unwrap_or_default(),
            no_random: row.get::<i32, _>("no_random") != 0,
            no_escape: row.get::<i32, _>("no_escape") != 0,
            no_recall: row.get::<i32, _>("no_recall") != 0,
            no_drug: row.get::<i32, _>("no_drug") != 0,
            no_position: row.get::<i32, _>("no_position") != 0,
            no_throw_item: row.get::<i32, _>("no_throw_item") != 0,
            no_drop_player: row.get::<i32, _>("no_drop_player") != 0,
            no_drop_monster: row.get::<i32, _>("no_drop_monster") != 0,
            no_names: row.get::<i32, _>("no_names") != 0,
            fight: row.get::<i32, _>("fight") != 0,
            fire: row.get::<i32, _>("fire") != 0,
            fire_damage: row.get("fire_damage"),
            lightning: row.get::<i32, _>("lightning") != 0,
            lightning_damage: row.get("lightning_damage"),
            map_dark_light: row.get("map_dark_light"),
            mine_index: row.get("mine_index"),
            no_mount: row.get::<i32, _>("no_mount") != 0,
            need_bridle: row.get::<i32, _>("need_bridle") != 0,
            no_fight: row.get::<i32, _>("no_fight") != 0,
            music: row.get::<i32, _>("music") != 0,
            no_town_teleport: row.get::<Option<i32>, _>("no_town_teleport").unwrap_or(0) != 0,
            no_reincarnation: row.get::<Option<i32>, _>("no_reincarnation").unwrap_or(0) != 0,
            weather_particles: row.get::<Option<i32>, _>("weather_particles").unwrap_or(0) != 0,
            gt: row.get::<Option<i32>, _>("gt").unwrap_or(0) != 0,
            gt_index: row.get::<Option<i32>, _>("gt_index").unwrap_or(0),
            safe_zones: sz_by_map.remove(&index).unwrap_or_default(),
            respawns: rs_by_map.remove(&index).unwrap_or_default(),
            movements: mv_by_map.remove(&index).unwrap_or_default(),
        });
    }

    Ok(maps)
}

/// Load all item infos from DB
pub async fn load_item_infos(pool: &DbPool) -> anyhow::Result<Vec<ItemInfo>> {
    let rows = sqlx::query("SELECT * FROM item_infos ORDER BY index").fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| {
        let stats_json: String = r.get("stats_json");
        let stats: HashMap<u8, i32> = serde_json::from_str(&stats_json)
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to parse item stats JSON for index {}: {}", r.get::<i32, _>("index"), e);
                HashMap::new()
            });
        ItemInfo {
            index: r.get("index"),
            name: r.get("name"),
            item_type: r.get("type"),
            grade: r.get("grade"),
            required_type: r.get("required_type"),
            required_class: r.get("required_class"),
            required_gender: r.get("required_gender"),
            set_type: r.get("set_type"),
            shape: r.get("shape"),
            weight: r.get("weight"),
            light: r.get("light"),
            required_amount: r.get("required_amount"),
            image: r.get("image"),
            durability: r.get("durability"),
            stack_size: r.get("stack_size"),
            price: r.get::<i64, _>("price") as u32,
            start_item: r.get::<i32, _>("start_item") != 0,
            effect: r.get("effect"),
            bool_flags: r.get("bool_flags"),
            bind_mode: r.get("bind_mode"),
            special_mode: r.get("special_mode"),
            random_stats_id: r.get("random_stats_id"),
            can_fast_run: r.get::<i32, _>("can_fast_run") != 0,
            can_awakening: r.get::<i32, _>("can_awakening") != 0,
            slots: r.get("slots"),
            stats_json,
            stats,
            has_tool_tip: r.get::<i32, _>("has_tool_tip") != 0,
            tool_tip: r.get::<Option<String>, _>("tool_tip"),
        }
    }).collect())
}

/// Load all monster infos from DB
pub async fn load_monster_infos(pool: &DbPool) -> anyhow::Result<Vec<MonsterInfo>> {
    let rows = sqlx::query("SELECT * FROM monster_infos ORDER BY index").fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| {
        let stats_json: String = r.get("stats_json");
        let stats: HashMap<u8, i32> = serde_json::from_str(&stats_json)
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to parse monster stats JSON for index {}: {}", r.get::<i32, _>("index"), e);
                HashMap::new()
            });
        MonsterInfo {
            index: r.get("index"),
            name: r.get("name"),
            image: r.get("image"),
            ai: r.get("ai"),
            effect: r.get("effect"),
            level: r.get("level"),
            view_range: r.get("view_range"),
            cool_eye: r.get("cool_eye"),
            stats_json,
            stats,
            light: r.get("light"),
            attack_speed: r.get("attack_speed"),
            move_speed: r.get("move_speed"),
            experience: r.get("experience"),
            can_push: r.get::<i32, _>("can_push") != 0,
            can_tame: r.get::<i32, _>("can_tame") != 0,
            auto_rev: r.get::<i32, _>("auto_rev") != 0,
            undead: r.get::<i32, _>("undead") != 0,
            drop_path: r.get::<Option<String>, _>("drop_path"),
        }
    }).collect())
}

/// Load monster drops grouped by monster_index
pub async fn load_monster_drops(pool: &DbPool) -> anyhow::Result<HashMap<i32, Vec<MonsterDropInfo>>> {
    let rows = sqlx::query("SELECT * FROM monster_drops ORDER BY monster_index").fetch_all(pool).await?;
    let mut map: HashMap<i32, Vec<MonsterDropInfo>> = HashMap::new();
    for r in rows {
        let monster_index: i32 = r.get("monster_index");
        let entry = MonsterDropInfo {
            monster_index,
            item_index: r.get("item_index"),
            min_count: r.get::<i32, _>("min_count") as u16,
            max_count: r.get::<i32, _>("max_count") as u16,
            chance: r.get::<f64, _>("chance"),
        };
        map.entry(monster_index).or_default().push(entry);
    }
    Ok(map)
}

/// Load NPC goods grouped by npc_index
pub async fn load_npc_goods(pool: &DbPool) -> anyhow::Result<HashMap<i32, Vec<NpcGoodsInfo>>> {
    let rows = sqlx::query("SELECT * FROM npc_goods ORDER BY npc_index").fetch_all(pool).await?;
    let mut map: HashMap<i32, Vec<NpcGoodsInfo>> = HashMap::new();
    for r in rows {
        let npc_index: i32 = r.get("npc_index");
        let entry = NpcGoodsInfo {
            npc_index,
            item_index: r.get("item_index"),
            count: r.get("count"),
            price: r.get("price"),
        };
        map.entry(npc_index).or_default().push(entry);
    }
    Ok(map)
}

/// Load NPC scripts grouped by (npc_index, page_name)
pub async fn load_npc_scripts(pool: &DbPool) -> anyhow::Result<HashMap<(i32, String), Vec<String>>> {
    let rows = sqlx::query("SELECT * FROM npc_scripts").fetch_all(pool).await?;
    let mut map: HashMap<(i32, String), Vec<String>> = HashMap::new();
    for r in rows {
        let npc_index: i32 = r.get("npc_index");
        let page_name: String = r.get("page_name");
        let lines_json: String = r.get("lines_json");
        let lines: Vec<String> = serde_json::from_str(&lines_json).unwrap_or_default();
        map.insert((npc_index, page_name.clone()), lines);
    }
    Ok(map)
}

/// Load all NPC infos from DB
pub async fn load_npc_infos(pool: &DbPool) -> anyhow::Result<Vec<NPCInfo>> {
    let rows = sqlx::query("SELECT * FROM npc_infos ORDER BY index").fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| {
        let collect_quest_indexes: Vec<i32> =
            serde_json::from_str(&r.get::<String, _>("collect_quest_indexes")).unwrap_or_default();
        let finish_quest_indexes: Vec<i32> =
            serde_json::from_str(&r.get::<String, _>("finish_quest_indexes")).unwrap_or_default();

        NPCInfo {
            index: r.get("index"),
            map_index: r.get("map_index"),
            file_name: r.get("file_name"),
            name: r.get("name"),
            x: r.get("x"),
            y: r.get("y"),
            image: r.get("image"),
            rate: r.get("rate"),
            time_visible: r.get("time_visible"),
            hour_start: r.get("hour_start"),
            minute_start: r.get("minute_start"),
            hour_end: r.get("hour_end"),
            minute_end: r.get("minute_end"),
            min_lev: r.get("min_lev"),
            max_lev: r.get("max_lev"),
            day_of_week: r.get::<Option<String>, _>("day_of_week"),
            class_required: r.get::<Option<String>, _>("class_required"),
            conquest: r.get("conquest"),
            flag_needed: r.get("flag_needed"),
            show_on_big_map: r.get::<i32, _>("show_on_big_map") != 0,
            big_map_icon: r.get("big_map_icon"),
            can_teleport_to: r.get::<i32, _>("can_teleport_to") != 0,
            conquest_visible: r.get::<i32, _>("conquest_visible") != 0,
            collect_quest_indexes,
            finish_quest_indexes,
        }
    }).collect())
}

/// Load all quest infos from DB
pub async fn load_quest_infos(pool: &DbPool) -> anyhow::Result<Vec<QuestInfo>> {
    let rows = sqlx::query("SELECT * FROM quest_infos ORDER BY index").fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| QuestInfo {
        index: r.get("index"),
        name: r.get("name"),
        group_name: r.get("group_name"),
        file_name: r.get("file_name"),
        required_min_level: r.get("required_min_level"),
        required_max_level: r.get("required_max_level"),
        required_quest: r.get("required_quest"),
        required_class: r.get("required_class"),
        quest_type: r.get("quest_type"),
        exp_reward: r.get("exp_reward"),
        gold_reward: r.get("gold_reward"),
        goto_message: r.get::<Option<String>, _>("goto_message"),
        kill_message: r.get::<Option<String>, _>("kill_message"),
        item_message: r.get::<Option<String>, _>("item_message"),
        flag_message: r.get::<Option<String>, _>("flag_message"),
        time_limit_seconds: r.get("time_limit_seconds"),
        kill_tasks: Vec::new(),
        item_tasks: Vec::new(),
        flag_tasks: Vec::new(),
    }).collect())
}

/// Parse quest .txt files and resolve monster/item names to indices.
/// Call this after `load_quest_infos`, `load_monster_infos`, and `load_item_infos`.
pub fn resolve_quest_tasks(
    quests: &mut [QuestInfo],
    quest_dir: &Path,
    monster_infos: &HashMap<i32, MonsterInfo>,
    item_infos: &HashMap<i32, ItemInfo>,
) {
    // Build name → index lookups (case-insensitive, space-stripped fallback)
    let mut monster_by_name: HashMap<String, i32> = HashMap::new();
    for (idx, info) in monster_infos {
        monster_by_name.insert(info.name.to_lowercase(), *idx);
        monster_by_name.insert(info.name.to_lowercase().replace(' ', ""), *idx);
    }

    let mut item_by_name: HashMap<String, i32> = HashMap::new();
    for (idx, info) in item_infos {
        item_by_name.insert(info.name.to_lowercase(), *idx);
        item_by_name.insert(info.name.to_lowercase().replace(' ', ""), *idx);
    }

    for quest in quests {
        let path = quest_dir.join(format!("{}.txt", quest.file_name));
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let mut current_section: Option<String> = None;
        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                current_section = Some(line.to_uppercase());
                continue;
            }
            let section = match &current_section {
                Some(s) => s.as_str(),
                None => continue,
            };
            match section {
                "[@KILLTASKS]" => {
                    if let Some(task) = parse_kill_task(line, &monster_by_name) {
                        quest.kill_tasks.push(task);
                    }
                }
                "[@ITEMTASKS]" => {
                    if let Some(task) = parse_item_task(line, &item_by_name) {
                        quest.item_tasks.push(task);
                    }
                }
                "[@FLAGTASKS]" => {
                    if let Some(task) = parse_flag_task(line) {
                        quest.flag_tasks.push(task);
                    }
                }
                _ => {}
            }
        }
    }
}

fn extract_quoted_message(line: &str) -> Option<String> {
    let first = line.find('"')?;
    let last = line.rfind('"')?;
    if first >= last {
        return None;
    }
    Some(line[first + 1..last].to_string())
}

fn parse_kill_task(line: &str, monster_by_name: &HashMap<String, i32>) -> Option<QuestKillTask> {
    let message = extract_quoted_message(line).unwrap_or_default();
    let trimmed = if let Some(idx) = line.find('"') {
        line[..idx].trim()
    } else {
        line.trim()
    };
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let name = parts[0];
    let count = parts.get(1).and_then(|s| s.parse::<i32>().ok()).unwrap_or(1);

    let name_lower = name.to_lowercase();
    let monster_index = monster_by_name
        .get(&name_lower)
        .or_else(|| monster_by_name.get(&name_lower.replace(' ', "")))
        .copied()?;

    Some(QuestKillTask {
        monster_index,
        count,
        message,
    })
}

fn parse_item_task(line: &str, item_by_name: &HashMap<String, i32>) -> Option<QuestItemTask> {
    let message = extract_quoted_message(line).unwrap_or_default();
    let trimmed = if let Some(idx) = line.find('"') {
        line[..idx].trim()
    } else {
        line.trim()
    };
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let name = parts[0];
    let count = parts.get(1).and_then(|s| s.parse::<i32>().ok()).unwrap_or(1);

    let name_lower = name.to_lowercase();
    let item_index = item_by_name
        .get(&name_lower)
        .or_else(|| item_by_name.get(&name_lower.replace(' ', "")))
        .copied()?;

    Some(QuestItemTask {
        item_index,
        count,
        message,
    })
}

fn parse_flag_task(line: &str) -> Option<QuestFlagTask> {
    let message = extract_quoted_message(line).unwrap_or_default();
    let trimmed = if let Some(idx) = line.find('"') {
        line[..idx].trim()
    } else {
        line.trim()
    };
    let number = trimmed.parse::<i32>().ok()?;
    if number < 0 {
        return None;
    }
    Some(QuestFlagTask { number, message })
}

/// Load all magic infos from DB
pub async fn load_magic_infos(pool: &DbPool) -> anyhow::Result<Vec<MagicInfo>> {
    let rows = sqlx::query("SELECT * FROM magic_infos ORDER BY name").fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| MagicInfo {
        name: r.get("name"),
        spell: r.get("spell"),
        base_cost: r.get("base_cost"),
        level_cost: r.get("level_cost"),
        icon: r.get("icon"),
        level1: r.get("level1"),
        level2: r.get("level2"),
        level3: r.get("level3"),
        need1: r.get("need1"),
        need2: r.get("need2"),
        need3: r.get("need3"),
        delay_base: r.get("delay_base"),
        delay_reduction: r.get("delay_reduction"),
        power_base: r.get("power_base"),
        power_bonus: r.get("power_bonus"),
        mpower_base: r.get("mpower_base"),
        mpower_bonus: r.get("mpower_bonus"),
        range: r.get("range"),
        multiplier_base: r.get("multiplier_base"),
        multiplier_bonus: r.get("multiplier_bonus"),
    }).collect())
}

/// Load all game shop items from DB
pub async fn load_game_shop_items(pool: &DbPool) -> anyhow::Result<Vec<GameShopItem>> {
    let rows = sqlx::query("SELECT * FROM game_shop_items ORDER BY gindex").fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| GameShopItem {
        item_index: r.get("item_index"),
        gindex: r.get("gindex"),
        gold_price: r.get("gold_price"),
        credit_price: r.get("credit_price"),
        count: r.get("count"),
        class_name: r.get("class"),
        category: r.get("category"),
        stock: r.get("stock"),
        infinite_stock: r.get::<i32, _>("infinite_stock") != 0,
        deal: r.get::<i32, _>("deal") != 0,
        top_item: r.get::<i32, _>("top_item") != 0,
        date: r.get("date"),
        can_buy_credit: r.get::<i32, _>("can_buy_credit") != 0,
        can_buy_gold: r.get::<i32, _>("can_buy_gold") != 0,
    }).collect())
}

/// Load dragon info from DB (single row). Resolves monster_index from monster_name.
pub async fn load_dragon_info(
    pool: &DbPool,
    monster_infos: &HashMap<i32, MonsterInfo>,
) -> anyhow::Result<Option<DragonInfo>> {
    let row = sqlx::query("SELECT * FROM dragon_info LIMIT 1").fetch_optional(pool).await?;
    match row {
        Some(r) => {
            let exps: Vec<i64> = serde_json::from_str(&r.get::<String, _>("exps_json")).unwrap_or_default();
            let monster_name: String = r.get("monster_name");
            let monster_index = monster_infos.values().find(|m| m.name == monster_name).map(|m| m.index);
            if monster_index.is_none() {
                tracing::warn!("Dragon references unknown monster_name='{}'", monster_name);
            }
            Ok(Some(DragonInfo {
                id: r.get("id"),
                enabled: r.get::<i32, _>("enabled") != 0,
                map_file_name: r.get("map_file_name"),
                monster_name,
                monster_index,
                body_name: r.get("body_name"),
                location_x: r.get("location_x"),
                location_y: r.get("location_y"),
                drop_area_top_x: r.get("drop_area_top_x"),
                drop_area_top_y: r.get("drop_area_top_y"),
                drop_area_bottom_x: r.get("drop_area_bottom_x"),
                drop_area_bottom_y: r.get("drop_area_bottom_y"),
                exps,
            }))
        }
        None => Ok(None),
    }
}
