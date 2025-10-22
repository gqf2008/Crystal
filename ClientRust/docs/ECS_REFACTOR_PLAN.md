# GameScene ECS 重构计划

## 问题总结

当前 `GameScene` 的设计存在以下违反 ECS 原则的问题:

### 1. UI对话框不是组件
- ❌ `main_dialog`, `inventory_dialog` 等存储在 Scene 结构体中
- ✅ 应该作为 ECS 组件存储在 World 中

### 2. 业务逻辑在Scene中
- ❌ `use_skill()` 方法直接在 Scene 中处理技能逻辑
- ✅ 应该由 `SkillSystem` 处理

### 3. 直接修改UI状态
- ❌ `handle_network_event()` 直接调用 `self.chat_dialog.add_message()`
- ✅ 应该通过事件系统解耦

### 4. 缺少System抽象
- ❌ 缺少 `UISystem` 统一处理UI更新
- ✅ 需要创建完整的System架构

---

## 重构步骤

### 阶段1: UI组件化 (高优先级)

#### 1.1 定义UI组件
```rust
// src/ecs/ui/components.rs

/// 主对话框组件
#[derive(Debug)]
pub struct MainDialogComp {
    pub dialog: MainDialog,
}

/// 背包对话框组件
#[derive(Debug)]
pub struct InventoryDialogComp {
    pub dialog: InventoryDialog,
}

/// 角色对话框组件
#[derive(Debug)]
pub struct CharacterDialogComp {
    pub dialog: CharacterDialog,
}

/// 技能栏组件
#[derive(Debug)]
pub struct SkillBarComp {
    pub dialog: SkillBarDialog,
    pub bar_index: u8,
}

/// 聊天对话框组件
#[derive(Debug)]
pub struct ChatDialogComp {
    pub dialog: ChatDialog,
}
```

#### 1.2 修改 GameScene 结构
```rust
// src/ecs/scenes/game_scene.rs

pub struct GameScene {
    // 核心实体
    camera_entity: Entity,
    time_entity: Entity,
    config_entity: Entity,
    visible_area_entity: Entity,
    
    // UI实体引用
    main_dialog_entity: Entity,
    inventory_dialog_entity: Entity,
    character_dialog_entity: Entity,
    skillbar_entities: [Entity; 2],
    chat_dialog_entity: Entity,
    
    // 系统
    network_system: NetworkSystem,
    ui_system: UISystem,
    skill_system: SkillSystem,
}
```

#### 1.3 初始化时创建UI实体
```rust
impl GameScene {
    pub fn new(ctx: &mut Context, world: &mut World) -> GameResult<Self> {
        // ... 其他初始化 ...
        
        // 创建UI实体
        let screen = ctx.gfx.drawable_size();
        
        let main_dialog_entity = world.spawn((
            MainDialogComp {
                dialog: MainDialog::new(screen.0, screen.1),
            },
        ));
        
        let inventory_dialog_entity = world.spawn((
            InventoryDialogComp {
                dialog: InventoryDialog::new(),
            },
        ));
        
        let character_dialog_entity = world.spawn((
            CharacterDialogComp {
                dialog: CharacterDialog::new(),
            },
        ));
        
        let skillbar_entities = [
            world.spawn((SkillBarComp {
                dialog: SkillBarDialog::new(0),
                bar_index: 0,
            })),
            world.spawn((SkillBarComp {
                dialog: SkillBarDialog::new(1),
                bar_index: 1,
            })),
        ];
        
        let chat_dialog_entity = world.spawn((
            ChatDialogComp {
                dialog: ChatDialog::new(100.0, 400.0),
            },
        ));
        
        Ok(Self {
            camera_entity,
            time_entity,
            config_entity,
            visible_area_entity,
            main_dialog_entity,
            inventory_dialog_entity,
            character_dialog_entity,
            skillbar_entities,
            chat_dialog_entity,
            network_system: NetworkSystem::new(),
            ui_system: UISystem::new(),
            skill_system: SkillSystem::new(),
        })
    }
}
```

---

### 阶段2: 创建UISystem (高优先级)

