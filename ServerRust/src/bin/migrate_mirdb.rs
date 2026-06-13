// Migration tool: C# Server.MirDB -> SQLite
// Usage: cargo run --bin migrate_mirdb -- <path-to-Server.MirDB> [sqlite-db-path]
// Default sqlite path: data/crystal.db

#![allow(dead_code)]

use std::collections::HashMap;
use std::io::Read;
use byteorder::{LittleEndian, ReadBytesExt};
use tracing::{info, error, warn};

// ============================================================
// BinaryReader compatible with C# BinaryReader
// ============================================================

struct BinaryReader<R: Read> {
    inner: R,
    pos: usize,
}

impl<R: Read> BinaryReader<R> {
    fn new(inner: R) -> Self { Self { inner, pos: 0 } }
    fn position(&self) -> usize { self.pos }
    fn read_raw_i32(&mut self) -> std::io::Result<i32> { self.pos += 4; self.inner.read_i32::<LittleEndian>() }
    fn read_raw_u32(&mut self) -> std::io::Result<u32> { self.pos += 4; self.inner.read_u32::<LittleEndian>() }
    fn read_raw_i64(&mut self) -> std::io::Result<i64> { self.pos += 8; self.inner.read_i64::<LittleEndian>() }
    fn read_raw_u64(&mut self) -> std::io::Result<u64> { self.pos += 8; self.inner.read_u64::<LittleEndian>() }
    fn read_raw_u16(&mut self) -> std::io::Result<u16> { self.pos += 2; self.inner.read_u16::<LittleEndian>() }
    fn read_raw_i16(&mut self) -> std::io::Result<i16> { self.pos += 2; self.inner.read_i16::<LittleEndian>() }
    fn read_raw_u8(&mut self) -> std::io::Result<u8> { self.pos += 1; self.inner.read_u8() }
    fn read_raw_i8(&mut self) -> std::io::Result<i8> { self.pos += 1; self.inner.read_i8() }
    fn read_raw_f32(&mut self) -> std::io::Result<f32> { self.pos += 4; self.inner.read_f32::<LittleEndian>() }
    fn read_raw_f64(&mut self) -> std::io::Result<f64> { self.pos += 8; self.inner.read_f64::<LittleEndian>() }
    fn read_boolean(&mut self) -> std::io::Result<bool> { self.pos += 1; Ok(self.inner.read_u8()? != 0) }

    fn read_string(&mut self) -> std::io::Result<String> {
        let mut len: u32 = 0;
        let mut shift = 0;
        loop {
            let b = self.inner.read_u8()?;
            self.pos += 1;
            len |= ((b & 0x7F) as u32) << shift;
            shift += 7;
            if b & 0x80 == 0 { break; }
            if shift > 35 { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "String length overflow")); }
        }
        self.pos += len as usize;
        let mut buf = vec![0u8; len as usize];
        self.inner.read_exact(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf).to_string())
    }

    fn read_bytes(&mut self, count: usize) -> std::io::Result<Vec<u8>> {
        self.pos += count;
        let mut buf = vec![0u8; count];
        self.inner.read_exact(&mut buf)?;
        Ok(buf)
    }
}

// ============================================================
// Parsed data structures
// ============================================================

struct ParsedMapInfo {
    index: i32,
    file_name: String,
    title: String,
    mini_map: u16,
    light: u8,
    big_map: u16,
    safe_zones: Vec<ParsedSafeZone>,
    respawns: Vec<ParsedRespawnInfo>,
    movements: Vec<ParsedMovementInfo>,
    no_teleport: bool,
    no_reconnect: bool,
    no_reconnect_map: String,
    no_random: bool,
    no_escape: bool,
    no_recall: bool,
    no_drug: bool,
    no_position: bool,
    no_throw_item: bool,
    no_drop_player: bool,
    no_drop_monster: bool,
    no_names: bool,
    fight: bool,
    fire: bool,
    fire_damage: i32,
    lightning: bool,
    lightning_damage: i32,
    map_dark_light: u8,
    mine_zones: Vec<ParsedMineZone>,
    mine_index: u8,
    no_mount: bool,
    need_bridle: bool,
    no_fight: bool,
    music: u16,
    no_town_teleport: bool,
    no_reincarnation: bool,
    weather_particles: u16,
    gt: bool,
    gt_index: u8,
}

struct ParsedSafeZone {
    x: i32,
    y: i32,
    size: u16,
    start_point: bool,
}

struct ParsedRespawnInfo {
    monster_index: i32,
    x: i32,
    y: i32,
    count: u16,
    spread: u16,
    delay: u16,
    direction: u8,
    route_path: String,
    random_delay: u16,
    respawn_index: i32,
    save_respawn_time: bool,
    respawn_ticks: u16,
}

struct ParsedMovementInfo {
    map_index: i32,
    source_x: i32,
    source_y: i32,
    dest_x: i32,
    dest_y: i32,
    need_hole: bool,
    need_move: bool,
    conquest_index: i32,
    show_on_big_map: bool,
    icon: i32,
}

struct ParsedMineZone {
    x: i32,
    y: i32,
    size: u16,
    mine: u8,
}

struct ParsedItemInfo {
    index: i32,
    name: String,
    type_byte: u8,
    grade: u8,
    required_type: u8,
    required_class: u8,
    required_gender: u8,
    set_type: u8,
    shape: i16,
    weight: u8,
    light: u8,
    required_amount: u8,
    image: u16,
    durability: u16,
    stack_size: u16,
    price: u32,
    start_item: bool,
    effect: u8,
    bool_flags: u8,
    bind_mode: i16,
    special_mode: i16,
    random_stats_id: u8,
    can_fast_run: bool,
    can_awakening: bool,
    slots: u8,
    stats_json: String,
    has_tool_tip: bool,
    tool_tip: String,
}

struct ParsedMonsterInfo {
    index: i32,
    name: String,
    image: u16,
    ai: u8,
    effect: u8,
    level: u16,
    view_range: u8,
    cool_eye: u8,
    stats_json: String,
    light: u8,
    attack_speed: u16,
    move_speed: u16,
    experience: u32,
    can_push: bool,
    can_tame: bool,
    auto_rev: bool,
    undead: bool,
    drop_path: String,
}

struct ParsedNPCInfo {
    index: i32,
    map_index: i32,
    collect_quest_indexes: String,
    finish_quest_indexes: String,
    file_name: String,
    name: String,
    x: i32,
    y: i32,
    image: u16,
    rate: u16,
    time_visible: bool,
    hour_start: u8,
    minute_start: u8,
    hour_end: u8,
    minute_end: u8,
    min_lev: i16,
    max_lev: i16,
    day_of_week: String,
    class_required: String,
    conquest: i32,
    flag_needed: i32,
    show_on_big_map: bool,
    big_map_icon: i32,
    can_teleport_to: bool,
    conquest_visible: bool,
}

struct ParsedQuestInfo {
    index: i32,
    name: String,
    group_name: String,
    file_name: String,
    required_min_level: i32,
    required_max_level: i32,
    required_quest: i32,
    required_class: u8,
    quest_type: u8,
    exp_reward: i32,
    gold_reward: i32,
    goto_message: String,
    kill_message: String,
    item_message: String,
    flag_message: String,
    time_limit_seconds: i32,
}

struct ParsedDragonInfo {
    enabled: bool,
    map_file_name: String,
    monster_name: String,
    body_name: String,
    location_x: i32,
    location_y: i32,
    drop_area_top_x: i32,
    drop_area_top_y: i32,
    drop_area_bottom_x: i32,
    drop_area_bottom_y: i32,
    exps_json: String,
}

struct ParsedMagicInfo {
    name: String,
    spell: u8,
    base_cost: u8,
    level_cost: u8,
    icon: u8,
    level1: u8,
    level2: u8,
    level3: u8,
    need1: u16,
    need2: u16,
    need3: u16,
    delay_base: u32,
    delay_reduction: u32,
    power_base: u16,
    power_bonus: u16,
    mpower_base: u16,
    mpower_bonus: u16,
    range: u8,
    multiplier_base: f32,
    multiplier_bonus: f32,
}

struct ParsedGameShopItem {
    item_index: i32,
    gindex: i32,
    gold_price: u32,
    credit_price: u32,
    count: u16,
    class_name: String,
    category: String,
    stock: i32,
    infinite_stock: bool,
    deal: bool,
    top_item: bool,
    date: i64,
    can_buy_credit: bool,
    can_buy_gold: bool,
}

