// Migration tool: C# .MirADB/.MirDB -> SQLite
// Usage: cargo run --bin migrate -- <path-to-MirADB> [sqlite-db-path]
// Default sqlite path: data/crystal.db

#![allow(dead_code)]

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Read;
use tracing::{error, info};

// ============================================================
// BinaryReader compatible with C# BinaryReader
// ============================================================

struct BinaryReader<R: Read> {
    inner: R,
    /// 已读字节数（诊断用：错位排查要看「读到哪」而不是「读出了什么」）
    pos: usize,
}

impl<R: Read> BinaryReader<R> {
    fn new(inner: R) -> Self {
        Self { inner, pos: 0 }
    }
    fn position(&self) -> usize {
        self.pos
    }
    fn read_raw_i32(&mut self) -> std::io::Result<i32> {
        let at = self.pos;
        self.pos += 4;
        let v = self.inner.read_i32::<LittleEndian>()?;
        if trace_on() {
            eprintln!("[trace] i32  @{at} = {v}");
        }
        Ok(v)
    }
    fn read_raw_u32(&mut self) -> std::io::Result<u32> {
        self.pos += 4;
        self.inner.read_u32::<LittleEndian>()
    }
    fn read_raw_i64(&mut self) -> std::io::Result<i64> {
        self.pos += 8;
        self.inner.read_i64::<LittleEndian>()
    }
    fn read_raw_u64(&mut self) -> std::io::Result<u64> {
        self.pos += 8;
        self.inner.read_u64::<LittleEndian>()
    }
    fn read_raw_u16(&mut self) -> std::io::Result<u16> {
        self.pos += 2;
        self.inner.read_u16::<LittleEndian>()
    }
    fn read_raw_i16(&mut self) -> std::io::Result<i16> {
        self.pos += 2;
        self.inner.read_i16::<LittleEndian>()
    }
    fn read_raw_u8(&mut self) -> std::io::Result<u8> {
        self.pos += 1;
        self.inner.read_u8()
    }
    fn read_raw_i8(&mut self) -> std::io::Result<i8> {
        self.pos += 1;
        self.inner.read_i8()
    }
    fn read_boolean(&mut self) -> std::io::Result<bool> {
        self.pos += 1;
        Ok(self.inner.read_u8()? != 0)
    }

    fn read_string(&mut self) -> std::io::Result<String> {
        Ok(String::from_utf8_lossy(&self.read_dotnet_bytes()?).to_string())
    }

    /// 读 dotnet 字符串的**原始字节**（不经过 UTF-8 损失转换）。
    /// 密码字段必须是这条路径：C# `Crypto.HashPassword` 是
    /// `Encoding.UTF8.GetString(pbkdf2.GetBytes(24))`——24 字节哈希被**当成 UTF-8 字符串**存盘，
    /// 用 `read_string()` 读会经过 lossy 转换而改变字节，导致迁移后的账号永远验不过密码。
    fn read_dotnet_bytes(&mut self) -> std::io::Result<Vec<u8>> {
        let at = self.pos;
        let mut len: u32 = 0;
        let mut shift = 0;
        loop {
            let b = self.inner.read_u8()?;
            self.pos += 1;
            len |= ((b & 0x7F) as u32) << shift;
            shift += 7;
            if b & 0x80 == 0 {
                break;
            }
            if shift > 35 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "String length overflow",
                ));
            }
        }
        let mut buf = vec![0u8; len as usize];
        self.inner.read_exact(&mut buf)?;
        self.pos += len as usize;
        if trace_on() {
            let head: String = String::from_utf8_lossy(&buf[..buf.len().min(24)]).to_string();
            eprintln!("[trace] str  @{at} len={len} head={head:?}");
        }
        Ok(buf)
    }

    fn read_bytes(&mut self, count: usize) -> std::io::Result<Vec<u8>> {
        let mut buf = vec![0u8; count];
        self.inner.read_exact(&mut buf)?;
        self.pos += count;
        Ok(buf)
    }

    fn read_datetime(&mut self) -> std::io::Result<i64> {
        let ticks = self.read_raw_i64()?;
        const EPOCH_DIFF_TICKS: i64 = 621355968000000000;
        Ok((ticks - EPOCH_DIFF_TICKS) / 10_000_000)
    }
}

/// `MIR2_MIGRATE_TRACE=1` 时打印读取轨迹（定位二进制格式错位用：看「读到哪、读多长」，
/// 而不是只看到最后一句 "failed to fill whole buffer"）。
fn trace_on() -> bool {
    std::env::var("MIR2_MIGRATE_TRACE").is_ok()
}

/// 轨迹里的**段标记**：读错位时先看「最后一条段标记」就知道是哪一段开始的
/// （只看 i32/str 的裸轨迹要在几十条里对齐字段，慢且容易看错）。
fn trace_mark<R: Read>(reader: &BinaryReader<R>, section: &str) {
    if trace_on() {
        eprintln!("[trace] ==== {section} @{}", reader.position());
    }
}

