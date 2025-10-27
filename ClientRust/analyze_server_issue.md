# 服务器移动命令不响应问题分析

## 症状

客户端日志显示:
```
🌐 [DirectFollow] 发送Run命令: dir=UpLeft grid=(290, 623)
📍 UserLocation received: (291, 623)  ← 服务器返回固定位置
客户端移动到: (288, 621) → (287, 618) → (285, 617)
📍 UserLocation received: (291, 623)  ← 依然是固定位置
偏差增长: x=1 → x=3 → x=6 → x=10
```

## 服务器代码分析

### 1. 命令接收 (MirConnection.cs:1011-1027)

```csharp
private void Walk(C.Walk p)
{
    if (Stage != GameStage.Game) return;
    
    if (Player.ActionTime > Envir.Time)
        _retryList.Enqueue(p);  // 如果太快,加入重试队列
    else
        Player.Walk(p.Direction);
}

private void Run(C.Run p)
{
    if (Stage != GameStage.Game) return;
    
    if (Player.ActionTime > Envir.Time)
        _retryList.Enqueue(p);  // 如果太快,加入重试队列
    else
        Player.Run(p.Direction);
}
```

### 2. 移动验证 (HumanObject.cs:2403-2493)

**Walk方法检查:**
- `!CanMove || !CanWalk` → 返回`false`并发送当前位置
- `!CurrentMap.ValidPoint(location)` → 返回`false`
- `!CurrentMap.CheckDoorOpen(location)` → 返回`false`
- 碰撞检测:格子内有阻挡物 → 返回`false`
- `CheckMovement(location)` 返回`true` → 返回`false`

**成功时:**
```csharp
ActionTime = Envir.Time + GetDelayTime(MoveDelay);  // MoveDelay=600ms
Enqueue(new S.UserLocation { Direction = Direction, Location = CurrentLocation });
Broadcast(new S.ObjectWalk { ObjectID = ObjectID, Direction = Direction, Location = CurrentLocation });
```

### 3. 可能的原因

#### ❓ ActionTime检查失败
```
Player.ActionTime = 上次移动时间 + 600ms
当前时间: Envir.Time
如果 ActionTime > Envir.Time → 命令进入_retryList
```

客户端每次跨格子(48×32像素)发送命令:
- Run速度: 2.5 pixels/frame × 60fps = 150 px/s
- 跨一格时间: 48px ÷ 150px/s = **320ms**

**320ms < 600ms** → 第二个命令会被放入重试队列!

#### ❓ CanMove/CanWalk 为 false
可能原因:
- 玩家处于某种状态(眩晕、冰冻、死亡)
- HasBuff阻止移动

#### ❓ CheckMovement 返回 true
检查服务器端的位置差异验证

#### ❓ 碰撞检测失败
服务器认为目标格子有障碍物

## 解决方案

### 方案1: 调整客户端发送频率

将DirectFollow模式的发送间隔与服务器MoveDelay匹配:

```rust
// player_system.rs line 420
if elapsed >= player.move_delay {
    // 发送命令
    player.last_move_time = now;
    // ⚠️ DirectFollow不设置waiting_server_confirm
}
```

**当前问题:** 客户端每320ms发送,服务器每600ms处理 → 冲突

**解决:** 增加客户端节流,确保发送间隔≥600ms

### 方案2: 使用服务器端的预测位置

客户端不发送频繁的移动命令,而是:
1. 发送一个目标坐标
2. 服务器端处理路径规划
3. 客户端只做平滑插值

### 方案3: 检查服务器端日志

需要添加服务器端日志输出,查看:
```csharp
// MirConnection.cs
private void Walk(C.Walk p)
{
    if (Player.ActionTime > Envir.Time) {
        Logger.Debug($"[Move] Walk命令太快! ActionTime={Player.ActionTime}, Now={Envir.Time}, Delay={Player.ActionTime - Envir.Time}ms");
        _retryList.Enqueue(p);
    } else {
        Logger.Debug($"[Move] 执行Walk: direction={p.Direction}, from={Player.CurrentLocation}");
        Player.Walk(p.Direction);
    }
}
```

### 方案4: AutoPathfinding 不发送频繁命令

寻路模式应该只在到达格子中心时发送一次命令:
```rust
// ✅ 已实现
if at_cell_center && !waiting_server_confirm {
    send_command();
    waiting_server_confirm = true; // ⚠️ 等待服务器确认
}
```

## 下一步行动

1. ✅ 客户端将日志重定向到文件
2. ⬜ 服务器端添加详细日志(Walk/Run命令处理)
3. ⬜ 调整DirectFollow发送频率为800ms(留余量)
4. ⬜ 测试验证