#### 2.1 定义UI事件组件
```rust
// src/ecs/ui/events.rs

/// 聊天消息事件
#[derive(Debug, Clone)]
pub struct ChatMessageEvent {
    pub text: String,
    pub chat_type: ChatType,
}

/// 金币变化事件
#[derive(Debug, Clone)]
pub struct GoldChangedEvent {
    pub gold: u32,
}

/// 物品获得事件
#[derive(Debug, Clone)]
pub struct ItemGainedEvent {
    pub item_name: String,
}

/// 技能冷却提示事件
#[derive(Debug, Clone)]
pub struct SkillCooldownHintEvent {
    pub progress: f32,
}

/// MP不足提示事件
#[derive(Debug, Clone)]
pub struct InsufficientManaEvent {
    pub required: u16,
    pub current: i32,
}
```

#### 2.2 实现UISystem
```rust
// src/ecs/systems/ui_system.rs

pub struct UISystem;

impl UISystem {
    pub fn new() -> Self {
        Self
    }
    
    /// 更新所有UI组件
    pub fn update(&mut self, world: &mut World) {
        self.process_chat_events(world);
        self.process_gold_events(world);
        self.process_item_events(world);
        self.process_skill_hint_events(world);
    }
    
    /// 处理聊天消息事件
    fn process_chat_events(&self, world: &mut World) {
        // 收集所有聊天事件
        let events: Vec<_> = world
            .query::<&ChatMessageEvent>()
            .iter()
            .map(|(entity, event)| (entity, event.clone()))
            .collect();
        
        if events.is_empty() {
            return;
        }
        
        // 找到ChatDialog组件并更新
        for (_, chat_comp) in world.query_mut::<&mut ChatDialogComp>() {
            for (_, event) in &events {
                chat_comp.dialog.add_message(
                    event.text.clone(),
                    event.chat_type
                );
            }
        }
        
        // 删除已处理的事件
        for (entity, _) in events {
            let _ = world.despawn(entity);
        }
    }
    
    /// 处理金币变化事件
    fn process_gold_events(&self, world: &mut World) {
        let events: Vec<_> = world
            .query::<&GoldChangedEvent>()
            .iter()
            .map(|(entity, event)| (entity, event.gold))
            .collect();
        
        if events.is_empty() {
            return;
        }
        
        // 更新背包对话框的金币显示
        for (_, inv_comp) in world.query_mut::<&mut InventoryDialogComp>() {
            if let Some((_, gold)) = events.last() {
                inv_comp.dialog.set_gold(*gold);
            }
        }
        
        // 删除已处理的事件
        for (entity, _) in events {
            let _ = world.despawn(entity);
        }
    }
    
    /// 处理技能提示事件
    fn process_skill_hint_events(&self, world: &mut World) {
        // 处理冷却提示
        let cooldown_events: Vec<_> = world
            .query::<&SkillCooldownHintEvent>()
            .iter()
            .map(|(entity, event)| (entity, event.progress))
            .collect();
        
        for (entity, progress) in cooldown_events {
            // 添加到聊天
            for (_, chat_comp) in world.query_mut::<&mut ChatDialogComp>() {
                chat_comp.dialog.add_message(
                    format!("技能冷却中: {:.0}%", progress * 100.0),
                    ChatType::System
                );
            }
            let _ = world.despawn(entity);
        }
        
        // 处理MP不足提示
        let mana_events: Vec<_> = world
            .query::<&InsufficientManaEvent>()
            .iter()
            .map(|(entity, event)| (entity, event.required))
            .collect();
        
        for (entity, required) in mana_events {
            for (_, chat_comp) in world.query_mut::<&mut ChatDialogComp>() {
                chat_comp.dialog.add_message(
                    format!("MP不足! 需要: {}", required),
                    ChatType::System
                );
            }
            let _ = world.despawn(entity);
        }
    }
    
    /// 渲染所有UI
    pub fn draw(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        current_time: u64,
    ) -> GameResult {
        // 渲染主对话框
        for (_, dialog) in world.query::<&MainDialogComp>().iter() {
            dialog.dialog.draw(ctx, canvas)?;
        }
        
        // 渲染背包对话框
        for (_, dialog) in world.query::<&InventoryDialogComp>().iter() {
            dialog.dialog.draw(ctx, canvas)?;
        }
        
        // 渲染角色对话框
        for (_, dialog) in world.query::<&CharacterDialogComp>().iter() {
            dialog.dialog.draw(ctx, canvas)?;
        }
        
        // 渲染技能栏
        for (_, skill_bar) in world.query::<&SkillBarComp>().iter() {
            skill_bar.dialog.draw(ctx, canvas, current_time)?;
        }
        
        // 渲染聊天对话框
        for (_, dialog) in world.query::<&ChatDialogComp>().iter() {
            dialog.dialog.draw(ctx, canvas)?;
        }
        
        Ok(())
    }
}
```

