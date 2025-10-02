// ItemObject.rs - Ground item object
// Mirrors Client/MirObjects/ItemObject.cs

use mir2_shared::{Point, UserItem};

use super::map_object::MapObject;
use crate::network::network::protocol::ObjectItem;

/// Item object - represents items on the ground
#[derive(Debug, Clone)]
pub struct ItemObject {
    // Inherited from MapObject  
    pub map_object: MapObject,
    
    // Item specific fields
    pub item: UserItem,
    pub gold_amount: u32,
    
    // Visual
    pub draw_effect: bool,
    pub effect_index: i32,
    pub effect_time: i64,
    
    // Pickup
    pub owner_name: Option<String>,
    pub owner_expire_time: i64,
}

impl ItemObject {
    /// Create a new item object
    pub fn new(object_id: u32) -> Self {
        Self {
            map_object: MapObject::new_item(object_id),
            item: UserItem {
                unique_id: 0,
                item_index: 0,
                ac: 0,
                mac: 0,
                dc: 0,
                mc: 0,
                sc: 0,
                hp: 0,
                mp: 0,
                attack_speed: 0,
                accuracy: 0,
                agility: 0,
                luck: 0,
                current_dura: 0,
                max_dura: 0,
                count: 1,
                awake: 0,
                slots: Vec::new(),
                expire_time: None,
                flags: 0,
                weight: 0,
                appearance: 0,
            },
            gold_amount: 0,
            draw_effect: false,
            effect_index: 0,
            effect_time: 0,
            owner_name: None,
            owner_expire_time: 0,
        }
    }

    /// Load item information from server
    pub fn load(&mut self, info: &ObjectItem) {
        self.map_object.current_location = info.location;
        self.map_object.map_location = info.location;
        
        self.item = info.item.clone();
        self.gold_amount = info.gold;
        
        // TODO: Add to game scene map control
        // GameScene::Scene.MapControl.AddObject(self);
        
        // Start effect animation
        self.draw_effect = true;
        self.effect_time = get_current_time() + 5000; // Effect lasts 5 seconds
    }

    /// Check if item can be picked up by player
    pub fn can_pickup(&self, player_name: &str, current_time: i64) -> bool {
        // If no owner, anyone can pick up
        if self.owner_name.is_none() {
            return true;
        }
        
        // If owned, check if owner or time expired
        if let Some(ref owner) = self.owner_name {
            if owner == player_name {
                return true;
            }
            if current_time > self.owner_expire_time {
                return true;
            }
        }
        
        false
    }

    /// Check if item is gold
    pub fn is_gold(&self) -> bool {
        self.gold_amount > 0
    }

    /// Update item visual effects
    pub fn update_effect(&mut self, current_time: i64) {
        if self.draw_effect && current_time > self.effect_time {
            self.draw_effect = false;
        }
    }

    /// Get item name for display
    pub fn get_display_name(&self) -> String {
        if self.is_gold() {
            format!("{} Gold", self.gold_amount)
        } else {
            // TODO: Get item name from item database
            format!("Item {}", self.item.item_index)
        }
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

    #[test]
    fn test_item_object_creation() {
        let item = ItemObject::new(1);
        assert_eq!(item.map_object.object_id, 1);
        assert!(!item.is_gold());
    }

    #[test]
    fn test_gold_item() {
        let mut item = ItemObject::new(1);
        item.gold_amount = 100;
        assert!(item.is_gold());
        assert_eq!(item.get_display_name(), "100 Gold");
    }

    #[test]
    fn test_can_pickup() {
        let mut item = ItemObject::new(1);
        let current_time = get_current_time();
        
        // No owner, can pickup
        assert!(item.can_pickup("Player1", current_time));
        
        // Set owner
        item.owner_name = Some("Player1".to_string());
        item.owner_expire_time = current_time + 10000;
        
        // Owner can pickup
        assert!(item.can_pickup("Player1", current_time));
        
        // Others can't pickup yet
        assert!(!item.can_pickup("Player2", current_time));
        
        // After expiry, others can pickup
        assert!(item.can_pickup("Player2", current_time + 10001));
    }
}
