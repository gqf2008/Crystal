# 🎬 角色动画系统实现说明

## 问题诊断

**原始问题**: 游戏场景中角色纹理不对,且没有动画

### 根本原因

1. **错误的纹理库**: 使用了 **ChrSel** (角色选择界面的静态预览图)
   - ChrSel 只有 200 帧静态预览图 (每职业每性别每方向 1 帧)
   - 应该使用 **CArmours/AArmours/ARArmours** (游戏内动画装备纹理库)

2. **缺失动画系统**: 没有 FrameSet, Frame, FrameIndex 等动画管理
   - 只有简单的静态帧计算: `class_base + gender_offset + direction`
   - 缺少动画循环更新

3. **缺失分层渲染**: 没有实现 Body/Hair/Weapon/Wings 分层绘制

## 实现方案

### 阶段 1: 最小可行版 (MVP) ✅

目标: 显示静态站立的战士角色,使用正确的装备库

#### 1.1 添加动画跟踪字段

**文件**: `src/objects/player_object.rs`

```rust
// ==================== Animation Tracking ====================
/// Last frame update time (for animation timing)
pub last_frame_time: Instant,

/// Base frame index for current action (e.g., Standing=0, Walking=32)
pub action_frame_start: i32,

/// Number of frames per direction for current action
pub frames_per_direction: i32,
```

#### 1.2 添加帧计算方法

```rust
/// Get the draw frame index for current animation state
/// 
/// Mirrors C# PlayerObject.DrawFrame property:
/// DrawFrame = Frame.Start + Direction * Frame.Count + FrameIndex
pub fn get_draw_frame(&self) -> i32 {
    let direction_offset = (self.map_object.direction as i32) * self.frames_per_direction;
    self.action_frame_start + direction_offset + self.frame_index
}

/// Get armour offset based on gender and class
/// 
/// Warrior/Wizard/Taoist: Male=0, Female=808
/// Assassin: Male=0, Female=808
/// Archer (depends on action):
///   Walking/Running/Attack1: Male=0, Female=352
///   Other actions: Male=0, Female=808
pub fn get_armour_offset(&self) -> i32 {
    match self.class {
        MirClass::Warrior | MirClass::Wizard | MirClass::Taoist | MirClass::Assassin => {
            if self.gender == MirGender::Male { 0 } else { 808 }
        }
        MirClass::Archer => {
            let alt_anim = matches!(
                self.current_action,
                MirAction::Walking | MirAction::Running | MirAction::Attack1
            );
            if alt_anim {
                if self.gender == MirGender::Male { 0 } else { 352 }
            } else {
                if self.gender == MirGender::Male { 0 } else { 808 }
            }
        }
    }
}

/// Get final frame index for drawing (DrawFrame + Offset)
pub fn get_final_frame(&self) -> i32 {
    self.get_draw_frame() + self.get_armour_offset()
}
```

#### 1.3 添加动作设置方法

```rust
/// Set current action and update animation frame parameters
pub fn set_current_action(&mut self, action: MirAction) {
    if self.current_action == action {
        return;
    }
    
    self.current_action = action;
    self.frame_index = 0;
    self.last_frame_time = Instant::now();
    
    // Set frame parameters based on action
    match action {
        MirAction::Standing => {
            self.action_frame_start = 0;
            self.frames_per_direction = 4;
            self.frame_interval = 500; // ms
        }
        MirAction::Walking => {
            self.action_frame_start = 32;
            self.frames_per_direction = 6;
            self.frame_interval = 100; // ms
        }
        MirAction::Running => {
            self.action_frame_start = 80;
            self.frames_per_direction = 6;
            self.frame_interval = 80; // ms
        }
        MirAction::Attack1 => {
            self.action_frame_start = 128;
            self.frames_per_direction = 6;
            self.frame_interval = 100; // ms
        }
        _ => {
            // Default to standing
            self.action_frame_start = 0;
            self.frames_per_direction = 4;
            self.frame_interval = 500; // ms
        }
    }
}
```

#### 1.4 添加动画更新方法

```rust
/// Update animation frame (called each frame in GameScene::update)
pub fn update_animation(&mut self) {
    let now = Instant::now();
    let elapsed = now.duration_since(self.last_frame_time).as_millis() as i32;
    
    if elapsed >= self.frame_interval {
        self.frame_index = (self.frame_index + 1) % self.frames_per_direction;
        self.last_frame_time = now;
    }
}
```

#### 1.5 添加新的装备库类型

**文件**: `src/graphics/libraries.rs`

