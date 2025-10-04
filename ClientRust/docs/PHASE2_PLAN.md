# 📋 Phase 2 执行计划 - 完整功能实现

**阶段**: Phase 2 - 完整功能实现  
**预计时间**: 2-3 周  
**目标**: 让游戏真正"动起来" - 实现渲染、战斗、移动、特效

---

## 🎯 Phase 2 总体目标

在 Phase 1 建立的基础架构上，实现完整的游戏功能：

1. **渲染系统** - 真正绘制角色到屏幕
2. **动作系统** - 移动、攻击、技能动作
3. **战斗系统** - 伤害计算、生命值管理、死亡
4. **特效系统** - Buff、毒、元素、技能特效
5. **完整外观** - Transform 39 types, Archer, Assassin

---

## 📅 Week 1: 渲染系统集成（Day 1-7）

### Day 1-3: 完整 Draw() 实现 ⏳ IN PROGRESS

**目标**: 实现完整的 `draw()` 方法，真正调用图形库绘制

**参考 C# 代码**:
```csharp
// Client/MirObjects/PlayerObject.cs Line 4877
public override void Draw()
{
    DrawBehindEffects(Settings.Effect);

    float oldOpacity = DXManager.Opacity;
    if (Hidden && !DXManager.Blending) DXManager.SetOpacity(0.5F);

    DrawMount();

    if (!RidingMount)
    {
        if (Direction == Left || Direction == Up || Direction == UpLeft || Direction == DownLeft)
            DrawWeapon();
        else
            DrawWeapon2();
    }

    DrawBody();

    if (Direction == Up || Direction == UpLeft || Direction == UpRight || Direction == Right || Direction == Left)
    {
        DrawHead();
        if (this != User)
            DrawWings();
    }
    else
    {
        if (this != User)
            DrawWings();
        DrawHead();
    }
    
    if (!RidingMount)
    {
        if (Direction == UpRight || Direction == Right || Direction == DownRight || Direction == Down)
            DrawWeapon();
        else
            DrawWeapon2();

        if (Class == Archer && HasClassWeapon)
            DrawWeapon2();
    }

    DXManager.SetOpacity(oldOpacity);
}
```

**实现任务**:
- [ ] 定义 `LibraryManager` trait - 管理纹理库
- [ ] 定义 `Renderer` trait - 绘制接口
- [ ] 实现完整的 `draw()` 方法（调用实际渲染）
- [ ] 实现 `draw_body()` 实际绘制
- [ ] 实现 `draw_head()` 实际绘制
- [ ] 实现 `draw_weapon()` / `draw_weapon2()` 实际绘制
- [ ] 实现 `draw_wings()` 实际绘制
- [ ] 实现 `draw_mount()` 实际绘制
- [ ] 实现 Layer 排序逻辑
- [ ] 处理透明度（Hidden 状态）
- [ ] 单元测试

**预计代码**: ~300 lines

**C# 对应文件**:
- `Client/MirObjects/PlayerObject.cs` (lines 4877-5110)
- `Client/MirGraphics/MLibrary.cs` (纹理库)
- `Client/DXManager.cs` (DirectX 管理器)

---

### Day 4-5: 特效绘制

**目标**: 实现特效绘制系统

**参考 C# 代码**:
```csharp
// Client/MirObjects/PlayerObject.cs Line 4928
public override void DrawBehindEffects(bool effectsEnabled)
{
    for (int i = 0; i < Effects.Count; i++)
    {
        if (!effectsEnabled) continue;
        Effects[i].Draw();
    }
}

public override void DrawEffects(bool effectsEnabled)
{
    for (int i = 0; i < Effects.Count; i++)
    {
        if (!effectsEnabled) continue;
        if (IsVitalEffect(Effects[i])) continue;
        Effects[i].Draw();
    }
}

public override void DrawBlend()
{
    if (WingEffect > 0 && WingEffect < 100 && WingLibrary != null)
    {
        if (Frame != null)
            WingLibrary.DrawBlend(DrawWingFrame + WingOffset, DrawLocation, DrawColour, true);
    }
}
```

**实现任务**:
- [ ] 实现 `draw_behind_effects()` 方法
- [ ] 实现 `draw_effects()` 方法
- [ ] 实现 `draw_blend()` 方法（混合模式绘制）
- [ ] Buff 特效绘制
- [ ] 毒/元素特效绘制
- [ ] 盾/护甲特效绘制
- [ ] 单元测试

