# 快速测试指南 - 中文输入法

## 🚀 快速开始

### 步骤 1: 运行最小化测试
```powershell
cd d:\Users\gxh\Documents\GitHub\Crystal\ClientRust
cargo run --bin test_ime_minimal
```

### 步骤 2: 测试输入
1. 窗口打开后,直接开始输入
2. 先输入英文 `hello` (应该能看到)
3. 按 `Win + Space` 切换到中文输入法
4. 输入拼音 `nihao`
5. 选择汉字 `你好`

### 步骤 3: 观察结果

#### 查看窗口
- 能否看到输入的文字?
- 中文是否正确显示?

#### 查看控制台 (重要!)
寻找这样的输出:
```
=== KEY_DOWN_EVENT ===
Physical Key: Code(KeyN)
Text: Some("你")          <-- 这里是关键!
>>> 收到文本: '你'
>>> 添加字符: '你' (U+4F60)
    ✓ 这是中文字符!
```

## 🔍 问题诊断

### 情况 A: 控制台显示 `Text: Some("你")`
✅ **IME 工作正常!** 
- 问题在于主程序的事件处理
- 需要检查 `LoginScene` 的输入逻辑

### 情况 B: 控制台显示 `Text: Some("n")` (英文字母)
❌ **IME 未正确传递文字**
- winit 0.30 的 IME 事件可能需要特殊处理
- 需要监听 `WindowEvent::Ime` 事件

### 情况 C: 控制台显示 `Text: None`
❌ **没有文本输入**
- 检查输入法是否真的切换成功
- 在记事本中测试输入法是否正常

## 📊 期望的完整输出示例

```
=== 最小化 IME 测试 ===
这个程序会显示所有键盘输入事件
✓ 中文字体已加载
✓ IME 已启用

开始监听输入...

=== KEY_DOWN_EVENT ===
Physical Key: Code(KeyH)
Logical Key:  Character("h")
Text:         Some("h")
Location:     Standard
Repeated:     false
Modifiers:    Shift=false, Ctrl=false, Alt=false
>>> 收到文本: 'h'
>>> 添加字符: 'h' (U+0068)

=== KEY_DOWN_EVENT ===
Physical Key: Code(KeyE)
Text:         Some("e")
>>> 收到文本: 'e'
>>> 添加字符: 'e' (U+0065)

# ... 切换输入法后 ...

=== KEY_DOWN_EVENT ===
Physical Key: Code(KeyN)
Text:         Some("你")        <-- 期望看到中文!
>>> 收到文本: '你'
>>> 添加字符: '你' (U+4F60)
    ✓ 这是中文字符!

=== KEY_DOWN_EVENT ===
Physical Key: Code(KeyH)
Text:         Some("好")
>>> 收到文本: '好'
>>> 添加字符: '好' (U+597D)
    ✓ 这是中文字符!
```

## 🎯 下一步

根据测试结果:

### 如果能输入中文
1. 回到主程序 `mir2_client`
2. 测试登录界面的输入框
3. 如果主程序也能输入,问题解决 ✅

### 如果不能输入中文
我需要知道:
1. 控制台显示了什么?
2. `Text:` 后面是 `Some("你")` 还是 `Some("n")` 还是 `None`?
3. 在其他程序(如记事本)中能否输入中文?

请复制控制台输出并告诉我结果!

## 🛠️ 备选测试

### 测试完整版
```powershell
cargo run --bin test_chinese_ime
```

### 测试主程序
```powershell
cargo run --bin mir2_client
# 在登录界面点击账号输入框测试
```

## 📞 反馈格式

请告诉我:
```
1. 程序启动: [正常/失败]
2. 英文输入: [正常/失败]
3. 切换输入法: [成功/失败]
4. 控制台 Text 显示: [Some("你") / Some("n") / None]
5. 窗口显示文字: [正常/乱码/无显示]
```

---

**提示**: 如果完全无法输入,可以先在主程序中使用**粘贴**功能作为临时方案。
