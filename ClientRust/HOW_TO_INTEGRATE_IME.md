# 如何将完整 IME 支持集成到主程序

## ✅ 测试结果确认

`test_ime_custom_loop` 测试程序已验证:
1. ✅ 拼音显示正常
2. ✅ 中文输入正常
3. ✅ 中文显示正常

## 🎯 集成方案

由于主程序结构较复杂,建议采用**渐进式集成**方案:

### 方案 A: 保守方案 - 先用粘贴,后续升级

**优点**:
- 风险小,不影响现有功能
- 立即可用
- 集成工作量小 (1小时)

**实施步骤**:
1. 在 `LoginScene::key_down_event` 中添加 Ctrl+V 支持
2. 用户可以先用粘贴方式输入中文
3. 后续有时间再升级到完整 IME

**代码示例** (在 `src/scenes/login_scene.rs` 中):
```rust
use arboard::Clipboard;

fn key_down_event(&mut self, ctx: &mut Context, input: KeyInput, repeated: bool) -> GameResult {
    use ggez::winit::keyboard::{Key, ModifiersState};
    
    // Ctrl+V 粘贴
    if input.mods.contains(ModifiersState::CONTROL) {
        if let Key::Character(ch) = &input.event.logical_key {
            if ch.to_lowercase() == "v" {
                if let Ok(text) = Clipboard::new().and_then(|mut cb| cb.get_text()) {
                    // 粘贴到当前聚焦的输入框
                    match self.focused_input {
                        Some(InputField::Account) => self.account_text.push_str(&text),
                        Some(InputField::Password) => self.password_text.push_str(&text),
                        _ => {}
                    }
                }
                return Ok(());
            }
        }
    }
    
    // ... 其他按键处理 ...
}
```

### 方案 B: 完整方案 - 立即集成完整 IME

**优点**:
- 用户体验最好
- 一步到位

**缺点**:
- 需要修改主事件循环
- 测试工作量大
- 风险较高

**实施文件** (共需修改 4 个文件):

#### 1. src/main_ggez.rs (主文件 - 最复杂)

需要将标准的 `event::run()` 替换为自定义事件循环。

关键改动点:
- 修改 `main()` 函数签名: `fn main() -> Result<(), Box<dyn std::error::Error>>`
- 替换 `event::run(ctx, event_loop, game)` 为自定义循环
- 添加 IME 事件处理
- 手动调用 update/draw
- 实现帧率控制

**注意**: 这个改动影响整个游戏循环,需要**仔细测试**。

#### 2. src/main_ggez.rs - CrystalGame impl

添加新方法:
```rust
impl CrystalGame {
    /// 处理 IME 事件
    pub fn handle_ime_event(&mut self, ime: ggez::winit::event::Ime) {
        use ggez::winit::event::Ime;
        
        match ime {
            Ime::Preedit(text, _) => {
                let mut scene_mgr = self.scene_manager.write();
                scene_mgr.handle_ime_preedit(text);
            }
            Ime::Commit(text) => {
                let mut scene_mgr = self.scene_manager.write();
                scene_mgr.handle_ime_commit(text);
            }
            _ => {}
        }
    }
}
```

#### 3. src/scenes/mod.rs - SceneManager

添加新方法:
```rust
impl SceneManager {
    pub fn handle_ime_preedit(&mut self, text: String) {
        match self.current_scene {
            Some(SceneType::Login) => {
                if let Some(scene) = &mut self.login_scene {
                    scene.set_ime_preedit(text);
                }
            }
            _ => {}
        }
    }
    
    pub fn handle_ime_commit(&mut self, text: String) {
        match self.current_scene {
            Some(SceneType::Login) => {
                if let Some(scene) = &mut self.login_scene {
                    scene.handle_ime_commit(text);
                }
            }
            _ => {}
        }
    }
}
```

#### 4. src/scenes/login_scene.rs - LoginScene

添加字段和方法:
```rust
pub struct LoginScene {
    // ... 现有字段 ...
    ime_preedit: String,
}

impl LoginScene {
    pub fn set_ime_preedit(&mut self, text: String) {
        self.ime_preedit = text;
    }
    
    pub fn handle_ime_commit(&mut self, text: String) {
        for ch in text.chars() {
            if !ch.is_control() {
                match self.focused_input {
                    Some(InputField::Account) => self.account_text.push(ch),
                    Some(InputField::Password) => self.password_text.push(ch),
                    _ => {}
                }
            }
        }
        self.ime_preedit.clear();
    }
}
```

## 📝 推荐实施顺序

### 立即执行 (今天):
1. ✅ **方案 A - 添加粘贴支持** (1小时)
   - 只需修改 `login_scene.rs`
   - 立即可用
   - 风险极低

### 后续执行 (下周/有时间时):
2. ⏳ **方案 B - 完整 IME** (1天)
   - 修改 4 个文件
   - 需要全面测试
   - 用户体验最佳

## 🔧 具体实施代码

### 方案 A 完整代码 (立即可用)

在 `src/scenes/login_scene.rs` 中找到 `key_down_event` 方法,在开头添加:

```rust
fn key_down_event(&mut self, ctx: &mut Context, input: KeyInput, repeated: bool) -> GameResult {
    use ggez::winit::keyboard::{Key, ModifiersState, NamedKey};
    
    // ===== 新增: Ctrl+V 粘贴支持 =====
    if input.mods.contains(ModifiersState::CONTROL) {
        if let Key::Character(ch) = &input.event.logical_key {
            if ch.to_lowercase() == "v" {
                match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                    Ok(text) => {
                        // 根据当前聚焦的输入框粘贴
                        // 注意: 需要根据实际的字段名调整
                        if self.account_input_focused {
                            self.account_text.push_str(&text);
                        } else if self.password_input_focused {
                            self.password_text.push_str(&text);
                        }
                        tracing::info!("粘贴了 {} 个字符", text.chars().count());
                    }
                    Err(e) => {
                        tracing::warn!("粘贴失败: {}", e);
                    }
                }
                return Ok(());
            }
        }
    }
    // ===== 粘贴支持结束 =====
    
    // ... 原有的按键处理代码 ...
}
```

**注意**: 需要确保 `Cargo.toml` 中已有 `arboard = "3.6.1"` 依赖 (已有✅)

### 测试方案 A

1. 编译: `cargo build --bin mir2_client`
2. 运行主程序
3. 在记事本中输入"测试中文"
4. 复制 (Ctrl+C)
5. 在游戏登录界面,点击账号输入框
6. 粘贴 (Ctrl+V)
7. 应该能看到"测试中文"出现在输入框中

## 📊 方案对比

| 特性 | 方案 A (粘贴) | 方案 B (完整 IME) |
|------|--------------|------------------|
| 实施时间 | 1 小时 | 1 天 |
| 风险 | 极低 | 中等 |
| 用户体验 | 中等 (需要额外步骤) | 优秀 (原生输入) |
| 测试工作量 | 小 | 大 |
| 是否推荐先做 | ✅ 是 | ⏸️ 后续 |

## 🎯 建议

**今天**: 实施方案 A (粘贴支持)
- 工作量小
- 立即可用
- 不影响现有代码
- 可以先让用户使用起来

**下周/有时间**: 升级到方案 B (完整 IME)
- 基于测试程序的成功经验
- 提供最佳用户体验
- 可以参考 `test_ime_custom_loop.rs` 的实现

---

**需要我现在帮您实施方案 A 吗?** (只需修改一个文件,5分钟完成)
