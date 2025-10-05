# 🚀 运行测试指南

## 快速开始

### 1. 编译程序

```powershell
cd d:\Users\gxh\Documents\GitHub\Crystal\ClientRust
cargo build --bin mir2_client
```

### 2. 运行程序

```powershell
cargo run --bin mir2_client
```

## 预期结果

### ✅ 成功标志:

**控制台输出:**
```
========================================
🎮 Crystal Mir2 Client - Ggez版本
========================================

✓ Ggez 渲染管理器已创建: 1024x768
📦 正在加载图形库...
✓ 所有图形库加载成功!
🎨 ✓ 测试渲染成功 - Prguse.lib 图像 #0 已绘制到 (100,100)
```

**窗口:**
- 深蓝色背景(RGB: 20,30,60)
- 左上角(100,100)位置有一个测试图像
- 窗口标题: "Crystal - " (服务器名称)

### ❌ 如果失败:

**1. 库文件未找到**
```
⚠ 加载图形库失败: 9 个库加载失败
```
→ 检查 `Data/` 目录中是否有 .lib 文件

**2. 库未加载**
```
⚠ Prguse 库未加载
```
→ load_core_libraries() 失败,检查路径

**3. 渲染失败**
```
❌ 测试渲染失败: ...
```
→ ggez API 问题,检查日志

## 调试

### 启用详细日志:

```powershell
$env:RUST_LOG="debug"
cargo run --bin mir2_client
```

### 检查库文件:

```powershell
Get-ChildItem Data\*.lib | Select-Object Name, Length
```

应该看到:
- Prguse.lib (~12 MB)
- Prguse2.lib (~3.5 MB)
- Magic.lib (~9 MB)
- 等等...

## 控制

- **Ctrl+Q**: 退出程序
- **鼠标**: 可以移动窗口
- **键盘**: 暂无功能(等待实现)

## 下一步

如果测试成功,你会看到:
1. ✅ 窗口打开
2. ✅ 蓝色背景
3. ✅ 测试图像渲染
4. ✅ 控制台显示成功消息

然后我们可以继续实现:
- LoginScene 完整UI
- 用户输入(用户名/密码)
- 网络连接
- 等等...

---

**当前状态:** 渲染管线已准备好!🎉
