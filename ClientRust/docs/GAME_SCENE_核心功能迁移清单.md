# GameScene 核心功能迁移清单

## 🎯 目标

将 C# 版 `GameScene.cs` (13605行) 的核心功能迁移到 Rust ECS 架构

---

## 📋 C# GameScene.Process() 完整分析

### 当前 C# 实现的主要逻辑

```csharp
public void Process()
{
    // 1️⃣ 处理门动画 (300-400ms 间隔)
    Processdoors();
    
    // 2️⃣ 更新玩家对象
    User.Process();  // UserObject.Process()
    
    // 3️⃣ 更新所有其他对象 (怪物/NPC/掉落物)
    for (int i = ObjectsList.Count - 1; i >= 0; i--)
    {
        if (ObjectsList[i] == User) continue;
        ObjectsList[i].Process();
    }
    
    // 4️⃣ 更新特效对象
    for (int i = Effects.Count - 1; i >= 0; i--)
        Effects[i].Process();
    
    // 5️⃣ 清理无效目标
    if (TargetObject != null && TargetObject is MonsterObject && TargetObject.AI == 64)
        TargetObjectID = 0;
    
    // 6️⃣ 检查输入 (键盘/鼠标/技能)
    CheckInput();
    
    // 7️⃣ 鼠标悬停检测 (5x5格子范围)
    MapObject bestmouseobject = null;
    for (int y = MapLocation.Y + 2; y >= MapLocation.Y - 2; y--)
    {
        for (int x = MapLocation.X + 2; x >= MapLocation.X - 2; x--)
        {
            CellInfo cell = M2CellInfo[x, y];
            if (cell.CellObjects == null) continue;
            
            for (int i = cell.CellObjects.Count - 1; i >= 0; i--)
            {
                MapObject ob = cell.CellObjects[i];
                if (ob.MouseOver(CMain.MPoint))
                {
                    if (ob.Dead && !Settings.TargetDead) continue;
                    MouseObjectID = ob.ObjectID;
                    Redraw();
                    return;
                }
            }
        }
    }
}
```

---

## 🔄 迁移映射

### 已完成 ✅

| C# 功能 | Rust ECS 对应 | 位置 | 完成度 |
|---------|--------------|------|--------|
| User.Process() | PlayerSystem::update() | src/ecs/systems/player_system.rs | 80% |
| Camera更新 | CameraSystem::update() | src/ecs/systems/camera_system.rs | 90% |
| 动画更新 | AnimationSystem::update() | src/ecs/systems/animation_system.rs | 95% |
| 地图渲染 | RenderSystem::draw_tiles() | src/ecs/systems/render_system.rs | 90% |
| 键盘输入 | on_key_down() | game_scene.rs | 100% |
| 鼠标基础跟踪 | on_mouse_*() | game_scene.rs | 60% |
| 网络同步 | NetworkSystem | src/ecs/systems/network_system.rs | 70% |

### 未完成 ❌

| C# 功能 | 需要实现 | 优先级 | 复杂度 |
|---------|---------|--------|--------|
| ObjectsList[i].Process() | MonsterSystem::update() | 🔴 高 | 高 |
| Effects[i].Process() | EffectSystem::update() | 🔴 高 | 中 |
| Processdoors() | DoorSystem::update() | 🟡 中 | 低 |
| CheckInput() | InputSystem::process() | 🔴 高 | 中 |
| 鼠标悬停检测 | MouseHoverSystem | 🔴 高 | 中 |
| 尸体处理 | 集成到 HealthSystem | 🟡 中 | 低 |

---

## 🎮 第一优先级: 对象更新系统

### 1. MonsterSystem (怪物系统)

#### C# 原实现 (MonsterObject.Process)
```csharp
public override void Process()
{
    // 1. 处理当前动作
    ProcessFrames();  // 更新动画帧
    
    // 2. AI 逻辑
    if (!Dead && AI != 0)
    {
        ProcessAI();  // 追击玩家、攻击、巡逻
    }
    
    // 3. 移动逻辑
    if (Moving)
    {
        ProcessMovement();  // 平滑移动
    }
    
    // 4. Buff/Debuff
    ProcessBuffs();
    
    // 5. 聊天气泡
    ProcessChat();
}
```

