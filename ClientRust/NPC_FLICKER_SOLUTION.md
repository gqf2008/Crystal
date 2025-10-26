# NPC闪烁问题 - 解决方案总结

## 🎯 问题诊断

### 现象
公告牌NPC在游戏中时有时无，约每1-2秒闪烁一次，玩家移动后开始出现。

### 调查过程
1. 添加详细调试日志到 `NetworkSystem` 和 `RenderSystem`
2. 运行游戏并收集日志
3. 发现关键错误信息：
   ```
   ⚠️ [NPC闪烁] NPC BorderVillage_Board 纹理加载失败! 
   frame=10, lib_index=45, 
   error="图像索引 10 超出范围 (max: 6)"
   ```

### 根本原因
**帧索引计算超出图库范围：**

1. **DEFAULT_NPC_FRAMES 配置**：
   ```rust
   frames.insert(MirAction::Standing, Frame::basic(0, 4, 0, 450));
   frames.insert(MirAction::Harvest, Frame::basic(12, 10, 0, 200));
   //                                             ^^^^^^^^^
   //                                   start=12, count=10 → 帧范围12-21
   ```

2. **公告牌NPC的图库**：
   - 库索引：45 (BorderVillage_Board)
   - 图像数量：**7帧**（索引0-6）

3. **动画切换问题**：
   - `NPCActionSystem` 会随机切换NPC动作(Standing ↔ Harvest)
   - 当切换到 `Harvest` 时，计算出的帧索引(10, 11等)超出范围
   - `get_or_create_texture(10)` 返回错误
   - NPC不显示 → **闪烁效果**

## ✅ 解决方案

### 实施的修复
在 `src/ecs/systems/render_system/npc.rs::draw_single_npc` 添加**降级处理**：

```rust
// 🔧 修复NPC闪烁：如果帧索引超出范围，降级到第0帧
let image_result = lib_locked.get_or_create_texture(ctx, final_frame as usize);
let image_info = match image_result {
    Ok(info) => info,
    Err(e) => {
        // 帧索引超出范围，尝试使用第0帧作为降级方案
        tracing::debug!("⚠️ [NPC闪烁修复] NPC {} 帧{}加载失败，降级到第0帧。error={:?}", 
            npc.name, final_frame, e);
        match lib_locked.get_or_create_texture(ctx, 0) {
            Ok(fallback) => fallback,
            Err(e2) => {
                // 连第0帧都失败了，跳过此NPC
                tracing::error!("❌ NPC {} 第0帧也加载失败! lib_index={}, error={:?}", 
                    npc.name, lib_index, e2);
                return Ok(());
            }
        }
    }
};
```

### 工作原理
1. **优先使用计算的帧**：尝试加载动画系统计算出的帧索引
2. **降级到第0帧**：如果失败(超出范围)，使用第0帧(默认姿态)
3. **完全失败处理**：如果连第0帧都失败，才跳过NPC

### 效果
- ✅ NPC**始终显示**(即使在错误的动作帧)
- ✅ 不再闪烁消失
- ✅ 日志降级为 `debug` 级别，避免刷屏
- ⚠️ NPC在某些动作时显示为静态(第0帧)，而不是动画

## 📊 修改文件

| 文件 | 修改内容 | 状态 |
|------|---------|------|
| `src/ecs/systems/render_system/npc.rs` | 添加帧索引降级处理 | ✅ 完成 |
| `FIXES_2024.md` | 更新问题状态和解决方案 | ✅ 完成 |

## 🔮 后续优化 (可选)

### 方案1: 为不同NPC配置独立的FrameSet
```rust
// 为公告牌类NPC创建专用配置
pub static SIGNBOARD_NPC_FRAMES: LazyLock<FrameSet> = LazyLock::new(|| {
    let mut frames = HashMap::new();
    frames.insert(MirAction::Standing, Frame::basic(0, 4, 0, 450));
    // 不包含 Harvest 动作，或使用有效的帧范围
    frames
});
```

### 方案2: 限制NPC动作切换
在 `NPCActionSystem` 中检查NPC类型，某些静态NPC(如公告牌)只使用 `Standing` 动作。

### 方案3: 动态检测帧范围
在加载NPC时检测图库的实际帧数，动态调整动画配置。

## 📝 经验总结

1. **添加降级处理**很重要：即使数据不完美，也要确保基本功能可用
2. **调试日志是最好的工具**：详细的错误信息能快速定位问题
3. **资源索引验证**：在使用外部资源(图库)前，应验证索引有效性
4. **分离配置和实现**：使用 FrameSet 配置动画，便于调整和修复

## 🎉 测试结果

**预期行为**：
- NPC始终显示，不再闪烁
- 控制台可能出现 debug 级别的降级日志（正常）
- NPC可能在某些时刻显示为静态姿态（可接受）

**验证方法**：
1. 运行游戏进入游戏场景
2. 观察公告牌NPC是否始终可见
3. 移动角色，持续观察10秒
4. 如果NPC不再消失 = 修复成功 ✅