/// 读「某段的条目数」并做上限检查：错位后 count 会读成垃圾（实测 NPC/生物段曾读出 16 亿），
/// 不设上限就会空转十亿次（刷屏 + CPU 跑满）。超过 1_000_000 一律视为已错位，立刻失败。
fn read_bounded_count<R: Read>(reader: &mut BinaryReader<R>, what: &str) -> std::io::Result<i32> {
    let count = reader.read_raw_i32()?;
    if !(0..=1_000_000).contains(&count) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{what} 段条目数不合理（疑似前面已错位）: {count}"),
        ));
    }
    Ok(count)
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

fn read_user_item<R: Read>(
    reader: &mut BinaryReader<R>,
    version: i32,
) -> std::io::Result<ParsedUserItem> {
    let unique_id = reader.read_raw_u64()?;
    let item_index = reader.read_raw_i32()?;
    let current_dura = reader.read_raw_u16()?;
    let max_dura = reader.read_raw_u16()?;
    let count = if version <= 84 {
        reader.read_raw_u32()? as u16
    } else {
        reader.read_raw_u16()?
    };

    if version <= 84 {
        for _ in 0..12 {
            reader.read_raw_u8()?;
        } // old added stats
        reader.read_raw_i8()?;
        reader.read_raw_i8()?; // attack speed, luck
    }

    reader.read_raw_i32()?; // soul_bound_id
    let bools = reader.read_raw_u8()?;
    let identified = (bools & 0x01) == 0x01;
    let cursed = (bools & 0x02) == 0x02;

    if version <= 84 {
        for _ in 0..8 {
            reader.read_raw_u8()?;
        } // more old stats
    }

    let slot_count = reader.read_raw_i32()?;
    for _i in 0..slot_count {
        // ⚠️ 极性陷阱：`UserItem.Slots` 与「背包/仓库格」**相反**。
        // C# `UserItem.Save:479` 写的是 `writer.Write(Slots[i] == null)`，
        // 读侧对应 `UserItem.cs:397-402` 的 `if (reader.ReadBoolean()) continue;`
        // —— **true 表示该槽为空**。原实现按「true 就有物品」读，于是每件带孔装备都会
        // 多读/漏读一整件嵌套物品，误差按孔数累积（实测角色记录因此短 22 字节）。
        if reader.read_boolean()? {
            continue;
        }
        read_user_item(reader, version)?; // consume nested
    }

    let gem_count = if version <= 84 {
        reader.read_raw_u32()? as u16
    } else {
        reader.read_raw_u16()?
    };

    if version > 84 {
        let stats_count = reader.read_raw_i32()?;
        for _ in 0..stats_count {
            reader.read_raw_u8()?;
            reader.read_raw_i32()?;
        }
    }

    // C# `Awake(BinaryReader)`（Shared/Data/ItemData.cs:893-901）：
    // `Type u8 → count i32 → count × u8`。原实现读成 `i32 + i32`（每件物品多 3 字节）。
    // 这里 count 也做上限检查（错位时 count 会是垃圾）。
    let awake_type = reader.read_raw_u8()? as i32;
    let awake_count = reader.read_raw_i32()?;
    if !(0..=1024).contains(&awake_count) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("UserItem.Awake count 不合理（疑似已错位）: {awake_count}"),
        ));
    }
    for _ in 0..awake_count {
        reader.read_raw_u8()?;
    }
    let awake_level = awake_count;
    let refined_value = reader.read_raw_u8()?;
    let refine_added = reader.read_raw_u8()?;
    if version > 85 {
        reader.read_raw_i32()?;
    } // refine_success_chance
    let wedding_ring = reader.read_raw_i32()?;

    if version >= 65 && reader.read_boolean()? {
        // C# `ExpireInfo(BinaryReader)`：只有一个 i64（原实现读成两个 i32）
        reader.read_raw_i64()?; // expire_info.expiry_date
    }
    if version >= 76 && reader.read_boolean()? {
        // C# `RentalInformation(BinaryReader)`（ItemData.cs:761-767）：
        // `OwnerName string → BindingFlags i16 → ExpiryDate i64 → RentalLocked bool`
        reader.read_string()?; // owner_name
        reader.read_raw_i16()?; // binding_flags
        reader.read_datetime()?; // expiry_date
        reader.read_boolean()?; // rental_locked
    }
    let is_shop_item = if version >= 83 {
        reader.read_boolean()?
    } else {
        false
    };
    if version >= 92 && reader.read_boolean()? {
        // C# `SealedInfo(BinaryReader)`（ItemData.cs:735-742）：`ExpiryDate i64`，
        // 且 **v>92 再加一个** `NextSealDate i64`（原实现只读 i64 + i32）
        reader.read_raw_i64()?; // expiry_date
        if version > 92 {
            reader.read_datetime()?; // next_seal_date
        }
    }
    let gm_made = if version > 107 {
        reader.read_boolean()?
    } else {
        false
    };

    Ok(ParsedUserItem {
        unique_id,
        item_index,
        current_dura,
        max_dura,
        count,
        identified,
        cursed,
        gem_count,
        awake_type,
        awake_level,
        refined_value,
        refine_added,
        wedding_ring,
        is_shop_item,
        gm_made,
    })
}

