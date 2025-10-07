# 中文输入法问题诊断

## 当前状态
无法输入中文字符到测试程序中。

## 诊断步骤

### 1. 运行最小化测试程序
```powershell
cargo run --bin test_ime_minimal
```

这个程序会显示**所有**键盘输入事件的详细信息。

### 2. 测试步骤

1. **启动程序** - 看到窗口
2. **输入英文** - 输入 "hello"
   - 观察控制台输出
   - 看到 `KEY_DOWN_EVENT` 信息
   - 看到 `Text: Some("h")` 等
3. **切换中文输入法** (Win+Space 或 Shift+Ctrl)
   - 确认系统托盘显示中文输入法图标
4. **输入拼音** - 输入 "nihao"
   - 观察候选框是否弹出
   - 观察控制台输出什么
5. **选择汉字** - 按数字键或点击选择 "你好"
   - **关键**: 看控制台是否输出了汉字
   - 查找 `Text: Some("你")` 这样的输出

### 3. 检查点

#### 正常情况应该看到:
```
=== KEY_DOWN_EVENT ===
Physical Key: Code(KeyH)
Text: Some("h")
>>> 收到文本: 'h'
>>> 添加字符: 'h' (U+0068)

=== KEY_DOWN_EVENT ===
Physical Key: Code(KeyN)
Text: Some("你")      <-- 重点:这里应该是汉字!
>>> 收到文本: '你'
>>> 添加字符: '你' (U+4F60)
    ✓ 这是中文字符!
```

#### 如果看到:
```
Text: None
```
或者
```
Text: Some("n")  <-- 还是拼音字母
```

说明 IME 文本没有传递到程序。

## 可能的原因

### 原因1: IME 未正确启用
**验证**: 
- 在其他程序(记事本)中能否输入中文?
- 系统托盘是否显示中文输入法?

**解决**: 
- 确保系统安装了中文输入法(搜狗、微软拼音等)
- 在系统设置中启用输入法

### 原因2: winit 0.30 的 IME 事件处理
**问题**: ggez 0.10 使用 winit 0.30,可能有 IME 事件处理问题

**验证**: 
- 查看 `input.event.text` 是否包含中文字符
- 或者需要监听其他事件?

**解决**: 
可能需要:
1. 设置 IME 位置: `window.set_ime_position()`
2. 监听 IME 事件: `Ime::Preedit`, `Ime::Commit`

### 原因3: ggez 文本输入事件
**问题**: ggez 可能需要实现 `text_input_event` 而不是 `key_down_event`

**验证**:
- 看最小化测试是否输出 `TEXT_INPUT_EVENT`

## 调试命令

### 查看所有输入相关的 winit 事件
```powershell
# 运行并过滤输出
cargo run --bin test_ime_minimal 2>&1 | Select-String "KEY|TEXT|IME|你|中文"
```

### 检查主客户端的输入
```powershell
# 运行主客户端
cargo run --bin mir2_client

# 在登录界面的输入框中测试中文输入
```

## 下一步行动

### 如果 `input.event.text` 没有中文:
需要修改 winit 事件处理,监听 IME 相关事件:
- `WindowEvent::Ime(Ime::Preedit)` - 拼音编辑中
- `WindowEvent::Ime(Ime::Commit)` - 确认输入

### 如果 `input.event.text` 有中文:
检查主客户端的事件处理流程,确保:
1. `key_consumed` 逻辑正确
2. 文本输入优先级正确
3. 中文字符过滤条件正确

## 临时解决方案

如果 IME 真的无法工作,可以考虑:
1. 使用剪贴板粘贴中文
2. 直接编辑配置文件输入中文
3. 等待 ggez/winit 更新修复 IME 支持

## 参考资料

- winit IME 文档: https://docs.rs/winit/0.30/winit/event/enum.Ime.html
- ggez 文本输入: https://docs.rs/ggez/0.10/ggez/
- 相关 issue: 搜索 "ggez IME" 或 "winit Chinese input"
