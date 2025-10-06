# 对话框实现清单

## ✅ 已完成

### LoginDialog
- [x] 基础结构
- [x] 文本输入 (AccountID, Password)
- [x] 光标闪烁动画
- [x] 焦点管理
- [x] 按钮交互 (OK, NewAccount, ChangePassword, Close)
- [x] 输入验证
- [x] Tab键切换

### MessageBox
- [x] 基础结构
- [x] 显示/隐藏API
- [x] OK按钮交互逻辑
- [x] 自动关闭计时器
- [ ] UI绘制 (draw方法)
- [ ] 在LoginScene中集成绘制

---

## 🚧 NewAccountDialog

### 基础结构 (已存在)
- [x] 数据结构定义
- [x] 8个输入字段
- [x] 验证状态字段
- [x] NewAccountResult枚举

### 需要添加
- [ ] 焦点管理 (focused_field)
- [ ] 光标闪烁 (cursor_visible, cursor_blink_timer)
- [ ] 按钮悬停状态
- [ ] update() 方法 (光标闪烁)
- [ ] handle_text_input() 方法
- [ ] handle_backspace() 方法
- [ ] handle_tab() 方法
- [ ] handle_mouse_move() 方法
- [ ] handle_click() 方法
- [ ] UI绘制逻辑

---

## 🚧 ChangePasswordDialog

### 基础结构 (已存在)
- [x] 数据结构定义
- [x] 3个输入字段
- [x] 验证状态字段
- [x] ChangePasswordResult枚举

### 需要添加
- [ ] 焦点管理 (focused_field)
- [ ] 光标闪烁 (cursor_visible, cursor_blink_timer)
- [ ] 按钮悬停状态
- [ ] update() 方法 (光标闪烁)
- [ ] handle_text_input() 方法
- [ ] handle_backspace() 方法
- [ ] handle_tab() 方法
- [ ] handle_mouse_move() 方法
- [ ] handle_click() 方法
- [ ] UI绘制逻辑

---

## 🎯 SelectScene

### 需要创建
- [ ] 基础结构
- [ ] 角色列表显示
- [ ] 角色选择交互
- [ ] 开始游戏按钮
- [ ] 新建角色按钮
- [ ] 删除角色按钮
- [ ] UI绘制逻辑

---

## 优先级

1. **MessageBox UI绘制** - 让消息框可以显示
2. **测试MessageBox** - 在登录失败时显示
3. **NewAccountDialog交互** - 复制LoginDialog的交互模式
4. **ChangePasswordDialog交互** - 复制LoginDialog的交互模式
5. **SelectScene基础** - 创建基本结构

---

## 设计模式参考

所有对话框都应该遵循相同的模式：

```rust
pub struct Dialog {
    // UI state
    pub visible: bool,
    pub enabled: bool,
    
    // Input fields
    pub field1: String,
    pub field2: String,
    
    // Focus state
    pub focused_field: FocusedField,
    
    // Cursor state
    pub cursor_visible: bool,
    pub cursor_blink_timer: f32,
    
    // Button hover states
    pub ok_button_hovered: bool,
    pub cancel_button_hovered: bool,
}

impl Dialog {
    pub fn update(&mut self, delta_time: f32) { }
    pub fn handle_text_input(&mut self, ch: char) { }
    pub fn handle_backspace(&mut self) { }
    pub fn handle_tab(&mut self) { }
    pub fn handle_mouse_move(&mut self, x: f32, y: f32) { }
    pub fn handle_click(&mut self, x: f32, y: f32) -> ClickResult { }
}
```

---

## 估计工作量

- MessageBox UI绘制: 30分钟
- MessageBox测试: 15分钟
- NewAccountDialog交互: 1小时
- ChangePasswordDialog交互: 45分钟
- SelectScene基础: 1小时

**总计**: 约 3.5 小时
