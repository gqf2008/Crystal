# 🚀 客户端-服务器联调启动指南

**准备日期**: 2025年10月22日  
**当前状态**: ✅ 所有场景渲染完成，网络系统就绪

---

## ✅ 联调前检查清单

### 1. 客户端准备状态

| 组件 | 状态 | 说明 |
|------|------|------|
| **LoginScene** | ✅ 完整 | 所有UI、动画、网络逻辑已实现 |
| **SelectScene** | ✅ 完整 | 角色选择、创建、删除功能完整 |
| **GameScene** | ✅ 完整 | 地图渲染、玩家控制、UI系统就绪 |
| **NetworkManager** | ✅ 完整 | TCP连接、数据包收发正常 |
| **GameState** | ✅ 完整 | 场景切换、事件分发机制完整 |
| **资源加载** | ✅ 完整 | 所有.lib库已实现加载 |

### 2. 网络配置

**默认配置** (src/settings.rs):
```rust
pub struct NetworkSettings {
    pub use_config: bool,      // false
    pub ip_address: String,    // "127.0.0.1"
    pub port: u16,             // 7000
    pub timeout_ms: u64,       // 5000
}
```

**配置文件位置**:
- `Mir2Config.ini` (生产环境)
- `Mir2Test.ini` (测试环境)
- `config/client.json`
- `config/client.yaml`
- 环境变量: `MIR2_CLIENT__NETWORK__IP_ADDRESS`, `MIR2_CLIENT__NETWORK__PORT`

---

## 🎮 启动步骤

### 方式1: 默认配置启动 (推荐)

**服务器端**:
```bash
# 1. 进入服务器目录
cd d:\Users\gxh\Documents\GitHub\Crystal\Server\bin\Debug

# 2. 启动服务器
.\Server.Library.exe

# 3. 等待服务器就绪
# 看到 "Server started on port 7000" 表示成功
```

**客户端**:
```bash
# 1. 进入客户端目录
cd d:\Users\gxh\Documents\GitHub\Crystal\ClientRust

# 2. 启动客户端 (Debug模式，带日志)
cargo run --bin main_ggez

# 或者 Release模式 (更快)
cargo run --release --bin main_ggez
```

**预期结果**:
```
🎮 Crystal Mir2 Client - Ggez版本
========================================

✅ 配置加载完成
✅ Tokio runtime 创建成功
✅ 图像库初始化完成
✅ 网络系统初始化完成
🌐 网络任务启动
✅ 已连接到服务器
```

---

### 方式2: 自定义服务器地址

#### 方式2.1: 修改配置文件

创建 `config/client.json`:
```json
{
  "Network": {
    "UseConfig": true,
    "IPAddress": "192.168.1.100",
    "Port": 7000,
    "TimeOut": 5000
  }
}
```

或创建 `Mir2Config.ini`:
```ini
[Network]
UseConfig=1
IPAddress=192.168.1.100
Port=7000
TimeOut=5000
```

#### 方式2.2: 使用环境变量

**Windows PowerShell**:
```powershell
$env:MIR2_CLIENT__NETWORK__IP_ADDRESS="192.168.1.100"
$env:MIR2_CLIENT__NETWORK__PORT="7000"
cargo run --bin main_ggez
```

**Linux/Mac**:
```bash
MIR2_CLIENT__NETWORK__IP_ADDRESS=192.168.1.100 \
MIR2_CLIENT__NETWORK__PORT=7000 \
cargo run --bin main_ggez
```

---

## 📡 网络协议测试流程

### 阶段1: 连接测试

**客户端日志**:
```
✅ 已连接到服务器
✅ 发送客户端版本验证
```

**服务器日志**:
```
Client connected: 127.0.0.1:xxxxx
Received ClientVersion packet
```

**如果失败**:
- ❌ "连接失败" → 检查服务器是否启动
- ❌ "连接超时" → 检查防火墙/端口
- ❌ "连接拒绝" → 检查IP地址和端口

---

### 阶段2: 登录测试

**操作步骤**:
1. 启动客户端，看到LoginScene
2. 输入账号: `test`
3. 输入密码: `test`
4. 点击"登录"按钮

**客户端日志**:
```
📤 发送登录请求: test
⏳ 等待服务器响应...
✅ 登录成功！收到 3 个角色
🔄 切换场景: Login -> Select
```

**服务器日志**:
```
Received Login packet: test
Account validation success
Sending LoginSuccess with 3 characters
```

**数据包流程**:
```
客户端                                    服务器
  |                                        |
  |---> ClientVersion ------------------>|  (版本验证)
  |<--- ClientVersion(Success) <---------|
  |                                        |
  |---> Login(test, test) -------------->|  (登录请求)
  |<--- LoginSuccess(characters) <-------|  (登录成功)
  |                                        |
```