struct ParsedConquestInfo {
    index: i32,
    full_map: bool,
    location_x: i32,
    location_y: i32,
    size: u16,
    name: String,
    map_index: i32,
    palace_index: i32,
    guard_index: i32,
    gate_index: i32,
    wall_index: i32,
    siege_index: i32,
    flag_index: i32,
    extra_maps_json: String,
    start_hour: u8,
    war_length: i32,
    conquest_type: u8,
    conquest_game: u8,
    days: [bool; 7],
    king_x: i32,
    king_y: i32,
    king_size: u16,
    control_point_index: i32,
}

struct ParsedGTMap {
    index: i32,
    key: i32,
    name: String,
    owner: String,
    leader: String,
    leader2: String,
    price: i32,
    days: i32,
    begin_time: i32,
}

// ============================================================
// Stats dictionary parser
// ============================================================

fn read_stats<R: Read>(reader: &mut BinaryReader<R>) -> std::io::Result<HashMap<u8, i32>> {
    let count = reader.read_raw_i32()?;
    let mut map = HashMap::new();
    for _ in 0..count {
        let key = reader.read_raw_u8()?;
        let val = reader.read_raw_i32()?;
        map.insert(key, val);
    }
    Ok(map)
}

fn read_stats_dict<R: Read>(reader: &mut BinaryReader<R>) -> std::io::Result<HashMap<u8, i32>> {
    // Stats.Save() format: count (Int32) + entries (key: byte, value: Int32)
    let count = reader.read_raw_i32()?;
    let mut stats = HashMap::new();
    for _ in 0..count {
        let key = reader.read_raw_u8()?;
        let value = reader.read_raw_i32()?;
        stats.insert(key, value);
    }
    Ok(stats)
}

fn stats_to_json(stats: &HashMap<u8, i32>) -> String {
    let items: Vec<String> = stats.iter()
        .map(|(k, v)| format!("\"{}\":{}", k, v))
        .collect();
    format!("{{{}}}", items.join(","))
}

// ============================================================
// Parsing functions
// ============================================================

fn read_safe_zone<R: Read>(reader: &mut BinaryReader<R>) -> std::io::Result<ParsedSafeZone> {
    let x = reader.read_raw_i32()?;
    let y = reader.read_raw_i32()?;
    let size = reader.read_raw_u16()?;
    let start_point = reader.read_boolean()?;
    Ok(ParsedSafeZone { x, y, size, start_point })
}

fn read_respawn_info<R: Read>(reader: &mut BinaryReader<R>, _version: i32) -> std::io::Result<ParsedRespawnInfo> {
    let monster_index = reader.read_raw_i32()?;
    let x = reader.read_raw_i32()?;
    let y = reader.read_raw_i32()?;
    let count = reader.read_raw_u16()?;
    let spread = reader.read_raw_u16()?;
    let delay = reader.read_raw_u16()?;
    let direction = reader.read_raw_u8()?;
    let route_path = reader.read_string()?;

    // Save always writes these fields
    let random_delay = reader.read_raw_u16()?;
    let respawn_index = reader.read_raw_i32()?;
    let save_respawn_time = reader.read_boolean()?;
    let respawn_ticks = reader.read_raw_u16()?;

    Ok(ParsedRespawnInfo {
        monster_index, x, y, count, spread, delay, direction, route_path,
        random_delay, respawn_index, save_respawn_time, respawn_ticks,
    })
}

fn read_movement_info<R: Read>(reader: &mut BinaryReader<R>, version: i32) -> std::io::Result<ParsedMovementInfo> {
    let map_index = reader.read_raw_i32()?;
    let source_x = reader.read_raw_i32()?;
    let source_y = reader.read_raw_i32()?;
    let dest_x = reader.read_raw_i32()?;
    let dest_y = reader.read_raw_i32()?;
    let need_hole = reader.read_boolean()?;
    let need_move = reader.read_boolean()?;

    // ConquestIndex added at v69; ShowOnBigMap+Icon added at v95
    let (conquest_index, show_on_big_map, icon) = if version >= 95 {
        (reader.read_raw_i32()?, reader.read_boolean()?, reader.read_raw_i32()?)
    } else if version >= 69 {
        (reader.read_raw_i32()?, false, 0)
    } else {
        (0, false, 0)
    };

    Ok(ParsedMovementInfo {
        map_index, source_x, source_y, dest_x, dest_y,
        need_hole, need_move, conquest_index, show_on_big_map, icon,
    })
}

fn read_mine_zone<R: Read>(reader: &mut BinaryReader<R>) -> std::io::Result<ParsedMineZone> {
    let x = reader.read_raw_i32()?;
    let y = reader.read_raw_i32()?;
    let size = reader.read_raw_u16()?;
    let mine = reader.read_raw_u8()?;
    Ok(ParsedMineZone { x, y, size, mine })
}

fn read_map_info<R: Read>(reader: &mut BinaryReader<R>, version: i32) -> std::io::Result<ParsedMapInfo> {
    let index = reader.read_raw_i32()?;
    let file_name = reader.read_string()?;
    let title = reader.read_string()?;
    let mini_map = reader.read_raw_u16()?;
    let light = reader.read_raw_u8()?;
    let big_map = reader.read_raw_u16()?;

    // SafeZones
    let sz_count = reader.read_raw_i32()?;
    let mut safe_zones = Vec::new();
    for _ in 0..sz_count {
        safe_zones.push(read_safe_zone(reader)?);
    }

    // Respawns
    let rs_count = reader.read_raw_i32()?;
    let mut respawns = Vec::new();
    for _ in 0..rs_count {
        respawns.push(read_respawn_info(reader, version)?);
    }

    // Movements
    let mv_count = reader.read_raw_i32()?;
    let mut movements = Vec::new();
    for _ in 0..mv_count {
        movements.push(read_movement_info(reader, version)?);
    }

    let no_teleport = reader.read_boolean()?;
    let no_reconnect = reader.read_boolean()?;
    let no_reconnect_map = reader.read_string()?;
    let no_random = reader.read_boolean()?;
    let no_escape = reader.read_boolean()?;
    let no_recall = reader.read_boolean()?;
    let no_drug = reader.read_boolean()?;
    let no_position = reader.read_boolean()?;
    let no_throw_item = reader.read_boolean()?;
    let no_drop_player = reader.read_boolean()?;
    let no_drop_monster = reader.read_boolean()?;
    let no_names = reader.read_boolean()?;
    let fight = reader.read_boolean()?;
    let fire = reader.read_boolean()?;
    let fire_damage = reader.read_raw_i32()?;
    let lightning = reader.read_boolean()?;
    let lightning_damage = reader.read_raw_i32()?;
    let map_dark_light = reader.read_raw_u8()?;

    // MineZones
    let mz_count = reader.read_raw_i32()?;
    let mut mine_zones = Vec::new();
    for _ in 0..mz_count {
        mine_zones.push(read_mine_zone(reader)?);
    }

    let mine_index = reader.read_raw_u8()?;
    let no_mount = reader.read_boolean()?;
    let need_bridle = reader.read_boolean()?;
    let no_fight = reader.read_boolean()?;
    let music = reader.read_raw_u16()?;
    // Version-gated tail fields: these were added to Save in later commits
    // The MirDB was saved by the server at version 83, which didn't have these yet
    let no_town_teleport = if version >= 78 { reader.read_boolean()? } else { false };
    let no_reincarnation = if version >= 79 { reader.read_boolean()? } else { false };
    let weather_particles = if version >= 110 { reader.read_raw_u16()? } else { 0 };
    let gt = if version >= 111 { reader.read_boolean()? } else { false };
    let gt_index = if version >= 111 { reader.read_raw_u8()? } else { 0 };

    Ok(ParsedMapInfo {
        index, file_name, title, mini_map, light, big_map, safe_zones, respawns, movements,
        no_teleport, no_reconnect, no_reconnect_map, no_random, no_escape, no_recall,
        no_drug, no_position, no_throw_item, no_drop_player, no_drop_monster, no_names,
        fight, fire, fire_damage, lightning, lightning_damage, map_dark_light,
        mine_zones, mine_index, no_mount, need_bridle, no_fight, music,
        no_town_teleport, no_reincarnation, weather_particles, gt, gt_index,
    })
}

