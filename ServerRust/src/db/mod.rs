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

/// Initialize the SQLite database from a URL and run migrations
pub async fn init_db_pool(db_url: &str) -> anyhow::Result<DbPool> {
    let pool = SqlitePool::connect(db_url).await?;

    // Phase 1.2: SQLite WAL 模式 + 同步策略调优(生产级持久化)
    //   WAL = Write-Ahead Logging,允许并发读不阻塞写,显著提升高负载性能
    //   synchronous=NORMAL = 在 WAL 模式下是安全的,比 FULL 快 2-10 倍
    //   busy_timeout = 写锁竞争时等 5 秒而不是立刻报错
    sqlx::query("PRAGMA journal_mode=WAL").execute(&pool).await?;
    sqlx::query("PRAGMA synchronous=NORMAL").execute(&pool).await?;
    sqlx::query("PRAGMA busy_timeout=5000").execute(&pool).await?;
    // FK 禁用：INSERT OR REPLACE 在 characters 表会触发子表级联删除+重插，
    // 中间状态（character 行被删、子表引用悬空）导致 FK constraint failed。
    // 游戏服务器的数据完整性由应用层保证（save_character 用事务）。
    sqlx::query("PRAGMA foreign_keys=OFF").execute(&pool).await?;

    // Create tables if not exists
    // Phase A fix: sqlx::query() 不支持多语句;改用 raw_sql() 执行整个 schema 批次
    sqlx::raw_sql(
        r#"
        CREATE TABLE IF NOT EXISTS accounts (
            username TEXT PRIMARY KEY,
            password_hash TEXT NOT NULL,
            is_online INTEGER NOT NULL DEFAULT 0
        );
        -- PR #1169: Warehouse password columns (nullable; NULL = no password set)
        -- (The CREATE TABLE above already supports ALTER-based migration below)
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
            allow_trade INTEGER NOT NULL DEFAULT 0,
            allow_observe INTEGER NOT NULL DEFAULT 0,
            allow_group INTEGER NOT NULL DEFAULT 0,
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
        CREATE TABLE IF NOT EXISTS hero_inventory_equipment (
            character_name TEXT NOT NULL,
            slot INTEGER NOT NULL,
            item_json TEXT NOT NULL,
            PRIMARY KEY (character_name, slot),
            FOREIGN KEY (character_name) REFERENCES characters(name)
        );
        CREATE TABLE IF NOT EXISTS heroes (
            character_name TEXT NOT NULL,
            hero_index INTEGER NOT NULL,
            name TEXT NOT NULL,
            level INTEGER NOT NULL DEFAULT 1,
            class INTEGER NOT NULL DEFAULT 0,
            gender INTEGER NOT NULL DEFAULT 0,
            dead INTEGER NOT NULL DEFAULT 0,
            sealed INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (character_name, hero_index),
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
            credit_reward INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (character_name, quest_index),
            FOREIGN KEY (character_name) REFERENCES characters(name)
        );
        CREATE TABLE IF NOT EXISTS completed_quests (
            character_name TEXT NOT NULL,
            quest_index INTEGER NOT NULL,
            PRIMARY KEY (character_name, quest_index),
            FOREIGN KEY (character_name) REFERENCES characters(name)
        );
        CREATE TABLE IF NOT EXISTS player_magics (
            character_name TEXT NOT NULL,
            spell INTEGER NOT NULL,
            level INTEGER NOT NULL DEFAULT 0,
            experience INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (character_name, spell),
            FOREIGN KEY (character_name) REFERENCES characters(name)
        );
        CREATE TABLE IF NOT EXISTS hero_magics (
            character_name TEXT NOT NULL,
            spell INTEGER NOT NULL,
            level INTEGER NOT NULL DEFAULT 0,
            experience INTEGER NOT NULL DEFAULT 0,
            key INTEGER NOT NULL DEFAULT 0,
            toggled INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (character_name, spell),
            FOREIGN KEY (character_name) REFERENCES characters(name)
        );
        CREATE TABLE IF NOT EXISTS player_flags (
            character_name TEXT NOT NULL,
            flag_key TEXT NOT NULL,
            flag_value INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (character_name, flag_key),
            FOREIGN KEY (character_name) REFERENCES characters(name)
        );
        CREATE TABLE IF NOT EXISTS guilds (
            name TEXT PRIMARY KEY,
            notice_json TEXT NOT NULL DEFAULT '[]',
            gold INTEGER NOT NULL DEFAULT 0,
            storage_items_json TEXT NOT NULL DEFAULT '[]',
            experience INTEGER NOT NULL DEFAULT 0,
            level INTEGER NOT NULL DEFAULT 1,
            max_experience INTEGER NOT NULL DEFAULT 0,
            spare_points INTEGER NOT NULL DEFAULT 0,
            member_cap INTEGER NOT NULL DEFAULT 50
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
            active_level INTEGER NOT NULL DEFAULT 1,
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
            idx INTEGER PRIMARY KEY,
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
            FOREIGN KEY (map_index) REFERENCES map_infos(idx)
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
            FOREIGN KEY (map_index) REFERENCES map_infos(idx)
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
            FOREIGN KEY (map_index) REFERENCES map_infos(idx)
        );
        CREATE TABLE IF NOT EXISTS mine_zones (
            map_index INTEGER NOT NULL,
            x INTEGER NOT NULL,
            y INTEGER NOT NULL,
            size INTEGER NOT NULL DEFAULT 0,
            mine INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (map_index) REFERENCES map_infos(idx)
        );
        CREATE TABLE IF NOT EXISTS item_infos (
            idx INTEGER PRIMARY KEY,
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
            idx INTEGER PRIMARY KEY,
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
            idx INTEGER PRIMARY KEY,
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
            idx INTEGER PRIMARY KEY,
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
            credit_reward INTEGER NOT NULL DEFAULT 0,
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
            idx INTEGER PRIMARY KEY,
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
            gold INTEGER NOT NULL DEFAULT 0,
            quest_required INTEGER NOT NULL DEFAULT 0,
            group_parent_id INTEGER NOT NULL DEFAULT 0,
            group_random INTEGER NOT NULL DEFAULT 0,
            group_first INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (monster_index) REFERENCES monster_infos(idx)
        );
        CREATE INDEX IF NOT EXISTS idx_monster_drops_monster ON monster_drops(monster_index);
        CREATE TABLE IF NOT EXISTS npc_goods (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            npc_index INTEGER NOT NULL,
            item_index INTEGER NOT NULL,
            count INTEGER NOT NULL DEFAULT 1,
            price INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (npc_index) REFERENCES npc_infos(idx)
        );
        CREATE INDEX IF NOT EXISTS idx_npc_goods_npc ON npc_goods(npc_index);
        CREATE TABLE IF NOT EXISTS npc_scripts (
            npc_index INTEGER NOT NULL,
            page_name TEXT NOT NULL,
            lines_json TEXT NOT NULL DEFAULT '[]',
            PRIMARY KEY (npc_index, page_name)
        );
        CREATE TABLE IF NOT EXISTS auctions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            auction_id INTEGER NOT NULL UNIQUE,
            seller_name TEXT NOT NULL,
            item_json TEXT NOT NULL,
            price INTEGER NOT NULL DEFAULT 0,
            consignment_date INTEGER NOT NULL DEFAULT 0,
            sold INTEGER NOT NULL DEFAULT 0,
            buyer_name TEXT,
            item_type INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_auctions_sold ON auctions(sold);
        CREATE INDEX IF NOT EXISTS idx_auctions_seller ON auctions(seller_name);
        -- Recipes (crafting). C# Server loads these from Envir/Recipe/*.txt files
        -- (NOT from MirDB binary), so the Rust port persists them in SQLite instead.
        -- recipe_id mirrors C# NextRecipeID (1-based). product_* is the crafted item.
        -- Requirements (level/class/gender/flag/quest) mirror C# RecipeInfo criteria.
        CREATE TABLE IF NOT EXISTS recipes (
            recipe_id INTEGER PRIMARY KEY,
            product_item_index INTEGER NOT NULL,
            product_count INTEGER NOT NULL DEFAULT 1,
            gold_cost INTEGER NOT NULL DEFAULT 0,
            chance INTEGER NOT NULL DEFAULT 100,
            required_level INTEGER,
            required_gender INTEGER,
            required_flags TEXT NOT NULL DEFAULT '[]',
            required_quests TEXT NOT NULL DEFAULT '[]',
            required_classes TEXT NOT NULL DEFAULT '[]'
        );
        CREATE TABLE IF NOT EXISTS recipe_ingredients (
            recipe_id INTEGER NOT NULL,
            item_index INTEGER NOT NULL,
            count INTEGER NOT NULL DEFAULT 1,
            FOREIGN KEY (recipe_id) REFERENCES recipes(recipe_id)
        );
        CREATE INDEX IF NOT EXISTS idx_recipe_ingredients_recipe ON recipe_ingredients(recipe_id);
        CREATE TABLE IF NOT EXISTS recipe_tools (
            recipe_id INTEGER NOT NULL,
            item_index INTEGER NOT NULL,
            FOREIGN KEY (recipe_id) REFERENCES recipes(recipe_id)
        );
        CREATE INDEX IF NOT EXISTS idx_recipe_tools_recipe ON recipe_tools(recipe_id);
        "#
    ).execute(&pool).await?;

    // #995/#996：旧库补 monster_drops 列（safe to re-run；新库 CREATE 已含）
    let _ = sqlx::query("ALTER TABLE monster_drops ADD COLUMN gold INTEGER NOT NULL DEFAULT 0").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE monster_drops ADD COLUMN quest_required INTEGER NOT NULL DEFAULT 0").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE monster_drops ADD COLUMN group_parent_id INTEGER NOT NULL DEFAULT 0").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE monster_drops ADD COLUMN group_random INTEGER NOT NULL DEFAULT 0").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE monster_drops ADD COLUMN group_first INTEGER NOT NULL DEFAULT 0").execute(&pool).await;

    // Migration: add quest timer columns (safe to re-run)
    let _ = sqlx::query("ALTER TABLE quests ADD COLUMN start_time INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE quests ADD COLUMN time_limit_seconds INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    // Migration: add GM flag to accounts (safe to re-run)
    let _ = sqlx::query("ALTER TABLE accounts ADD COLUMN credit INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE accounts ADD COLUMN admin_account INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    // PR #1169: Warehouse password columns (safe to re-run; NULL = no password set)
    let _ = sqlx::query("ALTER TABLE accounts ADD COLUMN storage_password_hash TEXT")
        .execute(&pool).await;
    // #887: 仓库扩容字段（C# AccountInfo.HasExpandedStorage / ExpandedStorageExpiryDate，safe to re-run）
    let _ = sqlx::query("ALTER TABLE accounts ADD COLUMN has_expanded_storage INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE accounts ADD COLUMN expanded_storage_expiry_date INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE accounts ADD COLUMN storage_password_last_set INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    // #493: 地图进入规则列（C# MapInfo NoGroup/NoPets/NoIntelligentCreatures/NoHero，safe to re-run）
    let _ = sqlx::query("ALTER TABLE map_infos ADD COLUMN no_group INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE map_infos ADD COLUMN no_pets INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE map_infos ADD COLUMN no_intelligent_creatures INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE map_infos ADD COLUMN no_hero INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    // #491: 角色最后上线时间（C# LastLogoutDate，safe to re-run）
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN last_access INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    // #516: 强制改密标记（C# AccountInfo.RequirePasswordChange，safe to re-run）
    let _ = sqlx::query("ALTER TABLE accounts ADD COLUMN require_password_change INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    // #899: 背包格数（C# CharacterInfo.Inventory.Length，扩容后重登不丢，safe to re-run）
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN backpack_size INTEGER NOT NULL DEFAULT 40")
        .execute(&pool).await;
    // #932: 无经验地图（C# MapInfo.NoExperience，safe to re-run）
    let _ = sqlx::query("ALTER TABLE map_infos ADD COLUMN no_experience INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    // #935: 必须组队地图（C# MapInfo.RequiredGroup/RequiredGroupSize，safe to re-run）
    let _ = sqlx::query("ALTER TABLE map_infos ADD COLUMN required_group INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE map_infos ADD COLUMN required_group_size INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    // #480: 密码错误锁定（C# WrongPasswordCount / ExpiryDate，safe to re-run）
    let _ = sqlx::query("ALTER TABLE accounts ADD COLUMN wrong_password_count INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE accounts ADD COLUMN banned_until INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    // Migration: add key column to player_magics (safe to re-run)
    let _ = sqlx::query("ALTER TABLE player_magics ADD COLUMN key INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    // Migration: add toggled column to player_magics (safe to re-run)
    let _ = sqlx::query("ALTER TABLE player_magics ADD COLUMN toggled INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    // Migration: add hero_behaviour column to characters (safe to re-run)
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN hero_behaviour INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN allow_lover_recall INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN allow_trade INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN allow_observe INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN allow_group INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN is_mounted INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN mount_type INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN reincarnation_host TEXT")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN reincarnation_ready INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN reincarnation_expire_time INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN enable_group_recall INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN last_recall_time INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN exp_multiplier REAL NOT NULL DEFAULT 1.0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN exp_multiplier_end_tick INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN is_gm INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    // Migration: add auto_pot columns to characters (safe to re-run)
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN auto_pot_hp INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN auto_pot_mp INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN auto_pot_hp_item INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN auto_pot_mp_item INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    // Migration: add magic stats to characters
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN min_mc INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN max_mc INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN min_sc INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN max_sc INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN freezing INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN poison_attack INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN poison_recovery INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN holy INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN accuracy INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN agility INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    // 战斗公式扩展字段（AC/MAC/Luck/Crit/MagicResist/Reflect/DamageReduction 等）
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN min_ac INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN max_ac INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN min_mac INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN max_mac INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN pearl_count INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE heroes ADD COLUMN dead INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE heroes ADD COLUMN sealed INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE creatures ADD COLUMN active_level INTEGER NOT NULL DEFAULT 1")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE heroes ADD COLUMN sealed INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN can_gain_exp INTEGER NOT NULL DEFAULT 1")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN luck INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN critical_rate INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN critical_damage INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN magic_resist INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN reflect INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN damage_reduction_percent INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN attack_bonus INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN hp_drain_rate_percent INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN energy_shield_percent INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN energy_shield_hp_gain INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN bind_map_index INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN is_mentor INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN bind_x INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE characters ADD COLUMN bind_y INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    // Migration: quests（角色任务）信用奖励列（#1161 任务奖励对齐）
    let _ = sqlx::query("ALTER TABLE quests ADD COLUMN credit_reward INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    // Migration: quest_infos 信用奖励列（#1161 任务奖励对齐）
    let _ = sqlx::query("ALTER TABLE quest_infos ADD COLUMN credit_reward INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    // Migration: guilds 行会经验/等级列（#1161）
    let _ = sqlx::query("ALTER TABLE guilds ADD COLUMN experience INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE guilds ADD COLUMN level INTEGER NOT NULL DEFAULT 1")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE guilds ADD COLUMN max_experience INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE guilds ADD COLUMN spare_points INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE guilds ADD COLUMN member_cap INTEGER NOT NULL DEFAULT 50")
        .execute(&pool).await;
    // Migration: add weather_particles to old map_infos (from migrate_mirdb)
    let _ = sqlx::query("ALTER TABLE map_infos ADD COLUMN weather_particles INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    // Fix potentially broken gt column (TEXT→INTEGER from old migration).
    // 只在 gt 列类型为 TEXT 时才 DROP+ADD（避免每次重启丢数据）。
    // SQLite 的 ALTER TABLE DROP COLUMN 在 3.35+ 支持。若版本旧或列已是 INTEGER，跳过。
    let need_gt_fix = sqlx::query("SELECT typeof(gt) FROM map_infos LIMIT 1")
        .fetch_optional(&pool).await
        .ok().flatten()
        .and_then(|row| row.try_get::<String, _>("typeof(gt)").ok())
        .map(|t| t == "text")
        .unwrap_or(false);
    if need_gt_fix {
        let _ = sqlx::query("ALTER TABLE map_infos DROP COLUMN gt").execute(&pool).await;
        let _ = sqlx::query("ALTER TABLE map_infos ADD COLUMN gt INTEGER NOT NULL DEFAULT 0")
            .execute(&pool).await;
    } else {
        // 确保列存在（首次运行时可能没 gt 列）
        let _ = sqlx::query("ALTER TABLE map_infos ADD COLUMN gt INTEGER NOT NULL DEFAULT 0")
            .execute(&pool).await;
    }
    let _ = sqlx::query("ALTER TABLE map_infos ADD COLUMN gt_index INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE item_infos ADD COLUMN tool_tip TEXT NOT NULL DEFAULT ''")
        .execute(&pool).await;

    // Report logs
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS report_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            reporter_name TEXT NOT NULL,
            issue_type INTEGER NOT NULL DEFAULT 0,
            description TEXT NOT NULL DEFAULT '',
            created_at INTEGER NOT NULL DEFAULT 0
        )"#
    ).execute(&pool).await?;

    // Rental persistence
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS rentals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            item_unique_id INTEGER NOT NULL,
            item_index INTEGER NOT NULL,
            owner_name TEXT NOT NULL,
            renter_name TEXT NOT NULL,
            fee INTEGER NOT NULL DEFAULT 0,
            period_days INTEGER NOT NULL DEFAULT 7,
            started_at INTEGER NOT NULL DEFAULT 0,
            expires_at INTEGER NOT NULL DEFAULT 0,
            returned INTEGER NOT NULL DEFAULT 0
        )"#
    ).execute(&pool).await?;

    info!("SQLite database initialized: {}", db_url);
    Ok(pool)
}

    #[tokio::test]
    async fn test_save_load_heroes_roundtrip() {
        let pool = SqlitePool::connect("sqlite::memory:?cache=shared").await.unwrap();
        sqlx::query(
            "CREATE TABLE heroes (
                character_name TEXT NOT NULL,
                hero_index INTEGER NOT NULL,
                name TEXT NOT NULL,
                level INTEGER NOT NULL DEFAULT 1,
                class INTEGER NOT NULL DEFAULT 0,
                gender INTEGER NOT NULL DEFAULT 0,
                dead INTEGER NOT NULL DEFAULT 0,
                sealed INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (character_name, hero_index)
            )"
        ).execute(&pool).await.unwrap();
        let heroes = vec![
            DbHero { index: 1, name: "HeroOne".to_string(), level: 3, class: 1, gender: 0, dead: false, sealed: false },
        ];
        save_heroes(&pool, "TestChar", &heroes).await.unwrap();
        let loaded = load_heroes(&pool, "TestChar").await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "HeroOne");
        assert_eq!(loaded[0].level, 3);
        assert_eq!(loaded[0].class, 1);
        // 覆盖保存（清空）
        save_heroes(&pool, "TestChar", &[]).await.unwrap();
        assert!(load_heroes(&pool, "TestChar").await.unwrap().is_empty());
    }

/// Initialize the SQLite database from a file path and run migrations
pub async fn init_db(db_path: &Path) -> anyhow::Result<DbPool> {
    let path_str = db_path.display().to_string().replace('\\', "/");
    let db_url = format!("sqlite:{}", path_str);
    init_db_pool(&db_url).await
}

// ============================================================
// Account save/load
// ============================================================

pub async fn save_account(pool: &DbPool, account: &AccountInfo) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT OR REPLACE INTO accounts
           (username, password_hash, is_online, storage_password_hash, storage_password_last_set,
            wrong_password_count, banned_until, require_password_change,
            has_expanded_storage, expanded_storage_expiry_date)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
    .bind(&account.username)
    .bind(&account.password_hash)
    .bind(if account.is_online { 1 } else { 0 })
    .bind(account.storage_password_hash.as_deref())
    .bind(account.storage_password_last_set)
    .bind(account.wrong_password_count as i64)
    .bind(account.banned_until)
    .bind(if account.require_password_change { 1 } else { 0 })
    .bind(if account.has_expanded_storage { 1 } else { 0 })
    .bind(account.expanded_storage_expiry_date)
    .execute(pool)
    .await?;
    Ok(())
}

/// 角色摘要（登录列表用）
#[derive(Debug, Clone)]
pub struct CharacterSummary {
    pub name: String,
    pub class: u8,
    pub gender: u8,
    pub level: u16,
    pub last_access: i64,
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

/// 角色摘要列表（含 class/gender/level，登录选角用）
pub async fn list_character_summaries(pool: &DbPool, account_username: &str) -> anyhow::Result<Vec<CharacterSummary>> {
    let rows = sqlx::query(
        "SELECT name, class, gender, level, last_access FROM characters WHERE account_username = ? ORDER BY name"
    )
    .bind(account_username)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| CharacterSummary {
        name: r.get::<String, _>("name"),
        class: r.get::<i32, _>("class") as u8,
        gender: r.get::<i32, _>("gender") as u8,
        level: r.get::<i32, _>("level") as u16,
        last_access: r.try_get::<i64, _>("last_access").unwrap_or(0),
    }).collect())
}

/// 读取角色最后上线时间（unix 秒；C# CharacterInfo.LastLogoutDate）
pub async fn get_character_last_access(pool: &DbPool, character_name: &str) -> anyhow::Result<i64> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT last_access FROM characters WHERE name = ?")
        .bind(character_name)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.0).unwrap_or(0))
}

