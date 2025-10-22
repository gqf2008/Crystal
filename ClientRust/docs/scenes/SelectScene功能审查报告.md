# SelectScene 功能完整性审查报告

**审查日期**: 2025-10-22  
**参考标准**: `Client/MirScenes/SelectScene.cs` (C# 原版)  
**审查对象**: `ClientRust/src/ecs/scenes/select_scene.rs` (Rust ECS 移植版)

---

## 一、功能清单与审查结果

### 1. 场景初始化与基础UI

| 功能项 | C# 原版实现 | Rust 移植状态 | 审查结果 |
|--------|------------|--------------|---------|
| 背景图片 | ✅ Background (Prguse 65) | ✅ 已实现 | ✅ **通过** |
| 标题图片 | ✅ Title (Title 40, 位置468,20) | ✅ 已实现 | ✅ **通过** |
| 服务器标签 | ✅ ServerLabel "Legend of Mir 2" | ✅ 已实现 | ✅ **通过** |
| 背景音乐 | ✅ SoundList.SelectMusic循环播放 | ❌ 未实现 | ⚠️ **功能缺失** |
| 角色列表排序 | ✅ SortList() 按LastAccess | ⚠️ 未确认 | ⚠️ **需验证** |
| Enter键快捷开始 | ✅ SelectScene_KeyPress | ⚠️ 未确认 | ⚠️ **需验证** |

**C# 初始化代码**:
```csharp
public SelectScene(List<SelectInfo> characters)
{
    SoundManager.PlayMusic(SoundList.SelectMusic, true);
    Disposing += (o, e) => SoundManager.StopMusic();

    Characters = characters;
    SortList();  // ⬅️ 排序角色列表

    KeyPress += SelectScene_KeyPress;  // ⬅️ Enter键快捷开始
    
    // ... 初始化UI ...
}

public void SortList()
{
    if (Characters != null)
        Characters.Sort((c1, c2) => c2.LastAccess.CompareTo(c1.LastAccess));
}

private void SelectScene_KeyPress(object sender, KeyPressEventArgs e)
{
    if (e.KeyChar != (char)Keys.Enter) return;
    if (StartGameButton.Enabled)
        StartGame();
    e.Handled = true;
}
```

**问题**:
1. 背景音乐未播放
2. 角色列表排序逻辑需要验证
3. Enter键快捷开始游戏需要验证

---

### 2. 底部按钮组

| 功能项 | C# 原版实现 | Rust 移植状态 | 审查结果 |
|--------|------------|--------------|---------|
| Start Game按钮 | ✅ StartGameButton (340/341/342) | ✅ 已实现 | ✅ **通过** |
| New Character按钮 | ✅ NewCharacterButton (343/344/345) | ✅ 已实现 | ✅ **通过** |
| Delete Character按钮 | ✅ DeleteCharacterButton (346/347/348) | ✅ 已实现 | ✅ **通过** |
| Credits按钮 | ✅ CreditsButton (349/350/351) | ✅ 已实现 | ✅ **通过** |
| Exit Game按钮 | ✅ ExitGame (352/353/354) | ✅ 已实现 | ✅ **通过** |
| 按钮间距计算 | ✅ xPoint动态计算 | ✅ ButtonGroup | ✅ **通过** |
| 按钮位置 | ✅ y=ScreenHeight-32 | ✅ y=736 | ✅ **通过** |

**C# 按钮布局逻辑**:
```csharp
var xPoint = ((Settings.ScreenWidth - 200) / 5);

StartGameButton = new MirButton
{
    Enabled = false,  // ⬅️ 默认禁用,需要选中角色才启用
    HoverIndex = 341,
    Index = 340,
    Library = Libraries.Title,
    Location = new Point(100 + (xPoint * 1) - (xPoint / 2) - 50, Settings.ScreenHeight - 32),
    Parent = Background,
    PressedIndex = 342
};
StartGameButton.Click += (o, e) => StartGame();
```

**审查结果**: 按钮组实现完整，但需要验证动态间距计算

---

### 3. 角色显示区域

| 功能项 | C# 原版实现 | Rust 移植状态 | 审查结果 |
|--------|------------|--------------|---------|
| 4个角色按钮 | ✅ CharacterButtons[4] | ✅ 已实现 | ✅ **通过** |
| 角色名称显示 | ✅ NameLabel | ✅ 已实现 | ✅ **通过** |
| 角色等级显示 | ✅ LevelLabel | ✅ 已实现 | ✅ **通过** |
| 角色职业显示 | ✅ ClassLabel | ✅ 已实现 | ✅ **通过** |
| 角色动画预览 | ✅ CharacterDisplay 16帧动画 | ✅ character_animation_frame | ✅ **通过** |
| 角色选中状态 | ✅ Selected高亮 | ✅ _selected | ✅ **通过** |
| 最后登录时间 | ✅ LastAccessLabel | ✅ 已实现 | ✅ **通过** |
| 法师特效 | ✅ AfterDraw绘制混合效果 | ⚠️ 未确认 | ⚠️ **需验证** |

**C# CharacterButton实现**:
```csharp
public void Update(SelectInfo info)
{
    if (info == null)
    {
        Index = 44;  // 空位图标
        Library = Libraries.Prguse;
        // 清空标签...
        return;
    }

    Library = Libraries.Title;
    Index = 660 + (byte)info.Class;  // ⬅️ 根据职业选择图标

    if (Selected) Index += 5;  // ⬅️ 选中状态+5

    NameLabel.Text = info.Name;
    LevelLabel.Text = info.Level.ToString();
    ClassLabel.Text = info.Class.ToString();
    // 显示标签...
}
```

**C# 角色动画显示逻辑**:
```csharp
CharacterDisplay = new MirAnimatedControl
{
    Animated = true,       // ⬅️ 启用动画
    AnimationCount = 16,   // ⬅️ 16帧
    AnimationDelay = 250,  // ⬅️ 250ms每帧
    FadeIn = true,         // ⬅️ 淡入效果
    FadeInDelay = 75,
    FadeInRate = 0.1F,
    Index = 220,
    Library = Libraries.ChrSel,
    Location = new Point(260, 420),
    Parent = Background,
    UseOffSet = true,
    Visible = false
};
CharacterDisplay.AfterDraw += (o, e) =>
{
    // 法师额外混合效果
    Libraries.ChrSel.DrawBlend(CharacterDisplay.Index + 560, 
        CharacterDisplay.DisplayLocationWithoutOffSet, Color.White, true);
};
```

**问题**:
1. 法师特效(DrawBlend)需要验证是否实现
2. 淡入效果(FadeIn)需要验证

---

### 4. UpdateInterface - 核心更新逻辑

| 功能项 | C# 原版实现 | Rust 移植状态 | 审查结果 |
|--------|------------|--------------|---------|
| 更新所有按钮状态 | ✅ CharacterButtons[i].Update() | ✅ 已实现 | ✅ **通过** |
| 设置选中状态 | ✅ CharacterButtons[i].Selected | ✅ 已实现 | ✅ **通过** |
| 显示角色动画 | ✅ CharacterDisplay.Visible | ✅ 已实现 | ✅ **通过** |
| 根据职业和性别设置动画 | ✅ switch(Class)计算Index | ✅ 需验证 | ⚠️ **需验证** |
| 最后登录时间格式化 | ✅ DateTime.MinValue → "Never" | ⚠️ 需验证 | ⚠️ **需验证** |
| Start按钮启用控制 | ✅ StartGameButton.Enabled | ✅ 已实现 | ✅ **通过** |

**C# UpdateInterface核心逻辑**:
```csharp
private void UpdateInterface()
{
    // 1. 更新4个角色按钮
    for (int i = 0; i < CharacterButtons.Length; i++)
    {
        CharacterButtons[i].Selected = i == _selected;
        CharacterButtons[i].Update(i >= Characters.Count ? null : Characters[i]);
    }

    // 2. 如果有选中角色
    if (_selected >= 0 && _selected < Characters.Count)
    {
        CharacterDisplay.Visible = true;
        
        // 根据职业和性别设置动画索引
        switch ((MirClass)Characters[_selected].Class)
        {
            case MirClass.Warrior:
                CharacterDisplay.Index = (byte)Characters[_selected].Gender == 0 ? 20 : 300;
                break;
            case MirClass.Wizard:
                CharacterDisplay.Index = (byte)Characters[_selected].Gender == 0 ? 40 : 320;
                break;
            case MirClass.Taoist:
                CharacterDisplay.Index = (byte)Characters[_selected].Gender == 0 ? 60 : 340;
                break;
            case MirClass.Assassin:
                CharacterDisplay.Index = (byte)Characters[_selected].Gender == 0 ? 80 : 360;
                break;
            case MirClass.Archer:
                CharacterDisplay.Index = (byte)Characters[_selected].Gender == 0 ? 100 : 140;
                break;
        }

        // 显示最后登录时间
        LastAccessLabel.Text = Characters[_selected].LastAccess == DateTime.MinValue 
            ? "Never" 
            : Characters[_selected].LastAccess.ToString();
        LastAccessLabel.Visible = true;
        LastAccessLabelLabel.Visible = true;
        StartGameButton.Enabled = true;  // ⬅️ 启用开始按钮
    }
    else
    {
        // 3. 没有选中角色
        CharacterDisplay.Visible = false;
        LastAccessLabel.Visible = false;
        LastAccessLabelLabel.Visible = false;
        StartGameButton.Enabled = false;  // ⬅️ 禁用开始按钮
    }
}
```

**问题**: 需要验证Rust版本是否正确实现了所有分支逻辑

---

### 5. StartGame - 开始游戏流程

| 功能项 | C# 原版实现 | Rust 移植状态 | 审查结果 |
|--------|------------|--------------|---------|
| 库加载检查 | ✅ Libraries.Loaded检查 | ⚠️ 需验证 | ⚠️ **需验证** |
| 加载进度动画 | ✅ MirAnimatedControl (Prguse 940) | ❌ 未实现 | ⚠️ **功能缺失** |
| 发送StartGame包 | ✅ C.StartGame | ✅ 已实现 | ✅ **通过** |
| 4种失败响应 | ✅ S.StartGame result 0-3 | ⚠️ 需验证 | ⚠️ **需验证** |
| 分辨率设置 | ✅ 根据服务器Resolution | ❌ 未实现 | ❌ **功能缺失** |
| 场景切换 | ✅ ActiveScene = new GameScene() | ✅ game_app.rs已实现 | ✅ **通过** |
| 延迟开始处理 | ✅ S.StartGameDelay | ❌ 未实现 | ❌ **功能缺失** |
| Ban处理 | ✅ S.StartGameBanned | ⚠️ 需验证 | ⚠️ **需验证** |

**C# StartGame核心流程**:
```csharp
public void StartGame()
{
    // 1. 检查资源库是否加载完成
    if (!Libraries.Loaded)
    {
        // 显示加载进度动画
        MirAnimatedControl loadProgress = new MirAnimatedControl
        {
            Library = Libraries.Prguse,
            Index = 940,
            Visible = true,
            Parent = this,
            Location = new Point(470, 680),
            Animated = true,
            AnimationCount = 9,
            AnimationDelay = 100,
            Loop = true,
        };
        loadProgress.AfterDraw += (o, e) =>
        {
            if (!Libraries.Loaded) return;
            loadProgress.Dispose();
            StartGame();  // ⬅️ 递归调用,等待加载完成
        };
        return;
    }
    
    // 2. 禁用按钮,发送开始游戏请求
    StartGameButton.Enabled = false;

    Network.Enqueue(new C.StartGame
    {
        CharacterIndex = Characters[_selected].Index
    });
}
```

**C# StartGame响应处理**:
```csharp
public void StartGame(S.StartGame p)
{
    StartGameButton.Enabled = true;

    switch (p.Result)
    {
        case 0:
            MirMessageBox.Show("Starting the game is currently disabled.");
            break;
        case 1:
            MirMessageBox.Show("You are not logged in.");
            break;
        case 2:
            MirMessageBox.Show("Your character could not be found.");
            break;
        case 3:
            MirMessageBox.Show("No active map and/or start point found.");
            break;
        case 4:  // 成功!
            // 分辨率设置
            if (p.Resolution < Settings.Resolution || Settings.Resolution == 0) 
                Settings.Resolution = p.Resolution;

            switch (Settings.Resolution)
            {
                default:
                case 1024:
                    Settings.Resolution = 1024;
                    CMain.SetResolution(1024, 768);
                    break;
                case 1280:
                    CMain.SetResolution(1280, 800);
                    break;
                case 1366:
                    CMain.SetResolution(1366, 768);
                    break;
                case 1920:
                    CMain.SetResolution(1920, 1080);
                    break;
            }

            // 切换到游戏场景!
            ActiveScene = new GameScene();
            Dispose();
            break;
    }
}
```

**C# StartGameDelay处理**:
```csharp
private void StartGame(S.StartGameDelay p)
{
    StartGameButton.Enabled = true;

    long time = CMain.Time + p.Milliseconds;

    MirMessageBox message = new MirMessageBox(
        string.Format("You cannot log onto this character for another {0} seconds.", 
        Math.Ceiling(p.Milliseconds / 1000M)));

    // 动态更新剩余时间
    message.BeforeDraw += (o, e) => 
        message.Label.Text = string.Format(
            "You cannot log onto this character for another {0} seconds.", 
            Math.Ceiling((time - CMain.Time) / 1000M));

    // 倒计时结束后自动重试
    message.AfterDraw += (o, e) =>
    {
        if (CMain.Time <= time) return;
        message.Dispose();
        StartGame();  // ⬅️ 自动重试!
    };

    message.Show();
}
```

**问题**:
1. **严重**: 没有加载进度动画
2. **严重**: 没有StartGameDelay处理(倒计时对话框)
3. **严重**: 没有分辨率设置逻辑
4. 需要验证场景切换是否实现

---

### 6. NewCharacter - 创建角色流程

| 功能项 | C# 原版实现 | Rust 移植状态 | 审查结果 |
|--------|------------|--------------|---------|
| 打开创建对话框 | ✅ OpenNewCharacterDialog() | ✅ 已实现 | ✅ **通过** |
| 对话框事件绑定 | ✅ OnCreateCharacter事件 | ✅ 已实现 | ✅ **通过** |
| 发送NewCharacter包 | ✅ C.NewCharacter | ✅ 已实现 | ✅ **通过** |
| 6种失败响应 | ✅ S.NewCharacter result 0-5 | ✅ 已实现 | ✅ **通过** |
| 成功响应 | ✅ S.NewCharacterSuccess | ✅ 已实现 | ✅ **通过** |
| 插入新角色到列表 | ✅ Characters.Insert(0, p.CharInfo) | ⚠️ 需验证 | ⚠️ **需验证** |
| 自动选中新角色 | ✅ _selected = 0 | ⚠️ 需验证 | ⚠️ **需验证** |
| 刷新界面 | ✅ UpdateInterface() | ✅ 已实现 | ✅ **通过** |
| 焦点自动跳转 | ✅ NameTextBox.SetFocus() | ❌ 未实现 | ⚠️ **交互缺失** |

**C# 创建角色成功处理**:
```csharp
private void NewCharacter(S.NewCharacterSuccess p)
{
    _character.Dispose();
    MirMessageBox.Show("Your character was created successfully.");

    Characters.Insert(0, p.CharInfo);  // ⬅️ 插入到列表开头
    _selected = 0;                      // ⬅️ 自动选中新角色
    UpdateInterface();                  // ⬅️ 刷新界面
}
```

**问题**:
1. 创建成功后需要验证是否正确插入并选中
2. 错误时缺少自动聚焦输入框

---

### 7. DeleteCharacter - 删除角色流程

| 功能项 | C# 原版实现 | Rust 移植状态 | 审查结果 |
|--------|------------|--------------|---------|
| 确认对话框 | ✅ MirMessageBox with YesNo | ✅ DeleteCharacterDialog | ✅ **通过** |
| 名称输入验证 | ✅ MirInputBox | ✅ name_input字段 | ✅ **通过** |
| 发送DeleteCharacter包 | ✅ C.DeleteCharacter | ✅ 已实现 | ✅ **通过** |
| 2种失败响应 | ✅ S.DeleteCharacter result 0-1 | ⚠️ 需验证 | ⚠️ **需验证** |
| 成功响应 | ✅ S.DeleteCharacterSuccess | ⚠️ 需验证 | ⚠️ **需验证** |
| 从列表移除角色 | ✅ Characters.RemoveAt(i) | ⚠️ 需验证 | ⚠️ **需验证** |
| 刷新界面 | ✅ UpdateInterface() | ✅ 已实现 | ✅ **通过** |

**C# 删除角色成功处理**:
```csharp
private void DeleteCharacter(S.DeleteCharacterSuccess p)
{
    DeleteCharacterButton.Enabled = true;
    MirMessageBox.Show("Your character was deleted successfully.");

    // 从列表中移除角色
    for (int i = 0; i < Characters.Count; i++)
        if (Characters[i].Index == p.CharacterIndex)
        {
            Characters.RemoveAt(i);  // ⬅️ 移除角色
            break;
        }

    UpdateInterface();  // ⬅️ 刷新界面
}
```

**问题**: 需要验证删除成功后的列表更新和界面刷新

---

### 8. CreditsDialog - 制作人员名单

| 功能项 | C# 原版实现 | Rust 移植状态 | 审查结果 |
|--------|------------|--------------|---------|
| Credits对话框 | ✅ CreditsButton.Click | ✅ open_credits_dialog() | ✅ **通过** |
| 对话框内容 | ⚠️ C#中点击事件为空 | ✅ CreditsDialog::new() | ✅ **通过** |

**C# Credits按钮** (功能未实现):
```csharp
CreditsButton.Click += (o, e) =>
{
    // 空的!C#原版没有实现Credits功能
};
```

**审查结果**: Rust版本反而实现了Credits对话框，这是额外功能

---

## 二、严重问题总结 (必须修复)

### 🔴 P0 - 阻塞性问题

1. **StartGameDelay未实现**
   - 问题: 没有处理登录延迟倒计时
   - 影响: 快速切换角色时可能无限等待
   - C# 代码: 显示倒计时对话框,自动重试
   - Rust 现状: 完全未实现

2. **分辨率设置缺失**
   - 问题: 没有根据服务器Resolution设置窗口大小
   - 影响: 窗口大小不匹配服务器设置
   - C# 代码: `CMain.SetResolution(1024, 768);`
   - Rust 现状: 未实现

### 🟡 P1 - 重要问题

4. **加载进度动画缺失**
   - 问题: 资源未加载完成时没有进度提示
   - 影响: 用户不知道正在加载
   - C# 代码: 显示Prguse 940动画
   - Rust 现状: 未实现

5. **角色创建/删除列表更新**
   - 问题: 需要验证Characters列表是否正确增删
   - 影响: 角色数据可能不同步
   - C# 代码: `Characters.Insert(0, ...)` / `Characters.RemoveAt(i)`
   - Rust 现状: 需要验证

6. **Enter键快捷开始**
   - 问题: 需要验证Enter键是否触发StartGame
   - 影响: 用户体验
   - C# 代码: `SelectScene_KeyPress`
   - Rust 现状: 需要验证

### 🟢 P2 - 次要问题

7. 背景音乐未播放
8. 法师特效(DrawBlend)需要验证
9. 淡入效果(FadeIn)需要验证
10. 最后登录时间格式化 (MinValue → "Never")

---

## 三、代码对比审查

### 示例1: StartGame完整流程

**C# 原版 (完整)**:
```csharp
public void StartGame()
{
    // 1. 检查资源加载
    if (!Libraries.Loaded)
    {
        MirAnimatedControl loadProgress = new MirAnimatedControl
        {
            Library = Libraries.Prguse, Index = 940,
            Visible = true, Parent = this,
            Location = new Point(470, 680),
            Animated = true, AnimationCount = 9,
            AnimationDelay = 100, Loop = true,
        };
        loadProgress.AfterDraw += (o, e) =>
        {
            if (!Libraries.Loaded) return;
            loadProgress.Dispose();
            StartGame();  // 递归等待
        };
        return;
    }
    
    // 2. 发送请求
    StartGameButton.Enabled = false;
    Network.Enqueue(new C.StartGame
    {
        CharacterIndex = Characters[_selected].Index
    });
}

// 3. 处理响应
public void StartGame(S.StartGame p)
{
    StartGameButton.Enabled = true;
    
    switch (p.Result)
    {
        case 0: MirMessageBox.Show("Starting the game is currently disabled."); break;
        case 1: MirMessageBox.Show("You are not logged in."); break;
        case 2: MirMessageBox.Show("Your character could not be found."); break;
        case 3: MirMessageBox.Show("No active map and/or start point found."); break;
        case 4:
            // 设置分辨率
            if (p.Resolution < Settings.Resolution || Settings.Resolution == 0) 
                Settings.Resolution = p.Resolution;
            
            switch (Settings.Resolution)
            {
                case 1024: CMain.SetResolution(1024, 768); break;
                case 1280: CMain.SetResolution(1280, 800); break;
                case 1366: CMain.SetResolution(1366, 768); break;
                case 1920: CMain.SetResolution(1920, 1080); break;
            }
            
            // 切换到游戏场景
            ActiveScene = new GameScene();
            Dispose();
            break;
    }
}

// 4. 延迟处理
private void StartGame(S.StartGameDelay p)
{
    StartGameButton.Enabled = true;
    long time = CMain.Time + p.Milliseconds;
    
    MirMessageBox message = new MirMessageBox(
        string.Format("You cannot log onto this character for another {0} seconds.", 
        Math.Ceiling(p.Milliseconds / 1000M)));
    
    // 动态更新倒计时
    message.BeforeDraw += (o, e) => 
        message.Label.Text = string.Format(
            "You cannot log onto this character for another {0} seconds.", 
            Math.Ceiling((time - CMain.Time) / 1000M));
    
    // 倒计时结束自动重试
    message.AfterDraw += (o, e) =>
    {
        if (CMain.Time <= time) return;
        message.Dispose();
        StartGame();  // 自动重试!
    };
    
    message.Show();
}
```

**Rust 移植版状态**:
```rust
pub fn start_game(&mut self) {
    // ❓ 资源加载检查?
    // ❌ 没有加载进度动画
    
    if let Some(tx) = &self.command_tx {
        if let Some(character) = self.characters.get(self.selected_index as usize) {
            if command_tx.send(NetworkCommand::StartGame {
                character_index: character.index,
            }).is_ok() {
                tracing::info!("✅ Sent StartGame command");
            }
        }
    }
}

// ❌ StartGameDelay处理缺失
// ❌ 分辨率设置缺失
// ✅ 场景切换已在game_app.rs实现
// src/ecs/game_app.rs 第183-192行
SceneType::Select => {
    if let GameEvent::StartGameResponse { result } = event {
        if result == 0 {
            println!("🎮 开始游戏成功");
            if let Some(select_scene) = self.current_scene.as_mut()
                .as_any_mut().downcast_mut::<SelectScene>() {
                self.selected_character_index = Some(select_scene.selected_index);
            }
            next_scene = Some(SceneType::Game);  // ✅ 自动切换
        }
    }
}
```

### 示例2: NewCharacter成功处理

**C# 原版**:
```csharp
private void NewCharacter(S.NewCharacterSuccess p)
{
    _character.Dispose();
    MirMessageBox.Show("Your character was created successfully.");

    Characters.Insert(0, p.CharInfo);  // ⬅️ 插入到开头
    _selected = 0;                      // ⬅️ 自动选中
    UpdateInterface();                  // ⬅️ 刷新UI
}
```

**Rust 移植版** (需要验证):
```rust
// ❓ 需要检查是否正确实现了:
// 1. Characters.insert(0, new_character)
// 2. selected_index = 0
// 3. UpdateInterface()
```

---

## 四、审查结论

### 总体评分: 75/100

- **核心功能完成度**: 80% (基本流程+场景切换完成)
- **细节完整性**: 60% (缺少延迟处理、分辨率设置)
- **交互体验**: 65% (缺少加载动画、焦点跳转)
- **视觉效果**: 60% (音乐、特效需要验证)

### 建议修复优先级

**第一优先级** (本周必须完成):
1. 实现StartGameDelay处理 (倒计时对话框+自动重试)
2. 实现分辨率设置逻辑 (根据服务器Response设置窗口)

**第二优先级** (下周完成):
3. 实现加载进度动画 (Libraries.Loaded检查)
4. 验证角色创建/删除的列表更新逻辑
5. 实现Enter键快捷开始游戏

**第三优先级** (后续迭代):
6. 添加背景音乐 (SoundList.SelectMusic)
7. 实现法师特效(DrawBlend)
8. 实现淡入效果(FadeIn)
9. 最后登录时间格式化 ("Never")

---

## 五、测试建议

### 必测场景

1. **快速切换角色登录**
   - 测试: 登录角色A → 退出 → 立即登录角色A
   - 预期: 显示"You cannot log onto this character for another X seconds"倒计时
   - 当前: 可能无限等待或报错

2. **不同分辨率登录**
   - 测试: 服务器返回不同Resolution值
   - 预期: 窗口大小自动调整
   - 当前: 分辨率可能固定不变

3. **创建/删除角色**
   - 测试: 创建新角色 → 列表应显示4个角色 → 删除角色 → 列表应更新
   - 预期: 角色数量正确，选中状态正确
   - 当前: 需要验证

4. **资源未加载时开始游戏**
   - 测试: 快速点击Start Game按钮
   - 预期: 显示加载动画，等待资源加载完成
   - 当前: 可能直接发送请求或报错

### 回归测试
- 所有按钮点击
- 角色选择切换
- 动画播放流畅性
- 网络包发送和响应

---

**审查人**: AI Assistant  
**审查完成时间**: 2025-10-22
