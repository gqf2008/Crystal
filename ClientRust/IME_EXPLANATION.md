# IME 事件监听说明

## 问题确认 ✅

从 `test_ime_minimal` 的输出可以看到:

### 英文输入正常
```
Text: Some("h")  ← 有文本
```

### 中文输入失败
```
Logical Key: Named(Process)  ← IME 在处理
Text: None                    ← 没有文本!
```

**结论**: winit 0.30 不会通过 `KeyInput.text` 传递中文字符!

## 正确方式: 监听 WindowEvent::Ime

中文字符通过 `WindowEvent::Ime::Commit` 事件传递:

```rust
WindowEvent::Ime(Ime::Commit(text)) => {
    // text 包含中文字符 "你好"
}
```

## ⚠️ ggez 0.10 的问题

**ggez 的 EventHandler 不支持 IME 事件!**

```rust
pub trait EventHandler {
    fn key_down_event(...);   // ✓ 有
    fn text_input_event(...);  // ✓ 有
    fn ime_event(...);         // ❌ 没有!
}
```

## 解决方案

### 方案 A: 使用 egui (推荐!)

egui 已经完美支持 IME,直接使用:

```toml
[dependencies]
egui = "0.27"
egui-ggez = "0.3"
```

```rust
use egui_ggez::EguiBackend;

struct LoginScene {
    egui_backend: EguiBackend,
    account: String,
}

impl EventHandler for LoginScene {
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        self.egui_backend.draw(ctx, |egui_ctx| {
            egui::Window::new("登录").show(egui_ctx, |ui| {
                ui.text_edit_singleline(&mut self.account);
                // ✅ 中文输入自动工作!
            });
        });
        Ok(())
    }
}
```

### 方案 B: Fork ggez 添加 IME 支持

在 `ggez/src/event.rs` 中添加:

```rust
fn ime_event(&mut self, ctx: &mut Context, ime: Ime) -> GameResult {
    Ok(())
}
```

然后修改事件循环调用这个方法。

### 方案 C: 绕过 ggez (复杂)

直接使用 winit 事件循环,手动调用 ggez 的更新/绘制。

## 🎯 推荐行动

### 立即可行: 使用 egui

1. 添加依赖:
```toml
[dependencies]
egui = "0.27"
egui-ggez = "0.3"
```

2. 在登录界面集成 egui 文本框

3. 保持游戏画面用 ggez 渲染,UI 输入用 egui

### 优点
- ✅ 中文输入开箱即用
- ✅ 文本选择/复制/粘贴全支持
- ✅ 跨平台 (Windows/Linux/macOS)
- ✅ 不需要修改 ggez

### 演示代码

让我创建一个 egui 集成示例...

## 📊 技术细节

### winit IME 事件流

1. 用户切换输入法 → `Ime::Enabled`
2. 输入拼音 "nihao" → `Ime::Preedit("nihao", cursor_pos)`
3. 选择候选字 → `Ime::Commit("你好")` ← **中文字符在这里!**

### 为什么 KeyInput.text 没有中文?

因为 IME 是"输入法编辑器",它拦截了普通键盘输入:

```
键盘 → IME (拼音转汉字) → 应用程序
     ↑ KeyInput.text = None (被拦截)
                        ↑ Ime::Commit = "你好" (这里才有)
```

## 🔗 相关链接

- [winit IME 文档](https://docs.rs/winit/latest/winit/event/enum.Ime.html)
- [egui-ggez GitHub](https://github.com/lucasmerlin/egui_ggez)
- [ggez 讨论: IME 支持](https://github.com/ggez/ggez/issues)

---

**下一步**: 我将创建一个使用 egui 的登录界面示例