```rust
pub enum LibraryName {
    // ... 现有库
    
    // ==================== 角色装备库 (NEW) ====================
    /// 通用装备库 (Warrior/Wizard/Taoist)
    /// CArmour/0000.Lib - CArmour/0999.Lib
    CArmours(usize),
    
    /// 刺客装备库 (Assassin)
    /// AArmour/0000.Lib - AArmour/0999.Lib
    AArmours(usize),
    
    /// 弓箭手装备库 (Archer alternative animation)
    /// ARArmour/0000.Lib - ARArmour/0999.Lib
    ARArmours(usize),
    
    /// 通用发型库 (Warrior/Wizard/Taoist)
    CHair(usize),
    
    /// 刺客发型库
    AHair(usize),
    
    /// 弓箭手发型库
    ARHair(usize),
    
    /// 通用武器库
    CWeapons(usize),
    
    /// 弓箭手武器库
    ARWeapons(usize),
    
    /// 人物特效库 (翅膀等)
    CHumEffect(usize),
}

impl LibraryName {
    pub fn default_path(&self) -> String {
        match self {
            // ... 现有路径
            
            // 角色装备库路径
            LibraryName::CArmours(idx) => format!("CArmour/{:04}", idx),
            LibraryName::AArmours(idx) => format!("AArmour/{:04}", idx),
            LibraryName::ARArmours(idx) => format!("ARArmour/{:04}", idx),
            LibraryName::CHair(idx) => format!("CHair/{:04}", idx),
            LibraryName::AHair(idx) => format!("AHair/{:04}", idx),
            LibraryName::ARHair(idx) => format!("ARHair/{:04}", idx),
            LibraryName::CWeapons(idx) => format!("CWeapon/{:04}", idx),
            LibraryName::ARWeapons(idx) => format!("ARWeapon/{:04}", idx),
            LibraryName::CHumEffect(idx) => format!("CHumEffect/{:04}", idx),
        }
    }
}
```

#### 1.6 修改角色绘制逻辑

**文件**: `src/scenes/game_scene.rs`

**旧代码 (错误)**:
```rust
// ❌ WRONG: Using ChrSel library (static preview)
let class_base = match user.player.class {
    MirClass::Warrior => 0,
    MirClass::Wizard => 40,
    // ...
};
let frame_index = class_base + gender_offset + direction;

if let Some(lib_arc) = get_library(LibraryName::ChrSel) {
    lib.draw_with_color(ctx, canvas, frame_index, ...);
}
```

**新代码 (正确)**:
```rust
// ✅ CORRECT: Using CArmours library (animated equipment)
let final_frame = user.player.get_final_frame() as usize;

// 选择装备库
let library_name = match user.player.class {
    MirClass::Warrior | MirClass::Wizard | MirClass::Taoist => {
        LibraryName::CArmours(user.player.armour as usize)
    }
    MirClass::Assassin => {
        LibraryName::AArmours(user.player.armour as usize)
    }
    MirClass::Archer => {
        let alt_anim = matches!(
            user.player.current_action,
            MirAction::Walking | MirAction::Running | MirAction::Attack1
        );
        if alt_anim {
            LibraryName::ARArmours(user.player.armour as usize)
        } else {
            LibraryName::CArmours(user.player.armour as usize)
        }
    }
};

if let Some(lib_arc) = get_library(library_name.clone()) {
    if let Ok(mut lib) = lib_arc.try_lock() {
        lib.draw_with_color(ctx, canvas, final_frame, screen_x, screen_y, Color::WHITE, true)?;
    }
}
```

#### 1.7 在 update 中更新动画

```rust
fn update(&mut self, ctx: &mut ggez::Context, _delta_time: f32) {
    // ... 现有代码
    
    // ==================== 更新玩家角色动画 (NEW) ====================
    if let Some(ref mut user) = self.user {
        user.player.update_animation();
    }
    
    // ... 后续代码
}
```

## 帧布局参考

### CArmours 库 (通用装备) - 男性角色

| 动作 | 起始帧 | 每方向帧数 | 总帧数 | 帧索引范围 |
|------|--------|-----------|--------|-----------|
| Standing | 0 | 4 | 32 | 0-31 |
| Walking | 32 | 6 | 48 | 32-79 |
| Running | 80 | 6 | 48 | 80-127 |
| Attack1 | 128 | 6 | 48 | 128-175 |
| Attack2 | 176 | 6 | 48 | 176-223 |
| Attack3 | 224 | 8 | 64 | 224-287 |
| Spell | 288 | 6 | 48 | 288-335 |
| Harvest | 336 | 2 | 16 | 336-351 |
| Struck | 352 | 2 | 16 | 352-367 |
| Die | 368 | 10 | 80 | 368-447 |
| Dead | 448 | 1 | 8 | 448-455 |
| Show | 456 | 10 | 80 | 456-535 |
| Hide | 536 | 10 | 80 | 536-615 |

### CArmours 库 (通用装备) - 女性角色

**偏移量**: +808

| 动作 | 起始帧 | 每方向帧数 | 帧索引范围 |
|------|--------|-----------|-----------|
| Standing | 808 | 4 | 808-839 |
| Walking | 840 | 6 | 840-887 |
| Running | 888 | 6 | 888-935 |
| Attack1 | 936 | 6 | 936-983 |
| ... | ... | ... | ... |

### 方向映射

| 方向 | 枚举值 | 索引 |
|------|--------|------|
| Up | 0 | 0 |
| UpRight | 1 | 1 |
| Right | 2 | 2 |
| DownRight | 3 | 3 |
| Down | 4 | 4 |
| DownLeft | 5 | 5 |
| Left | 6 | 6 |
| UpLeft | 7 | 7 |

### 计算示例

