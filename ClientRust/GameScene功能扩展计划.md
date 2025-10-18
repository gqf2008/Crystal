# 🎮 GameScene 功能扩展计划

**时间**: 2024年10月18日  
**目标**: 完善游戏场景的核心功能  
**优先级**: 高

---

## 📊 当前状态分析

### ✅ 已实现

| 功能 | 状态 | 行数 |
|------|------|------|
| GameSceneState 资源 | ✅ | 40 行 |
| UI 框架 (HUD/聊天/技能栏) | ✅ | 200+ 行 |
| 基础系统 (15 个) | ✅ | 200+ 行 |
| 消息类型 (13 种) | ✅ | 50+ 行 |

### ⏳ 需要完善

| 功能 | 优先级 | 预计 |
|------|--------|------|
| 玩家实体完整管理 | 🔴 高 | 2h |
| 地图加载和渲染 | 🔴 高 | 3h |
| NPC/对象系统 | 🟡 中 | 2h |
| 聊天系统完整实现 | 🟡 中 | 2h |
| 网络同步集成 | 🔴 高 | 3h |
| 完整事件循环 | 🟡 中 | 1h |

---

## 🎯 功能扩展方案

### Phase 1: 玩家实体管理 (HIGH PRIORITY) 🔴

**目标**: 完整的玩家数据和行为管理

#### 1.1 增强 Player 组件
```rust
#[derive(Component, Clone, Debug)]
pub struct Player {
    pub character_id: i32,
    pub name: String,
    pub class: u8,
    pub gender: u8,
    pub level: u16,
    
    // 扩展字段
    pub hair: u8,              // 发型
    pub face: u8,              // 脸型
    pub equipment: [Option<Item>; 16],  // 装备槽位
    pub stats: CharacterStats, // 角色属性
    pub buffs: Vec<BuffEffect>, // 增益效果
}

pub struct CharacterStats {
    pub hp: u16,
    pub max_hp: u16,
    pub mp: u16,
    pub max_mp: u16,
    pub attack: u16,
    pub defense: u16,
    pub magic_attack: u16,
    pub magic_defense: u16,
}

pub struct Item {
    pub item_id: u32,
    pub name: String,
    pub count: u8,
}

pub struct BuffEffect {
    pub buff_id: u32,
    pub name: String,
    pub duration: f32,
    pub effect_type: BuffType,
}

pub enum BuffType {
    Healing,
    Damage,
    Speed,
    Defense,
}
```

#### 1.2 新增系统
```rust
// 玩家属性更新系统
pub fn update_player_stats_system(
    mut player_query: Query<(&Player, &mut Transform), Changed<Player>>,
    mut game_state: ResMut<GameSceneState>,
) {
    for (player, _transform) in player_query.iter_mut() {
        // 更新游戏状态中的玩家属性
        game_state.player_level = player.level;
        game_state.player_health = player.stats.hp;
        game_state.player_max_health = player.stats.max_hp;
        game_state.player_mana = player.stats.mp;
        game_state.player_max_mana = player.stats.max_mp;
    }
}

// 增益效果处理系统
pub fn process_buffs_system(
    mut player_query: Query<&mut Player>,
    time: Res<Time>,
) {
    for mut player in player_query.iter_mut() {
        // 更新增益持续时间
        for buff in player.buffs.iter_mut() {
            buff.duration -= time.delta_secs();
        }
        
        // 移除过期增益
        player.buffs.retain(|buff| buff.duration > 0.0);
        
        // 应用增益效果到属性
        apply_buff_effects(&mut player);
    }
}

fn apply_buff_effects(player: &mut Player) {
    // 重置属性到基础值
    let base_stats = get_base_stats(player.level);
    
    // 遍历所有增益，应用效果
    for buff in player.buffs.iter() {
        match buff.effect_type {
            BuffType::Healing => {
                player.stats.hp = (player.stats.hp + 5).min(player.stats.max_hp);
            },
            BuffType::Defense => {
                player.stats.defense = (player.stats.defense as f32 * 1.2) as u16;
            },
            // ... 其他增益类型
        }
    }
}
```

