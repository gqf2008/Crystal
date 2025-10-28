// ============================================================================
// Layer 1: 输入/网络收集层 - Mock网络系统
// ============================================================================
//
// ## 设计意图 (Design Intent)
//
// ### 目标 (Purpose)
// 在地图浏览器 (map_viewer_ecs) 中模拟真实的客户端-服务器网络交互,
// 用于测试和验证客户端预测 + 服务器校正 (Client Prediction + Server Reconciliation) 架构。
//
// ### 为什么需要Mock系统? (Why Mock?)
//
// 1. **独立测试** - 无需启动真实服务器即可测试完整的网络同步流程
// 2. **调试预测机制** - 模拟网络延迟,观察客户端预测是否流畅
// 3. **测试校正逻辑** - 故意返回不同的服务器状态,验证回滚和重播是否正确
// 4. **网络条件模拟** - 测试丢包、延迟抖动等真实网络问题
//
// ### 系统在ECS架构中的位置 (Position in ECS Architecture)
//
// ```
// ┌─────────────────────────────────────────────────────────────┐
// │  Layer 1: Input/Network Collecting (输入/网络收集层)        │
// ├─────────────────────────────────────────────────────────────┤
// │  • MouseInputSystem          - 收集鼠标输入                  │
// │  • KeyboardInputSystem        - 收集键盘输入                 │
// │  • MockNetworkSystem          - 模拟服务器响应 (本系统)     │
// │  • ClientNetworkSystem        - 真实网络通信 (游戏中使用)   │
// └─────────────────────────────────────────────────────────────┘
//          ↓ 写入 PlayerInput / ServerState
// ┌─────────────────────────────────────────────────────────────┐
// │  Layer 2: Logic (核心逻辑层)                                │
// ├─────────────────────────────────────────────────────────────┤
// │  • LocalPredictionSystem      - 客户端预测 (读取输入)       │
// │  • ReconciliationSystem       - 服务器校正 (读取服务器状态) │
// │  • MovementSystem             - 物理移动                     │
// └─────────────────────────────────────────────────────────────┘
// ```
//
// ### 工作原理 (How It Works)
//
// #### 真实网络流程 (Real Network Flow):
// ```
// 1. 玩家点击 → PlayerInput
// 2. ClientNetworkSystem 发送移动命令到服务器
// 3. LocalPredictionSystem 立即预测移动 (不等服务器)
// 4. 服务器验证并返回权威位置 → ServerState
// 5. ReconciliationSystem 比较预测和服务器状态
// 6. 如果不一致,回滚并重播未确认的操作
// ```
//
// #### Mock流程 (Mock Flow):
// ```
// 1. 玩家点击 → PlayerInput
// 2. MockNetworkSystem 记录"发送"的命令 (不真的发包)
// 3. LocalPredictionSystem 立即预测移动 (和真实流程一样)
// 4. 延迟 N 毫秒后,MockNetworkSystem 模拟服务器响应
// 5. 写入 ServerState (伪造的权威位置)
// 6. ReconciliationSystem 比较预测和"服务器"状态
// 7. 测试回滚和重播机制
// ```
//
// ### 测试场景 (Test Scenarios)
//
// #### 场景1: 完美网络 (Perfect Network)
// - 延迟: 0ms
// - 丢包率: 0%
// - 期望: 客户端预测和服务器状态完全一致,无校正
//
// #### 场景2: 正常网络 (Normal Network)
// - 延迟: 50-100ms
// - 丢包率: 0%
// - 期望: 客户端流畅移动,服务器确认后无回滚
//
// #### 场景3: 高延迟 (High Latency)
// - 延迟: 200-500ms
// - 丢包率: 0%
// - 期望: 客户端依然流畅,但服务器确认会晚很多
//
// #### 场景4: 不一致校正 (Misprediction)
// - 延迟: 100ms
// - 服务器故意返回不同位置 (模拟碰撞检测)
// - 期望: 客户端回滚到服务器位置,重新播放
//
// #### 场景5: 丢包重发 (Packet Loss)
// - 延迟: 50ms
// - 丢包率: 10%
// - 期望: 客户端重发丢失的命令,最终同步
//
// ### 可配置参数 (Configurable Parameters)
//
// ```rust
// pub struct MockNetworkConfig {
//     // 基础延迟 (Base latency)
//     pub latency_ms: u64,                  // 例如: 50ms
//     
//     // 延迟抖动 (Latency jitter)
//     pub jitter_ms: u64,                   // 例如: ±20ms
//     
//     // 丢包率 (Packet loss rate)
//     pub packet_loss_rate: f32,            // 例如: 0.05 (5%)
//     
//     // 是否启用预测不一致测试
//     pub force_misprediction: bool,        // 故意返回错误位置
//     pub misprediction_offset: (f32, f32), // 偏移量 (x, y)
//     
//     // 服务器Tick率 (Server tick rate)
//     pub server_tick_rate: u32,            // 例如: 20 TPS (每50ms一次更新)
// }
// ```
//
// ### 数据结构 (Data Structures)
//
// #### MockCommand (待发送的命令)
// ```rust
// struct MockCommand {
//     sequence: u32,              // 命令序列号 (用于确认和重发)
//     move_to: (f32, f32),        // 目标位置 (世界坐标)
//     is_running: bool,           // 是否奔跑
//     timestamp: Instant,         // 发送时间 (用于计算延迟)
//     retry_count: u8,            // 重试次数 (模拟丢包重发)
// }
// ```
//
// #### MockServerState (模拟的服务器状态)
// ```rust
// struct MockServerState {
//     position: Position,         // 服务器确认的位置
//     velocity: (f32, f32),       // 服务器确认的速度
//     sequence: u32,              // 确认的命令序列号
//     timestamp: Instant,         // 服务器时间戳
// }
// ```
//
// ### 实现细节 (Implementation Details)
//
// #### update() 方法流程:
// ```
// 1. 读取 PlayerInput (玩家的新操作)
// 2. 如果有新操作:
//    a. 创建 MockCommand
//    b. 随机决定是否"丢包" (根据 packet_loss_rate)
//    c. 如果不丢包,加入 pending_commands 队列
//    d. sequence++
//
// 3. 处理 pending_commands 队列:
//    a. 遍历所有待处理的命令
//    b. 检查是否已经过了模拟的延迟时间
//    c. 如果延迟已过:
//       - 模拟服务器寻路和碰撞检测
//       - 决定最终的服务器位置 (可能和客户端预测不同)
//       - 写入 ServerState
//       - 从队列移除
//
// 4. 模拟丢包重发:
//    a. 如果命令在队列中超过 timeout (例如 500ms)
//    b. 且 retry_count < max_retries
//    c. 重新加入队列 (模拟重发)
// ```
//
// ### 与真实系统的对比 (Comparison with Real System)
//
// | 特性                | MockNetworkSystem        | ClientNetworkSystem     |
// |---------------------|--------------------------|-------------------------|
// | 网络通信            | ❌ 无真实网络            | ✅ TCP/UDP socket       |
// | 延迟模拟            | ✅ 可配置                | ❌ 取决于真实网络       |
// | 丢包模拟            | ✅ 可配置                | ❌ 取决于真实网络       |
// | 服务器逻辑          | ✅ 简化模拟              | ❌ 真实服务器执行       |
// | 测试友好            | ✅ 完全可控              | ❌ 需要真实服务器       |
// | 用于生产环境        | ❌ 仅用于测试            | ✅ 游戏中使用           |
//
// ### 使用方法 (Usage)
//
// ```rust
// // 在 map_viewer_ecs.rs 中:
// let mock_network = MockNetworkSystem::new(MockNetworkConfig {
//     latency_ms: 50,
//     jitter_ms: 20,
//     packet_loss_rate: 0.05,
//     force_misprediction: false,
//     misprediction_offset: (0.0, 0.0),
//     server_tick_rate: 20,
// });
//
// // 在 update() 中:
// mock_network.update(&mut world, delta_time);
// ```
//
// ### 调试输出 (Debug Output)
//
// 启用详细日志可以观察:
// - 每个命令的发送和确认
// - 模拟的延迟和丢包
// - 预测不一致的情况
// - 回滚和重播的触发
//
// 例如:
// ```
// [MockNetwork] 发送命令 #42: move_to=(1234.5, 5678.9), delay=73ms
// [MockNetwork] 丢包: 命令 #43 (loss_rate=5%)
// [MockNetwork] 确认命令 #42: server_pos=(1234.2, 5678.7) [预测差异: 0.5px]
// [MockNetwork] 重发命令 #43 (retry 1/3)
// ```
//
// ============================================================================

