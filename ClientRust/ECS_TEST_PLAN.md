# ECS 系统测试计划

**创建日期**: 2025-10-28  
**版本**: v1.0  
**目的**: 系统地测试重构后的ECS五层架构

---

## 📋 测试概览

根据从README文档中了解的架构，我们需要按照五层架构逐层测试：

1. **Layer 1 - 输入与网络层**: 输入收集、网络通信
2. **Layer 2 - 核心逻辑层**: 移动、预测、战斗、AI
3. **Layer 3 - 表现状态层**: 动画状态、音效触发
4. **Layer 4 - 渲染层**: 相机、渲染、动画播放
5. **Layer 5 - UI层**: 对话框、事件处理

---

## 🎯 测试检查清单

### ✅ 已完成的测试

从终端输出可以看到：
- [x] 地图库加载 (137/400个库已加载)
- [x] 游戏内容库初始化 (Monsters, NPCs, Gates等)
- [x] 地图加载 (700x700)
- [x] 玩家实体创建
- [x] 怪物生成 (15只测试怪物)
- [x] 相机系统 (跟随玩家)
- [x] 角色动画渲染 (CArmour库加载)
- [x] UI对话框显示 (MainDialog)
- [x] 对话框按钮点击事件
- [x] 任务系统加载 (87个任务)

### ❌ 发现的问题

1. **容量溢出崩溃** (已修复) ✅
   - 位置: `SharedRust/src/data/item.rs:507`
   - 原因: 服务器发送的 `slot_count` 异常
   - 修复: 添加了范围验证 (0-100)

---

## 📝 详细测试项

### Layer 1: 输入与网络层

#### 1.1 输入收集系统 (InputCollectingSystem)
- [x] 鼠标左键点击检测
- [x] 鼠标右键点击检测
- [ ] 双击检测
- [ ] 键盘按键检测
- [ ] 鼠标滚轮缩放

**测试方法**:
```rust
// 在 input_collecting_system.rs 中添加调试日志
info!("🖱️ 鼠标点击: ({}, {}), 按钮: {:?}", x, y, button);
```

#### 1.2 网络通信系统 (ClientNetworkSystem)
- [x] 连接服务器
- [x] 接收服务器数据包
- [ ] 发送客户端数据包
- [ ] 断线重连
- [ ] 心跳包

**测试方法**:
```bash
# 查看网络日志
cd ClientRust
cargo run --bin mir2x 2>&1 | grep "网络\|Network"
```

---

### Layer 2: 核心逻辑层

#### 2.1 本地预测系统 (LocalPredictionSystem)
- [ ] 玩家点击移动预测
- [ ] 寻路计算
- [ ] 零延迟移动

**测试步骤**:
1. 点击地图某处
2. 观察玩家是否立即开始移动
3. 检查是否有平滑的路径

**预期结果**: 点击后玩家立即响应，无网络延迟感

#### 2.2 移动系统 (MovementSystem)
- [ ] 速度更新位置
- [ ] 碰撞检测
- [ ] 边界限制

**测试方法**:
```rust
// 添加调试输出
info!("🚶 玩家移动: pos=({}, {}), vel=({}, {})", 
      pos.x, pos.y, vel.x, vel.y);
```

#### 2.3 校正系统 (ReconciliationSystem)
- [ ] 服务器位置校正
- [ ] 误差检测
- [ ] 平滑校正

**测试方法**: 故意制造延迟，观察是否有瞬移

#### 2.4 插值系统 (InterpolationSystem)
- [ ] 其他玩家移动插值
- [ ] 怪物移动插值

#### 2.5 怪物AI系统 (MonsterSystem)
- [x] 怪物生成
- [ ] 怪物巡逻
- [ ] 怪物追击玩家
- [ ] 怪物攻击
- [ ] 怪物死亡

**测试方法**:
1. 靠近怪物
2. 观察怪物是否追击
3. 攻击怪物，观察反应

#### 2.6 战斗系统 (CombatSystem)
- [ ] 玩家攻击
- [ ] 伤害计算
- [ ] 命中判定
- [ ] 暴击判定

#### 2.7 技能系统 (MagicCastSystem)
- [ ] 技能施放
- [ ] MP消耗
- [ ] 冷却时间
- [ ] 技能效果

---

### Layer 3: 表现状态层

#### 3.1 动画状态系统 (AnimationStateSystem)
- [x] 站立动画
- [ ] 行走动画
- [ ] 跑步动画
- [ ] 攻击动画
- [ ] 施法动画
- [ ] 死亡动画

**当前状态**: 角色一直播放站立动画 (Stand, frame 0)

**测试方法**:
1. 点击移动 → 应切换到行走动画
2. 攻击怪物 → 应切换到攻击动画

#### 3.2 怪物动画状态系统
- [ ] 怪物站立
- [ ] 怪物移动
- [ ] 怪物攻击

#### 3.3 音效触发系统
- [ ] 脚步声
- [ ] 攻击音效
- [ ] 技能音效
- [ ] 环境音效

---

### Layer 4: 渲染层

#### 4.1 相机系统 (CameraSystem)
- [x] 跟随玩家
- [ ] 边缘滚动
- [ ] 平滑移动
- [ ] 缩放功能

**当前状态**: 相机正确跟随玩家 (11448.0, 18288.0)