---

### Phase 2: 地图加载和渲染 (HIGH PRIORITY) 🔴

**目标**: 完整的地图系统

#### 2.1 地图数据结构
```rust
pub struct MapData {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub tile_size: u32,        // 通常 48×32 像素
    pub layers: Vec<MapLayer>,
    pub objects: Vec<MapObject>,
    pub npc_spawns: Vec<NPCSpawn>,
}

pub struct MapLayer {
    pub layer_index: u32,
    pub is_collision: bool,
    pub tiles: Vec<u32>,       // 瓦片 ID
    pub properties: HashMap<String, String>,
}

pub struct MapObject {
    pub object_id: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub object_type: String,
    pub properties: HashMap<String, String>,
}

pub struct NPCSpawn {
    pub npc_id: u32,
    pub npc_type: u32,
    pub x: f32,
    pub y: f32,
    pub direction: u8,
}
```

#### 2.2 地图加载系统
```rust
pub fn load_map_system(
    mut commands: Commands,
    mut game_state: ResMut<GameSceneState>,
    map_assets: Res<MapAssets>,
    images: Res<Assets<Image>>,
) {
    if !game_state.is_initialized {
        // 加载地图数据
        let map_name = &game_state.current_map.clone();
        
        match load_map_file(map_name) {
            Ok(map_data) => {
                // 创建地图渲染层
                for (layer_idx, layer) in map_data.layers.iter().enumerate() {
                    create_map_layer(&mut commands, &layer, layer_idx as u32);
                }
                
                // 创建地图对象 (交互物件)
                for obj in map_data.objects.iter() {
                    create_map_object(&mut commands, obj);
                }
                
                // 生成 NPC
                for spawn in map_data.npc_spawns.iter() {
                    spawn_npc(&mut commands, spawn);
                }
                
                game_state.is_initialized = true;
                info!("✅ 地图已加载: {}", map_name);
            },
            Err(e) => {
                error!("❌ 地图加载失败: {}", e);
            }
        }
    }
}

fn create_map_layer(
    commands: &mut Commands,
    layer: &MapLayer,
    layer_idx: u32,
) {
    // 为每个瓦片创建精灵
    for (idx, tile_id) in layer.tiles.iter().enumerate() {
        if *tile_id == 0 {
            continue;  // 跳过空瓦片
        }
        
        let x = (idx as u32 % layer.width) as f32 * 48.0;
        let y = (idx as u32 / layer.width) as f32 * 32.0;
        
        commands.spawn((
            Sprite {
                // 从地图集获取纹理
                ..default()
            },
            Transform::from_xyz(x, y, layer_idx as f32),
            MapLayer {
                layer_index: layer_idx,
            },
        ));
    }
}

fn create_map_object(commands: &mut Commands, obj: &MapObject) {
    commands.spawn((
        InteractiveObject {
            object_id: obj.object_id as i32,
            name: obj.name.clone(),
            object_type: obj.object_type.clone(),
            interaction_range: 50.0,
        },
        Transform::from_xyz(obj.x, obj.y, 100.0),
    ));
}

fn spawn_npc(commands: &mut Commands, spawn: &NPCSpawn) {
    commands.spawn((
        NPC {
            npc_id: spawn.npc_id as i32,
            name: format!("NPC_{}", spawn.npc_id),
            dialogue_id: Some(spawn.npc_id as i32),
        },
        Transform::from_xyz(spawn.x, spawn.y, 50.0),
    ));
}
```

---

### Phase 3: NPC 和对象交互 (MEDIUM PRIORITY) 🟡

**目标**: 完整的 NPC 系统和交互