**预计代码**: ~200 lines

---

### Day 6-7: 名字和状态栏绘制

**目标**: 实现名字、血条、Buff 图标绘制

**参考 C# 代码**:
```csharp
// Client/MirObjects/PlayerObject.cs Line 5255
public override void DrawName()
{
    if (!Name.Visible) return;
    
    string name = Name.Text;
    // Draw name
    // Draw guild
    // Draw level
    // Draw buffs
    // Draw health bar
}
```

**实现任务**:
- [ ] 实现 `draw_name()` 方法
- [ ] 绘制角色名字（带颜色）
- [ ] 绘制公会名
- [ ] 绘制等级
- [ ] 绘制血条
- [ ] 绘制 Buff 图标
- [ ] 单元测试

**预计代码**: ~150 lines

---

## 📅 Week 2: 动作和战斗系统（Day 8-14）

### Day 8-10: 动作系统

**目标**: 实现角色移动和动作

**参考 C# 代码**:
```csharp
// Client/MirObjects/MapObject.cs
public void Walk(MirDirection direction)
{
    // Movement logic
}

public void Run(MirDirection direction)
{
    // Running logic
}

public void Jump(Point location, MirDirection direction)
{
    // Jumping logic
}
```

**实现任务**:
- [ ] 实现 `walk()` 方法
- [ ] 实现 `run()` 方法
- [ ] 实现 `jump()` 方法
- [ ] 实现 `push_back()` 方法（击退）
- [ ] 实现路径寻找
- [ ] 实现碰撞检测
- [ ] 动作动画切换
- [ ] 单元测试

**预计代码**: ~400 lines

---

### Day 11-12: 战斗系统

**目标**: 实现攻击和伤害系统

**参考 C# 代码**:
```csharp
// Client/MirObjects/PlayerObject.cs
public void Attack(MirDirection direction, Spell spell)
{
    // Attack logic
}

public void Struck(int damage)
{
    // Damage handling
}

public void Die()
{
    // Death logic
}
```

**实现任务**:
- [ ] 实现 `attack()` 方法
- [ ] 实现 `struck()` 方法（受击）
- [ ] 实现 `die()` 方法（死亡）
- [ ] 实现 `revive()` 方法（复活）
- [ ] 伤害数字显示
- [ ] 死亡动画
- [ ] HP/MP 同步
- [ ] 单元测试

**预计代码**: ~300 lines

---

### Day 13-14: Buff/Debuff 系统

**目标**: 实现完整的 Buff 系统

**参考 C# 代码**:
```csharp
// Client/MirObjects/MapObject.cs
public void AddBuff(BuffType type, ...)
{
    // Add buff
}

public void RemoveBuff(BuffType type)
{
    // Remove buff
}

public void UpdateBuffs(long time)
{
    // Update buffs
}
```

**实现任务**:
- [ ] 实现 `add_buff()` 方法
- [ ] 实现 `remove_buff()` 方法
- [ ] 实现 `update_buffs()` 方法
- [ ] Buff 持续时间管理
- [ ] Buff 层数管理
- [ ] Buff 图标显示
- [ ] Buff 特效显示
- [ ] 单元测试

**预计代码**: ~250 lines

---

## 📅 Week 3: 完整外观和特殊系统（Day 15-21）

### Day 15-17: Transform 系统

**目标**: 实现 39 种变身类型

**参考 C# 代码**:
```csharp
// Client/MirObjects/PlayerObject.cs
public void SetLibraries()
{
    // Handle transforms
    switch (TransformType)
    {
        case 0: // Normal
        case 1: // Pig
        case 2: // SpiderBat
        // ... 39 types total
    }
}
```

**实现任务**:
- [ ] 实现 Transform 枚举（39 types）
- [ ] 实现变身纹理加载
- [ ] 实现变身动画
- [ ] 实现变身状态管理
- [ ] 各变身类型测试
- [ ] 单元测试

**预计代码**: ~400 lines

---

### Day 18-19: Archer 和 Assassin 特殊处理

**目标**: 实现 Archer altAnim 和 Assassin 双武器

