# 中文输入法最终解决方案

## 🎯 问题总结

经过测试确认:
- ✅ **英文输入正常**: `KeyInput.text = Some("h")`
- ❌ **中文输入失败**: `KeyInput.text = None`, `Logical Key = Named(Process)`

**根本原因**: winit 0.30 的 IME (输入法) 事件通过 `WindowEvent::Ime` 传递,而不是 `KeyInput.text`

## 🚧 ggez 0.10 的限制

ggez 的 `EventHandler` trait **不支持** IME 事件:

```rust
pub trait EventHandler {
    fn key_down_event(...);    // ✓ 有
    fn text_input_event(...);   // ✓ 有
    fn ime_event(...);          // ❌ 没有!
}
```

## ✅ 推荐解决方案

### 方案 1: 等待 ggez 更新 (长期)

等待 ggez 添加 IME 支持,或者自己 fork ggez 添加:

```rust
// 在 ggez/src/event.rs 添加
fn ime_event(&mut self, _ctx: &mut Context, _ime: Ime) -> GameResult {
    Ok(())
}
```

### 方案 2: 使用第三方 UI 库

#### 选项 A: egui (推荐但需要兼容性检查)
- 完美 IME 支持
- 但 `egui_ggez` 可能不存在或不兼容 ggez 0.10-rc0
- 需要查找或自己实现 egui + ggez 集成

#### 选项 B: imgui-rs
- 也支持 IME
- 有 imgui-ggez 库
- 但可能也有兼容性问题

### 方案 3: 自定义事件循环 (实用)

绕过 ggez 的事件循环,直接使用 winit:

```rust
use ggez::winit::event_loop::EventLoop;
use ggez::winit::event::{Event, WindowEvent, Ime};

fn main() -> GameResult {
    let event_loop = EventLoop::new()?;
    let (mut ctx, _) = ContextBuilder::new("game", "author").build()?;
    let mut state = GameState::new(&mut ctx)?;

    event_loop.run(move |event, elwt| {
        // 处理 IME 事件
        if let Event::WindowEvent { 
            event: WindowEvent::Ime(Ime::Commit(text)), .. 
        } = &event {
            // 收到中文字符!
            for ch in text.chars() {
                state.input_boxes[state.focused_box].text.push(ch);
            }
        }

        // 处理其他事件
        ctx.process_event(&event);
        
        // 更新和绘制
        match event {
            Event::AboutToWait => {
                state.update(&mut ctx).ok();
                state.draw(&mut ctx).ok();
            }
            Event::WindowEvent { 
                event: WindowEvent::CloseRequested, .. 
            } => elwt.exit(),
            _ => {}
        }
    })?;
    
    Ok(())
}
```

### 方案 4: 临时方案 - 支持粘贴 (最简单)

在完美的 IME 支持实现之前,支持 Ctrl+V 粘贴:

```rust
fn key_down_event(&mut self, _ctx: &mut Context, input: KeyInput, _repeated: bool) -> GameResult {
    // 检测 Ctrl+V
    if input.modifiers.contains(ModifiersState::CONTROL) {
        if input.event.logical_key == Key::Character("v") {
            // 从剪贴板读取
            if let Ok(text) = arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                self.input_boxes[self.focused_box].text.push_str(&text);
            }
        }
    }
    
    // 普通输入
    if let Some(text) = &input.event.text {
        if input.event.logical_key != Key::Named(NamedKey::Process) {
            for ch in text.chars() {
                if !ch.is_control() {
                    self.input_boxes[self.focused_box].text.push(ch);
                }
            }
        }
    }
    
    Ok(())
}
```

## 🎬 建议的行动计划

### 第1步: 实现粘贴功能 (本周)