fn read_character_info<R: Read>(
    reader: &mut BinaryReader<R>,
    version: i32,
) -> std::io::Result<ParsedCharacter> {
    reader.read_raw_i32()?; // index
    let name = reader.read_string()?;
    let level = if version < 62 {
        reader.read_raw_u8()? as u16
    } else {
        reader.read_raw_u16()?
    };
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
    if version > 81 {
        reader.read_datetime()?;
    } // last_login_date
    let _deleted = reader.read_boolean()?;
    reader.read_datetime()?; // delete_date

    let current_map_index = reader.read_raw_i32()?;
    let current_x = reader.read_raw_i32()?;
    let current_y = reader.read_raw_i32()?;
    let direction = reader.read_raw_u8()?;
    reader.read_raw_i32()?; // bind_map_index
    reader.read_raw_i32()?;
    reader.read_raw_i32()?; // bind_location
    let (hp, mp) = if version <= 84 {
        (reader.read_raw_u16()? as i32, reader.read_raw_u16()? as i32)
    } else {
        (reader.read_raw_i32()?, reader.read_raw_i32()?)
    };

    let experience = reader.read_raw_i64()?;
    let attack_mode = reader.read_raw_u8()?;
    let pet_mode = reader.read_raw_u8()?;
    if version > 34 {
        reader.read_raw_i32()?;
    } // pk_points

    trace_mark(reader, "inventory");
    // Inventory
    let inv_count = reader.read_raw_i32()?;
    let mut inventory: Vec<Option<ParsedUserItem>> = Vec::with_capacity(inv_count as usize);
    for _ in 0..inv_count {
        // C# `CharacterInfo.Save`（Server/MirDatabase/CharacterInfo.cs:434-437）每格先写
        // `Inventory[i] != null`，**true 才跟一个 UserItem**。原实现三个循环都判反了
        // （空格反而去读 UserItem）⇒ 角色记录从第一个空格起整段错位直到 EOF
        // （实测真实 Server.MirADB：三个账号全部 "failed to fill whole buffer"、
        // 账号 #0 的解析把整个文件读完 pos=20456 > 文件 20455）。
        if !reader.read_boolean()? {
            inventory.push(None);
        } else {
            inventory.push(Some(read_user_item(reader, version)?));
        }
    }

    trace_mark(reader, "equipment");
    // Equipment
    let eq_count = reader.read_raw_i32()?;
    let mut equipment: Vec<Option<ParsedUserItem>> = Vec::with_capacity(eq_count as usize);
    for _ in 0..eq_count {
        // 同上（C# :443-446）
        if !reader.read_boolean()? {
            equipment.push(None);
        } else {
            equipment.push(Some(read_user_item(reader, version)?));
        }
    }

    trace_mark(reader, "quest_inventory");
    // QuestInventory (consume but don't store separately)
    let qi_count = reader.read_raw_i32()?;
    for _ in 0..qi_count {
        // 同上（C# :452-455）
        if !reader.read_boolean()? {
            continue;
        }
        read_user_item(reader, version)?;
    }

    trace_mark(reader, "magics");
    // Magics
    let magic_count = reader.read_raw_i32()?;
    for _ in 0..magic_count {
        // C# `UserMagic`（Server/MirDatabase/MagicInfo.cs:120-133）：
        // `Spell u8 → Level u8 → Key u8 → Experience u16 → [v>=15] IsTempSpell bool
        //  → [v>=65] CastTime i64`。
        // 原实现读的是 `magic_id u32 + level u8 + u64 + is_temp + cast_time i32` ⇒ 结构性不符。
        reader.read_raw_u8()?; // spell（C# 用 byte，不是 u32）
        reader.read_raw_u8()?; // level
        reader.read_raw_u8()?; // key
        reader.read_raw_u16()?; // experience
        if version >= 15 {
            reader.read_boolean()?; // is_temp_spell
        }
        if version >= 65 {
            reader.read_raw_i64()?; // cast_time
        }
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
        if version <= 84 {
            reader.read_raw_u32()?;
        } else {
            reader.read_raw_i32()?;
        }
        reader.read_raw_u32()?;
        reader.read_raw_u8()?;
        reader.read_raw_u8()?;
    }

    reader.read_boolean()?; // allow_group
                            // C# `CharacterInfo.Load`（Server/MirDatabase/CharacterInfo.cs:251）用的是
                            // `for (int i = 0; i < Globals.FlagIndexCount; i++) Flags[i] = reader.ReadBoolean();`，
                            // 而 `Shared/Globals.cs:31` 写着 **FlagIndexCount = 1999**。
                            // 原实现写死 256 ⇒ 每条角色少读 1743 字节，之后整段错位（这是最要命的一处）。
    trace_mark(reader, "flags(1999)");
    const FLAG_COUNT: usize = 1999; // Globals.FlagIndexCount
    for _ in 0..FLAG_COUNT {
        reader.read_boolean()?;
    }
    reader.read_raw_i32()?; // guild_index
    reader.read_boolean()?; // allow_trade
    if version > 104 {
        reader.read_boolean()?;
    } // allow_observe

    trace_mark(reader, "quests");
    // CurrentQuests (store indices)
    let quest_count = reader.read_raw_i32()?;
    let mut quests = Vec::new();
    for _ in 0..quest_count {
        quests.push(reader.read_raw_i32()?); // index
        reader.read_datetime()?; // start
        reader.read_datetime()?; // end
                                 // 任务进度表（C# `QuestProgressInfo.cs:85-190`）。
                                 //
                                 // **分支依赖 `Info != null`**：原版按「该 quest 定义是否在库里」选 orphan / 非 orphan
                                 // 两套长度不同的布局。本工具只有 .MirADB（没有 quest 定义表），无法判定，因此按
                                 // **非 orphan（正常）** 布局读——这也是实际数据里的绝大多数；若真遇到 orphan 记录，
                                 // 解析会错位（可用 `MIR2_MIGRATE_TRACE=1` 看出），届时需要先 import quest_infos 再回填。
                                 //
                                 // v>=90 非 orphan：每个 kill task 只写 **一个** i32（当前计数），item task 同样只写一个 i32，
                                 // flag task 只写一个 bool；v<90 的老布局才是 (id,count) 成对。
        let kill_count = reader.read_raw_i32()?;
        for _ in 0..kill_count {
            if version < 90 {
                reader.read_raw_i32()?;
                reader.read_raw_i32()?;
            } else {
                reader.read_raw_i32()?;
            }
        }
        let item_count = reader.read_raw_i32()?;
        for _ in 0..item_count {
            if version < 90 {
                reader.read_raw_i32()?;
                reader.read_raw_i32()?;
            } else {
                reader.read_raw_i32()?;
            }
        }
        let flag_count = reader.read_raw_i32()?;
        for _ in 0..flag_count {
            // 两种布局下 flag 状态都是一个 bool（v<90 的老布局亦然）
            reader.read_boolean()?;
        }
    }

    // Buffs
    trace_mark(reader, "buffs");
    let buff_count = reader.read_raw_i32()?;
    for _ in 0..buff_count {
        reader.read_raw_u8()?; // type
        if version < 88 {
            reader.read_boolean()?;
        }
        reader.read_raw_u32()?;
        reader.read_raw_i64()?;
        if version <= 84 {
            let vc = reader.read_raw_i32()?;
            for _ in 0..vc {
                reader.read_raw_i32()?;
            }
            if version < 88 {
                reader.read_boolean()?;
            }
        } else {
            if version < 88 {
                reader.read_boolean()?;
            }
            let sc = reader.read_raw_i32()?;
            for _ in 0..sc {
                reader.read_raw_u8()?;
                reader.read_raw_i32()?;
            }
            let dc = reader.read_raw_i32()?;
            for _ in 0..dc {
                reader.read_string()?;
                let l = reader.read_raw_i32()?;
                reader.read_bytes(l as usize)?;
            }
            if version > 86 {
                let vc = reader.read_raw_i32()?;
                for _ in 0..vc {
                    reader.read_raw_i32()?;
                }
            }
        }
    }

    // Mail
    trace_mark(reader, "mail");
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
            mail_id,
            sender,
            message,
            gold: mail_gold,
            items,
            date_sent,
            collected,
            locked,
        });
    }

    trace_mark(reader, "creatures");
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
        for i in 0..9 {
            if reader.read_boolean()? {
                pickup_flags |= 1 << i;
            }
        }
        let pickup_grade = if version > 48 {
            reader.read_raw_u8()?
        } else {
            0
        };
        if version > 48 {
            reader.read_raw_i64()?;
        }
        creatures.push(ParsedCreature {
            pet_type,
            custom_name,
            fullness,
            slot_index,
            pet_mode,
            pickup_flags,
            pickup_grade,
        });
    }

    if version == 45 {
        reader.read_raw_u8()?;
        reader.read_boolean()?;
    }
    trace_mark(reader, "pearl_completed_refine_friends");
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

    trace_mark(reader, "rented_gs_heroes");
    // RentedItems
    if version > 75 {
        let ri_count = reader.read_raw_i32()?;
        for _ in 0..ri_count {
            // C# `ItemRentalInformation`（Shared/Data/ItemData.cs:1099-1105）：
            // `ItemId u64 → ItemName string → RentingPlayerName string → ItemReturnDate i64`。
            reader.read_raw_u64()?; // item_id
            reader.read_string()?; // item_name
            reader.read_string()?; // renting_player_name
            reader.read_datetime()?; // item_return_date
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
        for _ in 0..gs_count {
            reader.read_raw_i32()?;
            reader.read_raw_i32()?;
        }
    }

    // Heroes
    let _hero_count = if version > 98 {
        let count = reader.read_raw_i32()?;
        if version > 102 {
            for _ in 0..count {
                reader.read_raw_i32()?;
            }
        } else {
            for _ in 0..count {
                read_character_info(reader, version)?; // inline hero
            }
        }
        if version < 104 {
            reader.read_raw_i32()?;
        }
        count
    } else {
        1
    };
    let current_hero_index = if version > 98 {
        reader.read_raw_i32()?
    } else {
        0
    };
    if version > 98 {
        reader.read_boolean()?;
    } // hero_spawned
    if version > 100 {
        reader.read_raw_u8()?;
    } // hero_behaviour

    Ok(ParsedCharacter {
        name,
        level,
        class_byte,
        _gender_byte,
        _hair,
        current_map_index,
        current_x,
        current_y,
        direction,
        hp,
        mp,
        experience,
        attack_mode,
        pet_mode,
        inventory,
        equipment,
        friends,
        mail,
        quests,
        completed_quests,
        creatures,
        married,
        mentor,
        is_mentor,
        current_hero_index,
    })
}

