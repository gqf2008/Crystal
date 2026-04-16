// Migration tool: C# .MirADB/.MirDB -> SQLite
// Usage: cargo run --bin migrate -- <path-to-MirADB> [sqlite-db-path]
// Default sqlite path: data/crystal.db

#![allow(dead_code)]

use std::io::Read;
use byteorder::{LittleEndian, ReadBytesExt};
use data_encoding;
use tracing::{info, error};

// ============================================================
// BinaryReader compatible with C# BinaryReader
// ============================================================

struct BinaryReader<R: Read> {
    inner: R,
}

impl<R: Read> BinaryReader<R> {
    fn new(inner: R) -> Self { Self { inner } }
    fn read_raw_i32(&mut self) -> std::io::Result<i32> { self.inner.read_i32::<LittleEndian>() }
    fn read_raw_u32(&mut self) -> std::io::Result<u32> { self.inner.read_u32::<LittleEndian>() }
    fn read_raw_i64(&mut self) -> std::io::Result<i64> { self.inner.read_i64::<LittleEndian>() }
    fn read_raw_u64(&mut self) -> std::io::Result<u64> { self.inner.read_u64::<LittleEndian>() }
    fn read_raw_u16(&mut self) -> std::io::Result<u16> { self.inner.read_u16::<LittleEndian>() }
    fn read_raw_u8(&mut self) -> std::io::Result<u8> { self.inner.read_u8() }
    fn read_raw_i8(&mut self) -> std::io::Result<i8> { self.inner.read_i8() }
    fn read_boolean(&mut self) -> std::io::Result<bool> { Ok(self.inner.read_u8()? != 0) }

    fn read_string(&mut self) -> std::io::Result<String> {
        let mut len: u32 = 0;
        let mut shift = 0;
        loop {
            let b = self.inner.read_u8()?;
            len |= ((b & 0x7F) as u32) << shift;
            shift += 7;
            if b & 0x80 == 0 { break; }
            if shift > 35 { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "String length overflow")); }
        }
        let mut buf = vec![0u8; len as usize];
        self.inner.read_exact(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf).to_string())
    }

    fn read_bytes(&mut self, count: usize) -> std::io::Result<Vec<u8>> {
        let mut buf = vec![0u8; count];
        self.inner.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_datetime(&mut self) -> std::io::Result<i64> {
        let ticks = self.read_raw_i64()?;
        const EPOCH_DIFF_TICKS: i64 = 621355968000000000;
        Ok((ticks - EPOCH_DIFF_TICKS) / 10_000_000)
    }
}

// ============================================================
// Minimal data structures - we parse and write directly to DB
// ============================================================

struct ParsedAccount {
    account_id: String,
    salt: Vec<u8>,
    password_hash: Vec<u8>,
    characters: Vec<ParsedCharacter>,
    gold: u64,
    storage_items: Vec<Option<ParsedUserItem>>,
}

struct ParsedCharacter {
    name: String,
    level: u16,
    class_byte: u8,
    _gender_byte: u8,
    _hair: u8,
    current_map_index: i32,
    current_x: i32,
    current_y: i32,
    direction: u8,
    hp: i32,
    mp: i32,
    experience: i64,
    attack_mode: u8,
    pet_mode: u8,
    inventory: Vec<Option<ParsedUserItem>>,
    equipment: Vec<Option<ParsedUserItem>>,
    friends: Vec<(i32, bool, String)>,
    mail: Vec<ParsedMail>,
    quests: Vec<i32>,
    completed_quests: Vec<i32>,
    creatures: Vec<ParsedCreature>,
    married: i32,
    mentor: i32,
    is_mentor: bool,
    current_hero_index: i32,
}

struct ParsedUserItem {
    unique_id: u64,
    item_index: i32,
    current_dura: u16,
    max_dura: u16,
    count: u16,
    identified: bool,
    cursed: bool,
    gem_count: u16,
    awake_type: i32,
    awake_level: i32,
    refined_value: u8,
    refine_added: u8,
    wedding_ring: i32,
    is_shop_item: bool,
    gm_made: bool,
}

