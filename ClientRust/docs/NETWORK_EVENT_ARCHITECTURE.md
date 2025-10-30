# 网络事件架构

## 架构概述

```
┌─────────────────────────────────────────────────────────────┐
│                      游戏逻辑层 (ECS Systems)                 │
│  MovementSystem | CombatSystem | QuestSystem | ...          │
└───────────────────┬─────────────────────────────────────────┘
                    │ send_network_command()
                    ↓
┌─────────────────────────────────────────────────────────────┐
│                   GlobalEvents (Event Hub)                   │
│  ┌─────────────────────┐    ┌─────────────────────────┐    │
│  │ network_command_tx  │    │ network_incoming: Vec   │    │
│  │ (Channel Sender)    │    │ (Packet Buffer)         │    │
│  └──────────┬──────────┘    └──────────▲──────────────┘    │
└─────────────┼──────────────────────────┼──────────────────┘
              │                          │
              │ (立即发送)               │ (批量写入)
              ↓                          │
┌─────────────────────────────────────────────────────────────┐
│                      网络线程                                │
│  network_command_rx → 序列化 → TCP发送                       │
│  TCP接收 → 反序列化 → push_incoming_packet()                │
└─────────────────────────────────────────────────────────────┘
              ↓                          ↑
              └──────── 网络通信 ─────────┘
```

## 核心设计

### 1. 网络命令发送 (客户端→服务器)

**使用 Channel (立即发送模式)**

```rust
pub struct GlobalEvents {
    /// 网络命令发送通道 (Sender - 游戏系统持有)
    network_command_sender: Sender<NetworkCommand>,
    
    /// 网络命令接收通道 (Receiver - 网络线程持有)
    network_command_receiver: Arc<Mutex<Receiver<NetworkCommand>>>,
}
```

**优势:**
- ✅ **零延迟**: 游戏系统调用 `send_network_command()` 后,网络线程立即接收
- ✅ **异步解耦**: 游戏系统不需要等待网络发送完成
- ✅ **无轮询开销**: 网络线程使用阻塞接收,无需周期性检查
- ✅ **自然背压**: Channel 满时自动阻塞发送方

**使用示例:**
```rust
// MovementSystem: 玩家按下方向键
fn update(&self, world: &mut World) {
    for (_, (player, input, events)) in world.query_mut::<(&Player, &Input, &mut GlobalEvents)>() {
        if input.is_key_pressed(KeyCode::Up) {
            // 立即发送到网络线程
            events.send_network_command(NetworkCommand::Walk {
                direction: MirDirection::Up,
            });
        }
    }
}

// 网络线程: 阻塞接收命令
let command_rx = global_events.get_command_receiver();
loop {
    if let Ok(command) = command_rx.lock().unwrap().recv() {
        // 序列化并发送
        send_to_server(command);
    }
}
```

### 2. 网络包接收 (服务器→客户端)

**使用 Vec (批量缓存模式)**

```rust
pub struct GlobalEvents {
    /// 网络包队列 (由 NetworkSyncSystem 批量写入)
    pub network_incoming: Vec<NetworkPacket>,
}
```

**优势:**
- ✅ **批量处理**: NetworkSyncSystem 可以一次性写入多个包
- ✅ **多系统读取**: PacketProcessingSystem, UISystem 等可以同时读取
- ✅ **帧对齐**: 所有系统在同一帧处理相同的数据包集合
- ✅ **易于调试**: 可以查看当前帧收到的所有包

**使用示例:**
```rust
// NetworkSyncSystem: 从网络线程读取包
fn update(&self, world: &mut World) {
    for (_, events) in world.query_mut::<&mut GlobalEvents>() {
        // 从网络线程的队列中读取
        while let Some(packet) = network_thread_rx.try_recv() {
            events.push_incoming_packet(packet);
        }
    }
}

// PacketProcessingSystem: 处理接收到的包
fn update(&self, world: &mut World) {
    for (_, events) in world.query_mut::<&mut GlobalEvents>() {
        for packet in events.drain_incoming_packets() {
            match packet.packet_type.as_str() {
                "ObjectPlayer" => spawn_player(world, packet),
                "ObjectMonster" => spawn_monster(world, packet),
                _ => {}
            }
        }
    }
}
```

## 数据流向

### 发送流程 (客户端→服务器)

```
1. 用户输入
   ↓
2. InputSystem → GlobalEvents.keyboard_events
   ↓
3. MovementSystem 检测到移动键
   ↓
4. events.send_network_command(Walk { direction })
   ↓ (Channel 立即发送)
5. 网络线程接收: command_rx.recv()
   ↓
6. 序列化: serialize_client_packet(&command)
   ↓
7. TCP 发送: stream.write_all(&bytes)
```

### 接收流程 (服务器→客户端)