#### 3.1 对话系统
```rust
pub struct DialogueTree {
    pub nodes: HashMap<i32, DialogueNode>,
    pub start_node: i32,
}

pub struct DialogueNode {
    pub node_id: i32,
    pub speaker: String,
    pub text: String,
    pub options: Vec<DialogueOption>,
    pub actions: Vec<DialogueAction>,
}

pub struct DialogueOption {
    pub text: String,
    pub next_node: i32,
    pub condition: Option<String>,
}

pub struct DialogueAction {
    pub action_type: String,  // "quest", "trade", "attack", etc
    pub target: String,
}

pub resource DialogueState {
    pub current_dialogue: Option<i32>,
    pub current_node: i32,
    pub speaker_npc: Option<i32>,
}
```

#### 3.2 交互系统
```rust
pub fn handle_interaction_system(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    interactive_query: Query<(&Transform, &InteractiveObject)>,
    npc_query: Query<(&Transform, &NPC)>,
    mut dialogue_state: ResMut<DialogueState>,
) {
    if !mouse_button.just_pressed(MouseButton::Right) {
        return;
    }
    
    let Some((camera, camera_transform)) = camera_query.iter().next() else {
        return;
    };
    
    let Some(window) = windows.iter().next() else {
        return;
    };
    
    if let Some(cursor_pos) = window.cursor_position() {
        if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
            // 检查点击的是否是可交互对象
            for (transform, obj) in interactive_query.iter() {
                let distance = world_pos.distance(transform.translation.truncate());
                if distance < obj.interaction_range {
                    info!("🔗 与对象交互: {} ({})", obj.name, obj.object_type);
                    handle_object_interaction(&mut commands, obj);
                    return;
                }
            }
            
            // 检查点击的是否是 NPC
            for (transform, npc) in npc_query.iter() {
                let distance = world_pos.distance(transform.translation.truncate());
                if distance < 50.0 {
                    info!("💬 与 NPC 交互: {}", npc.name);
                    start_dialogue(&mut dialogue_state, npc.npc_id, npc.dialogue_id);
                    return;
                }
            }
        }
    }
}

fn handle_object_interaction(commands: &mut Commands, obj: &InteractiveObject) {
    match obj.object_type.as_str() {
        "door" => {
            // 处理门
            info!("🚪 打开门");
        },
        "chest" => {
            // 打开宝箱
            info!("💎 打开宝箱");
        },
        "item_drop" => {
            // 拾取掉落物品
            info!("📦 拾取物品");
        },
        _ => {
            info!("❓ 未知对象类型: {}", obj.object_type);
        }
    }
}

fn start_dialogue(
    dialogue_state: &mut ResMut<DialogueState>,
    npc_id: i32,
    dialogue_id: Option<i32>,
) {
    if let Some(d_id) = dialogue_id {
        dialogue_state.current_dialogue = Some(d_id);
        dialogue_state.current_node = 0;
        dialogue_state.speaker_npc = Some(npc_id);
    }
}
```

---

### Phase 4: 聊天系统完整实现 (MEDIUM PRIORITY) 🟡

**目标**: 完整的聊天功能

#### 4.1 增强聊天消息
```rust
pub struct ChatMessage {
    pub sender: String,
    pub content: String,
    pub timestamp: f32,
    pub message_type: ChatMessageType,
    pub channel: ChatChannel,
}

pub enum ChatMessageType {
    Normal,
    Whisper,
    System,
    Broadcast,
    Guild,
    Party,
}

pub enum ChatChannel {
    General,
    Whisper(String),  // 私聊目标
    Guild,
    Party,
}

pub resource ChatManager {
    pub history: VecDeque<ChatMessage>,
    pub max_history: usize,
    pub input_buffer: String,
}
```

