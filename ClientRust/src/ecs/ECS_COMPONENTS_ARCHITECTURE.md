# 熱血傳奇 ECS 實體與組件設計

## 🏗️ 總體組件分類

### 組件模塊結構
```
src/components/
├── mod.rs
├── common.rs        # 通用組件
├── player.rs        # 玩家相關
├── monster.rs       # 怪物相關  
├── npc.rs          # NPC相關
├── combat.rs        # 戰鬥相關
├── animation.rs     # 動畫相關
├── movement.rs      # 移動相關
├── inventory.rs     # 物品相關
├── ui.rs           # UI相關
├── network.rs      # 網絡相關
├── effects.rs      # 特效相關
└── events.rs       # 事件組件
```

## 📋 完整組件詳細表

### 通用組件 (`src/components/common.rs`)
```rust
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub position: Vector2,
    pub rotation: f32,
    pub scale: Vector2,
}

#[derive(Component, Debug, Clone)]
pub struct Velocity {
    pub linear: Vector2,
    pub angular: f32,
}

#[derive(Component, Debug, Clone)]
pub struct Health {
    pub current: i32,
    pub max: i32,
    pub last_damage_time: Option<Instant>,
}

#[derive(Component, Debug, Clone)]
pub struct Mana {
    pub current: i32,
    pub max: i32,
}

#[derive(Component, Debug, Clone)]
pub struct Name {
    pub value: String,
}

#[derive(Component, Debug, Clone)]
pub struct Level {
    pub current: u32,
    pub experience: u64,
    pub experience_to_next: u64,
}

#[derive(Component, Debug, Clone)]
pub struct Visible {
    pub is_visible: bool,
}

#[derive(Component, Debug, Clone)]
pub struct Temporary {
    pub lifetime: f32,
    pub elapsed: f32,
}
```

### 玩家組件 (`src/components/player.rs`)
```rust
#[derive(Component, Debug, Clone)]
pub struct Player {
    pub id: u32,
    pub class: PlayerClass,
    pub guild_id: Option<u32>,
    pub pk_value: i32,           // PK值
    pub reputation: i32,         // 聲望
}

#[derive(Component, Debug, Clone)]
pub struct LocalPlayer; // 標記本地玩家

#[derive(Component, Debug, Clone)]
pub struct PlayerInput {
    pub move_direction: Vector2,
    pub is_attacking: bool,
    pub is_using_skill: bool,
    pub selected_skill: Option<u32>,
    pub target_entity: Option<Entity>,
}

#[derive(Component, Debug, Clone)]
pub struct PlayerStats {
    pub strength: i32,      // 力量
    pub agility: i32,       // 敏捷
    pub intelligence: i32,  // 智力
    pub stamina: i32,       // 體力
    pub spirit: i32,        // 精神
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerClass {
    Warrior,  // 戰士
    Mage,     // 法師
    Taoist,   // 道士
}
```

### 怪物組件 (`src/components/monster.rs`)
```rust
#[derive(Component, Debug, Clone)]
pub struct Monster {
    pub id: u32,
    pub monster_type: MonsterType,
    pub level: u32,
    pub ai_type: MonsterAI,
    pub respawn_time: f32,
}

#[derive(Component, Debug, Clone)]
pub struct MonsterAIState {
    pub current_behavior: MonsterBehavior,
    pub target_entity: Option<Entity>,
    pub last_decision_time: Instant,
    pub patrol_route: Vec<Vector2>,
    pub current_patrol_index: usize,
}

#[derive(Component, Debug, Clone)]
pub struct LootTable {
    pub items: Vec<LootItem>,
    pub gold_range: (i32, i32),
}

#[derive(Debug, Clone)]
pub enum MonsterType {
    Normal,    // 普通怪物
    Elite,     // 精英怪物  
    Boss,      // BOSS
    Summoned,  // 召喚物
}

#[derive(Debug, Clone)]
pub enum MonsterAI {
    Passive,   // 被動
    Aggressive, // 主動攻擊
    Defensive, // 防禦型
}
```