use hecs::World;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use rand::Rng;

use crate::ecs::components::{
    Position,
    LocalPlayer,
    input::PlayerInput,
    prediction::Prediction,
};
use crate::ecs::ServerState; // 通过公共重导出引入
use crate::ecs::coordinates::Coordinates;
use crate::ecs::components::map::MapData;
use crate::algorithms::Pathfinding;

// ============================================================================
// 配置结构 (Configuration)
// ============================================================================

/// Mock网络系统配置
#[derive(Debug, Clone)]
pub struct MockNetworkConfig {
    /// 基础延迟 (毫秒)
    pub latency_ms: u64,
    
    /// 延迟抖动 (毫秒, ±jitter_ms)
    pub jitter_ms: u64,
    
    /// 丢包率 (0.0 - 1.0)
    pub packet_loss_rate: f32,
    
    /// 是否强制预测不一致 (用于测试校正机制)
    pub force_misprediction: bool,
    
    /// 预测不一致的偏移量 (世界坐标)
    pub misprediction_offset: (f32, f32),
    
    /// 服务器Tick率 (每秒更新次数)
    pub server_tick_rate: u32,
}

impl Default for MockNetworkConfig {
    fn default() -> Self {
        Self {
            latency_ms: 50,          // 50ms延迟 (正常网络)
            jitter_ms: 20,           // ±20ms抖动
            packet_loss_rate: 0.0,   // 无丢包
            force_misprediction: false,
            misprediction_offset: (0.0, 0.0),
            server_tick_rate: 20,    // 20 TPS (每50ms一次)
        }
    }
}