```
1. 网络线程: TCP 接收 bytes
   ↓
2. 反序列化: parse_packet_header(&bytes)
   ↓
3. 创建 NetworkPacket { type, data }
   ↓
4. 发送到主线程队列 (mpsc channel)
   ↓
5. NetworkSyncSystem: 
   events.push_incoming_packet(packet)
   ↓
6. PacketProcessingSystem:
   for packet in events.drain_incoming_packets() { ... }
   ↓
7. 根据包类型创建/更新 ECS 实体
```

## 关键方法 API

### 发送命令

```rust
impl GlobalEvents {
    /// 发送网络命令到网络线程 (立即发送)
    pub fn send_network_command(&self, command: NetworkCommand) {
        let _ = self.network_command_sender.send(command);
    }
    
    /// 获取命令接收端 (供网络线程使用)
    pub fn get_command_receiver(&self) -> Arc<Mutex<Receiver<NetworkCommand>>> {
        Arc::clone(&self.network_command_receiver)
    }
}
```

### 接收数据包

```rust
impl GlobalEvents {
    /// 添加接收到的网络包 (由 NetworkSyncSystem 调用)
    pub fn push_incoming_packet(&mut self, packet: NetworkPacket) {
        self.network_incoming.push(packet);
        self.frame_event_count += 1;
    }
    
    /// 消费网络包队列 (PacketProcessingSystem 使用)
    pub fn drain_incoming_packets(&mut self) -> impl Iterator<Item = NetworkPacket> + '_ {
        self.network_incoming.drain(..)
    }
}
```

## 性能特性

### 命令发送 (Channel)
- **延迟**: < 1μs (直接写入 channel)
- **吞吐量**: ~10M ops/s (单线程)
- **内存**: O(1) - channel 容量固定

### 包接收 (Vec)
- **延迟**: 一帧 (~16ms @ 60fps)
- **吞吐量**: 批量处理,无限制
- **内存**: O(n) - n = 每帧包数量 (通常 < 100)

## 对比其他方案

### 方案 A: 两个都用 Vec (原方案)

```rust
pub network_incoming: Vec<NetworkPacket>,
pub network_outgoing: Vec<NetworkCommand>,  // ❌ 需要轮询
```

**问题:**
- ❌ NetworkSyncSystem 需要周期性 `drain_outgoing_commands()`
- ❌ 命令发送延迟 = 1 帧 (~16ms)
- ❌ CPU 浪费在空轮询上

### 方案 B: 两个都用 Channel

```rust
network_command_tx/rx: Channel<NetworkCommand>,
network_packet_tx/rx: Channel<NetworkPacket>,  // ❌ 难以多系统读取
```

**问题:**
- ❌ 只有一个系统可以 `recv()` 包
- ❌ 多个系统需要复制数据或使用 broadcast channel
- ❌ 帧对齐困难 (不同系统可能读到不同的包)

### 方案 C: 混合模式 (当前方案) ✅

```rust
network_command_tx/rx: Channel<NetworkCommand>,  // 立即发送
network_incoming: Vec<NetworkPacket>,            // 批量缓存
```

**优势:**
- ✅ 命令发送零延迟
- ✅ 包处理批量高效
- ✅ 多系统可以读取同一帧的数据
- ✅ 网络线程无需主动推送到 Vec

## 线程安全

### Channel (Send + Sync)
```rust
Sender<NetworkCommand>: Send + Clone         // 可跨线程发送
Arc<Mutex<Receiver<NetworkCommand>>>: Sync   // 可跨线程共享
```

### Vec (非线程安全)
```rust
Vec<NetworkPacket>: !Send + !Sync            // 仅主线程访问
```

**结论**: GlobalEvents 本身不是 `Send`,只能在主线程中被 ECS World 持有。
网络线程通过 Channel 与主线程通信。

## 最佳实践

### ✅ DO:
1. 游戏系统调用 `send_network_command()` 发送命令
2. NetworkSyncSystem 定期调用 `push_incoming_packet()` 写入包
3. PacketProcessingSystem 使用 `drain_incoming_packets()` 消费包
4. EventCleanupSystem 在帧末调用 `clear_frame_events()` 清理

### ❌ DON'T:
1. 不要在游戏系统中直接访问网络线程
2. 不要在多个系统中 `drain_incoming_packets()` (会重复消费)
3. 不要跳过 EventCleanupSystem (会导致事件重放)
4. 不要在 `send_network_command()` 中执行阻塞操作

## 总结

**核心思想: 发送用 Channel (立即),接收用 Vec (批量)**

- **网络命令 (outgoing)**: Channel 实现零延迟发送
- **网络包 (incoming)**: Vec 实现高效批量处理和多系统读取

这种混合架构兼顾了**低延迟发送**和**高效批量处理**,是 ECS 架构下的最佳实践。