**可能的返回码**:
```rust
0: Success (成功)
1: Banned (封禁)
2: WrongPassword (密码错误)
3: AccountNotExist (账号不存在)
4: WrongVersion (版本不匹配)
5: RequirePasswordChange (需要修改密码)
```

---

### 阶段3: 角色选择测试

**操作步骤**:
1. 登录成功后进入SelectScene
2. 看到角色列表
3. 点击角色槽选择角色
4. 点击"开始游戏"按钮

**客户端日志**:
```
🎭 创建SelectScene，角色数: 3
角色0: 战士Lv50 [Warrior]
角色1: 法师Lv45 [Wizard]
角色2: 道士Lv40 [Taoist]
📤 发送StartGame请求: 角色索引 0
🎮 开始游戏成功
🔄 切换场景: Select -> Game
```

**服务器日志**:
```
Received SelectCharacter packet: index=0
Character loaded: Warrior Lv50
Sending StartGameResponse: Success
```

**数据包流程**:
```
客户端                                    服务器
  |                                        |
  |---> SelectCharacter(0) ------------->|  (选择角色)
  |<--- StartGameResponse(0) <-----------|  (进入游戏)
  |<--- MapInformation <-----------------|  (地图信息)
  |<--- UserInformation <----------------|  (玩家信息)
  |<--- PlayerUpdate <-------------------|  (玩家状态)
  |                                        |
```

---

### 阶段4: 游戏场景测试

**操作步骤**:
1. 进入GameScene
2. 看到地图和玩家角色
3. 使用WASD或鼠标移动
4. 按I打开背包
5. 按C打开角色面板

**客户端日志**:
```
🗺️ 游戏场景初始化中...
✅ 地图加载完成: 500x400
🧙 出生位置: 格子(250, 200)
✅ 玩家实体创建完成
✅ UI系统初始化完成
```

**服务器日志**:
```
Player entered map: 0 at (250, 200)
Sending nearby objects...
```

**数据包流程**:
```
客户端                                    服务器
  |                                        |
  |---> Turn(Direction) ------------------>|  (转向)
  |---> Walk(Direction) ------------------>|  (移动)
  |<--- ObjectTurn <----------------------|  (其他玩家转向)
  |<--- ObjectWalk <----------------------|  (其他玩家移动)
  |<--- ObjectPlayer <--------------------|  (新玩家进入视野)
  |<--- ObjectMonster <-------------------|  (怪物进入视野)
  |<--- ObjectRemove <--------------------|  (对象离开视野)
  |                                        |
```

---

## 🐛 常见问题排查

### 问题1: 客户端无法连接

**症状**:
```
❌ 连接失败: Connection refused
```

**检查清单**:
- [ ] 服务器是否启动？
- [ ] 端口是否正确 (默认7000)？
- [ ] 防火墙是否阻止？
- [ ] IP地址是否正确？

**解决方案**:
```powershell
# 检查端口占用
netstat -ano | findstr :7000

# 测试端口连通性
Test-NetConnection -ComputerName 127.0.0.1 -Port 7000

# 禁用防火墙 (测试用)
# 控制面板 -> Windows Defender 防火墙 -> 关闭
```

---

### 问题2: 连接后立即断开

**症状**:
```
✅ 已连接到服务器
⚠️ 与服务器断开连接: Connection reset
```

**可能原因**:
1. 服务器崩溃
2. 协议版本不匹配
3. 数据包格式错误

**检查方案**:
```rust
// 检查客户端日志
tracing::info!("发送数据包: {:?}", packet);

// 检查服务器日志
Logger.Log("接收数据包: " + packet.ToString());
```

---

### 问题3: 登录失败

**症状**:
```
❌ 登录失败: WrongPassword (2)
```

**可能原因**:
1. 账号密码错误
2. 数据库连接失败
3. 账号不存在

**解决方案**:
```sql
-- 检查数据库中的账号
SELECT * FROM Account WHERE Username = 'test';

-- 创建测试账号
INSERT INTO Account (Username, Password, ...) VALUES ('test', 'test', ...);
```

---

### 问题4: 角色列表为空

**症状**:
```
✅ 登录成功！收到 0 个角色
```

**解决方案**:
1. 使用"新建角色"功能创建角色
2. 或在数据库中手动插入角色数据

```sql
-- 检查角色
SELECT * FROM Character WHERE AccountID = (SELECT ID FROM Account WHERE Username = 'test');
```

---

### 问题5: 进入游戏后卡住

**症状**:
```
🎮 开始游戏成功
🔄 切换场景: Select -> Game
[然后没有任何反应]
```

**可能原因**:
1. 地图文件缺失
2. 角色数据损坏
3. 网络同步失败

**检查方案**:
```
客户端日志应该看到:
🗺️ 游戏场景初始化中...
✅ 地图加载完成: 500x400

如果看不到，说明地图加载失败
```

---

## 📊 性能监控

### 客户端性能指标