/// 更新角色最后上线时间（unix 秒；C# CharacterInfo.LastLogoutDate）
pub async fn update_last_access(pool: &DbPool, character_name: &str, now_unix: i64) -> anyhow::Result<()> {
    sqlx::query("UPDATE characters SET last_access = ? WHERE name = ?")
        .bind(now_unix)
        .bind(character_name)
        .execute(pool)
        .await?;
    Ok(())
}

/// #200：账号是否设置了仓库密码（仓库解锁门）
pub async fn account_has_storage_password(pool: &DbPool, username: &str) -> anyhow::Result<bool> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT storage_password_hash FROM accounts WHERE username = ?")
            .bind(username)
            .fetch_optional(pool)
            .await?;
    Ok(row.and_then(|r| r.0).is_some_and(|h| !h.is_empty()))
}

/// #887：更新仓库扩容状态（C# ADDSTORAGE 购买 / 过期降级）
pub async fn update_account_storage_expansion(
    pool: &DbPool,
    username: &str,
    has_expanded_storage: bool,
    expanded_storage_expiry_date: i64,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE accounts SET has_expanded_storage = ?, expanded_storage_expiry_date = ? WHERE username = ?"
    )
    .bind(if has_expanded_storage { 1 } else { 0 })
    .bind(expanded_storage_expiry_date)
    .bind(username)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn load_account(pool: &DbPool, username: &str) -> anyhow::Result<Option<AccountInfo>> {
    let row = sqlx::query(
        "SELECT username, password_hash, is_online,
                storage_password_hash, storage_password_last_set, credit,
                wrong_password_count, banned_until, require_password_change,
                has_expanded_storage, expanded_storage_expiry_date
         FROM accounts WHERE username = ?"
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| AccountInfo {
        username: r.get::<String, _>("username"),
        password_hash: r.get::<String, _>("password_hash"),
        is_online: r.get::<i32, _>("is_online") != 0,
        storage_password_hash: r.try_get::<Option<String>, _>("storage_password_hash").ok().flatten(),
        storage_password_last_set: r.try_get::<i64, _>("storage_password_last_set").unwrap_or(0),
        credit: r.try_get::<i64, _>("credit").unwrap_or(0).max(0) as u64,
        wrong_password_count: r.try_get::<i64, _>("wrong_password_count").unwrap_or(0).max(0) as u32,
        banned_until: r.try_get::<i64, _>("banned_until").unwrap_or(0),
        require_password_change: r.try_get::<i64, _>("require_password_change").unwrap_or(0) != 0,
        has_expanded_storage: r.try_get::<i64, _>("has_expanded_storage").unwrap_or(0) != 0,
        expanded_storage_expiry_date: r.try_get::<i64, _>("expanded_storage_expiry_date").unwrap_or(0),
    }))
}