#### Rust ECS 实现计划
```rust
// 创建文件: src/ecs/systems/monster_system.rs

pub struct MonsterSystem;

impl MonsterSystem {
    pub fn update(world: &mut World, delta_time: f32) {
        // 1. 更新怪物AI
        for (entity, (monster, pos, target, health)) in 
            world.query::<(&mut MonsterComp, &mut Position, &mut AIState, &Health)>().iter() 
        {
            if health.current <= 0 { continue; }  // 跳过死亡怪物
            
            // AI逻辑
            match monster.ai_type {
                1 => Self::ai_melee_attack(entity, pos, target, world),
                2 => Self::ai_ranged_attack(entity, pos, target, world),
                3 => Self::ai_patrol(entity, pos, monster, world),
                _ => {}
            }
        }
        
        // 2. 更新移动
        for (entity, (pos, velocity)) in world.query::<(&mut Position, &Velocity)>().iter() {
            pos.x += velocity.dx * delta_time;
            pos.y += velocity.dy * delta_time;
        }
    }
    
    fn ai_melee_attack(entity: Entity, pos: &mut Position, target: &mut AIState, world: &World) {
        // 查找玩家
        let player_pos = Self::find_player_position(world);
        
        // 计算距离
        let distance = Self::distance(pos, &player_pos);
        
        if distance < 1.5 {
            // 攻击
            target.current_action = AIAction::Attack;
        } else if distance < 10.0 {
            // 追击
            target.current_action = AIAction::Chase;
            Self::move_towards(pos, &player_pos);
        } else {
            // 巡逻
            target.current_action = AIAction::Idle;
        }
    }
}
```

**任务清单**:
- [ ] 创建 `MonsterSystem`
- [ ] 实现 AI 类型 1-10 (近战/远程/BOSS等)
- [ ] 集成寻路算法
- [ ] 实现攻击判定
- [ ] 添加技能施放逻辑

---

### 2. EffectSystem (特效系统)

#### C# 原实现 (Effect.Process)
```csharp
public override void Process()
{
    // 1. 播放音效
    if (Start < CMain.Time)
    {
        PlaySound();
        Start = long.MaxValue;
    }
    
    // 2. 更新动画帧
    ProcessFrames();
    
    // 3. 检查是否结束
    if (FrameIndex >= Frame.Count)
    {
        Remove();  // 移除特效
    }
}
```

#### Rust ECS 实现计划
```rust
// 创建文件: src/ecs/systems/effect_system.rs

pub struct EffectSystem;

impl EffectSystem {
    pub fn update(world: &mut World, delta_time: f32) {
        let mut to_remove = Vec::new();
        
        // 1. 更新所有特效
        for (entity, (effect, anim, lifetime)) in 
            world.query::<(&SpellComp, &mut AnimationComp, &mut Lifetime)>().iter() 
        {
            // 更新生命周期
            lifetime.elapsed += delta_time;
            
            // 播放音效 (首帧)
            if anim.frame_index == 0 && !effect.sound_played {
                Self::play_sound(effect.sound_id);
            }
            
            // 检查是否结束
            if lifetime.elapsed >= lifetime.duration {
                to_remove.push(entity);
            }
        }
        
        // 2. 移除过期特效
        for entity in to_remove {
            let _ = world.despawn(entity);
        }
    }
    
    pub fn spawn_effect(world: &mut World, effect_type: u16, pos: Position) -> Entity {
        world.spawn((
            SpellComp {
                spell_type: effect_type,
                sound_id: Self::get_sound_for_effect(effect_type),
                sound_played: false,
            },
            pos,
            AnimationComp {
                action: MirAction::Spell,
                frame_index: 0,
                frame_count: Self::get_frame_count(effect_type),
                frame_interval: 100,
                last_frame_time: 0,
            },
            Lifetime {
                duration: 1.0,
                elapsed: 0.0,
            },
        ))
    }
}
```

**任务清单**:
- [ ] 创建 `EffectSystem`
- [ ] 实现特效生成
- [ ] 添加音效触发
- [ ] 实现粒子效果
- [ ] 添加特效渲染

---

### 3. InputSystem (输入检测系统)

#### C# 原实现 (CheckInput)
```csharp
private void CheckInput()
{
    // 1. 鼠标左键 - 移动/攻击
    if (CMain.MLeftDown)
    {
        if (TargetObject != null && TargetObject.Attackable)
        {
            AutoHit = true;  // 自动攻击
        }
        else
        {
            AutoRun = true;  // 自动移动
        }
    }
    
    // 2. 鼠标右键 - 跑步
    if (CMain.MRightDown)
    {
        AutoRun = true;
        CanRun = true;
    }
    
    // 3. 技能快捷键 F1-F8
    for (int i = 0; i < 8; i++)
    {
        if (CMain.InputKeys.Forward(KeybindOptions.Bar1Skill1 + i))
        {
            UseSpell(i);
        }
    }
    
    // 4. Tab 键 - 切换目标
    if (CMain.InputKeys.Forward(KeybindOptions.NextTarget))
    {
        SelectNextTarget();
    }
}
```