fn read_item_info<R: Read>(reader: &mut BinaryReader<R>, _version: i32) -> std::io::Result<ParsedItemInfo> {
    // File was saved with OLD Save format (before commit 8d03fafe added Slots to Save).
    // Format: Index, Name, 6 type/grade bytes, Shape(i16), Weight, Light, ReqAmount,
    // Image(u16), Durability(u16), StackSize(u32), Price(u32),
    // MinAC..MaxSC (10 u8), HP(u16), MP(u16), Acc(u8), Agi(u8), Luck(s8), AtkSpd(s8),
    // StartItem(bool), BagWeight, HandWeight, WearWeight (3 u8), Effect(u8),
    // Strong..CriticalDamage (10 u8), BoolFlags(u8), MaxAcRate..PoisonAttack (5 u8),
    // Bind(i16), Reflect(u8), HpDrainRate(u8), Unique(i16), RandomStatsId(u8),
    // CanFastRun(bool), CanAwakening(bool),
    // HasToolTip(bool), ToolTip(string)
    let index = reader.read_raw_i32()?;
    let name = reader.read_string()?;
    let type_byte = reader.read_raw_u8()?;
    let grade = reader.read_raw_u8()?;
    let required_type = reader.read_raw_u8()?;
    let required_class = reader.read_raw_u8()?;
    let required_gender = reader.read_raw_u8()?;
    let set_type = reader.read_raw_u8()?;

    let shape = reader.read_raw_i16()?;
    let weight = reader.read_raw_u8()?;
    let light = reader.read_raw_u8()?;
    let required_amount = reader.read_raw_u8()?;

    let image = reader.read_raw_u16()?;
    let durability = reader.read_raw_u16()?;

    let stack_size = reader.read_raw_u32()? as u16;
    let price = reader.read_raw_u32()?;

    let mut stats = HashMap::new();
    stats.insert(0u8, reader.read_raw_u8()? as i32);   // MinAC
    stats.insert(1u8, reader.read_raw_u8()? as i32);   // MaxAC
    stats.insert(2u8, reader.read_raw_u8()? as i32);   // MinMAC
    stats.insert(3u8, reader.read_raw_u8()? as i32);   // MaxMAC
    stats.insert(4u8, reader.read_raw_u8()? as i32);   // MinDC
    stats.insert(5u8, reader.read_raw_u8()? as i32);   // MaxDC
    stats.insert(6u8, reader.read_raw_u8()? as i32);   // MinMC
    stats.insert(7u8, reader.read_raw_u8()? as i32);   // MaxMC
    stats.insert(8u8, reader.read_raw_u8()? as i32);   // MinSC
    stats.insert(9u8, reader.read_raw_u8()? as i32);   // MaxSC
    stats.insert(12u8, reader.read_raw_u16()? as i32); // HP
    stats.insert(13u8, reader.read_raw_u16()? as i32); // MP
    stats.insert(10u8, reader.read_raw_u8()? as i32);  // Accuracy
    stats.insert(11u8, reader.read_raw_u8()? as i32);  // Agility
    stats.insert(15u8, reader.read_raw_i8()? as i32);  // Luck
    stats.insert(14u8, reader.read_raw_i8()? as i32);  // AttackSpeed

    let start_item = reader.read_boolean()?;

    stats.insert(16u8, reader.read_raw_u8()? as i32);  // BagWeight
    stats.insert(17u8, reader.read_raw_u8()? as i32);  // HandWeight
    stats.insert(18u8, reader.read_raw_u8()? as i32);  // WearWeight

    let effect = reader.read_raw_u8()?;

    stats.insert(20u8, reader.read_raw_u8()? as i32);  // Strong
    stats.insert(30u8, reader.read_raw_u8()? as i32);  // MagicResist
    stats.insert(31u8, reader.read_raw_u8()? as i32);  // PoisonResist
    stats.insert(32u8, reader.read_raw_u8()? as i32);  // HealthRecovery
    stats.insert(33u8, reader.read_raw_u8()? as i32);  // SpellRecovery
    stats.insert(34u8, reader.read_raw_u8()? as i32);  // PoisonRecovery
    stats.insert(46u8, reader.read_raw_u8()? as i32);  // HPRatePercent
    stats.insert(47u8, reader.read_raw_u8()? as i32);  // MPRatePercent
    stats.insert(35u8, reader.read_raw_u8()? as i32);  // CriticalRate
    stats.insert(36u8, reader.read_raw_u8()? as i32);  // CriticalDamage

    let bool_flags = reader.read_raw_u8()?;

    stats.insert(40u8, reader.read_raw_u8()? as i32);  // MaxACRatePercent
    stats.insert(41u8, reader.read_raw_u8()? as i32);  // MaxMACRatePercent
    stats.insert(21u8, reader.read_raw_u8()? as i32);  // Holy
    stats.insert(22u8, reader.read_raw_u8()? as i32);  // Freezing
    stats.insert(23u8, reader.read_raw_u8()? as i32);  // PoisonAttack

    let bind_mode = reader.read_raw_i16()?;

    stats.insert(19u8, reader.read_raw_u8()? as i32);  // Reflect
    stats.insert(48u8, reader.read_raw_u8()? as i32);  // HPDrainRatePercent

    let special_mode = reader.read_raw_i16()?; // Unique
    let random_stats_id = reader.read_raw_u8()?;
    let can_fast_run = reader.read_boolean()?;
    let can_awakening = reader.read_boolean()?;

    // No Slots in this version of the file (added in commit 8d03fafe)
    let slots = 0u8;

    let stats_json = stats_to_json(&stats);

    let has_tool_tip = reader.read_boolean()?;
    let tool_tip = if has_tool_tip { reader.read_string()? } else { String::new() };

    Ok(ParsedItemInfo {
        index, name, type_byte, grade, required_type, required_class, required_gender,
        set_type, shape, weight, light, required_amount, image, durability, stack_size,
        price, start_item, effect, bool_flags, bind_mode, special_mode, random_stats_id,
        can_fast_run, can_awakening, slots, stats_json, has_tool_tip, tool_tip,
    })
}

fn read_monster_info<R: Read>(reader: &mut BinaryReader<R>, _version: i32) -> std::io::Result<ParsedMonsterInfo> {
    // File was saved with OLD MonsterInfo Save format (before commit 4ee54261).
    // Format: Index(i32), Name, Image(u16), AI(u8), Effect(u8), Level(u16),
    // ViewRange(u8), CoolEye(u8), HP(u32),
    // MinAC..MaxSC (10 x u16), Accuracy(u8), Agility(u8),
    // Light(u8), AttackSpeed(u16), MoveSpeed(u16), Experience(u32),
    // CanPush, CanTame, AutoRev, Undead (4 bools)
    // NO DropPath, NO Stats.Save() dictionary
    let index = reader.read_raw_i32()?;
    let name = reader.read_string()?;
    let image = reader.read_raw_u16()?;
    let ai = reader.read_raw_u8()?;
    let effect = reader.read_raw_u8()?;
    let level = reader.read_raw_u16()?;
    let view_range = reader.read_raw_u8()?;
    let cool_eye = reader.read_raw_u8()?;

    // Old format: HP as UInt32, then 10 stats as UInt16, then Accuracy/Agility as UInt8
    let mut stats = HashMap::new();
    stats.insert(38u8, reader.read_raw_u32()? as i32); // HP

    stats.insert(0u8, reader.read_raw_u16()? as i32);  // MinAC
    stats.insert(1u8, reader.read_raw_u16()? as i32);  // MaxAC
    stats.insert(2u8, reader.read_raw_u16()? as i32);  // MinMAC
    stats.insert(3u8, reader.read_raw_u16()? as i32);  // MaxMAC
    stats.insert(4u8, reader.read_raw_u16()? as i32);  // MinDC
    stats.insert(5u8, reader.read_raw_u16()? as i32);  // MaxDC
    stats.insert(6u8, reader.read_raw_u16()? as i32);  // MinMC
    stats.insert(7u8, reader.read_raw_u16()? as i32);  // MaxMC
    stats.insert(8u8, reader.read_raw_u16()? as i32);  // MinSC
    stats.insert(9u8, reader.read_raw_u16()? as i32);  // MaxSC

    stats.insert(10u8, reader.read_raw_u8()? as i32);  // Accuracy
    stats.insert(11u8, reader.read_raw_u8()? as i32);  // Agility

    let stats_json = stats_to_json(&stats);

    let light = reader.read_raw_u8()?;
    let attack_speed = reader.read_raw_u16()?;
    let move_speed = reader.read_raw_u16()?;
    let experience = reader.read_raw_u32()?;
    let can_push = reader.read_boolean()?;
    let can_tame = reader.read_boolean()?;
    let auto_rev = reader.read_boolean()?;
    let undead = reader.read_boolean()?;

    // No DropPath in old format
    let drop_path = String::new();

    Ok(ParsedMonsterInfo {
        index, name, image, ai, effect, level, view_range, cool_eye, stats_json,
        light, attack_speed, move_speed, experience, can_push, can_tame,
        auto_rev, undead, drop_path,
    })
}

