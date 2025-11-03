# GameContext 事件便捷方法文档

本文档详细说明 GameContext 中新增的游戏事件访问便捷方法。

## 📋 目录

- [基础事件访问](#基础事件访问)
- [键盘输入查询](#键盘输入查询)
- [鼠标事件查询](#鼠标事件查询)
- [其他输入事件](#其他输入事件)
- [使用示例](#使用示例)
- [API 参考表](#api-参考表)

---

## 基础事件访问

### `global_events()` - 获取 GlobalEvents 引用

```rust
pub fn global_events(&self) -> Option<hecs::Ref<'_, GlobalEvents>>
```

**用途**: 获取全局事件组件的只读引用

**返回**: 
- `Some(Ref)` - 成功获取 GlobalEvents
- `None` - GlobalEvents 组件未找到（会输出警告日志）

**示例**:
```rust
fn run(&mut self, ctx: GameContext) {
    if let Some(events) = ctx.global_events() {
        println!("输入事件数量: {}", events.input_events.len());
        println!("网络事件数量: {}", events.net_events.total_count());
    }
}
```

---

### `input_events()` - 获取输入事件列表

```rust
pub fn input_events(&self) -> Vec<InputEvent>
```

**用途**: 获取当前帧的所有输入事件（克隆）

**返回**: 输入事件向量（如果 GlobalEvents 不存在则返回空向量）

**事件类型**:
- `KeyDown` - 键盘按下
- `KeyUp` - 键盘释放
- `MouseMove` - 鼠标移动
- `MouseDown` - 鼠标按钮按下
- `MouseUp` - 鼠标按钮释放
- `MouseWheel` - 鼠标滚轮
- `Ime` - IME 输入
- `Resize` - 窗口大小改变

**示例**:
```rust
fn run(&mut self, ctx: GameContext) {
    for event in ctx.input_events() {
        match event {
            InputEvent::KeyDown { keycode, .. } => {
                println!("按键按下: {:?}", keycode);
            }
            InputEvent::MouseDown { button, x, y } => {
                println!("鼠标点击: {:?} at ({}, {})", button, x, y);
            }
            _ => {}
        }
    }
}
```

---

### `net_events()` - 获取网络事件

```rust
pub fn net_events(&self) -> Option<CategorizedEvents>
```

**用途**: 获取分类的网络事件（服务器→客户端的消息）

**返回**:
- `Some(CategorizedEvents)` - 成功获取网络事件
- `None` - GlobalEvents 不存在

**事件分类** (CategorizedEvents 包含11个类别):
- `login` - 登录相关
- `character` - 角色相关
- `map` - 地图相关
- `object` - 对象相关
- `chat` - 聊天相关
- `item` - 物品相关
- `skill` - 技能相关
- `quest` - 任务相关
- `guild` - 公会相关
- `trade` - 交易相关
- `other` - 其他

**示例**:
```rust
fn run(&mut self, ctx: GameContext) {
    if let Some(net_events) = ctx.net_events() {
        // 处理登录事件
        for event in net_events.login {
            // 处理登录成功/失败
        }
        
        // 处理聊天消息
        for event in net_events.chat {
            // 显示聊天消息
        }
    }
}
```

---

## 键盘输入查询

### `is_key_just_pressed()` - 检查按键刚刚按下

```rust
pub fn is_key_just_pressed(&self, key: KeyCode) -> bool
```

**用途**: 检测单次按键（忽略长按重复）

**特点**:
- 只检测 `repeat=false` 的 KeyDown 事件
- 适合菜单选择、技能释放等场景
- 每次按键只触发一次

**示例**:
```rust
use ggez::input::keyboard::KeyCode;

fn run(&mut self, ctx: GameContext) {
    // 检测空格键单次按下
    if ctx.is_key_just_pressed(KeyCode::Space) {
        self.player_jump();
    }
    
    // 检测 ESC 键打开菜单
    if ctx.is_key_just_pressed(KeyCode::Escape) {
        self.toggle_menu();
    }
    
    // 检测 F1 键打开帮助
    if ctx.is_key_just_pressed(KeyCode::F1) {
        self.show_help();
    }
}
```

---

### `is_key_just_released()` - 检查按键刚刚释放

```rust
pub fn is_key_just_released(&self, key: KeyCode) -> bool
```

**用途**: 检测按键释放事件

**适用场景**:
- 技能蓄力释放
- 长按检测结束
- 拖拽操作结束

**示例**:
```rust
fn run(&mut self, ctx: GameContext) {
    // 蓄力技能：按住 Q 蓄力，释放时发射
    if ctx.is_key_just_released(KeyCode::Q) {
        self.release_charged_skill();
    }
}
```

---

### `pressed_keys_this_frame()` - 获取本帧按下的所有按键

```rust
pub fn pressed_keys_this_frame(&self) -> Vec<KeyCode>
```

**用途**: 获取本帧触发 KeyDown 事件的所有按键

**返回**: 按键代码向量

**示例**:
```rust
fn run(&mut self, ctx: GameContext) {
    let keys = ctx.pressed_keys_this_frame();
    if !keys.is_empty() {
        println!("本帧按下的按键: {:?}", keys);
    }
    
    // 检测组合键
    if keys.contains(&KeyCode::LControl) && keys.contains(&KeyCode::S) {
        self.save_game();
    }
}
```

---

### `released_keys_this_frame()` - 获取本帧释放的所有按键

```rust
pub fn released_keys_this_frame(&self) -> Vec<KeyCode>
```

**用途**: 获取本帧触发 KeyUp 事件的所有按键

**返回**: 按键代码向量

**示例**:
```rust
fn run(&mut self, ctx: GameContext) {
    let released = ctx.released_keys_this_frame();
    for key in released {
        self.on_key_released(key);
    }
}
```

---

## 鼠标事件查询

### `mouse_left_just_pressed()` - 检查鼠标左键刚刚按下

```rust
pub fn mouse_left_just_pressed(&self) -> bool
```

**用途**: 检测鼠标左键点击事件（瞬间）

**区别**:
- `mouse_left_pressed()` - 检测按住状态（持续）
- `mouse_left_just_pressed()` - 检测按下瞬间（一次）

**示例**:
```rust
fn run(&mut self, ctx: GameContext) {
    // 检测点击
    if ctx.mouse_left_just_pressed() {
        let (x, y) = ctx.mouse_position();
        self.on_click(x, y);
    }
}
```

---

### `mouse_right_just_pressed()` - 检查鼠标右键刚刚按下

```rust
pub fn mouse_right_just_pressed(&self) -> bool
```

**用途**: 检测鼠标右键点击事件

**示例**:
```rust
fn run(&mut self, ctx: GameContext) {
    // 右键打开上下文菜单
    if ctx.mouse_right_just_pressed() {
        let (x, y) = ctx.mouse_position();
        self.show_context_menu(x, y);
    }
}
```

---

### `mouse_left_just_released()` / `mouse_right_just_released()`

```rust
pub fn mouse_left_just_released(&self) -> bool
pub fn mouse_right_just_released(&self) -> bool
```

**用途**: 检测鼠标按钮释放事件

**示例**:
```rust
fn run(&mut self, ctx: GameContext) {
    // 拖拽操作：按下开始，释放结束
    if ctx.mouse_left_pressed() {
        self.dragging = true;
    }
    
    if ctx.mouse_left_just_released() && self.dragging {
        self.dragging = false;
        self.on_drop();
    }
}
```

---

### `mouse_wheel_delta()` - 获取鼠标滚轮增量

```rust
pub fn mouse_wheel_delta(&self) -> Option<(f32, f32)>
```

**用途**: 获取本帧的鼠标滚轮滚动量

**返回**:
- `Some((x, y))` - 滚轮增量（x: 横向, y: 纵向）
- `None` - 本帧没有滚轮事件

**注意**: ggez 不提供滚轮状态查询，只有事件

**示例**:
```rust
fn run(&mut self, ctx: GameContext) {
    // 缩放视图
    if let Some((_, dy)) = ctx.mouse_wheel_delta() {
        self.camera.zoom *= 1.0 + dy * 0.1;
        self.camera.zoom = self.camera.zoom.clamp(0.5, 3.0);
    }
}
```

---

### `mouse_move_delta()` - 获取鼠标移动增量

```rust
pub fn mouse_move_delta(&self) -> Option<(f32, f32)>
```

**用途**: 获取本帧的鼠标相对移动量

**返回**:
- `Some((dx, dy))` - 移动增量（相对上一帧）
- `None` - 本帧没有移动事件

**适用场景**:
- 相机拖拽
- 物体旋转
- 画笔绘制

**示例**:
```rust
fn run(&mut self, ctx: GameContext) {
    // 拖拽相机
    if ctx.mouse_middle_pressed() {
        if let Some((dx, dy)) = ctx.mouse_move_delta() {
            self.camera.x -= dx;
            self.camera.y -= dy;
        }
    }
}
```

---

## 其他输入事件

### `ime_characters()` - 获取 IME 字符输入

```rust
pub fn ime_characters(&self) -> Vec<char>
```

**用途**: 获取本帧通过 IME（输入法）输入的所有字符

**适用场景**:
- 聊天输入框
- 文本编辑器
- 名称输入框

**示例**:
```rust
fn run(&mut self, ctx: GameContext) {
    // 聊天输入
    for ch in ctx.ime_characters() {
        self.chat_input.push(ch);
    }
}
```

---

### `window_resized()` - 检查窗口大小改变

```rust
pub fn window_resized(&self) -> Option<(f32, f32)>
```

**用途**: 检测本帧窗口是否发生 resize

**返回**:
- `Some((width, height))` - 新的窗口尺寸
- `None` - 本帧没有 resize 事件

**示例**:
```rust
fn run(&mut self, ctx: GameContext) {
    // 响应窗口大小改变
    if let Some((w, h)) = ctx.window_resized() {
        self.ui.resize(w, h);
        self.camera.update_viewport(w, h);
    }
}
```

---

## 使用示例

### 示例 1: 玩家输入处理系统

```rust
use ggez::input::keyboard::KeyCode;

pub struct PlayerInputSystem;

impl SystemV2 for PlayerInputSystem {
    fn run(&mut self, ctx: GameContext) {
        // === 键盘输入 ===
        
        // 移动（持续按住）
        let mut dx = 0.0;
        let mut dy = 0.0;
        
        if ctx.is_key_pressed(KeyCode::W) { dy -= 1.0; }
        if ctx.is_key_pressed(KeyCode::S) { dy += 1.0; }
        if ctx.is_key_pressed(KeyCode::A) { dx -= 1.0; }
        if ctx.is_key_pressed(KeyCode::D) { dx += 1.0; }
        
        if dx != 0.0 || dy != 0.0 {
            // 发送移动命令
        }
        
        // 技能释放（单次按下）
        if ctx.is_key_just_pressed(KeyCode::Key1) {
            self.cast_skill(0);
        }
        if ctx.is_key_just_pressed(KeyCode::Key2) {
            self.cast_skill(1);
        }
        
        // === 鼠标输入 ===
        
        // 左键攻击
        if ctx.mouse_left_just_pressed() {
            let (x, y) = ctx.mouse_position();
            self.attack_at(x, y);
        }
        
        // 右键移动
        if ctx.mouse_right_just_pressed() {
            let (x, y) = ctx.mouse_position();
            self.move_to(x, y);
        }
    }
}
```

---

### 示例 2: UI 输入框系统

```rust
pub struct TextInputBoxSystem {
    text: String,
    active: bool,
}

impl SystemV2 for TextInputBoxSystem {
    fn run(&mut self, ctx: GameContext) {
        if !self.active {
            return;
        }
        
        // 处理 IME 输入
        for ch in ctx.ime_characters() {
            if ch.is_alphanumeric() || ch.is_whitespace() {
                self.text.push(ch);
            }
        }
        
        // 处理退格键
        if ctx.is_key_just_pressed(KeyCode::Back) && !self.text.is_empty() {
            self.text.pop();
        }
        
        // 处理回车键提交
        if ctx.is_key_just_pressed(KeyCode::Return) {
            self.submit();
            self.text.clear();
        }
        
        // ESC 取消输入
        if ctx.is_key_just_pressed(KeyCode::Escape) {
            self.active = false;
            self.text.clear();
        }
    }
    
    fn submit(&mut self) {
        println!("提交文本: {}", self.text);
    }
}
```

---

### 示例 3: 相机控制系统

```rust
pub struct CameraControlSystem {
    camera: Camera,
}

impl SystemV2 for CameraControlSystem {
    fn run(&mut self, ctx: GameContext) {
        // 鼠标中键拖拽
        if ctx.mouse_middle_pressed() {
            if let Some((dx, dy)) = ctx.mouse_move_delta() {
                self.camera.x -= dx / self.camera.zoom;
                self.camera.y -= dy / self.camera.zoom;
            }
        }
        
        // 滚轮缩放
        if let Some((_, dy)) = ctx.mouse_wheel_delta() {
            let zoom_factor = 1.0 + dy * 0.1;
            self.camera.zoom *= zoom_factor;
            self.camera.zoom = self.camera.zoom.clamp(0.1, 5.0);
        }
        
        // 键盘快捷键：Home 键重置视图
        if ctx.is_key_just_pressed(KeyCode::Home) {
            self.camera.reset();
        }
        
        // Ctrl+0 重置缩放
        let ctrl_pressed = ctx.is_key_pressed(KeyCode::LControl) || 
                          ctx.is_key_pressed(KeyCode::RControl);
        if ctrl_pressed && ctx.is_key_just_pressed(KeyCode::Key0) {
            self.camera.zoom = 1.0;
        }
    }
}
```

---

## API 参考表

### 基础事件访问

| 方法 | 返回类型 | 说明 |
|------|---------|------|
| `global_events()` | `Option<Ref<GlobalEvents>>` | 获取全局事件组件引用 |
| `input_events()` | `Vec<InputEvent>` | 获取输入事件列表（克隆） |
| `net_events()` | `Option<CategorizedEvents>` | 获取网络事件 |

### 键盘输入

| 方法 | 返回类型 | 说明 |
|------|---------|------|
| `is_key_just_pressed(key)` | `bool` | 检查按键刚刚按下（单次） |
| `is_key_just_released(key)` | `bool` | 检查按键刚刚释放 |
| `pressed_keys_this_frame()` | `Vec<KeyCode>` | 获取本帧按下的所有按键 |
| `released_keys_this_frame()` | `Vec<KeyCode>` | 获取本帧释放的所有按键 |

### 鼠标输入

| 方法 | 返回类型 | 说明 |
|------|---------|------|
| `mouse_left_just_pressed()` | `bool` | 检查鼠标左键刚刚按下 |
| `mouse_right_just_pressed()` | `bool` | 检查鼠标右键刚刚按下 |
| `mouse_left_just_released()` | `bool` | 检查鼠标左键刚刚释放 |
| `mouse_right_just_released()` | `bool` | 检查鼠标右键刚刚释放 |
| `mouse_wheel_delta()` | `Option<(f32, f32)>` | 获取滚轮增量 |
| `mouse_move_delta()` | `Option<(f32, f32)>` | 获取鼠标移动增量 |

### 其他输入

| 方法 | 返回类型 | 说明 |
|------|---------|------|
| `ime_characters()` | `Vec<char>` | 获取 IME 字符输入 |
| `window_resized()` | `Option<(f32, f32)>` | 检查窗口大小改变 |

---

## 注意事项

### 1. 键盘状态查询限制

⚠️ **重要**: ggez 0.10 的 `KeyboardContext` 不提供持续按键状态查询 API（如 `is_key_pressed`）

**解决方案**:
- 使用 `is_key_just_pressed()` 检测单次按键
- 如需持续按键检测，请在系统中自行维护状态：

```rust
pub struct KeyboardStateTracker {
    pressed_keys: HashSet<KeyCode>,
}

impl KeyboardStateTracker {
    fn update(&mut self, ctx: &GameContext) {
        // 更新按键状态
        for key in ctx.pressed_keys_this_frame() {
            self.pressed_keys.insert(key);
        }
        for key in ctx.released_keys_this_frame() {
            self.pressed_keys.remove(&key);
        }
    }
    
    fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }
}
```

### 2. 事件时机

- **"just_pressed/released"** 方法检测的是**本帧的事件**
- 如果错过本帧，事件不会保留到下一帧
- 对于持续输入（如移动），使用状态跟踪而非事件

### 3. 性能考虑

- `input_events()` 会克隆事件向量，有一定开销
- 如果只需检测特定事件，使用 `is_key_just_pressed()` 等便捷方法更高效
- `global_events()` 返回引用，零拷贝访问

### 4. 事件顺序

事件在 `input_events()` 中按时间顺序排列，可以用于：
- 检测双击
- 记录输入序列
- 实现组合键

---

## 与旧 API 的对比

### 旧方式（GlobalEvents 直接访问）

```rust
// 繁琐且容易出错
if let Some((_, events)) = ctx.world.query::<&GlobalEvents>().iter().next() {
    for event in &events.input_events {
        if let InputEvent::KeyDown { keycode: KeyCode::Space, repeat: false, .. } = event {
            self.jump();
        }
    }
}
```

### 新方式（便捷方法）

```rust
// 简洁明了
if ctx.is_key_just_pressed(KeyCode::Space) {
    self.jump();
}
```

**改进**:
- 代码量减少 70%
- 可读性提升
- 类型安全
- 易于维护

---

## 总结

GameContext 的事件便捷方法提供了：

✅ **简洁的 API** - 一行代码实现常见输入查询  
✅ **类型安全** - 编译时检查，减少运行时错误  
✅ **性能优化** - 直接访问事件，避免不必要的克隆  
✅ **易于维护** - 统一的访问接口，便于重构  
✅ **完整覆盖** - 支持键盘、鼠标、IME、窗口事件

使用这些方法可以大幅简化系统代码，提高开发效率！