// ============================================================================
// 内部数据结构 (Internal Data Structures)
// ============================================================================

/// 待处理的Mock命令
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct MockCommand {
    /// 命令序列号
    sequence: u32,
    
    /// 目标位置 (世界坐标)
    move_to: (f32, f32),
    
    /// 是否奔跑
    is_running: bool,
    
    /// 发送时间
    timestamp: Instant,
    
    /// 重试次数
    retry_count: u8,
    
    /// 应该在何时处理 (发送时间 + 延迟)
    process_at: Instant,
}

/// 模拟的服务器状态
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct MockServerState {
    /// 服务器确认的位置
    position: Position,
    
    /// 确认的命令序列号
    sequence: u32,
    
    /// 服务器时间戳
    timestamp: Instant,
}

// ============================================================================
// Mock网络系统 (Mock Network System)
// ============================================================================

pub struct MockNetworkSystem {
    /// 配置
    config: MockNetworkConfig,
    
    /// 待处理的命令队列
    pending_commands: VecDeque<MockCommand>,
    
    /// 当前命令序列号
    current_sequence: u32,
    
    /// 随机数生成器 (用于延迟抖动和丢包)
    rng: rand::rngs::ThreadRng,
    
    /// 上次服务器更新时间 (用于模拟服务器Tick)
    last_server_tick: Instant,
    
    /// 当前模拟的服务器状态
    server_state: Option<MockServerState>,
}

impl MockNetworkSystem {
    /// 创建新的Mock网络系统
    pub fn new(config: MockNetworkConfig) -> Self {
        Self {
            config,
            pending_commands: VecDeque::new(),
            current_sequence: 0,
            rng: rand::rng(), // 使用新API
            last_server_tick: Instant::now(),
            server_state: None,
        }
    }
    
    /// 更新Mock网络系统
    /// 
    /// 执行顺序: 在 InputCollectingSystem 之后, LocalPredictionSystem 之前
    pub fn update(&mut self, world: &mut World, map_data: &MapData, _dt: f32) {
        // 1️⃣ 收集玩家输入并创建Mock命令
        self.collect_player_input(world);
        
        // 2️⃣ 处理待确认的命令 (模拟服务器响应)
        self.process_pending_commands(world, map_data);
        
        // 3️⃣ 模拟服务器Tick (定期广播状态)
        self.simulate_server_tick(world);
    }
    