#### 4.2 聊天系统
```rust
pub fn handle_chat_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut chat_manager: ResMut<ChatManager>,
    mut game_state: ResMut<GameSceneState>,
) {
    // 切换聊天开启/关闭
    if keyboard.just_pressed(KeyCode::Enter) {
        game_state.show_chat = !game_state.show_chat;
    }
    
    if !game_state.show_chat {
        return;
    }
    
    // 处理文本输入 (需要集成文本输入系统)
    // ... 文本输入逻辑
    
    // 发送消息
    if keyboard.just_pressed(KeyCode::Enter) && !chat_manager.input_buffer.is_empty() {
        send_chat_message(&mut chat_manager);
    }
}

fn send_chat_message(chat_manager: &mut ResMut<ChatManager>) {
    let message = ChatMessage {
        sender: "Player".to_string(),
        content: chat_manager.input_buffer.clone(),
        timestamp: 0.0,  // 应该使用真实时间
        message_type: ChatMessageType::Normal,
        channel: ChatChannel::General,
    };
    
    chat_manager.history.push_back(message);
    
    // 保持历史记录大小
    if chat_manager.history.len() > chat_manager.max_history {
        chat_manager.history.pop_front();
    }
    
    chat_manager.input_buffer.clear();
    
    info!("💬 聊天消息已发送");
}

pub fn update_chat_display_system(
    mut text_query: Query<&mut Text, With<ChatMessageList>>,
    chat_manager: Res<ChatManager>,
) {
    if let Ok(mut text) = text_query.get_single_mut() {
        let mut display_text = String::new();
        
        for msg in chat_manager.history.iter() {
            let line = format!("[{}] {}: {}\n", 
                chrono::Local::now().format("%H:%M:%S"),
                msg.sender,
                msg.content
            );
            display_text.push_str(&line);
        }
        
        text.0 = display_text;
    }
}
```

---

### Phase 5: 网络同步集成 (HIGH PRIORITY) 🔴

**目标**: 与服务器进行完整的数据同步

#### 5.1 网络事件处理
```rust
pub fn handle_network_events_system(
    mut player_query: Query<(&mut Player, &mut Transform)>,
    mut game_state: ResMut<GameSceneState>,
    mut commands: Commands,
    // network: Res<NetworkResource>,  // 需要实现
) {
    // 检查网络事件队列
    // let mut event_queue = network.event_queue.lock().unwrap();
    // 
    // while let Some(event) = event_queue.pop_front() {
    //     match event {
    //         GameEvent::PlayerPositionUpdate { player_id, x, y, direction } => {
    //             update_player_position(&mut player_query, player_id, x, y);
    //         },
    //         GameEvent::PlayerStatsUpdate { player_id, hp, mp, level } => {
    //             update_player_stats(&mut player_query, player_id, hp, mp, level);
    //         },
    //         GameEvent::OtherPlayerAppear { player_id, name, x, y } => {
    //             spawn_other_player(&mut commands, player_id, name, x, y);
    //         },
    //         GameEvent::OtherPlayerDisappear { player_id } => {
    //             despawn_other_player(&mut commands, player_id);
    //         },
    //         GameEvent::ChatMessage { sender, text } => {
    //             receive_chat_message(&mut chat_manager, sender, text);
    //         },
    //         _ => {}
    //     }
    // }
}

fn update_player_position(
    player_query: &mut Query<(&mut Player, &mut Transform)>,
    player_id: u32,
    x: f32,
    y: f32,
) {
    for (player, mut transform) in player_query.iter_mut() {
        if player.character_id == player_id as i32 {
            transform.translation = Vec3::new(x, y, transform.translation.z);
            break;
        }
    }
}

fn spawn_other_player(
    commands: &mut Commands,
    player_id: u32,
    name: String,
    x: f32,
    y: f32,
) {
    // 生成其他玩家的实体
    commands.spawn((
        Player {
            character_id: player_id as i32,
            name,
            class: 0,
            gender: 0,
            level: 1,
            ..default()
        },
        Transform::from_xyz(x, y, 50.0),
    ));
}
```

