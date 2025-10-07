# 集成完整 IME 支持到主程序

## 🎯 实现方案

基于 `test_ime_custom_loop.rs` 的成功经验,将完整的 IME 支持集成到主程序。

### 核心改动

#### 1. 修改 main() 函数

**现有代码**:
```rust
fn main() -> Result<()> {
    // ...初始化...
    let (mut ctx, event_loop) = cb.build()?;
    let game = CrystalGame::new(&mut ctx, settings)?;
    
    // 使用 ggez 的标准事件循环 (不支持 IME)
    event::run(ctx, event_loop, game)
        .map_err(|e| anyhow::anyhow!("游戏循环错误: {}", e))
}
```

**新代码**:
```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ...初始化...
    let (mut ctx, event_loop) = cb.build()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    let mut game = CrystalGame::new(&mut ctx, settings)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    
    // 使用自定义事件循环 (支持 IME)
    let mut last_update = std::time::Instant::now();
    
    #[allow(deprecated)]
    event_loop.run(move |event, elwt| {
        use ggez::winit::event_loop::ControlFlow;
        use ggez::winit::event::{Event, WindowEvent, Ime};
        
        elwt.set_control_flow(ControlFlow::Poll);
        
        match event {
            // ===== IME 事件处理 =====
            Event::WindowEvent { 
                event: WindowEvent::Ime(ime_event), 
                .. 
            } => {
                game.handle_ime_event(ime_event);
            }
            
            // ===== 其他事件 =====
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { event, .. },
                ..
            } => {
                let key_input = KeyInput {
                    event,
                    mods: ggez::winit::keyboard::ModifiersState::empty(),
                };
                let _ = game.key_down_event(&mut ctx, key_input, false);
            }
            
            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                if state == ggez::winit::event::ElementState::Pressed {
                    let pos = ctx.mouse.position();
                    let _ = game.mouse_button_down_event(
                        &mut ctx,
                        convert_mouse_button(button),
                        pos.x,
                        pos.y,
                    );
                }
            }
            
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                elwt.exit();
            }
            
            Event::AboutToWait => {
                let now = std::time::Instant::now();
                if now.duration_since(last_update) >= std::time::Duration::from_millis(16) {
                    last_update = now;
                    
                    if let Err(e) = game.update(&mut ctx) {
                        eprintln!("Update error: {}", e);
                        elwt.exit();
                    }
                    
                    if let Err(e) = game.draw(&mut ctx) {
                        eprintln!("Draw error: {}", e);
                        elwt.exit();
                    }
                }
            }
            
            _ => {}
        }
    }).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}
```

#### 2. 在 CrystalGame 中添加 handle_ime_event 方法

```rust
impl CrystalGame {
    /// 处理 IME 事件 (中文输入法)
    pub fn handle_ime_event(&mut self, ime: ggez::winit::event::Ime) {
        use ggez::winit::event::Ime;
        
        match ime {
            Ime::Enabled => {
                tracing::debug!("IME 启用");
            }
            Ime::Disabled => {
                tracing::debug!("IME 禁用");
            }
            Ime::Preedit(text, _cursor) => {
                // 正在输入拼音 - 传递给场景管理器
                let mut scene_mgr = self.scene_manager.write();
                scene_mgr.handle_ime_preedit(text);
            }
            Ime::Commit(text) => {
                // 确认输入 - 中文字符在这里!
                tracing::debug!("IME 输入: {}", text);
                let mut scene_mgr = self.scene_manager.write();
                scene_mgr.handle_ime_commit(text);
            }
        }
    }
}
```

#### 3. 在 SceneManager 中添加 IME 处理方法

```rust
// src/scenes/mod.rs

impl SceneManager {
    /// 处理 IME 正在编辑的文本 (拼音)
    pub fn handle_ime_preedit(&mut self, text: String) {
        match self.current_scene {
            Some(SceneType::Login) => {
                if let Some(login_scene) = &mut self.login_scene {
                    login_scene.set_ime_preedit(text);
                }
            }
            _ => {}
        }
    }
    
    /// 处理 IME 确认的文本 (中文字符)
    pub fn handle_ime_commit(&mut self, text: String) {
        match self.current_scene {
            Some(SceneType::Login) => {
                if let Some(login_scene) = &mut self.login_scene {
                    login_scene.handle_ime_commit(text);
                }
            }
            _ => {}
        }
    }
}
```

#### 4. 在 LoginScene 中实现 IME 支持

```rust
// src/scenes/login_scene.rs

pub struct LoginScene {
    // ... 现有字段 ...
    ime_preedit: String,  // 正在输入的拼音
}

impl LoginScene {
    pub fn set_ime_preedit(&mut self, text: String) {
        self.ime_preedit = text;
        // 可以在输入框中显示灰色的拼音
    }
    
    pub fn handle_ime_commit(&mut self, text: String) {
        // 将确认的中文字符添加到当前聚焦的输入框
        // 根据 self.focused_input 决定添加到哪个输入框
        
        for ch in text.chars() {
            if !ch.is_control() {
                match self.focused_input {
                    FocusedInput::AccountInput => {
                        self.account_text.push(ch);
                    }
                    FocusedInput::PasswordInput => {
                        self.password_text.push(ch);
                    }
                    _ => {}
                }
            }
        }
        
        // 清空拼音显示
        self.ime_preedit.clear();
        
        tracing::info!("中文输入: {}", text);
    }
}
```

### 实现步骤

1. **第一步**: 修改 `src/main_ggez.rs` 的 `main()` 函数
2. **第二步**: 在 `CrystalGame` 添加 `handle_ime_event()` 方法
3. **第三步**: 在 `SceneManager` 添加 IME 转发方法
4. **第四步**: 在 `LoginScene` 实现具体的 IME 处理
5. **第五步**: 测试中文输入功能

### 优势

✅ **完整的原生中文输入支持**
✅ **显示输入法候选窗口**
✅ **可以看到正在输入的拼音**
✅ **所有输入框都支持中文**
✅ **不影响英文输入**

### 注意事项

- 自定义事件循环需要手动处理所有事件
- 需要正确转换 winit 事件到 ggez 事件
- 帧率控制需要手动实现
- 需要测试所有输入场景 (登录、注册、聊天等)

---

**准备好开始实施吗?**
