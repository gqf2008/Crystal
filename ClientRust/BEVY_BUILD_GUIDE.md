# Bevy 编译快速指南

## 问题: Cargo 文件锁冲突

```
Blocking waiting for file lock on package cache
```

## 解决方案

### 方法 1: 关闭 rust-analyzer (推荐)
1. 在 VS Code 中,按 `Ctrl+Shift+P`
2. 输入 "Disable Extensions"
3. 选择 "rust-analyzer" 并禁用
4. 重新运行编译

### 方法 2: 清理并重新编译
```powershell
# 停止所有 cargo 进程
Get-Process | Where-Object {$_.ProcessName -like "*cargo*"} | Stop-Process -Force

# 等待几秒
Start-Sleep -Seconds 3

# 清理构建
cd d:\Users\gxh\Documents\GitHub\Crystal\ClientRust
cargo clean

# 重新编译
cargo build --bin mir2_bevy
```

### 方法 3: 检查并强制编译
```powershell
# 检查运行中的 cargo 进程
Get-Process | Where-Object {$_.ProcessName -like "*cargo*"}

# 如果有进程,强制停止
Stop-Process -Name cargo -Force

# 编译
cd d:\Users\gxh\Documents\GitHub\Crystal\ClientRust
cargo build --bin mir2_bevy --jobs 1
```

## 编译命令

### 开发模式 (快速编译)
```bash
cargo build --bin mir2_bevy
```

### 运行程序
```bash
cargo run --bin mir2_bevy
```

### 发布模式 (优化)
```bash
cargo build --bin mir2_bevy --release
```

## 预期输出

编译成功后,控制台应该显示:

```
✅ Bevy 原型启动成功!
🎮 窗口大小: 1024x768
📦 插件: DefaultPlugins + 最近邻插值
🏗️ ECS 架构初始化完成
📊 状态机: Loading -> Login -> Select -> Game
🎮 进入游戏状态 (测试模式)
```

如果 MLibrary 加载成功:
```
✅ 加载人物库: Data/ChrSel.lib (索引: 0)
✅ 成功加载测试精灵 (库:0, 图像:0)
🎮 测试玩家已生成在网格坐标 (5, 5)
```

## 测试检查清单

- [ ] 窗口成功打开 (1024×768)
- [ ] 控制台输出无错误
- [ ] MLibrary 成功加载
- [ ] 测试精灵显示在屏幕上
- [ ] 摄像机居中在精灵上
- [ ] 每5秒输出调试信息

## 常见问题

### Q: 窗口打开但是黑屏?
A: 正常!因为我们还没有添加背景或其他可见内容。测试精灵会显示在中央。

### Q: 找不到 Data/ChrSel.lib?
A: 确保在项目根目录运行,或者修改 `assets.rs` 中的路径。

### Q: 编译时间太长?
A: 第一次编译 Bevy 需要 5-10 分钟,后续增量编译会快很多。

### Q: 动态链接错误?
A: 如果遇到问题,可以在 `Cargo.toml` 中移除 `dynamic_linking` 特性。

## 下一步

编译成功后:
1. 验证精灵显示
2. 测试摄像机跟随
3. 开始 Phase 2 (完善系统)
