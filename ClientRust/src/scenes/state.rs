use std::collections::hash_map::Entry;
use std::collections::HashMap;

use mir2_shared::{
    ClientMagic, ClientQuestProgress, Point, SelectInfo, UserItem, enums::{
        AttackMode, DamageType, HeroSpawnState, ItemGrade, LightSetting, MirAction, MirDirection,
        PetMode, Spell,
    }
};

use crate::objects::{
    AnimationAdvanceSummary, MapObject, MapObjectType, ObjectActionOutcome, ObjectAttackOutcome,
    ObjectDeathOutcome, ObjectStruckOutcome, ObjectUpdateOutcome,
};

// 使用新的数据包架构 - 直接从 mir2_shared 导入
use mir2_shared::packets::server::{ObjectPlayer, ObjectHero, ObjectItem, ObjectMonster, ObjectNpc};
use crate::network::protocol::{packets};

// 为了简化代码，创建类型别名 - 映射所有使用的数据包类型
type UserInformation = packets::UserInformation;
type MapInformation = packets::MapInformation;
type NewMapInfo = packets::NewMapInfo;
type WorldMapSetupInfo = packets::WorldMapSetupInfo;
type SearchMapResult = packets::SearchMapResult;
type UserLocation = packets::UserLocation;
type UserSlotsRefresh = packets::UserSlotsRefresh;
type ObjectGold = packets::ObjectGold;
type DamageIndicator = packets::DamageIndicator;
type Death = packets::Death;
type DuraChanged = packets::DuraChanged;
type DeleteItem = packets::DeleteItem;
type DeleteQuestItem = packets::DeleteQuestItem;
type GainExperience = packets::GainExperience;
type GainHeroExperience = packets::GainHeroExperience;
type LevelChanged = packets::LevelChanged;
type HeroLevelChanged = packets::HeroLevelChanged;
type ObjectLeveled = packets::ObjectLeveled;
type ColourChanged = packets::ColourChanged;
type ObjectColourChanged = packets::ObjectColourChanged;
type ObjectGuildNameChanged = packets::ObjectGuildNameChanged;
type HealthChanged = packets::HealthChanged;
type HeroHealthChanged = packets::HeroHealthChanged;
type ObjectAttack = packets::ObjectAttack;
type ObjectHarvest = packets::ObjectHarvest;
type ObjectHarvested = packets::ObjectHarvested;
type ObjectStruck = packets::ObjectStruck;
type ObjectDied = packets::ObjectDied;

// 占位类型 - 这些还需要找到正确的映射
type NpcResponse = packets::NPCResponse;  // NPC对话响应
// ObjectMotion 可能是 ObjectWalk, ObjectRun 等多种移动packet的统称
// 暂时使用ObjectWalk作为代表，后续可能需要使用trait或enum来统一处理
type ObjectMotion = packets::ObjectWalk;  // 对象移动 (临时使用ObjectWalk)

#[derive(Debug, Default)]
pub struct ClientState {
    pub character: Option<UserInformation>,
    pub map_information: Option<MapInformation>,
    pub map_details: Option<NewMapInfo>,
    pub world_map: Option<WorldMapSetupInfo>,
    pub search_map_result: Option<SearchMapResult>,
    pub location: Option<UserLocation>,
    pub inventory: Vec<Option<UserItem>>,
    pub equipment: Vec<Option<UserItem>>,
    pub quest_inventory: Vec<Option<UserItem>>,
    pub objects: HashMap<u32, MapObject>,
    pub hero_object_id: Option<u32>,
    pub ground_objects: HashMap<u32, GroundObject>,
    pub npcs: HashMap<u32, NpcEntry>,
    pub player_dead: bool,
    pub last_player_death: Option<PlayerDeathEvent>,
    pub damage_history: Vec<DamageIndicatorEvent>,
    pub player_hp: Option<i32>,
    pub player_mp: Option<i32>,
    pub hero_hp: Option<i32>,
    pub hero_mp: Option<i32>,
    pub hero_level: Option<u16>,
    pub hero_experience: Option<i64>,
    pub hero_max_experience: Option<i64>,
    pub last_health_change: Option<HealthChangedEvent>,
    pub last_hero_health_change: Option<HeroHealthChangedEvent>,
    pub last_dura_change: Option<DuraChangedEvent>,
    pub dura_change_history: Vec<DuraChangedEvent>,
    pub last_item_delete: Option<ItemDeletionEvent>,
    pub item_delete_history: Vec<ItemDeletionEvent>,
    pub last_item_gain: Option<ItemGainEvent>,
    pub item_gain_history: Vec<ItemGainEvent>,
    pub last_quest_item_gain: Option<ItemGainEvent>,
    pub quest_item_gain_history: Vec<ItemGainEvent>,
    pub gold: u32,
    pub credit: u32,
    pub last_gold_change: Option<GoldChangeEvent>,
    pub gold_change_history: Vec<GoldChangeEvent>,
    pub last_credit_change: Option<CreditChangeEvent>,
    pub credit_change_history: Vec<CreditChangeEvent>,
    pub last_colour_change: Option<ColourChangeEvent>,
    pub colour_change_history: Vec<ColourChangeEvent>,
    pub last_object_colour_change: Option<ObjectColourChangeEvent>,
    pub object_colour_change_history: Vec<ObjectColourChangeEvent>,
    pub last_object_guild_change: Option<ObjectGuildChangeEvent>,
    pub object_guild_change_history: Vec<ObjectGuildChangeEvent>,
    pub last_experience_gain: Option<ExperienceGainEvent>,
    pub experience_gain_history: Vec<ExperienceGainEvent>,
    pub last_hero_experience_gain: Option<HeroExperienceGainEvent>,
    pub hero_experience_gain_history: Vec<HeroExperienceGainEvent>,
    pub last_level_change: Option<LevelChangeEvent>,
    pub level_change_history: Vec<LevelChangeEvent>,
    pub last_hero_level_change: Option<HeroLevelChangeEvent>,
    pub hero_level_change_history: Vec<HeroLevelChangeEvent>,
    pub last_object_level_up: Option<ObjectLevelUpEvent>,
    pub object_level_up_history: Vec<ObjectLevelUpEvent>,
    pub last_npc_response: Option<NpcResponseEvent>,
    pub npc_response_history: Vec<NpcResponseEvent>,
    // New fields for extended packet support
    pub player_magics: Vec<ClientMagic>,
    pub hero_magics: Vec<ClientMagic>,
    pub storage: Vec<Option<UserItem>>,
    pub hero_storage: Vec<Option<UserItem>>,
    pub quest_progress: Vec<ClientQuestProgress>,
    pub attack_mode: Option<AttackMode>,
    pub pet_mode: Option<PetMode>,
    pub light_setting: Option<LightSetting>,
    pub hero_spawn_state: Option<HeroSpawnState>,
    pub npc_rate: f32,
    pub logout_characters: Vec<SelectInfo>,
}