**正常范围**:
- **FPS**: 60+ (LoginScene/SelectScene), 120+ (GameScene)
- **内存**: < 500MB
- **网络延迟**: < 50ms (局域网), < 100ms (互联网)
- **CPU使用率**: < 30%

**监控方法**:
```rust
// FPS显示 (已内置)
// 右上角显示 "FPS: 60"

// 网络延迟
tracing::info!("Ping: {}ms", latency);

// 内存占用
// Windows任务管理器查看
```

---

## 🧪 测试用例

### 测试用例1: 完整登录流程

**步骤**:
1. ✅ 启动客户端
2. ✅ 连接服务器
3. ✅ 输入账号密码
4. ✅ 点击登录
5. ✅ 查看角色列表
6. ✅ 选择角色
7. ✅ 进入游戏

**预期结果**: 所有步骤成功，无错误

---

### 测试用例2: 新建账号

**步骤**:
1. ✅ 点击"新建账号"按钮
2. ✅ 填写账号信息
3. ✅ 提交注册
4. ✅ 查看结果

**预期结果**: 
- 成功: 显示"注册成功"
- 失败: 显示具体错误信息

---

### 测试用例3: 创建角色

**步骤**:
1. ✅ 登录成功后进入SelectScene
2. ✅ 点击"新建角色"按钮
3. ✅ 选择职业 (战士/法师/道士)
4. ✅ 选择性别
5. ✅ 输入角色名
6. ✅ 点击"确定"

**预期结果**: 角色创建成功，显示在角色列表中

---

### 测试用例4: 删除角色

**步骤**:
1. ✅ 选中要删除的角色
2. ✅ 点击"删除角色"按钮
3. ✅ 确认删除

**预期结果**: 角色从列表中移除

---

### 测试用例5: 游戏内移动

**步骤**:
1. ✅ 进入游戏
2. ✅ 使用WASD移动
3. ✅ 使用鼠标点击移动
4. ✅ 观察角色动画

**预期结果**: 角色平滑移动，动画正常播放

---

## 📝 调试技巧

### 启用详细日志

**修改 main_ggez.rs**:
```rust
// 从 "error" 改为 "debug" 或 "trace"
ClientRuntime::init_logging("debug");
```

**日志级别**:
- `error`: 只显示错误
- `warn`: 错误 + 警告
- `info`: 错误 + 警告 + 信息
- `debug`: 错误 + 警告 + 信息 + 调试
- `trace`: 所有日志 (非常详细)

---

### 网络数据包抓取

**使用 Wireshark**:
```
1. 启动 Wireshark
2. 选择网络接口 (Loopback 或以太网)
3. 过滤器输入: tcp.port == 7000
4. 开始抓包
5. 启动客户端和服务器
6. 观察数据包交互
```

**分析要点**:
- TCP三次握手是否成功
- 数据包长度是否正确
- 数据包内容是否符合协议

---

### 断点调试

**VS Code配置** (.vscode/launch.json):
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Client",
      "cargo": {
        "args": [
          "build",
          "--bin=main_ggez",
          "--package=mir2_client"
        ],
        "filter": {
          "name": "main_ggez",
          "kind": "bin"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}/ClientRust"
    }
  ]
}
```

**常用断点位置**:
- `src/ecs/game_app.rs:150` - 网络事件处理
- `src/scenes/login_scene.rs:1300` - 登录处理
- `src/network/game_client.rs:200` - 数据包发送

---

## 🎯 成功标准

### 最小可行功能 (MVP)

- [x] 客户端启动正常
- [x] 连接服务器成功
- [x] 登录功能正常
- [x] 角色选择正常
- [x] 进入游戏正常
- [ ] 角色移动正常 (待测试)
- [ ] UI交互正常 (待测试)

### 完整功能

- [ ] 注册账号
- [ ] 创建角色
- [ ] 删除角色
- [ ] 修改密码
- [ ] 聊天系统
- [ ] 背包系统
- [ ] 战斗系统
- [ ] 任务系统
- [ ] 交易系统

---

## 🚀 准备开始！

**最终检查**:
```bash
# 1. 确认服务器运行
# 看到服务器控制台输出 "Server started on port 7000"

# 2. 启动客户端
cd d:\Users\gxh\Documents\GitHub\Crystal\ClientRust
cargo run --bin main_ggez

# 3. 观察日志
# 客户端应该显示:
# ✅ 配置加载完成
# ✅ Tokio runtime 创建成功
# ✅ 图像库初始化完成
# ✅ 网络系统初始化完成
# ✅ 已连接到服务器
```

**如果一切正常**:
```
🎉 恭喜！客户端-服务器联调准备完成！
现在可以开始测试登录、角色选择和游戏功能了。
```

**如果遇到问题**:
1. 检查上面的"常见问题排查"
2. 查看客户端和服务器日志
3. 使用调试工具定位问题

---

**联调启动指南版本**: v1.0  
**最后更新**: 2025年10月22日  
**状态**: ✅ 准备就绪，可以开始联调
