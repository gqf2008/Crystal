// NPCObject.rs - Non-player character object
// Mirrors Client/MirObjects/NPCObject.cs

use mir2_shared::{Point,packets::*};

use super::map_object::MapObject;
/// NPC image types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum NpcImage {
    // Town NPCs
    Guard = 0,
    Merchant = 1,
    Blacksmith = 2,
    Warehouse = 3,
    GuildMaster = 4,
    // ... 添加更多NPC类型
}

impl NpcImage {
    pub fn from_u16(value: u16) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

/// NPC object - represents friendly NPCs in the game
#[derive(Debug, Clone)]
pub struct NPCObject {
    // Inherited from MapObject
    pub map_object: MapObject,
    
    // NPC specific fields
    pub image: NpcImage,
    pub turn_time: i64,
}

impl NPCObject {
    /// Create a new NPC object
    pub fn new(object_id: u32) -> Self {
        Self {
            map_object: MapObject::for_monster(object_id, String::new()),
            image: NpcImage::Guard,
            turn_time: 0,
        }
    }

    /// Load NPC information from server
    pub fn load(&mut self, info: &ObjectNpc) {
        self.map_object.set_name(info.name.clone());
        self.map_object.set_name_colour_argb(info.name_colour);
        self.image = NpcImage::from_u16(info.image);
        
        let location = Point::new(info.location_x, info.location_y);
        self.map_object.set_location(location);
        
        self.map_object.set_direction(info.direction);
        self.map_object.set_light(0); // NPCs typically don't have light
        
        // TODO: Add to game scene map control
        // GameScene::Scene.MapControl.AddObject(self);
    }

    /// Check if NPC is blocking (usually they are)
    pub fn is_blocking(&self) -> bool {
        true
    }

    /// Update NPC turn animation
    pub fn update_turn(&mut self, current_time: i64) {
        if current_time > self.turn_time {
            // TODO: Randomly turn NPC to face different directions
            // This makes NPCs look more alive
            self.turn_time = current_time + 5000; // Turn every 5 seconds
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npc_object_creation() {
        let npc = NPCObject::new(1);
        assert_eq!(npc.map_object.object_id(), 1);
        assert!(npc.is_blocking());
    }
}