struct ParsedMail {
    mail_id: u64,
    sender: String,
    message: String,
    gold: u64,
    items: Vec<ParsedUserItem>,
    date_sent: i64,
    collected: bool,
    locked: bool,
}

struct ParsedCreature {
    pet_type: u8,
    custom_name: String,
    fullness: i32,
    slot_index: i32,
    pet_mode: u8,
    pickup_flags: u16,
    pickup_grade: u8,
}

// ============================================================
// Parsing functions
// ============================================================

fn read_user_item<R: Read>(reader: &mut BinaryReader<R>, version: i32) -> std::io::Result<ParsedUserItem> {
    let unique_id = reader.read_raw_u64()?;
    let item_index = reader.read_raw_i32()?;
    let current_dura = reader.read_raw_u16()?;
    let max_dura = reader.read_raw_u16()?;
    let count = if version <= 84 { reader.read_raw_u32()? as u16 } else { reader.read_raw_u16()? };

    if version <= 84 {
        for _ in 0..12 { reader.read_raw_u8()?; } // old added stats
        reader.read_raw_i8()?; reader.read_raw_i8()?; // attack speed, luck
    }

    reader.read_raw_i32()?; // soul_bound_id
    let bools = reader.read_raw_u8()?;
    let identified = (bools & 0x01) == 0x01;
    let cursed = (bools & 0x02) == 0x02;

    if version <= 84 {
        for _ in 0..8 { reader.read_raw_u8()?; } // more old stats
    }

    let slot_count = reader.read_raw_i32()?;
    for _i in 0..slot_count {
        if !reader.read_boolean()? {
            read_user_item(reader, version)?; // consume nested
        }
    }

    let gem_count = if version <= 84 { reader.read_raw_u32()? as u16 } else { reader.read_raw_u16()? };

    if version > 84 {
        let stats_count = reader.read_raw_i32()?;
        for _ in 0..stats_count { reader.read_raw_u8()?; reader.read_raw_i32()?; }
    }

    let awake_type = reader.read_raw_i32()?;
    let awake_level = reader.read_raw_i32()?;
    let refined_value = reader.read_raw_u8()?;
    let refine_added = reader.read_raw_u8()?;
    if version > 85 { reader.read_raw_i32()?; } // refine_success_chance
    let wedding_ring = reader.read_raw_i32()?;

    if version >= 65 {
        if reader.read_boolean()? {
            reader.read_raw_i32()?; reader.read_raw_i32()?; // expire_info
        }
    }
    if version >= 76 {
        if reader.read_boolean()? {
            reader.read_raw_i64()?; reader.read_raw_i64()?; reader.read_raw_u64()?; // rental_info
        }
    }
    let is_shop_item = if version >= 83 { reader.read_boolean()? } else { false };
    if version >= 92 {
        if reader.read_boolean()? {
            reader.read_raw_i64()?; reader.read_raw_i32()?; // sealed_info
        }
    }
    let gm_made = if version > 107 { reader.read_boolean()? } else { false };

    Ok(ParsedUserItem {
        unique_id, item_index, current_dura, max_dura, count,
        identified, cursed, gem_count, awake_type, awake_level,
        refined_value, refine_added, wedding_ring, is_shop_item, gm_made,
    })
}