fn read_npc_info<R: Read>(reader: &mut BinaryReader<R>, version: i32) -> std::io::Result<ParsedNPCInfo> {
    let index = reader.read_raw_i32()?;
    let map_index = reader.read_raw_i32()?;

    // CollectQuestIndexes
    let cq_count = reader.read_raw_i32()?;
    let mut collect_quests = Vec::new();
    for _ in 0..cq_count {
        collect_quests.push(reader.read_raw_i32()?);
    }
    let collect_quest_indexes = serde_json::to_string(&collect_quests).unwrap_or_default();

    // FinishQuestIndexes
    let fq_count = reader.read_raw_i32()?;
    let mut finish_quests = Vec::new();
    for _ in 0..fq_count {
        finish_quests.push(reader.read_raw_i32()?);
    }
    let finish_quest_indexes = serde_json::to_string(&finish_quests).unwrap_or_default();

    let file_name = reader.read_string()?;
    let name = reader.read_string()?;
    let x = reader.read_raw_i32()?;
    let y = reader.read_raw_i32()?;

    // Image: v72+ is UInt16, below is byte
    let image = if version >= 72 {
        reader.read_raw_u16()?
    } else {
        reader.read_raw_u8()? as u16
    };

    let rate = reader.read_raw_u16()?;

    // TimeVisible block: v64+
    let (time_visible, hour_start, minute_start, hour_end, minute_end, min_lev, max_lev, day_of_week, class_required) =
        if version >= 64 {
            (
                reader.read_boolean()?,
                reader.read_raw_u8()?,
                reader.read_raw_u8()?,
                reader.read_raw_u8()?,
                reader.read_raw_u8()?,
                reader.read_raw_i16()?,
                reader.read_raw_i16()?,
                reader.read_string()?,
                reader.read_string()?,
            )
        } else {
            (false, 0, 0, 0, 1, 0, 0, String::new(), String::new())
        };

    // Conquest (v66+) or Sabuk (v65 and below)
    let conquest = if version >= 66 {
        reader.read_raw_i32()?
    } else {
        reader.read_boolean()? as i32
    };

    let flag_needed = reader.read_raw_i32()?;

    // ShowOnBigMap + BigMapIcon: v96+ (version > 95)
    let (show_on_big_map, big_map_icon) = if version > 95 {
        (reader.read_boolean()?, reader.read_raw_i32()?)
    } else {
        (false, 0)
    };

    // CanTeleportTo: v97+ (version > 96)
    let can_teleport_to = if version > 96 { reader.read_boolean()? } else { false };

    // ConquestVisible: v107+
    let conquest_visible = if version >= 107 { reader.read_boolean()? } else { true };

    Ok(ParsedNPCInfo {
        index, map_index, collect_quest_indexes, finish_quest_indexes,
        file_name, name, x, y, image, rate,
        time_visible, hour_start, minute_start, hour_end, minute_end,
        min_lev, max_lev, day_of_week, class_required, conquest, flag_needed,
        show_on_big_map, big_map_icon, can_teleport_to, conquest_visible,
    })
}

fn read_quest_info<R: Read>(reader: &mut BinaryReader<R>, version: i32) -> std::io::Result<ParsedQuestInfo> {
    let index = reader.read_raw_i32()?;
    let name = reader.read_string()?;
    let group_name = reader.read_string()?;
    let file_name = reader.read_string()?;
    let required_min_level = reader.read_raw_i32()?;
    let required_max_level = reader.read_raw_i32()?;
    let required_quest = reader.read_raw_i32()?;
    let required_class = reader.read_raw_u8()?;
    let quest_type = reader.read_raw_u8()?;
    let goto_message = reader.read_string()?;
    let kill_message = reader.read_string()?;
    let item_message = reader.read_string()?;
    let flag_message = reader.read_string()?;

    // TimeLimitInSeconds: v91+ (version > 90)
    let time_limit_seconds = if version > 90 { reader.read_raw_i32()? } else { 0 };

    Ok(ParsedQuestInfo {
        index, name, group_name, file_name,
        required_min_level, required_max_level, required_quest,
        required_class, quest_type,
        // quest rewards (exp/gold) 不在 Server.MirDB binary;这些在 master
        // C# 端也是从 quest .txt 文件 [@FIXEDREWARDS] / 完成的 NPC script
        // (GIVEEXP/GIVEGOLD 命令) 里取。
        // 我们的实现 (db::resolve_quest_tasks in mod.rs:2301) 解析
        //  的 [@KILLTASKS] / [@ITEMTASKS]
        // / [@FLAGTASKS] / [@FIXEDREWARDS] / [@SELECTREWARDS] section;
        // 然后 QuestProgress.exp_reward / gold_reward 字段在 runtime 由
        // quest_complete_actor::complete_quest() 触发 GIVEEXP/GIVEGOLD。
        // 所以这 2 个字段在 ParsedQuestInfo 上**未用**(留给向后兼容)
        exp_reward: 0,
        gold_reward: 0,
        goto_message, kill_message, item_message, flag_message,
        time_limit_seconds,
    })
}

fn read_dragon_info<R: Read>(reader: &mut BinaryReader<R>, _version: i32) -> std::io::Result<ParsedDragonInfo> {
    let enabled = reader.read_boolean()?;
    let map_file_name = reader.read_string()?;
    let monster_name = reader.read_string()?;
    let body_name = reader.read_string()?;
    let location_x = reader.read_raw_i32()?;
    let location_y = reader.read_raw_i32()?;
    let drop_area_top_x = reader.read_raw_i32()?;
    let drop_area_top_y = reader.read_raw_i32()?;
    let drop_area_bottom_x = reader.read_raw_i32()?;
    let drop_area_bottom_y = reader.read_raw_i32()?;

    // Exps: fixed 12 entries (MaxDragonLevel - 1 = 13 - 1 = 12)
    let mut exps = Vec::new();
    for _ in 0..12 {
        exps.push(reader.read_raw_i64()?);
    }
    let exps_json = serde_json::to_string(&exps).unwrap_or_default();

    Ok(ParsedDragonInfo {
        enabled, map_file_name, monster_name, body_name,
        location_x, location_y,
        drop_area_top_x, drop_area_top_y,
        drop_area_bottom_x, drop_area_bottom_y,
        exps_json,
    })
}

fn read_magic_info<R: Read>(reader: &mut BinaryReader<R>, version: i32) -> std::io::Result<ParsedMagicInfo> {
    let name = reader.read_string()?;
    let spell = reader.read_raw_u8()?;
    let base_cost = reader.read_raw_u8()?;
    let level_cost = reader.read_raw_u8()?;
    let icon = reader.read_raw_u8()?;
    let level1 = reader.read_raw_u8()?;
    let level2 = reader.read_raw_u8()?;
    let level3 = reader.read_raw_u8()?;
    let need1 = reader.read_raw_u16()?;
    let need2 = reader.read_raw_u16()?;
    let need3 = reader.read_raw_u16()?;
    let delay_base = reader.read_raw_u32()?;
    let delay_reduction = reader.read_raw_u32()?;
    let power_base = reader.read_raw_u16()?;
    let power_bonus = reader.read_raw_u16()?;
    let mpower_base = reader.read_raw_u16()?;
    let mpower_bonus = reader.read_raw_u16()?;

    // Range: v67+ (version > 66); Multiplier: v71+ (version > 70)
    let range = if version > 66 { reader.read_raw_u8()? } else { 9 };
    let (multiplier_base, multiplier_bonus) = if version > 70 {
        (reader.read_raw_f32()?, reader.read_raw_f32()?)
    } else {
        (1.0, 0.0)
    };

    Ok(ParsedMagicInfo {
        name, spell, base_cost, level_cost, icon, level1, level2, level3,
        need1, need2, need3, delay_base, delay_reduction,
        power_base, power_bonus, mpower_base, mpower_bonus,
        range, multiplier_base, multiplier_bonus,
    })
}

