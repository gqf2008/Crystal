// GameScene - Main game scene (核心游戏场景)
// Mirrors Client/MirScenes/GameScene.cs (12,297 lines)

use super::{Scene, SceneType};
use crate::objects::*;
use crate::network::game_client::GameEvent;
use mir2_shared::UserItem;
use mir2_shared::enums::{AttackMode, PetMode, LightSetting};
use std::collections::{HashMap, VecDeque};

/// Output message (chat/system messages)
#[derive(Debug, Clone)]
pub struct OutputMessage {
    pub text: String,
    pub color: (u8, u8, u8), // RGB
    pub timestamp: i64,
}

/// Main game scene
#[derive(Debug)]
pub struct GameScene {
    // Player and hero
    pub user: Option<UserObject>,
    pub hero: Option<HeroObject>,
    
    // Game objects
    pub objects: HashMap<u32, Box<dyn std::fmt::Debug>>, // All game objects by ID
    pub monsters: HashMap<u32, MonsterObject>,
    pub npcs: HashMap<u32, NPCObject>,
    pub items: HashMap<u32, ItemObject>,
    pub players: HashMap<u32, UserObject>,
    pub spells: Vec<SpellObject>,
    pub effects: Vec<Effect>,
    pub damages: Vec<Damage>,
    
    // Game state
    pub gold: u32,
    pub credit: u32,
    pub attack_mode: AttackMode,
    pub pet_mode: PetMode,
    pub lights: LightSetting,
    
    // Storage
    pub storage: Vec<Option<UserItem>>,         // 80 slots
    pub guild_storage: Vec<Option<UserItem>>,   // 112 slots
    pub refine_storage: Vec<Option<UserItem>>,  // 16 slots
    pub hero_storage: [Option<HeroObject>; 8],  // 8 hero slots
    
    // Item interaction
    pub hover_item: Option<UserItem>,
    pub selected_item: Option<UserItem>,
    pub picked_up_gold: bool,
    
    // Timing
    pub move_time: i64,
    pub attack_time: i64,
    pub spell_time: i64,
    pub pickup_time: i64,
    pub use_item_time: i64,
    
    // Flags
    pub can_move: bool,
    pub can_run: bool,
    pub observing: bool,
    pub allow_observe: bool,
    
    // Output messages (chat log)
    pub output_messages: VecDeque<OutputMessage>,
    pub max_output_messages: usize,
    
    // Map info
    pub map_info: HashMap<i32, String>, // map_id -> map_name
    pub current_map_index: i32,
    
    // Dialogs (TODO: implement when dialog system ready)
    // pub main_dialog: MainDialog,
    // pub chat_dialog: ChatDialog,
    // pub inventory_dialog: InventoryDialog,
    // ... 40+ dialogs
    
    // Pathfinding
    pub pathfinder: Option<PathFinder>,
}

impl GameScene {
    /// Create new game scene
    pub fn new() -> Self {
        Self {
            user: None,
            hero: None,
            objects: HashMap::new(),
            monsters: HashMap::new(),
            npcs: HashMap::new(),
            items: HashMap::new(),
            players: HashMap::new(),
            spells: Vec::new(),
            effects: Vec::new(),
            damages: Vec::new(),
            gold: 0,
            credit: 0,
            attack_mode: AttackMode::Peace,
            pet_mode: PetMode::Both,
            lights: LightSetting::Normal,
            storage: vec![None; 80],
            guild_storage: vec![None; 112],
            refine_storage: vec![None; 16],
            hero_storage: [None, None, None, None, None, None, None, None],
            hover_item: None,
            selected_item: None,
            picked_up_gold: false,
            move_time: 0,
            attack_time: 0,
            spell_time: 0,
            pickup_time: 0,
            use_item_time: 0,
            can_move: true,
            can_run: true,
            observing: false,
            allow_observe: false,
            output_messages: VecDeque::new(),
            max_output_messages: 100,
            map_info: HashMap::new(),
            current_map_index: 0,
            pathfinder: None,
        }
    }
    
    /// Add output message to chat log
    pub fn add_output_message(&mut self, text: String, color: (u8, u8, u8)) {
        let message = OutputMessage {
            text,
            color,
            timestamp: get_current_time(),
        };
        
        self.output_messages.push_back(message);
        
        // Limit message count
        while self.output_messages.len() > self.max_output_messages {
            self.output_messages.pop_front();
        }
    }
    
    /// Add game object
    pub fn add_monster(&mut self, monster: MonsterObject) {
        let id = monster.map_object.object_id();
        self.monsters.insert(id, monster);
    }
    
    pub fn add_npc(&mut self, npc: NPCObject) {
        let id = npc.map_object.object_id();
        self.npcs.insert(id, npc);
    }
    
