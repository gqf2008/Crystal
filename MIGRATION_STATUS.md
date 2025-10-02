# SharedRust Protocol Packets Migration Status

## 执行时间
2025-10-02

## 迁移策略
**选项B**: 从ClientRust迁移protocol_packets代码到SharedRust

## 当前进度

### ✅ 已完成
1. **备份工作** (5分钟)
   - Git commit: "WIP: Phase 1.2 partial implementation (21 packets, 30 errors) - before migration"
   - 创建 server_packets.rs.backup

2. **复制模块** (10分钟)
   - 成功复制 `ClientRust/src/protocol_packets` → `SharedRust/src/protocol_packets`
   - 16个子模块全部复制

3. **导入路径修复** (30分钟)
   - 修改lib.rs添加protocol_packets模块导出
   - 批量替换 `mir2_shared::` → `crate::`
   - 添加CharacterSummary类型别名 = SelectInfo
   - 修复client_data.rs缺少write_bool导入

4. **第一次编译尝试**
   - 警告: 5个重复导出(ClientVersion, Disconnect, KeepAlive, NewAccount, ChangePassword)
   - 错误: 56个编译错误

### ❌ 发现的问题

#### 问题1: 错误类型不兼容 (~30个错误)
**症状**:
```rust
// ClientRust packets使用:
Result<T, String>

// SharedRust期望:
Result<T, SharedError>

// 错误示例:
error[E0277]: `?` couldn't convert the error to `std::string::String`
  --> protocol_packets/packets/magic.rs:49
   |
49 |     let magic = ClientMagic::read_from(&mut cursor)?;
   |                                                     ^ the trait `From<SharedError>` is not implemented for `std::string::String`
```

**原因**: ClientRust的protocol_packets是客户端简化版本,使用String作为错误类型

**影响范围**:
- magic.rs: 2个错误 (ClientMagic::read_from)
- quest.rs: 2个错误 (ClientQuestProgress, ClientQuestInfo)
- npc.rs: 2个错误 (read_dotnet_string)
- group.rs: 2个错误 (read_dotnet_string)
- guild.rs: 4个错误 (read_dotnet_string, GuildRank::read_from)
- item.rs: 2个错误 (UserItem::read_from, ItemInfo::read_from)
- player.rs: ~15个错误 (各种enum转换)

#### 问题2: TryFromPrimitive转换 (~18个错误)
**症状**:
```rust
error[E0277]: `?` couldn't convert the error to `SharedError`
  --> server_packets.rs:863
   |
863 |         let class = MirClass::try_from(reader.read_u8()?)?;
   |                                                           ^ the trait `From<TryFromPrimitiveError<MirClass>>` is not implemented for `SharedError`
```

**原因**: SharedRust的SharedError没有实现From<TryFromPrimitiveError<T>>

**影响类型**:
- MirClass (4处)
- MirGender (4处)
- MirDirection (6处)
- SpellEffect, BuffType, LightSetting等 (4处)

#### 问题3: UserItem/ItemInfo参数不匹配 (~5个错误)
**症状**:
```rust
error[E0061]: this function takes 3 arguments but 1 argument was supplied
  --> protocol_packets/packets/item.rs:178
   |
178 |     let item = UserItem::read_from(&mut cursor)?;
   |                ^^^^^^^^^^^^^^^^^^^--------------- expected 3 arguments
```

**原因**: 
- ClientRust: `UserItem::read_from(reader)` (无版本参数)
- SharedRust: `UserItem::read_from(reader, version, custom_version)` (需要版本参数)

**影响文件**:
- item.rs: 2处
- guild.rs: 1处  
- player.rs: ~2处

#### 问题4: GuildRank参数不匹配 (1个错误)
```rust
error[E0061]: this function takes 2 arguments but 1 argument was supplied
  --> protocol_packets/packets/guild.rs:89
   |
89 |     ranks.push(GuildRank::read_from(&mut cursor)?);
   |                ^^^^^^^^^^^^^^^^^^^^--------------- expected 2 arguments
```