fn read_game_shop_item<R: Read>(reader: &mut BinaryReader<R>, version: i32) -> std::io::Result<ParsedGameShopItem> {
    let item_index = reader.read_raw_i32()?;
    let gindex = reader.read_raw_i32()?;
    let gold_price = reader.read_raw_u32()?;
    let credit_price = reader.read_raw_u32()?;
    // v84-: Count is UInt32; v85+: UInt16
    let count = if version <= 84 {
        reader.read_raw_u32()? as u16
    } else {
        reader.read_raw_u16()?
    };
    let class_name = reader.read_string()?;
    let category = reader.read_string()?;
    let stock = reader.read_raw_i32()?;
    let infinite_stock = reader.read_boolean()?;
    let deal = reader.read_boolean()?;
    let top_item = reader.read_boolean()?;
    let date = reader.read_raw_i64()?;

    // CanBuyCredit/CanBuyGold: v106+ (version > 105)
    let (can_buy_credit, can_buy_gold) = if version > 105 {
        (reader.read_boolean()?, reader.read_boolean()?)
    } else {
        (false, false)
    };

    Ok(ParsedGameShopItem {
        item_index, gindex, gold_price, credit_price, count,
        class_name, category, stock, infinite_stock, deal, top_item, date,
        can_buy_credit, can_buy_gold,
    })
}

fn read_conquest_archer<R: Read>(reader: &mut BinaryReader<R>) -> std::io::Result<(i32, i32, i32, i32, String, u32)> {
    let index = reader.read_raw_i32()?;
    let x = reader.read_raw_i32()?;
    let y = reader.read_raw_i32()?;
    let mob_index = reader.read_raw_i32()?;
    let name = reader.read_string()?;
    let repair_cost = reader.read_raw_u32()?;
    Ok((index, x, y, mob_index, name, repair_cost))
}

fn read_conquest_gate<R: Read>(reader: &mut BinaryReader<R>, _version: i32) -> std::io::Result<(i32, i32, i32, i32, String, i32)> {
    let index = reader.read_raw_i32()?;
    let x = reader.read_raw_i32()?;
    let y = reader.read_raw_i32()?;
    let mob_index = reader.read_raw_i32()?;
    let name = reader.read_string()?;
    // Save always writes RepairCost as Int32
    let repair_cost = reader.read_raw_i32()?;
    Ok((index, x, y, mob_index, name, repair_cost))
}

fn read_conquest_flag<R: Read>(reader: &mut BinaryReader<R>) -> std::io::Result<(i32, i32, i32, String, String)> {
    let index = reader.read_raw_i32()?;
    let x = reader.read_raw_i32()?;
    let y = reader.read_raw_i32()?;
    let name = reader.read_string()?;
    let file_name = reader.read_string()?;
    Ok((index, x, y, name, file_name))
}

fn read_conquest_info<R: Read>(reader: &mut BinaryReader<R>, _version: i32) -> std::io::Result<ParsedConquestInfo> {
    let index = reader.read_raw_i32()?;

    // Save always writes FullMap
    let full_map = reader.read_boolean()?;

    let location_x = reader.read_raw_i32()?;
    let location_y = reader.read_raw_i32()?;
    let size = reader.read_raw_u16()?;
    let name = reader.read_string()?;
    let map_index = reader.read_raw_i32()?;
    let palace_index = reader.read_raw_i32()?;
    let guard_index = reader.read_raw_i32()?;
    let gate_index = reader.read_raw_i32()?;
    let wall_index = reader.read_raw_i32()?;
    let siege_index = reader.read_raw_i32()?;

    // Save always writes FlagIndex
    let flag_index = reader.read_raw_i32()?;

    // ConquestGuards
    let cg_count = reader.read_raw_i32()?;
    let mut guards = Vec::new();
    for _ in 0..cg_count {
        guards.push(read_conquest_archer(reader)?);
    }

    // ExtraMaps
    let em_count = reader.read_raw_i32()?;
    let mut extra_maps = Vec::new();
    for _ in 0..em_count {
        extra_maps.push(reader.read_raw_i32()?);
    }
    let extra_maps_json = serde_json::to_string(&extra_maps).unwrap_or_default();

    // ConquestGates
    let cg2_count = reader.read_raw_i32()?;
    let mut gates = Vec::new();
    for _ in 0..cg2_count {
        gates.push(read_conquest_gate(reader, 0)?);
    }

    // ConquestWalls
    let cw_count = reader.read_raw_i32()?;
    let mut walls = Vec::new();
    for _ in 0..cw_count {
        walls.push(read_conquest_gate(reader, 0)?);
    }

    // ConquestSieges
    let cs_count = reader.read_raw_i32()?;
    let mut sieges = Vec::new();
    for _ in 0..cs_count {
        sieges.push(read_conquest_gate(reader, 0)?);
    }

    // ConquestFlags - Save always writes this
    let cf_count = reader.read_raw_i32()?;
    let mut conquest_flags = Vec::new();
    for _ in 0..cf_count {
        conquest_flags.push(read_conquest_flag(reader)?);
    }

    let start_hour = reader.read_raw_u8()?;
    let war_length = reader.read_raw_i32()?;
    let conquest_type = reader.read_raw_u8()?;
    let conquest_game = reader.read_raw_u8()?;

    let mut days = [false; 7];
    days[0] = reader.read_boolean()?; // Monday
    days[1] = reader.read_boolean()?; // Tuesday
    days[2] = reader.read_boolean()?; // Wednesday
    days[3] = reader.read_boolean()?; // Thursday
    days[4] = reader.read_boolean()?; // Friday
    days[5] = reader.read_boolean()?; // Saturday
    days[6] = reader.read_boolean()?; // Sunday

    let king_x = reader.read_raw_i32()?;
    let king_y = reader.read_raw_i32()?;
    let king_size = reader.read_raw_u16()?;

    // Save always writes ControlPointIndex and ControlPoints
    let control_point_index = reader.read_raw_i32()?;

    let cp_count = reader.read_raw_i32()?;
    for _ in 0..cp_count {
        read_conquest_flag(reader)?; // consume
    }

    Ok(ParsedConquestInfo {
        index, full_map, location_x, location_y, size, name,
        map_index, palace_index, guard_index, gate_index, wall_index, siege_index,
        flag_index, extra_maps_json,
        start_hour, war_length, conquest_type, conquest_game, days,
        king_x, king_y, king_size, control_point_index,
    })
}

fn read_gt_map<R: Read>(reader: &mut BinaryReader<R>, _version: i32) -> std::io::Result<ParsedGTMap> {
    let index = reader.read_raw_i32()?;
    let key = reader.read_raw_i32()?;
    let name = reader.read_string()?;
    let owner = reader.read_string()?;
    let leader = reader.read_string()?;
    let leader2 = reader.read_string()?;
    let price = reader.read_raw_i32()?;
    let days = reader.read_raw_i32()?;
    let begin_time = reader.read_raw_i32()?;
    Ok(ParsedGTMap { index, key, name, owner, leader, leader2, price, days, begin_time })
}

// ============================================================
// DB insertion
// ============================================================