#### 5.2 发送玩家位置更新
```rust
pub fn send_position_update_system(
    player_query: Query<(&Player, &Transform), Changed<Transform>>,
    mut game_state: ResMut<GameSceneState>,
    // network: Res<NetworkResource>,
) {
    for (player, transform) in player_query.iter() {
        // 定期发送玩家位置到服务器
        // let cmd = NetworkCommand::PlayerMove {
        //     x: transform.translation.x as i32,
        //     y: transform.translation.y as i32,
        // };
        // network.command_tx.send(cmd).ok();
        
        info!("📍 玩家位置: ({:.1}, {:.1})", 
            transform.translation.x, 
            transform.translation.y
        );
    }
}
```

---

### Phase 6: 完整事件循环 (MEDIUM PRIORITY) 🟡

**目标**: 整合所有系统的事件循环

#### 6.1 增强的主游戏循环
```rust
pub fn game_loop_system(
    mut game_state: ResMut<GameSceneState>,
    time: Res<Time>,
) {
    if !game_state.is_paused {
        // 更新游戏时间
        game_state.game_time += time.delta_secs();
        
        // 每 5 秒输出一次游戏状态
        if (game_state.game_time % 5.0) < time.delta_secs() {
            info!("🎮 游戏时间: {:.1}s | 玩家等级: {} | HP: {}/{}", 
                game_state.game_time,
                game_state.player_level,
                game_state.player_health,
                game_state.player_max_health
            );
        }
    }
}

pub fn update_ui_system(
    game_state: Res<GameSceneState>,
    mut text_query: Query<&mut Text, With<PlayerInfoHud>>,
) {
    if let Ok(mut text) = text_query.get_single_mut() {
        text.0 = format!(
            "Lv. {} | HP: {}/{} | MP: {}/{} | Time: {:.1}s",
            game_state.player_level,
            game_state.player_health,
            game_state.player_max_health,
            game_state.player_mana,
            game_state.player_max_mana,
            game_state.game_time
        );
    }
}
```

---

## 📋 实现检查清单

### Phase 1: 玩家实体管理
- [ ] 增强 Player 组件字段
- [ ] 添加 CharacterStats 结构
- [ ] 实现 update_player_stats_system
- [ ] 实现 process_buffs_system
- [ ] 注册系统到应用

### Phase 2: 地图加载
- [ ] 定义 MapData 结构
- [ ] 实现 load_map_system
- [ ] 实现 create_map_layer
- [ ] 实现 create_map_object
- [ ] 实现 spawn_npc

### Phase 3: NPC 交互
- [ ] 定义对话系统数据结构
- [ ] 实现 handle_interaction_system
- [ ] 实现对话树逻辑
- [ ] 测试 NPC 交互

### Phase 4: 聊天系统
- [ ] 实现 ChatManager 资源
- [ ] 实现 handle_chat_input_system
- [ ] 实现 update_chat_display_system
- [ ] 集成文本输入

### Phase 5: 网络同步
- [ ] 建立网络通信架构
- [ ] 实现 handle_network_events_system
- [ ] 实现 send_position_update_system
- [ ] 测试网络同步

### Phase 6: 事件循环
- [ ] 实现 game_loop_system
- [ ] 实现 update_ui_system
- [ ] 集成所有系统
- [ ] 完整流程测试

---

## ⏱️ 工作量估计

| 阶段 | 任务 | 估计 |
|------|------|------|
| Phase 1 | 玩家实体管理 | 2 小时 |
| Phase 2 | 地图加载渲染 | 3 小时 |
| Phase 3 | NPC 交互 | 2 小时 |
| Phase 4 | 聊天系统 | 2 小时 |
| Phase 5 | 网络同步 | 3 小时 |
| Phase 6 | 事件循环 | 1 小时 |
| **总计** | **GameScene 完整功能** | **13 小时** |

---

## 🎯 建议优先级

### 🔴 必做 (今天)
1. Phase 1 - 玩家实体管理
2. Phase 2 - 地图加载 (至少基础)
3. Phase 6 - 事件循环整合

### 🟡 重要 (本周)
4. Phase 3 - NPC 交互
5. Phase 4 - 聊天系统
6. Phase 5 - 网络同步

---

**下一步**: 选择一个阶段开始实现，我可以帮助编写代码和集成系统。