#### Rust ECS 实现计划
```rust
// 扩展 game_scene.rs 的输入处理

impl GameScene {
    fn check_input(&mut self, world: &mut World, ctx: &Context) {
        // 查询鼠标输入状态
        if let Some((_, mouse)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
            // 1. 鼠标左键长按 - 移动
            if mouse.left_pressed {
                mouse.left_press_time += 1;
                
                if mouse.left_press_time > 10 {  // 长按阈值
                    // 转换屏幕坐标到世界坐标
                    let world_pos = self.screen_to_world(mouse.x, mouse.y, world);
                    
                    // 发送行走命令
                    self.move_to_position(world, world_pos);
                }
            }
            
            // 2. 鼠标右键长按 - 跑步
            if mouse.right_pressed {
                mouse.right_press_time += 1;
                
                if mouse.right_press_time > 10 {
                    let world_pos = self.screen_to_world(mouse.x, mouse.y, world);
                    self.run_to_position(world, world_pos);
                }
            }
        }
    }
    
    fn move_to_position(&self, world: &mut World, target: (f32, f32)) {
        // 查找玩家实体
        for (entity, (player, pos)) in world.query_mut::<(&mut Player, &Position)>() {
            // 计算路径
            let path = self.find_path(pos, target, world);
            
            // 设置玩家路径
            player.path = path;
            player.is_moving = true;
            player.move_mode = MoveMode::Walk;
        }
    }
}
```

**任务清单**:
- [ ] 实现鼠标左键长按移动
- [ ] 实现鼠标右键长按跑步
- [ ] 添加技能快捷键 (F1-F8)
- [ ] 实现 Tab 切换目标
- [ ] 添加双击检测
- [ ] 实现拖拽物品

---

### 4. MouseHoverSystem (鼠标悬停检测)

#### C# 原实现
```csharp
// 在 5x5 格子范围内检测鼠标悬停
for (int y = MapLocation.Y + 2; y >= MapLocation.Y - 2; y--)
{
    for (int x = MapLocation.X + 2; x >= MapLocation.X - 2; x--)
    {
        CellInfo cell = M2CellInfo[x, y];
        if (cell.CellObjects == null) continue;
        
        for (int i = cell.CellObjects.Count - 1; i >= 0; i--)
        {
            MapObject ob = cell.CellObjects[i];
            if (ob.MouseOver(CMain.MPoint))
            {
                MouseObjectID = ob.ObjectID;
                return;
            }
        }
    }
}
```

#### Rust ECS 实现计划
```rust
// 创建文件: src/ecs/systems/mouse_hover_system.rs

pub struct MouseHoverSystem;

impl MouseHoverSystem {
    pub fn update(world: &mut World, mouse_pos: (f32, f32)) {
        let mut hovered_entity: Option<Entity> = None;
        let mut best_z = f32::MIN;
        
        // 获取鼠标世界坐标
        let world_pos = Self::screen_to_world(mouse_pos, world);
        
        // 遍历所有可交互对象
        for (entity, (pos, sprite)) in world.query::<(&Position, &SpriteComp)>().iter() {
            // 检查鼠标是否在对象范围内
            if Self::point_in_sprite(world_pos, pos, sprite) {
                // 记录Z轴最高的对象
                if pos.y > best_z {
                    best_z = pos.y;
                    hovered_entity = Some(entity);
                }
            }
        }
        
        // 更新悬停状态
        Self::set_hovered_object(world, hovered_entity);
    }
    
    fn point_in_sprite(mouse: (f32, f32), pos: &Position, sprite: &SpriteComp) -> bool {
        let (mx, my) = mouse;
        
        // 简化的矩形碰撞检测
        let half_w = sprite.width as f32 / 2.0;
        let half_h = sprite.height as f32 / 2.0;
        
        mx >= pos.x - half_w && mx <= pos.x + half_w &&
        my >= pos.y - half_h && my <= pos.y + half_h
    }
}
```

**任务清单**:
- [ ] 创建 `MouseHoverSystem`
- [ ] 实现精确碰撞检测 (考虑透明像素)
- [ ] 添加 Z-order 排序
- [ ] 实现悬停状态更新
- [ ] 添加悬停高亮效果