pub async fn load_all_accounts(pool: &DbPool) -> anyhow::Result<Vec<AccountInfo>> {
    let rows = sqlx::query(
        "SELECT username, password_hash, is_online,
                storage_password_hash, storage_password_last_set, credit,
                wrong_password_count, banned_until, require_password_change,
                has_expanded_storage, expanded_storage_expiry_date
         FROM accounts"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| AccountInfo {
        username: r.get::<String, _>("username"),
        password_hash: r.get::<String, _>("password_hash"),
        is_online: r.get::<i32, _>("is_online") != 0,
        storage_password_hash: r.try_get::<Option<String>, _>("storage_password_hash").ok().flatten(),
        storage_password_last_set: r.try_get::<i64, _>("storage_password_last_set").unwrap_or(0),
        credit: r.try_get::<i64, _>("credit").unwrap_or(0).max(0) as u64,
        wrong_password_count: r.try_get::<i64, _>("wrong_password_count").unwrap_or(0).max(0) as u32,
        banned_until: r.try_get::<i64, _>("banned_until").unwrap_or(0),
        require_password_change: r.try_get::<i64, _>("require_password_change").unwrap_or(0) != 0,
        has_expanded_storage: r.try_get::<i64, _>("has_expanded_storage").unwrap_or(0) != 0,
        expanded_storage_expiry_date: r.try_get::<i64, _>("expanded_storage_expiry_date").unwrap_or(0),
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
    // Phase 1.2: 全量事务 — characters + inventory + friends + mail + quests
    // + creatures + refine + magics + flags 原子写入。
    // 如果任何步骤失败,整个事务回滚,不会出现半保存状态。
    let mut tx = pool.begin().await?;

    // 先删子表行：INSERT OR REPLACE 会先删旧角色行再插入，
    // 若子表仍有引用会触发立即 FK 冲突（尤其是有背包物品的角色）。
    for tbl in [
        "inventory_backpack", "inventory_equipment", "inventory_storage",
        "hero_inventory_backpack", "heroes", "friends", "blocked_list", "mail",
        "quests", "completed_quests", "player_magics", "player_flags",
        "creatures", "refine_log",
    ] {
        sqlx::query(&format!("DELETE FROM {tbl} WHERE character_name = ?"))
            .bind(&state.name)
            .execute(&mut *tx)
            .await?;
    }

    // Save character
    sqlx::query(
        r#"INSERT OR REPLACE INTO characters (
            name, account_username, schema_version, class, gender, hair,
            map_index, x, y, direction,
            attack_mode, pet_mode, level, experience, max_experience,
            hp, max_hp, mp, max_mp, min_attack, max_attack, defence,
            min_mc, max_mc, min_sc, max_sc,
            freezing, poison_attack, poison_recovery, holy, accuracy, agility,
            gold, group_id, guild_name, guild_rank,
            spouse_name, allow_mentor, mentor_name, hero_index, hero_behaviour,
            auto_pot_hp, auto_pot_mp, auto_pot_hp_item, auto_pot_mp_item,
            is_fishing, fishing_autocast, is_dead, allow_trade, allow_observe, allow_group, pk_points, pk_kill_count, can_gain_exp, pearl_count,
            last_access, bind_map_index, bind_x, bind_y, is_mentor, backpack_size
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
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
    .bind(state.min_mc)
    .bind(state.max_mc)
    .bind(state.min_sc)
    .bind(state.max_sc)
    .bind(state.freezing)
    .bind(state.poison_attack)
    .bind(state.poison_recovery)
    .bind(state.holy)
    .bind(state.accuracy)
    .bind(state.agility)
    .bind(state.inventory.gold as i64)
    .bind(state.group_id.map(|v| v as i64))
    .bind(&state.guild_name)
    .bind(state.guild_rank as i32)
    .bind(&state.spouse_name)
    .bind(if state.allow_mentor { 1 } else { 0 })
    .bind(&state.mentor_name)
    .bind(state.hero_index as i32)
    .bind(state.hero_behaviour as i32)
    .bind(state.auto_pot_hp as i64)
    .bind(state.auto_pot_mp as i64)
    .bind(state.auto_pot_hp_item)
    .bind(state.auto_pot_mp_item)
    .bind(if state.is_fishing { 1 } else { 0 })
    .bind(if state.fishing_autocast { 1 } else { 0 })
    .bind(if state.is_dead { 1 } else { 0 })
    .bind(if state.allow_trade { 1 } else { 0 })
    .bind(if state.allow_observe { 1 } else { 0 })
    .bind(if state.allow_group { 1 } else { 0 })
    .bind(state.pk_points)
    .bind(state.pk_kill_count as i32)
    .bind(if state.can_gain_exp { 1 } else { 0 })
    .bind(state.pearl_count)
    .bind(state.last_access)
    .bind(state.bind_map_index)
    .bind(state.bind_x)
    .bind(state.bind_y)
    .bind(if state.is_mentor { 1 } else { 0 })
    .bind(state.inventory.backpack.len() as i32)
    .execute(&mut *tx)
    .await?;

    // Save backpack
    save_inventory(&mut *tx, &state.name, &state.inventory).await?;

    // Save hero inventory
    save_hero_inventory(&mut *tx, &state.name, &state.hero_inventory).await?;

    // Save friends
    save_friends(&mut *tx, &state.name, &state.friend_list).await?;

    // Save mail
    save_mail(&mut *tx, &state.name, &state.mailbox).await?;

    // Save quests
    save_quests(&mut *tx, &state.name, &state.quest_log).await?;

    // Save creatures
    save_creatures(&mut *tx, &state.name, &state.creature_log).await?;

    // Save refine
    save_refine(&mut *tx, &state.name, &state.refine_log).await?;

    // Save magics
    save_magics(&mut *tx, &state.name, &state.magics).await?;

    // Save hero magics (#218)
    save_hero_magics(&mut *tx, &state.name, &state.hero_magics).await?;

    // Save flags
    save_flags(&mut *tx, &state.name, &state.flags).await?;

    // 提交事务 — 所有写入原子生效
    tx.commit().await?;
    Ok(())
}

pub async fn load_character(pool: &DbPool, character_name: &str) -> anyhow::Result<Option<PlayerState>> {
    let row = sqlx::query(
        "SELECT c.*, a.admin_account FROM characters c JOIN accounts a ON c.account_username = a.username WHERE c.name = ?"
    )
    .bind(character_name)
    .fetch_optional(pool)
    .await?;

    let row = match row {
        Some(r) => r,
        None => return Ok(None),
    };

    let mut inventory = load_inventory(pool, character_name).await?;
    // 金币持久化：load_inventory 不读 gold，这里从 characters.gold 恢复
    inventory.gold = row.get::<i64, _>("gold").max(0) as u64;
    // #899：背包扩容持久化（C# CharacterInfo.Inventory.Length；旧存档默认 40）
    let backpack_size = row.try_get::<i64, _>("backpack_size").unwrap_or(40).max(40) as usize;
    if inventory.backpack.len() < backpack_size {
        inventory.backpack.resize(backpack_size, None);
    }
    let friend_list = load_friends(pool, character_name).await?;
    let mailbox = load_mail(pool, character_name).await?;
    let quest_log = load_quests(pool, character_name).await?;
    let creature_log = load_creatures(pool, character_name).await?;
    let refine_log = load_refine(pool, character_name).await?;
    let magics = load_magics(pool, character_name).await?;
    let flags = load_flags(pool, character_name).await?;
    let hero_inventory = load_hero_inventory(pool, character_name).await?;
    let hero_magics = load_hero_magics(pool, character_name).await?;

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
        can_gain_exp: row.try_get("can_gain_exp").unwrap_or(1) != 0,
        pearl_count: row.try_get("pearl_count").unwrap_or(0),
        hp: row.get("hp"),
        max_hp: row.get("max_hp"),
        mp: row.get("mp"),
        max_mp: row.get("max_mp"),
        min_attack: row.get("min_attack"),
        max_attack: row.get("max_attack"),
        defence: row.get("defence"),
        min_mc: row.try_get("min_mc").unwrap_or(0),
        max_mc: row.try_get("max_mc").unwrap_or(0),
        min_sc: row.try_get("min_sc").unwrap_or(0),
        max_sc: row.try_get("max_sc").unwrap_or(0),
        bonus_min_attack: 0,
        bonus_max_attack: 0,
        bonus_defence: 0,
        bonus_max_hp: 0,
        bonus_max_mp: 0,
        bonus_min_mc: 0,
        bonus_max_mc: 0,
        bonus_min_sc: 0,
        bonus_max_sc: 0,
        freezing: row.try_get("freezing").unwrap_or(0),
        poison_attack: row.try_get("poison_attack").unwrap_or(0),
        poison_recovery: row.try_get("poison_recovery").unwrap_or(0),
        // C# HealthRecovery/SpellRecovery：登录后由装备加成计算（此处运行时 0）
        health_recovery: 0,
        spell_recovery: 0,
        // C# AttackSpeed/PoisonResist：登录后由装备加成计算（此处运行时 0）
        attack_speed: 0,
        poison_resist: 0,
        holy: row.try_get("holy").unwrap_or(0),
        accuracy: row.try_get("accuracy").unwrap_or(0),
        agility: row.try_get("agility").unwrap_or(0),
        min_ac: row.try_get("min_ac").unwrap_or(0),
        max_ac: row.try_get("max_ac").unwrap_or(0),
        min_mac: row.try_get("min_mac").unwrap_or(0),
        max_mac: row.try_get("max_mac").unwrap_or(0),
        bonus_min_ac: 0,
        bonus_max_ac: 0,
        bonus_min_mac: 0,
        bonus_max_mac: 0,
        luck: row.try_get("luck").unwrap_or(0),
        critical_rate: row.try_get("critical_rate").unwrap_or(0),
        critical_damage: row.try_get("critical_damage").unwrap_or(0),
        magic_resist: row.try_get("magic_resist").unwrap_or(0),
        reflect: row.try_get("reflect").unwrap_or(0),
        damage_reduction_percent: row.try_get("damage_reduction_percent").unwrap_or(0),
        attack_bonus: row.try_get("attack_bonus").unwrap_or(0),
        hp_drain_rate_percent: row.try_get("hp_drain_rate_percent").unwrap_or(0),
        energy_shield_percent: row.try_get("energy_shield_percent").unwrap_or(0),
        energy_shield_hp_gain: row.try_get("energy_shield_hp_gain").unwrap_or(0),
        poison_list: Vec::new(),
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
        hero_behaviour: row.try_get::<i32, _>("hero_behaviour").unwrap_or(0) as u8,
        hero_despawned: false,
        auto_pot_hp: row.try_get::<i64, _>("auto_pot_hp").unwrap_or(0) as u32,
        auto_pot_mp: row.try_get::<i64, _>("auto_pot_mp").unwrap_or(0) as u32,
        auto_pot_hp_item: row.try_get::<i32, _>("auto_pot_hp_item").unwrap_or(0),
        auto_pot_mp_item: row.try_get::<i32, _>("auto_pot_mp_item").unwrap_or(0),
        hero_inventory,
        hero_magics,
        refine_log,
        is_fishing: row.get::<i32, _>("is_fishing") != 0,
        is_mounted: false,
        mount_type: 0,
        is_dead: row.get::<i32, _>("is_dead") != 0,
        unlock_curse: false, // C# UnlockCurse 运行时状态，不持久化
        last_revival_time: 0, // C# LastRevivalTime 运行时状态，不持久化
        last_access: row.try_get::<i64, _>("last_access").unwrap_or(0),
        // C# 登录时 _restedCounter = (int)((Now - LastLogoutDate).TotalMinutes * 60)
        rested_counter: {
            let last = row.try_get::<i64, _>("last_access").unwrap_or(0);
            if last > 0 {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                let minutes = ((now - last).max(0) as f64 / 60.0).floor() as u32;
                minutes.saturating_mul(60)
            } else {
                0
            }
        },
        rested_exp_percent: 0,
        rested_exp_end_tick: 0,
            has_map_shout: false,
            has_server_shout: false,
            last_shout_time: 0,
        fishing_autocast: row.get::<i32, _>("fishing_autocast") != 0,
        reincarnation_host: None,
        reincarnation_ready: false,
        reincarnation_expire_time: 0,
        enable_group_recall: false,
        last_recall_time: 0,
        allow_lover_recall: row.get::<Option<i32>, _>("allow_lover_recall").map(|v| v != 0).unwrap_or(false),
        is_gm: row.get::<i32, _>("admin_account") != 0,
        has_expanded_storage: false,
        expanded_storage_expiry_date: 0,
        has_storage_password: false,
        require_storage_password: false,
        storage_password_last_set: 0,
        allow_observe: row.get::<Option<i32>, _>("allow_observe").map(|v| v != 0).unwrap_or(false),
        enable_guild_invite: false,
        allow_trade: row.get::<Option<i32>, _>("allow_trade").map(|v| v != 0).unwrap_or(false),
        allow_group: row.get::<Option<i32>, _>("allow_group").map(|v| v != 0).unwrap_or(false),
        pk_points: row.get::<i32, _>("pk_points"),
        pk_kill_count: row.get::<i32, _>("pk_kill_count") as u32,
        buffs: Vec::new(),
        magics,
        flags,
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

            bind_map_index: row.try_get("bind_map_index").unwrap_or(0),

            bind_x: row.try_get("bind_x").unwrap_or(0),

            bind_y: row.try_get("bind_y").unwrap_or(0),
            level_effects: row.try_get("level_effects").unwrap_or(0) as u16,
            is_mentor: row.try_get("is_mentor").unwrap_or(0) != 0,
            mentee_exp: 0,
            mentor_damage_bonus: false,
            newbie_exp_bonus: false,
            exp_bonus_lover_percent: 0,
            exp_bonus_mentee_percent: 0,
            exp_bonus_newbie_percent: 0,
            no_experience_map: false,
            brown_until_ms: 0,
            mount_loyalty_decrease_time: 0,
            mount_loyalty_increase_time: 0,
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

async fn save_inventory(conn: &mut sqlx::sqlite::SqliteConnection, character_name: &str, inv: &PlayerInventory) -> anyhow::Result<()> {
    // Clear existing
    sqlx::query("DELETE FROM inventory_backpack WHERE character_name = ?")
        .bind(character_name).execute(&mut *conn).await?;
    sqlx::query("DELETE FROM inventory_equipment WHERE character_name = ?")
        .bind(character_name).execute(&mut *conn).await?;
    sqlx::query("DELETE FROM inventory_storage WHERE character_name = ?")
        .bind(character_name).execute(&mut *conn).await?;

    // Backpack
    for (grid, slot) in inv.backpack.iter().enumerate() {
        if let Some(s) = slot {
            let item_json = serde_json::to_string(&s.item)?;
            sqlx::query("INSERT INTO inventory_backpack (character_name, grid, item_json) VALUES (?, ?, ?)")
                .bind(character_name).bind(grid as i32).bind(&item_json)
                .execute(&mut *conn).await?;
        }
    }

    // Equipment
    for (slot, item) in inv.equipment.iter().enumerate() {
        if let Some(item) = item {
            let item_json = serde_json::to_string(item)?;
            sqlx::query("INSERT INTO inventory_equipment (character_name, slot, item_json) VALUES (?, ?, ?)")
                .bind(character_name).bind(slot as i32).bind(&item_json)
                .execute(&mut *conn).await?;
        }
    }

    // Storage
    for (grid, slot) in inv.storage.iter().enumerate() {
        if let Some(s) = slot {
            let item_json = serde_json::to_string(&s.item)?;
            sqlx::query("INSERT INTO inventory_storage (character_name, grid, item_json) VALUES (?, ?, ?)")
                .bind(character_name).bind(grid as i32).bind(&item_json)
                .execute(&mut *conn).await?;
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

async fn save_hero_inventory(conn: &mut sqlx::sqlite::SqliteConnection, character_name: &str, inv: &PlayerInventory) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM hero_inventory_backpack WHERE character_name = ?")
        .bind(character_name).execute(&mut *conn).await?;

    for (grid, slot) in inv.backpack.iter().enumerate() {
        if let Some(s) = slot {
            let item_json = serde_json::to_string(&s.item)?;
            sqlx::query("INSERT INTO hero_inventory_backpack (character_name, grid, item_json) VALUES (?, ?, ?)")
                .bind(character_name).bind(grid as i32).bind(&item_json)
                .execute(&mut *conn).await?;
        }
    }

    // #1180：英雄装备持久化（C# Hero 装备随角色保存；此前仅存背包，换线丢失）
    sqlx::query("DELETE FROM hero_inventory_equipment WHERE character_name = ?")
        .bind(character_name).execute(&mut *conn).await?;
    for (slot, eq) in inv.equipment.iter().enumerate() {
        if let Some(item) = eq {
            let item_json = serde_json::to_string(item)?;
            sqlx::query("INSERT INTO hero_inventory_equipment (character_name, slot, item_json) VALUES (?, ?, ?)")
                .bind(character_name).bind(slot as i32).bind(&item_json)
                .execute(&mut *conn).await?;
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

    // #1180：英雄装备加载
    let eq_rows = sqlx::query(
        "SELECT slot, item_json FROM hero_inventory_equipment WHERE character_name = ?"
    )
    .bind(character_name)
    .fetch_all(pool)
    .await?;
    for row in eq_rows {
        let slot: i32 = row.get("slot");
        let item_json: String = row.get("item_json");
        if slot >= 0 && (slot as usize) < inv.equipment.len() {
            if let Ok(item) = serde_json::from_str::<mir2_shared::data::item::UserItem>(&item_json) {
                inv.equipment[slot as usize] = Some(item);
            }
        }
    }

    Ok(inv)
}

// ============================================================
// Friends save/load
// ============================================================

async fn save_friends(conn: &mut sqlx::sqlite::SqliteConnection, character_name: &str, list: &FriendList) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM friends WHERE character_name = ?")
        .bind(character_name).execute(&mut *conn).await?;
    sqlx::query("DELETE FROM blocked_list WHERE character_name = ?")
        .bind(character_name).execute(&mut *conn).await?;

    for f in &list.friends {
        sqlx::query(
            "INSERT INTO friends (character_name, friend_object_id, friend_name, memo) VALUES (?, ?, ?, ?)"
        )
        .bind(character_name).bind(f.object_id as i64).bind(&f.name).bind(&f.memo)
        .execute(&mut *conn).await?;
    }

    for b in &list.blocked {
        sqlx::query(
            "INSERT INTO blocked_list (character_name, blocked_object_id, blocked_name) VALUES (?, ?, ?)"
        )
        .bind(character_name).bind(b.object_id as i64).bind(&b.name)
        .execute(&mut *conn).await?;
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

async fn save_mail(conn: &mut sqlx::sqlite::SqliteConnection, character_name: &str, mailbox: &Mailbox) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM mail WHERE character_name = ?")
        .bind(character_name).execute(&mut *conn).await?;

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
        .execute(&mut *conn).await?;
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


/// 英雄持久化记录（#194：原始 class/gender，转换由调用方完成）
#[derive(Debug, Clone)]
pub struct DbHero {
    pub index: i32,
    pub name: String,
    pub level: u16,
    pub class: u8,
    pub gender: u8,
    pub dead: bool,
    pub sealed: bool,
}

/// 保存角色英雄列表（DELETE + INSERT，事务内）
pub async fn save_heroes(pool: &DbPool, character_name: &str, heroes: &[DbHero]) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM heroes WHERE character_name = ?")
        .bind(character_name)
        .execute(&mut *tx)
        .await?;
    for h in heroes {
        sqlx::query(
            r#"INSERT INTO heroes (character_name, hero_index, name, level, class, gender, dead, sealed) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(character_name)
        .bind(h.index)
        .bind(&h.name)
        .bind(h.level as i32)
        .bind(h.class as i32)
        .bind(h.gender as i32)
        .bind(if h.dead { 1 } else { 0 })
        .bind(if h.sealed { 1 } else { 0 })
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// 加载角色英雄列表
pub async fn load_heroes(pool: &DbPool, character_name: &str) -> anyhow::Result<Vec<DbHero>> {
    let rows = sqlx::query(
        "SELECT hero_index, name, level, class, gender, dead, sealed FROM heroes WHERE character_name = ? ORDER BY hero_index",
    )
    .bind(character_name)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| DbHero {
        index: r.get::<i32, _>("hero_index"),
        name: r.get::<String, _>("name"),
        level: r.get::<i32, _>("level") as u16,
        class: r.get::<i32, _>("class") as u8,
        gender: r.get::<i32, _>("gender") as u8,
        dead: r.try_get::<i32, _>("dead").unwrap_or(0) != 0,
        sealed: r.try_get::<i32, _>("sealed").unwrap_or(0) != 0,
    }).collect())
}

/// 插入单封邮件（用于离线玩家收邮件）
pub async fn insert_mail(pool: &DbPool, character_name: &str, mail: &MailMessage) -> anyhow::Result<()> {
    let items_json = serde_json::to_string(&mail.items)?;
    sqlx::query(
        r#"INSERT INTO mail (character_name, mail_id, sender_name, subject, body, timestamp,
            read_flag, collected, locked, gold, items_json)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
    .bind(character_name)
    .bind(mail.mail_id as i64)
    .bind(&mail.sender_name)
    .bind(&mail.subject)
    .bind(&mail.body)
    .bind(mail.timestamp)
    .bind(if mail.read { 1 } else { 0 })
    .bind(if mail.collected { 1 } else { 0 })
    .bind(if mail.locked { 1 } else { 0 })
    .bind(mail.gold as i64)
    .bind(&items_json)
    .execute(pool)
    .await?;
    Ok(())
}

// ============================================================
// Quests save/load
// ============================================================

async fn save_quests(conn: &mut sqlx::sqlite::SqliteConnection, character_name: &str, log: &QuestLog) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM quests WHERE character_name = ?")
        .bind(character_name).execute(&mut *conn).await?;
    sqlx::query("DELETE FROM completed_quests WHERE character_name = ?")
        .bind(character_name).execute(&mut *conn).await?;

    for q in &log.quests {
        let progress_json = serde_json::to_string(&q.progress)?;
        let status_str = match q.status {
            QuestStatus::Accepted => "Accepted",
            QuestStatus::InProgress => "InProgress",
            QuestStatus::Completed => "Completed",
            QuestStatus::Failed => "Failed",
        };
        sqlx::query(
            "INSERT INTO quests (character_name, quest_index, title, status, progress_json, exp_reward, gold_reward, credit_reward, start_time, time_limit_seconds)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(character_name)
        .bind(q.quest_index)
        .bind(&q.title)
        .bind(status_str)
        .bind(&progress_json)
        .bind(q.exp_reward)
        .bind(q.gold_reward as i64)
        .bind(q.credit_reward)
        .bind(q.start_time as i64)
        .bind(q.time_limit_seconds)
        .execute(&mut *conn).await?;
    }

    for qi in &log.completed_indices {
        sqlx::query("INSERT INTO completed_quests (character_name, quest_index) VALUES (?, ?)")
            .bind(character_name).bind(qi)
            .execute(&mut *conn).await?;
    }

    Ok(())
}

async fn load_quests(pool: &DbPool, character_name: &str) -> anyhow::Result<QuestLog> {
    let mut log = QuestLog::new();

    let rows = sqlx::query(
        "SELECT quest_index, title, status, progress_json, exp_reward, gold_reward, credit_reward, start_time, time_limit_seconds
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
            "Failed" => QuestStatus::Failed,
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
            credit_reward: row.get("credit_reward"),
            start_time: row.get::<i64, _>("start_time") as u64,
            time_limit_seconds: row.get("time_limit_seconds"),
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
    sqlx::query("INSERT OR REPLACE INTO guilds (name, notice_json, gold, storage_items_json, experience, level, max_experience, spare_points, member_cap) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&guild.name)
        .bind(&notice_json)
        .bind(guild.gold as i64)
        .bind(&storage_items_json)
        .bind(guild.experience)
        .bind(guild.level as i32)
        .bind(guild.max_experience)
        .bind(guild.spare_points as i32)
        .bind(guild.member_cap)
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

    let guild_rows = sqlx::query("SELECT name, notice_json, gold, storage_items_json, experience, level, max_experience, spare_points, member_cap FROM guilds")
        .fetch_all(pool)
        .await?;

    for row in guild_rows {
        let name: String = row.get("name");
        let notice: Vec<String> = serde_json::from_str(&row.get::<String, _>("notice_json")).unwrap_or_default();
        let gold: i64 = row.get("gold");
        let storage_items: Vec<Option<(mir2_shared::data::item::UserItem, u32)>> =
            serde_json::from_str(&row.get::<String, _>("storage_items_json")).unwrap_or_else(|_| vec![None; 100]);
        let experience: i64 = row.get("experience");
        let level: i32 = row.get("level");
        let max_experience: i64 = row.get("max_experience");
        let spare_points: i32 = row.get("spare_points");
        let member_cap: i32 = row.get("member_cap");

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
            buffs: Vec::new(),
            experience,
            level: level.clamp(1, 255) as u8,
            max_experience,
            spare_points: spare_points.clamp(0, 255) as u8,
            member_cap,
        });
    }

    Ok(guilds)
}

// ============================================================
// Creatures save/load
// ============================================================

async fn save_creatures(conn: &mut sqlx::sqlite::SqliteConnection, character_name: &str, log: &CreatureLog) -> anyhow::Result<()> {
    let owned_json = serde_json::to_string(&log.owned_creatures)?;

    let (active_type, active_custom_name, active_pickup_mode, active_hunger, active_enabled, active_level) =
        if let Some(c) = &log.active_creature {
            (c.creature_type as i32, c.custom_name.clone(), c.pickup_mode as i32, c.hunger as i32, if c.enabled { 1 } else { 0 }, c.level as i32)
        } else {
            (0, None, 0, 100, 0, 1)
        };

    sqlx::query(
        r#"INSERT OR REPLACE INTO creatures (
            character_name, active_type, active_custom_name, active_pickup_mode,
            active_hunger, active_enabled, active_level, owned_json, request_updates
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
    .bind(character_name)
    .bind(active_type)
    .bind(active_custom_name)
    .bind(active_pickup_mode)
    .bind(active_hunger)
    .bind(active_enabled)
    .bind(active_level)
    .bind(&owned_json)
    .bind(if log.request_updates { 1 } else { 0 })
    .execute(&mut *conn).await?;

    Ok(())
}

async fn load_creatures(pool: &DbPool, character_name: &str) -> anyhow::Result<CreatureLog> {
    let row = sqlx::query(
        "SELECT active_type, active_custom_name, active_pickup_mode, active_hunger,
                active_enabled, active_level, owned_json, request_updates
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
                    level: r.try_get::<i32, _>("active_level").unwrap_or(1).max(1) as u8,
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

async fn save_refine(conn: &mut sqlx::sqlite::SqliteConnection, character_name: &str, log: &RefineLog) -> anyhow::Result<()> {
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
    .execute(&mut *conn).await?;

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

async fn save_magics(conn: &mut sqlx::sqlite::SqliteConnection, character_name: &str, magics: &[crate::actors::player::PlayerMagic]) -> anyhow::Result<()> {
    // Delete existing magics for this character
    sqlx::query("DELETE FROM player_magics WHERE character_name = ?")
        .bind(character_name)
        .execute(&mut *conn).await?;
    // Insert current magics（#937：临时技能不持久化，C# IsTempSpell）
    for magic in magics.iter().filter(|m| !m.temp_skill) {
        sqlx::query(
            "INSERT INTO player_magics (character_name, spell, level, experience, key, toggled) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(character_name)
        .bind(magic.spell)
        .bind(magic.level as i32)
        .bind(magic.experience as i32)
        .bind(magic.key as i32)
        .bind(if magic.toggled { 1i32 } else { 0i32 })
        .execute(&mut *conn).await?;
    }
    Ok(())
}

async fn load_magics(pool: &DbPool, character_name: &str) -> anyhow::Result<Vec<crate::actors::player::PlayerMagic>> {
    let rows = sqlx::query("SELECT spell, level, experience, key, toggled FROM player_magics WHERE character_name = ?")
        .bind(character_name)
        .fetch_all(pool).await?;
    let mut magics = Vec::new();
    for r in rows {
        magics.push(crate::actors::player::PlayerMagic {
            spell: r.get("spell"),
            level: r.get::<i32, _>("level") as u8,
            experience: r.get::<i32, _>("experience") as u16,
            key: r.try_get::<i32, _>("key").unwrap_or(0) as u8,
            toggled: r.try_get::<i32, _>("toggled").unwrap_or(0) != 0,
            cast_time: 0,
            temp_skill: false,
        });
    }
    Ok(magics)
}

async fn save_hero_magics(
    conn: &mut sqlx::sqlite::SqliteConnection,
    character_name: &str,
    magics: &[crate::actors::player::PlayerMagic],
) -> anyhow::Result<()> {
    // #218：英雄魔法持久化（与 player_magics 同构）
    sqlx::query("DELETE FROM hero_magics WHERE character_name = ?")
        .bind(character_name)
        .execute(&mut *conn)
        .await?;
    for magic in magics {
        sqlx::query(
            "INSERT INTO hero_magics (character_name, spell, level, experience, key, toggled) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(character_name)
        .bind(magic.spell)
        .bind(magic.level as i32)
        .bind(magic.experience as i32)
        .bind(magic.key as i32)
        .bind(if magic.toggled { 1i32 } else { 0i32 })
        .execute(&mut *conn).await?;
    }
    Ok(())
}

async fn load_hero_magics(
    pool: &DbPool,
    character_name: &str,
) -> anyhow::Result<Vec<crate::actors::player::PlayerMagic>> {
    // #218：读取英雄魔法
    let rows = sqlx::query(
        "SELECT spell, level, experience, key, toggled FROM hero_magics WHERE character_name = ?",
    )
    .bind(character_name)
    .fetch_all(pool)
    .await?;
    let mut magics = Vec::new();
    for r in rows {
        magics.push(crate::actors::player::PlayerMagic {
            spell: r.get("spell"),
            level: r.get::<i32, _>("level") as u8,
            experience: r.get::<i32, _>("experience") as u16,
            key: r.try_get::<i32, _>("key").unwrap_or(0) as u8,
            toggled: r.try_get::<i32, _>("toggled").unwrap_or(0) != 0,
            cast_time: 0,
            temp_skill: false,
        });
    }
    Ok(magics)
}
async fn save_flags(conn: &mut sqlx::sqlite::SqliteConnection, character_name: &str, flags: &std::collections::HashMap<String, i32>) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM player_flags WHERE character_name = ?")
        .bind(character_name)
        .execute(&mut *conn).await?;
    for (key, value) in flags {
        sqlx::query("INSERT INTO player_flags (character_name, flag_key, flag_value) VALUES (?, ?, ?)")
            .bind(character_name)
            .bind(key)
            .bind(*value)
            .execute(&mut *conn).await?;
    }
    Ok(())
}

async fn load_flags(pool: &DbPool, character_name: &str) -> anyhow::Result<std::collections::HashMap<String, i32>> {
    let rows = sqlx::query("SELECT flag_key, flag_value FROM player_flags WHERE character_name = ?")
        .bind(character_name)
        .fetch_all(pool).await?;
    let mut flags = std::collections::HashMap::new();
    for r in rows {
        let key: String = r.get("flag_key");
        let value: i32 = r.get("flag_value");
        flags.insert(key, value);
    }
    Ok(flags)
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

/// Map mine zone（C# MineSpot：x/y 为中心、size 为半径）
#[derive(Debug, Clone)]
pub struct MineZoneInfo {
    pub map_index: i32,
    pub x: i32,
    pub y: i32,
    pub size: i32,
    pub mine: i32,
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
    /// 禁止组队（C# MapInfo.NoGroup；进入时解散队伍）
    pub no_group: bool,
    /// 禁止战斗宠物（C# MapInfo.NoPets）
    pub no_pets: bool,
    /// 禁止拾取宠物（C# MapInfo.NoIntelligentCreatures；进入时解散）
    pub no_intelligent_creatures: bool,
    /// 禁止英雄（C# MapInfo.NoHero；进入时解除）
    pub no_hero: bool,
    /// 禁止获得经验（C# MapInfo.NoExperience；WinExp/GainExp 入口拦截）
    pub no_experience: bool,
    /// 必须组队才能进入/停留（C# MapInfo.RequiredGroup）
    pub required_group: bool,
    /// 所需组队人数（C# MapInfo.RequiredGroupSize；实际门槛 = max(2, size)）
    pub required_group_size: i32,
    pub music: bool,
    pub no_town_teleport: bool,
    pub no_reincarnation: bool,
    pub weather_particles: bool,
    pub gt: bool,
    pub gt_index: i32,
    pub safe_zones: Vec<SafeZoneInfo>,
    pub respawns: Vec<MapRespawnInfo>,
    pub movements: Vec<MapMovementInfo>,
    pub mine_zones: Vec<MineZoneInfo>,
}

/// Item info (flat from DB, stats parsed from JSON)
#[derive(Debug, Clone, Default)]
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

impl ItemInfo {
    /// 物品是否默认已鉴定。
    ///
    /// 对齐 master C# 端 ItemInfo 设置逻辑:
    /// - start_item 永远已鉴定(任务物品/新手物品)
    /// - bool_flags bit 0 (0x01) 表示从 DB 加载时已鉴定
    /// - 其他情况需玩家用鉴定卷轴后才显示真实属性
    pub fn is_identified(&self) -> bool {
        self.start_item || (self.bool_flags & 0x01) != 0
    }
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
    pub id: i64,
    pub monster_index: i32,
    pub item_index: i32,
    pub min_count: u16,
    pub max_count: u16,
    pub chance: f64,
    /// 金币条目（C# DropInfo.Gold；>0 时落地金币而非物品）
    pub gold: u64,
    /// 任务掉落标记（C# DropInfo.QuestRequired；普通掉落跳过，任务系统发放）
    pub quest_required: bool,
    /// 组子条目归属（C# DropInfo.GroupedDrop；>0 = 父组行 id）
    pub group_parent_id: i64,
    /// 组随机（C# GROUP*：命中子条目中随机取 1）
    pub group_random: bool,
    /// 组首个（C# GROUP^：第一个命中即停）
    pub group_first: bool,
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
pub struct QuestItemReward {
    pub item_index: i32,
    pub count: u16,
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
    /// 信用奖励（C# QuestInfo.CreditReward，[@CREDITREWARD]）
    pub credit_reward: i32,
    pub goto_message: Option<String>,
    pub kill_message: Option<String>,
    pub item_message: Option<String>,
    pub flag_message: Option<String>,
    pub time_limit_seconds: i32,
    /// Parsed from quest .txt files
    pub kill_tasks: Vec<QuestKillTask>,
    pub item_tasks: Vec<QuestItemTask>,
    pub flag_tasks: Vec<QuestFlagTask>,
    pub fixed_rewards: Vec<QuestItemReward>,
    pub select_rewards: Vec<QuestItemReward>,
}

/// NPC 商品信息
#[derive(Debug, Clone)]
pub struct NpcGoodsInfo {
    pub npc_index: i32,
    pub item_index: i32,
    pub count: i32,
    pub price: i32,
    /// 当前库存（>0 表示有货，0=售罄）
    pub stock: i32,
    /// 是否无限库存
    pub infinite_stock: bool,
    /// 最大库存（用于自动补货）
    pub max_stock: i32,
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

/// A single crafting ingredient (item + quantity). Mirrors C# UserItem ingredient rows.
#[derive(Debug, Clone)]
pub struct RecipeIngredient {
    pub item_index: i32,
    pub count: u16,
}

/// Crafting recipe. Mirrors C# Server.MirDatabase.RecipeInfo.
/// In C# these are loaded from Envir/Recipe/*.txt (not MirDB); the Rust port
/// persists them in the SQLite `recipes` / `recipe_ingredients` / `recipe_tools` tables.
#[derive(Debug, Clone)]
pub struct RecipeInfo {
    pub recipe_id: i32,
    /// Index of the produced item (product UserItem.ItemIndex)
    pub product_item_index: i32,
    /// How many of the product are produced
    pub product_count: u16,
    /// Gold cost to craft
    pub gold_cost: u32,
    /// Success chance 0-100
    pub chance: u8,
    /// Ingredients consumed on a successful craft
    pub ingredients: Vec<RecipeIngredient>,
    /// Required tools (referenced but not consumed). Stored as item indices.
    pub tools: Vec<i32>,
    /// Optional level requirement (None = no requirement)
    pub required_level: Option<u16>,
    /// Optional gender requirement (None = no requirement)
    pub required_gender: Option<u8>,
    /// Required quest flags (must all be completed)
    pub required_quests: Vec<i32>,
    /// Required player flags (must all be set)
    pub required_flags: Vec<i32>,
    /// Required classes (empty = any class)
    pub required_classes: Vec<u8>,
}

/// Load all map infos from DB with nested safe_zones, respawns, movements
pub async fn load_map_infos(pool: &DbPool) -> anyhow::Result<Vec<MapInfo>> {
    // 4 queries total instead of 1 + N*3
    let rows = sqlx::query("SELECT * FROM map_infos").fetch_all(pool).await?;
    let sz_rows = sqlx::query("SELECT * FROM safe_zones").fetch_all(pool).await?;
    let rs_rows = sqlx::query("SELECT * FROM map_respawns").fetch_all(pool).await?;
    let mv_rows = sqlx::query("SELECT * FROM map_movements").fetch_all(pool).await?;
    let mz_rows = sqlx::query("SELECT * FROM mine_zones").fetch_all(pool).await?;

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

    let mut mz_by_map: HashMap<i32, Vec<MineZoneInfo>> = HashMap::new();
    for r in mz_rows {
        let mi: i32 = r.get("map_index");
        mz_by_map.entry(mi).or_default().push(MineZoneInfo {
            map_index: mi,
            x: r.get("x"),
            y: r.get("y"),
            size: r.get("size"),
            mine: r.try_get::<i32, _>("mine").unwrap_or(0),
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
        let index: i32 = row.get("idx");
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
            no_group: row.try_get::<i32, _>("no_group").unwrap_or(0) != 0,
            no_pets: row.try_get::<i32, _>("no_pets").unwrap_or(0) != 0,
            no_intelligent_creatures: row.try_get::<i32, _>("no_intelligent_creatures").unwrap_or(0) != 0,
            no_hero: row.try_get::<i32, _>("no_hero").unwrap_or(0) != 0,
            no_experience: row.try_get::<i32, _>("no_experience").unwrap_or(0) != 0,
            required_group: row.try_get::<i32, _>("required_group").unwrap_or(0) != 0,
            required_group_size: row.try_get::<i32, _>("required_group_size").unwrap_or(0),
            music: row.get::<i32, _>("music") != 0,
            no_town_teleport: row.get::<Option<i32>, _>("no_town_teleport").unwrap_or(0) != 0,
            no_reincarnation: row.get::<Option<i32>, _>("no_reincarnation").unwrap_or(0) != 0,
            weather_particles: row.try_get::<i32, _>("weather_particles").unwrap_or(0) != 0,
            gt: row.try_get::<String, _>("gt").map(|s| s == "1").unwrap_or(false),
            gt_index: row.try_get::<i32, _>("gt_index").unwrap_or(0),
            safe_zones: sz_by_map.remove(&index).unwrap_or_default(),
            respawns: rs_by_map.remove(&index).unwrap_or_default(),
            movements: mv_by_map.remove(&index).unwrap_or_default(),
            mine_zones: mz_by_map.remove(&index).unwrap_or_default(),
        });
    }

    Ok(maps)
}

/// Load all item infos from DB
pub async fn load_item_infos(pool: &DbPool) -> anyhow::Result<Vec<ItemInfo>> {
    let rows = sqlx::query("SELECT * FROM item_infos ORDER BY idx").fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| {
        let stats_json: String = r.get("stats_json");
        let stats_raw: HashMap<u8, i32> = serde_json::from_str(&stats_json)
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to parse item stats JSON for index {}: {}", r.get::<i32, _>("idx"), e);
                HashMap::new()
            });
        // DB stats_json 使用 C# Stat 枚举值（SharedRust Stat = C# + 3），统一 +3 转换供内部读取
        let stats: HashMap<u8, i32> = stats_raw.into_iter().map(|(k, v)| (k.saturating_add(3), v)).collect();
        ItemInfo {
            index: r.get("idx"),
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
    let rows = sqlx::query("SELECT * FROM monster_infos ORDER BY idx").fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| {
        let stats_json: String = r.get("stats_json");
        let stats_raw: HashMap<u8, i32> = serde_json::from_str(&stats_json)
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to parse monster stats JSON for index {}: {}", r.get::<i32, _>("idx"), e);
                HashMap::new()
            });
        // DB stats_json 使用 C# Stat 枚举值（SharedRust Stat = C# + 3），统一 +3 转换供内部读取
        let stats: HashMap<u8, i32> = stats_raw.into_iter().map(|(k, v)| (k.saturating_add(3), v)).collect();
        MonsterInfo {
            index: r.get("idx"),
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
            id: r.get::<i64, _>("id"),
            monster_index,
            item_index: r.get("item_index"),
            min_count: r.get::<i32, _>("min_count") as u16,
            max_count: r.get::<i32, _>("max_count") as u16,
            chance: r.get::<f64, _>("chance"),
            gold: r.try_get::<i64, _>("gold").unwrap_or(0).max(0) as u64,
            quest_required: r.try_get::<i32, _>("quest_required").unwrap_or(0) != 0,
            group_parent_id: r.try_get::<i64, _>("group_parent_id").unwrap_or(0),
            group_random: r.try_get::<i32, _>("group_random").unwrap_or(0) != 0,
            group_first: r.try_get::<i32, _>("group_first").unwrap_or(0) != 0,
        };
        map.entry(monster_index).or_default().push(entry);
    }
    Ok(map)
}

/// 从 C# 格式的 Drops/*.txt 文本文件导入掉落表到 SQLite。
///
/// C# 格式（每行）：`几率/总数 物品名 [数量]`，如 `1/60 BronzeSword`
/// 文件名 = 怪物名（去掉 .txt 后缀），特殊文件如 `00.txt`=通用掉落。
///
/// 参数：
/// - `drop_dir`: Drops 目录路径
/// - `monster_name_index`: 怪物名(小写) → monster_index 映射
/// - `item_name_index`: 物品名(小写) → item_index 映射
/// - `pool`: SQLite 连接池
/// 返回导入的条目数。
pub async fn import_drops_from_dir(
    drop_dir: &Path,
    monster_infos: &HashMap<i32, MonsterInfo>,
    item_name_index: &HashMap<String, i32>,
    pool: &DbPool,
) -> anyhow::Result<usize> {
    // 先检查是否已导入（避免重复）
    let existing: i32 = sqlx::query("SELECT COUNT(*) as cnt FROM monster_drops")
        .fetch_one(pool).await?
        .get::<i32, _>("cnt");
    if existing > 100 {
        tracing::info!("monster_drops already has {} rows, skipping import", existing);
        return Ok(existing as usize);
    }

    // 建怪物名(小写) → index 的反向索引
    let monster_name_index: HashMap<String, i32> = monster_infos.iter()
        .map(|(idx, m)| (m.name.to_lowercase(), *idx))
        .collect();

    let mut total = 0usize;
    let mut matched_monsters = 0usize;

    // 遍历 Drops/*.txt 文件，用文件名匹配怪物名
    // 用事务批量插入（避免逐行 fsync）
    let entries = std::fs::read_dir(drop_dir)?;
    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        if !file_name.ends_with(".txt") { continue; }

        // 文件名 → 怪物名候选（去 .txt + 去常见前缀/后缀）
        let base = file_name.trim_end_matches(".txt");
        let candidates = [
            base.to_lowercase(),                                      // ancient_axeskeleton
            base.strip_prefix("Ancient_").unwrap_or(base).to_lowercase(), // axeskeleton
            base.trim_end_matches('0').trim_end_matches('_').to_lowercase(), // 去尾部 _0
        ];

        // 查找匹配的 monster_index
        let m_idx = candidates.iter()
            .find_map(|c| monster_name_index.get(c).copied())
            .or_else(|| monster_name_index.get(&base.to_lowercase()).copied());

        let m_idx = match m_idx {
            Some(idx) => idx,
            None => continue, // 文件名不匹配任何怪物
        };

        let content = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        matched_monsters += 1;

        // #1006：C# ParseInsert——`#INSERT [相对路径]` 引入共享掉落表（递归展开，防循环）
        let mut drop_lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        for _ in 0..8 {
            let mut appended = false;
            let mut next: Vec<String> = Vec::new();
            for line in &drop_lines {
                let t = line.trim();
                if let Some(rest) = t.strip_prefix("#INSERT") {
                    let sub = rest.trim().trim_start_matches('[').trim_end_matches(']').trim();
                    if !sub.is_empty() {
                        let sub_path = drop_dir.join(sub);
                        if let Ok(sub_content) = std::fs::read_to_string(&sub_path) {
                            next.extend(sub_content.lines().map(|l| l.to_string()));
                            appended = true;
                            tracing::debug!("Drop #INSERT expanded: {} -> {}", drop_dir.display(), sub);
                        } else {
                            tracing::warn!("Drop #INSERT file not found: {}", sub_path.display());
                        }
                    }
                    // #INSERT 行本身不保留
                } else {
                    next.push(line.clone());
                }
            }
            drop_lines = next;
            if !appended {
                break;
            }
        }
        let raw_lines: Vec<&str> = drop_lines.iter().map(|l| l.trim()).collect();
        let mut li = 0usize;
        while li < raw_lines.len() {
            let line = raw_lines[li];
            li += 1;
            if line.is_empty() || line.starts_with(';') || line.starts_with("//") || line == "{" || line == "}" { continue; }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 { continue; }

            // chance/total
            let chance_str = parts[0];
            let chance = if chance_str.contains('/') {
                let frac: Vec<&str> = chance_str.split('/').collect();
                if frac.len() == 2 {
                    let n: f64 = frac[0].parse().unwrap_or(0.0);
                    let d: f64 = frac[1].parse().unwrap_or(1.0);
                    if d > 0.0 { n / d } else { 0.0 }
                } else { 0.0 }
            } else if chance_str.parse::<f64>().is_ok() {
                chance_str.parse::<f64>().unwrap_or(0.0)
            } else { 0.01 };

            // #995：金币条目（C# DropInfo：`1/10 Gold 1000`）
            if parts.get(1).map(|s| s.eq_ignore_ascii_case("gold")).unwrap_or(false) {
                let gold: u64 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
                if gold > 0 {
                    let _ = sqlx::query(
                        "INSERT INTO monster_drops (monster_index, item_index, min_count, max_count, chance, gold, quest_required, group_parent_id, group_random, group_first) VALUES (?, 0, 1, 1, ?, ?, 0, 0, 0, 0)"
                    )
                    .bind(m_idx)
                    .bind(chance)
                    .bind(gold as i64)
                    .execute(pool).await;
                    total += 1;
                }
                continue;
            }

            // #1002：组合掉落（C# `GROUP`/`GROUP*`/`GROUP^` + `{ ... }` 子表）
            if parts.get(1).map(|s| s.to_uppercase().starts_with("GROUP")).unwrap_or(false) {
                let group_random = parts[1].ends_with('*');
                let group_first = parts[1].ends_with('^');
                let res = sqlx::query(
                    "INSERT INTO monster_drops (monster_index, item_index, min_count, max_count, chance, gold, quest_required, group_parent_id, group_random, group_first) VALUES (?, 0, 1, 1, ?, 0, 0, 0, ?, ?)"
                )
                .bind(m_idx)
                .bind(chance)
                .bind(if group_random { 1i32 } else { 0i32 })
                .bind(if group_first { 1i32 } else { 0i32 })
                .execute(pool).await;
                let parent_id: i64 = res.map(|r| r.last_insert_rowid()).unwrap_or(0);
                total += 1;
                // 消费 `{ ... }` 内的子条目
                while li < raw_lines.len() {
                    let st = raw_lines[li];
                    li += 1;
                    if st == "}" { break; }
                    if st.is_empty() || st.starts_with(';') || st.starts_with("//") { continue; }
                    let sub_parts: Vec<&str> = st.split_whitespace().collect();
                    if sub_parts.len() < 2 { continue; }
                    let sub_chance = if sub_parts[0].contains('/') {
                        let frac: Vec<&str> = sub_parts[0].split('/').collect();
                        if frac.len() == 2 {
                            let n: f64 = frac[0].parse().unwrap_or(0.0);
                            let d: f64 = frac[1].parse().unwrap_or(1.0);
                            if d > 0.0 { n / d } else { 0.0 }
                        } else { 0.0 }
                    } else { 0.01 };
                    // 子条目金币
                    if sub_parts.get(1).map(|s| s.eq_ignore_ascii_case("gold")).unwrap_or(false) {
                        let gold: u64 = sub_parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
                        if gold > 0 {
                            let _ = sqlx::query(
                                "INSERT INTO monster_drops (monster_index, item_index, min_count, max_count, chance, gold, quest_required, group_parent_id, group_random, group_first) VALUES (?, 0, 1, 1, ?, ?, 0, ?, 0, 0)"
                            )
                            .bind(m_idx)
                            .bind(sub_chance)
                            .bind(gold as i64)
                            .bind(parent_id)
                            .execute(pool).await;
                            total += 1;
                        }
                        continue;
                    }
                    // 子条目物品
                    let mut sp = sub_parts.to_vec();
                    let sub_quest = sp.last().map(|s| s.eq_ignore_ascii_case("q")).unwrap_or(false);
                    if sub_quest { sp.pop(); }
                    let sub_item_name = if sp.len() >= 3 && sp[sp.len()-1].parse::<u16>().is_ok() {
                        sp[1..sp.len()-1].join(" ")
                    } else {
                        sp[1..].join(" ")
                    };
                    let sub_count: u16 = if sp.len() >= 3 && sp[sp.len()-1].parse::<u16>().is_ok() {
                        sp[sp.len()-1].parse().unwrap_or(1)
                    } else { 1 };
                    let sub_idx = item_name_index.get(&sub_item_name.to_lowercase()).copied()
                        .or_else(|| item_name_index.get(&sub_item_name.to_lowercase().replace(' ', "")).copied());
                    if let Some(sidx) = sub_idx {
                        let _ = sqlx::query(
                            "INSERT INTO monster_drops (monster_index, item_index, min_count, max_count, chance, gold, quest_required, group_parent_id, group_random, group_first) VALUES (?, ?, ?, ?, ?, 0, ?, ?, 0, 0)"
                        )
                        .bind(m_idx).bind(sidx)
                        .bind(sub_count as i32).bind(sub_count as i32)
                        .bind(sub_chance)
                        .bind(if sub_quest { 1i32 } else { 0i32 })
                        .bind(parent_id)
                        .execute(pool).await;
                        total += 1;
                    }
                }
                continue;
            }

            // #996：QuestRequired 标记（C# `1/10 ItemName Q`，行尾 Q）
            let mut parts_vec = parts.to_vec();
            let quest_required = parts_vec.last()
                .map(|s| s.eq_ignore_ascii_case("q"))
                .unwrap_or(false);
            if quest_required {
                parts_vec.pop();
            }

            // 物品名（可能含空格，取最后一个数字为 count）
            let item_name = if parts_vec.len() >= 3 && parts_vec[parts_vec.len()-1].parse::<u16>().is_ok() {
                parts_vec[1..parts_vec.len()-1].join(" ")
            } else {
                parts_vec[1..].join(" ")
            };
            let count: u16 = if parts_vec.len() >= 3 && parts_vec[parts_vec.len()-1].parse::<u16>().is_ok() {
                parts_vec[parts_vec.len()-1].parse().unwrap_or(1)
            } else { 1 };

            // 物品名 → item_index（精确 + 去空格模糊）
            let i_idx = item_name_index.get(&item_name.to_lowercase()).copied()
                .or_else(|| item_name_index.get(&item_name.to_lowercase().replace(' ', "")).copied());
            let i_idx = match i_idx {
                Some(idx) => idx,
                None => continue,
            };

            let _ = sqlx::query(
                "INSERT INTO monster_drops (monster_index, item_index, min_count, max_count, chance, gold, quest_required, group_parent_id, group_random, group_first) VALUES (?, ?, ?, ?, ?, 0, ?, 0, 0, 0)"
            )
            .bind(m_idx).bind(i_idx)
            .bind(count as i32).bind(count as i32)
            .bind(chance)
            .bind(if quest_required { 1i32 } else { 0i32 })
            .execute(pool).await;
            total += 1;
        }
    }
    tracing::info!("Imported {} drop entries for {} monsters from {}", total, matched_monsters, drop_dir.display());
    Ok(total)
}

/// Load NPC goods grouped by npc_index
pub async fn load_npc_goods(pool: &DbPool) -> anyhow::Result<HashMap<i32, Vec<NpcGoodsInfo>>> {
    let rows = sqlx::query("SELECT * FROM npc_goods ORDER BY npc_index").fetch_all(pool).await?;
    let mut map: HashMap<i32, Vec<NpcGoodsInfo>> = HashMap::new();
    for r in rows {
        let npc_index: i32 = r.get("npc_index");
        // 老库没有 stock/infinite_stock 列：NPC 商店默认无限库存（C# 语义）
        let stock: i32 = r.try_get("stock").unwrap_or(i32::MAX);
        let infinite: i32 = r.try_get("infinite_stock").unwrap_or(1);
        let entry = NpcGoodsInfo {
            npc_index,
            item_index: r.get("item_index"),
            count: r.get("count"),
            price: r.get("price"),
            stock,
            infinite_stock: infinite != 0,
            max_stock: stock,
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

/// 从 C# NPC 脚本目录导入脚本到 DB。
///
/// 遍历 npc_infos，按 file_name 读取对应 .txt 脚本文件，
/// 按 `[@section]` 分段存储到 npc_scripts 表（每段一行 JSON 数组）。
pub async fn import_npc_scripts_from_dir(
    npc_dir: &Path,
    npc_infos: &[NPCInfo],
    pool: &DbPool,
) -> anyhow::Result<usize> {
    // 检查是否已导入
    let existing: i32 = sqlx::query("SELECT COUNT(*) as cnt FROM npc_scripts")
        .fetch_one(pool).await?
        .get::<i32, _>("cnt");
    if existing > 100 {
        tracing::info!("npc_scripts already has {} rows, skipping import", existing);
        return Ok(existing as usize);
    }

    let mut total = 0usize;
    let mut matched = 0usize;

    for info in npc_infos {
        if info.file_name.is_empty() { continue; }

        // C# file_name 是相对 NPCPath 的路径（如 BichonProvince\BichonWall\Blacksmith-0103）
        // 转换为实际文件路径
        let rel_path = info.file_name.replace('\\', "/");
        let file_path = npc_dir.join(format!("{}.txt", rel_path));

        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(_) => continue, // 文件不存在，跳过
        };

        // 按 [section] 分段
        let mut current_section: Option<String> = None;
        let mut current_lines: Vec<String> = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // 检测 section 开头：[@name] 或 [TRADE] 等大写标签
            if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() > 2 {
                // 保存前一个 section
                if let Some(ref section) = current_section {
                    if !current_lines.is_empty() {
                        let key = format!("[{}]", section.to_uppercase());
                        let json = serde_json::to_string(&current_lines).unwrap_or_else(|_| "[]".to_string());
                        let _ = sqlx::query(
                            "INSERT OR REPLACE INTO npc_scripts (npc_index, page_name, lines_json) VALUES (?, ?, ?)"
                        )
                        .bind(info.index).bind(&key).bind(&json)
                        .execute(pool).await;
                        total += 1;
                    }
                }
                // 开始新 section（提取 @name 或大写标签名）
                let inner = &trimmed[1..trimmed.len()-1];
                current_section = Some(inner.to_string());
                current_lines = Vec::new();
                continue;
            }

            // #INSERT：内联引用的文件内容（对齐 C# ParseInsert）
            if trimmed.starts_with("#INSERT") {
                // 格式：#INSERT [相对路径\文件名.txt] @section
                // 取方括号内的路径
                if let (Some(open), Some(close)) = (trimmed.find('['), trimmed.find(']')) {
                    if close > open {
                        let rel_path = &trimmed[open+1..close];
                        // 路径相对于 Envir/ 目录
                        let insert_path = npc_dir.parent() // Envir/
                            .map(|p| p.join(rel_path.replace('\\', "/")))
                            .unwrap_or_else(|| std::path::PathBuf::from(rel_path));
                        if let Ok(inserted) = std::fs::read_to_string(&insert_path) {
                            // 把引用文件的内容追加到当前 lines（跳过它自己的 #INSERT 避免递归）
                            for ins_line in inserted.lines() {
                                let ins_trim = ins_line.trim();
                                if ins_trim.starts_with("#INSERT") { continue; }
                                current_lines.push(ins_line.to_string());
                            }
                        }
                    }
                }
                continue;
            }

            // 收集所有行（包括注释、空行、#IF/#SAY 等）
            current_lines.push(line.to_string());
        }

        // 保存最后一个 section
        if let Some(ref section) = current_section {
            if !current_lines.is_empty() {
                let key = format!("[{}]", section.to_uppercase());
                let json = serde_json::to_string(&current_lines).unwrap_or_else(|_| "[]".to_string());
                let _ = sqlx::query(
                    "INSERT OR REPLACE INTO npc_scripts (npc_index, page_name, lines_json) VALUES (?, ?, ?)"
                )
                .bind(info.index).bind(&key).bind(&json)
                .execute(pool).await;
                total += 1;
            }
        }
        matched += 1;
    }

    tracing::info!("Imported {} NPC script pages for {} NPCs from {}", total, matched, npc_dir.display());
    Ok(total)
}
pub async fn load_npc_infos(pool: &DbPool) -> anyhow::Result<Vec<NPCInfo>> {
    let rows = sqlx::query("SELECT * FROM npc_infos ORDER BY idx").fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| {
        let collect_quest_indexes: Vec<i32> =
            serde_json::from_str(&r.get::<String, _>("collect_quest_indexes")).unwrap_or_default();
        let finish_quest_indexes: Vec<i32> =
            serde_json::from_str(&r.get::<String, _>("finish_quest_indexes")).unwrap_or_default();

        NPCInfo {
            index: r.get("idx"),
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
    let rows = sqlx::query("SELECT * FROM quest_infos ORDER BY idx").fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| QuestInfo {
        index: r.get("idx"),
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
        credit_reward: r.get("credit_reward"),
        goto_message: r.get::<Option<String>, _>("goto_message"),
        kill_message: r.get::<Option<String>, _>("kill_message"),
        item_message: r.get::<Option<String>, _>("item_message"),
        flag_message: r.get::<Option<String>, _>("flag_message"),
        time_limit_seconds: r.get("time_limit_seconds"),
        kill_tasks: Vec::new(),
        item_tasks: Vec::new(),
        flag_tasks: Vec::new(),
        fixed_rewards: Vec::new(),
        select_rewards: Vec::new(),
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
                "[@FIXEDREWARDS]" => {
                    if let Some(reward) = parse_reward(line, &item_by_name) {
                        quest.fixed_rewards.push(reward);
                    }
                }
                "[@SELECTREWARDS]" => {
                    if let Some(reward) = parse_reward(line, &item_by_name) {
                        quest.select_rewards.push(reward);
                    }
                }
                _ => {}
            }
        }
    }
}

fn parse_reward(line: &str, item_by_name: &HashMap<String, i32>) -> Option<QuestItemReward> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let name = parts[0];
    let count = parts.get(1).and_then(|s| s.parse::<u16>().ok()).unwrap_or(1);

    let name_lower = name.to_lowercase();
    let item_index = item_by_name
        .get(&name_lower)
        .or_else(|| item_by_name.get(&name_lower.replace(' ', "")))
        .copied()?;

    Some(QuestItemReward { item_index, count })
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

/// Load all craft recipes from DB, joining ingredients/tools.
///
/// NOTE on data source: C# `Server.MirDatabase.RecipeInfo` does NOT read from the
/// Server.MirDB binary; it loads recipes from `Envir/Recipe/*.txt` files at runtime
/// (see `Envir.cs` line ~3309 and `RecipeInfo.LoadIngredients`). Because the Rust
/// migration tool only reads Server.MirDB, recipes are seeded into SQLite as default
/// data (see `migrate_mirdb.rs` `seed_default_recipes`) and loaded here. If the
/// `recipes` table is empty, the caller should fall back to hardcoded defaults.
pub async fn load_recipe_infos(pool: &DbPool) -> anyhow::Result<Vec<RecipeInfo>> {
    let rows = sqlx::query("SELECT * FROM recipes ORDER BY recipe_id")
        .fetch_all(pool).await?;
    let ing_rows = sqlx::query("SELECT * FROM recipe_ingredients").fetch_all(pool).await?;
    let tool_rows = sqlx::query("SELECT * FROM recipe_tools").fetch_all(pool).await?;

    // Index children by recipe_id
    let mut ing_by_recipe: HashMap<i32, Vec<RecipeIngredient>> = HashMap::new();
    for r in ing_rows {
        let rid: i32 = r.get("recipe_id");
        ing_by_recipe.entry(rid).or_default().push(RecipeIngredient {
            item_index: r.get("item_index"),
            count: r.get::<i32, _>("count") as u16,
        });
    }
    let mut tools_by_recipe: HashMap<i32, Vec<i32>> = HashMap::new();
    for r in tool_rows {
        let rid: i32 = r.get("recipe_id");
        tools_by_recipe.entry(rid).or_default().push(r.get("item_index"));
    }

    Ok(rows.into_iter().map(|r| {
        let recipe_id: i32 = r.get("recipe_id");
        let required_quests: Vec<i32> =
            serde_json::from_str(&r.get::<String, _>("required_quests")).unwrap_or_default();
        let required_flags: Vec<i32> =
            serde_json::from_str(&r.get::<String, _>("required_flags")).unwrap_or_default();
        let required_classes: Vec<u8> =
            serde_json::from_str(&r.get::<String, _>("required_classes")).unwrap_or_default();
        RecipeInfo {
            recipe_id,
            product_item_index: r.get("product_item_index"),
            product_count: r.get::<i32, _>("product_count") as u16,
            gold_cost: r.get::<i64, _>("gold_cost") as u32,
            chance: r.get::<i32, _>("chance") as u8,
            ingredients: ing_by_recipe.remove(&recipe_id).unwrap_or_default(),
            tools: tools_by_recipe.remove(&recipe_id).unwrap_or_default(),
            required_level: r.get::<Option<i64>, _>("required_level").map(|v| v as u16),
            required_gender: r.get::<Option<i64>, _>("required_gender").map(|v| v as u8),
            required_quests,
            required_flags,
            required_classes,
        }
    }).collect())
}

/// 从 C# Recipe/*.txt 导入合成配方到 DB。
///
/// 格式：
/// ```text
/// [Recipe]
/// Amount 10          ; 产物数量
/// Chance 80          ; 成功率
/// Gold 100           ; 金币消耗
///
/// [Tools]
/// ToolItemName       ; 工具（不消耗）
///
/// [Ingredients]
/// BlackThread 3      ; 材料名 数量
/// LargeBone 1
/// ```
/// 产物从文件名推断（去掉括号和 .txt）。
pub async fn import_recipes_from_dir(
    recipe_dir: &Path,
    item_name_index: &HashMap<String, i32>,
    pool: &DbPool,
) -> anyhow::Result<usize> {
    let existing: i32 = sqlx::query("SELECT COUNT(*) as cnt FROM recipes")
        .fetch_one(pool).await?.get::<i32, _>("cnt");
    if existing > 100 {
        tracing::info!("recipes already has {} rows, skipping import", existing);
        return Ok(existing as usize);
    }

    let mut total = 0usize;
    let entries = std::fs::read_dir(recipe_dir)?;
    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        if !file_name.ends_with(".txt") { continue; }
        let content = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // 从文件名推断产物物品名
        let product_name = file_name.trim_end_matches(".txt")
            .trim_start_matches('(').trim_end_matches(')');
        let product_index = item_name_index.get(&product_name.to_lowercase())
            .or_else(|| item_name_index.get(&product_name.to_lowercase().replace(' ', "")))
            .copied()
            .unwrap_or(0);

        let mut amount = 1u16;
        let mut chance = 100u8;
        let mut gold_cost = 0u32;
        let mut ingredients: Vec<(String, u16)> = Vec::new();
        let mut tools: Vec<String> = Vec::new();
        let mut section = String::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                section = trimmed.trim_matches(|c| c == '[' || c == ']').to_lowercase();
                continue;
            }
            if trimmed.is_empty() || trimmed.starts_with(';') { continue; }
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            match section.as_str() {
                "recipe" => {
                    if parts.len() >= 2 {
                        match parts[0].to_uppercase().as_str() {
                            "AMOUNT" => amount = parts[1].parse().unwrap_or(1),
                            "CHANCE" => chance = parts[1].parse().unwrap_or(100),
                            "GOLD" => gold_cost = parts[1].parse().unwrap_or(0),
                            _ => {}
                        }
                    }
                }
                "ingredients" => {
                    if !parts.is_empty() {
                        let name = parts[0].to_string();
                        let count: u16 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
                        ingredients.push((name, count));
                    }
                }
                "tools" => {
                    if !parts.is_empty() { tools.push(parts[0].to_string()); }
                }
                _ => {}
            }
        }

        if product_index == 0 && ingredients.is_empty() { continue; }

        // 插入 recipe
        let recipe_id = product_index; // 用产物 index 作为 recipe_id
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO recipes (recipe_id, product_item_index, product_count, gold_cost, chance) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(recipe_id).bind(product_index).bind(amount as i32)
        .bind(gold_cost as i64).bind(chance as i32)
        .execute(pool).await;

        // 插入 ingredients
        for (name, count) in &ingredients {
            let idx = item_name_index.get(&name.to_lowercase())
                .or_else(|| item_name_index.get(&name.to_lowercase().replace(' ', "")))
                .copied().unwrap_or(0);
            if idx > 0 {
                let _ = sqlx::query("INSERT INTO recipe_ingredients (recipe_id, item_index, count) VALUES (?, ?, ?)")
                    .bind(recipe_id).bind(idx).bind(*count as i32)
                    .execute(pool).await;
            }
        }
        // 插入 tools
        for name in &tools {
            let idx = item_name_index.get(&name.to_lowercase())
                .or_else(|| item_name_index.get(&name.to_lowercase().replace(' ', "")))
                .copied().unwrap_or(0);
            if idx > 0 {
                let _ = sqlx::query("INSERT INTO recipe_tools (recipe_id, item_index) VALUES (?, ?)")
                    .bind(recipe_id).bind(idx)
                    .execute(pool).await;
            }
        }
        total += 1;
    }
    tracing::info!("Imported {} recipes from {}", total, recipe_dir.display());
    Ok(total)
}

/// 从 NPC 脚本的 [Trade] 段导入商品到 npc_goods 表。
///
/// 遍历已导入的 npc_scripts，如果某个 NPC 的 `[@MAIN]` 页包含 [Trade] 段，
/// 解析其中的 `ItemName [count]` 行并插入 npc_goods。
pub async fn import_npc_goods_from_scripts(
    pool: &DbPool,
    npc_scripts: &HashMap<(i32, String), Vec<String>>,
    item_name_index: &HashMap<String, i32>,
) -> anyhow::Result<usize> {
    let existing: i32 = sqlx::query("SELECT COUNT(*) as cnt FROM npc_goods")
        .fetch_one(pool).await?.get::<i32, _>("cnt");
    if existing > 100 {
        tracing::info!("npc_goods already has {} rows, skipping import", existing);
        return Ok(existing as usize);
    }

    let mut total = 0usize;
    // npc_scripts 按 section 分段存储，[TRADE] 段的 page_name = "[TRADE]"
    for ((npc_index, page), lines) in npc_scripts {
        // 只处理 [TRADE] 页
        if !page.eq_ignore_ascii_case("[TRADE]") { continue; }

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('[') { continue; }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() { continue; }
            let item_name = parts[0].to_string();
            let count: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            let idx = item_name_index.get(&item_name.to_lowercase())
                .or_else(|| item_name_index.get(&item_name.to_lowercase().replace(' ', "")))
                .copied().unwrap_or(0);
            if idx > 0 {
                let _ = sqlx::query(
                    "INSERT INTO npc_goods (npc_index, item_index, count, price) VALUES (?, ?, ?, 0)"
                )
                .bind(npc_index).bind(idx).bind(count)
                .execute(pool).await;
                total += 1;
            }
        }
    }
    tracing::info!("Imported {} NPC goods entries", total);
    Ok(total)
}

pub async fn save_auction(
    pool: &DbPool,
    auction_id: i64,
    seller_name: &str,
    item_json: &str,
    price: i64,
    consignment_date: i64,
    item_type: i32,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT INTO auctions (auction_id, seller_name, item_json, price, consignment_date, sold, item_type)
           VALUES (?, ?, ?, ?, ?, 0, ?)"#
    )
    .bind(auction_id)
    .bind(seller_name)
    .bind(item_json)
    .bind(price)
    .bind(consignment_date)
    .bind(item_type)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn load_all_auctions(
    pool: &DbPool,
) -> anyhow::Result<Vec<(i64, String, String, i64, i64, i64, i64, Option<String>)>> {
    let rows = sqlx::query("SELECT auction_id, seller_name, item_json, price, consignment_date, sold, item_type, buyer_name FROM auctions ORDER BY consignment_date DESC")
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|r| {
        (
            r.get::<i64, _>("auction_id"),
            r.get::<String, _>("seller_name"),
            r.get::<String, _>("item_json"),
            r.get::<i64, _>("price"),
            r.get::<i64, _>("consignment_date"),
            r.get::<i64, _>("sold"),
            r.get::<i64, _>("item_type"),
            r.get::<Option<String>, _>("buyer_name"),
        )
    }).collect())
}

pub async fn mark_auction_sold(
    pool: &DbPool,
    auction_id: i64,
    buyer_name: &str,
) -> anyhow::Result<bool> {
    let result = sqlx::query(
        "UPDATE auctions SET sold = 1, buyer_name = ? WHERE auction_id = ? AND sold = 0"
    )
    .bind(buyer_name)
    .bind(auction_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_auction(
    pool: &DbPool,
    auction_id: i64,
) -> anyhow::Result<bool> {
    let result = sqlx::query("DELETE FROM auctions WHERE auction_id = ?")
        .bind(auction_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    async fn temp_pool() -> DbPool {
        let pool = SqlitePool::connect("sqlite::memory:?cache=shared").await.unwrap();
        sqlx::query(
            "CREATE TABLE player_flags (
                character_name TEXT NOT NULL,
                flag_key TEXT NOT NULL,
                flag_value INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (character_name, flag_key)
            )"
        ).execute(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_save_load_flags_roundtrip() {
        let pool = temp_pool().await;
        let mut flags = HashMap::new();
        flags.insert("quest_started".to_string(), 1);
        flags.insert("npc_talk_count".to_string(), 5);

        let mut conn = pool.acquire().await.unwrap(); save_flags(&mut *conn, "Hero", &flags).await.unwrap();
        let loaded = load_flags(&pool, "Hero").await.unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.get("quest_started"), Some(&1));
        assert_eq!(loaded.get("npc_talk_count"), Some(&5));
    }

    #[tokio::test]
    async fn test_save_flags_overwrites() {
        let pool = temp_pool().await;
        let mut flags = HashMap::new();
        flags.insert("key".to_string(), 10);
        let mut conn = pool.acquire().await.unwrap(); save_flags(&mut *conn, "Hero", &flags).await.unwrap();

        let mut flags2 = HashMap::new();
        flags2.insert("key".to_string(), 20);
        flags2.insert("new_key".to_string(), 30);
        let mut conn = pool.acquire().await.unwrap(); save_flags(&mut *conn, "Hero", &flags2).await.unwrap();

        let loaded = load_flags(&pool, "Hero").await.unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.get("key"), Some(&20));
        assert_eq!(loaded.get("new_key"), Some(&30));
    }

    #[tokio::test]
    async fn test_load_flags_empty() {
        let pool = temp_pool().await;
        let loaded = load_flags(&pool, "Nobody").await.unwrap();
        assert!(loaded.is_empty());
    }

    #[tokio::test]
    async fn test_load_item_infos_converts_stats_keys() {
        // DB stats_json 使用 C# Stat key（HP=12/Luck=15），加载时应 +3 转 SharedRust（HP=15/Luck=18）
        let pool = init_db_pool("sqlite::memory:?cache=shared").await.unwrap();
        sqlx::query("INSERT INTO item_infos (idx, name, stats_json) VALUES (1, 'TestPotion', '{\"12\":30,\"15\":50}')")
            .execute(&pool).await.unwrap();
        let infos = load_item_infos(&pool).await.unwrap();
        assert_eq!(infos.len(), 1);
        let info = &infos[0];
        assert_eq!(info.stats.get(&15), Some(&30)); // C# HP=12 -> SharedRust HP=15
        assert_eq!(info.stats.get(&18), Some(&50)); // C# Luck=15 -> SharedRust Luck=18
    }

    #[tokio::test]
    async fn test_account_storage_expansion_roundtrip() {
        // #887：仓库扩容字段持久化（C# AccountInfo.HasExpandedStorage / ExpandedStorageExpiryDate）
        let pool = init_db_pool("sqlite::memory:?cache=shared").await.unwrap();
        let mut account = AccountInfo {
            username: "storagetest".to_string(),
            password_hash: "x".to_string(),
            is_online: false,
            storage_password_hash: None,
            storage_password_last_set: 0,
            credit: 0,
            wrong_password_count: 0,
            banned_until: 0,
            require_password_change: false,
            has_expanded_storage: true,
            expanded_storage_expiry_date: 1_800_000_000,
        };
        save_account(&pool, &account).await.unwrap();
        let loaded = load_account(&pool, "storagetest").await.unwrap().expect("account exists");
        assert!(loaded.has_expanded_storage);
        assert_eq!(loaded.expanded_storage_expiry_date, 1_800_000_000);

        // 过期降级：update → 重新加载应读到 false / 0
        update_account_storage_expansion(&pool, "storagetest", false, 0).await.unwrap();
        let loaded = load_account(&pool, "storagetest").await.unwrap().expect("account exists");
        assert!(!loaded.has_expanded_storage);
        assert_eq!(loaded.expanded_storage_expiry_date, 0);

        // 重新购买续期：update → 再次读到 true / 新到期时间
        account.has_expanded_storage = true;
        account.expanded_storage_expiry_date = 1_800_864_000;
        save_account(&pool, &account).await.unwrap();
        let loaded = load_account(&pool, "storagetest").await.unwrap().expect("account exists");
        assert!(loaded.has_expanded_storage);
        assert_eq!(loaded.expanded_storage_expiry_date, 1_800_864_000);
    }
}

/// 读取账户积分（NPC 脚本 CHECKCREDIT）
pub async fn get_account_credit(pool: &DbPool, username: &str) -> anyhow::Result<u64> {
    let row = sqlx::query("SELECT credit FROM accounts WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.get::<i64, _>("credit").max(0) as u64).unwrap_or(0))
}

/// 增加/减少账户积分（delta 为负数表示减少，下限 0；NPC 脚本 GIVECREDIT/TAKECREDIT）
pub async fn add_account_credit(pool: &DbPool, username: &str, delta: i64) -> anyhow::Result<()> {
    sqlx::query("UPDATE accounts SET credit = MAX(credit + ?, 0) WHERE username = ?")
        .bind(delta)
        .bind(username)
        .execute(pool)
        .await?;
    Ok(())
}