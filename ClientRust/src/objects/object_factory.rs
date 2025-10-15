// object_factory.rs - Factory for creating game objects from server packets
// Handles conversion from network packets to game objects
//
// NOTE: This is a Phase 1 implementation focusing on basic object creation.
// Full packet parsing and state synchronization will be implemented in Phase 2.

use mir2_shared::packets::server::*;
use mir2_shared::Point;

use super::{
    MonsterObject, NPCObject, ItemObject, UserObject, HeroObject, PlayerMovementFSM,
};

#[allow(unused_imports)]
use super::{PlayerObject, MapObject, MapObjectType};

/// Object factory - creates game objects from server packets
pub struct ObjectFactory;

impl ObjectFactory {
    /// Create MonsterObject from ObjectMonster packet
    /// C# Reference: Client/MirScenes/GameScene.cs ObjectMonster handler
    pub fn create_monster(packet: &ObjectMonster) -> MonsterObject {
        let mut monster = MonsterObject::new(packet.object_id);
        
        // Set basic info
        monster.map_object.set_name(packet.name.clone());
        monster.map_object.set_name_colour_argb(packet.name_colour);
        
        // Set location
        let location = Point::new(packet.location_x, packet.location_y);
        monster.map_object.set_location(location);
        monster.map_object.set_current_location(location);
        monster.map_object.set_map_location(location);
        
        // Set visual properties
        monster.map_object.set_direction(packet.direction);
        monster.map_object.set_light(packet.light as i32);  // u8 -> i32
        
        // Set monster-specific properties
        monster.base_image = super::monster_object::Monster::from_u16(packet.image);
        monster.effect = packet.effect;
        
        // Set state
        monster.map_object.set_dead(packet.dead);
        monster.skeleton = packet.skeleton;
        monster.map_object.set_poison(packet.poison);
        
        // Set health (if packet has this field)
        // Note: ObjectMonster packet structure may vary
        // monster.map_object.set_percent_health(packet.percent_health);
        
        // Set buffs (if packet has this field)
        // Note: Packet structure may not include buffs
        // if !packet.buffs.is_empty() {
        //     for buff in &packet.buffs {
        //         monster.map_object.add_buff(*buff);
        //     }
        // }
        
        tracing::debug!("Created MonsterObject: id={}, name='{}', image={:?}, location=({}, {})",
            packet.object_id, packet.name, monster.base_image, location.x, location.y);
        
        monster
    }
    
    /// Create NPCObject from ObjectNpc packet
    /// C# Reference: Client/MirScenes/GameScene.cs ObjectNpc handler
    pub fn create_npc(packet: &ObjectNpc) -> NPCObject {
        let mut npc = NPCObject::new(packet.object_id);
        
        // Set basic info
        npc.map_object.set_name(packet.name.clone());
        npc.map_object.set_name_colour_argb(packet.name_colour);
        
        // Set location
        let location = Point::new(packet.location_x, packet.location_y);
        npc.map_object.set_location(location);
        npc.map_object.set_current_location(location);
        npc.map_object.set_map_location(location);
        
        // Set visual properties
        npc.map_object.set_direction(packet.direction);
        npc.map_object.set_light(packet.colour);  // NPC uses colour field instead of light
        
        // Set NPC-specific properties
        npc.image = super::npc_object::NpcImage::from_u16(packet.image);
        
        tracing::debug!("Created NPCObject: id={}, name='{}', image={:?}, location=({}, {})",
            packet.object_id, packet.name, npc.image, location.x, location.y);
        
        npc
    }
    
    /// Create ItemObject from ObjectItem packet
    /// C# Reference: Client/MirScenes/GameScene.cs ObjectItem handler
    pub fn create_item(packet: &ObjectItem) -> ItemObject {
        let mut item = ItemObject::new(packet.object_id);
        
        // Set location
        let location = Point::new(packet.location_x, packet.location_y);
        item.map_object.set_location(location);
        item.map_object.set_current_location(location);
        item.map_object.set_map_location(location);
        
        // Set item data
        item.item = packet.item.clone();
        
        // Start pickup effect
        item.draw_effect = true;
        item.effect_time = get_current_time() + 5000; // 5 seconds effect
        
        tracing::debug!("Created ItemObject: id={}, item_index={}, location=({}, {})",
            packet.object_id, item.item.item_index, location.x, location.y);
        
        item
    }
    
    /// Create ItemObject for gold from ObjectGold packet
    /// C# Reference: Client/MirScenes/GameScene.cs ObjectGold handler
    pub fn create_gold(packet: &ObjectGold) -> ItemObject {
        let mut item = ItemObject::new(packet.object_id);
        
        // Set location
        let location = Point::new(packet.location_x, packet.location_y);
        item.map_object.set_location(location);
        item.map_object.set_current_location(location);
        item.map_object.set_map_location(location);
        
        // Set gold amount
        item.gold_amount = packet.gold;
        
        // Start pickup effect
        item.draw_effect = true;
        item.effect_time = get_current_time() + 5000;
        
        tracing::debug!("Created Gold ItemObject: id={}, amount={}, location=({}, {})",
            packet.object_id, packet.gold, location.x, location.y);
        
        item
    }
    