### NPC組件 (`src/components/npc.rs`)
```rust
#[derive(Component, Debug, Clone)]
pub struct NPC {
    pub id: u32,
    pub npc_type: NPCType,
    pub dialogue_tree: DialogueTree,
    pub services: Vec<NPCService>,
}

#[derive(Component, Debug, Clone)]
pub struct Merchant {
    pub shop_items: Vec<ShopItem>,
    pub buy_rate: f32,    // 收購價格比率
    pub sell_rate: f32,   // 出售價格比率
}

#[derive(Component, Debug, Clone)]
pub struct QuestGiver {
    pub available_quests: Vec<u32>,
    pub completed_quests: Vec<u32>,
}

#[derive(Debug, Clone)]
pub enum NPCType {
    Merchant,     // 商人
    QuestGiver,   // 任務給予者
    Blacksmith,   // 鐵匠
    SkillTrainer, // 技能導師
}
```

### 戰鬥組件 (`src/components/combat.rs`)
```rust
#[derive(Component, Debug, Clone)]
pub struct CombatState {
    pub is_in_combat: bool,
    pub last_attack_time: Instant,
    pub attack_cooldown: f32,
    pub attack_range: f32,
    pub last_target: Option<Entity>,
}

#[derive(Component, Debug, Clone)]
pub struct SkillSet {
    pub learned_skills: Vec<Skill>,
    pub equipped_skills: [Option<u32>; 8], // 快捷欄技能
}

#[derive(Component, Debug, Clone)]
pub struct BuffList {
    pub buffs: Vec<Buff>,
}

#[derive(Component, Debug, Clone)]
pub struct RegenStats {
    pub health_regen: i32,      // 生命恢復
    pub mana_regen: i32,        // 魔法恢復
    pub health_regen_bonus: f32, // 恢復加成
    pub mana_regen_bonus: f32,
    pub last_regen_time: Instant,
}

#[derive(Component, Debug, Clone)]
pub struct DotEffects {
    pub effects: Vec<DotEffect>,
}

// 技能數據
#[derive(Debug, Clone)]
pub struct Skill {
    pub id: u32,
    pub name: String,
    pub level: u32,
    pub cooldown: f32,
    pub mana_cost: i32,
    pub damage: i32,
    pub skill_type: SkillType,
}

// Buff數據
#[derive(Debug, Clone)]
pub struct Buff {
    pub id: u32,
    pub buff_type: BuffType,
    pub start_time: Instant,
    pub duration: f32,
    pub stats: StatModifiers,
}

// DoT效果
#[derive(Debug, Clone)]
pub struct DotEffect {
    pub dot_type: DotType,
    pub damage_per_tick: i32,
    pub tick_interval: f32,
    pub last_tick_time: Instant,
    pub total_ticks: u32,
    pub current_ticks: u32,
    pub source: Entity,
}
```

### 動畫組件 (`src/components/animation.rs`)
```rust
#[derive(Component, Debug, Clone)]
pub struct AnimationState {
    pub current_animation: AnimationType,
    pub current_frame: u32,
    pub frame_timer: f32,
    pub frame_duration: f32,
    pub total_frames: u32,
    pub looping: bool,
    pub playing: bool,
}

#[derive(Component, Debug, Clone)]
pub struct Sprite {
    pub texture_id: TextureId,
    pub source_rect: Rect,
    pub color: Color,
    pub layer: u8,
}

#[derive(Component, Debug, Clone)]
pub struct AttackAnimationData {
    pub attacker: Entity,
    pub attack_type: AttackType,
    pub damage_frame: u32,    // 造成傷害的幀
    pub effect_frame: u32,    // 播放特效的幀
    pub start_time: Instant,
}

#[derive(Debug, Clone)]
pub enum AnimationType {
    Idle,
    Walk,
    Run,
    Attack,
    Skill(String),
    Hurt,
    Death,
}

#[derive(Component, Debug, Clone)]
pub struct SpriteSheet {
    pub texture_id: TextureId,
    pub frame_size: (u32, u32),
    pub frame_count: u32,
    pub frames_per_row: u32,
}
```