impl ClientState {
    pub fn update_from_user_information(&mut self, info: UserInformation) {
        if let Some(ref inventory) = info.inventory {
            self.inventory = inventory.clone();
        }
        if let Some(ref equipment) = info.equipment {
            self.equipment = equipment.clone();
        }
        if let Some(ref quest_inventory) = info.quest_inventory {
            self.quest_inventory = quest_inventory.clone();
        }
        self.gold = info.gold;
        self.credit = info.credit;
        self.character = Some(info);
    }

    pub fn update_inventory_slots(&mut self, slots: UserSlotsRefresh) {
        if let Some(inventory) = slots.inventory {
            self.inventory = inventory;
        }
        if let Some(equipment) = slots.equipment {
            self.equipment = equipment;
        }
    }

    pub fn update_map_information(&mut self, info: MapInformation) {
        self.map_information = Some(info);
    }

    pub fn update_map_details(&mut self, info: NewMapInfo) {
        self.map_details = Some(info);
    }

    pub fn update_world_map(&mut self, info: WorldMapSetupInfo) {
        self.world_map = Some(info);
    }

    pub fn update_search_map_result(&mut self, result: SearchMapResult) {
        self.search_map_result = Some(result);
    }

    pub fn update_location(&mut self, info: UserLocation) {
        self.location = Some(info);
    }

    pub fn upsert_player_object(&mut self, object: ObjectPlayer) -> ObjectUpdateOutcome {
        match self.objects.entry(object.object_id) {
            Entry::Occupied(mut entry) => {
                let map_object = entry.get_mut();
                let sync = map_object.sync_from_player_packet(&object);
                ObjectUpdateOutcome {
                    created: false,
                    object_type: map_object.object_type(),
                    sync,
                }
            }
            Entry::Vacant(entry) => {
                let (map_object, sync) = MapObject::from_player_packet(&object);
                let object_type = map_object.object_type();
                entry.insert(map_object);
                ObjectUpdateOutcome {
                    created: true,
                    object_type,
                    sync,
                }
            }
        }
    }

    pub fn upsert_hero_object(&mut self, object: ObjectHero) -> ObjectUpdateOutcome {
        let object_id = object.player.object_id;
        let hero_level = object.player.level;
        self.hero_object_id = Some(object_id);
        self.hero_level = Some(hero_level);
        match self.objects.entry(object_id) {
            Entry::Occupied(mut entry) => {
                let map_object = entry.get_mut();
                let sync = map_object.sync_from_hero_packet(&object);
                ObjectUpdateOutcome {
                    created: false,
                    object_type: map_object.object_type(),
                    sync,
                }
            }
            Entry::Vacant(entry) => {
                let (map_object, sync) = MapObject::from_hero_packet(&object);
                let object_type = map_object.object_type();
                entry.insert(map_object);
                ObjectUpdateOutcome {
                    created: true,
                    object_type,
                    sync,
                }
            }
        }
    }

    pub fn upsert_monster_object(&mut self, object: ObjectMonster) -> ObjectUpdateOutcome {
        let object_id = object.object_id;
        match self.objects.entry(object_id) {
            Entry::Occupied(mut entry) => {
                let map_object = entry.get_mut();
                let sync = map_object.sync_from_monster_packet(&object);
                ObjectUpdateOutcome {
                    created: false,
                    object_type: map_object.object_type(),
                    sync,
                }
            }
            Entry::Vacant(entry) => {
                let (map_object, sync) = MapObject::from_monster_packet(&object);
                let object_type = map_object.object_type();
                entry.insert(map_object);
                ObjectUpdateOutcome {
                    created: true,
                    object_type,
                    sync,
                }
            }
        }
    }

    pub fn remove_object(&mut self, object_id: u32) -> Option<MapObject> {
        let removed = self.objects.remove(&object_id);
        if removed.is_some() && self.hero_object_id == Some(object_id) {
            self.hero_object_id = None;
        }
        removed
    }

    pub fn remove_npc(&mut self, object_id: u32) -> Option<NpcEntry> {
        self.npcs.remove(&object_id)
    }

    pub fn remove_ground_object(&mut self, object_id: u32) -> Option<GroundObjectRemoval> {
        self.ground_objects
            .remove(&object_id)
            .map(|object| GroundObjectRemoval { object_id, object })
    }

    pub fn spawn_object_item(&mut self, packet: ObjectItem) -> GroundObjectSpawn {
        use mir2_shared::Point;
        let item_info = packet.item.info.as_ref();
        let name = item_info.map(|info| info.name.clone()).unwrap_or_default();
        let image = item_info.map(|info| info.image).unwrap_or(0);
        let grade = item_info.map(|info| info.grade).unwrap_or(mir2_shared::ItemGrade::None);
        // 根据grade计算颜色 (临时实现)
        let name_colour_argb = match grade {
            mir2_shared::ItemGrade::None => 0xFFFF_FFFFu32 as i32,
            mir2_shared::ItemGrade::Common => 0xFFFF_FFFFu32 as i32,
            mir2_shared::ItemGrade::Rare => 0xFF00_FF00u32 as i32,
            mir2_shared::ItemGrade::Legendary => 0xFFFF_AA00u32 as i32,
            mir2_shared::ItemGrade::Mythical => 0xFF00_80FFu32 as i32,
            mir2_shared::ItemGrade::Heroic => 0xFFFF_00FFu32 as i32,
        };
        let object = GroundObject::Item(GroundItemEntry {
            object_id: packet.object_id,
            name,
            name_colour_argb,
            location: Point::new(packet.location_x, packet.location_y),
            image,
            grade,
        });
        self.ground_objects.insert(packet.object_id, object.clone());
        GroundObjectSpawn {
            object_id: packet.object_id,
            object,
        }
    }