    /// 收集玩家输入并创建Mock命令
    fn collect_player_input(&mut self, world: &mut World) {
        for (_entity, (player_input, prediction)) in world
            .query_mut::<(&PlayerInput, Option<&Prediction>)>()
            .with::<&LocalPlayer>()
        {
            // 检查是否有新的移动输入
            if let Some((target_x, target_y)) = player_input.move_to {
                // 检查是否需要发送新命令 (避免重复发送相同目标)
                let need_new_command = if let Some(last_cmd) = self.pending_commands.back() {
                    // 如果目标位置变化超过1个格子,发送新命令
                    let distance = ((target_x - last_cmd.move_to.0).powi(2) 
                                  + (target_y - last_cmd.move_to.1).powi(2)).sqrt();
                    distance > 48.0  // 一个格子的宽度
                } else {
                    true  // 队列为空,发送第一个命令
                };
                
                if need_new_command {
                    self.send_command(target_x, target_y, player_input.is_running, prediction);
                }
            }
        }
    }
    
    /// 发送Mock命令 (模拟网络发包)
    fn send_command(
        &mut self,
        target_x: f32,
        target_y: f32,
        is_running: bool,
        _prediction: Option<&Prediction>,
    ) {
        // 模拟丢包
        if self.rng.random_range(0.0..1.0) < self.config.packet_loss_rate {
            println!("[MockNetwork] 🔴 丢包: 命令 #{} (loss_rate={:.1}%)",
                self.current_sequence, self.config.packet_loss_rate * 100.0);
            return;
        }
        
        // 计算延迟 (基础延迟 + 抖动)
        let jitter = self.rng.random_range(0..=self.config.jitter_ms * 2) as i64 - self.config.jitter_ms as i64;
        let delay_ms = (self.config.latency_ms as i64 + jitter).max(0) as u64;
        let process_at = Instant::now() + Duration::from_millis(delay_ms);
        
        // 创建Mock命令
        let command = MockCommand {
            sequence: self.current_sequence,
            move_to: (target_x, target_y),
            is_running,
            timestamp: Instant::now(),
            retry_count: 0,
            process_at,
        };
        
        println!("[MockNetwork] 📤 发送命令 #{}: move_to=({:.1}, {:.1}), running={}, delay={}ms",
            command.sequence, target_x, target_y, is_running, delay_ms);
        
        // 加入队列
        self.pending_commands.push_back(command);
        self.current_sequence += 1;
    }
    
    /// 处理待确认的命令 (模拟服务器响应)
    fn process_pending_commands(&mut self, world: &mut World, map_data: &MapData) {
        let now = Instant::now();
        let mut commands_to_process = Vec::new();
        
        // 收集需要处理的命令
        while let Some(command) = self.pending_commands.front() {
            if now >= command.process_at {
                commands_to_process.push(self.pending_commands.pop_front().unwrap());
            } else {
                break; // 队列已排序,后面的命令都还没到时间
            }
        }
        
        // 处理所有到期的命令
        for command in commands_to_process {
            self.simulate_server_process(world, map_data, &command);
        }
    }
    