async fn create_tables(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
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
            music INTEGER NOT NULL DEFAULT 0,
            no_town_teleport INTEGER NOT NULL DEFAULT 0,
            no_reincarnation INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS safe_zones (
            map_index INTEGER NOT NULL,
            x INTEGER NOT NULL,
            y INTEGER NOT NULL,
            size INTEGER NOT NULL DEFAULT 0,
            start_point INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (map_index, x, y)
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
            respawn_ticks INTEGER NOT NULL DEFAULT 0
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
            icon INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS mine_zones (
            map_index INTEGER NOT NULL,
            x INTEGER NOT NULL,
            y INTEGER NOT NULL,
            size INTEGER NOT NULL DEFAULT 0,
            mine INTEGER NOT NULL DEFAULT 0
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
        CREATE TABLE IF NOT EXISTS gt_maps (
            idx INTEGER PRIMARY KEY,
            key INTEGER NOT NULL DEFAULT 0,
            name TEXT NOT NULL,
            owner TEXT,
            leader TEXT,
            leader2 TEXT,
            price INTEGER NOT NULL DEFAULT 0,
            days INTEGER NOT NULL DEFAULT 0,
            begin_time INTEGER NOT NULL DEFAULT 0
        );
        "#
    ).execute(pool).await?;
    Ok(())
}

async fn insert_map_info(pool: &sqlx::SqlitePool, map: &ParsedMapInfo) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT OR REPLACE INTO map_infos (
            idx, file_name, title, mini_map, light, big_map,
            no_teleport, no_reconnect, no_reconnect_map, no_random, no_escape,
            no_recall, no_drug, no_position, no_throw_item, no_drop_player,
            no_drop_monster, no_names, fight, fire, fire_damage, lightning,
            lightning_damage, map_dark_light, mine_index, no_mount, need_bridle,
            no_fight, music, no_town_teleport, no_reincarnation
        ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#
    )
    .bind(map.index).bind(&map.file_name).bind(&map.title)
    .bind(map.mini_map as i32).bind(map.light as i32).bind(map.big_map as i32)
    .bind(if map.no_teleport { 1 } else { 0 })
    .bind(if map.no_reconnect { 1 } else { 0 })
    .bind(&map.no_reconnect_map)
    .bind(if map.no_random { 1 } else { 0 })
    .bind(if map.no_escape { 1 } else { 0 })
    .bind(if map.no_recall { 1 } else { 0 })
    .bind(if map.no_drug { 1 } else { 0 })
    .bind(if map.no_position { 1 } else { 0 })
    .bind(if map.no_throw_item { 1 } else { 0 })
    .bind(if map.no_drop_player { 1 } else { 0 })
    .bind(if map.no_drop_monster { 1 } else { 0 })
    .bind(if map.no_names { 1 } else { 0 })
    .bind(if map.fight { 1 } else { 0 })
    .bind(if map.fire { 1 } else { 0 })
    .bind(map.fire_damage)
    .bind(if map.lightning { 1 } else { 0 })
    .bind(map.lightning_damage)
    .bind(map.map_dark_light as i32)
    .bind(map.mine_index as i32)
    .bind(if map.no_mount { 1 } else { 0 })
    .bind(if map.need_bridle { 1 } else { 0 })
    .bind(if map.no_fight { 1 } else { 0 })
    .bind(map.music as i32)
    .bind(if map.no_town_teleport { 1 } else { 0 })
    .bind(if map.no_reincarnation { 1 } else { 0 })
    .execute(pool).await?;

    for sz in &map.safe_zones {
        sqlx::query(
            "INSERT OR REPLACE INTO safe_zones (map_index, x, y, size, start_point) VALUES (?,?,?,?,?)"
        )
        .bind(map.index).bind(sz.x).bind(sz.y).bind(sz.size as i32)
        .bind(if sz.start_point { 1 } else { 0 })
        .execute(pool).await?;
    }

    for rs in &map.respawns {
        sqlx::query(
            r#"INSERT INTO map_respawns (
                map_index, monster_index, x, y, count, spread, delay, direction,
                route_path, random_delay, respawn_index, save_respawn_time, respawn_ticks
            ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)"#
        )
        .bind(map.index).bind(rs.monster_index).bind(rs.x).bind(rs.y)
        .bind(rs.count as i32).bind(rs.spread as i32).bind(rs.delay as i32)
        .bind(rs.direction as i32).bind(&rs.route_path)
        .bind(rs.random_delay as i32).bind(rs.respawn_index)
        .bind(if rs.save_respawn_time { 1 } else { 0 })
        .bind(rs.respawn_ticks as i32)
        .execute(pool).await?;
    }

    for mv in &map.movements {
        sqlx::query(
            r#"INSERT INTO map_movements (
                map_index, source_x, source_y, dest_x, dest_y,
                need_hole, need_move, conquest_index, show_on_big_map, icon
            ) VALUES (?,?,?,?,?,?,?,?,?,?)"#
        )
        .bind(mv.map_index).bind(mv.source_x).bind(mv.source_y)
        .bind(mv.dest_x).bind(mv.dest_y)
        .bind(if mv.need_hole { 1 } else { 0 })
        .bind(if mv.need_move { 1 } else { 0 })
        .bind(mv.conquest_index)
        .bind(if mv.show_on_big_map { 1 } else { 0 })
        .bind(mv.icon)
        .execute(pool).await?;
    }

    for mz in &map.mine_zones {
        sqlx::query(
            "INSERT INTO mine_zones (map_index, x, y, size, mine) VALUES (?,?,?,?,?)"
        )
        .bind(map.index).bind(mz.x).bind(mz.y).bind(mz.size as i32).bind(mz.mine as i32)
        .execute(pool).await?;
    }

    Ok(())
}

async fn insert_item_info(pool: &sqlx::SqlitePool, item: &ParsedItemInfo) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT OR REPLACE INTO item_infos (
            idx, name, type, grade, required_type, required_class, required_gender,
            set_type, shape, weight, light, required_amount, image, durability,
            stack_size, price, start_item, effect, bool_flags, bind_mode, special_mode,
            random_stats_id, can_fast_run, can_awakening, slots, stats_json,
            has_tool_tip, tool_tip
        ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#
    )
    .bind(item.index).bind(&item.name).bind(item.type_byte as i32).bind(item.grade as i32)
    .bind(item.required_type as i32).bind(item.required_class as i32).bind(item.required_gender as i32)
    .bind(item.set_type as i32).bind(item.shape as i32).bind(item.weight as i32)
    .bind(item.light as i32).bind(item.required_amount as i32).bind(item.image as i32)
    .bind(item.durability as i32).bind(item.stack_size as i32).bind(item.price as i64)
    .bind(if item.start_item { 1 } else { 0 }).bind(item.effect as i32)
    .bind(item.bool_flags as i32).bind(item.bind_mode as i32).bind(item.special_mode as i32)
    .bind(item.random_stats_id as i32)
    .bind(if item.can_fast_run { 1 } else { 0 })
    .bind(if item.can_awakening { 1 } else { 0 })
    .bind(item.slots as i32).bind(&item.stats_json)
    .bind(if item.has_tool_tip { 1 } else { 0 }).bind(&item.tool_tip)
    .execute(pool).await?;
    Ok(())
}

async fn insert_monster_info(pool: &sqlx::SqlitePool, m: &ParsedMonsterInfo) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT OR REPLACE INTO monster_infos (
            idx, name, image, ai, effect, level, view_range, cool_eye, stats_json,
            light, attack_speed, move_speed, experience, can_push, can_tame,
            auto_rev, undead, drop_path
        ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#
    )
    .bind(m.index).bind(&m.name).bind(m.image as i32).bind(m.ai as i32)
    .bind(m.effect as i32).bind(m.level as i32).bind(m.view_range as i32)
    .bind(m.cool_eye as i32).bind(&m.stats_json).bind(m.light as i32)
    .bind(m.attack_speed as i32).bind(m.move_speed as i32).bind(m.experience as i64)
    .bind(if m.can_push { 1 } else { 0 }).bind(if m.can_tame { 1 } else { 0 })
    .bind(if m.auto_rev { 1 } else { 0 }).bind(if m.undead { 1 } else { 0 })
    .bind(&m.drop_path)
    .execute(pool).await?;
    Ok(())
}

async fn insert_npc_info(pool: &sqlx::SqlitePool, npc: &ParsedNPCInfo) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT OR REPLACE INTO npc_infos (
            idx, map_index, file_name, name, x, y, image, rate,
            time_visible, hour_start, minute_start, hour_end, minute_end,
            min_lev, max_lev, day_of_week, class_required, conquest, flag_needed,
            show_on_big_map, big_map_icon, can_teleport_to, conquest_visible,
            collect_quest_indexes, finish_quest_indexes
        ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#
    )
    .bind(npc.index).bind(npc.map_index).bind(&npc.file_name).bind(&npc.name)
    .bind(npc.x).bind(npc.y).bind(npc.image as i32).bind(npc.rate as i32)
    .bind(if npc.time_visible { 1 } else { 0 })
    .bind(npc.hour_start as i32).bind(npc.minute_start as i32)
    .bind(npc.hour_end as i32).bind(npc.minute_end as i32)
    .bind(npc.min_lev as i32).bind(npc.max_lev as i32)
    .bind(&npc.day_of_week).bind(&npc.class_required)
    .bind(npc.conquest).bind(npc.flag_needed)
    .bind(if npc.show_on_big_map { 1 } else { 0 })
    .bind(npc.big_map_icon)
    .bind(if npc.can_teleport_to { 1 } else { 0 })
    .bind(if npc.conquest_visible { 1 } else { 0 })
    .bind(&npc.collect_quest_indexes)
    .bind(&npc.finish_quest_indexes)
    .execute(pool).await?;
    Ok(())
}

