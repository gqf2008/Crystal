# 新建账号功能测试说明

## BUG修复

**问题**: NewAccountDialog的OK按钮永远不可用  
**原因**: `ok_button_enabled`初始化为`false`，但从未更新  
**修复**: 在`update()`方法中添加验证逻辑，根据必填字段自动更新按钮状态

## 修复内容

文件: `src/ecs/scenes/login_scene/new_account_dialog.rs`

```rust
pub fn update(&mut self, delta_time: f32) {
    // 光标闪烁
    self.cursor_blink_timer += delta_time;
    if self.cursor_blink_timer >= 0.5 {
        self.cursor_blink_timer = 0.0;
        self.cursor_visible = !self.cursor_visible;
    }
    
    // ✅ 新增: 更新OK按钮状态
    self.ok_button_enabled = 
        self.account_id_valid && 
        self.password1_valid && 
        self.password2_valid && 
        self.email_valid &&
        !self.registration.account_id.is_empty() &&
        !self.registration.password.is_empty() &&
        !self.registration.email.is_empty();
}
```

## 测试步骤

### 1. 启动程序
```bash
cargo run --bin mir2x
```

### 2. 打开新建账号对话框
- 点击登录界面的"新建账号"按钮
- 应该看到NewAccountDialog弹出

### 3. 填写必填字段
- **账号** (Account ID): 输入 `test_user_123`
  - 要求: 3-20个字符，字母数字下划线
- **密码** (Password): 输入 `test_pass_456`
  - 要求: 5-20个字符
- **确认密码** (Confirm): 再次输入 `test_pass_456`
  - 要求: 必须与密码相同
- **邮箱** (Email): 输入 `test@example.com`
  - 要求: 有效的邮箱格式

### 4. 验证OK按钮状态
- 填写完所有必填字段后，OK按钮应该变为可用（高亮）
- 如果字段无效，OK按钮保持禁用状态

### 5. 提交注册请求
- 点击OK按钮
- 应该看到控制台输出:
  ```
  🔘 NewAccountDialog OK按钮被点击
  ✅ 已发送新建账号请求: test_user_123
  ```
- Terminal应该显示网络数据包发送日志

### 6. 检查服务器响应
- 如果服务器返回成功: 对话框关闭，显示成功消息
- 如果服务器返回错误: 显示错误消息，自动聚焦到错误字段

## 验证字段规则

### 账号 (Account ID)
- ✅ 长度: 3-20个字符
- ✅ 字符: 字母、数字、下划线
- ❌ 空格、特殊字符

### 密码 (Password)
- ✅ 长度: 5-20个字符
- ✅ 任意可打印字符

### 确认密码 (Confirm Password)
- ✅ 必须与密码完全相同

### 邮箱 (Email)
- ✅ 必须包含 `@` 符号
- ✅ @ 前后都有内容
- ❌ 空字符串

## 已知问题

1. **生日日期解析**: 目前未实现，固定发送0
2. **密钥问题/答案**: 可选字段，未强制验证

## 调试日志关键字

- `🔘 新建账号按钮被点击` - 打开对话框
- `🔘 NewAccountDialog OK按钮被点击` - 提交注册
- `✅ 已发送新建账号请求` - 网络命令已发送
- `📦 Received packet: opcode=5` - 收到NewAccount响应

## 代码路径

- 对话框定义: `src/ecs/scenes/login_scene/new_account_dialog.rs`
- 场景集成: `src/ecs/scenes/login_scene.rs`
  - `open_new_account_dialog()` - 打开对话框
  - `submit_new_account()` - 提交注册
  - `handle_new_account_response()` - 处理服务器响应
- 鼠标事件: `LoginScene::on_mouse_down()` 第1668行

## 修复前后对比

### 修复前
```rust
// update() 方法
pub fn update(&mut self, delta_time: f32) {
    // 只有光标闪烁逻辑
    self.cursor_blink_timer += delta_time;
    if self.cursor_blink_timer >= 0.5 {
        self.cursor_blink_timer = 0.0;
        self.cursor_visible = !self.cursor_visible;
    }
    // ❌ ok_button_enabled 永远是 false
}
```

### 修复后
```rust
// update() 方法
pub fn update(&mut self, delta_time: f32) {
    // 光标闪烁逻辑
    self.cursor_blink_timer += delta_time;
    if self.cursor_blink_timer >= 0.5 {
        self.cursor_blink_timer = 0.0;
        self.cursor_visible = !self.cursor_visible;
    }
    
    // ✅ 根据字段验证状态自动更新按钮
    self.ok_button_enabled = 
        self.account_id_valid && 
        self.password1_valid && 
        self.password2_valid && 
        self.email_valid &&
        !self.registration.account_id.is_empty() &&
        !self.registration.password.is_empty() &&
        !self.registration.email.is_empty();
}
```

## 测试结果预期

✅ **成功场景**:
1. 填写所有必填字段 → OK按钮变为可用
2. 点击OK → 发送NewAccount网络命令
3. 服务器返回Success → 对话框关闭，显示成功消息

❌ **失败场景**:
1. 字段格式错误 → OK按钮禁用，无法提交
2. 服务器返回错误 → 显示错误消息，自动聚焦到错误字段，清空需要修改的字段
