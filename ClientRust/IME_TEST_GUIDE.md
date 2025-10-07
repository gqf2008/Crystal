# 中文输入法测试程序使用说明

## 概述
这是一个独立的测试程序,用于验证 ggez 0.10 + winit 0.30 框架下的中文输入法(IME)支持。

## 运行方法

```powershell
# 编译并运行
cargo run --bin test_chinese_ime

# 或者先编译
cargo build --bin test_chinese_ime
# 然后运行
.\target\debug\test_chinese_ime.exe
```

## 功能说明

### 1. 三个测试输入框
- **输入框 1**: 测试基本中文输入
- **输入框 2**: 测试多输入框切换
- **输入框 3**: 测试焦点管理

### 2. 中文输入法支持
- ✅ **IME 启用**: 程序启动时自动启用 IME
- ✅ **中文字符输入**: 支持拼音输入法
- ✅ **候选框显示**: 系统输入法候选框正常显示
- ✅ **字符确认**: 选择候选字后正确输入到文本框

### 3. 视觉反馈
- **焦点状态**: 
  - 无焦点: 灰色边框 + 灰色背景
  - 有焦点: 蓝色边框 + 白色背景
- **光标显示**: 黑色光标,0.5秒闪烁
- **IME 编辑**: 橙色下划线表示正在输入法编辑中
- **文本渲染**: 使用阿里巴巴普惠体显示中文

### 4. 事件日志
- 实时显示输入事件
- 区分中文字符和英文字符
- 显示最近8条日志

## 操作说明

### 基本操作
1. **点击输入框**: 激活输入焦点
2. **切换输入法**: 
   - Windows: `Win + Space` 或 `Shift + Ctrl`
   - 切换到中文输入法(搜狗、微软拼音等)
3. **输入拼音**: 直接输入拼音,候选框会弹出
4. **选择汉字**: 按数字键或点击选择候选字
5. **继续输入**: 输入更多汉字

### 快捷键
- `Backspace`: 删除光标前的字符
- `Ctrl + A`: 清空当前输入框
- `ESC`: 退出程序
- `鼠标点击`: 切换输入框焦点

## 测试场景

### 场景1: 基本中文输入
```
1. 点击输入框1
2. 切换到中文输入法
3. 输入 "nihao" → 选择 "你好"
4. 观察: 文本框显示 "你好"
5. 日志显示: "输入中文字符: 你" "输入中文字符: 好"
```

### 场景2: 中英混输
```
1. 输入 "hello"
2. 切换到中文输入法
3. 输入 "shijie" → 选择 "世界"
4. 切换回英文输入法
5. 输入 "!"
6. 观察: "hello世界!"
```

### 场景3: 多输入框切换
```
1. 在输入框1输入 "测试1"
2. 点击输入框2
3. 输入 "测试2"
4. 点击输入框3
5. 输入 "测试3"
6. 观察: 三个输入框独立保持各自内容
```

### 场景4: 删除和清空
```
1. 输入 "你好世界"
2. 按 Backspace 3次 → 删除 "界世好"
3. 剩余 "你"
4. 按 Ctrl+A → 清空
5. 观察: 输入框为空
```

## 技术实现

### IME 启用
```rust
// 在创建窗口后启用 IME
ctx.gfx.window().set_ime_allowed(true);
```

### 文本输入事件处理
```rust
fn key_down_event(&mut self, ctx: &mut Context, input: KeyInput, _repeated: bool) -> GameResult {
    if let PhysicalKey::Code(keycode) = input.event.physical_key {
        // 处理 IME 文本输入
        if let Some(text) = &input.event.text {
            for ch in text.chars() {
                if !ch.is_control() {
                    // 插入字符
                    input_box.insert_char(ch);
                    
                    // 判断是否为中文字符 (Unicode CJK 统一汉字: U+4E00 - U+9FFF)
                    if ch as u32 >= 0x4E00 && ch as u32 <= 0x9FFF {
                        println!("输入中文字符: {}", ch);
                    }
                }
            }
        }
    }
    Ok(())
}
```

### 中文字符检测
使用 Unicode 范围判断:
- **CJK 统一汉字**: U+4E00 - U+9FFF (常用汉字)
- **扩展区域**: U+3400 - U+4DBF, U+20000 - U+2A6DF 等

```rust
// 判断是否为中文字符
fn is_chinese_char(ch: char) -> bool {
    let code = ch as u32;
    (code >= 0x4E00 && code <= 0x9FFF) ||   // CJK 统一汉字
    (code >= 0x3400 && code <= 0x4DBF) ||   // CJK 扩展 A
    (code >= 0x20000 && code <= 0x2A6DF)    // CJK 扩展 B
}
```

