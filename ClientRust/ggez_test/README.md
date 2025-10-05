# Ggez独立测试项目

## 快速验证ggez是否工作

这是一个独立的Cargo项目，用于验证ggez 0.10.0-rc0是否可以正常工作。

### 运行

```bash
cd ggez_test
cargo run
```

### 预期结果

- 窗口显示 "Ggez 工作正常!"
- 显示帧数和FPS
- ESC键退出

### 如果成功

说明ggez本身没问题，可以继续集成到主项目。

### 如果失败

检查：
1. Windows版本（需要支持wgpu的GPU驱动）
2. 显卡驱动是否最新
3. ggez版本是否正确 (0.10.0-rc0)