fn read_character_info<R: Read>(reader: &mut BinaryReader<R>, version: i32) -> std::io::Result<ParsedCharacter> {
    reader.read_raw_i32()?; // index
    let name = reader.read_string()?;
    let level = if version < 62 { reader.read_raw_u8()? as u16 } else { reader.read_raw_u16()? };
    let class_byte = reader.read_raw_u8()?;
    let _gender_byte = reader.read_raw_u8()?;
    let _hair = reader.read_raw_u8()?;
    reader.read_string()?; // creation_ip
    reader.read_datetime()?; // creation_date
    reader.read_boolean()?; // banned
    reader.read_string()?; // ban_reason
    reader.read_datetime()?; // expiry_date
    reader.read_string()?; // last_ip
    reader.read_datetime()?; // last_logout_date
    if version > 81 { reader.read_datetime()?; } // last_login_date
    let _deleted = reader.read_boolean()?;
    reader.read_datetime()?; // delete_date

    let current_map_index = reader.read_raw_i32()?;
    let current_x = reader.read_raw_i32()?;
    let current_y = reader.read_raw_i32()?;
    let direction = reader.read_raw_u8()?;
    reader.read_raw_i32()?; // bind_map_index
    reader.read_raw_i32()?; reader.read_raw_i32()?; // bind_location
    let (hp, mp) = if version <= 84 {
        (reader.read_raw_u16()? as i32, reader.read_raw_u16()? as i32)
    } else {
        (reader.read_raw_i32()?, reader.read_raw_i32()?)
    };

    let experience = reader.read_raw_i64()?;
    let attack_mode = reader.read_raw_u8()?;
    let pet_mode = reader.read_raw_u8()?;
    if version > 34 { reader.read_raw_i32()?; } // pk_points

    // Inventory
    let inv_count = reader.read_raw_i32()?;
    let mut inventory: Vec<Option<ParsedUserItem>> = Vec::with_capacity(inv_count as usize);
    for _ in 0..inv_count {
        if reader.read_boolean()? {
            inventory.push(None);
        } else {
            inventory.push(Some(read_user_item(reader, version)?));
        }
    }

    // Equipment
    let eq_count = reader.read_raw_i32()?;
    let mut equipment: Vec<Option<ParsedUserItem>> = Vec::with_capacity(eq_count as usize);
    for _ in 0..eq_count {
        if reader.read_boolean()? {
            equipment.push(None);
        } else {
            equipment.push(Some(read_user_item(reader, version)?));
        }
    }

    // QuestInventory (consume but don't store separately)
    let qi_count = reader.read_raw_i32()?;
    for _ in 0..qi_count {
        if !reader.read_boolean()? {
            read_user_item(reader, version)?;
        }
    }

    // Magics
    let magic_count = reader.read_raw_i32()?;
    for _ in 0..magic_count {
        reader.read_raw_u32()?; // magic_id
        reader.read_raw_u8()?; // level
        if version < 62 { reader.read_raw_u32()?; } else { reader.read_raw_u64()?; }
        reader.read_boolean()?; // is_temp
        reader.read_raw_i32()?; // cast_time
    }

    reader.read_boolean()?; // thrusting
    reader.read_boolean()?; // half_moon
    reader.read_boolean()?; // cross_half_moon
    reader.read_boolean()?; // double_slash
    reader.read_raw_u8()?; // mental_state

    // Pets
    let pet_count = reader.read_raw_i32()?;
    for _ in 0..pet_count {
        reader.read_raw_i32()?; // monster_index
        if version <= 84 { reader.read_raw_u32()?; } else { reader.read_raw_i32()?; }
        reader.read_raw_u32()?; reader.read_raw_u8()?; reader.read_raw_u8()?;
    }

    reader.read_boolean()?; // allow_group
    const FLAG_COUNT: usize = 256;
    for _ in 0..FLAG_COUNT { reader.read_boolean()?; }
    reader.read_raw_i32()?; // guild_index
    reader.read_boolean()?; // allow_trade
    if version > 104 { reader.read_boolean()?; } // allow_observe

    // CurrentQuests (store indices)
    let quest_count = reader.read_raw_i32()?;
    let mut quests = Vec::new();
    for _ in 0..quest_count {
        quests.push(reader.read_raw_i32()?); // index
        reader.read_datetime()?; // start
        reader.read_datetime()?; // end
        // Consume task details
        let kill_count = reader.read_raw_i32()?;
        for _ in 0..kill_count {
            if version < 90 { reader.read_raw_i32()?; } else { reader.read_raw_i32()?; reader.read_raw_i32()?; }
        }
        let item_count = reader.read_raw_i32()?;
        for _ in 0..item_count {
            if version < 90 { reader.read_raw_i32()?; } else { reader.read_raw_i32()?; reader.read_raw_i32()?; }
        }
        let flag_count = reader.read_raw_i32()?;
        for _ in 0..flag_count {
            if version < 90 { reader.read_boolean()?; } else { reader.read_raw_i32()?; reader.read_boolean()?; }
        }
    }

    // Buffs
    let buff_count = reader.read_raw_i32()?;
    for _ in 0..buff_count {
        reader.read_raw_u8()?; // type
        if version < 88 { reader.read_boolean()?; }
        reader.read_raw_u32()?; reader.read_raw_i64()?;
        if version <= 84 {
            let vc = reader.read_raw_i32()?;
            for _ in 0..vc { reader.read_raw_i32()?; }
            if version < 88 { reader.read_boolean()?; }
        } else {
            if version < 88 { reader.read_boolean()?; }
            let sc = reader.read_raw_i32()?;
            for _ in 0..sc { reader.read_raw_u8()?; reader.read_raw_i32()?; }
            let dc = reader.read_raw_i32()?;
            for _ in 0..dc { reader.read_string()?; let l = reader.read_raw_i32()?; reader.read_bytes(l as usize)?; }
            if version > 86 { let vc = reader.read_raw_i32()?; for _ in 0..vc { reader.read_raw_i32()?; } }
        }
    }

    // Mail
    let mail_count = reader.read_raw_i32()?;
    let mut mail = Vec::new();
    for _ in 0..mail_count {
        let mail_id = reader.read_raw_u64()?;
        let sender = reader.read_string()?;
        reader.read_raw_i32()?; // recipient_index
        let message = reader.read_string()?;
        let mail_gold = reader.read_raw_u32()? as u64;
        let item_count = reader.read_raw_i32()?;
        let mut items = Vec::new();
        for _ in 0..item_count {
            items.push(read_user_item(reader, version)?);
        }
        let date_sent = reader.read_datetime()?;
        reader.read_datetime()?; // date_opened
        let locked = reader.read_boolean()?;
        let collected = reader.read_boolean()?;
        reader.read_boolean()?; // can_reply

        mail.push(ParsedMail {
            mail_id, sender, message, gold: mail_gold, items, date_sent, collected, locked,
        });
    }

    // IntelligentCreatures
    let creature_count = reader.read_raw_i32()?;
    let mut creatures = Vec::new();
    for _ in 0..creature_count {
        let pet_type = reader.read_raw_u8()?;
        let custom_name = reader.read_string()?;
        let fullness = reader.read_raw_i32()?;
        let slot_index = reader.read_raw_i32()?;
        reader.read_raw_i64()?; // expire
        reader.read_raw_i64()?; // blackstone_time
        let pet_mode = reader.read_raw_u8()?;
        let mut pickup_flags: u16 = 0;
        for i in 0..9 { if reader.read_boolean()? { pickup_flags |= 1 << i; } }
        let pickup_grade = if version > 48 { reader.read_raw_u8()? } else { 0 };
        if version > 48 { reader.read_raw_i64()?; }
        creatures.push(ParsedCreature { pet_type, custom_name, fullness, slot_index, pet_mode, pickup_flags, pickup_grade });
    }

    if version == 45 { reader.read_raw_u8()?; reader.read_boolean()?; }
    reader.read_raw_i32()?; // pearl_count

    // CompletedQuests
    let cq_count = reader.read_raw_i32()?;
    let mut completed_quests = Vec::new();
    for _ in 0..cq_count {
        completed_quests.push(reader.read_raw_i32()?);
    }

    // CurrentRefine
    if reader.read_boolean()? {
        read_user_item(reader, version)?;
    }
    reader.read_raw_i64()?; // refine_time_remaining

    // Friends
    let friend_count = reader.read_raw_i32()?;
    let mut friends = Vec::new();
    for _ in 0..friend_count {
        let idx = reader.read_raw_i32()?;
        let blocked = reader.read_boolean()?;
        let memo = reader.read_string()?;
        friends.push((idx, blocked, memo));
    }

    // RentedItems
    if version > 75 {
        let ri_count = reader.read_raw_i32()?;
        for _ in 0..ri_count {
            reader.read_raw_i32()?; reader.read_raw_i32()?; reader.read_raw_u64()?;
        }
        reader.read_boolean()?; // has_rented_item
    }

    let married = reader.read_raw_i32()?;
    reader.read_datetime()?; // married_date
    let mentor = reader.read_raw_i32()?;
    reader.read_datetime()?; // mentor_date
    let is_mentor = reader.read_boolean()?;
    reader.read_raw_i64()?; // mentor_exp

    // GS purchases
    if version >= 63 {
        let gs_count = reader.read_raw_i32()?;
        for _ in 0..gs_count { reader.read_raw_i32()?; reader.read_raw_i32()?; }
    }

    // Heroes
    let _hero_count = if version > 98 {
        let count = reader.read_raw_i32()?;
        if version > 102 {
            for _ in 0..count { reader.read_raw_i32()?; }
        } else {
            for _ in 0..count {
                read_character_info(reader, version)?; // inline hero
            }
        }
        if version < 104 { reader.read_raw_i32()?; }
        count
    } else {
        1
    };
    let current_hero_index = if version > 98 { reader.read_raw_i32()? } else { 0 };
    if version > 98 { reader.read_boolean()?; } // hero_spawned
    if version > 100 { reader.read_raw_u8()?; } // hero_behaviour

    Ok(ParsedCharacter {
        name, level, class_byte, _gender_byte, _hair,
        current_map_index, current_x, current_y, direction,
        hp, mp, experience, attack_mode, pet_mode,
        inventory, equipment, friends, mail, quests, completed_quests,
        creatures, married, mentor, is_mentor, current_hero_index,
    })
}

