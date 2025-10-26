# NPC 闪烁问题调试指南

## 问题描述
公告牌NPC在游戏中时有时无，大约1-2秒闪烁一次。

## 已添加的调试日志

### 1. 网络事件日志
- `🔷 [NPC闪烁调试] 收到 ObjectSpawned` - NPC生成事件
- `🗑️ [NPC闪烁调试] 移除网络对象` - NPC移除事件

### 2. 渲染错误日志
- `⚠️ [NPC闪烁] 图像为空` - 纹理加载成功但图像为空
- `⚠️ [NPC闪烁] 纹理加载失败` - 纹理加载失败

## 测试步骤

1. **清理旧日志**
   ```powershell
   cd ClientRust
   Remove-Item -Path target/debug/*.log -ErrorAction SilentlyContinue
   ```

2. **运行游戏**
   ```powershell
   $env:RUST_LOG="warn,mir2_client=info"
   cargo run --package mir2_client --bin mir2x 2>&1 | Tee-Object -FilePath npc_debug.log
   ```

3. **重现问题**
   - 登录游戏
   - 进入游戏场景
   - 确认公告牌NPC存在
   - 走2-3步，观察NPC是否开始闪烁
   - 等待10秒，持续观察

4. **收集日志**
   ```powershell
   # 搜索NPC相关事件
   Select-String -Path npc_debug.log -Pattern "NPC闪烁"
   
   # 搜索ObjectSpawned/ObjectRemoved
   Select-String -Path npc_debug.log -Pattern "ObjectSpawned|ObjectRemoved"
   ```

## 可能的原因

### A. 服务器重复发送 ObjectRemove/ObjectSpawn
**症状**: 日志中出现交替的 `ObjectSpawned` 和 `ObjectRemoved`
**解决方案**: 需要修复服务器端逻辑

### B. 纹理加载失败
**症状**: 日志中出现 `纹理加载失败` 或 `图像为空`
**原因**: 
- 帧索引计算错误
- NPC库索引错误
- 纹理文件不存在或损坏

**解决方案**: 
```rust
// 检查帧索引范围
if final_frame >= 0 && final_frame < library.image_count {
    // 加载纹理...
} else {
    // 使用默认帧
}
```

### C. Animation 组件缺失 Direction
**症状**: 日志中 NPC 数量为 0
**原因**: `draw_single_npc` 要求 Direction 组件，如果缺失则跳过渲染

**解决方案**:
```rust
// 在创建NPC时确保添加Direction组件
world.spawn((
    NPCData { .. },
    Position { .. },
    Animation { .. },
    Direction::new(MirDirection::Down), // ✅ 必须添加
));
```

### D. Y-sorting 收集逻辑问题
**症状**: NPC 数量时而有时而无
**调试**: 查看 Y-sorting 日志中的 npc_count

## 调试按键

按 **F9** 切换NPC边框绘制，确认NPC的渲染范围

## 预期输出

### 正常情况（NPC 不闪烁）
```
🔷 [NPC闪烁调试] 收到 ObjectSpawned: ID=100001, name=公告牌
// ... 游戏运行中，无更多NPC事件 ...
```

### 异常情况（服务器重复发送）
```
🔷 [NPC闪烁调试] 收到 ObjectSpawned: ID=100001, name=公告牌
🗑️ [NPC闪烁调试] 移除网络对象: ID=100001, type=NPC, name=公告牌
🔷 [NPC闪烁调试] 收到 ObjectSpawned: ID=100001, name=公告牌
🗑️ [NPC闪烁调试] 移除网络对象: ID=100001, type=NPC, name=公告牌
// ... 循环重复 ...
```

### 异常情况（纹理加载失败）
```
⚠️ [NPC闪烁] NPC 公告牌 纹理加载失败! frame=123, lib_index=0, error=...
⚠️ [NPC闪烁] NPC 公告牌 纹理加载失败! frame=124, lib_index=0, error=...
// ... 每帧都失败 ...
```

## 下一步

根据日志输出确定具体原因，然后：
1. **如果是服务器问题**: 需要检查服务器端的可见范围判定逻辑
2. **如果是纹理问题**: 需要修复帧索引计算或添加错误处理
3. **如果是组件问题**: 需要确保NPC创建时添加所有必需组件