#### 4.2 渲染系统 (RenderSystem)
- [x] 地图瓦片渲染
- [x] 角色渲染
- [ ] 怪物渲染
- [ ] NPC渲染
- [ ] 物品渲染
- [ ] Y-sorting排序

**已验证**: 
- 地图加载: 162744个瓦片实体
- 角色渲染: CArmour(0) 库, 1616张图像
- 角色帧计算正确: FinalFrame=824

#### 4.3 动画播放系统
- [ ] 帧更新
- [ ] 循环播放
- [ ] 帧率控制

#### 4.4 移动插值系统
- [ ] 视觉平滑移动

---

### Layer 5: UI层

#### 5.1 对话框管理系统
- [x] MainDialog 显示
- [x] Inventory 对话框开/关
- [x] Character 对话框开/关
- [x] Skills 对话框开/关
- [x] Quest 对话框开/关
- [ ] Options 对话框 (暂未实现)
- [ ] Menu 对话框 (暂未实现)
- [ ] GameShop 对话框 (暂未实现)

**测试记录**:
```
✅ Character按钮点击响应
✅ Inventory对话框切换
✅ Skills对话框切换
✅ Quest对话框切换
⚠️ Options按钮点击 - "暂未实现"
⚠️ Menu按钮点击 - "暂未实现"
⚠️ GameShop按钮点击 - "暂未实现"
```

#### 5.2 UI事件系统
- [x] 鼠标点击事件
- [ ] 鼠标拖拽
- [ ] 键盘快捷键

#### 5.3 物品系统
- [ ] 物品拾取
- [ ] 物品拖拽
- [ ] 物品使用
- [ ] 物品丢弃

#### 5.4 任务系统
- [x] 任务列表加载 (87个任务)
- [ ] 任务接受
- [ ] 任务完成
- [ ] 任务奖励

#### 5.5 交易系统
- [ ] 发起交易
- [ ] 添加物品
- [ ] 确认交易

---

## 🐛 已知问题清单

### 高优先级 (P0)

1. ✅ **[已修复] 容量溢出崩溃**
   - 文件: `SharedRust/src/data/item.rs:507`
   - 现象: 程序崩溃 "capacity overflow"
   - 原因: 服务器发送的 slot_count 异常
   - 修复: 添加范围验证

### 中优先级 (P1)

2. **角色不移动**
   - 现象: 点击地图后角色不移动
   - 可能原因: 
     - LocalPredictionSystem 未正确处理输入
     - MovementSystem 未更新速度
     - 寻路系统未触发

3. **动画不切换**
   - 现象: 角色一直播放站立动画
   - 可能原因: AnimationStateSystem 未检测到动作变化

### 低优先级 (P2)

4. **部分UI未实现**
   - Options 对话框
   - Menu 对话框
   - GameShop 对话框

---

## 🔬 测试方法建议

### 1. 添加调试日志

在关键系统中添加日志：

```rust
// Layer 2: 移动系统
info!("🚶 Movement: entity={:?}, pos=({}, {})", entity, pos.x, pos.y);

// Layer 3: 动画状态
info!("🎭 Animation: action={:?}, frame={}/{}", action, frame, total);

// Layer 4: 渲染
info!("🎨 Render: entity={:?}, sprite_index={}", entity, index);
```

### 2. 可视化调试

启用调试渲染：

```rust
// 在 game_scene.rs 中
let debug_enabled = true;

// 显示：
// - 实体边界框
// - 碰撞区域
// - 路径点
// - 速度向量
```

### 3. 单元测试

为关键系统编写测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_movement_system() {
        // 创建测试世界
        // 添加测试实体
        // 运行系统
        // 验证结果
    }
}
```

---

## 📊 下一步计划

### 第一阶段: 修复核心功能 (1-2天)

1. ✅ 修复容量溢出问题
2. 🔧 修复角色移动问题
   - 检查 LocalPredictionSystem
   - 检查 MovementSystem
   - 检查输入事件传递
3. 🔧 修复动画切换问题
   - 检查 AnimationStateSystem
   - 验证动作状态更新

### 第二阶段: 完善游戏逻辑 (3-5天)

4. 实现怪物AI
5. 实现战斗系统
6. 实现技能系统
7. 完善UI对话框

### 第三阶段: 优化与测试 (2-3天)

8. 性能优化
9. 内存优化
10. 完整功能测试
11. 压力测试

---

## 📝 测试记录

### 2025-10-28 测试会话

**测试时间**: 04:53 - ?  
**测试者**: AI + User  
**测试版本**: ECS重构版 v2.0

**测试结果**:
- ✅ 程序启动正常
- ✅ 地图加载成功 (700x700)
- ✅ UI响应正常
- ❌ 程序崩溃 (已修复)
- ⏳ 角色移动待测试
- ⏳ 战斗系统待测试

**下次测试计划**:
1. 重新编译测试崩溃修复
2. 测试角色移动功能
3. 测试怪物AI

---

## 📚 参考文档

- [src/README.md](./src/README.md) - 项目架构总览
- [src/ecs/systems/README.md](./src/ecs/systems/README.md) - 五层架构详解
- [src/ecs/components/README.md](./src/ecs/components/README.md) - 组件系统
- [src/ecs/scenes/README.md](./src/ecs/scenes/README.md) - 场景系统
- [src/ecs/ui/README.md](./src/ecs/ui/README.md) - UI系统
- [src/graphics/README.md](./src/graphics/README.md) - 图形系统
- [src/network/README.md](./src/network/README.md) - 网络系统