fn read_account<R: Read>(
    reader: &mut BinaryReader<R>,
    version: i32,
) -> std::io::Result<ParsedAccount> {
    reader.read_raw_i32()?; // index
    let account_id = reader.read_string()?;

    // C# `AccountInfo` 在 v94 前后换了字段名（`Password` → `password`），但**线格式都是
    // 一个 dotnet 字符串**，故这里不需要分支（原先写了同体的 if/else，clippy 判 identical blocks）。
    //
    // 这里必须取**原始字节**（见 `read_dotnet_bytes`）：C# 把 24 字节 PBKDF2 哈希用
    // `Encoding.UTF8.GetString` 变成字符串再落盘。用 `read_string()` 会经过 lossy 转换，
    // 迁移后的账号在 Rust 服务端永远验不过密码（服务端是按字节比对 pbkdf2 结果的）。
    let password_field = reader.read_dotnet_bytes()?;

    let salt = if version > 93 {
        let salt_len = reader.read_raw_i32()?;
        reader.read_bytes(salt_len as usize)?
    } else {
        vec![0u8; 24]
    };

    // 服务端 `verify_password` 认的是 `pbkdf2_sha1$<b64 salt>$<b64 hash>`：
    // 第二段必须是 C# 落盘的那个 24 字节 PBKDF2-SHA1 结果（这里取原始字节，见上）。
    // 原实现写 `salt.clone()` 当哈希 ⇒ 迁移后的账号**必然验不过**密码（这是个静默的功能缺口）。
    let password_hash = password_field;
    if version > 97 {
        reader.read_boolean()?;
    } // require_password_change

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
        let cstart = reader.position();
        let info = read_character_info(reader, version)?;
        if trace_on() {
            eprintln!(
                "[trace] ==== char_end name={:?} {} -> {}（{} 字节）",
                info.name,
                cstart,
                reader.position(),
                reader.position() - cstart
            );
        }
        characters.push(info);
    }

    let _has_expanded_storage = if version > 75 {
        reader.read_boolean()?
    } else {
        false
    };
    if version > 75 {
        reader.read_datetime()?;
    } // expanded_storage_expiry

    let gold = reader.read_raw_u32()? as u64;
    if version >= 63 {
        reader.read_raw_u32()?;
    } // credit

    let storage_count = reader.read_raw_i32()?;
    let mut storage_items: Vec<Option<ParsedUserItem>> = (0..storage_count).map(|_| None).collect();
    for i in 0..storage_count {
        // C# `AccountInfo.Save`（Server/MirDatabase/AccountInfo.cs:242-248）每格先写
        // `Storage[i] != null`，**只有 true 后面才跟一个 UserItem**。
        // 原实现判反了（true 就 continue）⇒ 每个**空**格都被当成有物品去读，
        // 整个账号记录从此错位直到 EOF（实测真实 Server.MirADB 三个账号全部
        // "failed to fill whole buffer"、Accounts migrated: 0）。
        if !reader.read_boolean()? {
            continue;
        }
        let item = read_user_item(reader, version)?;
        if (i as usize) < storage_items.len() {
            storage_items[i as usize] = Some(item);
        }
    }

    if version >= 10 {
        reader.read_boolean()?;
    } // admin_account

    Ok(ParsedAccount {
        account_id,
        salt,
        password_hash,
        characters,
        gold,
        storage_items,
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
    }))
    .unwrap_or_default()
}