---

### 阶段3: 创建SkillSystem (高优先级)

#### 3.1 定义技能相关组件
```rust
// src/ecs/components/skill.rs

/// 技能冷却组件
#[derive(Debug, Clone)]
pub struct SkillCooldowns {
    /// 技能ID -> 施法时间戳
    pub cooldowns: std::collections::HashMap<u8, u64>,
}

impl SkillCooldowns {
    pub fn new() -> Self {
        Self {
            cooldowns: std::collections::HashMap::new(),
        }
    }
    
    pub fn is_cooling(&self, spell_id: u8, current_time: u64, cooldown_ms: u32) -> bool {
        if let Some(&cast_time) = self.cooldowns.get(&spell_id) {
            let elapsed = current_time.saturating_sub(cast_time);
            elapsed < cooldown_ms as u64
        } else {
            false
        }
    }
    
    pub fn start_cooldown(&mut self, spell_id: u8, current_time: u64) {
        self.cooldowns.insert(spell_id, current_time);
    }
    
    pub fn get_progress(&self, spell_id: u8, current_time: u64, cooldown_ms: u32) -> f32 {
        if let Some(&cast_time) = self.cooldowns.get(&spell_id) {
            let elapsed = current_time.saturating_sub(cast_time);
            if elapsed >= cooldown_ms as u64 {
                return 0.0;
            }
            let progress = elapsed as f32 / cooldown_ms as f32;
            1.0 - progress
        } else {
            0.0
        }
    }
}
```

#### 3.2 实现SkillSystem
```rust
// src/ecs/systems/skill_system.rs

pub struct SkillSystem;

impl SkillSystem {
    pub fn new() -> Self {
        Self
    }
    
    /// 尝试使用技能
    pub fn try_use_skill(
        world: &mut World,
        skill_bar_entity: Entity,
        slot_index: usize,
        current_time: u64,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> Result<(), SkillError> {
        // 1. 获取技能信息
        let skill_info = {
            let skill_bar = world.get::<&SkillBarComp>(skill_bar_entity)
                .map_err(|_| SkillError::InvalidSkillBar)?;
            
            let skill = skill_bar.dialog.get_skill(slot_index)
                .ok_or(SkillError::EmptySlot)?;
            
            (skill.spell, skill.level, skill.mp_cost, skill.cooldown_ms)
        };
        
        let (spell_id, level, mp_cost, cooldown_ms) = skill_info;
        
        // 2. 查找本地玩家
        let mut player_found = false;
        let mut cooldown_error = false;
        let mut mana_error = false;
        
        for (_, (_, mana, cooldowns)) in world.query_mut::<(
            &LocalPlayer,
            &mut Mana,
            &mut SkillCooldowns,
        )>() {
            player_found = true;
            
            // 3. 检查冷却
            if cooldowns.is_cooling(spell_id, current_time, cooldown_ms) {
                let progress = cooldowns.get_progress(spell_id, current_time, cooldown_ms);
                // 生成冷却提示事件
                world.spawn((SkillCooldownHintEvent { progress }));
                cooldown_error = true;
                break;
            }
            
            // 4. 检查MP
            if !mana.has_enough(mp_cost as i32) {
                // 生成MP不足事件
                world.spawn((InsufficientManaEvent {
                    required: mp_cost,
                    current: mana.current,
                }));
                mana_error = true;
                break;
            }
            
            // 5. 消耗MP并开始冷却
            mana.consume(mp_cost as i32);
            cooldowns.start_cooldown(spell_id, current_time);
            
            println!("⚔️ 使用技能: spell_id={}, level={}, MP消耗={}", 
                spell_id, level, mp_cost);
            
            // 6. 发送网络命令 (TODO)
            // let _ = network_tx.send(NetworkCommand::Magic {
            //     spell: spell_id,
            //     target_id: None,
            //     location: None,
            // });
            
            break;
        }
        
        if !player_found {
            return Err(SkillError::PlayerNotFound);
        }
        
        if cooldown_error {
            return Err(SkillError::OnCooldown);
        }
        
        if mana_error {
            return Err(SkillError::InsufficientMana);
        }
        
        Ok(())
    }
}

#[derive(Debug)]
pub enum SkillError {
    InvalidSkillBar,
    EmptySlot,
    PlayerNotFound,
    OnCooldown,
    InsufficientMana,
}
```