    /// 模拟服务器处理命令
    fn simulate_server_process(&mut self, world: &mut World, map_data: &MapData, command: &MockCommand) {
        // 首先查找LocalPlayer实体和当前位置
        let local_player_query: Vec<_> = world
            .query_mut::<&Position>()
            .with::<&LocalPlayer>()
            .into_iter()
            .map(|(entity, position)| (entity, position.clone()))
            .collect();
        
        if local_player_query.is_empty() {
            return;
        }
        
        let (entity, current_position) = local_player_query[0];
        
        // 模拟服务器寻路 (简化版,使用客户端相同的寻路算法)
        let (current_gx, current_gy) = Coordinates::world_to_grid(current_position.x, current_position.y);
        let (target_gx, target_gy) = Coordinates::world_to_grid(command.move_to.0, command.move_to.1);
        
        // 服务器验证路径是否合法
        let server_position = if let Some(_path) = Pathfinding::find_path(map_data, (current_gx, current_gy), (target_gx, target_gy)) {
            // 路径合法,确认目标位置
            let mut final_x = command.move_to.0;
            let mut final_y = command.move_to.1;
            
            // 如果启用了强制预测不一致,添加偏移
            if self.config.force_misprediction {
                final_x += self.config.misprediction_offset.0;
                final_y += self.config.misprediction_offset.1;
                println!("[MockNetwork] ⚠️ 强制预测不一致: 偏移 ({:.1}, {:.1})",
                    self.config.misprediction_offset.0, self.config.misprediction_offset.1);
            }
            
            Position { x: final_x, y: final_y }
        } else {
            // 路径不合法,保持当前位置 (服务器拒绝移动)
            println!("[MockNetwork] ❌ 服务器拒绝移动: 路径不合法");
            current_position
        };
        
        // 计算预测误差
        let prediction_error = ((server_position.x - current_position.x).powi(2)
                              + (server_position.y - current_position.y).powi(2)).sqrt();
        
        println!("[MockNetwork] ✅ 确认命令 #{}: server_pos=({:.1}, {:.1}), 预测误差: {:.2}px",
            command.sequence, server_position.x, server_position.y, prediction_error);
        
        // 检查是否已有ServerStateComponent
        let has_server_state = world.get::<&ServerState>(entity).is_ok();
        
        if has_server_state {
            // 更新现有组件
            if let Ok(mut server_state) = world.get::<&mut ServerState>(entity) {
                server_state.position = server_position;
                server_state.sequence_number = command.sequence;
                server_state.last_update_time = Instant::now();
            }
        } else {
            // 创建新组件
            world.insert_one(entity, ServerState {
                position: server_position,
                direction: 0,
                sequence_number: command.sequence,
                last_update_time: Instant::now(),
            }).ok();
        }
        
        // 更新模拟的服务器状态
        self.server_state = Some(MockServerState {
            position: server_position,
            sequence: command.sequence,
            timestamp: Instant::now(),
        });
    }
    
    /// 模拟服务器Tick (定期广播状态)
    fn simulate_server_tick(&mut self, _world: &mut World) {
        let tick_interval = Duration::from_millis(1000 / self.config.server_tick_rate as u64);
        
        if self.last_server_tick.elapsed() >= tick_interval {
            // TODO: 在真实游戏中,服务器会定期广播所有对象的状态
            // 这里可以添加更多的模拟逻辑,例如:
            // - 广播其他玩家的位置
            // - 广播怪物的位置
            // - 广播掉落物品
            
            self.last_server_tick = Instant::now();
        }
    }
    
    /// 获取配置 (用于调试)
    pub fn config(&self) -> &MockNetworkConfig {
        &self.config
    }
    
    /// 设置配置 (运行时调整)
    pub fn set_config(&mut self, config: MockNetworkConfig) {
        self.config = config;
    }
    
    /// 获取待处理的命令数量
    pub fn pending_commands_count(&self) -> usize {
        self.pending_commands.len()
    }
    
    /// 重置系统状态
    pub fn reset(&mut self) {
        self.pending_commands.clear();
        self.current_sequence = 0;
        self.server_state = None;
        self.last_server_tick = Instant::now();
    }
}

// ============================================================================
// 测试辅助函数 (Test Helpers)
// ============================================================================

impl MockNetworkSystem {
    /// 创建用于完美网络测试的配置
    pub fn perfect_network() -> Self {
        Self::new(MockNetworkConfig {
            latency_ms: 0,
            jitter_ms: 0,
            packet_loss_rate: 0.0,
            force_misprediction: false,
            misprediction_offset: (0.0, 0.0),
            server_tick_rate: 60,
        })
    }
    
    /// 创建用于高延迟测试的配置
    pub fn high_latency() -> Self {
        Self::new(MockNetworkConfig {
            latency_ms: 300,
            jitter_ms: 50,
            packet_loss_rate: 0.0,
            force_misprediction: false,
            misprediction_offset: (0.0, 0.0),
            server_tick_rate: 20,
        })
    }
    
    /// 创建用于丢包测试的配置
    pub fn packet_loss() -> Self {
        Self::new(MockNetworkConfig {
            latency_ms: 50,
            jitter_ms: 20,
            packet_loss_rate: 0.1,  // 10% 丢包
            force_misprediction: false,
            misprediction_offset: (0.0, 0.0),
            server_tick_rate: 20,
        })
    }
    
    /// 创建用于预测不一致测试的配置
    pub fn misprediction_test() -> Self {
        Self::new(MockNetworkConfig {
            latency_ms: 100,
            jitter_ms: 20,
            packet_loss_rate: 0.0,
            force_misprediction: true,
            misprediction_offset: (48.0, 32.0),  // 偏移一个格子
            server_tick_rate: 20,
        })
    }
}
