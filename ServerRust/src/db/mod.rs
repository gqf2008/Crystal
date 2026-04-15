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
            name, account_username, schema_version, map_index, x, y, direction,
            attack_mode, pet_mode, level, experience, max_experience,
            hp, max_hp, mp, max_mp, min_attack, max_attack, defence,
            gold, group_id, guild_name, guild_rank,
            spouse_name, allow_mentor, mentor_name, hero_index,
            is_fishing, fishing_autocast
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
    .bind(&state.name)
    .bind(account_username)
    .bind(1i32)
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
        fishing_autocast: row.get::<i32, _>("fishing_autocast") != 0,
        reincarnation_host: None,
        reincarnation_ready: false,
        reincarnation_expire_time: 0,
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