---

### 阶段4: 修改GameScene (集成)

#### 4.1 修改 update 方法
```rust
impl Scene for GameScene {
    fn update(
        &mut self, 
        ctx: &mut Context, 
        world: &mut World,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>
    ) -> GameResult<Option<SceneType>> {
        // ... 帧率限制代码 ...
        
        // 1. 更新游戏逻辑系统
        AnimationSystem::update(world, animation_count);
        CameraSystem::update(world);
        PlayerSystem::update(world);
        MonsterSystem::update(world, delta_time);
        
        // 2. 更新UI系统 (最后执行,处理本帧产生的事件)
        self.ui_system.update(world);
        
        Ok(None)
    }
    
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult {
        // 渲染游戏世界
        // ... 地图、角色、怪物渲染代码 ...
        
        // 获取当前时间
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 渲染UI (使用UISystem)
        self.ui_system.draw(ctx, canvas, world, current_time)?;
        
        Ok(())
    }
}
```

#### 4.2 修改事件处理
```rust
impl GameScene {
    /// 处理网络事件
    pub fn handle_network_event(&mut self, world: &mut World, event: &GameEvent) {
        // NetworkSystem处理实体同步
        self.network_system.process_event(world, event);
        
        // 生成UI事件 (由UISystem在下一帧处理)
        match event {
            GameEvent::ChatReceived { message } => {
                world.spawn((ChatMessageEvent {
                    text: format!("[{}] {}", message.sender, message.text),
                    chat_type: ChatType::Normal,
                }));
            }
            
            GameEvent::SystemMessage { message } => {
                world.spawn((ChatMessageEvent {
                    text: message.clone(),
                    chat_type: ChatType::System,
                }));
            }
            
            GameEvent::GoldChanged { gold } => {
                world.spawn((GoldChangedEvent { gold: *gold }));
            }
            
            GameEvent::ItemGained { item, .. } => {
                if let Some(ref info) = item.info {
                    world.spawn((ItemGainedEvent {
                        item_name: info.name.clone(),
                    }));
                }
            }
            
            _ => {}
        }
    }
    
    /// 处理键盘输入
    fn on_key_down(&mut self, world: &mut World, ...) {
        match keycode {
            // F1-F8 使用技能
            KeyCode::F1 => {
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                
                let _ = SkillSystem::try_use_skill(
                    world,
                    self.skillbar_entities[0],
                    0,
                    current_time,
                    network_tx,
                );
            }
            // ... F2-F8 类似 ...
            
            _ => {}
        }
    }
}
```

---

## 重构收益

### 1. 架构一致性
- ✅ 所有数据都在 World 中,符合 ECS 原则
- ✅ UI 成为可查询的组件,可被多个系统访问

### 2. 职责分离
- ✅ `GameScene` 只负责协调,不处理业务逻辑
- ✅ `UISystem` 统一处理所有 UI 更新
- ✅ `SkillSystem` 独立处理技能逻辑

### 3. 可测试性
- ✅ 系统可以独立测试,不依赖 Scene
- ✅ 事件系统使得逻辑流程清晰可追踪

### 4. 可扩展性
- ✅ 添加新UI只需要添加组件和事件
- ✅ 添加新技能类型只需扩展 SkillSystem

### 5. 性能优化潜力
- ✅ UI查询可以被缓存
- ✅ 事件处理可以批量化
- ✅ 未来可以并行化系统更新

---

## 实施建议

1. **渐进式重构**: 不要一次性改完,按阶段进行
2. **保持功能不变**: 每个阶段完成后确保功能正常
3. **添加测试**: 为新的 System 添加单元测试
4. **文档更新**: 更新架构文档说明新设计

---

## 参考资源

- [Bevy ECS 设计哲学](https://bevyengine.org/learn/book/getting-started/ecs/)
- [Specs ECS 最佳实践](https://specs.amethyst.rs/docs/tutorials/)
- [ECS FAQ](https://github.com/SanderMertens/ecs-faq)