```rust
// 在 LoginScene 的 key_down_event 中添加:
if input.modifiers.contains(ModifiersState::CONTROL) {
    match input.event.logical_key {
        Key::Character(ref ch) if ch == "v" || ch == "V" => {
            // 粘贴
            if let Ok(text) = arboard::Clipboard::new()
                .and_then(|mut cb| cb.get_text()) 
            {
                self.account_input_box.text.push_str(&text);
            }
        }
        Key::Character(ref ch) if ch == "c" || ch == "C" => {
            // 复制选中文本 (如果有选择功能)
            if let Some(selected) = &self.account_input_box.selected_text {
                arboard::Clipboard::new()
                    .and_then(|mut cb| cb.set_text(selected))
                    .ok();
            }
        }
        _ => {}
    }
}
```

**优点**:
- ✅ 立即可用
- ✅ 用户可以在其他地方输入中文,然后粘贴
- ✅ 不需要修改 ggez

**缺点**:
- ❌ 不是原生输入体验
- ❌ 需要额外步骤

### 第2步: 使用自定义事件循环 (下周)

参考方案3,修改 `src/main_ggez.rs` 的 `main()` 函数。

### 第3步: 长期考虑迁移到支持 IME 的框架

- bevy_egui (Bevy 游戏引擎 + egui)
- macroquad (更简单的游戏框架,有IME支持)
- 或者自己维护 ggez fork

## 📋 代码示例: 添加粘贴支持

### 1. 确保 arboard 依赖已添加

```toml
# Cargo.toml 已有
arboard = "3.6.1"
```

### 2. 修改 LoginScene

在 `src/scenes/login_scene.rs` (或类似文件) 中:

```rust
use arboard::Clipboard;
use ggez::input::keyboard::{Key, ModifiersState};

impl EventHandler for LoginScene {
    fn key_down_event(&mut self, ctx: &mut Context, input: KeyInput, repeated: bool) -> GameResult {
        // ... 现有代码 ...
        
        // 添加剪贴板支持
        if input.modifiers.contains(ModifiersState::CONTROL) {
            match &input.event.logical_key {
                Key::Character(ch) if ch.to_lowercase() == "v" => {
                    // Ctrl+V 粘贴
                    if let Ok(mut clipboard) = Clipboard::new() {
                        if let Ok(text) = clipboard.get_text() {
                            // 粘贴到当前聚焦的输入框
                            match self.focused_input {
                                InputField::Account => self.account_text.push_str(&text),
                                InputField::Password => self.password_text.push_str(&text),
                            }
                            tracing::info!("粘贴文本: {} 字符", text.len());
                        }
                    }
                    return Ok(());
                }
                Key::Character(ch) if ch.to_lowercase() == "c" => {
                    // Ctrl+C 复制 (如果有选择功能)
                    // TODO: 实现文本选择
                    return Ok(());
                }
                _ => {}
            }
        }
        
        // 普通字符输入
        if let Some(text) = &input.event.text {
            // 跳过 IME 处理中的按键
            if let Key::Named(named) = &input.event.logical_key {
                if named == &ggez::winit::keyboard::NamedKey::Process {
                    return Ok(());
                }
            }
            
            // 添加普通字符
            for ch in text.chars() {
                if !ch.is_control() {
                    match self.focused_input {
                        InputField::Account => self.account_text.push(ch),
                        InputField::Password => self.password_text.push(ch),
                    }
                }
            }
        }
        
        Ok(())
    }
}
```

## 📝 使用说明 (给用户)

**目前中文输入的方法**:

1. 在任意文本编辑器 (记事本、微信等) 中输入中文
2. 复制文本 (Ctrl+C)
3. 在游戏中点击输入框
4. 粘贴 (Ctrl+V)

**未来计划**: 实现完整的原生中文输入支持

## 🔗 相关链接

- [winit IME 文档](https://docs.rs/winit/0.30/winit/event/enum.Ime.html)
- [ggez Event Handler](https://docs.rs/ggez/0.10.0-rc0/ggez/event/trait.EventHandler.html)
- [arboard (剪贴板库)](https://docs.rs/arboard/)

---

**总结**: 
1. ✅ 短期: 实现粘贴功能 (2小时工作量)
2. ⏳ 中期: 自定义事件循环处理 IME (1天工作量)
3. 🔮 长期: 考虑框架迁移或维护 ggez fork