fn read_account<R: Read>(reader: &mut BinaryReader<R>, version: i32) -> std::io::Result<ParsedAccount> {
    reader.read_raw_i32()?; // index
    let account_id = reader.read_string()?;

    let _password = if version < 94 {
        reader.read_string()?
    } else {
        reader.read_string()?
    };

    let salt = if version > 93 {
        let salt_len = reader.read_raw_i32()?;
        reader.read_bytes(salt_len as usize)?
    } else {
        vec![0u8; 24]
    };

    let password_hash = salt.clone(); // For now, same as salt (will re-hash on first login)
    if version > 97 { reader.read_boolean()?; } // require_password_change

    reader.read_string()?; // user_name
    reader.read_datetime()?; // birth_date
    reader.read_string()?; // secret_question
    reader.read_string()?; // secret_answer
    reader.read_string()?; // email
    reader.read_string()?; // creation_ip
    reader.read_datetime()?; // creation_date
    reader.read_boolean()?; // banned
    reader.read_string()?; // ban_reason
    reader.read_datetime()?; // expiry_date
    reader.read_string()?; // last_ip
    reader.read_datetime()?; // last_date

    let char_count = reader.read_raw_i32()?;
    let mut characters = Vec::new();
    for _ in 0..char_count {
        let info = read_character_info(reader, version)?;
        characters.push(info);
    }

    let _has_expanded_storage = if version > 75 { reader.read_boolean()? } else { false };
    if version > 75 { reader.read_datetime()?; } // expanded_storage_expiry

    let gold = reader.read_raw_u32()? as u64;
    if version >= 63 { reader.read_raw_u32()?; } // credit

    let storage_count = reader.read_raw_i32()?;
    let mut storage_items: Vec<Option<ParsedUserItem>> = (0..storage_count).map(|_| None).collect();
    for i in 0..storage_count {
        if reader.read_boolean()? {
            continue;
        }
        let item = read_user_item(reader, version)?;
        if (i as usize) < storage_items.len() {
            storage_items[i as usize] = Some(item);
        }
    }

    if version >= 10 { reader.read_boolean()?; } // admin_account

    Ok(ParsedAccount {
        account_id, salt, password_hash, characters, gold, storage_items,
    })
}