    pub fn add_item(&mut self, item: ItemObject) {
        let id = item.map_object.object_id();
        self.items.insert(id, item);
    }
    
    pub fn add_player(&mut self, player: UserObject) {
        let id = player.player.map_object.object_id();
        self.players.insert(id, player);
    }
    
    /// Remove object by ID
    pub fn remove_object(&mut self, object_id: u32) {
        self.monsters.remove(&object_id);
        self.npcs.remove(&object_id);
        self.items.remove(&object_id);
        self.players.remove(&object_id);
        self.spells.retain(|s| s.map_object.object_id() != object_id);
    }
    
    /// Get current time in milliseconds
    fn get_time(&self) -> i64 {
        get_current_time()
    }
    
    /// Update all game objects
    pub fn update_objects(&mut self, delta_time: f32) {
        let current_time = self.get_time();
        
        // Update user
        if let Some(ref mut user) = self.user {
            // TODO: user.update(delta_time);
            let _ = user;
        }
        
        // Update hero
        if let Some(ref mut hero) = self.hero {
            hero.update_loyalty(delta_time);
        }
        
        // Update monsters
        for monster in self.monsters.values_mut() {
            // TODO: monster.update(delta_time);
            let _ = monster;
        }
        
        // Update spells
        for spell in self.spells.iter_mut() {
            spell.update_position(current_time);
        }
        
        // Remove expired spells
        self.spells.retain(|s| !s.should_remove(current_time));
        
        // Update effects
        for effect in self.effects.iter_mut() {
            effect.update(current_time);
        }
        
        // Remove finished effects
        self.effects.retain(|e| !e.is_finished());
        
        // Update damage numbers
        for damage in self.damages.iter_mut() {
            damage.update(current_time, delta_time);
        }
        
        // Remove finished damage numbers
        self.damages.retain(|d| !d.is_finished());
    }
    
    /// Change attack mode
    pub fn set_attack_mode(&mut self, mode: AttackMode) {
        self.attack_mode = mode;
        self.add_output_message(
            format!("Attack mode: {:?}", mode),
            (255, 255, 0),
        );
    }
    
    /// Change pet mode
    pub fn set_pet_mode(&mut self, mode: PetMode) {
        self.pet_mode = mode;
        self.add_output_message(
            format!("Pet mode: {:?}", mode),
            (255, 255, 0),
        );
    }
    
    /// Pick up item
    pub fn pickup_item(&mut self, item_id: u32) {
        let current_time = self.get_time();
        
        // Check pickup cooldown
        if current_time < self.pickup_time {
            return;
        }
        
        // TODO: Send pickup packet to server
        println!("Picking up item {}", item_id);
        
        self.pickup_time = current_time + 200; // 200ms cooldown
    }
    
    /// Use item from inventory
    pub fn use_item(&mut self, slot: usize) {
        let current_time = self.get_time();
        
        // Check use item cooldown
        if current_time < self.use_item_time {
            return;
        }
        
        // TODO: Check if item exists in slot
        // TODO: Send use item packet to server
        println!("Using item in slot {}", slot);
        
        self.use_item_time = current_time + 300; // 300ms cooldown
    }
}

impl Default for GameScene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene for GameScene {
    fn scene_type(&self) -> SceneType {
        SceneType::Game
    }
    
    fn initialize(&mut self) {
        println!("GameScene::initialize");
        // TODO: Initialize pathfinder
        // TODO: Load UI dialogs
        // TODO: Request initial game state from server
    }
    
    fn update(&mut self, delta_time: f32) {
        // Update all game objects
        self.update_objects(delta_time);
        
        // TODO: Update camera
        // TODO: Update map
    }
    
    fn draw(&self) {
        // TODO: Draw map
        // TODO: Draw game objects (sorted by Y position)
        // TODO: Draw effects
        // TODO: Draw damage numbers
        // TODO: Draw UI dialogs
    }
    
