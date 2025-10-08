// ItemObject.rs - Ground item object
// Mirrors Client/MirObjects/ItemObject.cs

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;
use mir2_shared::{Point, UserItem,packets::*};

use super::map_object::MapObject;
use super::drawable::DrawableMapObject;

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
            map_object: MapObject::for_monster(object_id, String::new()), // Items don't need full MapObject
            item: UserItem::default(),
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
        let location = Point::new(info.location_x, info.location_y);
        self.map_object.set_location(location);
        
        self.item = info.item.clone();
        // Note: ObjectItem doesn't have gold field, use ObjectGold packet instead
        // self.gold_amount = info.gold;
        
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

// Implement DrawableMapObject trait for ItemObject
impl DrawableMapObject for ItemObject {
    fn draw(&self, ctx: &mut Context, canvas: &mut Canvas, draw_location: Point) -> GameResult {
        // C# Reference: Client/MirObjects/ItemObject.rs Draw() method
        // Items are rendered as ground items with optional pickup effect
        
        use ggez::graphics::{Color, DrawParam, Text, PxScale};
        use ggez::mint::Point2;
        
        // Calculate actual draw position
        let x = draw_location.x as f32;
        let y = draw_location.y as f32;
        let draw_pos = Point2 { x, y };
        
        // TODO: Get item texture from Items library
        // For now, draw a placeholder shape
        if self.is_gold() {
            // Draw gold (yellow circle)
            let circle = ggez::graphics::Mesh::new_circle(
                ctx,
                ggez::graphics::DrawMode::fill(),
                Point2 { x: 0.0, y: 0.0 },
                8.0,
                0.1,
                Color::from_rgb(255, 215, 0), // Gold color
            )?;
            
            canvas.draw(&circle, DrawParam::default().dest(draw_pos));
            
            // Draw gold amount text
            if self.gold_amount > 0 {
                let mut text = Text::new(format!("{}", self.gold_amount));
                text.set_scale(PxScale::from(10.0));
                
                let text_pos = Point2 {
                    x: x - 8.0,
                    y: y - 16.0, // Above the gold
                };
                
                canvas.draw(&text, DrawParam::default()
                    .dest(text_pos)
                    .color(Color::from_rgb(255, 255, 0))
                );
            }
        } else {
            // Draw regular item (cyan rectangle for visibility)
            let rect = ggez::graphics::Mesh::new_rectangle(
                ctx,
                ggez::graphics::DrawMode::fill(),
                ggez::graphics::Rect::new(0.0, 0.0, 16.0, 16.0),
                Color::from_rgb(0, 200, 255), // Cyan color
            )?;
            
            canvas.draw(&rect, DrawParam::default().dest(draw_pos));
        }
        
        tracing::trace!("Drawing ItemObject {} at ({}, {})", 
            self.map_object.object_id(), draw_location.x, draw_location.y);
        
        Ok(())
        
        /* TODO: Full implementation with texture loading
        use crate::graphics::MLibrary;
        
        // 1. Get texture from Items library
        let texture = libraries::get_item_texture(ctx, self.item.item_index)?;
        
        // 2. Calculate draw position with offset
        let offset = libraries::get_item_offset(self.item.item_index);
        let final_pos = Point2 {
            x: (draw_location.x + offset.x) as f32,
            y: (draw_location.y + offset.y) as f32,
        };
        
        // 3. Draw sprite
        canvas.draw(&texture, DrawParam::default().dest(final_pos));
        
        // 4. Apply pickup effect (flashing)
        if self.draw_effect {
            let time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as f32;
            let alpha = ((time / 300.0).sin() * 0.3 + 0.7) * 255.0;
            canvas.draw(&texture, DrawParam::default()
                .dest(final_pos)
                .color(Color::from_rgba(255, 255, 255, alpha as u8))
            );
        }
        */
    }
    
    fn object_id(&self) -> u32 {
        self.map_object.object_id()
    }
    
    fn is_dead(&self) -> bool {
        false // Items don't die
    }
    
    fn is_hidden(&self) -> bool {
        self.map_object.is_hidden()
    }
    
    fn draw_priority(&self) -> i32 {
        0 // Items draw first (behind everything)
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
        assert_eq!(item.map_object.object_id(), 1);
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