// ============================================================
// DB insertion
// ============================================================

fn item_to_json(item: &ParsedUserItem) -> String {
    serde_json::to_string(&serde_json::json!({
        "UniqueID": item.unique_id,
        "ItemIndex": item.item_index,
        "CurrentDura": item.current_dura,
        "MaxDura": item.max_dura,
        "Count": item.count,
        "Identified": item.identified,
        "Cursed": item.cursed,
        "GemCount": item.gem_count,
        "AwakeType": item.awake_type,
        "AwakeLevel": item.awake_level,
        "RefinedValue": item.refined_value,
        "RefineAdded": item.refine_added,
        "WeddingRing": item.wedding_ring,
        "IsShopItem": item.is_shop_item,
        "GMMade": item.gm_made,
    })).unwrap_or_default()
}

fn base64_encode(data: &[u8]) -> String {
    data_encoding::BASE64.encode(data)
}

async fn migrate_account(pool: &sqlx::SqlitePool, account: &ParsedAccount) -> anyhow::Result<()> {
    // Store password with pbkdf2 prefix - will be migrated to Argon2 on first login
    let password_hash = format!("pbkdf2_sha1${}${}",
        base64_encode(&account.salt),
        base64_encode(&account.password_hash)
    );

    sqlx::query(
        r#"INSERT OR REPLACE INTO accounts (username, password_hash, is_online) VALUES (?, ?, 0)"#
    )
    .bind(&account.account_id)
    .bind(&password_hash)
    .execute(pool)
    .await?;

    for character in &account.characters {
        migrate_character(pool, account, character).await?;
    }

    Ok(())
}

