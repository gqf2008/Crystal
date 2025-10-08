// GameScene - Main game scene (核心游戏场景)
// Mirrors Client/MirScenes/GameScene.cs (12,297 lines)
pub mod map_control;
// pub mod map_loader; // 已删除 - 使用 objects::MapReader 代替
pub mod tile_texture_manager;

use super::{Scene, SceneType};
use crate::objects::*;
use crate::objects::ObjectFactory; // Object creation from server packets
use crate::network::game_client::GameEvent;
use mir2_shared::{UserItem, Point};
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
pub struct GameScene {
    // Player and hero
    pub user: Option<UserObject>,
    pub hero: Option<HeroObject>,
    
    // Game objects - unified drawable object map for rendering
    pub objects: HashMap<u32, Box<dyn DrawableMapObject>>, // All drawable objects by ID
    
    // Separate collections for specific object types (for game logic)
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
    pub map_control: Option<map_control::MapControl>,
    
    // Rendering
    pub tile_texture_manager: std::cell::RefCell<crate::scenes::game_scene::tile_texture_manager::TileTextureManager>,
    pub player_x: i32,
    pub player_y: i32,
    pub animation_count: u32, // C#: AnimationCount (for tile animations)
    
    // Dialogs (TODO: implement when dialog system ready)
    // pub main_dialog: MainDialog,
    // pub chat_dialog: ChatDialog,
    // pub inventory_dialog: InventoryDialog,
    // ... 40+ dialogs
    
    // Pathfinding
    pub pathfinder: Option<PathFinder>,
}

