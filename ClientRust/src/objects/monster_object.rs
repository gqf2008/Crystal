// MonsterObject.rs - Monster/NPC enemy object
// Mirrors Client/MirObjects/MonsterObject.cs

use mir2_shared::{
    enums::{MirDirection, SpellEffect},
    Point,
};

use super::map_object::MapObject;
use crate::network::protocol::ObjectMonster;

/// Monster image enum - corresponds to different monster graphics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Monster {
    Guard = 0,
    TaoistGuard = 1,
    Guard2 = 2,
    Hen = 3,
    Deer = 4,
    Scarecrow = 5,
    HookingCat = 6,
    RakingCat = 7,
    Yob = 8,
    Oma = 9,
    // ... 添加所有怪物类型 (C# 中有数百种)
    // 这里只列出几个示例
    EvilMir = 999,
    PalaceWall1 = 1000,
    PalaceWall2 = 1001,
    PalaceWallLeft = 1002,
    GiGateSouth = 1003,
    GiGateWest = 1004,
    GiGateEast = 1005,
    SSabukWall1 = 1006,
    SSabukWall2 = 1007,
    SSabukWall3 = 1008,
}

impl Monster {
    pub fn from_u16(value: u16) -> Self {
        // TODO: 完整的转换逻辑
        unsafe { std::mem::transmute(value) }
    }

    /// Get manual location offset for special monsters (walls, gates, etc.)
    pub fn manual_location_offset(&self) -> Point {
        match self {
            Monster::EvilMir => Point::new(-21, -15),
            Monster::PalaceWall2
            | Monster::PalaceWallLeft
            | Monster::PalaceWall1
            | Monster::GiGateSouth
            | Monster::GiGateWest
            | Monster::SSabukWall1
            | Monster::SSabukWall2
            | Monster::SSabukWall3 => Point::new(-10, 0),
            Monster::GiGateEast => Point::new(-45, 7),
            _ => Point::new(0, 0),
        }
    }
}

/// Frame set for monster animations
#[derive(Debug, Clone)]
pub struct FrameSet {
    pub standing: Vec<usize>,
    pub walking: Vec<usize>,
    pub attacking: Vec<usize>,
    pub struck: Vec<usize>,
    pub dying: Vec<usize>,
    pub dead: Vec<usize>,
}

impl Default for FrameSet {
    fn default() -> Self {
        Self {
            standing: vec![0],
            walking: vec![],
            attacking: vec![],
            struck: vec![],
            dying: vec![],
            dead: vec![],
        }
    }
}

/// Monster object - represents enemies in the game
#[derive(Debug, Clone)]
pub struct MonsterObject {
    // Inherited from MapObject
    pub map_object: MapObject,
    
    // Monster specific fields
    pub base_image: Monster,
    pub effect: u8,
    pub skeleton: bool,
    
    // Animation
    pub frames: FrameSet,
    pub frame_index: i32,
    pub frame_interval: i32,
    pub effect_frame_index: i32,
    
    // Combat
    pub target_id: u32,
    pub target_point: Point,
    
    // Special states
    pub stoned: bool,
    pub stage: u8,
    pub base_sound: i32,
    
    // Effects
    pub shock_time: i64,
    pub binding_shot_center: bool,
    
    // Visual
    pub old_name_color: u32,
    pub current_effect: SpellEffect,
}

impl MonsterObject {
    /// Create a new monster object
    pub fn new(object_id: u32) -> Self {
        Self {
            map_object: MapObject::new_monster(object_id),
            base_image: Monster::Guard,
            effect: 0,
            skeleton: false,
            frames: FrameSet::default(),
            frame_index: 0,
            frame_interval: 0,
            effect_frame_index: 0,
            target_id: 0,
            target_point: Point::new(0, 0),
            stoned: false,
            stage: 0,
            base_sound: 0,
            shock_time: 0,
            binding_shot_center: false,
            old_name_color: 0xFFFFFFFF,
            current_effect: SpellEffect::None,
        }
    }