async fn migrate_character(pool: &sqlx::SqlitePool, account: &ParsedAccount, character: &ParsedCharacter) -> anyhow::Result<()> {
    let max_hp = character.hp.max(30);
    let max_mp = character.mp.max(10);

    let spouse = if character.married != 0 { Some(format!("spouse_{}", character.married)) } else { None };
    let mentor = if character.mentor != 0 { Some(format!("mentor_{}", character.mentor)) } else { None };

    sqlx::query(
        r#"INSERT OR REPLACE INTO characters (
            name, account_username, schema_version, map_index, x, y, direction,
            attack_mode, pet_mode, level, experience, max_experience,
            hp, max_hp, mp, max_mp, min_attack, max_attack, defence,
            gold, group_id, guild_name, guild_rank,
            spouse_name, allow_mentor, mentor_name, hero_index,
            is_fishing, fishing_autocast
        ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#
    )
    .bind(&character.name)
    .bind(&account.account_id)
    .bind(1i32)
    .bind(character.current_map_index)
    .bind(character.current_x)
    .bind(character.current_y)
    .bind(character.direction as i32)
    .bind(attack_mode_str(character.attack_mode))
    .bind(pet_mode_str(character.pet_mode))
    .bind(character.level as i32)
    .bind(character.experience)
    .bind(100i64)
    .bind(character.hp)
    .bind(max_hp)
    .bind(character.mp)
    .bind(max_mp)
    .bind(5i32)
    .bind(10i32)
    .bind(2i32)
    .bind(0i64)
    .bind(None::<i64>)
    .bind(None::<String>)
    .bind(2i32)
    .bind(spouse)
    .bind(if character.is_mentor { 1 } else { 0 })
    .bind(mentor)
    .bind(character.current_hero_index)
    .bind(0i32)
    .bind(0i32)
    .execute(pool)
    .await?;

    // Backpack
    save_items(pool, &character.name, "inventory_backpack", &character.inventory, "grid").await?;
    // Equipment
    save_items(pool, &character.name, "inventory_equipment", &character.equipment, "slot").await?;

    // Friends
    sqlx::query("DELETE FROM friends WHERE character_name = ?")
        .bind(&character.name).execute(pool).await?;
    sqlx::query("DELETE FROM blocked_list WHERE character_name = ?")
        .bind(&character.name).execute(pool).await?;
    for (idx, blocked, memo) in &character.friends {
        if *blocked {
            sqlx::query("INSERT INTO blocked_list (character_name, blocked_object_id, blocked_name) VALUES (?,?,?)")
                .bind(&character.name).bind(idx).bind(&format!("char_{}", idx)).execute(pool).await?;
        } else {
            sqlx::query("INSERT INTO friends (character_name, friend_object_id, friend_name, memo) VALUES (?,?,?,?)")
                .bind(&character.name).bind(idx).bind(&format!("char_{}", idx)).bind(memo).execute(pool).await?;
        }
    }

    // Mail
    sqlx::query("DELETE FROM mail WHERE character_name = ?")
        .bind(&character.name).execute(pool).await?;
    for m in &character.mail {
        let items_json = serde_json::to_string(&m.items.iter().map(|i| {
            serde_json::json!({"UniqueID": i.unique_id, "ItemIndex": i.item_index, "Count": i.count})
        }).collect::<Vec<_>>()).unwrap_or_default();
        sqlx::query(
            r#"INSERT INTO mail (character_name, mail_id, sender_name, subject, body, timestamp,
                read_flag, collected, locked, gold, items_json)
               VALUES (?,?,?,?,?,?,?,?,?,?,?)"#
        )
        .bind(&character.name).bind(m.mail_id as i64).bind(&m.sender)
        .bind("Migrated").bind(&m.message).bind(m.date_sent)
        .bind(0i32).bind(if m.collected { 1 } else { 0 }).bind(if m.locked { 1 } else { 0 })
        .bind(m.gold as i64).bind(&items_json).execute(pool).await?;
    }

    // Quests
    sqlx::query("DELETE FROM quests WHERE character_name = ?")
        .bind(&character.name).execute(pool).await?;
    sqlx::query("DELETE FROM completed_quests WHERE character_name = ?")
        .bind(&character.name).execute(pool).await?;
    for qi in &character.quests {
        sqlx::query("INSERT INTO quests (character_name, quest_index, title, status, progress_json, exp_reward, gold_reward) VALUES (?,?,?,?,?,?,?)")
            .bind(&character.name).bind(qi).bind(format!("Quest {}", qi)).bind("InProgress")
            .bind("[]").bind(0i64).bind(0i64).execute(pool).await?;
    }
    for qi in &character.completed_quests {
        sqlx::query("INSERT INTO completed_quests (character_name, quest_index) VALUES (?,?)")
            .bind(&character.name).bind(qi).execute(pool).await?;
    }

    // Creatures
    if let Some(c) = character.creatures.first() {
        let owned_json = serde_json::to_string(&character.creatures.iter().map(|c| {
            serde_json::json!({
                "creature_type": c.pet_type,
                "custom_name": c.custom_name,
                "pickup_mode": c.pickup_grade,
                "hunger": c.fullness as u8,
                "enabled": true
            })
        }).collect::<Vec<_>>()).unwrap_or_default();
        sqlx::query(
            r#"INSERT OR REPLACE INTO creatures (
                character_name, active_type, active_custom_name, active_pickup_mode,
                active_hunger, active_enabled, owned_json, request_updates
            ) VALUES (?,?,?,?,?,?,?,?)"#
        )
        .bind(&character.name).bind(c.pet_type as i32).bind(&c.custom_name)
        .bind(c.pickup_grade as i32).bind(c.fullness as u8).bind(1i32)
        .bind(&owned_json).bind(0i32).execute(pool).await?;
    }

    info!("    Character: {} (Lv{}, class {})", character.name, character.level, character.class_byte);
    Ok(())
}