fn base64_encode(data: &[u8]) -> String {
    data_encoding::BASE64.encode(data)
}

async fn migrate_account(pool: &sqlx::SqlitePool, account: &ParsedAccount) -> anyhow::Result<()> {
    // Store password with pbkdf2 prefix - will be migrated to Argon2 on first login
    let password_hash = format!(
        "pbkdf2_sha1${}${}",
        base64_encode(&account.salt),
        base64_encode(&account.password_hash)
    );
    // C# 落盘时把 24 字节哈希经 `Encoding.UTF8.GetString` 变字符串：**若那 24 字节不是合法
    // UTF-8，就会丢成 U+FFFD**，原始哈希不可还原（C# 自己两侧都走同一损失转换所以还能比对，
    // Rust 服务端是按字节比对，认不出来）。如实告警，别让用户以为"迁移完就能用原密码登"。
    if account.password_hash.len() != 24 {
        tracing::warn!(
            "账号 {} 的密码哈希长度={}（≠24）⇒ C# 落盘的 UTF-8 损失转换已不可还原，迁移后无法用原密码登录（需 GM 重置密码）",
            account.account_id,
            account.password_hash.len()
        );
    }

    sqlx::query(
        r#"INSERT OR REPLACE INTO accounts (username, password_hash, is_online) VALUES (?, ?, 0)"#,
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

async fn migrate_character(
    pool: &sqlx::SqlitePool,
    account: &ParsedAccount,
    character: &ParsedCharacter,
) -> anyhow::Result<()> {
    let max_hp = character.hp.max(30);
    let max_mp = character.mp.max(10);

    let spouse = if character.married != 0 {
        Some(format!("spouse_{}", character.married))
    } else {
        None
    };
    let mentor = if character.mentor != 0 {
        Some(format!("mentor_{}", character.mentor))
    } else {
        None
    };

    sqlx::query(
        r#"INSERT OR REPLACE INTO characters (
            name, account_username, schema_version, class, gender, hair, map_index, x, y, direction,
            attack_mode, pet_mode, level, experience, max_experience,
            hp, max_hp, mp, max_mp, min_attack, max_attack, defence,
            gold, group_id, guild_name, guild_rank,
            spouse_name, allow_mentor, allow_marriage, mentor_name, hero_index,
            is_fishing, fishing_autocast
        ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#,
    )
    .bind(&character.name)
    .bind(&account.account_id)
    .bind(1i32)
    // class/gender/hair 是服务端角色列表要用的列（C# 里也随角色一起存）
    .bind(character.class_byte as i32)
    .bind(character._gender_byte as i32)
    .bind(character._hair as i32)
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
    .bind(0i32) // allow_marriage（C# AllowMarriage 默认 false）
    .bind(mentor)
    .bind(character.current_hero_index)
    .bind(0i32)
    .bind(0i32)
    .execute(pool)
    .await?;

    // Backpack
    save_items(
        pool,
        &character.name,
        "inventory_backpack",
        &character.inventory,
        "grid",
    )
    .await?;
    // Equipment
    save_items(
        pool,
        &character.name,
        "inventory_equipment",
        &character.equipment,
        "slot",
    )
    .await?;

    // Friends
    sqlx::query("DELETE FROM friends WHERE character_name = ?")
        .bind(&character.name)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM blocked_list WHERE character_name = ?")
        .bind(&character.name)
        .execute(pool)
        .await?;
    for (idx, blocked, memo) in &character.friends {
        if *blocked {
            sqlx::query("INSERT INTO blocked_list (character_name, blocked_object_id, blocked_name) VALUES (?,?,?)")
                .bind(character.name.as_str()).bind(idx).bind(format!("char_{}", idx)).execute(pool).await?;
        } else {
            sqlx::query("INSERT INTO friends (character_name, friend_object_id, friend_name, memo) VALUES (?,?,?,?)")
                .bind(character.name.as_str()).bind(idx).bind(format!("char_{}", idx)).bind(memo).execute(pool).await?;
        }
    }

    // Mail
    sqlx::query("DELETE FROM mail WHERE character_name = ?")
        .bind(&character.name)
        .execute(pool)
        .await?;
    for m in &character.mail {
        let items_json = serde_json::to_string(&m.items.iter().map(|i| {
            serde_json::json!({"UniqueID": i.unique_id, "ItemIndex": i.item_index, "Count": i.count})
        }).collect::<Vec<_>>()).unwrap_or_default();
        sqlx::query(
            r#"INSERT INTO mail (character_name, mail_id, sender_name, subject, body, timestamp,
                read_flag, collected, locked, gold, items_json)
               VALUES (?,?,?,?,?,?,?,?,?,?,?)"#,
        )
        .bind(&character.name)
        .bind(m.mail_id as i64)
        .bind(&m.sender)
        .bind("Migrated")
        .bind(&m.message)
        .bind(m.date_sent)
        .bind(0i32)
        .bind(if m.collected { 1 } else { 0 })
        .bind(if m.locked { 1 } else { 0 })
        .bind(m.gold as i64)
        .bind(&items_json)
        .execute(pool)
        .await?;
    }

    // Quests
    sqlx::query("DELETE FROM quests WHERE character_name = ?")
        .bind(&character.name)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM completed_quests WHERE character_name = ?")
        .bind(&character.name)
        .execute(pool)
        .await?;
    for qi in &character.quests {
        sqlx::query("INSERT INTO quests (character_name, quest_index, title, status, progress_json, exp_reward, gold_reward) VALUES (?,?,?,?,?,?,?)")
            .bind(&character.name).bind(qi).bind(format!("Quest {}", qi)).bind("InProgress")
            .bind("[]").bind(0i64).bind(0i64).execute(pool).await?;
    }
    for qi in &character.completed_quests {
        sqlx::query("INSERT INTO completed_quests (character_name, quest_index) VALUES (?,?)")
            .bind(&character.name)
            .bind(qi)
            .execute(pool)
            .await?;
    }

    // Creatures
    if let Some(c) = character.creatures.first() {
        let owned_json = serde_json::to_string(
            &character
                .creatures
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "creature_type": c.pet_type,
                        "custom_name": c.custom_name,
                        "pickup_mode": c.pickup_grade,
                        "hunger": c.fullness as u8,
                        "enabled": true
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default();
        sqlx::query(
            r#"INSERT OR REPLACE INTO creatures (
                character_name, active_type, active_custom_name, active_pickup_mode,
                active_hunger, active_enabled, owned_json, request_updates
            ) VALUES (?,?,?,?,?,?,?,?)"#,
        )
        .bind(&character.name)
        .bind(c.pet_type as i32)
        .bind(&c.custom_name)
        .bind(c.pickup_grade as i32)
        .bind(c.fullness as u8)
        .bind(1i32)
        .bind(&owned_json)
        .bind(0i32)
        .execute(pool)
        .await?;
    }

    info!(
        "    Character: {} (Lv{}, class {})",
        character.name, character.level, character.class_byte
    );
    Ok(())
}

fn attack_mode_str(b: u8) -> &'static str {
    match b {
        1 => "Group",
        2 => "Guild",
        3 => "EnemyGuild",
        4 => "RedBrown",
        5 => "All",
        _ => "Peace",
    }
}