impl GameScene {
    /// Load map file by name (searches in Map and Data/Map directories)
    fn load_map_file(map_name: &str) -> std::io::Result<map_control::MapControl> {
        use std::path::PathBuf;
        
        // Try different paths - prioritize ClientRust/Map first
        let paths = [
            PathBuf::from(format!("Map/{}.map", map_name)),         // ClientRust/Map
            PathBuf::from(format!("./Map/{}.map", map_name)),
            PathBuf::from(format!("Data/Map/{}.map", map_name)),
            PathBuf::from(format!("./Data/Map/{}.map", map_name)),
            PathBuf::from(format!("../Data/Map/{}.map", map_name)),
            PathBuf::from(format!("../../Data/Map/{}.map", map_name)),
        ];
        
        for path in &paths {
            if path.exists() {
                tracing::debug!("📂 Found map file at: {}", path.display());
                match MapReader::new(path.to_str().unwrap()) {
                    Ok(reader) => {
                        tracing::info!("✅ MapReader loaded: {}x{}", reader.width, reader.height);
                        return Ok(map_control::MapControl::from_map_reader(reader));
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to parse map {}: {}", path.display(), e);
                        return Err(e);
                    }
                }
            }
        }
        
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Map file not found: {}", map_name),
        ))
    }
    
    /// Create new game scene
    pub fn new() -> Self {
        // 🧪 临时: 尝试加载测试地图 "0"
        let test_map = Self::load_map_file("0").ok();
        let (initial_x, initial_y) = if let Some(ref map) = test_map {
            let center_x = map.width / 2;
            let center_y = map.height / 2;
            // 确保是偶数坐标
            let x = if center_x % 2 == 0 { center_x } else { center_x + 1 };
            let y = if center_y % 2 == 0 { center_y } else { center_y + 1 };
            tracing::info!("🧪 Loaded test map '0': {}x{}, player at ({}, {})", map.width, map.height, x, y);
            (x, y)
        } else {
            tracing::warn!("⚠️  Failed to load test map '0', using default empty position");
            (100, 100) // 默认偶数坐标
        };
        
        // ✅ 注意: initialize() 会由 SceneManager::switch_scene() 调用
        // 不在构造函数中调用,遵循Scene trait的两阶段初始化模式
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
            map_control: test_map, // 🧪 使用测试地图
            tile_texture_manager: std::cell::RefCell::new(
                crate::scenes::game_scene::tile_texture_manager::TileTextureManager::new()
            ),
            player_x: initial_x,
            player_y: initial_y,
            animation_count: 0,
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
    
    /// Add game object (adds to both objects map and cell, plus specific collection)
    pub fn add_monster(&mut self, monster: MonsterObject) {
        let id = monster.map_object.object_id();
        let location = monster.map_object.location();
        
        // Add to drawable objects map
        self.objects.insert(id, Box::new(monster.clone()));
        
        // Add to monsters collection
        self.monsters.insert(id, monster);
        
        // Add to cell's object list
        if let Some(ref mut map) = self.map_control {
            if let Some(cell) = map.get_cell_mut(location.x, location.y) {
                cell.add_object(id);
            }
        }
    }
    
    pub fn add_npc(&mut self, npc: NPCObject) {
        let id = npc.map_object.object_id();
        let location = npc.map_object.location();
        
        // Add to drawable objects map
        self.objects.insert(id, Box::new(npc.clone()));
        
        // Add to NPCs collection
        self.npcs.insert(id, npc);
        
        // Add to cell's object list
        if let Some(ref mut map) = self.map_control {
            if let Some(cell) = map.get_cell_mut(location.x, location.y) {
                cell.add_object(id);
            }
        }
    }
    
    pub fn add_item(&mut self, item: ItemObject) {
        let id = item.map_object.object_id();
        let location = item.map_object.location();
        
        // Add to drawable objects map
        self.objects.insert(id, Box::new(item.clone()));
        
        // Add to items collection
        self.items.insert(id, item);
        
        // Add to cell's object list
        if let Some(ref mut map) = self.map_control {
            if let Some(cell) = map.get_cell_mut(location.x, location.y) {
                cell.add_object(id);
            }
        }
    }
    
    pub fn add_player(&mut self, player: UserObject) {
        let id = player.player.map_object.object_id();
        let location = player.player.map_object.location();
        
        // Add to drawable objects map
        self.objects.insert(id, Box::new(player.clone()));
        
        // Add to players collection
        self.players.insert(id, player);
        
        // Add to cell's object list
        if let Some(ref mut map) = self.map_control {
            if let Some(cell) = map.get_cell_mut(location.x, location.y) {
                cell.add_object(id);
            }
        }
    }
    
    /// Find monster by ID
    pub fn get_monster(&self, object_id: u32) -> Option<&MonsterObject> {
        self.monsters.get(&object_id)
    }
    
    /// Find monster by ID (mutable)
    pub fn get_monster_mut(&mut self, object_id: u32) -> Option<&mut MonsterObject> {
        self.monsters.get_mut(&object_id)
    }
    
    /// Find NPC by ID
    pub fn get_npc(&self, object_id: u32) -> Option<&NPCObject> {
        self.npcs.get(&object_id)
    }
    
    /// Find item by ID
    pub fn get_item(&self, object_id: u32) -> Option<&ItemObject> {
        self.items.get(&object_id)
    }
    
    /// Find player by ID
    pub fn get_player(&self, object_id: u32) -> Option<&UserObject> {
        self.players.get(&object_id)
    }
    
    /// Find player by ID (mutable)
    pub fn get_player_mut(&mut self, object_id: u32) -> Option<&mut UserObject> {
        self.players.get_mut(&object_id)
    }
    
    /// Get all objects at a specific location
    pub fn get_objects_at(&self, location: Point) -> Vec<u32> {
        if let Some(ref map) = self.map_control {
            if let Some(cell) = map.get_cell(location.x, location.y) {
                return cell.cell_objects.clone().unwrap_or_default();
            }
        }
        Vec::new()
    }
    
    /// Find closest monster to a location
    pub fn find_closest_monster(&self, location: Point, max_distance: i32) -> Option<u32> {
        let mut closest_id = None;
        let mut closest_dist = max_distance * max_distance; // Use squared distance
        
        for monster in self.monsters.values() {
            if monster.map_object.is_dead() {
                continue;
            }
            
            let pos = monster.map_object.location();
            let dx = pos.x - location.x;
            let dy = pos.y - location.y;
            let dist_sq = dx * dx + dy * dy;
            
            if dist_sq < closest_dist {
                closest_dist = dist_sq;
                closest_id = Some(monster.map_object.object_id());
            }
        }
        
        closest_id
    }
    
    /// Clear all objects from the scene
    pub fn clear_all_objects(&mut self) {
        tracing::info!("Clearing all objects from scene");
        
        self.objects.clear();
        self.monsters.clear();
        self.npcs.clear();
        self.items.clear();
        self.players.clear();
        self.spells.clear();
        self.effects.clear();
        self.damages.clear();
        
        // Clear cell object lists
        if let Some(ref mut map) = self.map_control {
            for y in 0..map.height {
                for x in 0..map.width {
                    if let Some(cell) = map.get_cell_mut(x, y) {
                        cell.cell_objects = None;
                    }
                }
            }
        }
    }
    
    /// Remove a specific object from the scene
    pub fn remove_object(&mut self, object_id: u32) -> bool {
        let mut removed = false;
        
        // Remove from general objects map
        if self.objects.remove(&object_id).is_some() {
            removed = true;
        }
        
        // Try removing from each specific collection
        if self.monsters.remove(&object_id).is_some() {
            tracing::debug!("Removed monster {}", object_id);
            removed = true;
        }
        
        if self.npcs.remove(&object_id).is_some() {
            tracing::debug!("Removed NPC {}", object_id);
            removed = true;
        }
        
        if self.items.remove(&object_id).is_some() {
            tracing::debug!("Removed item {}", object_id);
            removed = true;
        }
        
        if self.players.remove(&object_id).is_some() {
            tracing::debug!("Removed player {}", object_id);
            removed = true;
        }
        
        // Remove from map cell tracking
        if removed {
            self.remove_object_from_all_cells(object_id);
        }
        
        removed
    }
    
    /// Remove object from all map cells (helper method)
    fn remove_object_from_all_cells(&mut self, object_id: u32) {
        if let Some(ref mut map) = self.map_control {
            for y in 0..map.height {
                for x in 0..map.width {
                    if let Some(cell) = map.get_cell_mut(x, y) {
                        if let Some(ref mut objects) = cell.cell_objects {
                            objects.retain(|&id| id != object_id);
                        }
                    }
                }
            }
        }
    }
    
    /// Remove multiple objects at once
    pub fn remove_objects(&mut self, object_ids: &[u32]) {
        for &object_id in object_ids {
            self.remove_object(object_id);
        }
        tracing::debug!("Removed {} objects", object_ids.len());
    }
    
    /// Remove all dead monsters
    pub fn remove_dead_monsters(&mut self) {
        let dead_ids: Vec<u32> = self.monsters
            .iter()
            .filter(|(_, m)| m.map_object.is_dead())
            .map(|(id, _)| *id)
            .collect();
        
        for id in &dead_ids {
            self.remove_object(*id);
        }
        
        if !dead_ids.is_empty() {
            tracing::debug!("Removed {} dead monsters", dead_ids.len());
        }
    }
    
    /// Remove all items at a specific location
    pub fn remove_items_at(&mut self, location: Point) -> usize {
        let items_to_remove: Vec<u32> = self.items
            .iter()
            .filter(|(_, item)| item.map_object.location() == location)
            .map(|(id, _)| *id)
            .collect();
        
        let count = items_to_remove.len();
        for id in items_to_remove {
            self.remove_object(id);
        }
        
        if count > 0 {
            tracing::debug!("Removed {} items at ({}, {})", count, location.x, location.y);
        }
        
        count
    }
    
    /// Get current time in milliseconds
    fn get_time(&self) -> i64 {
        get_current_time()
    }
    
    /// Update all game objects
    pub fn update_objects(&mut self, delta_time: f32) {
        let current_time = self.get_time();
        let delta_ms = (delta_time * 1000.0) as u32;
        
        // Update user
        if let Some(ref mut user) = self.user {
            // Update movement
            if user.player.map_object.update_movement(delta_time) {
                user.player.map_object.update_draw_location();
            }
            
            // Update animation
            user.player.map_object.advance(delta_ms);
        }
        
        // Update hero
        if let Some(ref mut hero) = self.hero {
            hero.update_loyalty(delta_time);
            
            // Update movement
            if hero.player.map_object.update_movement(delta_time) {
                hero.player.map_object.update_draw_location();
            }
            
            // Update animation
            hero.player.map_object.advance(delta_ms);
        }
        
        // Update monsters
        for monster in self.monsters.values_mut() {
            // Update movement
            if monster.map_object.update_movement(delta_time) {
                monster.map_object.update_draw_location();
            }
            
            // Update animation
            monster.map_object.advance(delta_ms);
        }
        
        // Update NPCs (usually stationary, but animate)
        for npc in self.npcs.values_mut() {
            npc.map_object.advance(delta_ms);
        }
        
        // Update players
        for player in self.players.values_mut() {
            // Update movement
            if player.player.map_object.update_movement(delta_time) {
                player.player.map_object.update_draw_location();
            }
            
            // Update animation
            player.player.map_object.advance(delta_ms);
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
    
    /// Handle server packets for object spawning
    /// This uses ObjectFactory to create game objects from network packets
    pub fn handle_server_object_packets(&mut self) {
        // TODO: 当网络层完善后,从这里接收并处理ObjectMonster/ObjectNpc等包
        // 示例:
        // 
        // use mir2_shared::packets::server::{ObjectMonster, ObjectNpc, ObjectPlayer};
        // 
        // match server_packet {
        //     ServerPacket::ObjectMonster(packet) => {
        //         let monster = ObjectFactory::create_monster(&packet);
        //         self.add_monster(monster);
        //     }
        //     ServerPacket::ObjectNpc(packet) => {
        //         let npc = ObjectFactory::create_npc(&packet);
        //         self.add_npc(npc);
        //     }
        //     ServerPacket::ObjectPlayer(packet) => {
        //         let player = ObjectFactory::create_player(&packet);
        //         self.add_player(player);
        //     }
        //     ServerPacket::ObjectItem(packet) => {
        //         let item = ObjectFactory::create_item(&packet);
        //         self.add_item(item);
        //     }
        //     ServerPacket::ObjectGold(packet) => {
        //         let gold = ObjectFactory::create_gold(&packet);
        //         self.add_item(gold);
        //     }
        //     ServerPacket::ObjectHero(packet) => {
        //         let hero = ObjectFactory::create_hero(&packet);
        //         self.hero = Some(hero);
        //     }
        //     _ => {}
        // }
    }
    
    /// 🧪 Create test player character
    fn create_test_player(&mut self) {
        use mir2_shared::packets::server::ObjectPlayer;
        use mir2_shared::enums::{MirClass, MirGender, MirDirection, PoisonType};
        
        tracing::info!("🧪 Creating test player character...");
        
        // Create test player packet
        let player_packet = ObjectPlayer {
            object_id: 1,
            name: "TestPlayer".to_string(),
            guild_name: String::new(),
            guild_rank_name: String::new(),
            name_colour: 0xFFFFFF,
            class: MirClass::Warrior,
            gender: MirGender::Male,
            level: 1,
            location_x: self.player_x,
            location_y: self.player_y,
            direction: MirDirection::Down,
            hair: 0,
            light: 3,
            weapon: -1,
            weapon_effect: 0,
            armour: -1,
            poison: PoisonType::NONE,
            dead: false,
            hidden: false,
            effect: mir2_shared::enums::SpellEffect::None,
            wing_effect: 0,
            extra: false,
            mount_type: -1,
            riding_mount: false,
            fishing: false,
            transform_type: -1,
            element_orb_effect: 0,
            element_orb_lvl: 0,
            element_orb_max: 0,
            buffs: vec![],
            level_effects: mir2_shared::enums::LevelEffects::NONE,
        };
        
        // Create player using ObjectFactory
        let user = ObjectFactory::create_player(&player_packet);
        
        let player_id = user.player.map_object.object_id();
        let player_location = user.player.map_object.location();
        
        tracing::info!("✅ Created test player: id={}, name='{}', pos=({}, {}), class={:?}", 
            player_id,
            user.player.map_object.name,
            player_location.x,
            player_location.y,
            user.player.class
        );
        
        // 🔧 关键: 将玩家添加到drawable objects HashMap
        self.objects.insert(player_id, Box::new(user.clone()));
        
        // 🔧 关键: 将玩家添加到地图单元格 (用于渲染)
        if let Some(ref mut map) = self.map_control {
            if let Some(cell) = map.get_cell_mut(player_location.x, player_location.y) {
                cell.add_object(player_id);
                tracing::info!("✅ Added player to cell ({}, {})", player_location.x, player_location.y);
            } else {
                tracing::warn!("⚠️  Failed to get cell at ({}, {})", player_location.x, player_location.y);
            }
        }
        
        self.user = Some(user);
        
        tracing::info!("🎉 Test player setup complete! Total objects in scene: {}", self.objects.len());
    }
    
    /// 🧪 Create test objects using ObjectFactory
    /// This demonstrates how to use the object factory system
    fn create_test_objects(&mut self) {
        use mir2_shared::packets::server::{ObjectMonster, ObjectNpc, ObjectGold};
        use mir2_shared::enums::{MirDirection, PoisonType};
        
        tracing::info!("🧪 Creating test objects using ObjectFactory...");
        
        // 创建测试NPC (使用ObjectFactory)
        let npc_packet = ObjectNpc {
            object_id: 1001,
            name: "Test Guard".to_string(),
            name_colour: 0xFFFFFF,
            image: 10,
            colour: 0,
            location_x: self.player_x + 2,
            location_y: self.player_y + 2,
            direction: MirDirection::Down,
        };
        
        let npc = ObjectFactory::create_npc(&npc_packet);
        tracing::info!("✅ Created NPC via factory: id={}, name='{}', pos=({}, {})", 
            npc.map_object.object_id(), 
            npc.map_object.name,
            npc.map_object.location().x,
            npc.map_object.location().y
        );
        self.add_npc(npc);
        
        // 创建测试怪物 (使用ObjectFactory)
        let monster_packet = ObjectMonster {
            object_id: 1002,
            name: "Test Monster".to_string(),
            name_colour: 0xFF0000,
            location_x: self.player_x - 3,
            location_y: self.player_y + 1,
            image: 5,
            direction: MirDirection::Right,
            effect: 0,
            ai: 0,
            light: 3,
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
        
        let monster = ObjectFactory::create_monster(&monster_packet);
        tracing::info!("✅ Created Monster via factory: id={}, name='{}', pos=({}, {})", 
            monster.map_object.object_id(), 
            monster.map_object.name,
            monster.map_object.location().x,
            monster.map_object.location().y
        );
        self.add_monster(monster);
        
        // 创建测试金币 (使用ObjectFactory)
        let gold_packet = ObjectGold {
            object_id: 1003,
            gold: 1000,
            location_x: self.player_x + 1,
            location_y: self.player_y - 2,
        };
        
        let gold = ObjectFactory::create_gold(&gold_packet);
        tracing::info!("✅ Created Gold via factory: id={}, amount={}, pos=({}, {})", 
            gold.map_object.object_id(), 
            gold.gold_amount,
            gold.map_object.location().x,
            gold.map_object.location().y
        );
        self.add_item(gold);
        
        tracing::info!("🎉 Test objects created: {} total objects, {} monsters, {} npcs, {} items", 
            self.objects.len(), 
            self.monsters.len(), 
            self.npcs.len(), 
            self.items.len()
        );
    }
    
    /// Move an object to a new location (with interpolation)
    pub fn move_object(&mut self, object_id: u32, target: Point) -> bool {
        // Try to find and move the object in each collection
        if let Some(monster) = self.monsters.get_mut(&object_id) {
            let old_location = monster.map_object.location();
            monster.map_object.start_move(target);
            
            // Update cell tracking
            self.update_object_cell(object_id, old_location, target);
            
            tracing::debug!("Moving monster {} from ({},{}) to ({},{})", 
                object_id, old_location.x, old_location.y, target.x, target.y);
            return true;
        }
        
        if let Some(player) = self.players.get_mut(&object_id) {
            let old_location = player.player.map_object.location();
            player.player.map_object.start_move(target);
            self.update_object_cell(object_id, old_location, target);
            return true;
        }
        
        if let Some(ref mut user) = self.user {
            if user.player.map_object.object_id() == object_id {
                user.player.map_object.start_move(target);
                // Update player position
                self.player_x = target.x;
                self.player_y = target.y;
                return true;
            }
        }
        
        if let Some(ref mut hero) = self.hero {
            if hero.player.map_object.object_id() == object_id {
                let old_location = hero.player.map_object.location();
                hero.player.map_object.start_move(target);
                self.update_object_cell(object_id, old_location, target);
                return true;
            }
        }
        
        false
    }
    
    /// Teleport an object instantly (no interpolation)
    pub fn teleport_object(&mut self, object_id: u32, target: Point) -> bool {
        if let Some(monster) = self.monsters.get_mut(&object_id) {
            let old_location = monster.map_object.location();
            monster.map_object.teleport_to(target);
            self.update_object_cell(object_id, old_location, target);
            return true;
        }
        
        if let Some(player) = self.players.get_mut(&object_id) {
            let old_location = player.player.map_object.location();
            player.player.map_object.teleport_to(target);
            self.update_object_cell(object_id, old_location, target);
            return true;
        }
        
        false
    }
    
    /// Update object's cell tracking when it moves
    fn update_object_cell(&mut self, object_id: u32, old_location: Point, new_location: Point) {
        if old_location == new_location {
            return;
        }
        
        if let Some(ref mut map) = self.map_control {
            // Remove from old cell
            if let Some(old_cell) = map.get_cell_mut(old_location.x, old_location.y) {
                old_cell.remove_object(object_id);
            }
            
            // Add to new cell
            if let Some(new_cell) = map.get_cell_mut(new_location.x, new_location.y) {
                new_cell.add_object(object_id);
            }
        }
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
    
    /// Draw map with 3 layers (Back, Middle, Front)
    fn draw_map(&self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas, ggez_manager: &mut crate::graphics::GgezManager, map: &map_control::MapControl) {
        // Get tile manager
        let mut tile_manager = self.tile_texture_manager.borrow_mut();
        
        // Map tile dimensions (matches C# CellWidth/CellHeight)
        const TILE_WIDTH: f32 = 48.0;
        const TILE_HEIGHT: f32 = 32.0;
        
        // 使用实际窗口大小
        let screen_width = ctx.gfx.drawable_size().0;
        let screen_height = ctx.gfx.drawable_size().1;
        
        // Screen offset in tiles (C# formula: OffSetX = ScreenWidth / 2 / CellWidth)
        let offset_x_tiles = (screen_width / 2.0 / TILE_WIDTH) as i32;
        let offset_y_tiles = (screen_height / 2.0 / TILE_HEIGHT) as i32 - 1;
        
        // Calculate visible tile range (C#: ViewRangeX = OffSetX + 6)
        let view_range_x = offset_x_tiles + 6;
        let view_range_y = offset_y_tiles + 6;
        
        // C#允许循环从负数开始,在循环内部检查边界!
        // 不要用.max(0)裁剪,这会改变遍历顺序导致坐标错位
        let start_x = self.player_x - view_range_x;
        let start_y = self.player_y - view_range_y;
        let end_x = self.player_x + view_range_x;
        let end_y = self.player_y + view_range_y;
        
        tracing::debug!("🎨 Drawing map: player=({}, {}), range=({},{} to {},{})", 
            self.player_x, self.player_y, start_x, start_y, end_x, end_y);
        
        use ggez::graphics::DrawParam;
        
        let mut drawn_tiles = 0;
        
        // ========== LAYER 1: BackImage (Ground) ==========
        let mut checked_cells = 0;
        let mut back_image_cells = 0;
        let mut texture_found = 0;
        for y in start_y..=end_y {  // C# uses <=, so use ..= in Rust
            if y <= 0 || y % 2 == 1 { continue; } // Skip y<=0 or odd rows (matches C#)
            if y >= map.height { break; }  // C#: if (y >= Height) break;
            for x in start_x..=end_x {  // C# uses <=, so use ..= in Rust
                if x <= 0 || x % 2 == 1 { continue; } // Skip x<=0 or odd columns (matches C#)
                if x >= map.width { break; }  // C#: if (x >= Width) break;
                
                if let Some(cell) = map.get_cell(x, y) {
                    checked_cells += 1;
                    if cell.back_image > 0 && cell.back_index != -1 {
                        back_image_cells += 1;
                        // C#: index = (M2CellInfo[x, y].BackImage & 0x1FFFFFFF) - 1;
                        let image_index = ((cell.back_image & 0x1FFFFFFF) - 1) as u16;
                        if back_image_cells == 1 {
                            tracing::debug!("🎨 First BackImage cell: ({}, {}), back_image=0x{:X} (raw), masked_index={}, back_index={}", 
                                x, y, cell.back_image, image_index, cell.back_index);
                        }
                        
                        // C# formula (EXACT):
                        // drawX = (x - User.Movement.X + OffSetX) * CellWidth - OffSetX + User.OffSetMove.X
                        // drawY = (y - User.Movement.Y + OffSetY) * CellHeight + User.OffSetMove.Y
                        // 注意: 第二个OffSetX是tile数(非像素),作为微调值!
                        let draw_x = ((x - self.player_x + offset_x_tiles) as f32 * TILE_WIDTH 
                            - offset_x_tiles as f32) + 0.0; // User.OffSetMove.X (平滑移动偏移,暂时为0)
                        let draw_y = ((y - self.player_y + offset_y_tiles) as f32 * TILE_HEIGHT) 
                            + 0.0; // User.OffSetMove.Y (平滑移动偏移,暂时为0)
                        
                        if let Some(texture) = tile_manager.get_tile_texture(ctx, cell.back_index as i32, image_index, ggez_manager) {
                            texture_found += 1;
                            if texture_found == 1 {
                                tracing::debug!("🎨 First texture: {} (width={}, height={}, offset=({}, {}))", 
                                    texture.texture_name, texture.width, texture.height, 
                                    texture.offset_x, texture.offset_y);
                            }
                            if let Some(image) = ggez_manager.get_texture(&texture.texture_name) {
                                
                                // C#: Draw(index, drawX, drawY) - 不应用offset,直接绘制
                                // ggez坐标系与DirectX相同:左上角(0,0),Y轴向下
                                if drawn_tiles == 0 {
                                    tracing::debug!("🎨 First tile: cell({},{}), draw_pos=({:.1},{:.1}), tex_size=({}x{}), tex_offset=({},{})", 
                                        x, y, draw_x, draw_y, texture.width, texture.height, texture.offset_x, texture.offset_y);
                                }
                                
                                canvas.draw(
                                    image,
                                    DrawParam::default()
                                        .dest([draw_x, draw_y])
                                );
                                
                                drawn_tiles += 1;
                            }
                        }
                    }
                }
            }
        }
        
        // ========== LAYER 2: MiddleImage (Decorations) ==========
        // Note: Middle layer does NOT skip odd coordinates (unlike BackImage)
        for y in start_y..=(end_y + 5) {  // C#: y <= User.Movement.Y + ViewRangeY + 5
            if y <= 0 { continue; } // Skip y<=0 (matches C#)
            if y >= map.height { break; }  // C#: boundary check
            for x in start_x..=end_x {  // C# uses <=
                if x < 0 { continue; } // Skip x<0 (matches C#)
                if x >= map.width { break; }  // C#: boundary check
                if let Some(cell) = map.get_cell(x, y) {
                    if cell.middle_image > 0 && cell.middle_index != -1 {
                        // C#: index = M2CellInfo[x, y].MiddleImage - 1; (no mask)
                        let image_index = (cell.middle_image - 1) as u16;
                        
                        // C# formula (EXACT):
                        // drawY = (y - User.Movement.Y + OffSetY) * CellHeight + User.OffSetMove.Y;
                        let draw_y = ((y - self.player_y + offset_y_tiles) as f32 * TILE_HEIGHT) + 0.0;
                        // drawX = (x - User.Movement.X + OffSetX) * CellWidth - OffSetX + User.OffSetMove.X;
                        let draw_x = ((x - self.player_x + offset_x_tiles) as f32 * TILE_WIDTH 
                            - offset_x_tiles as f32) + 0.0;
                        
                        if let Some(texture) = tile_manager.get_tile_texture(ctx, cell.middle_index as i32, image_index, ggez_manager) {
                            // C# size filtering:
                            // if ((s.Width != CellWidth || s.Height != CellHeight) &&
                            //     ((s.Width != CellWidth * 2) || (s.Height != CellHeight * 2))) continue;
                            let w = texture.width as i32;
                            let h = texture.height as i32;
                            let valid_size = (w == TILE_WIDTH as i32 && h == TILE_HEIGHT as i32) ||
                                           (w == TILE_WIDTH as i32 * 2 && h == TILE_HEIGHT as i32 * 2);
                            
                            if valid_size {
                                if let Some(image) = ggez_manager.get_texture(&texture.texture_name) {
                                    // C#: Draw(index, drawX, drawY)
                                    canvas.draw(
                                        image,
                                        DrawParam::default()
                                            .dest([draw_x, draw_y])
                                    );
                                    
                                    drawn_tiles += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // ========== LAYER 3: FrontImage (Standard Tiles in DrawFloor) ==========
        // C# DrawFloor: Only draws standard size tiles (48x32 or 96x64)
        for y in start_y..=(end_y + 5) {  // C#: y <= User.Movement.Y + ViewRangeY + 5
            if y <= 0 { continue; } // Skip y<=0 (matches C#)
            if y >= map.height { break; }  // C#: boundary check
            for x in start_x..=end_x {  // C# uses <=
                if x < 0 { continue; } // Skip x<0 (matches C#)
                if x >= map.width { break; }  // C#: boundary check
                if let Some(cell) = map.get_cell(x, y) {
                    if cell.front_image > 0 && cell.front_index != -1 && cell.front_index != 200 {
                        let image_index = (cell.front_image & 0x7FFF) - 1;
                        
                        if image_index >= 0 {
                            // C# formula (EXACT):
                            // drawY = (y - User.Movement.Y + OffSetY) * CellHeight + User.OffSetMove.Y;
                            let draw_y = ((y - self.player_y + offset_y_tiles) as f32 * TILE_HEIGHT) + 0.0;
                            // drawX = (x - User.Movement.X + OffSetX) * CellWidth - OffSetX + User.OffSetMove.X;
                            let draw_x = ((x - self.player_x + offset_x_tiles) as f32 * TILE_WIDTH 
                                - offset_x_tiles as f32) + 0.0;
                            
                            if let Some(texture) = tile_manager.get_tile_texture(ctx, cell.front_index as i32, image_index as u16, ggez_manager) {
                                // C# DrawFloor Front layer: only standard sizes
                                // if (index < 0 || ((s.Width != CellWidth || s.Height != CellHeight) &&
                                //     ((s.Width != CellWidth * 2) || (s.Height != CellHeight * 2)))) continue;
                                let w = texture.width as i32;
                                let h = texture.height as i32;
                                let is_standard_size = (w == TILE_WIDTH as i32 && h == TILE_HEIGHT as i32) ||
                                                      (w == TILE_WIDTH as i32 * 2 && h == TILE_HEIGHT as i32 * 2);
                                
                                if is_standard_size {
                                    if let Some(image) = ggez_manager.get_texture(&texture.texture_name) {
                                        // C#: Draw(index, drawX, drawY)
                                        canvas.draw(
                                            image,
                                            DrawParam::default()
                                                .dest([draw_x, draw_y])
                                        );
                                        
                                        drawn_tiles += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // ========== LAYER 4: FrontImage Large Objects (Buildings, Trees) ==========
        // C# DrawObjects: Draws non-standard size objects with bottom alignment (drawY - s.Height)
        let mut layer4_checked = 0;
        let mut layer4_drawn = 0;
        let mut layer4_negative_index = 0;
        let mut layer4_texture_fail = 0;
        let mut layer4_standard_size = 0;
        
        for y in start_y..=(end_y + 25) {  // C#: y <= User.Movement.Y + ViewRangeY + 25 (larger range for tall objects)
            if y <= 0 { continue; }
            if y >= map.height { break; }
            
            // C# DrawObjects formula: drawY = (y - User.Movement.Y + OffSetY + 1) * CellHeight + User.OffSetMove.Y
            let draw_y = (y - self.player_y + offset_y_tiles + 1) as f32 * TILE_HEIGHT + 0.0;
            
            for x in start_x..=end_x {
                if x < 0 { continue; }
                if x >= map.width { break; }
                
                if let Some(cell) = map.get_cell(x, y) {
                    if cell.front_image > 0 && cell.front_index != -1 && cell.front_index != 200 {
                        layer4_checked += 1;
                        let image_index = (cell.front_image & 0x7FFF) - 1;
                        
                        if image_index >= 0 {
                            let draw_x = ((x - self.player_x + offset_x_tiles) as f32 * TILE_WIDTH 
                                - offset_x_tiles as f32) + 0.0;
                            
                            if let Some(texture) = tile_manager.get_tile_texture(ctx, cell.front_index as i32, image_index as u16, ggez_manager) {
                                let w = texture.width as i32;
                                let h = texture.height as i32;
                                let is_standard_size = (w == TILE_WIDTH as i32 && h == TILE_HEIGHT as i32) ||
                                                      (w == TILE_WIDTH as i32 * 2 && h == TILE_HEIGHT as i32 * 2);
                                
                                // C# DrawObjects Front: Skip standard sizes, draw large objects with bottom alignment
                                // if (s.Width == CellWidth && s.Height == CellHeight && animation == 0) continue;
                                // if ((s.Width == CellWidth * 2) && (s.Height == CellHeight * 2) && (animation == 0)) continue;
                                // Libraries.MapLibs[fileIndex].Draw(index, drawX, drawY - s.Height);
                                if !is_standard_size {
                                    if let Some(image) = ggez_manager.get_texture(&texture.texture_name) {
                                        // Bottom alignment: drawY - s.Height
                                        let aligned_y = draw_y - texture.height as f32;
                                        
                                        if layer4_drawn == 0 {
                                            tracing::debug!("🎨 Layer 4 first draw: cell({},{}), size={}x{}, index={}, draw_y={:.1}, aligned_y={:.1}", 
                                                x, y, w, h, image_index, draw_y, aligned_y);
                                        }
                                        
                                        canvas.draw(
                                            image,
                                            DrawParam::default()
                                                .dest([draw_x, aligned_y])
                                        );
                                        
                                        drawn_tiles += 1;
                                        layer4_drawn += 1;
                                    }
                                } else {
                                    layer4_standard_size += 1;
                                }
                            } else {
                                layer4_texture_fail += 1;
                            }
                        } else {
                            layer4_negative_index += 1;
                        }
                    }
                }
            }
        }
        
        if layer4_checked > 0 {
            tracing::debug!("🎨 Layer 4 stats: checked={}, drawn={}, neg_index={}, tex_fail={}, standard_size={}", 
                layer4_checked, layer4_drawn, layer4_negative_index, layer4_texture_fail, layer4_standard_size);
        }
        
        // ========== LAYER 5: TileAnimationImage (Animated Tiles) ==========
        // C# DrawObjects: Shanda's tile animation layer using library 190
        for y in start_y..=(end_y + 25) {
            if y <= 0 { continue; }
            if y >= map.height { break; }
            
            // C#: drawY = (y - User.Movement.Y + OffSetY + 1) * CellHeight + User.OffSetMove.Y
            let draw_y = ((y - self.player_y + offset_y_tiles + 1) as f32 * TILE_HEIGHT) + 0.0;
            
            for x in start_x..=end_x {
                if x < 0 { continue; }
                if x >= map.width { break; }
                let draw_x = ((x - self.player_x + offset_x_tiles) as f32 * TILE_WIDTH 
                    - offset_x_tiles as f32) + 0.0;
                
                if let Some(cell) = map.get_cell(x, y) {
                    // C#: index = M2CellInfo[x, y].TileAnimationImage;
                    // C#: animation = M2CellInfo[x, y].TileAnimationFrames;
                    let mut index = cell.tile_animation_image;
                    let animation = cell.tile_animation_frames;
                    
                    if index > 0 && animation > 0 {
                        // C#: index--;
                        // C#: int animationoffset = M2CellInfo[x, y].TileAnimationOffset ^ 0x2000;
                        // C#: index += animationoffset * (AnimationCount % animation);
                        index -= 1;
                        let animation_offset = cell.tile_animation_offset ^ 0x2000;
                        index += animation_offset * ((self.animation_count % animation as u32) as i16);
                        
                        // C#: Libraries.MapLibs[190].DrawUp(index, drawX, drawY);
                        // DrawUp means: y -= height (bottom alignment)
                        if let Some(texture) = tile_manager.get_tile_texture(ctx, 190, index as u16, ggez_manager) {
                            if let Some(image) = ggez_manager.get_texture(&texture.texture_name) {
                                let aligned_y = draw_y - texture.height as f32;
                                canvas.draw(
                                    image,
                                    DrawParam::default()
                                        .dest([draw_x, aligned_y])
                                );
                                drawn_tiles += 1;
                            }
                        }
                    }
                }
            }
        }
        
        // ========== LAYER 6: MiddleImage Animated (Mir3 middle layer with animations) ==========
        // C# DrawObjects: Middle layer with animation support (different from DrawFloor Middle)
        for y in start_y..=(end_y + 25) {
            if y <= 0 { continue; }
            if y >= map.height { break; }
            
            let draw_y = ((y - self.player_y + offset_y_tiles + 1) as f32 * TILE_HEIGHT) + 0.0;
            
            for x in start_x..=end_x {
                if x < 0 { continue; }
                if x >= map.width { break; }
                let draw_x = ((x - self.player_x + offset_x_tiles) as f32 * TILE_WIDTH 
                    - offset_x_tiles as f32) + 0.0;
                
                if let Some(cell) = map.get_cell(x, y) {
                    // C#: if ((M2CellInfo[x, y].MiddleIndex >= 0) && (M2CellInfo[x, y].MiddleIndex != -1))
                    if cell.middle_index >= 0 && cell.middle_index != -1 {
                        // C#: index = M2CellInfo[x, y].MiddleImage - 1;
                        let mut index = cell.middle_image - 1;
                        
                        if index > 0 {
                            // C#: animation = M2CellInfo[x, y].MiddleAnimationFrame;
                            let mut animation = cell.middle_animation_frame;
                            let mut blend = false;
                            
                            // C#: if ((animation > 0) && (animation < 255))
                            if animation > 0 && animation < 255 {
                                // C#: if ((animation & 0x0f) > 0) { blend = true; animation &= 0x0f; }
                                if (animation & 0x0f) > 0 {
                                    blend = true;
                                    animation &= 0x0f;
                                }
                                
                                if animation > 0 {
                                    // C#: byte animationTick = M2CellInfo[x, y].MiddleAnimationTick;
                                    // C#: index += (AnimationCount % (animation + (animation * animationTick))) / (1 + animationTick);
                                    let animation_tick = cell.middle_animation_tick;
                                    let anim_total = animation as u32 + (animation as u32 * animation_tick as u32);
                                    index += ((self.animation_count % anim_total) / (1 + animation_tick as u32)) as i32;
                                    
                                    // C#: if (blend && (animation == 10 || animation == 8)) DrawUpBlend else DrawUp
                                    if let Some(texture) = tile_manager.get_tile_texture(ctx, cell.middle_index as i32, index as u16, ggez_manager) {
                                        if let Some(image) = ggez_manager.get_texture(&texture.texture_name) {
                                            let aligned_y = draw_y - texture.height as f32;
                                            canvas.draw(
                                                image,
                                                DrawParam::default()
                                                    .dest([draw_x, aligned_y])
                                                    // TODO: Add blend mode for blend=true cases
                                            );
                                            drawn_tiles += 1;
                                        }
                                    }
                                }
                            }
                            
                            // C#: s = Libraries.MapLibs[M2CellInfo[x, y].MiddleIndex].GetSize(index);
                            // C#: if ((s.Width != CellWidth || s.Height != CellHeight) && 
                            //         (s.Width != (CellWidth * 2) || s.Height != (CellHeight * 2)) && !blend)
                            //     Libraries.MapLibs[M2CellInfo[x, y].MiddleIndex].DrawUp(index, drawX, drawY);
                            if let Some(texture) = tile_manager.get_tile_texture(ctx, cell.middle_index as i32, index as u16, ggez_manager) {
                                let w = texture.width as i32;
                                let h = texture.height as i32;
                                let is_standard_size = (w == TILE_WIDTH as i32 && h == TILE_HEIGHT as i32) ||
                                                      (w == TILE_WIDTH as i32 * 2 && h == TILE_HEIGHT as i32 * 2);
                                
                                if !is_standard_size && !blend {
                                    if let Some(image) = ggez_manager.get_texture(&texture.texture_name) {
                                        let aligned_y = draw_y - texture.height as f32;
                                        canvas.draw(
                                            image,
                                            DrawParam::default()
                                                .dest([draw_x, aligned_y])
                                        );
                                        drawn_tiles += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // ========== OBJECT RENDERING LAYER ==========
        // C# DrawObjects: Iterate cells and call M2CellInfo[x,y].DrawObjects()
        // This renders all game objects (monsters, NPCs, players, items, spells)
        let mut objects_drawn = 0;
        let mut cells_with_objects = 0;
        
        for y in start_y..=(end_y + 25) {
            if y <= 0 { continue; }
            if y >= map.height { break; }
            
            for x in start_x..=end_x {
                if x < 0 { continue; }
                if x >= map.width { break; }
                
                if let Some(cell) = map.get_cell(x, y) {
                    // Skip cells without objects
                    if cell.cell_objects.is_none() {
                        continue;
                    }
                    
                    cells_with_objects += 1;
                    
                    // Calculate draw location for this cell
                    // C#: drawY = (y - User.Movement.Y + OffSetY + 1) * CellHeight + User.OffSetMove.Y
                    let draw_location = Point::new(
                        (x - self.player_x + offset_x_tiles) * 48 - offset_x_tiles,
                        (y - self.player_y + offset_y_tiles + 1) * 32
                    );
                    
                    // Draw dead objects first (corpses behind live objects)
                    if let Err(e) = cell.draw_dead_objects(ctx, canvas, &self.objects, draw_location) {
                        tracing::warn!("Failed to draw dead objects at ({}, {}): {}", x, y, e);
                    }
                    
                    // Draw live objects
                    if let Err(e) = cell.draw_objects(ctx, canvas, &self.objects, draw_location) {
                        tracing::warn!("Failed to draw objects at ({}, {}): {}", x, y, e);
                    } else {
                        objects_drawn += cell.cell_objects.as_ref().map(|o| o.len()).unwrap_or(0);
                    }
                }
            }
        }
        
        // Debug: Count cells with front/middle/animation data
        let mut front_cells = 0;
        let mut middle_anim_cells = 0;
        let mut tile_anim_cells = 0;
        for y in start_y..=(end_y + 25) {
            if y <= 0 || y >= map.height { continue; }
            for x in start_x..=end_x {
                if x < 0 || x >= map.width { continue; }
                if let Some(cell) = map.get_cell(x, y) {
                    if cell.front_image > 0 && cell.front_index != -1 && cell.front_index != 200 {
                        front_cells += 1;
                    }
                    if cell.middle_index >= 0 && cell.middle_index != -1 && cell.middle_image > 0 && cell.middle_animation_frame > 0 {
                        middle_anim_cells += 1;
                    }
                    if cell.tile_animation_image > 0 && cell.tile_animation_frames > 0 {
                        tile_anim_cells += 1;
                    }
                }
            }
        }
        
        tracing::debug!("🎨 Map draw summary: checked={}, back_image={}, texture_found={}, drawn={}", 
            checked_cells, back_image_cells, texture_found, drawn_tiles);
        tracing::debug!("🎨 Layer data: front_cells={}, middle_anim={}, tile_anim={}", 
            front_cells, middle_anim_cells, tile_anim_cells);
        tracing::debug!("🎨 Objects: cells_with_objects={}, objects_drawn={}", 
            cells_with_objects, objects_drawn);
        
        if drawn_tiles > 0 {
            tracing::trace!("Drew {} tiles (6 layers) + {} objects", drawn_tiles, objects_drawn);
        }
    }
}

impl std::fmt::Debug for GameScene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameScene")
            .field("user", &self.user)
            .field("hero", &self.hero)
            .field("objects_count", &self.objects.len())
            .field("monsters_count", &self.monsters.len())
            .field("npcs_count", &self.npcs.len())
            .field("items_count", &self.items.len())
            .field("players_count", &self.players.len())
            .field("gold", &self.gold)
            .field("current_map_index", &self.current_map_index)
            .finish()
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
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn initialize(&mut self) {
        println!("🎮🎮🎮 GameScene::initialize CALLED! 🎮🎮🎮");
        tracing::error!("🎮🎮🎮 GameScene::initialize CALLED! 🎮🎮🎮");
        
        // Load map tile libraries
        let mut tile_mgr = self.tile_texture_manager.borrow_mut();
        tracing::error!("🔄 About to call load_tiles_libraries()...");
        match tile_mgr.load_tiles_libraries() {
            Ok(count) => {
                tracing::error!("✅✅✅ Loaded {} map tile libraries ✅✅✅", count);
            }
            Err(e) => {
                tracing::error!("❌❌❌ Failed to load map tile libraries: {} ❌❌❌", e);
            }
        }
        drop(tile_mgr);
        
        // 🧪 测试: 创建测试玩家和对象
        if self.map_control.is_some() {
            self.create_test_player();  // 先创建玩家
            self.create_test_objects(); // 再创建其他对象
        }
        
        tracing::error!("🎮 GameScene::initialize COMPLETED!");
        // TODO: Initialize pathfinder
        // TODO: Load UI dialogs
        // TODO: Request initial game state from server
    }
    
    fn update(&mut self, delta_time: f32) {
        // Update animation counter (C#: AnimationCount++)
        self.animation_count = self.animation_count.wrapping_add(1);
        
        // Update all game objects
        self.update_objects(delta_time);
        
        // TODO: Update camera
        // TODO: Update map
    }
    
    fn draw(&self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas, ggez_manager: &mut crate::graphics::GgezManager) {
        use ggez::graphics::{Color, DrawParam, Text, PxScale};
        use ggez::mint::Point2;
        
        // 1. Draw map (if loaded)
        if let Some(map) = &self.map_control {
            self.draw_map(ctx, canvas, ggez_manager, map);
            
            // Display map info (debug - bottom left)
            let map_info_text = format!(
                "Map: {} ({}x{})\nPos: ({}, {})", 
                map.title, map.width, map.height,
                self.player_x, self.player_y
            );
            let mut map_info = Text::new(map_info_text);
            map_info.set_scale(PxScale::from(14.0));
            canvas.draw(
                &map_info,
                DrawParam::default()
                    .dest(Point2 { x: 10.0, y: 700.0 })
                    .color(Color::from_rgba(100, 255, 100, 200)),
            );
        } else {
            // No map loaded - show waiting message
            let mut waiting = Text::new("🗺️  Waiting for map data...");
            waiting.set_scale(PxScale::from(32.0));
            canvas.draw(
                &waiting,
                DrawParam::default()
                    .dest(Point2 { x: 300.0, y: 350.0 })
                    .color(Color::from_rgb(255, 200, 100)),
            );
        }
        
        // 2. Display player info (top right)
        if let Some(user) = &self.user {
            let player_text = format!(
                "Player: {} | Gold: {}", 
                user.player.map_object.name,
                self.gold
            );
            let mut player_info = Text::new(player_text);
            player_info.set_scale(PxScale::from(18.0));
            canvas.draw(
                &player_info,
                DrawParam::default()
                    .dest(Point2 { x: 700.0, y: 10.0 })
                    .color(Color::from_rgb(255, 255, 100)),
            );
        }
        
        // TODO: Draw game objects (sorted by Y position)
        // TODO: Draw effects
        // TODO: Draw damage numbers
        // TODO: Draw UI dialogs
    }
    
    /// Process game events from GameClient
    fn process_event(&mut self, event: &GameEvent) {
        match event {
            GameEvent::PlayerSpawned { player } => {
                tracing::info!("Player spawned: {}", player.name);
                // TODO: Create UserObject from PlayerState
                // For now, just log
            }
            
            GameEvent::PlayerMoved { location } => {
                if let Some(ref mut user) = self.user {
                    // Update user position
                    user.player.map_object.start_move(*location);
                    tracing::debug!("Player moved to: ({}, {})", location.x, location.y);
                }
            }
            
            GameEvent::ObjectSpawned { object } => {
                use crate::network::game_client::GameObject;
                
                match object {
                    GameObject::Player { id, name, .. } => {
                        tracing::info!("Player spawned: {} ({})", name, id);
                        // TODO: 当有完整的ObjectPlayer packet时使用ObjectFactory::create_player
                        // let player = ObjectFactory::create_player(&packet);
                        // self.add_player(player);
                    }
                    
                    GameObject::Monster { id, name, .. } => {
                        tracing::info!("Monster spawned: {} ({})", name, id);
                        // TODO: 当有完整的ObjectMonster packet时使用ObjectFactory::create_monster
                        // let monster = ObjectFactory::create_monster(&packet);
                        // self.add_monster(monster);
                    }
                    
                    GameObject::Npc { id, name, .. } => {
                        tracing::info!("NPC spawned: {} ({})", name, id);
                        // TODO: 当有完整的ObjectNpc packet时使用ObjectFactory::create_npc
                        // let npc = ObjectFactory::create_npc(&packet);
                        // self.add_npc(npc);
                    }
                    
                    GameObject::Item { id, .. } => {
                        tracing::info!("Item spawned: {}", id);
                        // TODO: 当有完整的ObjectItem packet时使用ObjectFactory::create_item
                        // let item = ObjectFactory::create_item(&packet);
                        // self.add_item(item);
                    }
                }
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
            
            GameEvent::MapInformation { map_index, file_name, title } => {
                tracing::info!("🗺️  Loading map: {} ({})", title, file_name);
                self.current_map_index = *map_index;
                self.map_info.insert(*map_index, title.clone());
                
                // Load map file using objects::MapReader
                match Self::load_map_file(file_name) {
                    Ok(mut map) => {
                        map.title = title.clone();
                        map.filename = file_name.clone();
                        tracing::info!("✅ Map loaded: {} ({}x{})", map.title, map.width, map.height);
                        self.map_control = Some(map);
                        
                        // Update player position (center of map for now)
                        // ⚠️ 重要: 坐标必须是偶数 (C# MapControl 只绘制偶数坐标的格子)
                        if self.player_x == 0 && self.player_y == 0 {
                            let center_x = self.map_control.as_ref().unwrap().width / 2;
                            let center_y = self.map_control.as_ref().unwrap().height / 2;
                            // 确保是偶数坐标
                            self.player_x = if center_x % 2 == 0 { center_x } else { center_x + 1 };
                            self.player_y = if center_y % 2 == 0 { center_y } else { center_y + 1 };
                            tracing::info!("📍 Player positioned at even coords: ({}, {})", self.player_x, self.player_y);
                        }
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to load map {}: {}", file_name, e);
                        // Create empty fallback map
                        let mut fallback = map_control::MapControl::new(100, 100);
                        fallback.title = title.clone();
                        fallback.filename = file_name.clone();
                        self.map_control = Some(fallback);
                    }
                }
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
    
    fn handle_mouse_button(&mut self, button: super::MouseButton, pressed: bool, x: i32, y: i32) {
        use super::MouseButton;
        
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
    
    fn handle_key_press(&mut self, key: super::KeyCode, modifiers: super::ModifiersState) -> bool {
        use super::KeyCode;
        
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