async fn insert_quest_info(pool: &sqlx::SqlitePool, q: &ParsedQuestInfo) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT OR REPLACE INTO quest_infos (
            idx, name, group_name, file_name,
            required_min_level, required_max_level, required_quest,
            required_class, quest_type, exp_reward, gold_reward,
            goto_message, kill_message, item_message, flag_message,
            time_limit_seconds
        ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#
    )
    .bind(q.index).bind(&q.name).bind(&q.group_name).bind(&q.file_name)
    .bind(q.required_min_level).bind(q.required_max_level).bind(q.required_quest)
    .bind(q.required_class as i32).bind(q.quest_type as i32)
    .bind(q.exp_reward).bind(q.gold_reward)
    .bind(&q.goto_message).bind(&q.kill_message).bind(&q.item_message).bind(&q.flag_message)
    .bind(q.time_limit_seconds)
    .execute(pool).await?;
    Ok(())
}

async fn insert_dragon_info(pool: &sqlx::SqlitePool, d: &ParsedDragonInfo) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT OR REPLACE INTO dragon_info (
            id, enabled, map_file_name, monster_name, body_name,
            location_x, location_y, drop_area_top_x, drop_area_top_y,
            drop_area_bottom_x, drop_area_bottom_y, exps_json
        ) VALUES (1,?,?,?,?,?,?,?,?,?,?,?)"#
    )
    .bind(if d.enabled { 1 } else { 0 })
    .bind(&d.map_file_name).bind(&d.monster_name).bind(&d.body_name)
    .bind(d.location_x).bind(d.location_y)
    .bind(d.drop_area_top_x).bind(d.drop_area_top_y)
    .bind(d.drop_area_bottom_x).bind(d.drop_area_bottom_y)
    .bind(&d.exps_json)
    .execute(pool).await?;
    Ok(())
}

async fn insert_magic_info(pool: &sqlx::SqlitePool, m: &ParsedMagicInfo) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT OR REPLACE INTO magic_infos (
            name, spell, base_cost, level_cost, icon, level1, level2, level3,
            need1, need2, need3, delay_base, delay_reduction,
            power_base, power_bonus, mpower_base, mpower_bonus,
            range, multiplier_base, multiplier_bonus
        ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#
    )
    .bind(&m.name).bind(m.spell as i32).bind(m.base_cost as i32).bind(m.level_cost as i32)
    .bind(m.icon as i32).bind(m.level1 as i32).bind(m.level2 as i32).bind(m.level3 as i32)
    .bind(m.need1 as i32).bind(m.need2 as i32).bind(m.need3 as i32)
    .bind(m.delay_base as i64).bind(m.delay_reduction as i64)
    .bind(m.power_base as i32).bind(m.power_bonus as i32)
    .bind(m.mpower_base as i32).bind(m.mpower_bonus as i32)
    .bind(m.range as i32).bind(m.multiplier_base).bind(m.multiplier_bonus)
    .execute(pool).await?;
    Ok(())
}

async fn insert_game_shop_item(pool: &sqlx::SqlitePool, g: &ParsedGameShopItem) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT OR REPLACE INTO game_shop_items (
            item_index, gindex, gold_price, credit_price, count,
            class, category, stock, infinite_stock, deal, top_item, date,
            can_buy_credit, can_buy_gold
        ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#
    )
    .bind(g.item_index).bind(g.gindex).bind(g.gold_price as i64).bind(g.credit_price as i64)
    .bind(g.count as i32).bind(&g.class_name).bind(&g.category).bind(g.stock)
    .bind(if g.infinite_stock { 1 } else { 0 })
    .bind(if g.deal { 1 } else { 0 })
    .bind(if g.top_item { 1 } else { 0 })
    .bind(g.date)
    .bind(if g.can_buy_credit { 1 } else { 0 })
    .bind(if g.can_buy_gold { 1 } else { 0 })
    .execute(pool).await?;
    Ok(())
}

async fn insert_conquest_info(pool: &sqlx::SqlitePool, c: &ParsedConquestInfo) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT OR REPLACE INTO conquest_infos (
            idx, full_map, location_x, location_y, size, name,
            map_index, palace_index, guard_index, gate_index, wall_index, siege_index,
            flag_index, extra_maps_json,
            start_hour, war_length, conquest_type, conquest_game,
            monday, tuesday, wednesday, thursday, friday, saturday, sunday,
            king_x, king_y, king_size, control_point_index
        ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#
    )
    .bind(c.index).bind(if c.full_map { 1 } else { 0 })
    .bind(c.location_x).bind(c.location_y).bind(c.size as i32).bind(&c.name)
    .bind(c.map_index).bind(c.palace_index).bind(c.guard_index)
    .bind(c.gate_index).bind(c.wall_index).bind(c.siege_index)
    .bind(c.flag_index).bind(&c.extra_maps_json)
    .bind(c.start_hour as i32).bind(c.war_length)
    .bind(c.conquest_type as i32).bind(c.conquest_game as i32)
    .bind(if c.days[0] { 1 } else { 0 }).bind(if c.days[1] { 1 } else { 0 })
    .bind(if c.days[2] { 1 } else { 0 }).bind(if c.days[3] { 1 } else { 0 })
    .bind(if c.days[4] { 1 } else { 0 }).bind(if c.days[5] { 1 } else { 0 })
    .bind(if c.days[6] { 1 } else { 0 })
    .bind(c.king_x).bind(c.king_y).bind(c.king_size as i32)
    .bind(c.control_point_index)
    .execute(pool).await?;
    Ok(())
}

async fn insert_gt_map(pool: &sqlx::SqlitePool, g: &ParsedGTMap) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT OR REPLACE INTO gt_maps (
            idx, key, name, owner, leader, leader2, price, days, begin_time
        ) VALUES (?,?,?,?,?,?,?,?,?)"#
    )
    .bind(g.index).bind(g.key).bind(&g.name).bind(&g.owner)
    .bind(&g.leader).bind(&g.leader2).bind(g.price).bind(g.days).bind(g.begin_time)
    .execute(pool).await?;
    Ok(())
}