    /// Create PlayerObject from ObjectPlayer packet (for other players)
    /// C# Reference: Client/MirScenes/GameScene.cs ObjectPlayer handler
    pub fn create_player(packet: &ObjectPlayer) -> UserObject {
        // Create PlayerObject with basic info
        let player = PlayerObject::new(
            packet.object_id,
            packet.name.clone(),
            packet.class,
            packet.gender
        );
        
        // Create UserObject wrapper
        let mut user = UserObject::new_from_player(player);
        
        // Set location
        let location = Point::new(packet.location_x, packet.location_y);
        user.player.map_object.set_location(location);
        user.player.map_object.set_current_location(location);
        user.player.map_object.set_map_location(location);
        
        // 🔧 CRITICAL FIX: Initialize movement FSM with player location
        user.movement_fsm = PlayerMovementFSM::new(location);
        
        // Set visual properties
        user.player.map_object.set_direction(packet.direction);
        user.player.map_object.set_light(packet.light as i32);  // u8 -> i32
        user.player.map_object.set_name_colour_argb(packet.name_colour);
        
        // Set player appearance
        user.player.hair = packet.hair;
        user.player.level = packet.level;
        
        // Set equipment appearance
        user.player.armour = packet.armour as i32;  // i16 -> i32
        user.player.weapon = packet.weapon as i32;  // i16 -> i32
        user.player.weapon_effect = packet.weapon_effect as i32;  // i16 -> i32
        
        // Set state
        user.player.map_object.set_dead(packet.dead);
        user.player.map_object.set_poison(packet.poison);
        user.player.map_object.set_hidden(packet.hidden);
        
        // Set buffs
        if !packet.buffs.is_empty() {
            for buff in &packet.buffs {
                user.player.map_object.add_buff(*buff);
            }
        }
        
        tracing::debug!("Created PlayerObject: id={}, name='{}', class={:?}, level={}, location=({}, {})",
            packet.object_id, packet.name, packet.class, packet.level, location.x, location.y);
        
        user
    }
    
    /// Create HeroObject from ObjectHero packet
    /// C# Reference: Client/MirScenes/GameScene.cs ObjectHero handler
    pub fn create_hero(packet: &ObjectHero) -> HeroObject {
        // ObjectHero contains a nested ObjectPlayer
        let player_packet = &packet.player;
        
        // Create PlayerObject base
        let player = PlayerObject::new(
            player_packet.object_id,
            player_packet.name.clone(),
            player_packet.class,
            player_packet.gender
        );
        
        // Create HeroObject
        let mut hero = HeroObject::new_from_player(player, packet.owner_name.clone());
        
        // Set location
        let location = Point::new(player_packet.location_x, player_packet.location_y);
        hero.player.map_object.set_location(location);
        hero.player.map_object.set_current_location(location);
        hero.player.map_object.set_map_location(location);
        
        // Set visual properties
        hero.player.map_object.set_direction(player_packet.direction);
        hero.player.map_object.set_light(player_packet.light as i32);  // u8 -> i32
        hero.player.map_object.set_name_colour_argb(player_packet.name_colour);
        
        // Set hero appearance
        hero.player.hair = player_packet.hair;
        hero.player.level = player_packet.level;
        hero.player.armour = player_packet.armour as i32;  // i16 -> i32
        hero.player.weapon = player_packet.weapon as i32;  // i16 -> i32
        hero.player.weapon_effect = player_packet.weapon_effect as i32;  // i16 -> i32
        
        // Set state
        hero.player.map_object.set_dead(player_packet.dead);
        hero.player.map_object.set_poison(player_packet.poison);
        hero.player.map_object.set_hidden(player_packet.hidden);
        
        // Set buffs
        if !player_packet.buffs.is_empty() {
            for buff in &player_packet.buffs {
                hero.player.map_object.add_buff(*buff);
            }
        }
        
        tracing::debug!("Created HeroObject: id={}, name='{}', owner='{}', location=({}, {})",
            player_packet.object_id, player_packet.name, packet.owner_name, location.x, location.y);
        
        hero
    }
}

/// Get current time in milliseconds
fn get_current_time() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use mir2_shared::enums::{MirDirection, PoisonType, BuffType};

    #[test]
    fn test_create_monster() {
        let packet = ObjectMonster {
            object_id: 1,
            name: "TestMonster".to_string(),
            name_colour: 0xFFFFFF,
            location_x: 100,
            location_y: 200,
            image: 10,
            direction: MirDirection::Up,
            effect: 0,
            ai: 1,
            light: 5,
            dead: false,
            skeleton: false,
            poison: PoisonType::NONE,
            hidden: false,
            shock_time: 0,
            binding_shot_center: false,
            extra: false,
            extra_byte: 0,
            buffs: vec![],
        };
        
        let monster = ObjectFactory::create_monster(&packet);
        assert_eq!(monster.map_object.object_id(), 1);
        assert_eq!(monster.map_object.name, "TestMonster");
        assert_eq!(monster.map_object.location().x, 100);
        assert_eq!(monster.map_object.location().y, 200);
    }

    #[test]
    fn test_create_npc() {
        let packet = ObjectNpc {
            object_id: 2,
            name: "TestNPC".to_string(),
            name_colour: 0xFFFF00,
            location_x: 50,
            location_y: 75,
            image: 5,
            colour: 0,
            direction: MirDirection::Down,
        };
        
        let npc = ObjectFactory::create_npc(&packet);
        assert_eq!(npc.map_object.object_id(), 2);
        assert_eq!(npc.map_object.name, "TestNPC");
    }

    #[test]
    fn test_create_gold() {
        let packet = ObjectGold {
            object_id: 3,
            gold: 500,
            location_x: 10,
            location_y: 20,
        };
        
        let item = ObjectFactory::create_gold(&packet);
        assert_eq!(item.map_object.object_id(), 3);
        assert_eq!(item.gold_amount, 500);
        assert!(item.is_gold());
    }
}
