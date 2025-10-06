# MessageBox 功能说明

## ✅ 已实现功能

### 基础功能
- ✅ 消息框组件结构 (`MessageBox`)
- ✅ 显示/隐藏API
- ✅ 自动关闭计时器支持
- ✅ OK按钮交互
- ✅ UI绘制完成

### UI设计
- **位置**: 屏幕居中
- **尺寸**: 400x200 像素
- **背景**: 深蓝色 (RGB: 50, 50, 80)
- **边框**: 浅蓝色 2px (RGB: 150, 150, 200)
- **遮罩**: 半透明黑色背景 (阻止其他交互)

### 交互
- ✅ 鼠标悬停按钮高亮
- ✅ 点击 OK 按钮关闭
- ✅ ESC 键关闭
- ✅ 阻止底层UI交互

### 文本显示
- ✅ 支持多行文本
- ✅ 自动换行 (`\n`)
- ✅ 黄色标题
- ✅ 白色正文

---

## 🎮 测试方法

### 1. 启动程序
```bash
cargo run --bin mir2_client
```

### 2. 按 M 键
显示测试消息框：
```
这是一个测试消息框!

您可以点击 OK 按钮关闭它。
或按 ESC 键关闭。
```

### 3. 关闭方式
- **方式1**: 点击 OK 按钮
- **方式2**: 按 ESC 键

---

## 📝 API 使用

### 显示简单消息
```rust
self.show_message("登录失败！请检查账号和密码。");
```

### 显示多行消息
```rust
self.show_message("账号被封禁。\n\n原因: 违规操作\n到期时间: 2025-12-31");
```

### 显示带标题的消息
```rust
self.show_message_with_title("账号已存在", "注册失败");
```

---

## 🔧 集成到登录流程

MessageBox 已集成到登录失败处理中：

```rust
fn handle_login_response(&mut self, result: u8) {
    // ...
    if let Some(message) = Self::login_result_message(result) {
        self.record_status(message);
        // 显示错误消息框
        if result != 0 {
            self.show_message(message);
        }
    }
    // ...
}
```

### 错误码对应消息
- **0**: 登录已禁用
- **1**: 账号ID不可接受
- **2**: 密码不可接受
- **3**: 账号不存在
- **4**: 账号或密码错误
- **5**: 需要修改密码

---

## 🎨 样式定制

### 修改颜色
在 `draw_message_box()` 方法中修改：

```rust
// 背景色
GgezColor::from_rgb(50, 50, 80)  // 深蓝色

// 边框色
GgezColor::from_rgb(150, 150, 200)  // 浅蓝色

// 标题色
GgezColor::from_rgb(255, 255, 100)  // 黄色

// 按钮正常状态
GgezColor::from_rgb(70, 70, 150)

// 按钮悬停状态
GgezColor::from_rgb(100, 100, 200)
```

### 修改尺寸
```rust
let box_width = 400.0;   // 宽度
let box_height = 200.0;  // 高度
let button_width = 80.0;  // 按钮宽度
let button_height = 30.0; // 按钮高度
```

---

## 📊 技术细节

### 绘制顺序
1. 半透明背景遮罩 (全屏)
2. 消息框背景矩形
3. 消息框边框
4. 标题文本
5. 消息内容 (多行)
6. OK按钮背景
7. OK按钮边框
8. OK按钮文字

### 事件处理优先级
MessageBox 显示时会阻止所有底层交互：
```rust
if let Some(msg_box) = &mut self.message_box {
    if msg_box.visible {
        // 处理 MessageBox 交互
        // ...
        return; // 阻止其他交互
    }
}
```

---

## ✨ 未来改进

可能的增强功能：
- [ ] 添加多按钮支持 (Yes/No, OK/Cancel)
- [ ] 添加图标支持 (Info, Warning, Error)
- [ ] 添加音效
- [ ] 添加淡入淡出动画
- [ ] 支持自定义按钮文字
- [ ] 支持回调函数

---

## 🐛 已知问题

无

---

## 📅 更新历史

- **2025-10-06**: 初始实现
  - 基础MessageBox组件
  - UI绘制
  - 鼠标和键盘交互
  - 集成到登录流程