**参考 C# 代码**:
```csharp
// Archer AltAnim
if (Class == MirClass.Archer)
{
    if (altAnim)
        BodyLibrary = Libraries.CHumEffect[8];
    else
        BodyLibrary = Libraries.ChrSel;
}

// Assassin dual weapons
if (Class == MirClass.Assassin && HasClassWeapon)
    DrawWeapon2();
```

**实现任务**:
- [ ] 实现 Archer `alt_anim` 切换
- [ ] 实现 Assassin 双武器绘制
- [ ] 特殊职业动画处理
- [ ] 单元测试

**预计代码**: ~200 lines

---

### Day 20-21: 特殊装备和特效

**目标**: 实现钓鱼竿、翅膀特效 100+、坐骑等

**参考 C# 代码**:
```csharp
// Fishing
public bool HasFishingRod()
{
    return Weapon == 8240 || Weapon == 8241;
}

// Wing effects
public void DrawWings()
{
    if (WingEffect <= 0 || WingEffect >= 100) return;
    WingLibrary.DrawBlend(...);
}
```

**实现任务**:
- [ ] 钓鱼竿特殊处理
- [ ] 翅膀特效系统（100+ 种）
- [ ] 坐骑系统完善
- [ ] 时装系统
- [ ] 单元测试

**预计代码**: ~300 lines

---

## 📊 Phase 2 预期成果

### 代码量预估

| 模块 | 预计代码 | 说明 |
|------|---------|------|
| **渲染系统** | ~650 lines | draw() 完整实现 + 特效 + 名字 |
| **动作系统** | ~400 lines | walk/run/jump + 路径寻找 |
| **战斗系统** | ~300 lines | attack/struck/die + HP 管理 |
| **Buff 系统** | ~250 lines | add/remove/update buffs |
| **Transform** | ~400 lines | 39 种变身类型 |
| **特殊职业** | ~200 lines | Archer altAnim + Assassin 双武器 |
| **特殊装备** | ~300 lines | 钓鱼竿 + 翅膀 100+ + 坐骑 |
| **测试代码** | ~500 lines | 单元测试 + 集成测试 |
| **总计** | **~3000 lines** | Phase 2 新增代码 |

### 功能覆盖

| 功能模块 | C# 方法数 | Phase 1 | Phase 2 目标 | 覆盖率目标 |
|----------|-----------|---------|-------------|-----------|
| **外观系统** | 8 | 7 (87.5%) | 8 | 100% ✅ |
| **动画系统** | 5 | 5 (100%) | 5 | 100% ✅ |
| **绘制系统** | 12 | 8 (67%) | 12 | 100% ✅ |
| **动作系统** | 8 | 0 | 8 | 100% ✅ |
| **战斗系统** | 10 | 0 | 10 | 100% ✅ |
| **技能系统** | 15 | 6 (40%) | 15 | 100% ✅ |
| **特效系统** | 15 | 0 | 15 | 100% ✅ |
| **总计** | **73** | **26 (36%)** | **73** | **100%** ✅ |

---

## 🎯 Phase 2 完成标准

### 功能完成度

- ✅ 角色能正确绘制到屏幕
- ✅ 角色能移动和转向
- ✅ 角色能攻击和受击
- ✅ 角色能施放技能
- ✅ 特效能正确显示
- ✅ Buff/Debuff 能正确显示和生效
- ✅ 所有 39 种变身正常工作
- ✅ Archer 和 Assassin 特殊功能正常

### 代码质量

- ✅ 编译无错误
- ✅ 测试覆盖率 ≥ 80%
- ✅ 所有单元测试通过
- ✅ 代码符合 Rust 最佳实践
- ✅ 性能满足需求（60 FPS）

### 文档完成度

- ✅ 每周创建总结文档
- ✅ 关键决策有记录
- ✅ 与 C# 代码对应关系清晰
- ✅ Phase 2 完成报告

---

## 🚀 立即开始

**当前状态**: Phase 1 完成 100%  
**下一步**: Phase 2 Day 1-3 - 完整 Draw() 实现  
**参考文件**: `Client/MirObjects/PlayerObject.cs` (lines 4877-5110)

**准备工作**:
1. 分析 C# 绘制流程
2. 设计 Rust 渲染接口
3. 实现 LibraryManager trait
4. 实现 Renderer trait
5. 实现完整 draw() 方法

**开始时间**: 2025-01-XX  
**预计完成**: 2025-01-XX + 2-3 weeks

---

让我们开始 Phase 2 的征程！🎉
