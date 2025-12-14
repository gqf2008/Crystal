# 网络协议对齐清单（C# ↔ Rust macroquad）

本文件用于把 **C# 客户端/服务器实现** 与 **Rust(macroquad) 客户端** 的网络协议逐条对齐，避免“能连但行为不一致”。

## 1. 权威来源（以 C# 为准）

- 报文框架与包分发：`Shared/Packet.cs`
- Client → Server 包体定义：`Shared/ClientPackets.cs`
- Server → Client 包体定义：`Shared/ServerPackets.cs`
- Opcode/PacketId 顺序：`Shared/Enums.cs`（`ClientPacketIds`/`ServerPacketIds`）
- 压缩算法：`Shared/Functions/Functions.cs`（`GZipStream`）

Rust 侧对应实现：

- 报文序列化/反序列化 + gzip：`SharedRust/src/packets/base.rs`
- client 包体：`SharedRust/src/packets/client/*`
- server 包体：`SharedRust/src/packets/server/*`
- macroquad 网络线程：`Client-Macroquad/src/network/client.rs`
- macroquad 协议 → 事件：`Client-Macroquad/src/network/handlers/*`
- ECS 桥接：`Client-Macroquad/src/systems/infra/network_system.rs`

## 2. 线协议（Wire Format）

### 2.1 报文头
C# `Packet.ReceivePacket()` 约定：

- `u16 length`（小端）
- `i16 opcode`（小端）
- `payload`（长度为 `length - 4`）

Rust `SharedRust`/`mir2_shared` 通过 `PacketHeader` + `serialize_packet/deserialize_packet` 复现相同结构。

### 2.2 压缩（Compressed）
C# `Packet.Compressed` 为 `true` 的包：payload 使用 **GZip** 压缩。

- C#: `Functions.CompressBytes/DecompressBytes`（`GZipStream`）
- Rust: `SharedRust/src/packets/base.rs`（`flate2::GzEncoder/GzDecoder`）

示例：`ServerPackets.NPCGoods` 在 C# 端明确 `Compressed => true`。

### 2.3 字符串（.NET BinaryReader/BinaryWriter）
C# `ReadString/Write(string)` 使用 .NET 的 length 前缀（7-bit encoded int）编码。

Rust 侧必须使用 `read_dotnet_string/write_dotnet_string`（`SharedRust/src/binary.rs`）而不是简单的 `u16 len + bytes`。

## 3. macroquad 网络架构（当前状态）

- 网络读写：双线程（read_thread / write_thread），统一使用 `NetworkEvent` 作为消息。
- ECS 侧：`NetworkSystem` 每帧从 `GameContext.net` 拉取 `NetworkEvent`，写入 `EventBus.network_events`。
- 默认未连接：`GameContext.net == None`，`NetworkSystem` 为 no-op，不影响 `test_game_scene`。

## 4. 已对齐/已修复项（关键子集）

### 4.1 连接/心跳
- `ClientPacketIds.ClientVersion` ↔ `NetworkEvent::ClientVersionSend { version_hash }`
  - C#：`int length` + `byte[] VersionHash`
  - Rust：`SharedRust/src/packets/client/connection.rs::ClientVersion` ✅
- `ClientPacketIds.KeepAlive` ↔ `NetworkEvent::KeepAliveSend { time }` ✅
- `ServerPacketIds.Connected/KeepAlive/Disconnect/ClientVersion` ↔ connection handler ✅

### 4.2 账号/角色
- `ClientPacketIds.NewAccount` ↔ `NetworkEvent::NewAccountRequest { .. }` ✅
- `ClientPacketIds.ChangePassword` ↔ `NetworkEvent::ChangePasswordRequest { .. }` ✅
- `ClientPacketIds.Login` ↔ `NetworkEvent::LoginRequest { .. }` ✅
- `ClientPacketIds.NewCharacter/DeleteCharacter/StartGame` ↔ 对应 `NetworkEvent::*Request` ✅

### 4.3 移动
- `ClientPacketIds.Turn/Walk/Run` ↔ `NetworkEvent::TurnRequest/WalkRequest/RunRequest` ✅
- `ServerPacketIds.ObjectWalk/ObjectRun/ObjectTurn/UserLocation` ↔ movement handler（按需扩展）

### 4.4 聊天（本轮修复）
C# `ClientPackets.Chat` 实际格式为：

- `string Message`
- `int count`
- `count * ChatItem`（`ulong UniqueID` + `string Title` + `byte Grid`）

Rust 侧此前只发送 `message`，会导致服务器读包失败。

已修复：

- `SharedRust/src/packets/client/chat.rs::Chat` 增加 `linked_items: Vec<ChatItem>` 并写入 `count + items`
- macroquad `NetworkEvent::ChatRequest` 改为 `message + linked_items`

### 4.5 MapChanged（本轮修复）

C# `ServerPackets.MapChanged` 的 wire format 包含：

- `int MapIndex`
- `string FileName`
- `string Title`
- `u16 MiniMap`
- `u16 BigMap`
- `byte Lights`
- `int Location.X`
- `int Location.Y`
- `byte Direction`
- `byte MapDarkLight`
- `u16 Music`
- `u16 Weather`

已修复 `mir2_shared`（SharedRust）的 `MapChanged` 结构体与读写顺序，避免客户端解析偏移。

## 5. 需要继续核对的重点（建议优先级）

P0（连服/进场链路必需）：
- `ServerPackets.LoginSuccess`/`NewCharacterSuccess`/`StartGame*`/`MapInformation`/`MapChanged`/`UserInformation` 的字段对齐与消费流程
- `UserInformation`：C# 里包含大量角色/背包/技能等信息；macroquad 目前事件里仅保留少量字段（需要补齐或改为携带原始 packet 数据）

P1（基础玩法）：
- Object 同步：`ObjectPlayer/ObjectMonster/ObjectNpc/ObjectItem/ObjectRemove` 等
- 战斗：`Attack/Magic/Struck/ObjectStruck/DamageIndicator` 等

P2（系统功能）：
- 组队/公会/交易/邮件/市场/任务等（macroquad handlers 虽已分组，但事件/字段需逐包核对）

## 6. 建议的落地方式（最安全）

- 对于复杂 server 包（例如 `UserInformation`），优先在 `NetworkEvent` 中携带 **mir2_shared 的 packet struct**（而不是拆成少量字段），再由 ECS system 负责落地到组件/资源。
- 每完成一组包（登录/进场/对象同步/战斗），就用一个最小场景或脚本验证：
  - 能连上
  - 不断线
  - 对应事件能被消费且状态一致

