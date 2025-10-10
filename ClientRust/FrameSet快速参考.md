# FrameSet 快速参考

## 导入

```rust
use crate::objects::frames::{
    Frame, FrameSet,
    PLAYER_FRAMES,
    DEFAULT_NPC_FRAMES,
    DEFAULT_MONSTER_FRAMES,
    DRAGON_STATUE_FRAMES,
    GREAT_FOX_SPIRIT_FRAMES,
    HELL_BOMB_FRAMES,
    CAVE_STATUE_FRAMES,
    get_player_frame,
    get_default_npc_frame,
    get_default_monster_frame,
};
use mir2_shared::enums::MirAction;
```

## 常用操作

### 1. 获取玩家帧数据

```rust
// 方法 1: 直接访问静态数据
if let Some(frame) = PLAYER_FRAMES.get(&MirAction::Standing) {
    println!("Start: {}, Count: {}", frame.start, frame.count);
}

// 方法 2: 使用辅助函数
if let Some(frame) = get_player_frame(MirAction::Attack1) {
    let offset = frame.offset();
    println!("Attack offset: {}", offset);
}
```

### 2. 获取 NPC 帧数据

```rust
if let Some(frame) = get_default_npc_frame(MirAction::Standing) {
    println!("NPC standing: start={}, count={}, interval={}", 
        frame.start, frame.count, frame.interval);
}
```

### 3. 获取怪物帧数据

```rust
if let Some(frame) = get_default_monster_frame(MirAction::Die) {
    println!("Monster die animation: {} frames", frame.count);
}
```

### 4. 访问特殊实体帧数据

```rust
// DragonStatue 第一个变体
let dragon_frames = &DRAGON_STATUE_FRAMES[0];
if let Some(frame) = dragon_frames.get(&MirAction::Standing) {
    println!("DragonStatue frame: {}", frame.start);
}

// GreatFoxSpirit 等级 3
let fox_frames = &GREAT_FOX_SPIRIT_FRAMES[3];
if let Some(frame) = fox_frames.get(&MirAction::Attack1) {
    println!("Fox level 3 attack frame");
}

// HellBomb 第二个变体
let bomb_frames = &HELL_BOMB_FRAMES[1];
if let Some(frame) = bomb_frames.get(&MirAction::Standing) {
    println!("HellBomb uses blend: {}", frame.blend);
}
```

## Frame 属性

```rust
let frame = get_player_frame(MirAction::Walking).unwrap();

// 基础属性
println!("起始帧: {}", frame.start);
println!("帧数: {}", frame.count);
println!("跳过帧: {}", frame.skip);
println!("帧间隔(ms): {}", frame.interval);

// 效果层属性
println!("效果起始帧: {}", frame.effect_start);
println!("效果帧数: {}", frame.effect_count);
println!("效果跳过帧: {}", frame.effect_skip);
println!("效果帧间隔(ms): {}", frame.effect_interval);

// 播放控制
println!("反向播放: {}", frame.reverse);
println!("混合模式: {}", frame.blend);

// 计算属性
println!("总偏移: {}", frame.offset());          // count + skip
println!("效果偏移: {}", frame.effect_offset()); // effect_count + effect_skip
```

## 创建自定义帧

```rust
// 完整构造
let frame1 = Frame::new(
    0,    // start
    4,    // count
    0,    // skip
    500,  // interval
    0,    // effect_start
    8,    // effect_count
    0,    // effect_skip
    250   // effect_interval
);

// 简化构造（无效果层）
let frame2 = Frame::basic(32, 6, 0, 100);

// 使用构建器模式
let frame3 = Frame::basic(52, 9, -9, 100)
    .with_blend(true)
    .with_reverse(false);
```

## 动作枚举参考

### 玩家通用动作
- `MirAction::Standing` - 站立
- `MirAction::Walking` - 行走
- `MirAction::Running` - 跑步
- `MirAction::Stance` - 姿态
- `MirAction::Attack1/2/3/4` - 攻击
- `MirAction::Spell` - 施法
- `MirAction::Harvest` - 收获
- `MirAction::Struck` - 受击
- `MirAction::Die` - 死亡
- `MirAction::Dead` - 尸体
- `MirAction::Revive` - 复活
- `MirAction::Mine` - 挖矿

### 职业特殊动作
#### 刺客
- `MirAction::Sneek` - 潜行
- `MirAction::DashAttack` - 冲刺攻击
- `MirAction::Lunge` - 突刺

#### 弓箭手
- `MirAction::WalkingBow` - 持弓行走
- `MirAction::RunningBow` - 持弓奔跑
- `MirAction::AttackRange1/2/3` - 远程攻击
- `MirAction::Jump` - 跳跃

### 坐骑动作
- `MirAction::MountStanding` - 骑乘站立
- `MirAction::MountWalking` - 骑乘行走
- `MirAction::MountRunning` - 骑乘奔跑
- `MirAction::MountStruck` - 骑乘受击
- `MirAction::MountAttack` - 骑乘攻击

### 钓鱼动作
- `MirAction::FishingCast` - 抛竿
- `MirAction::FishingWait` - 等待
- `MirAction::FishingReel` - 收线

## 特殊实体变体数量

| 实体类型 | 变体数量 | 说明 |
|---------|---------|------|
| DragonStatue | 6 | 龙像雕塑 |
| GreatFoxSpirit | 5 | 九尾狐精等级 |
| HellBomb | 3 | 地狱炸弹类型 |
| CaveStatue | 2 | 洞穴雕像 |

## 常见模式

### 遍历所有玩家动作

```rust
for (action, frame) in PLAYER_FRAMES.iter() {
    println!("{:?}: start={}, count={}, interval={}ms",
        action, frame.start, frame.count, frame.interval);
}
```

### 检查动作是否存在

```rust
if PLAYER_FRAMES.contains_key(&MirAction::FishingCast) {
    println!("Player can fish!");
}
```

### 计算动画总时长

```rust
fn calculate_duration(frame: &Frame) -> u32 {
    frame.count as u32 * frame.interval as u32
}

if let Some(frame) = get_player_frame(MirAction::Die) {
    let duration = calculate_duration(frame);
    println!("Death animation duration: {}ms", duration);
}
```

### 处理特效层

```rust
if let Some(frame) = get_player_frame(MirAction::Attack1) {
    if frame.effect_count > 0 {
        println!("Attack has {} effect frames", frame.effect_count);
        println!("Effect duration: {}ms", 
            frame.effect_count as u32 * frame.effect_interval as u32);
    }
}
```

## 注意事项

1. **负数 skip 值**: 某些帧使用负数 skip（如龙像），用于特殊帧索引计算
2. **效果层**: 主要用于玩家动作，显示武器、翅膀等效果
3. **混合模式**: HellBomb 使用 `blend=true`，CaveStatue 明确使用 `blend=false`
4. **反向播放**: 复活动作通常设置 `reverse=true`
5. **线程安全**: 所有静态数据使用 `LazyLock`，可安全地在多线程中访问

## 性能提示

- 静态数据在首次访问时初始化，之后访问无开销
- 使用 `get()` 方法比重复克隆数据更高效
- 考虑缓存频繁访问的帧引用

## 测试

运行测试以验证帧数据的完整性：

```bash
cargo test frames_test --lib
```

所有 20 个测试应该通过。
