// NPCObject.rs - Non-player character object
// Mirrors Client/MirObjects/NPCObject.cs

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;
use mir2_shared::{Point,packets::*};

use super::map_object::MapObject;
use super::drawable::DrawableMapObject;
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

// Implement DrawableMapObject trait for NPCObject
impl DrawableMapObject for NPCObject {
    fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas, draw_location: Point) -> GameResult {
        // C# Reference: Client/MirObjects/NPCObject.cs Draw() method
        // NPCs are static entities with direction-based sprites
        
        // TODO Phase 2: Implement actual NPC rendering
        // Real implementation needs:
        // 1. Get NPC texture from NPC library (based on self.image)
        // 2. Get frame based on direction (self.map_object.direction())
        // 3. Calculate screen position with proper offset
        // 4. Draw NPC sprite
        // 5. Draw name label above NPC (using self.map_object.name)
        // 6. Apply name color (self.map_object.name_colour)
        
        tracing::trace!("Drawing NPCObject {} '{}' at ({}, {})", 
            self.map_object.object_id(), 
            self.map_object.name,
            draw_location.x, draw_location.y);
        
        Ok(())
    }
    
    fn object_id(&self) -> u32 {
        self.map_object.object_id()
    }
    
    fn is_dead(&self) -> bool {
        false // NPCs don't die
    }
    
    fn is_hidden(&self) -> bool {
        self.map_object.is_hidden()
    }
    
    fn draw_priority(&self) -> i32 {
        2 // NPCs draw after items and spells
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