### 移動組件 (`src/components/movement.rs`)
```rust
#[derive(Component, Debug, Clone)]
pub struct Movement {
    pub speed: f32,
    pub direction: Direction,
    pub is_moving: bool,
    pub destination: Option<Vector2>,
}

#[derive(Component, Debug, Clone)]
pub struct Pathfinding {
    pub current_path: Vec<Vector2>,
    pub current_target_index: usize,
    pub pathfinding_radius: f32,
}

#[derive(Component, Debug, Clone)]
pub struct Collision {
    pub radius: f32,
    pub collision_type: CollisionType,
    pub is_solid: bool,
}

#[derive(Debug, Clone)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

#[derive(Debug, Clone)]
pub enum CollisionType {
    Player,
    Monster,
    NPC,
    Item,
    Obstacle,
}
```

### 物品組件 (`src/components/inventory.rs`)
```rust
#[derive(Component, Debug, Clone)]
pub struct Inventory {
    pub items: Vec<ItemSlot>,
    pub gold: i32,
    pub capacity: usize,
}

#[derive(Component, Debug, Clone)]
pub struct Equipment {
    pub equipped_items: HashMap<EquipmentSlot, Entity>,
}

#[derive(Component, Debug, Clone)]
pub struct Item {
    pub id: u32,
    pub item_type: ItemType,
    pub stack_size: u32,
    pub max_stack_size: u32,
    pub durability: i32,
    pub max_durability: i32,
    pub requirements: ItemRequirements,
}

#[derive(Component, Debug, Clone)]
pub struct GroundItem {
    pub spawn_time: Instant,
    pub owner: Option<Entity>, // 歸屬權
    pub pickup_timer: f32,
}

#[derive(Debug, Clone)]
pub struct ItemSlot {
    pub item: Option<Entity>,
    pub quantity: u32,
}

#[derive(Debug, Clone)]
pub enum ItemType {
    Weapon(WeaponType),
    Armor(ArmorType),
    Consumable,
    Material,
    Quest,
}

#[derive(Debug, Clone)]
pub enum EquipmentSlot {
    Head,
    Body,
    Hands,
    Legs,
    Feet,
    Weapon,
    Shield,
    Accessory1,
    Accessory2,
}
```

### UI組件 (`src/components/ui.rs`)
```rust
#[derive(Component, Debug, Clone)]
pub struct UIElement {
    pub element_type: UIElementType,
    pub position: Vector2,
    pub size: Vector2,
    pub visible: bool,
    pub z_index: i32,
}

#[derive(Component, Debug, Clone)]
pub struct Button {
    pub text: String,
    pub normal_color: Color,
    pub hover_color: Color,
    pub pressed_color: Color,
    pub is_pressed: bool,
    pub is_hovered: bool,
}

#[derive(Component, Debug, Clone)]
pub struct HealthBar {
    pub entity: Entity,
    pub offset: Vector2,
    pub size: Vector2,
}

#[derive(Component, Debug, Clone)]
pub struct FloatingText {
    pub text: String,
    pub color: Color,
    pub lifetime: f32,
    pub velocity: Vector2,
}

#[derive(Debug, Clone)]
pub enum UIElementType {
    Panel,
    Button,
    Label,
    HealthBar,
    ManaBar,
    Minimap,
    ChatWindow,
}
```

### 網絡組件 (`src/components/network.rs`)
```rust
#[derive(Component, Debug, Clone)]
pub struct NetworkSync {
    pub last_update: Instant,
    pub server_position: Vector2,
    pub interpolation_time: f32,
    pub is_local: bool,
}

#[derive(Component, Debug, Clone)]
pub struct NetworkEventQueue {
    pub incoming_events: Vec<NetworkEvent>,
    pub outgoing_events: Vec<NetworkEvent>,
}

#[derive(Component, Debug, Clone)]
pub struct PlayerSession {
    pub player_id: u32,
    pub session_id: u64,
    pub last_ping_time: Instant,
    pub latency: u32,
}
```