---

## 🎨 第二优先级: UI 系统完善

### 1. 背包系统 (InventoryDialog)

#### C# 原实现特性
- 60格背包空间
- 物品拖拽
- 右键使用物品
- Shift+右键快速出售
- 物品堆叠
- 物品提示框

#### Rust ECS 实现
```rust
// src/ecs/ui/inventory.rs

pub struct InventoryUI {
    pub items: [Option<UserItem>; 60],
    pub selected_slot: Option<usize>,
    pub dragging_item: Option<(usize, UserItem)>,
    pub gold: u32,
}

impl InventoryUI {
    pub fn handle_click(&mut self, slot: usize, button: MouseButton, shift: bool) {
        match button {
            MouseButton::Left => {
                if shift {
                    self.quick_sell(slot);
                } else {
                    self.start_drag(slot);
                }
            }
            MouseButton::Right => {
                self.use_item(slot);
            }
            _ => {}
        }
    }
    
    pub fn use_item(&mut self, slot: usize) {
        if let Some(item) = &self.items[slot] {
            match item.item_type {
                ItemType::Potion => { /* 使用药水 */ }
                ItemType::Scroll => { /* 使用卷轴 */ }
                ItemType::Equipment => { /* 装备物品 */ }
                _ => {}
            }
        }
    }
}
```

---

### 2. 聊天系统增强

#### 需要添加的功能
- [ ] 聊天输入框激活 (Enter键)
- [ ] 频道切换 (全体/组队/公会/私聊)
- [ ] 物品链接 (Ctrl+点击物品)
- [ ] 表情系统
- [ ] 聊天过滤

---

## 🔧 第三优先级: 战斗系统

### 1. 技能系统

```rust
// src/ecs/systems/spell_system.rs

pub struct SpellSystem;

impl SpellSystem {
    pub fn cast_spell(
        world: &mut World,
        caster: Entity,
        spell_id: u8,
        target: Option<Entity>,
        target_pos: Option<Position>
    ) -> Result<(), String> {
        // 1. 验证技能
        let spell_info = Self::get_spell_info(spell_id)?;
        
        // 2. 检查魔法值
        let mana = world.get::<&mut Mana>(caster)?;
        if mana.current < spell_info.mana_cost {
            return Err("魔法值不足".to_string());
        }
        
        // 3. 检查冷却
        let cooldown = world.get::<&mut SpellCooldown>(caster)?;
        if !cooldown.is_ready(spell_id) {
            return Err("技能冷却中".to_string());
        }
        
        // 4. 扣除魔法值
        mana.current -= spell_info.mana_cost;
        
        // 5. 设置冷却
        cooldown.set_cooldown(spell_id, spell_info.cooldown_ms);
        
        // 6. 创建特效
        Self::spawn_spell_effect(world, spell_id, target_pos);
        
        // 7. 造成伤害
        if let Some(target_entity) = target {
            Self::apply_damage(world, target_entity, spell_info.damage);
        }
        
        Ok(())
    }
}
```

---

## 📅 实施时间表

### Week 1-2: 核心对象系统
- [ ] MonsterSystem 基础框架
- [ ] EffectSystem 基础框架
- [ ] InputSystem 鼠标移动
- [ ] MouseHoverSystem

### Week 3-4: AI 和战斗
- [ ] 怪物AI (3种类型)
- [ ] 寻路集成
- [ ] 基础战斗系统
- [ ] 技能施放

### Week 5-6: UI 完善
- [ ] 背包拖拽
- [ ] 装备系统
- [ ] 聊天输入
- [ ] 技能栏

### Week 7-8: 优化和测试
- [ ] 性能优化
- [ ] 与 C# 版本行为对比测试
- [ ] Bug 修复
- [ ] 文档完善

---

## 🎯 成功标准

### 核心功能对齐
- ✅ 玩家移动与 C# 版本完全一致
- ✅ 怪物AI 行为与 C# 版本匹配
- ✅ 战斗数值计算相同
- ✅ UI 操作体验一致

### 性能指标
- ✅ 60 FPS @ 1000 对象
- ✅ 内存占用 < 100MB
- ✅ 加载时间 < 2s

### 代码质量
- ✅ 所有系统有单元测试
- ✅ 文档覆盖率 > 80%
- ✅ 无 unsafe 代码
- ✅ Clippy 无警告

---

**下一步行动**: 从 `MonsterSystem` 开始，逐步实现对象更新逻辑。

**最后更新**: 2025-10-21