    /// Load monster information from server
    pub fn load(&mut self, info: &ObjectMonster, _update: bool) {
        self.map_object.set_name(info.name.clone());
        self.map_object.set_name_colour_argb(info.name_colour);
        self.base_image = Monster::from_u16(info.image);
        
        self.old_name_color = info.name_colour as u32;
        
        let location = Point::new(info.location_x, info.location_y);
        self.map_object.set_location(location);
        
        // Don't add to map if updating
        // if !update {
        //     GameScene::Scene.MapControl.AddObject(self);
        // }
        
        self.effect = info.effect;
        self.map_object.set_ai(info.ai);
        self.map_object.set_light(info.light);
        
        self.map_object.set_direction(info.direction);
        self.map_object.set_dead(info.dead);
        self.map_object.set_poison(info.poison);
        self.skeleton = info.skeleton;
        self.map_object.set_hidden(info.hidden);
        
        // TODO: Calculate shock time properly
        // self.shock_time = CMain::Time + info.shock_time;
        self.binding_shot_center = info.binding_shot_center;
        
        self.map_object.set_buffs(info.buffs.clone());
        
        // Handle stage changes for transforming monsters
        if self.stage != info.extra_byte {
            match self.base_image {
                Monster::EvilMir => {
                    // TODO: Handle GreatFoxSpirit stage changes
                }
                _ => {}
            }
            self.stage = info.extra_byte;
        }
        
        // TODO: Set frames based on base_image
        // self.set_frames();
    }

    /// Check if monster is blocking (can't walk through)
    pub fn is_blocking(&self) -> bool {
        // AI 64 = non-blocking, AI 81 with direction 6 = non-blocking
        if self.map_object.ai() == 64 {
            return false;
        }
        if self.map_object.ai() == 81 && self.map_object.direction() == MirDirection::Down {
            // Direction 6 in C# maps to a specific direction
            return false;
        }
        !self.map_object.is_dead()
    }

    /// Get location offset for rendering
    pub fn get_location_offset(&self) -> Point {
        self.base_image.manual_location_offset()
    }

    /// Update monster animation
    pub fn update_frame(&mut self, _delta_time: f32) {
        // TODO: Implement frame animation logic
        // This should update frame_index based on current action
    }

    /// Check if monster is special (wall, gate, etc.)
    pub fn is_special(&self) -> bool {
        matches!(
            self.base_image,
            Monster::PalaceWall1
                | Monster::PalaceWall2
                | Monster::PalaceWallLeft
                | Monster::GiGateSouth
                | Monster::GiGateWest
                | Monster::GiGateEast
                | Monster::SSabukWall1
                | Monster::SSabukWall2
                | Monster::SSabukWall3
        )
    }

    /// Play monster sound
    pub fn play_sound(&self, sound_type: MonsterSoundType) {
        // TODO: Implement sound playing
        let _sound_index = match sound_type {
            MonsterSoundType::Attack => self.base_sound,
            MonsterSoundType::Struck => self.base_sound + 1,
            MonsterSoundType::Die => self.base_sound + 2,
        };
        // SoundManager::PlaySound(sound_index);
    }

    /// Check if shocked (stunned)
    pub fn is_shocked(&self) -> bool {
        // TODO: Get current time and compare with shock_time
        // CMain::Time < self.shock_time
        false
    }
}

/// Monster sound types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterSoundType {
    Attack,
    Struck,
    Die,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monster_object_creation() {
        let monster = MonsterObject::new(1);
        assert_eq!(monster.map_object.object_id(), 1);
        assert!(!monster.skeleton);
    }

    #[test]
    fn test_monster_location_offset() {
        let offset = Monster::EvilMir.manual_location_offset();
        assert_eq!(offset.x, -21);
        assert_eq!(offset.y, -15);
    }

    #[test]
    fn test_monster_blocking() {
        let mut monster = MonsterObject::new(1);
        assert!(monster.is_blocking());
        
        monster.map_object.set_dead(true);
        assert!(!monster.is_blocking());
    }
}
