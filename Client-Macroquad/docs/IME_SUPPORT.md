# IME 输入法支持

## 当前状态

- ✅ 中文输入已支持（通过 `get_char_pressed()` 获取 IME 最终输出的字符）
- ❌ IME 候选框位置无法控制（显示在屏幕左上角）

## 问题根源

macroquad/miniquad 底层没有完整的 IME 支持。相关 Issue：
- https://github.com/not-fl3/miniquad/issues/781

## 需要修改的位置

### 主要修改：miniquad

miniquad 是底层窗口/图形/输入处理库，直接与操作系统交互。

需要修改的文件：
```
miniquad/src/native/windows.rs  - Windows 平台
miniquad/src/native/linux_x11.rs - Linux X11
miniquad/src/native/macos.rs    - macOS
```

### 次要修改：macroquad

macroquad 只需要把 miniquad 的新 IME 事件暴露出来（很小的改动）。

## Windows IME 实现要点

### 需要处理的 Windows 消息

```rust
// 在 windows.rs 的消息循环中添加处理
WM_IME_SETCONTEXT      // IME 上下文切换
WM_IME_STARTCOMPOSITION // 开始输入
WM_IME_COMPOSITION     // 获取正在输入的拼音/候选字
WM_IME_ENDCOMPOSITION  // 输入完成，获取最终文字
WM_IME_NOTIFY          // IME 状态变化通知
```

### 设置候选框位置

```rust
use windows_sys::Win32::UI::Input::Ime::*;

// 获取 IME 上下文
let himc = ImmGetContext(hwnd);

// 设置候选框位置
let mut composition_form = COMPOSITIONFORM {
    dwStyle: CFS_POINT,
    ptCurrentPos: POINT { x: cursor_x, y: cursor_y },
    rcArea: RECT { left: 0, top: 0, right: 0, bottom: 0 },
};
ImmSetCompositionWindow(himc, &mut composition_form);

// 释放 IME 上下文
ImmReleaseContext(hwnd, himc);
```

### 新增的事件类型

```rust
// 在 miniquad 的事件枚举中添加
pub enum Event {
    // ... 现有事件 ...
    
    /// IME 输入开始
    ImeStart,
    
    /// IME 正在输入（preedit 文本）
    ImePreedit {
        text: String,
        cursor_pos: usize,
    },
    
    /// IME 输入完成
    ImeCommit {
        text: String,
    },
    
    /// IME 输入取消
    ImeCancel,
}
```

### 需要暴露的 API

```rust
// 设置 IME 候选框位置
pub fn set_ime_position(x: f32, y: f32);

// 启用/禁用 IME
pub fn set_ime_enabled(enabled: bool);
```

## 实现步骤

1. Fork miniquad 仓库
2. 在 `src/native/windows.rs` 中：
   - 添加 IME 相关的 Windows API 绑定
   - 在窗口消息循环中处理 IME 消息
   - 实现 `set_ime_position` 函数
3. 在 `src/event.rs` 中添加新的 IME 事件类型
4. 在 macroquad 中暴露新的 API
5. 在 Client-Macroquad 中使用新 API 设置候选框位置

## 参考资料

- [Windows IME 文档](https://docs.microsoft.com/en-us/windows/win32/intl/input-method-manager)
- [winit 的 IME 实现](https://github.com/rust-windowing/winit/blob/master/src/platform_impl/windows/ime.rs)
- [egui-winit 的 IME 处理](https://github.com/emilk/egui/blob/master/crates/egui-winit/src/lib.rs)

## 临时方案

当前项目中已创建了 IME 位置控制模块（已禁用）：
- `src/platform/mod.rs`
- `src/platform/ime.rs`

这些代码尝试在应用层直接调用 Win32 API，但会干扰 miniquad 的事件处理。
正确的做法是在 miniquad 内部集成 IME 支持。
