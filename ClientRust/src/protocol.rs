use std::{convert::TryFrom, io::Cursor};

use byteorder::{LittleEndian, ReadBytesExt};
use mir2_shared::{
    binary::{read_bool, read_dotnet_string},
    client_data::{ClientIntelligentCreature, ClientMagic},
    enums::{
        BuffType, DamageType, HeroBehaviour, IntelligentCreatureType, ItemGrade, LevelEffects,
        MirClass, MirDirection, MirGender, PoisonType, Spell, SpellEffect, WeatherSetting,
    },
    packet::PacketHeader,
    packet_ids::ServerPacketId,
    ClientMapInfo, Point, UserItem, WorldMapSetup,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerMessage {
    Connected,
    ClientVersion {
        result: ClientVersionResult,
    },
    Disconnect {
        reason: u8,
    },
    KeepAlive {
        time: i64,
    },
    Login {
        result: LoginResult,
    },
    LoginBanned {
        reason: String,
        expiry_ticks: i64,
    },
    LoginSuccess {
        characters: Vec<CharacterSummary>,
    },
    Unknown {
        opcode: i16,
        payload: Vec<u8>,
    },
    ParseError {
        opcode: i16,
        message: String,
    },
    StartGame {
        result: StartGameResult,
        resolution: i32,
    },
    StartGameBanned {
        reason: String,
        expiry_ticks: i64,
    },
    StartGameDelay {
        milliseconds: i64,
    },
    MapInformation(MapInformation),
    NewMapInfo(NewMapInfo),
    WorldMapSetup(WorldMapSetupInfo),
    SearchMapResult(SearchMapResult),
    UserInformation(UserInformation),
    UserLocation(UserLocation),
    UserSlotsRefresh(UserSlotsRefresh),
    ObjectPlayer(PlayerObject),
    ObjectHero(HeroObject),
    ObjectRemove(ObjectRemove),
    ObjectTurn(ObjectMotion),
    ObjectWalk(ObjectMotion),
    ObjectRun(ObjectMotion),
    ObjectAttack(ObjectAttack),
    Struck(Struck),
    ObjectStruck(ObjectStruck),
    DamageIndicator(DamageIndicator),
    DuraChanged(DuraChanged),
    DeleteItem(DeleteItem),
    DeleteQuestItem(DeleteQuestItem),
    ObjectItem(ObjectItem),
    ObjectGold(ObjectGold),
    GainedItem(GainedItem),
    GainedGold(GainedGold),
    LoseGold(LoseGold),
    GainedCredit(GainedCredit),
    LoseCredit(LoseCredit),
    GainedQuestItem(GainedQuestItem),
    Death(Death),
    ObjectDied(ObjectDied),
    ColourChanged(ColourChanged),
    ObjectColourChanged(ObjectColourChanged),
    ObjectGuildNameChanged(ObjectGuildNameChanged),
    GainExperience(GainExperience),
    GainHeroExperience(GainHeroExperience),
    LevelChanged(LevelChanged),
    HeroLevelChanged(HeroLevelChanged),
    ObjectLeveled(ObjectLeveled),
    HealthChanged(HealthChanged),
    HeroHealthChanged(HeroHealthChanged),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientVersionResult {
    WrongVersion,
    CorrectVersion,
    Other(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginResult {
    Disabled,
    BadAccountId,
    BadPassword,
    AccountNotExist,
    WrongPassword,
    PasswordMustChange,
    Unknown(u8),
}

impl From<u8> for LoginResult {
    fn from(value: u8) -> Self {
        match value {
            0 => LoginResult::Disabled,
            1 => LoginResult::BadAccountId,
            2 => LoginResult::BadPassword,
            3 => LoginResult::AccountNotExist,
            4 => LoginResult::WrongPassword,
            5 => LoginResult::PasswordMustChange,
            other => LoginResult::Unknown(other),
        }
    }
}

impl From<u8> for ClientVersionResult {
    fn from(value: u8) -> Self {
        match value {
            0 => ClientVersionResult::WrongVersion,
            1 => ClientVersionResult::CorrectVersion,
            other => ClientVersionResult::Other(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartGameResult {
    Disabled,
    NotLoggedIn,
    CharacterNotFound,
    NoStartPoint,
    Success,
    Unknown(u8),
}

impl From<u8> for StartGameResult {
    fn from(value: u8) -> Self {
        match value {
            0 => StartGameResult::Disabled,
            1 => StartGameResult::NotLoggedIn,
            2 => StartGameResult::CharacterNotFound,
            3 => StartGameResult::NoStartPoint,
            4 => StartGameResult::Success,
            other => StartGameResult::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterSummary {
    pub index: i32,
    pub name: String,
    pub level: u16,
    pub class: MirClass,
    pub gender: MirGender,
    pub last_access_ticks: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapInformation {
    pub map_index: i32,
    pub file_name: String,
    pub title: String,
    pub mini_map: u16,
    pub big_map: u16,
    pub lights: u8,
    pub lightning: bool,
    pub fire: bool,
    pub map_dark_light: u8,
    pub music: u16,
    pub weather: WeatherSetting,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMapInfo {
    pub map_index: i32,
    pub info: ClientMapInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMapSetupInfo {
    pub setup: WorldMapSetup,
    pub teleport_to_npc_cost: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMapResult {
    pub map_index: i32,
    pub npc_index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerObject {
    pub object_id: u32,
    pub name: String,
    pub guild_name: String,
    pub guild_rank_name: String,
    pub name_colour_argb: i32,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,
    pub location: Point,
    pub direction: MirDirection,
    pub hair: u8,
    pub light: u8,
    pub weapon: i16,
    pub weapon_effect: i16,
    pub armour: i16,
    pub poison: PoisonType,
    pub dead: bool,
    pub hidden: bool,
    pub effect: SpellEffect,
    pub wing_effect: u8,
    pub extra: bool,
    pub mount_type: i16,
    pub riding_mount: bool,
    pub fishing: bool,
    pub transform_type: i16,
    pub element_orb_effect: u32,
    pub element_orb_level: u32,
    pub element_orb_max: u32,
    pub buffs: Vec<BuffType>,
    pub level_effects: LevelEffects,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeroObject {
    pub player: PlayerObject,
    pub owner_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectRemove {
    pub object_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectMotion {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectAttack {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
    pub spell: Spell,
    pub level: u8,
    pub attack_type: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Struck {
    pub attacker_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectStruck {
    pub object_id: u32,
    pub attacker_id: u32,
    pub location: Point,
    pub direction: MirDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageIndicator {
    pub damage: i32,
    pub damage_type: DamageType,
    pub object_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuraChanged {
    pub unique_id: u64,
    pub current_dura: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectItem {
    pub object_id: u32,
    pub name: String,
    pub name_colour_argb: i32,
    pub location: Point,
    pub image: u16,
    pub grade: ItemGrade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectGold {
    pub object_id: u32,
    pub gold: u32,
    pub location: Point,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GainedItem {
    pub item: UserItem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GainedQuestItem {
    pub item: UserItem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GainedGold {
    pub gold: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoseGold {
    pub gold: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GainedCredit {
    pub credit: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoseCredit {
    pub credit: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteItem {
    pub unique_id: u64,
    pub count: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteQuestItem {
    pub unique_id: u64,
    pub count: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColourChanged {
    pub name_colour_argb: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectColourChanged {
    pub object_id: u32,
    pub name_colour_argb: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectGuildNameChanged {
    pub object_id: u32,
    pub guild_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GainExperience {
    pub amount: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GainHeroExperience {
    pub amount: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelChanged {
    pub level: u16,
    pub experience: i64,
    pub max_experience: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeroLevelChanged {
    pub level: u16,
    pub experience: i64,
    pub max_experience: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectLeveled {
    pub object_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Death {
    pub location: Point,
    pub direction: MirDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectDied {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
    pub death_type: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthChanged {
    pub hp: i32,
    pub mp: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeroHealthChanged {
    pub hp: i32,
    pub mp: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserInformation {
    pub object_id: u32,
    pub real_id: u32,
    pub name: String,
    pub guild_name: String,
    pub guild_rank: String,
    pub name_colour_argb: i32,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,
    pub location: Point,
    pub direction: MirDirection,
    pub hair: u8,
    pub hp: i32,
    pub mp: i32,
    pub experience: i64,
    pub max_experience: i64,
    pub level_effects: LevelEffects,
    pub has_hero: bool,
    pub hero_behaviour: HeroBehaviour,
    pub inventory: Option<Vec<Option<UserItem>>>,
    pub equipment: Option<Vec<Option<UserItem>>>,
    pub quest_inventory: Option<Vec<Option<UserItem>>>,
    pub gold: u32,
    pub credit: u32,
    pub has_expanded_storage: bool,
    pub expanded_storage_expiry_binary: i64,
    pub magics: Vec<ClientMagic>,
    pub intelligent_creatures: Vec<ClientIntelligentCreature>,
    pub summoned_creature_type: IntelligentCreatureType,
    pub creature_summoned: bool,
    pub allow_observe: bool,
    pub observer: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserLocation {
    pub location: Point,
    pub direction: MirDirection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSlotsRefresh {
    pub inventory: Option<Vec<Option<UserItem>>>,
    pub equipment: Option<Vec<Option<UserItem>>>,
}

pub fn parse_server_message(header: PacketHeader, payload: Vec<u8>) -> ServerMessage {
    match ServerPacketId::try_from(header.opcode) {
        Ok(ServerPacketId::Connected) => ServerMessage::Connected,
        Ok(ServerPacketId::ClientVersion) => match payload.first().copied() {
            Some(byte) => ServerMessage::ClientVersion {
                result: ClientVersionResult::from(byte),
            },
            None => ServerMessage::ParseError {
                opcode: header.opcode,
                message: "client version packet missing result byte".to_string(),
            },
        },
        Ok(ServerPacketId::Disconnect) => match payload.first().copied() {
            Some(reason) => ServerMessage::Disconnect { reason },
            None => ServerMessage::ParseError {
                opcode: header.opcode,
                message: "disconnect packet missing reason byte".to_string(),
            },
        },
        Ok(ServerPacketId::KeepAlive) => {
            if payload.len() < 8 {
                return ServerMessage::ParseError {
                    opcode: header.opcode,
                    message: "keepalive packet too short".to_string(),
                };
            }
            let time = i64::from_le_bytes(payload[0..8].try_into().expect("slice length checked"));
            ServerMessage::KeepAlive { time }
        }
        Ok(ServerPacketId::Login) => match payload.first().copied() {
            Some(result) => ServerMessage::Login {
                result: LoginResult::from(result),
            },
            None => ServerMessage::ParseError {
                opcode: header.opcode,
                message: "login packet missing result byte".to_string(),
            },
        },
        Ok(ServerPacketId::LoginBanned) => {
            let mut cursor = Cursor::new(payload.as_slice());
            match read_dotnet_string(&mut cursor) {
                Ok(reason) => match cursor.read_i64::<LittleEndian>() {
                    Ok(expiry_ticks) => ServerMessage::LoginBanned {
                        reason,
                        expiry_ticks,
                    },
                    Err(err) => ServerMessage::ParseError {
                        opcode: header.opcode,
                        message: format!("login banned packet missing expiry: {err}"),
                    },
                },
                Err(err) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message: format!("login banned packet invalid reason: {err}"),
                },
            }
        }
        Ok(ServerPacketId::LoginSuccess) => match parse_login_success(&payload) {
            Ok(characters) => ServerMessage::LoginSuccess { characters },
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::StartGame) => {
            let mut cursor = Cursor::new(payload.as_slice());
            match cursor.read_u8() {
                Ok(result_byte) => match cursor.read_i32::<LittleEndian>() {
                    Ok(resolution) => ServerMessage::StartGame {
                        result: StartGameResult::from(result_byte),
                        resolution,
                    },
                    Err(err) => ServerMessage::ParseError {
                        opcode: header.opcode,
                        message: format!("start game packet missing resolution: {err}"),
                    },
                },
                Err(err) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message: format!("start game packet missing result: {err}"),
                },
            }
        }
        Ok(ServerPacketId::StartGameBanned) => {
            let mut cursor = Cursor::new(payload.as_slice());
            match read_dotnet_string(&mut cursor) {
                Ok(reason) => match cursor.read_i64::<LittleEndian>() {
                    Ok(expiry_ticks) => ServerMessage::StartGameBanned {
                        reason,
                        expiry_ticks,
                    },
                    Err(err) => ServerMessage::ParseError {
                        opcode: header.opcode,
                        message: format!("start game banned packet missing expiry: {err}"),
                    },
                },
                Err(err) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message: format!("start game banned packet invalid reason: {err}"),
                },
            }
        }
        Ok(ServerPacketId::StartGameDelay) => {
            let mut cursor = Cursor::new(payload.as_slice());
            match cursor.read_i64::<LittleEndian>() {
                Ok(milliseconds) => ServerMessage::StartGameDelay { milliseconds },
                Err(err) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message: format!("start game delay packet invalid payload: {err}"),
                },
            }
        }
        Ok(ServerPacketId::NewMapInfo) => match parse_new_map_info(&payload) {
            Ok(info) => ServerMessage::NewMapInfo(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::MapInformation) => match parse_map_information(&payload) {
            Ok(info) => ServerMessage::MapInformation(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::WorldMapSetup) => match parse_world_map_setup(&payload) {
            Ok(info) => ServerMessage::WorldMapSetup(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::SearchMapResult) => match parse_search_map_result(&payload) {
            Ok(result) => ServerMessage::SearchMapResult(result),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::UserInformation) => match parse_user_information(&payload) {
            Ok(info) => ServerMessage::UserInformation(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::UserLocation) => match parse_user_location(&payload) {
            Ok(info) => ServerMessage::UserLocation(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::UserSlotsRefresh) => match parse_user_slots_refresh(&payload) {
            Ok(info) => ServerMessage::UserSlotsRefresh(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectPlayer) => match parse_object_player(&payload) {
            Ok(object) => ServerMessage::ObjectPlayer(object),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectHero) => match parse_object_hero(&payload) {
            Ok(object) => ServerMessage::ObjectHero(object),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectRemove) => match parse_object_remove(&payload) {
            Ok(object) => ServerMessage::ObjectRemove(object),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectTurn) => match parse_object_motion(&payload) {
            Ok(object) => ServerMessage::ObjectTurn(object),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectWalk) => match parse_object_motion(&payload) {
            Ok(object) => ServerMessage::ObjectWalk(object),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectRun) => match parse_object_motion(&payload) {
            Ok(object) => ServerMessage::ObjectRun(object),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectAttack) => match parse_object_attack(&payload) {
            Ok(object) => ServerMessage::ObjectAttack(object),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::Struck) => match parse_struck(&payload) {
            Ok(info) => ServerMessage::Struck(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectStruck) => match parse_object_struck(&payload) {
            Ok(object) => ServerMessage::ObjectStruck(object),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::DamageIndicator) => match parse_damage_indicator(&payload) {
            Ok(info) => ServerMessage::DamageIndicator(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::DuraChanged) => match parse_dura_changed(&payload) {
            Ok(info) => ServerMessage::DuraChanged(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::DeleteItem) => match parse_delete_item(&payload) {
            Ok(info) => ServerMessage::DeleteItem(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::DeleteQuestItem) => match parse_delete_quest_item(&payload) {
            Ok(info) => ServerMessage::DeleteQuestItem(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectItem) => match parse_object_item(&payload) {
            Ok(info) => ServerMessage::ObjectItem(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectGold) => match parse_object_gold(&payload) {
            Ok(info) => ServerMessage::ObjectGold(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::GainedItem) => match parse_gained_item(&payload) {
            Ok(info) => ServerMessage::GainedItem(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::GainedGold) => match parse_gained_gold(&payload) {
            Ok(info) => ServerMessage::GainedGold(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::LoseGold) => match parse_lose_gold(&payload) {
            Ok(info) => ServerMessage::LoseGold(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::GainedCredit) => match parse_gained_credit(&payload) {
            Ok(info) => ServerMessage::GainedCredit(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::LoseCredit) => match parse_lose_credit(&payload) {
            Ok(info) => ServerMessage::LoseCredit(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::Death) => match parse_death(&payload) {
            Ok(info) => ServerMessage::Death(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectDied) => match parse_object_died(&payload) {
            Ok(info) => ServerMessage::ObjectDied(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ColourChanged) => match parse_colour_changed(&payload) {
            Ok(info) => ServerMessage::ColourChanged(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectColourChanged) => match parse_object_colour_changed(&payload) {
            Ok(info) => ServerMessage::ObjectColourChanged(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectGuildNameChanged) => {
            match parse_object_guild_name_changed(&payload) {
                Ok(info) => ServerMessage::ObjectGuildNameChanged(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::GainExperience) => match parse_gain_experience(&payload) {
            Ok(info) => ServerMessage::GainExperience(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::GainHeroExperience) => match parse_gain_hero_experience(&payload) {
            Ok(info) => ServerMessage::GainHeroExperience(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::LevelChanged) => match parse_level_changed(&payload) {
            Ok(info) => ServerMessage::LevelChanged(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::HeroLevelChanged) => match parse_hero_level_changed(&payload) {
            Ok(info) => ServerMessage::HeroLevelChanged(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectLeveled) => match parse_object_leveled(&payload) {
            Ok(info) => ServerMessage::ObjectLeveled(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::HealthChanged) => match parse_health_changed(&payload) {
            Ok(info) => ServerMessage::HealthChanged(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::HeroHealthChanged) => match parse_hero_health_changed(&payload) {
            Ok(info) => ServerMessage::HeroHealthChanged(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::GainedQuestItem) => match parse_gained_quest_item(&payload) {
            Ok(info) => ServerMessage::GainedQuestItem(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(_) => ServerMessage::Unknown {
            opcode: header.opcode,
            payload,
        },
        Err(_) => ServerMessage::Unknown {
            opcode: header.opcode,
            payload,
        },
    }
}

fn parse_login_success(payload: &[u8]) -> Result<Vec<CharacterSummary>, String> {
    let mut cursor = Cursor::new(payload);
    let count = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read character count: {err}"))?;

    if count < 0 {
        return Err(format!("negative character count {count}"));
    }

    let mut characters = Vec::with_capacity(count as usize);
    for slot in 0..count {
        let index = cursor
            .read_i32::<LittleEndian>()
            .map_err(|err| format!("failed to read character index #{slot}: {err}"))?;
        let name = read_dotnet_string(&mut cursor)
            .map_err(|err| format!("failed to read character name #{slot}: {err}"))?;
        let level = cursor
            .read_u16::<LittleEndian>()
            .map_err(|err| format!("failed to read level for `{name}`: {err}"))?;
        let class_byte = cursor
            .read_u8()
            .map_err(|err| format!("failed to read class for `{name}`: {err}"))?;
        let class = MirClass::try_from(class_byte)
            .map_err(|_| format!("unknown class discriminant {class_byte} for `{name}`"))?;
        let gender_byte = cursor
            .read_u8()
            .map_err(|err| format!("failed to read gender for `{name}`: {err}"))?;
        let gender = MirGender::try_from(gender_byte)
            .map_err(|_| format!("unknown gender discriminant {gender_byte} for `{name}`"))?;
        let last_access_ticks = cursor
            .read_i64::<LittleEndian>()
            .map_err(|err| format!("failed to read last access for `{name}`: {err}"))?;

        characters.push(CharacterSummary {
            index,
            name,
            level,
            class,
            gender,
            last_access_ticks,
        });
    }

    Ok(characters)
}

fn parse_map_information(payload: &[u8]) -> Result<MapInformation, String> {
    let mut cursor = Cursor::new(payload);

    let map_index = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read map index: {err}"))?;
    let file_name = read_dotnet_string(&mut cursor)
        .map_err(|err| format!("failed to read map file name: {err}"))?;
    let title = read_dotnet_string(&mut cursor)
        .map_err(|err| format!("failed to read map title: {err}"))?;
    let mini_map = cursor
        .read_u16::<LittleEndian>()
        .map_err(|err| format!("failed to read minimap id: {err}"))?;
    let big_map = cursor
        .read_u16::<LittleEndian>()
        .map_err(|err| format!("failed to read big map id: {err}"))?;
    let lights = cursor
        .read_u8()
        .map_err(|err| format!("failed to read light setting: {err}"))?;
    let bools = cursor
        .read_u8()
        .map_err(|err| format!("failed to read map flags: {err}"))?;
    let lightning = (bools & 0x01) != 0;
    let fire = (bools & 0x02) != 0;
    let map_dark_light = cursor
        .read_u8()
        .map_err(|err| format!("failed to read map dark light: {err}"))?;
    let music = cursor
        .read_u16::<LittleEndian>()
        .map_err(|err| format!("failed to read map music id: {err}"))?;
    let weather_bits = cursor
        .read_u16::<LittleEndian>()
        .map_err(|err| format!("failed to read map weather settings: {err}"))?;
    let weather = WeatherSetting::from_bits_truncate(weather_bits);

    Ok(MapInformation {
        map_index,
        file_name,
        title,
        mini_map,
        big_map,
        lights,
        lightning,
        fire,
        map_dark_light,
        music,
        weather,
    })
}

fn parse_new_map_info(payload: &[u8]) -> Result<NewMapInfo, String> {
    let mut cursor = Cursor::new(payload);

    let map_index = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read new map index: {err}"))?;
    let info = ClientMapInfo::read_from(&mut cursor)
        .map_err(|err| format!("failed to read client map info: {err}"))?;

    Ok(NewMapInfo { map_index, info })
}

fn parse_user_information(payload: &[u8]) -> Result<UserInformation, String> {
    let mut cursor = Cursor::new(payload);

    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read object id: {err}"))?;
    let real_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read real id: {err}"))?;
    let name = read_dotnet_string(&mut cursor)
        .map_err(|err| format!("failed to read character name: {err}"))?;
    let guild_name = read_dotnet_string(&mut cursor)
        .map_err(|err| format!("failed to read guild name: {err}"))?;
    let guild_rank = read_dotnet_string(&mut cursor)
        .map_err(|err| format!("failed to read guild rank: {err}"))?;
    let name_colour_argb = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read name colour: {err}"))?;
    let class_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read class: {err}"))?;
    let class = MirClass::try_from(class_byte)
        .map_err(|_| format!("unknown class discriminant {class_byte}"))?;
    let gender_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read gender: {err}"))?;
    let gender = MirGender::try_from(gender_byte)
        .map_err(|_| format!("unknown gender discriminant {gender_byte}"))?;
    let level = cursor
        .read_u16::<LittleEndian>()
        .map_err(|err| format!("failed to read level: {err}"))?;
    let location =
        Point::read_from(&mut cursor).map_err(|err| format!("failed to read location: {err}"))?;
    let direction_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read facing direction: {err}"))?;
    let direction = MirDirection::try_from(direction_byte)
        .map_err(|_| format!("unknown direction discriminant {direction_byte}"))?;
    let hair = cursor
        .read_u8()
        .map_err(|err| format!("failed to read hair style: {err}"))?;
    let hp = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read HP: {err}"))?;
    let mp = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read MP: {err}"))?;
    let experience = cursor
        .read_i64::<LittleEndian>()
        .map_err(|err| format!("failed to read experience: {err}"))?;
    let max_experience = cursor
        .read_i64::<LittleEndian>()
        .map_err(|err| format!("failed to read max experience: {err}"))?;
    let level_effects_bits = cursor
        .read_u16::<LittleEndian>()
        .map_err(|err| format!("failed to read level effects: {err}"))?;
    let level_effects = LevelEffects::from_bits_truncate(level_effects_bits);
    let has_hero =
        read_bool(&mut cursor).map_err(|err| format!("failed to read hero flag: {err}"))?;
    let hero_behaviour_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read hero behaviour: {err}"))?;
    let hero_behaviour = HeroBehaviour::try_from(hero_behaviour_byte)
        .map_err(|_| format!("unknown hero behaviour {hero_behaviour_byte}"))?;

    let inventory = read_item_slots(&mut cursor)?;
    let equipment = read_item_slots(&mut cursor)?;
    let quest_inventory = read_item_slots(&mut cursor)?;

    let gold = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read gold: {err}"))?;
    let credit = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read credit: {err}"))?;
    let has_expanded_storage = read_bool(&mut cursor)
        .map_err(|err| format!("failed to read expanded storage flag: {err}"))?;
    let expanded_storage_expiry_binary = cursor
        .read_i64::<LittleEndian>()
        .map_err(|err| format!("failed to read expanded storage expiry: {err}"))?;

    let magics = read_magic_list(&mut cursor)?;
    let intelligent_creatures = read_intelligent_creatures(&mut cursor)?;

    let summoned_creature_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read summoned creature type: {err}"))?;
    let summoned_creature_type = IntelligentCreatureType::try_from(summoned_creature_byte)
        .map_err(|_| format!("unknown summoned creature type {summoned_creature_byte}"))?;
    let creature_summoned = read_bool(&mut cursor)
        .map_err(|err| format!("failed to read creature summoned flag: {err}"))?;
    let allow_observe = read_bool(&mut cursor)
        .map_err(|err| format!("failed to read allow observe flag: {err}"))?;
    let observer =
        read_bool(&mut cursor).map_err(|err| format!("failed to read observer flag: {err}"))?;

    Ok(UserInformation {
        object_id,
        real_id,
        name,
        guild_name,
        guild_rank,
        name_colour_argb,
        class,
        gender,
        level,
        location,
        direction,
        hair,
        hp,
        mp,
        experience,
        max_experience,
        level_effects,
        has_hero,
        hero_behaviour,
        inventory,
        equipment,
        quest_inventory,
        gold,
        credit,
        has_expanded_storage,
        expanded_storage_expiry_binary,
        magics,
        intelligent_creatures,
        summoned_creature_type,
        creature_summoned,
        allow_observe,
        observer,
    })
}

fn parse_user_location(payload: &[u8]) -> Result<UserLocation, String> {
    let mut cursor = Cursor::new(payload);
    let location = Point::read_from(&mut cursor)
        .map_err(|err| format!("failed to read user location: {err}"))?;
    let direction_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read user direction: {err}"))?;
    let direction = MirDirection::try_from(direction_byte)
        .map_err(|_| format!("unknown user direction {direction_byte}"))?;
    Ok(UserLocation {
        location,
        direction,
    })
}

fn parse_user_slots_refresh(payload: &[u8]) -> Result<UserSlotsRefresh, String> {
    let mut cursor = Cursor::new(payload);
    let inventory = read_item_slots(&mut cursor)?;
    let equipment = read_item_slots(&mut cursor)?;
    Ok(UserSlotsRefresh {
        inventory,
        equipment,
    })
}

fn read_player_object(cursor: &mut Cursor<&[u8]>) -> Result<PlayerObject, String> {
    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read player object id: {err}"))?;
    let name =
        read_dotnet_string(cursor).map_err(|err| format!("failed to read player name: {err}"))?;
    let guild_name = read_dotnet_string(cursor)
        .map_err(|err| format!("failed to read player guild name: {err}"))?;
    let guild_rank_name = read_dotnet_string(cursor)
        .map_err(|err| format!("failed to read player guild rank name: {err}"))?;
    let name_colour_argb = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read player name colour: {err}"))?;
    let class_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read player class: {err}"))?;
    let class = MirClass::try_from(class_byte)
        .map_err(|_| format!("unknown player class discriminant {class_byte}"))?;
    let gender_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read player gender: {err}"))?;
    let gender = MirGender::try_from(gender_byte)
        .map_err(|_| format!("unknown player gender discriminant {gender_byte}"))?;
    let level = cursor
        .read_u16::<LittleEndian>()
        .map_err(|err| format!("failed to read player level: {err}"))?;
    let location =
        Point::read_from(cursor).map_err(|err| format!("failed to read player location: {err}"))?;
    let direction_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read player direction: {err}"))?;
    let direction = MirDirection::try_from(direction_byte)
        .map_err(|_| format!("unknown player direction discriminant {direction_byte}"))?;
    let hair = cursor
        .read_u8()
        .map_err(|err| format!("failed to read player hair: {err}"))?;
    let light = cursor
        .read_u8()
        .map_err(|err| format!("failed to read player light level: {err}"))?;
    let weapon = cursor
        .read_i16::<LittleEndian>()
        .map_err(|err| format!("failed to read player weapon: {err}"))?;
    let weapon_effect = cursor
        .read_i16::<LittleEndian>()
        .map_err(|err| format!("failed to read player weapon effect: {err}"))?;
    let armour = cursor
        .read_i16::<LittleEndian>()
        .map_err(|err| format!("failed to read player armour: {err}"))?;
    let poison_bits = cursor
        .read_u16::<LittleEndian>()
        .map_err(|err| format!("failed to read player poison flags: {err}"))?;
    let poison = PoisonType::from_bits(poison_bits)
        .ok_or_else(|| format!("unknown player poison flags {poison_bits:#06x}"))?;
    let dead =
        read_bool(cursor).map_err(|err| format!("failed to read player dead flag: {err}"))?;
    let hidden =
        read_bool(cursor).map_err(|err| format!("failed to read player hidden flag: {err}"))?;
    let effect_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read player effect: {err}"))?;
    let effect = SpellEffect::try_from(effect_byte)
        .map_err(|_| format!("unknown player effect discriminant {effect_byte}"))?;
    let wing_effect = cursor
        .read_u8()
        .map_err(|err| format!("failed to read player wing effect: {err}"))?;
    let extra =
        read_bool(cursor).map_err(|err| format!("failed to read player extra flag: {err}"))?;
    let mount_type = cursor
        .read_i16::<LittleEndian>()
        .map_err(|err| format!("failed to read player mount type: {err}"))?;
    let riding_mount = read_bool(cursor)
        .map_err(|err| format!("failed to read player riding mount flag: {err}"))?;
    let fishing =
        read_bool(cursor).map_err(|err| format!("failed to read player fishing flag: {err}"))?;
    let transform_type = cursor
        .read_i16::<LittleEndian>()
        .map_err(|err| format!("failed to read player transform type: {err}"))?;
    let element_orb_effect = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read player element orb effect: {err}"))?;
    let element_orb_level = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read player element orb level: {err}"))?;
    let element_orb_max = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read player element orb max: {err}"))?;
    let buff_count = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read player buff count: {err}"))?;
    if buff_count < 0 {
        return Err(format!("negative player buff count {buff_count}"));
    }
    let mut buffs = Vec::with_capacity(buff_count as usize);
    for index in 0..buff_count {
        let buff_byte = cursor
            .read_u8()
            .map_err(|err| format!("failed to read player buff #{index}: {err}"))?;
        let buff = BuffType::try_from(buff_byte)
            .map_err(|_| format!("unknown player buff discriminant {buff_byte}"))?;
        buffs.push(buff);
    }
    let level_effects_bits = cursor
        .read_u16::<LittleEndian>()
        .map_err(|err| format!("failed to read player level effects: {err}"))?;
    let level_effects = LevelEffects::from_bits_truncate(level_effects_bits);

    Ok(PlayerObject {
        object_id,
        name,
        guild_name,
        guild_rank_name,
        name_colour_argb,
        class,
        gender,
        level,
        location,
        direction,
        hair,
        light,
        weapon,
        weapon_effect,
        armour,
        poison,
        dead,
        hidden,
        effect,
        wing_effect,
        extra,
        mount_type,
        riding_mount,
        fishing,
        transform_type,
        element_orb_effect,
        element_orb_level,
        element_orb_max,
        buffs,
        level_effects,
    })
}

fn parse_object_player(payload: &[u8]) -> Result<PlayerObject, String> {
    let mut cursor = Cursor::new(payload);
    read_player_object(&mut cursor)
}

fn parse_object_hero(payload: &[u8]) -> Result<HeroObject, String> {
    let mut cursor = Cursor::new(payload);
    let player = read_player_object(&mut cursor)?;
    let owner_name = read_dotnet_string(&mut cursor)
        .map_err(|err| format!("failed to read hero owner name: {err}"))?;
    Ok(HeroObject { player, owner_name })
}

fn parse_object_remove(payload: &[u8]) -> Result<ObjectRemove, String> {
    let mut cursor = Cursor::new(payload);
    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read removed object id: {err}"))?;
    Ok(ObjectRemove { object_id })
}

fn parse_object_motion(payload: &[u8]) -> Result<ObjectMotion, String> {
    let mut cursor = Cursor::new(payload);

    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read object id: {err}"))?;
    let x = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read object x coordinate: {err}"))?;
    let y = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read object y coordinate: {err}"))?;
    let direction_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read object direction: {err}"))?;
    let direction = MirDirection::try_from(direction_byte)
        .map_err(|_| format!("unknown direction discriminant {direction_byte}"))?;

    Ok(ObjectMotion {
        object_id,
        location: Point::new(x, y),
        direction,
    })
}

fn parse_object_attack(payload: &[u8]) -> Result<ObjectAttack, String> {
    let mut cursor = Cursor::new(payload);

    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read attacking object id: {err}"))?;
    let x = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read attack x coordinate: {err}"))?;
    let y = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read attack y coordinate: {err}"))?;
    let direction_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read attack direction: {err}"))?;
    let direction = MirDirection::try_from(direction_byte)
        .map_err(|_| format!("unknown direction discriminant {direction_byte}"))?;
    let spell_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read attack spell: {err}"))?;
    let spell = Spell::try_from(spell_byte)
        .map_err(|_| format!("unknown spell discriminant {spell_byte}"))?;
    let level = cursor
        .read_u8()
        .map_err(|err| format!("failed to read attack level: {err}"))?;
    let attack_type = cursor
        .read_u8()
        .map_err(|err| format!("failed to read attack type: {err}"))?;

    Ok(ObjectAttack {
        object_id,
        location: Point::new(x, y),
        direction,
        spell,
        level,
        attack_type,
    })
}

fn parse_struck(payload: &[u8]) -> Result<Struck, String> {
    if payload.len() < 4 {
        return Err("struck packet too short".to_string());
    }

    let attacker_id = u32::from_le_bytes(payload[0..4].try_into().expect("slice length checked"));

    Ok(Struck { attacker_id })
}

fn parse_object_struck(payload: &[u8]) -> Result<ObjectStruck, String> {
    let mut cursor = Cursor::new(payload);

    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read struck object id: {err}"))?;
    let attacker_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read attacker id: {err}"))?;
    let x = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read struck x coordinate: {err}"))?;
    let y = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read struck y coordinate: {err}"))?;
    let direction_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read struck direction: {err}"))?;
    let direction = MirDirection::try_from(direction_byte)
        .map_err(|_| format!("unknown direction discriminant {direction_byte}"))?;

    Ok(ObjectStruck {
        object_id,
        attacker_id,
        location: Point::new(x, y),
        direction,
    })
}

fn parse_damage_indicator(payload: &[u8]) -> Result<DamageIndicator, String> {
    let mut cursor = Cursor::new(payload);

    let damage = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read damage amount: {err}"))?;
    let damage_type_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read damage type: {err}"))?;
    let damage_type = DamageType::try_from(damage_type_byte)
        .map_err(|_| format!("unknown damage type discriminant {damage_type_byte}"))?;
    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read damage target id: {err}"))?;

    Ok(DamageIndicator {
        damage,
        damage_type,
        object_id,
    })
}

fn parse_dura_changed(payload: &[u8]) -> Result<DuraChanged, String> {
    let mut cursor = Cursor::new(payload);

    let unique_id = cursor
        .read_u64::<LittleEndian>()
        .map_err(|err| format!("failed to read item unique id: {err}"))?;
    let current_dura = cursor
        .read_u16::<LittleEndian>()
        .map_err(|err| format!("failed to read item current dura: {err}"))?;

    Ok(DuraChanged {
        unique_id,
        current_dura,
    })
}

fn parse_delete_item(payload: &[u8]) -> Result<DeleteItem, String> {
    let mut cursor = Cursor::new(payload);

    let unique_id = cursor
        .read_u64::<LittleEndian>()
        .map_err(|err| format!("failed to read delete item unique id: {err}"))?;
    let count = cursor
        .read_u16::<LittleEndian>()
        .map_err(|err| format!("failed to read delete item count: {err}"))?;

    Ok(DeleteItem { unique_id, count })
}

fn parse_delete_quest_item(payload: &[u8]) -> Result<DeleteQuestItem, String> {
    let mut cursor = Cursor::new(payload);

    let unique_id = cursor
        .read_u64::<LittleEndian>()
        .map_err(|err| format!("failed to read delete quest item unique id: {err}"))?;
    let count = cursor
        .read_u16::<LittleEndian>()
        .map_err(|err| format!("failed to read delete quest item count: {err}"))?;

    Ok(DeleteQuestItem { unique_id, count })
}

fn parse_object_item(payload: &[u8]) -> Result<ObjectItem, String> {
    let mut cursor = Cursor::new(payload);

    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read object item id: {err}"))?;
    let name = read_dotnet_string(&mut cursor)
        .map_err(|err| format!("failed to read object item name: {err}"))?;
    let name_colour_argb = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read object item name colour: {err}"))?;
    let x = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read object item x coordinate: {err}"))?;
    let y = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read object item y coordinate: {err}"))?;
    let image = cursor
        .read_u16::<LittleEndian>()
        .map_err(|err| format!("failed to read object item image: {err}"))?;
    let grade_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read object item grade: {err}"))?;
    let grade = ItemGrade::try_from(grade_byte)
        .map_err(|_| format!("unknown item grade discriminant {grade_byte}"))?;

    Ok(ObjectItem {
        object_id,
        name,
        name_colour_argb,
        location: Point::new(x, y),
        image,
        grade,
    })
}

fn parse_object_gold(payload: &[u8]) -> Result<ObjectGold, String> {
    let mut cursor = Cursor::new(payload);

    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read object gold id: {err}"))?;
    let gold = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read object gold amount: {err}"))?;
    let x = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read object gold x coordinate: {err}"))?;
    let y = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read object gold y coordinate: {err}"))?;

    Ok(ObjectGold {
        object_id,
        gold,
        location: Point::new(x, y),
    })
}

fn parse_gained_item(payload: &[u8]) -> Result<GainedItem, String> {
    let mut cursor = Cursor::new(payload);
    let item = UserItem::read_default(&mut cursor)
        .map_err(|err| format!("failed to read gained item: {err}"))?;
    Ok(GainedItem { item })
}

fn parse_gained_quest_item(payload: &[u8]) -> Result<GainedQuestItem, String> {
    let mut cursor = Cursor::new(payload);
    let item = UserItem::read_default(&mut cursor)
        .map_err(|err| format!("failed to read gained quest item: {err}"))?;
    Ok(GainedQuestItem { item })
}

fn parse_gained_gold(payload: &[u8]) -> Result<GainedGold, String> {
    let mut cursor = Cursor::new(payload);
    let gold = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read gained gold amount: {err}"))?;
    Ok(GainedGold { gold })
}

fn parse_lose_gold(payload: &[u8]) -> Result<LoseGold, String> {
    let mut cursor = Cursor::new(payload);
    let gold = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read lost gold amount: {err}"))?;
    Ok(LoseGold { gold })
}

fn parse_gained_credit(payload: &[u8]) -> Result<GainedCredit, String> {
    let mut cursor = Cursor::new(payload);
    let credit = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read gained credit amount: {err}"))?;
    Ok(GainedCredit { credit })
}

fn parse_lose_credit(payload: &[u8]) -> Result<LoseCredit, String> {
    let mut cursor = Cursor::new(payload);
    let credit = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read lost credit amount: {err}"))?;
    Ok(LoseCredit { credit })
}

fn parse_colour_changed(payload: &[u8]) -> Result<ColourChanged, String> {
    let mut cursor = Cursor::new(payload);
    let name_colour_argb = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read name colour value: {err}"))?;
    Ok(ColourChanged { name_colour_argb })
}

fn parse_object_colour_changed(payload: &[u8]) -> Result<ObjectColourChanged, String> {
    let mut cursor = Cursor::new(payload);
    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read colour change object id: {err}"))?;
    let name_colour_argb = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read object name colour value: {err}"))?;
    Ok(ObjectColourChanged {
        object_id,
        name_colour_argb,
    })
}

fn parse_object_guild_name_changed(payload: &[u8]) -> Result<ObjectGuildNameChanged, String> {
    let mut cursor = Cursor::new(payload);
    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read guild name change object id: {err}"))?;
    let guild_name = read_dotnet_string(&mut cursor)
        .map_err(|err| format!("failed to read guild name: {err}"))?;
    Ok(ObjectGuildNameChanged {
        object_id,
        guild_name,
    })
}

fn parse_gain_experience(payload: &[u8]) -> Result<GainExperience, String> {
    let mut cursor = Cursor::new(payload);
    let amount = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read experience gain amount: {err}"))?;
    Ok(GainExperience { amount })
}

fn parse_gain_hero_experience(payload: &[u8]) -> Result<GainHeroExperience, String> {
    let mut cursor = Cursor::new(payload);
    let amount = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read hero experience gain amount: {err}"))?;
    Ok(GainHeroExperience { amount })
}

fn parse_level_changed(payload: &[u8]) -> Result<LevelChanged, String> {
    let mut cursor = Cursor::new(payload);
    let level = cursor
        .read_u16::<LittleEndian>()
        .map_err(|err| format!("failed to read new level: {err}"))?;
    let experience = cursor
        .read_i64::<LittleEndian>()
        .map_err(|err| format!("failed to read level experience total: {err}"))?;
    let max_experience = cursor
        .read_i64::<LittleEndian>()
        .map_err(|err| format!("failed to read level max experience: {err}"))?;
    Ok(LevelChanged {
        level,
        experience,
        max_experience,
    })
}

fn parse_hero_level_changed(payload: &[u8]) -> Result<HeroLevelChanged, String> {
    let mut cursor = Cursor::new(payload);
    let level = cursor
        .read_u16::<LittleEndian>()
        .map_err(|err| format!("failed to read hero level: {err}"))?;
    let experience = cursor
        .read_i64::<LittleEndian>()
        .map_err(|err| format!("failed to read hero experience total: {err}"))?;
    let max_experience = cursor
        .read_i64::<LittleEndian>()
        .map_err(|err| format!("failed to read hero max experience: {err}"))?;
    Ok(HeroLevelChanged {
        level,
        experience,
        max_experience,
    })
}

fn parse_object_leveled(payload: &[u8]) -> Result<ObjectLeveled, String> {
    let mut cursor = Cursor::new(payload);
    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read leveled object id: {err}"))?;
    Ok(ObjectLeveled { object_id })
}

fn parse_death(payload: &[u8]) -> Result<Death, String> {
    let mut cursor = Cursor::new(payload);

    let x = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read death x coordinate: {err}"))?;
    let y = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read death y coordinate: {err}"))?;
    let direction_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read death direction: {err}"))?;
    let direction = MirDirection::try_from(direction_byte)
        .map_err(|_| format!("unknown direction discriminant {direction_byte}"))?;

    Ok(Death {
        location: Point::new(x, y),
        direction,
    })
}

fn parse_object_died(payload: &[u8]) -> Result<ObjectDied, String> {
    let mut cursor = Cursor::new(payload);

    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read dying object id: {err}"))?;
    let x = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read death x coordinate: {err}"))?;
    let y = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read death y coordinate: {err}"))?;
    let direction_byte = cursor
        .read_u8()
        .map_err(|err| format!("failed to read death direction: {err}"))?;
    let direction = MirDirection::try_from(direction_byte)
        .map_err(|_| format!("unknown direction discriminant {direction_byte}"))?;
    let death_type = cursor
        .read_u8()
        .map_err(|err| format!("failed to read death type: {err}"))?;

    Ok(ObjectDied {
        object_id,
        location: Point::new(x, y),
        direction,
        death_type,
    })
}

fn parse_health_changed(payload: &[u8]) -> Result<HealthChanged, String> {
    let mut cursor = Cursor::new(payload);

    let hp = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read hp value: {err}"))?;
    let mp = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read mp value: {err}"))?;

    Ok(HealthChanged { hp, mp })
}

fn parse_hero_health_changed(payload: &[u8]) -> Result<HeroHealthChanged, String> {
    let mut cursor = Cursor::new(payload);

    let hp = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read hero hp value: {err}"))?;
    let mp = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read hero mp value: {err}"))?;

    Ok(HeroHealthChanged { hp, mp })
}

fn parse_world_map_setup(payload: &[u8]) -> Result<WorldMapSetupInfo, String> {
    let mut cursor = Cursor::new(payload);
    let setup = WorldMapSetup::read_from(&mut cursor)
        .map_err(|err| format!("failed to read world map setup: {err}"))?;
    let teleport_to_npc_cost = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read world map teleport cost: {err}"))?;
    Ok(WorldMapSetupInfo {
        setup,
        teleport_to_npc_cost,
    })
}

fn parse_search_map_result(payload: &[u8]) -> Result<SearchMapResult, String> {
    let mut cursor = Cursor::new(payload);
    let map_index = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read search map index: {err}"))?;
    let npc_index = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read search npc index: {err}"))?;
    Ok(SearchMapResult {
        map_index,
        npc_index,
    })
}

fn read_item_slots(cursor: &mut Cursor<&[u8]>) -> Result<Option<Vec<Option<UserItem>>>, String> {
    let has_items =
        read_bool(cursor).map_err(|err| format!("failed to read inventory flag: {err}"))?;
    if !has_items {
        return Ok(None);
    }

    let length = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read slot count: {err}"))?;
    if length < 0 {
        return Err(format!("negative slot count {length}"));
    }

    let mut slots = Vec::with_capacity(length as usize);
    for index in 0..length {
        let has_item = read_bool(cursor)
            .map_err(|err| format!("failed to read slot presence for index {index}: {err}"))?;
        if has_item {
            let item = UserItem::read_default(cursor)
                .map_err(|err| format!("failed to read user item at slot {index}: {err}"))?;
            slots.push(Some(item));
        } else {
            slots.push(None);
        }
    }

    Ok(Some(slots))
}

fn read_magic_list(cursor: &mut Cursor<&[u8]>) -> Result<Vec<ClientMagic>, String> {
    let count = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read magic count: {err}"))?;
    if count < 0 {
        return Err(format!("negative magic count {count}"));
    }

    let mut magics = Vec::with_capacity(count as usize);
    for index in 0..count {
        let magic = ClientMagic::read_from(cursor)
            .map_err(|err| format!("failed to read magic #{index}: {err}"))?;
        magics.push(magic);
    }
    Ok(magics)
}

fn read_intelligent_creatures(
    cursor: &mut Cursor<&[u8]>,
) -> Result<Vec<ClientIntelligentCreature>, String> {
    let count = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read intelligent creature count: {err}"))?;
    if count < 0 {
        return Err(format!("negative intelligent creature count {count}"));
    }

    let mut creatures = Vec::with_capacity(count as usize);
    for index in 0..count {
        let creature = ClientIntelligentCreature::read_from(cursor)
            .map_err(|err| format!("failed to read intelligent creature #{index}: {err}"))?;
        creatures.push(creature);
    }
    Ok(creatures)
}