### 特效組件 (`src/components/effects.rs`)
```rust
#[derive(Component, Debug, Clone)]
pub struct ParticleEffect {
    pub effect_type: ParticleType,
    pub particles: Vec<Particle>,
    pub lifetime: f32,
    pub elapsed: f32,
}

#[derive(Component, Debug, Clone)]
pub struct LightSource {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
    pub flicker: bool,
}

#[derive(Component, Debug, Clone)]
pub struct Projectile {
    pub start_position: Vector2,
    pub target_position: Vector2,
    pub speed: f32,
    pub damage: i32,
    pub source: Entity,
}

#[derive(Debug, Clone)]
pub enum ParticleType {
    Fire,
    Smoke,
    Blood,
    Magic,
    Heal,
}
```

### 事件組件 (`src/components/events.rs`)
```rust
// 臨時事件組件（處理後立即銷毀）

#[derive(Component, Debug, Clone)]
pub struct DamageEvent {
    pub target: Entity,
    pub damage: i32,
    pub damage_type: DamageType,
    pub source: Entity,
    pub is_critical: bool,
}

#[derive(Component, Debug, Clone)]
pub struct HealEvent {
    pub target: Entity,
    pub amount: i32,
    pub source: Entity,
}

#[derive(Component, Debug, Clone)]
pub struct LevelUpEvent {
    pub entity: Entity,
    pub new_level: u32,
}

#[derive(Component, Debug, Clone)]
pub struct ItemPickupEvent {
    pub entity: Entity,
    pub item: Entity,
}

#[derive(Component, Debug, Clone)]
pub struct AnimationChangeEvent {
    pub entity: Entity,
    pub new_animation: AnimationType,
    pub priority: AnimationPriority,
}

#[derive(Component, Debug, Clone)]
pub struct RegenEvent {
    pub entity: Entity,
    pub amount: i32,
    pub regen_type: RegenType,
    pub is_critical: bool,
}
```

## 🎯 實體組合示例

### 玩家實體：
```rust
// 本地玩家
world.spawn((
    Transform::default(),
    Velocity::default(),
    Health { current: 100, max: 100, last_damage_time: None },
    Mana { current: 50, max: 50 },
    Player {
        id: 1,
        class: PlayerClass::Warrior,
        guild_id: Some(1),
        pk_value: 0,
        reputation: 100,
    },
    LocalPlayer,
    PlayerInput::default(),
    PlayerStats {
        strength: 10,
        agility: 5,
        intelligence: 3,
        stamina: 8,
        spirit: 4,
    },
    CombatState::default(),
    SkillSet::default(),
    BuffList::default(),
    RegenStats::default(),
    AnimationState::default(),
    Sprite {
        texture_id: TextureId::WarriorIdle,
        source_rect: Rect::new(0, 0, 64, 64),
        color: Color::WHITE,
        layer: 10,
    },
    Movement::default(),
    Collision {
        radius: 0.5,
        collision_type: CollisionType::Player,
        is_solid: true,
    },
    Inventory::default(),
    Equipment::default(),
    NetworkSync {
        is_local: true,
        ..Default::default()
    },
    Name {
        value: "傳奇勇士".to_string(),
    },
    Level {
        current: 1,
        experience: 0,
        experience_to_next: 1000,
    },
    Visible { is_visible: true },
));
```

### 怪物實體：
```rust
// 普通怪物 - 殭屍
world.spawn((
    Transform::from_position(Vector2::new(100.0, 100.0)),
    Health { current: 50, max: 50, last_damage_time: None },
    Monster {
        id: 1001,
        monster_type: MonsterType::Normal,
        level: 5,
        ai_type: MonsterAI::Aggressive,
        respawn_time: 30.0,
    },
    MonsterAIState {
        current_behavior: MonsterBehavior::Idle,
        target_entity: None,
        last_decision_time: Instant::now(),
        patrol_route: vec![],
        current_patrol_index: 0,
    },
    CombatState {
        attack_range: 1.5,
        ..Default::default()
    },
    AnimationState {
        current_animation: AnimationType::Idle,
        frame_duration: 0.2,
        total_frames: 4,
        looping: true,
        playing: true,
        ..Default::default()
    },
    Sprite {
        texture_id: TextureId::ZombieIdle,
        source_rect: Rect::new(0, 0, 64, 64),
        color: Color::WHITE,
        layer: 8,
    },
    Movement {
        speed: 1.0,
        ..Default::default()
    },
    Collision {
        radius: 0.4,
        collision_type: CollisionType::Monster,
        is_solid: true,
    },
    LootTable {
        items: vec![
            LootItem { item_id: 1001, chance: 0.5, min_quantity: 1, max_quantity: 1 },
        ],
        gold_range: (1, 10),
    },
    Name {
        value: "殭屍".to_string(),
    },
    Visible { is_visible: true },
));
```