    pub fn spawn_object_gold(&mut self, packet: ObjectGold) -> GroundObjectSpawn {
        use mir2_shared::Point;
        let object = GroundObject::Gold(GroundGoldEntry {
            object_id: packet.object_id,
            amount: packet.gold,
            location: Point::new(packet.location_x, packet.location_y),
        });
        self.ground_objects.insert(packet.object_id, object.clone());
        GroundObjectSpawn {
            object_id: packet.object_id,
            object,
        }
    }

    pub fn apply_damage_indicator(&mut self, packet: DamageIndicator) -> DamageIndicatorOutcome {
        use mir2_shared::DamageType;
        let object_type = self
            .objects
            .get(&packet.object_id)
            .map(|object| object.object_type());

        let damage_type = DamageType::try_from(packet.damage_type).unwrap_or(DamageType::Hit);
        let event = DamageIndicatorEvent {
            object_id: packet.object_id,
            object_type,
            damage: packet.damage,
            damage_type,
        };
        if self.damage_history.len() >= 100 {
            self.damage_history.remove(0);
        }
        self.damage_history.push(event);

        DamageIndicatorOutcome { event }
    }

    pub fn apply_player_death(&mut self, packet: Death) -> PlayerDeathEvent {
        use mir2_shared::{Point, MirDirection};
        let location = Point::new(packet.location_x as i32, packet.location_y as i32);
        let direction = MirDirection::try_from(packet.direction).unwrap_or(MirDirection::Up);
        let event = PlayerDeathEvent {
            location,
            direction,
        };
        self.player_dead = true;
        self.last_player_death = Some(event);

        if let Some(character) = self.character.as_mut() {
            character.location_x = location.x;
            character.location_y = location.y;
            character.direction = direction;
        }
        if let Some(location_info) = self.location.as_mut() {
            location_info.location_x = location.x;
            location_info.location_y = location.y;
            location_info.direction = direction;
        }

        event
    }

    pub fn apply_dura_changed(&mut self, packet: DuraChanged) -> DuraChangedEvent {
        let mut location = None;

        if Self::update_dura_for_slots(
            self.inventory.as_mut_slice(),
            packet.unique_id,
            packet.current_dura,
        ) {
            location = Some(ItemContainer::Inventory);
        } else if Self::update_dura_for_slots(
            self.equipment.as_mut_slice(),
            packet.unique_id,
            packet.current_dura,
        ) {
            location = Some(ItemContainer::Equipment);
        } else if Self::update_dura_for_slots(
            self.quest_inventory.as_mut_slice(),
            packet.unique_id,
            packet.current_dura,
        ) {
            location = Some(ItemContainer::QuestInventory);
        }

        if let Some(character) = self.character.as_mut() {
            if let Some(ref mut inventory) = character.inventory {
                Self::update_dura_for_slots(
                    inventory.as_mut_slice(),
                    packet.unique_id,
                    packet.current_dura,
                );
            }
            if let Some(ref mut equipment) = character.equipment {
                Self::update_dura_for_slots(
                    equipment.as_mut_slice(),
                    packet.unique_id,
                    packet.current_dura,
                );
            }
            if let Some(ref mut quest_inventory) = character.quest_inventory {
                Self::update_dura_for_slots(
                    quest_inventory.as_mut_slice(),
                    packet.unique_id,
                    packet.current_dura,
                );
            }
        }

        let event = DuraChangedEvent {
            unique_id: packet.unique_id,
            current_dura: packet.current_dura,
            location,
        };

        if self.dura_change_history.len() >= 100 {
            self.dura_change_history.remove(0);
        }
        self.dura_change_history.push(event);
        self.last_dura_change = Some(event);

        event
    }

    pub fn apply_delete_item(&mut self, packet: DeleteItem) -> ItemDeletionEvent {
        let mut location = None;
        let mut remaining_count = None;
        let mut removed_completely = false;
        
        // Convert u32 count to u16 (clamp if necessary)
        let count = packet.count.min(u16::MAX as u32) as u16;

        if let Some(result) = Self::remove_item_from_slots(
            self.inventory.as_mut_slice(),
            packet.unique_id,
            count,
        ) {
            location = Some(ItemContainer::Inventory);
            remaining_count = result.remaining_count;
            removed_completely = result.removed_completely;
        } else if let Some(result) = Self::remove_item_from_slots(
            self.equipment.as_mut_slice(),
            packet.unique_id,
            count,
        ) {
            location = Some(ItemContainer::Equipment);
            remaining_count = result.remaining_count;
            removed_completely = result.removed_completely;
        } else if let Some(result) = Self::remove_item_from_slots(
            self.quest_inventory.as_mut_slice(),
            packet.unique_id,
            count,
        ) {
            location = Some(ItemContainer::QuestInventory);
            remaining_count = result.remaining_count;
            removed_completely = result.removed_completely;
        }

        if let Some(character) = self.character.as_mut() {
            if let Some(ref mut inventory) = character.inventory {
                Self::remove_item_from_slots(
                    inventory.as_mut_slice(),
                    packet.unique_id,
                    count,
                );
            }
            if let Some(ref mut equipment) = character.equipment {
                Self::remove_item_from_slots(
                    equipment.as_mut_slice(),
                    packet.unique_id,
                    count,
                );
            }
            if let Some(ref mut quest_inventory) = character.quest_inventory {
                Self::remove_item_from_slots(
                    quest_inventory.as_mut_slice(),
                    packet.unique_id,
                    count,
                );
            }
        }

        let event = ItemDeletionEvent {
            unique_id: packet.unique_id,
            removed_count: count,
            remaining_count,
            location,
            removed_completely,
        };

        if self.item_delete_history.len() >= 100 {
            self.item_delete_history.remove(0);
        }
        self.item_delete_history.push(event);
        self.last_item_delete = Some(event);

        event
    }