### 📊 错误统计
- **总计**: 56个编译错误
- **错误类型转换**: ~30个 (54%)
- **TryFromPrimitive**: ~18个 (32%)
- **函数参数不匹配**: ~8个 (14%)

## 问题根源分析

### ClientRust vs SharedRust 架构差异

| 特性 | ClientRust | SharedRust |
|-----|------------|------------|
| 错误类型 | `String` (简单) | `SharedError` (结构化) |
| UserItem | 无版本参数 | 需要version/custom_version |
| 数据结构 | 客户端专用简化版 | 服务器/客户端共享完整版 |
| 解析逻辑 | 客户端消费视角 | 网络协议完整实现 |

**核心问题**: ClientRust的protocol_packets是为**客户端使用**设计的,不是为**库共享**设计的。它:
- 使用简化的错误处理(String)
- 省略了服务器端需要的参数(如version)
- 针对客户端场景优化,缺少双向序列化支持

## 修复方案

### 方案A: 适配ClientRust代码 (推荐) ⭐
**工作量**: 3-4小时
**策略**: 修改protocol_packets使其适应SharedRust的API

**具体步骤**:
1. **修复错误类型** (1小时)
   - 将所有parse函数的返回类型从`Result<T, String>`改为`SharedResult<T>`
   - 将`.map_err(|e| format!("..."))`改为`.map_err(|e| SharedError::IoError(e))`等

2. **修复TryFromPrimitive** (1小时)  
   - 在stats.rs的SharedError中添加From实现:
     ```rust
     impl<T: num_enum::TryFromPrimitive> From<num_enum::TryFromPrimitiveError<T>> for SharedError {
         fn from(e: num_enum::TryFromPrimitiveError<T>) -> Self {
             SharedError::UnknownEnum { /* ... */ }
         }
     }
     ```

3. **修复UserItem参数** (1小时)
   - 选项1: 为protocol_packets创建简化版UserItem wrapper
   - 选项2: 修改ClientRust使用带版本的UserItem (影响ClientRust)
   - **推荐**: 在protocol_packets中使用默认version=0

4. **测试编译** (30分钟)
   - 修复剩余小问题
   - 确保SharedRust编译通过

### 方案B: 保留两套实现 (不推荐)
- 保留ClientRust/protocol_packets用于客户端
- 保留SharedRust/server_packets用于共享
- **问题**: 代码重复,维护困难

### 方案C: 完全重写 (最差)
- 放弃ClientRust代码
- 继续Phase 1.2的重写工作
- **工作量**: 90-114小时

## 下一步行动 (方案A)

### 立即执行
1. 在SharedRust/src/stats.rs添加TryFromPrimitiveError转换
2. 修改protocol_packets/packets中所有parse函数返回类型
3. 处理UserItem参数问题

### 预计时间
- 修复: 3-4小时
- 测试: 1小时
- 更新ClientRust: 1-2小时
- **总计**: 5-7小时

## 收益评估

### 如果继续方案A
- ✅ 节省80+ 小时(vs 完全重写)
- ✅ 获得100+测试过的packet定义
- ✅ 模块化架构
- ⚠️ 需要3-4小时适配工作

### 如果放弃回到重写
- ❌ 浪费已投入的1.5小时
- ❌ 仍需90-114小时完成
- ❌ 会再次遇到UserItem/TryFromPrimitive问题

## 建议
**继续方案A - 适配ClientRust代码**

虽然遇到了56个编译错误,但这些都是系统性问题,可以批量修复:
1. 错误类型 → 添加一个From实现即可解决30个错误
2. TryFromPrimitive → 添加一个泛型From实现即可解决18个错误
3. UserItem → 修改调用方式或创建wrapper即可解决8个错误

相比重写90小时,花3-4小时修复这些问题是非常值得的!

## 状态
🟡 **暂停等待决策** - 需要确认是否继续方案A