**男战士站立朝南 (Down)**:
- `current_action` = Standing
- `action_frame_start` = 0
- `frames_per_direction` = 4
- `direction` = Down = 4
- `frame_index` = 0~3 (循环)
- `DrawFrame` = 0 + 4*4 + 0~3 = **16~19**
- `ArmourOffSet` = 0 (男性)
- `final_frame` = **16~19**

**女战士站立朝南 (Down)**:
- `DrawFrame` = 0 + 4*4 + 0~3 = 16~19
- `ArmourOffSet` = 808 (女性)
- `final_frame` = 16 + 808 = **824~827**

**男战士行走朝东 (Right)**:
- `action_frame_start` = 32
- `frames_per_direction` = 6
- `direction` = Right = 2
- `frame_index` = 0~5 (循环)
- `DrawFrame` = 32 + 2*6 + 0~5 = **44~49**
- `final_frame` = **44~49**

## 测试验证

### 预期效果

1. ✅ 角色使用正确的 CArmours 装备纹理
2. ✅ 角色显示正确的性别和职业外观
3. ✅ 角色站立时有 4 帧动画循环
4. ✅ 动画每 500ms 切换一帧 (站立)
5. ✅ 不同方向显示不同角度的角色姿态

### 验证步骤

1. 启动客户端,登录并进入游戏
2. 观察角色是否显示装备纹理 (不再是角色选择界面的样子)
3. 观察角色是否有动画循环 (站立动作的 4 帧循环)
4. 按方向键移动,观察角色方向是否正确切换
5. 查看日志确认使用的是 CArmours 库而不是 ChrSel

### 调试信息

查看日志输出:
```
🎨 角色帧索引: 16 (动作:Standing, 方向:4, 性别:Male)
🎨 开始绘制 CArmours(0)[16] 纹理...
✅ CArmours(0)[16] 纹理绘制成功
```

## 后续计划

### 阶段 2: 行走动画 (短期)

- [ ] 监听移动输入
- [ ] 切换到 Walking 动作 (32-79 帧)
- [ ] 移动停止后切换回 Standing

### 阶段 3: 分层渲染 (中期)

- [ ] 添加 DrawHead (发型库 CHair)
- [ ] 添加 DrawWeapon (武器库 CWeapons)
- [ ] 添加 DrawWings (特效库 CHumEffect)
- [ ] 实现正确的绘制顺序 (方向相关)

### 阶段 4: 完整动作 (长期)

- [ ] Attack1/Attack2/Attack3 (攻击动作)
- [ ] Spell (施法动作)
- [ ] Struck (受击动作)
- [ ] Die/Dead (死亡动作)
- [ ] Harvest (挖矿/采集动作)

### 阶段 5: 高级特性

- [ ] 装备切换 (不同装备 ID)
- [ ] 发型切换
- [ ] 武器切换
- [ ] 翅膀特效
- [ ] 坐骑系统
- [ ] 变身系统

## 参考文档

- `ClientRust/角色绘制系统移植指南.md` - 完整的C#源码分析
- `Client/MirObjects/PlayerObject.cs` - C# 原版实现
  - Lines 450-800: SetLibraries() - 库选择逻辑
  - Lines 5001-5100: Draw() pipeline - 绘制流程
- `Client/MirGraphics/MLibrary.cs` - 图库绘制方法

## 技术细节

### 为什么使用 Clone?

`LibraryName` 包含 `String` (CArmours/AArmours 等),不能实现 `Copy` trait。所以在需要所有权的地方 (如 `libs.load(name)`) 使用 `.clone()` 创建副本。

### 为什么删除 Copy derive?

```rust
// ❌ 无法编译 (String 不是 Copy)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LibraryName {
    CArmours(usize), // 包含 String 在 default_path() 中
    // ...
}

// ✅ 正确
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LibraryName {
    CArmours(usize), // 可以 Clone
    // ...
}
```

### 动画流畅度

- 站立: 500ms/帧 = 2 FPS (慢速循环)
- 行走: 100ms/帧 = 10 FPS (快速)
- 跑步: 80ms/帧 = 12.5 FPS (更快)

游戏主循环 60 FPS,动画更新独立于主循环帧率。

## 已知问题

1. ⚠️ **硬编码帧数据**: 当前动作帧参数是硬编码的,应该从 Globals.DataReader 加载
2. ⚠️ **缺少发型/武器**: 只绘制了身体,没有头发和武器
3. ⚠️ **缺少装备加载**: 装备库 (CArmours[0-999]) 没有预加载
4. ⚠️ **缺少方向变化**: 手动输入方向键没有触发动画更新

## 提交信息

```
feat: 实现角色动画系统 (MVP)

- 修复角色纹理错误 (从 ChrSel 改为 CArmours)
- 添加动画帧计算 (DrawFrame + ArmourOffSet)
- 添加动作管理 (Standing/Walking/Running/Attack1)
- 添加动画更新循环 (update_animation)
- 添加装备库类型 (CArmours/AArmours/ARArmours)
- 角色现在显示正确的装备纹理并有站立动画

Issues: #角色纹理不对 #角色没有动画
```