    pub fn apply_delete_quest_item(&mut self, packet: DeleteQuestItem) -> ItemDeletionEvent {
        let mut remaining_count = None;
        let mut removed_completely = false;
        let mut location = None;
        let mut unique_id = 0u64;
        let mut removed_count = 0u16;

        // Find and remove item by item_id (not unique_id)
        // Since packet only has item_id, we need to find the first item with matching info.index
        for slot in self.quest_inventory.iter_mut() {
            if let Some(item) = slot {
                if let Some(info) = &item.info {
                    if info.index == packet.item_id {
                        unique_id = item.unique_id;
                        removed_count = item.count;
                        *slot = None;
                        location = Some(ItemContainer::QuestInventory);
                        removed_completely = true;
                        break;
                    }
                }
            }
        }

        // Also remove from character's quest inventory if present
        if let Some(character) = self.character.as_mut() {
            if let Some(ref mut quest_inventory) = character.quest_inventory {
                for slot in quest_inventory.iter_mut() {
                    if let Some(item) = slot {
                        if let Some(info) = &item.info {
                            if info.index == packet.item_id {
                                *slot = None;
                                break;
                            }
                        }
                    }
                }
            }
        }

        let event = ItemDeletionEvent {
            unique_id,
            removed_count,
            remaining_count,
            location,
            removed_completely,
        };

        if self.item_delete_history.len() >= 100 {
            self.item_delete_history.remove(0);
        }
        self.item_delete_history.push(event);
        self.last_item_delete = Some(event);

        event
    }

    pub fn apply_gained_item(&mut self, item: UserItem) -> ItemGainEvent {
        let event = self.insert_item_into_container(ItemContainer::Inventory, item);
        Self::record_item_gain(&mut self.item_gain_history, &mut self.last_item_gain, event)
    }

    pub fn apply_gained_quest_item(&mut self, item: UserItem) -> ItemGainEvent {
        let event = self.insert_item_into_container(ItemContainer::QuestInventory, item);
        Self::record_item_gain(
            &mut self.quest_item_gain_history,
            &mut self.last_quest_item_gain,
            event,
        )
    }

    pub fn apply_gained_gold(&mut self, amount: u32) -> GoldChangeEvent {
        let new_total = self.gold.saturating_add(amount);
        self.gold = new_total;
        self.sync_character_currency();
        let event = GoldChangeEvent {
            change: i64::from(amount),
            new_total,
        };
        Self::record_currency_change(
            &mut self.gold_change_history,
            &mut self.last_gold_change,
            event,
        )
    }

    pub fn apply_lose_gold(&mut self, amount: u32) -> GoldChangeEvent {
        let deducted = amount.min(self.gold);
        self.gold = self.gold.saturating_sub(amount);
        self.sync_character_currency();
        let event = GoldChangeEvent {
            change: -(i64::from(deducted)),
            new_total: self.gold,
        };
        Self::record_currency_change(
            &mut self.gold_change_history,
            &mut self.last_gold_change,
            event,
        )
    }

    pub fn apply_gained_credit(&mut self, amount: u32) -> CreditChangeEvent {
        let new_total = self.credit.saturating_add(amount);
        self.credit = new_total;
        self.sync_character_currency();
        let event = CreditChangeEvent {
            change: i64::from(amount),
            new_total,
        };
        Self::record_currency_change(
            &mut self.credit_change_history,
            &mut self.last_credit_change,
            event,
        )
    }

    pub fn apply_lose_credit(&mut self, amount: u32) -> CreditChangeEvent {
        let deducted = amount.min(self.credit);
        self.credit = self.credit.saturating_sub(amount);
        self.sync_character_currency();
        let event = CreditChangeEvent {
            change: -(i64::from(deducted)),
            new_total: self.credit,
        };
        Self::record_currency_change(
            &mut self.credit_change_history,
            &mut self.last_credit_change,
            event,
        )
    }

    pub fn apply_gain_experience(&mut self, packet: GainExperience) -> ExperienceGainEvent {
        if let Some(character) = self.character.as_mut() {
            character.experience = character
                .experience
                .saturating_add(i64::from(packet.amount));
        }

        let event = ExperienceGainEvent {
            amount: packet.amount,
            new_experience_total: self.character.as_ref().map(|info| info.experience),
            max_experience: self.character.as_ref().map(|info| info.max_experience),
        };

        Self::record_clone_event(
            &mut self.experience_gain_history,
            &mut self.last_experience_gain,
            event,
        )
    }

    pub fn apply_gain_hero_experience(
        &mut self,
        packet: GainHeroExperience,
    ) -> HeroExperienceGainEvent {
        let new_total = self
            .hero_experience
            .unwrap_or_default()
            .saturating_add(i64::from(packet.amount));
        self.hero_experience = Some(new_total);

        let event = HeroExperienceGainEvent {
            amount: packet.amount,
            new_experience_total: self.hero_experience,
            max_experience: self.hero_max_experience,
        };

        Self::record_clone_event(
            &mut self.hero_experience_gain_history,
            &mut self.last_hero_experience_gain,
            event,
        )
    }

    pub fn apply_level_changed(&mut self, packet: LevelChanged) -> LevelChangeEvent {
        if let Some(character) = self.character.as_mut() {
            character.level = packet.level;
            character.experience = packet.experience;
            character.max_experience = packet.max_experience;
        }

        if let Some(player_id) = self.character.as_ref().map(|info| info.object_id) {
            if let Some(object) = self.objects.get_mut(&player_id) {
                object.set_level(packet.level);
            }
        }

        let event = LevelChangeEvent {
            level: packet.level,
            experience: packet.experience,
            max_experience: packet.max_experience,
        };

        Self::record_clone_event(
            &mut self.level_change_history,
            &mut self.last_level_change,
            event,
        )
    }

    pub fn apply_hero_level_changed(&mut self, packet: HeroLevelChanged) -> HeroLevelChangeEvent {
        self.hero_level = Some(packet.level);
        self.hero_experience = Some(packet.experience);
        self.hero_max_experience = Some(packet.max_experience);

        if let Some(hero_id) = self.hero_object_id {
            if let Some(object) = self.objects.get_mut(&hero_id) {
                object.set_level(packet.level);
            }
        }

        let event = HeroLevelChangeEvent {
            level: packet.level,
            experience: packet.experience,
            max_experience: packet.max_experience,
        };

        Self::record_clone_event(
            &mut self.hero_level_change_history,
            &mut self.last_hero_level_change,
            event,
        )
    }

    pub fn apply_object_leveled(&mut self, packet: ObjectLeveled) -> ObjectLevelUpEvent {
        let object_id = packet.object_id;
        let mut event = ObjectLevelUpEvent {
            object_id,
            object_type: None,
            is_player: false,
            is_hero: false,
        };

        if let Some(object) = self.objects.get(&object_id) {
            let object_type = object.object_type();
            event.object_type = Some(object_type);
            if matches!(object_type, MapObjectType::User)
                && self.character.as_ref().map(|info| info.object_id) == Some(object_id)
            {
                event.is_player = true;
            }
            if matches!(object_type, MapObjectType::Hero) && self.hero_object_id == Some(object_id)
            {
                event.is_hero = true;
            }
        }

        Self::record_clone_event(
            &mut self.object_level_up_history,
            &mut self.last_object_level_up,
            event,
        )
    }