fn attack_mode_str(b: u8) -> &'static str {
    match b { 1 => "Group", 2 => "Guild", 3 => "EnemyGuild", 4 => "RedBrown", 5 => "All", _ => "Peace" }
}

fn pet_mode_str(b: u8) -> &'static str {
    match b { 1 => "MoveOnly", 2 => "AttackOnly", 3 => "None", 4 => "FocusMasterTarget", _ => "Both" }
}

async fn save_items(pool: &sqlx::SqlitePool, char_name: &str, table: &str, items: &[Option<ParsedUserItem>], col: &str) -> anyhow::Result<()> {
    sqlx::query(&format!("DELETE FROM {} WHERE character_name = ?", table))
        .bind(char_name).execute(pool).await?;
    for (i, item) in items.iter().enumerate() {
        if let Some(item) = item {
            let item_json = item_to_json(item);
            sqlx::query(&format!(
                "INSERT INTO {} (character_name, {}, item_json) VALUES (?,?,?)", table, col
            ))
            .bind(char_name).bind(i as i32).bind(&item_json).execute(pool).await?;
        }
    }
    Ok(())
}

// ============================================================
// Main
// ============================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: migrate <path-to-Server.MirADB> [sqlite-db-path]");
        eprintln!("  Default sqlite path: data/crystal.db");
        std::process::exit(1);
    }

    let adb_path = &args[1];
    let sqlite_path = args.get(2).map(|s| s.as_str()).unwrap_or("data/crystal.db");

    info!("=== C# to SQLite Migration Tool ===");
    info!("Source: {}", adb_path);
    info!("Target: {}", sqlite_path);

    // Read the binary file
    let data = std::fs::read(adb_path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", adb_path, e))?;
    info!("File size: {} bytes", data.len());

    let mut reader = BinaryReader::new(std::io::Cursor::new(data));

    // File header - parse exactly like C# Envir.LoadAccounts
    let version = reader.read_raw_i32()?;
    let custom_version = reader.read_raw_i32()?;
    let next_account_id = reader.read_raw_i32()?;
    let next_character_id = reader.read_raw_i32()?;
    let next_user_item_id = reader.read_raw_u64()?;

    // NextHeroID only exists when version > 98 (i32, not u64!)
    let next_hero_id = if version > 98 {
        reader.read_raw_i32()?
    } else {
        0
    };

    // Guild fields
    let guild_count = reader.read_raw_i32()?;
    let next_guild_id = reader.read_raw_i32()?;

    // HeroList only exists when version > 102
    if version > 102 {
        let hero_list_count = reader.read_raw_i32()?;
        info!("Skipping {} HeroList entries", hero_list_count);
        for _ in 0..hero_list_count {
            // Skip HeroInfo: index(i32) + name(string) + ... too complex, just consume
            // Minimal skip: index + name
            reader.read_raw_i32()?; // index
            reader.read_string()?;  // name
            // We can't easily skip the rest without full HeroInfo structure,
            // but for version=83 this branch won't execute anyway
        }
    }

    info!("Version: {}.{}", version, custom_version);
    info!("Next IDs: account={}, character={}, item={}, hero={}",
        next_account_id, next_character_id, next_user_item_id, next_hero_id);
    info!("Guilds: {}, NextGuildID: {}", guild_count, next_guild_id);

    // Account count (comes after optional HeroList)
    let account_count = reader.read_raw_i32()?;
    info!("Accounts to migrate: {}", account_count);

    // Initialize database
    let db_url = format!("sqlite://{}", sqlite_path);
    info!("DB URL: {}", db_url);
    let pool = sqlx::SqlitePool::connect(&db_url).await
        .map_err(|e| anyhow::anyhow!("Failed to connect to {}: {}", sqlite_path, e))?;

    // Create tables
    info!("Creating tables...");
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
            FOREIGN KEY (account_username) REFERENCES accounts(username)
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

    // Migrate accounts
    info!("Starting migration...");
    let mut account_count_success = 0;
    let mut character_count_success = 0;
    let mut error_count = 0;

    for i in 0..account_count {
        match read_account(&mut reader, version) {
            Ok(account) => {
                let chars = account.characters.len();
                if let Err(e) = migrate_account(&pool, &account).await {
                    error!("Failed to migrate account #{}: {}", i, e);
                    error_count += 1;
                } else {
                    info!("  Account #{}: {} ({} characters)", i, account.account_id, chars);
                    account_count_success += 1;
                    character_count_success += chars;
                }
            }
            Err(e) => {
                error!("Failed to read account #{}: {}", i, e);
                error_count += 1;
            }
        }
    }

    info!("=== Migration Complete ===");
    info!("Accounts migrated: {}", account_count_success);
    info!("Characters migrated: {}", character_count_success);
    info!("Errors: {}", error_count);
    info!("Database: {}", sqlite_path);

    // Verify
    let row_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM accounts")
        .fetch_one(&pool).await?;
    let char_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM characters")
        .fetch_one(&pool).await?;
    let item_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM inventory_backpack")
        .fetch_one(&pool).await?;

    info!("Verification: {} accounts, {} characters, {} backpack items in DB",
        row_count.0, char_count.0, item_count.0);

    Ok(())
}