### 字体加载
```rust
// 加载中文字体
let font_path = "resources/font/AlibabaPuHuiTi-3-55-Regular.ttf";
if let Ok(font_bytes) = std::fs::read(font_path) {
    ctx.gfx.add_font(
        "AlibabaPuHuiTi",
        ggez::graphics::FontData::from_vec(font_bytes)?,
    );
}

// 使用字体绘制文本
let text = Text::new(
    TextFragment::new("你好世界")
        .font("AlibabaPuHuiTi")
        .scale(20.0)
);
```

## 已知特性

### ✅ 支持的功能
- [x] IME 启用和禁用
- [x] 中文字符输入
- [x] 系统输入法候选框显示
- [x] 中文字体渲染
- [x] 光标闪烁
- [x] 多输入框管理
- [x] 焦点状态切换
- [x] Backspace 删除
- [x] 字符插入

### ⚠️ 限制
- **光标位置**: 当前简化为文本末尾插入(未实现中间插入)
- **选择文本**: 未实现文本选择和复制粘贴
- **IME 候选框定位**: 使用系统默认位置(未自定义)
- **长文本滚动**: 未实现横向滚动

### 📝 与主客户端的差异
- **测试程序**: 简化的输入框实现,专注测试 IME
- **主客户端**: 使用完整的 `LoginDialog`、`NewAccountDialog` 等
- **目的**: 验证 ggez + winit 的 IME 支持是否正常

## 调试信息

### 日志输出
程序会在控制台和窗口内显示事件日志:

```
[LOG] 输入框 1 获得焦点
[LOG] 输入字符: h
[LOG] 输入字符: e
[LOG] 输入字符: l
[LOG] 输入字符: l
[LOG] 输入字符: o
[LOG] 输入中文字符: 世
[LOG] 输入中文字符: 界
[LOG] 删除字符,剩余: hello世
```

### 性能
- **FPS**: 预期 60 FPS
- **输入延迟**: < 16ms
- **内存**: < 50MB

## 验证清单

测试前请确认:

- [ ] 系统已安装中文输入法(搜狗、微软拼音等)
- [ ] 字体文件存在: `resources/font/AlibabaPuHuiTi-3-55-Regular.ttf`
- [ ] 编译无错误: `cargo build --bin test_chinese_ime`

测试步骤:

1. [ ] 程序启动成功,显示3个输入框
2. [ ] 点击输入框1,边框变为蓝色
3. [ ] 切换中文输入法成功(状态栏显示)
4. [ ] 输入拼音,候选框正常弹出
5. [ ] 选择汉字,文本框显示中文
6. [ ] 中文字体渲染正常(无乱码、方块)
7. [ ] 日志显示"输入中文字符"
8. [ ] Backspace 正常删除中文字符
9. [ ] 切换到输入框2,焦点正常转移
10. [ ] Ctrl+A 清空输入框
11. [ ] ESC 退出程序

## 常见问题

### Q1: 候选框不显示?
**A**: 检查是否已切换到中文输入法(Windows 状态栏会显示"拼"或"搜")

### Q2: 输入的是乱码?
**A**: 字体文件加载失败,检查 `resources/font/` 目录

### Q3: 无法输入中文?
**A**: 
1. 确认 IME 已启用(程序日志应该显示 "✓ IME 已启用")
2. 尝试重启输入法进程
3. 检查是否在输入框内点击获得焦点

### Q4: 光标位置不对?
**A**: 当前版本使用简化的光标计算(固定12像素/字符),实际项目应该使用字体度量

## 下一步

测试成功后,可以将 IME 支持集成到主客户端:

1. ✅ 确认 `main_ggez.rs` 已启用 IME
2. ✅ 确认所有输入框使用相同的文本处理逻辑
3. ✅ 确认中文字体加载成功
4. [ ] 添加更多输入法相关功能:
   - 候选框定位
   - IME 编辑状态显示
   - 快捷键支持

## 参考资料

- **winit IME 文档**: https://docs.rs/winit/0.30/winit/
- **ggez 文本输入**: https://docs.rs/ggez/0.10/ggez/
- **Unicode CJK**: https://en.wikipedia.org/wiki/CJK_Unified_Ideographs

---

**测试完成日期**: 2025年10月6日
**框架版本**: ggez 0.10.0-rc0 + winit 0.30
**字体**: 阿里巴巴普惠体