    pub fn apply_colour_changed(&mut self, packet: ColourChanged) -> ColourChangeEvent {
        if let Some(character) = self.character.as_mut() {
            character.name_colour = packet.name_colour_argb;
        }

        let event = ColourChangeEvent {
            name_colour_argb: packet.name_colour_argb,
        };

        Self::record_clone_event(
            &mut self.colour_change_history,
            &mut self.last_colour_change,
            event,
        )
    }

    pub fn apply_object_colour_changed(
        &mut self,
        packet: ObjectColourChanged,
    ) -> ObjectColourChangeEvent {
        let ObjectColourChanged {
            object_id,
            name_colour_argb,
        } = packet;

        let mut event = ObjectColourChangeEvent {
            object_id,
            object_type: None,
            previous_colour: None,
            new_colour: name_colour_argb,
        };

        if let Some(object) = self.objects.get_mut(&object_id) {
            let previous = object.set_name_colour_argb(name_colour_argb);
            event.object_type = Some(object.object_type());
            event.previous_colour = Some(previous);
        }

        if let Some(character) = self.character.as_mut() {
            if character.object_id == object_id {
                character.name_colour = name_colour_argb;
            }
        }

        Self::record_clone_event(
            &mut self.object_colour_change_history,
            &mut self.last_object_colour_change,
            event,
        )
    }

    pub fn apply_object_guild_name_changed(
        &mut self,
        packet: ObjectGuildNameChanged,
    ) -> ObjectGuildChangeEvent {
        let ObjectGuildNameChanged {
            object_id,
            guild_name,
        } = packet;

        let mut event = ObjectGuildChangeEvent {
            object_id,
            object_type: None,
            previous_guild_name: None,
            new_guild_name: guild_name.clone(),
        };

        if let Some(object) = self.objects.get_mut(&object_id) {
            let previous = object.set_guild_name(guild_name.clone());
            event.object_type = Some(object.object_type());
            event.previous_guild_name = previous;
        }

        if let Some(character) = self.character.as_mut() {
            if character.object_id == object_id {
                character.guild_name = guild_name.clone();
            }
        }

        Self::record_clone_event(
            &mut self.object_guild_change_history,
            &mut self.last_object_guild_change,
            event,
        )
    }

    pub fn apply_health_changed(&mut self, packet: HealthChanged) -> HealthChangedEvent {
        // Convert u32 to i32 (clamp if necessary)
        let hp = packet.hp.min(i32::MAX as u32) as i32;
        let mp = packet.mp.min(i32::MAX as u32) as i32;
        
        let event = HealthChangedEvent {
            hp,
            mp,
        };
        self.player_hp = Some(hp);
        self.player_mp = Some(mp);
        self.last_health_change = Some(event);
        self.player_dead = hp <= 0;

        if let Some(character) = self.character.as_mut() {
            character.hp = hp;
            character.mp = mp;
        }

        event
    }

    pub fn apply_hero_health_changed(
        &mut self,
        packet: HeroHealthChanged,
    ) -> HeroHealthChangedEvent {
        // Convert u32 to i32 (clamp if necessary)
        let hp = packet.hp.min(i32::MAX as u32) as i32;
        let mp = packet.mp.min(i32::MAX as u32) as i32;
        
        let event = HeroHealthChangedEvent {
            hp,
            mp,
        };
        self.hero_hp = Some(hp);
        self.hero_mp = Some(mp);
        self.last_hero_health_change = Some(event);

        event
    }

    pub fn upsert_npc(&mut self, packet: ObjectNpc) -> NpcUpdateOutcome {
        use mir2_shared::Point;
        let entry = NpcEntry {
            object_id: packet.object_id,
            name: packet.name,
            name_colour_argb: packet.name_colour,  // packet uses name_colour
            image: packet.image,
            colour_argb: packet.colour,  // packet uses colour
            location: Point::new(packet.location_x, packet.location_y),
            direction: packet.direction,
            quest_ids: Vec::new(),  // packet doesn't include quest_ids, initialize empty
        };

        match self.npcs.entry(entry.object_id) {
            Entry::Occupied(mut occupied) => {
                occupied.insert(entry.clone());
                NpcUpdateOutcome {
                    created: false,
                    npc: occupied.get().clone(),
                }
            }
            Entry::Vacant(vacant) => {
                vacant.insert(entry.clone());
                NpcUpdateOutcome {
                    created: true,
                    npc: entry,
                }
            }
        }
    }

    pub fn apply_npc_response(&mut self, packet: NpcResponse) -> NpcResponseEvent {
        let event = NpcResponseEvent {
            line_count: packet.page.len(),
            page: packet.page,
        };
        Self::record_clone_event(
            &mut self.npc_response_history,
            &mut self.last_npc_response,
            event,
        )
    }

    pub fn apply_object_action(
        &mut self,
        motion: ObjectMotion,
        action: MirAction,
    ) -> Option<ObjectActionOutcome> {
        use mir2_shared::Point;
        let object = self.objects.get_mut(&motion.object_id)?;
        let location = Point::new(motion.location_x, motion.location_y);
        let result = object.apply_action(action, motion.direction, location);
        Some(ObjectActionOutcome {
            object_id: motion.object_id,
            object_type: object.object_type(),
            result,
        })
    }

    pub fn apply_object_attack(&mut self, packet: ObjectAttack) -> Option<ObjectAttackOutcome> {
        use mir2_shared::Point;
        let object = self.objects.get_mut(&packet.object_id)?;
        let location = Point::new(packet.location_x as i32, packet.location_y as i32);
        let direction = MirDirection::try_from(packet.direction).unwrap_or(MirDirection::Up);
        let spell = Spell::try_from(packet.spell).unwrap_or(Spell::None);
        let attack = object.apply_attack(
            direction,
            location,
            spell,
            packet.level as u8,
            packet.attack_type,
        );
        Some(ObjectAttackOutcome {
            object_id: packet.object_id,
            object_type: object.object_type(),
            attack,
        })
    }