fn pet_mode_str(b: u8) -> &'static str {
    match b {
        1 => "MoveOnly",
        2 => "AttackOnly",
        3 => "None",
        4 => "FocusMasterTarget",
        _ => "Both",
    }
}

async fn save_items(
    pool: &sqlx::SqlitePool,
    char_name: &str,
    table: &str,
    items: &[Option<ParsedUserItem>],
    col: &str,
) -> anyhow::Result<()> {
    sqlx::query(&format!("DELETE FROM {} WHERE character_name = ?", table))
        .bind(char_name)
        .execute(pool)
        .await?;
    for (i, item) in items.iter().enumerate() {
        if let Some(item) = item {
            let item_json = item_to_json(item);
            sqlx::query(&format!(
                "INSERT INTO {} (character_name, {}, item_json) VALUES (?,?,?)",
                table, col
            ))
            .bind(char_name)
            .bind(i as i32)
            .bind(&item_json)
            .execute(pool)
            .await?;
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
    let adb_size = data.len();

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
            reader.read_string()?; // name
                                   // We can't easily skip the rest without full HeroInfo structure,
                                   // but for version=83 this branch won't execute anyway
        }
    }

    info!("Version: {}.{}", version, custom_version);
    info!(
        "Next IDs: account={}, character={}, item={}, hero={}",
        next_account_id, next_character_id, next_user_item_id, next_hero_id
    );
    info!("Guilds: {}, NextGuildID: {}", guild_count, next_guild_id);

    // Account count (comes after optional HeroList)
    let account_count = reader.read_raw_i32()?;
    info!("Accounts to migrate: {}", account_count);

    // Initialize database
    // 与 `migrate_mirdb` 同一套口径：绝对化 → 反斜杠转正斜杠 → 建父目录 → 建文件 → 再连。
    // 原实现直接 `format!("sqlite://{}", sqlite_path)`：Windows 下会拼出
    // `sqlite://C:\Users\...\adb.db` 这种 URL，sqlx 解析后打不开（实测报 code 14 unable to open
    // database file），而且 sqlx 不会替你创建目录/文件 ⇒ 文档里「旧格式 .MirADB 用 migrate 子命令」
    // 这条路在 Windows 上是死的（2026-09-25 用真实 Server.MirADB 实跑发现）。
    let abs_path = if std::path::Path::new(sqlite_path).is_absolute() {
        sqlite_path.to_string()
    } else {
        let cwd = std::env::current_dir()
            .map_err(|e| anyhow::anyhow!("Failed to get current dir: {}", e))?;
        cwd.join(sqlite_path).to_string_lossy().to_string()
    };
    let normalized = abs_path.replace('\\', "/");
    let db_url = format!("sqlite://{}", normalized);
    info!("DB URL: {}", db_url);

    // Ensure parent directory and file exist (sqlx won't create them)
    if let Some(parent) = std::path::Path::new(&abs_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            anyhow::anyhow!("Failed to create directory {}: {}", parent.display(), e)
        })?;
    }
    std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&abs_path)
        .map_err(|e| anyhow::anyhow!("Failed to create DB file {}: {}", abs_path, e))?;
    // FK 必须连接选项池级禁用：PRAGMA foreign_keys 是【每连接】设置，sqlx 默认每条新连接
    // FK ON——INSERT OR REPLACE INTO characters 在 FK ON 连接上会触发子表级联删除+重插
    // 导致 FK constraint failed（与 db::init_db_pool 同理；数据完整性由应用层事务保证）。
    let options = db_url
        .parse::<sqlx::sqlite::SqliteConnectOptions>()
        .map_err(|e| anyhow::anyhow!("Failed to parse DB URL {}: {}", db_url, e))?
        .foreign_keys(false)
        .busy_timeout(std::time::Duration::from_secs(5));
    let pool = sqlx::SqlitePool::connect_with(options)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to {}: {}", sqlite_path, e))?;

    // 先让**服务端自己的** schema 初始化器建表（单一真源），再建工具自己的表。
    //
    // 为什么必须这样：工具原来用自己的一套 `CREATE TABLE IF NOT EXISTS characters (...)`，
    // 而那份 DDL 缺少服务端列（`class`/`gender`/`hair`/`is_dead`/`pk_points`/`banned`…），
    // `IF NOT EXISTS` 让服务端启动时的建表变成空操作 ⇒ 迁移出来的库**登录能过、角色列表查不出来**
    // （实测 `Failed to list characters for '333': no such column: class`，进图因此卡住）。
    // 交给 `db::init_db_pool` 建表后，下面这些 `IF NOT EXISTS` 自然变成空操作，不会再各建一套。
    let _ = crystal_server::db::init_db_pool(&db_url).await?;
    info!("Server schema ensured via db::init_db_pool（单一真源）");
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
            allow_marriage INTEGER NOT NULL DEFAULT 0,
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
        "#,
    )
    .execute(&pool)
    .await?;

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
                    info!(
                        "  Account #{}: {} ({} characters)",
                        i, account.account_id, chars
                    );
                    account_count_success += 1;
                    character_count_success += chars;
                }
            }
            Err(e) => {
                error!(
                    "Failed to read account #{}: {}（读到 pos={}）",
                    i,
                    e,
                    reader.position(),
                );
                error_count += 1;
            }
        }
    }

    // 账号之后的收尾段（C# `Envir.SaveAccounts`，Server/MirEnvir/Envir.cs:2591-2610）：
    //   NextAuctionID i32 → Auctions(count + AuctionInfo*) → NextMailID i32
    //   → GameshopLog(count + (i32,i32)*) → SavedSpawns(count + RespawnSave*)
    // 工具此前完全不读这段 ⇒ 收尾字节没被消费（实测差 28 字节）。
    // AuctionInfo：Server/MirDatabase/AuctionInfo.cs:47-69；
    // RespawnSave：Server/MirEnvir/RespawnTimer.cs:11-16。
    trace_mark(&reader, "post_accounts(auctions/mail/gameshoplog/spawns)");
    // 注意：`Envir.NextAuctionID` / `NextMailID` 是 **ulong**（Envir.cs:127），不是 i32
    let _next_auction_id = reader.read_raw_u64()?;
    let auction_count = read_bounded_count(&mut reader, "auctions")?;
    for _ in 0..auction_count {
        reader.read_raw_u64()?; // auction_id
        read_user_item(&mut reader, version)?; // item
        reader.read_datetime()?; // consignment_date
        reader.read_raw_u32()?; // price
        reader.read_raw_i32()?; // seller_index
        reader.read_boolean()?; // expired
        reader.read_boolean()?; // sold
        if version > 79 {
            reader.read_raw_u8()?; // item_type
            reader.read_raw_u32()?; // current_bid
            reader.read_raw_i32()?; // current_buyer_index
        }
    }
    let _next_mail_id = reader.read_raw_u64()?;
    let gs_log_count = read_bounded_count(&mut reader, "gameshop_log")?;
    for _ in 0..gs_log_count {
        reader.read_raw_i32()?;
        reader.read_raw_i32()?;
    }
    let spawn_count = read_bounded_count(&mut reader, "saved_spawns")?;
    for _ in 0..spawn_count {
        reader.read_boolean()?; // spawned
        reader.read_raw_u64()?; // next_spawn_tick
        reader.read_raw_i32()?; // respawn_index
    }

    info!("=== Migration Complete ===");
    info!("Accounts migrated: {}", account_count_success);
    info!("Characters migrated: {}", character_count_success);
    info!("Errors: {}", error_count);
    info!("Database: {}", sqlite_path);
    // 结构自证：读到的位置应当**恰好等于文件大小**（没有错位多读/少读）。
    // 这条判据比"行数看着对"硬得多——2026-09-25 就是靠它确认 v112 布局已对齐。
    info!(
        "File consumed: {} / {} bytes{}",
        reader.position(),
        adb_size,
        if reader.position() == adb_size {
            "（完全对齐）"
        } else {
            "（⚠️ 有错位）"
        }
    );

    // Verify
    let row_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM accounts")
        .fetch_one(&pool)
        .await?;
    let char_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM characters")
        .fetch_one(&pool)
        .await?;
    let item_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM inventory_backpack")
        .fetch_one(&pool)
        .await?;

    info!(
        "Verification: {} accounts, {} characters, {} backpack items in DB",
        row_count.0, char_count.0, item_count.0
    );

    Ok(())
}