    /// Process game events from GameClient
    fn process_event(&mut self, event: &GameEvent) {
        match event {
            GameEvent::PlayerSpawned { player } => {
                println!("Player spawned: {}", player.name);
                // TODO: Create UserObject from PlayerState
                // For now, just log
            }
            
            GameEvent::PlayerMoved { location } => {
                if self.user.is_some() {
                    // TODO: Update user position using public API
                    println!("Player moved to: ({}, {})", location.x, location.y);
                }
            }
            
            GameEvent::ObjectSpawned { object } => {
                match object {
                    crate::network::game_client::GameObject::Player { id, name, .. } => {
                        println!("Player spawned: {} ({})", name, id);
                    }
                    crate::network::game_client::GameObject::Monster { id, name, .. } => {
                        println!("Monster spawned: {} ({})", name, id);
                    }
                    crate::network::game_client::GameObject::Npc { id, name, .. } => {
                        println!("NPC spawned: {} ({})", name, id);
                    }
                    crate::network::game_client::GameObject::Item { id, .. } => {
                        println!("Item spawned: {}", id);
                    }
                }
                // TODO: Create appropriate object type (Monster/NPC/Item/Player)
            }
            
            GameEvent::ObjectRemoved { object_id } => {
                println!("Removing object {}", object_id);
                self.remove_object(*object_id);
            }
            
            GameEvent::ChatReceived { message } => {
                self.add_output_message(
                    message.text.clone(),
                    (255, 255, 255),
                );
            }
            
            GameEvent::GoldChanged { gold } => {
                self.gold = *gold;
            }
            
            GameEvent::SystemMessage { message } => {
                self.add_output_message(
                    message.clone(),
                    (255, 255, 0),
                );
            }
            
            GameEvent::ItemGained { item, grid_type } => {
                println!("Item gained: {:?} in {}", item, grid_type);
                // TODO: Add to appropriate inventory
            }
            
            GameEvent::MagicCast { spell, target_id } => {
                println!("Magic cast: {:?} on target {}", spell, target_id);
                // TODO: Create spell effect
            }
            
            _ => {
                // TODO: Handle other game events
            }
        }
    }
    
    fn handle_mouse_move(&mut self, _x: i32, _y: i32) {
        // TODO: Update hover states
        // TODO: Update cursor
    }
    
    fn handle_mouse_button(&mut self, button: winit::event::MouseButton, pressed: bool, x: i32, y: i32) {
        use winit::event::MouseButton;
        
        if pressed {
            tracing::debug!("GameScene click at ({}, {}) with {:?}", x, y, button);
            
            match button {
                MouseButton::Left => {
                    // TODO: Handle left click (move, attack, interact)
                }
                MouseButton::Right => {
                    // TODO: Handle right click (pickup item)
                }
                MouseButton::Middle => {
                    // TODO: Handle middle click
                }
                _ => {}
            }
        }
    }
    
    fn handle_key_press(&mut self, key: winit::keyboard::KeyCode, modifiers: winit::keyboard::ModifiersState) -> bool {
        use winit::keyboard::KeyCode;
        
        match key {
            // Movement keys handled separately
            KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::ArrowLeft | KeyCode::ArrowRight => {
                // TODO: Handle movement
                true
            }
            
            // Attack mode (Ctrl+H)
            KeyCode::KeyH if modifiers.control_key() => {
                // TODO: Cycle through attack modes
                true
            }
            
            // Pet mode (Ctrl+E)
            KeyCode::KeyE if modifiers.control_key() => {
                // TODO: Cycle through pet modes
                true
            }
            
            // Inventory (Tab)
            KeyCode::Tab => {
                // TODO: Toggle inventory
                true
            }
            
            // Character (C)
            KeyCode::KeyC => {
                // TODO: Toggle character dialog
                true
            }
            
            // Skills (S)
            KeyCode::KeyS => {
                // TODO: Toggle skills dialog
                true
            }
            
            // Quest (Q)
            KeyCode::KeyQ => {
                // TODO: Toggle quest dialog
                true
            }
            
            // Guild (G)
            KeyCode::KeyG => {
                // TODO: Toggle guild dialog
                true
            }
            
            _ => false
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
    fn test_game_scene_creation() {
        let scene = GameScene::new();
        assert_eq!(scene.scene_type(), SceneType::Game);
        assert_eq!(scene.gold, 0);
        assert_eq!(scene.storage.len(), 80);
        assert_eq!(scene.guild_storage.len(), 112);
    }

    #[test]
    fn test_add_output_message() {
        let mut scene = GameScene::new();
        
        scene.add_output_message("Test message".to_string(), (255, 255, 255));
        assert_eq!(scene.output_messages.len(), 1);
        
        // Test message limit
        scene.max_output_messages = 3;
        scene.add_output_message("Message 2".to_string(), (255, 255, 255));
        scene.add_output_message("Message 3".to_string(), (255, 255, 255));
        scene.add_output_message("Message 4".to_string(), (255, 255, 255));
        
        assert_eq!(scene.output_messages.len(), 3);
        assert_eq!(scene.output_messages[0].text, "Message 2");
    }

    #[test]
    fn test_attack_pet_modes() {
        let mut scene = GameScene::new();
        
        scene.set_attack_mode(AttackMode::All);
        assert_eq!(scene.attack_mode, AttackMode::All);
        
        scene.set_pet_mode(PetMode::AttackOnly);
        assert_eq!(scene.pet_mode, PetMode::AttackOnly);
    }
}