    pub fn apply_object_harvest(&mut self, packet: ObjectHarvest) -> Option<ObjectActionOutcome> {
        use mir2_shared::Point;
        let object = self.objects.get_mut(&packet.object_id)?;
        let location = Point::new(packet.location_x, packet.location_y);
        let result = object.apply_action(MirAction::Harvest, packet.direction, location);
        Some(ObjectActionOutcome {
            object_id: packet.object_id,
            object_type: object.object_type(),
            result,
        })
    }

    pub fn apply_object_harvested(
        &mut self,
        packet: ObjectHarvested,
    ) -> Option<ObjectActionOutcome> {
        use mir2_shared::Point;
        let object = self.objects.get_mut(&packet.object_id)?;
        let location = Point::new(packet.location_x, packet.location_y);
        let result = object.apply_action(MirAction::Skeleton, packet.direction, location);
        Some(ObjectActionOutcome {
            object_id: packet.object_id,
            object_type: object.object_type(),
            result,
        })
    }

    pub fn apply_object_struck(&mut self, packet: ObjectStruck) -> Option<ObjectStruckOutcome> {
        use mir2_shared::Point;
        let object = self.objects.get_mut(&packet.object_id)?;
        let location = Point::new(packet.location_x as i32, packet.location_y as i32);
        let direction = MirDirection::try_from(packet.direction).unwrap_or(MirDirection::Up);
        let struck = object.apply_struck(direction, location, packet.attacker_id);
        Some(ObjectStruckOutcome {
            object_id: packet.object_id,
            object_type: object.object_type(),
            struck,
        })
    }

    pub fn apply_object_died(&mut self, packet: ObjectDied) -> Option<ObjectDeathOutcome> {
        use mir2_shared::Point;
        let location = Point::new(packet.location_x as i32, packet.location_y as i32);
        match self.objects.entry(packet.object_id) {
            Entry::Occupied(mut entry) => {
                if packet.death_type == 0 {
                    let object_type = entry.get().object_type();
                    let direction = MirDirection::try_from(packet.direction).unwrap_or(MirDirection::Up);
                    let transition = entry
                        .get_mut()
                        .apply_death(direction, location);
                    Some(ObjectDeathOutcome {
                        object_id: packet.object_id,
                        object_type,
                        death_type: packet.death_type,
                        transition: Some(transition),
                        removed: false,
                        location,
                        direction,
                    })
                } else {
                    let object = entry.remove();
                    let object_type = object.object_type();
                    let direction = MirDirection::try_from(packet.direction).unwrap_or(MirDirection::Up);
                    Some(ObjectDeathOutcome {
                        object_id: packet.object_id,
                        object_type,
                        death_type: packet.death_type,
                        transition: None,
                        removed: true,
                        location,
                        direction,
                    })
                }
            }
            Entry::Vacant(_) => None,
        }
    }

    pub fn advance_animations(&mut self, delta_ms: u32) -> AnimationAdvanceSummary {
        let mut summary = AnimationAdvanceSummary::default();
        for object in self.objects.values_mut() {
            let step = object.advance(delta_ms);
            summary.record_step(&step);
        }
        summary
    }