// ============================================================
// Main
// ============================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: migrate_mirdb <path-to-Server.MirDB> [sqlite-db-path]");
        eprintln!("  Default sqlite path: data/crystal.db");
        std::process::exit(1);
    }

    let db_path = &args[1];
    let sqlite_path = args.get(2).map(|s| s.as_str()).unwrap_or("data/crystal.db");

    info!("=== Server.MirDB to SQLite Migration Tool ===");
    info!("Source: {}", db_path);
    info!("Target: {}", sqlite_path);

    let data = std::fs::read(db_path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", db_path, e))?;
    let file_size = data.len();
    info!("File size: {} bytes", file_size);

    let mut reader = BinaryReader::new(std::io::Cursor::new(data));

    // Parse header (10 Int32 fields)
    let version = reader.read_raw_i32()?;
    let custom_version = reader.read_raw_i32()?;
    let map_index = reader.read_raw_i32()?;
    let item_index = reader.read_raw_i32()?;
    let monster_index = reader.read_raw_i32()?;
    let npc_index = reader.read_raw_i32()?;
    let quest_index = reader.read_raw_i32()?;
    let gameshop_index = if version >= 63 { reader.read_raw_i32()? } else { 0 };
    let conquest_index = if version >= 66 { reader.read_raw_i32()? } else { 0 };
    let respawn_index = if version > 68 { reader.read_raw_i32()? } else { 0 };

    info!("Version: {}.{}", version, custom_version);
    info!("Next IDs: map={}, item={}, monster={}, npc={}, quest={}, gameshop={}, conquest={}, respawn={}",
        map_index, item_index, monster_index, npc_index, quest_index, gameshop_index, conquest_index, respawn_index);

    // Initialize database
    // Convert to absolute path for SQLite URL
    let abs_path = if std::path::Path::new(sqlite_path).is_absolute() {
        sqlite_path.to_string()
    } else {
        let cwd = std::env::current_dir().map_err(|e| anyhow::anyhow!("Failed to get current dir: {}", e))?;
        cwd.join(sqlite_path).to_string_lossy().to_string()
    };
    let normalized = abs_path.replace('\\', "/");
    let db_url = format!("sqlite://{}", normalized);

    // Ensure parent directory and file exist (sqlx won't create them)
    if let Some(parent) = std::path::Path::new(&abs_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| anyhow::anyhow!("Failed to create directory {}: {}", parent.display(), e))?;
    }
    std::fs::OpenOptions::new().create(true).truncate(true).write(true).open(&abs_path)
        .map_err(|e| anyhow::anyhow!("Failed to create DB file {}: {}", abs_path, e))?;

    let pool = sqlx::SqlitePool::connect(&db_url).await
        .map_err(|e| anyhow::anyhow!("Failed to connect to {}: {}", sqlite_path, e))?;

    info!("Creating tables...");
    create_tables(&pool).await?;

    // === MapInfo[] ===
    let map_count = reader.read_raw_i32()?;
    info!("Migrating {} maps...", map_count);
    let mut map_ok = 0;
    for i in 0..map_count {
        match read_map_info(&mut reader, version) {
            Ok(map) => {
                if let Err(e) = insert_map_info(&pool, &map).await {
                    error!("  Map #{} failed: {}", i, e);
                } else {
                    map_ok += 1;
                }
            }
            Err(e) => error!("  Map #{} parse error: {}", i, e),
        }
    }
    info!("  Maps migrated: {}", map_ok);

    // === ItemInfo[] ===
    let item_count = reader.read_raw_i32()?;
    info!("Migrating {} items...", item_count);
    let mut item_ok = 0;
    for i in 0..item_count {
        match read_item_info(&mut reader, version) {
            Ok(item) => {
                if let Err(e) = insert_item_info(&pool, &item).await {
                    error!("  Item #{} failed: {}", i, e);
                } else {
                    item_ok += 1;
                }
            }
            Err(e) => error!("  Item #{} parse error: {}", i, e),
        }
    }
    info!("  Items migrated: {}", item_ok);

    // === MonsterInfo[] ===
    let monster_count = reader.read_raw_i32()?;
    info!("Migrating {} monsters...", monster_count);
    let mut monster_ok = 0;
    for i in 0..monster_count {
        match read_monster_info(&mut reader, version) {
            Ok(m) => {
                if let Err(e) = insert_monster_info(&pool, &m).await {
                    error!("  Monster #{} failed: {}", i, e);
                } else {
                    monster_ok += 1;
                }
            }
            Err(e) => error!("  Monster #{} parse error: {}", i, e),
        }
    }
    info!("  Monsters migrated: {}", monster_ok);

    // === NPCInfo[] ===
    let npc_count = reader.read_raw_i32()?;
    info!("Migrating {} NPCs...", npc_count);
    let mut npc_ok = 0;
    for i in 0..npc_count {
        match read_npc_info(&mut reader, version) {
            Ok(npc) => {
                if let Err(e) = insert_npc_info(&pool, &npc).await {
                    error!("  NPC #{} failed: {}", i, e);
                } else {
                    npc_ok += 1;
                }
            }
            Err(e) => error!("  NPC #{} parse error: {}", i, e),
        }
    }
    info!("  NPCs migrated: {}", npc_ok);

    // === QuestInfo[] ===
    let quest_count = reader.read_raw_i32()?;
    info!("Migrating {} quests...", quest_count);
    let mut quest_ok = 0;
    for i in 0..quest_count {
        match read_quest_info(&mut reader, version) {
            Ok(q) => {
                if let Err(e) = insert_quest_info(&pool, &q).await {
                    error!("  Quest #{} failed: {}", i, e);
                } else {
                    quest_ok += 1;
                }
            }
            Err(e) => error!("  Quest #{} parse error: {}", i, e),
        }
    }
    info!("  Quests migrated: {}", quest_ok);

    // === DragonInfo (single) ===
    info!("Migrating DragonInfo...");
    match read_dragon_info(&mut reader, version) {
        Ok(d) => {
            if let Err(e) = insert_dragon_info(&pool, &d).await {
                error!("  Dragon failed: {}", e);
            } else {
                info!("  Dragon migrated");
            }
        }
        Err(e) => error!("  Dragon parse error: {}", e),
    }

    // === MagicInfo[] ===
    let magic_count = reader.read_raw_i32()?;
    info!("Migrating {} magics...", magic_count);
    let mut magic_ok = 0;
    for i in 0..magic_count {
        match read_magic_info(&mut reader, version) {
            Ok(m) => {
                if let Err(e) = insert_magic_info(&pool, &m).await {
                    error!("  Magic #{} failed: {}", i, e);
                } else {
                    magic_ok += 1;
                }
            }
            Err(e) => error!("  Magic #{} parse error: {}", i, e),
        }
    }
    info!("  Magics migrated: {}", magic_ok);

    // === GameShopItem[] (v63+) ===
    if version >= 63 {
        let shop_count = reader.read_raw_i32()?;
        if shop_count > 100_000 {
            warn!("GameShop count ({}) seems invalid, skipping", shop_count);
        } else {
            info!("Migrating {} game shop items...", shop_count);
            let mut shop_ok = 0;
            for i in 0..shop_count {
                match read_game_shop_item(&mut reader, version) {
                    Ok(g) => {
                        if let Err(e) = insert_game_shop_item(&pool, &g).await {
                            error!("  Shop item #{} failed: {}", i, e);
                        } else {
                            shop_ok += 1;
                        }
                    }
                    Err(e) => error!("  Shop item #{} parse error: {}", i, e),
                }
            }
            info!("  Shop items migrated: {}", shop_ok);
        }
    }

    // === ConquestInfo[] (v66+) ===
    if version >= 66 {
        let conquest_count = reader.read_raw_i32()?;
        if conquest_count > 100_000 {
            warn!("Conquest count ({}) seems invalid, skipping", conquest_count);
        } else {
            info!("Migrating {} conquests...", conquest_count);
            let mut conquest_ok = 0;
            for i in 0..conquest_count {
                match read_conquest_info(&mut reader, version) {
                    Ok(c) => {
                        if let Err(e) = insert_conquest_info(&pool, &c).await {
                            error!("  Conquest #{} failed: {}", i, e);
                        } else {
                            conquest_ok += 1;
                        }
                    }
                    Err(e) => error!("  Conquest #{} parse error: {}", i, e),
                }
            }
            info!("  Conquests migrated: {}", conquest_ok);
        }
    }

    // === RespawnTick (v68+) ===
    if version > 68 {
        // RespawnTick is runtime state, just skip it
        // It contains BaseSpawnRate, CurrentTickcounter, and Respawn options
        // We skip: 1 byte + 8 bytes + (count * (4 bytes + 8 bytes))
        reader.read_raw_u8()?; // BaseSpawnRate
        reader.read_raw_u64()?; // CurrentTickcounter
        let rt_count = reader.read_raw_i32()?;
        for _ in 0..rt_count {
            reader.read_raw_i32()?; // UserCount
            reader.read_raw_f64()?; // DelayLoss (f64)
        }
        info!("Skipped RespawnTick ({} entries) (position: {})", rt_count, reader.position());
    }

    // === GTMap[] (v111+) ===
    // GTMap feature was added at DB version 111
    let _gt_ok = if version >= 111 {
        let gt_count = reader.read_raw_i32()?;
        info!("Migrating {} GT maps... (position: {})", gt_count, reader.position());
        let mut gt_ok = 0;
        for i in 0..gt_count {
            match read_gt_map(&mut reader, version) {
                Ok(g) => {
                    if let Err(e) = insert_gt_map(&pool, &g).await {
                        error!("  GT map #{} failed: {}", i, e);
                    } else {
                        gt_ok += 1;
                    }
                }
                Err(e) => error!("  GT map #{} parse error: {}", i, e),
            }
        }
        info!("  GT maps migrated: {}", gt_ok);
        gt_ok
    } else {
        info!("Skipped GTMap (version {} < 111)", version);
        0
    };

    // === Summary ===
    info!("=== Migration Complete ===");
    info!("Maps: {}", map_ok);
    info!("Items: {}", item_ok);
    info!("Monsters: {}", monster_ok);
    info!("NPCs: {}", npc_ok);
    info!("Quests: {}", quest_ok);
    info!("Magics: {}", magic_ok);
    info!("Database: {}", sqlite_path);

    // Verify counts
    let tables = ["map_infos", "safe_zones", "map_respawns", "map_movements", "mine_zones",
                   "item_infos", "monster_infos", "npc_infos", "quest_infos", "dragon_info",
                   "magic_infos", "game_shop_items", "conquest_infos", "gt_maps"];
    for t in &tables {
        let count: (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM {}", t))
            .fetch_one(&pool).await.unwrap_or((0,));
        info!("  {}: {}", t, count.0);
    }

    Ok(())
}