### NPC實體：
```rust
// 商人NPC
world.spawn((
    Transform::from_position(Vector2::new(200.0, 150.0)),
    NPC {
        id: 2001,
        npc_type: NPCType::Merchant,
        dialogue_tree: DialogueTree::default(),
        services: vec![NPCService::Buy, NPCService::Sell],
    },
    Merchant {
        shop_items: vec![
            ShopItem { item_id: 1001, price: 50, stock: 10 },
            ShopItem { item_id: 1002, price: 100, stock: 5 },
        ],
        buy_rate: 0.5,
        sell_rate: 1.2,
    },
    AnimationState {
        current_animation: AnimationType::Idle,
        frame_duration: 0.3,
        total_frames: 2,
        looping: true,
        playing: true,
        ..Default::default()
    },
    Sprite {
        texture_id: TextureId::MerchantIdle,
        source_rect: Rect::new(0, 0, 64, 64),
        color: Color::WHITE,
        layer: 7,
    },
    Collision {
        radius: 0.4,
        collision_type: CollisionType::NPC,
        is_solid: true,
    },
    Name {
        value: "武器商人".to_string(),
    },
    Visible { is_visible: true },
));
```

### 地面物品實體：
```rust
// 地面上的治療藥水
world.spawn((
    Transform::from_position(Vector2::new(120.0, 80.0)),
    Item {
        id: 2001,
        item_type: ItemType::Consumable,
        stack_size: 1,
        max_stack_size: 10,
        durability: -1,
        max_durability: -1,
        requirements: ItemRequirements::default(),
    },
    GroundItem {
        spawn_time: Instant::now(),
        owner: None,
        pickup_timer: 0.0,
    },
    Sprite {
        texture_id: TextureId::HealthPotion,
        source_rect: Rect::new(0, 0, 32, 32),
        color: Color::WHITE,
        layer: 5,
    },
    Collision {
        radius: 0.3,
        collision_type: CollisionType::Item,
        is_solid: false,
    },
    Temporary {
        lifetime: 300.0, // 5分鐘後消失
        elapsed: 0.0,
    },
    Visible { is_visible: true },
));
```

### 特效實體：
```rust
// 治療特效
world.spawn((
    Transform::from_position(Vector2::new(150.0, 100.0)),
    ParticleEffect {
        effect_type: ParticleType::Heal,
        particles: vec![],
        lifetime: 1.0,
        elapsed: 0.0,
    },
    Temporary {
        lifetime: 1.0,
        elapsed: 0.0,
    },
    Visible { is_visible: true },
));
```

## 🔧 組件查詢示例

### 戰鬥系統查詢：
```rust
// 查找所有可攻擊的目標
for (entity, (transform, health, monster)) in 
    world.query::<(&Transform, &Health, &Monster)>().iter() 
{
    if health.current > 0 {
        // 處理戰鬥邏輯
    }
}
```

### 動畫系統查詢：
```rust
// 更新所有動畫實體
for (entity, (mut animation, mut sprite)) in 
    world.query_mut::<(&mut AnimationState, &mut Sprite)>().iter() 
{
    if animation.playing {
        // 更新動畫幀
    }
}
```

這個完整的實體與組件設計提供了熱血傳奇遊戲所需的所有數據結構，確保了系統間的清晰數據流和高效的組件查詢！