    pub fn summary(&self) -> SummaryState {
        let character_name = self.character.as_ref().map(|info| info.name.clone());
        let map = self
            .map_details
            .as_ref()
            .map(|details| (details.map_index, details.title.clone()))
            .or_else(|| {
                self.map_information
                    .as_ref()
                    .map(|info| (info.map_index, info.title.clone()))
            });

        let (map_index, map_title) = map.unzip();

        let world_map_enabled = self.world_map.as_ref().map(|_info| true);
        let world_map_icon_count = self
            .world_map
            .as_ref()
            .map(|info| info.world_maps.len())
            .unwrap_or(0);
        let teleport_to_npc_cost: Option<i32> = None;
        let last_search_map = self
            .search_map_result
            .as_ref()
            .map(|result| result.map_index);
        let last_search_npc: Option<u32> = None;

        let map_object_count = self.objects.len();
        let mut hero_object_count = 0;
        let mut visible_player_count = 0;
        let mut visible_hero_count = 0;
        let mut visible_monster_count = 0;
        for object in self.objects.values() {
            match object.object_type() {
                MapObjectType::User => {
                    if !object.is_hidden() {
                        visible_player_count += 1;
                    }
                }
                MapObjectType::Hero => {
                    hero_object_count += 1;
                    if !object.is_hidden() {
                        visible_hero_count += 1;
                    }
                }
                MapObjectType::Monster => {
                    if !object.is_hidden() {
                        visible_monster_count += 1;
                    }
                }
            }
        }

        let ground_object_count = self.ground_objects.len();
        let npc_count = self.npcs.len();

        let level = self.character.as_ref().map(|info| info.level);
        let experience = self.character.as_ref().map(|info| info.experience);
        let max_experience = self.character.as_ref().map(|info| info.max_experience);
        let hero_level = self.hero_level;
        let hero_experience = self.hero_experience;
        let hero_max_experience = self.hero_max_experience;

        SummaryState {
            character_name,
            map_index,
            map_title,
            location: self.location.as_ref().map(|loc| Point::new(loc.location_x, loc.location_y)),
            inventory_slots: self.inventory.len(),
            equipment_slots: self.equipment.len(),
            gold: self.gold,
            credit: self.credit,
            level,
            experience,
            max_experience,
            hero_level,
            hero_experience,
            hero_max_experience,
            world_map_enabled,
            world_map_icon_count,
            teleport_to_npc_cost,
            last_search_map,
            last_search_npc,
            map_object_count,
            hero_object_count,
            visible_player_count,
            visible_hero_count,
            visible_monster_count,
            ground_object_count,
            npc_count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SummaryState {
    pub character_name: Option<String>,
    pub map_index: Option<i32>,
    pub map_title: Option<String>,
    pub location: Option<mir2_shared::Point>,
    pub inventory_slots: usize,
    pub equipment_slots: usize,
    pub gold: u32,
    pub credit: u32,
    pub level: Option<u16>,
    pub experience: Option<i64>,
    pub max_experience: Option<i64>,
    pub hero_level: Option<u16>,
    pub hero_experience: Option<i64>,
    pub hero_max_experience: Option<i64>,
    pub world_map_enabled: Option<bool>,
    pub world_map_icon_count: usize,
    pub teleport_to_npc_cost: Option<i32>,
    pub last_search_map: Option<i32>,
    pub last_search_npc: Option<u32>,
    pub map_object_count: usize,
    pub hero_object_count: usize,
    pub visible_player_count: usize,
    pub visible_hero_count: usize,
    pub visible_monster_count: usize,
    pub ground_object_count: usize,
    pub npc_count: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct DamageIndicatorEvent {
    pub object_id: u32,
    pub object_type: Option<MapObjectType>,
    pub damage: i32,
    pub damage_type: DamageType,
}

#[derive(Debug, Clone, Copy)]
pub struct DamageIndicatorOutcome {
    pub event: DamageIndicatorEvent,
}

#[derive(Debug, Clone, Copy)]
pub struct PlayerDeathEvent {
    pub location: mir2_shared::Point,
    pub direction: MirDirection,
}

#[derive(Debug, Clone, Copy)]
pub enum ItemContainer {
    Inventory,
    Equipment,
    QuestInventory,
}

#[derive(Debug, Clone, Copy)]
pub struct HealthChangedEvent {
    pub hp: i32,
    pub mp: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct HeroHealthChangedEvent {
    pub hp: i32,
    pub mp: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct DuraChangedEvent {
    pub unique_id: u64,
    pub current_dura: u16,
    pub location: Option<ItemContainer>,
}

#[derive(Debug, Clone, Copy)]
pub struct ItemDeletionEvent {
    pub unique_id: u64,
    pub removed_count: u16,
    pub remaining_count: Option<u16>,
    pub location: Option<ItemContainer>,
    pub removed_completely: bool,
}

#[derive(Debug, Clone)]
pub struct ItemGainEvent {
    pub item: UserItem,
    pub container: ItemContainer,
    pub slot_index: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct ColourChangeEvent {
    pub name_colour_argb: i32,
}

#[derive(Debug, Clone)]
pub struct ObjectColourChangeEvent {
    pub object_id: u32,
    pub object_type: Option<MapObjectType>,
    pub previous_colour: Option<i32>,
    pub new_colour: i32,
}

#[derive(Debug, Clone)]
pub struct ObjectGuildChangeEvent {
    pub object_id: u32,
    pub object_type: Option<MapObjectType>,
    pub previous_guild_name: Option<String>,
    pub new_guild_name: String,
}

#[derive(Debug, Clone, Copy)]
pub struct ExperienceGainEvent {
    pub amount: u32,
    pub new_experience_total: Option<i64>,
    pub max_experience: Option<i64>,
}

#[derive(Debug, Clone, Copy)]
pub struct HeroExperienceGainEvent {
    pub amount: u32,
    pub new_experience_total: Option<i64>,
    pub max_experience: Option<i64>,
}

#[derive(Debug, Clone, Copy)]
pub struct LevelChangeEvent {
    pub level: u16,
    pub experience: i64,
    pub max_experience: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct HeroLevelChangeEvent {
    pub level: u16,
    pub experience: i64,
    pub max_experience: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct ObjectLevelUpEvent {
    pub object_id: u32,
    pub object_type: Option<MapObjectType>,
    pub is_player: bool,
    pub is_hero: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct GoldChangeEvent {
    pub change: i64,
    pub new_total: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct CreditChangeEvent {
    pub change: i64,
    pub new_total: u32,
}

#[derive(Debug, Clone)]
pub struct NpcResponseEvent {
    pub line_count: usize,
    pub page: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct NpcEntry {
    pub object_id: u32,
    pub name: String,
    pub name_colour_argb: i32,
    pub image: u16,
    pub colour_argb: i32,
    pub location: mir2_shared::Point,
    pub direction: MirDirection,
    pub quest_ids: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct NpcUpdateOutcome {
    pub created: bool,
    pub npc: NpcEntry,
}

impl ClientState {
    fn update_dura_for_slots(
        slots: &mut [Option<UserItem>],
        unique_id: u64,
        current_dura: u16,
    ) -> bool {
        for slot in slots.iter_mut() {
            if let Some(item) = slot {
                if item.unique_id == unique_id {
                    item.current_dura = current_dura;
                    return true;
                }
                if Self::update_dura_for_slots(item.slots.as_mut_slice(), unique_id, current_dura) {
                    return true;
                }
            }
        }
        false
    }

    fn remove_item_from_slots(
        slots: &mut [Option<UserItem>],
        unique_id: u64,
        count: u16,
    ) -> Option<ItemRemovalResult> {
        for slot in slots.iter_mut() {
            if let Some(item) = slot {
                if item.unique_id == unique_id {
                    if count >= item.count {
                        *slot = None;
                        return Some(ItemRemovalResult {
                            remaining_count: None,
                            removed_completely: true,
                        });
                    }
                    item.count = item.count.saturating_sub(count);
                    return Some(ItemRemovalResult {
                        remaining_count: Some(item.count),
                        removed_completely: false,
                    });
                }

                if let Some(result) =
                    Self::remove_item_from_slots(item.slots.as_mut_slice(), unique_id, count)
                {
                    return Some(result);
                }
            }
        }
        None
    }

    fn insert_item_into_container(
        &mut self,
        container: ItemContainer,
        item: UserItem,
    ) -> ItemGainEvent {
        let slot_index = match container {
            ItemContainer::Inventory => {
                Self::insert_item_into_slots(&mut self.inventory, item.clone())
            }
            ItemContainer::Equipment => {
                Self::insert_item_into_slots(&mut self.equipment, item.clone())
            }
            ItemContainer::QuestInventory => {
                Self::insert_item_into_slots(&mut self.quest_inventory, item.clone())
            }
        };
        self.sync_character_container(container);
        ItemGainEvent {
            item,
            container,
            slot_index,
        }
    }

    fn insert_item_into_slots(slots: &mut Vec<Option<UserItem>>, item: UserItem) -> usize {
        for (index, slot) in slots.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(item);
                return index;
            }
        }
        slots.push(Some(item));
        slots.len() - 1
    }

    fn sync_character_container(&mut self, container: ItemContainer) {
        if let Some(character) = self.character.as_mut() {
            match container {
                ItemContainer::Inventory => {
                    character.inventory = Some(self.inventory.clone());
                }
                ItemContainer::Equipment => {
                    character.equipment = Some(self.equipment.clone());
                }
                ItemContainer::QuestInventory => {
                    character.quest_inventory = Some(self.quest_inventory.clone());
                }
            }
        }
    }

    fn sync_character_currency(&mut self) {
        if let Some(character) = self.character.as_mut() {
            character.gold = self.gold;
            character.credit = self.credit;
        }
    }

    fn record_clone_event<T: Clone>(history: &mut Vec<T>, last: &mut Option<T>, event: T) -> T {
        if history.len() >= 100 {
            history.remove(0);
        }
        history.push(event.clone());
        *last = Some(event.clone());
        event
    }

    fn record_item_gain(
        history: &mut Vec<ItemGainEvent>,
        last: &mut Option<ItemGainEvent>,
        event: ItemGainEvent,
    ) -> ItemGainEvent {
        Self::record_clone_event(history, last, event)
    }

    fn record_currency_change<T: Copy>(history: &mut Vec<T>, last: &mut Option<T>, event: T) -> T {
        if history.len() >= 100 {
            history.remove(0);
        }
        history.push(event);
        *last = Some(event);
        event
    }
}

#[derive(Debug, Clone, Copy)]
struct ItemRemovalResult {
    remaining_count: Option<u16>,
    removed_completely: bool,
}

#[derive(Debug, Clone)]
pub enum GroundObject {
    Item(GroundItemEntry),
    Gold(GroundGoldEntry),
}

impl GroundObject {
    pub fn location(&self) -> mir2_shared::Point {
        match self {
            GroundObject::Item(entry) => entry.location,
            GroundObject::Gold(entry) => entry.location,
        }
    }

    pub fn kind(&self) -> GroundObjectKind {
        match self {
            GroundObject::Item(_) => GroundObjectKind::Item,
            GroundObject::Gold(_) => GroundObjectKind::Gold,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GroundItemEntry {
    pub object_id: u32,
    pub name: String,
    pub name_colour_argb: i32,
    pub location: mir2_shared::Point,
    pub image: u16,
    pub grade: ItemGrade,
}

#[derive(Debug, Clone)]
pub struct GroundGoldEntry {
    pub object_id: u32,
    pub amount: u32,
    pub location: mir2_shared::Point,
}

#[derive(Debug, Clone, Copy)]
pub enum GroundObjectKind {
    Item,
    Gold,
}

#[derive(Debug, Clone)]
pub struct GroundObjectSpawn {
    pub object_id: u32,
    pub object: GroundObject,
}

impl GroundObjectSpawn {
    pub fn location(&self) -> mir2_shared::Point {
        self.object.location()
    }

    pub fn kind(&self) -> GroundObjectKind {
        self.object.kind()
    }
}

#[derive(Debug, Clone)]
pub struct GroundObjectRemoval {
    pub object_id: u32,
    pub object: GroundObject,
}

impl GroundObjectRemoval {
    pub fn location(&self) -> mir2_shared::Point {
        self.object.location()
    }

    pub fn kind(&self) -> GroundObjectKind {
        self.object.kind()
    }
}

// Extended state management methods
impl ClientState {
    // NPC interaction methods
    pub fn set_npc_rate(&mut self, rate: f32) {
        self.npc_rate = rate;
    }

    // Magic/Skill methods
    pub fn add_magic(&mut self, magic: ClientMagic, hero: bool) {
        if hero {
            self.hero_magics.push(magic);
        } else {
            self.player_magics.push(magic);
        }
    }

    pub fn remove_magic(&mut self, place_id: u8, hero: bool) {
        if hero {
            if (place_id as usize) < self.hero_magics.len() {
                self.hero_magics.remove(place_id as usize);
            }
        } else {
            if (place_id as usize) < self.player_magics.len() {
                self.player_magics.remove(place_id as usize);
            }
        }
    }

    pub fn level_magic(&mut self, spell: Spell, level: u8, experience: u16, hero: bool) {
        let magics = if hero {
            &mut self.hero_magics
        } else {
            &mut self.player_magics
        };

        if let Some(magic) = magics.iter_mut().find(|m| m.spell == spell) {
            magic.level = level;
            magic.experience = experience;
        }
    }

    pub fn toggle_spell(&mut self, spell: Spell, hero: bool) {
        let magics = if hero {
            &mut self.hero_magics
        } else {
            &mut self.player_magics
        };

        if let Some(_magic) = magics.iter_mut().find(|m| m.spell == spell) {
            // ClientMagic doesn't have is_equipped field, spell toggling logic will be in UI
        }
    }

    // Storage methods
    pub fn update_storage(&mut self, storage: Vec<Option<UserItem>>) {
        self.storage = storage;
    }

    pub fn update_hero_storage(&mut self, storage: Vec<Option<UserItem>>) {
        self.hero_storage = storage;
    }

    // Mode methods
    pub fn set_attack_mode(&mut self, mode: AttackMode) {
        self.attack_mode = Some(mode);
    }

    pub fn set_pet_mode(&mut self, mode: PetMode) {
        self.pet_mode = Some(mode);
    }

    pub fn set_light_setting(&mut self, setting: LightSetting) {
        self.light_setting = Some(setting);
    }

    // Hero methods
    pub fn set_hero_spawn_state(&mut self, state: HeroSpawnState) {
        self.hero_spawn_state = Some(state);
    }

    // Quest methods
    pub fn update_quest(&mut self, quest: ClientQuestProgress) {
        if let Some(existing) = self.quest_progress.iter_mut().find(|q| q.id == quest.id) {
            *existing = quest;
        } else {
            self.quest_progress.push(quest);
        }
    }

    // Object status update methods (simplified - actual implementation depends on MapObject API)
    pub fn log_object_health(&self, object_id: u32, hp: u32, mp: u32) {
        if let Some(_object) = self.objects.get(&object_id) {
            tracing::trace!("Object {} health updated: HP={}, MP={}", object_id, hp, mp);
        }
    }

    pub fn log_object_mana(&self, object_id: u32, mp: u32) {
        if let Some(_object) = self.objects.get(&object_id) {
            tracing::trace!("Object {} mana updated: MP={}", object_id, mp);
        }
    }

    pub fn log_object_hidden(&self, object_id: u32, hidden: bool) {
        if let Some(_object) = self.objects.get(&object_id) {
            tracing::debug!("Object {} hidden status: {}", object_id, hidden);
        }
    }

    pub fn log_object_name(&self, object_id: u32, name: &str) {
        if let Some(_object) = self.objects.get(&object_id) {
            tracing::info!("Object {} name: {}", object_id, name);
        }
    }

    // Logout methods
    pub fn store_logout_characters(&mut self, characters: Vec<SelectInfo>) {
        self.logout_characters = characters;
    }
}
