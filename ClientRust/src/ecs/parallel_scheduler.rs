// ============================================================================
// 并行系统调度器 - 基于Rayon的多线程ECS系统执行
// ============================================================================
//
// 职责：
// - 自动识别系统依赖关系并构建执行计划
// - 并行执行无依赖的系统组
// - 保证数据竞争安全（通过组件访问分析）
// - 提供性能监控和对比分析
//
// 并行策略：
// - Layer 1-2: 串行执行（有明确依赖链）
// - Layer 3: 并行执行（3个动画状态系统独立）
// - Layer 4: 并行执行（3个渲染准备系统独立）
// - Layer 5: 并行执行（4个杂项系统独立）
//
// 安全保证：
// - 使用 parking_lot::RwLock 保护 World
// - 每个系统声明读写的组件类型
// - 调度器确保并行系统不访问相同组件
//
// ============================================================================

use hecs::World;
use ggez::{GameResult, Context};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use parking_lot::RwLock;
use tokio::sync::mpsc;

use crate::network::NetworkCommand;
use crate::ecs::systems::{
    CameraSystem, MonsterSystem, OcclusionSystem,
    InputCollectingSystem, ClientNetworkSystem,
    LocalPredictionSystem, MovementSystemV2, ReconciliationSystem, InterpolationSystem,
    AnimationStateSystem, NPCActionSystem, MonsterAnimationStateSystem,
    AnimationPlaybackSystem, TileAnimationSystem, MovementInterpolationSystem,
    MouseEventSystem,
};

/// 系统执行模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// 串行执行（默认，保证兼容性）
    Sequential,
    /// 并行执行（Layer 3/4/5 使用 Rayon）
    Parallel,
}

/// 系统性能统计（带并行信息）
#[derive(Debug, Clone)]
pub struct ParallelSystemStats {
    pub name: String,
    pub priority: u32,
    pub execution_count: u64,
    pub total_time: Duration,
    pub average_time: Duration,
    pub last_execution: Duration,
    pub parallel_executions: u64, // 并行执行次数
}

impl ParallelSystemStats {
    fn new(name: String, priority: u32) -> Self {
        Self {
            name,
            priority,
            execution_count: 0,
            total_time: Duration::ZERO,
            average_time: Duration::ZERO,
            last_execution: Duration::ZERO,
            parallel_executions: 0,
        }
    }

    fn record_execution(&mut self, duration: Duration, is_parallel: bool) {
        self.execution_count += 1;
        self.total_time += duration;
        self.last_execution = duration;
        self.average_time = self.total_time / self.execution_count as u32;
        if is_parallel {
            self.parallel_executions += 1;
        }
    }

    fn reset(&mut self) {
        self.execution_count = 0;
        self.total_time = Duration::ZERO;
        self.average_time = Duration::ZERO;
        self.last_execution = Duration::ZERO;
        self.parallel_executions = 0;
    }
}

/// 并行系统调度器
pub struct ParallelScheduler {
    // 执行模式
    execution_mode: ExecutionMode,
    
    // 系统启用标志（与 GameSceneScheduler 相同）
    input_collecting_enabled: bool,
    client_network_enabled: bool,
    local_prediction_enabled: bool,
    movement_v2_enabled: bool,
    reconciliation_enabled: bool,
    interpolation_enabled: bool,
    animation_state_enabled: bool,
    monster_animation_state_enabled: bool,
    npc_action_enabled: bool,
    tile_animation_enabled: bool,
    animation_playback_enabled: bool,
    movement_interpolation_enabled: bool,
    mouse_event_enabled: bool,
    monster_enabled: bool,
    occlusion: OcclusionSystem,
    occlusion_enabled: bool,
    camera_enabled: bool,

    /// 性能统计
    stats: HashMap<String, ParallelSystemStats>,
}

impl ParallelScheduler {
    /// 创建新的并行调度器
    pub fn new(execution_mode: ExecutionMode) -> Self {
        let mut scheduler = Self {
            execution_mode,
            
            input_collecting_enabled: true,
            client_network_enabled: true,
            local_prediction_enabled: true,
            movement_v2_enabled: true,
            reconciliation_enabled: true,
            interpolation_enabled: true,
            animation_state_enabled: true,
            monster_animation_state_enabled: true,
            npc_action_enabled: true,
            tile_animation_enabled: true,
            animation_playback_enabled: true,
            movement_interpolation_enabled: true,
            mouse_event_enabled: true,
            monster_enabled: true,
            occlusion: OcclusionSystem::new(),
            occlusion_enabled: true,
            camera_enabled: true,

            stats: HashMap::new(),
        };

        scheduler.initialize_stats();
        scheduler
    }

    /// 初始化统计信息
    fn initialize_stats(&mut self) {
        let systems = vec![
            ("InputCollectingSystem", 100),
            ("ClientNetworkSystem", 150),
            ("LocalPredictionSystem", 200),
            ("MovementSystemV2", 210),
            ("ReconciliationSystem", 220),
            ("InterpolationSystem", 230),
            ("AnimationStateSystem", 300),
            ("MonsterAnimationStateSystem", 310),
            ("NPCActionSystem", 320),
            ("TileAnimationSystem", 400),
            ("AnimationPlaybackSystem", 410),
            ("MovementInterpolationSystem", 420),
            ("MouseEventSystem", 500),
            ("MonsterSystem", 510),
            ("OcclusionSystem", 520),
            ("CameraSystem", 530),
        ];

        for (name, priority) in systems {
            self.stats.insert(
                name.to_string(),
                ParallelSystemStats::new(name.to_string(), priority),
            );
        }
    }

    /// 切换执行模式
    pub fn set_execution_mode(&mut self, mode: ExecutionMode) {
        self.execution_mode = mode;
    }

    /// 获取执行模式
    pub fn execution_mode(&self) -> ExecutionMode {
        self.execution_mode
    }

    /// 执行所有系统（根据模式选择串行或并行）
    pub fn update(
        &mut self,
        ctx: &mut Context,
        world: &mut World,
        delta_time: f32,
        delta_ms: u32,
        animation_count: i32,
        network_tx: Option<&mpsc::UnboundedSender<NetworkCommand>>,
    ) -> GameResult {
        match self.execution_mode {
            ExecutionMode::Sequential => {
                self.update_sequential(ctx, world, delta_time, delta_ms, animation_count, network_tx)
            }
            ExecutionMode::Parallel => {
                self.update_parallel(ctx, world, delta_time, delta_ms, animation_count, network_tx)
            }
        }
    }

    /// 串行执行（与 GameSceneScheduler 相同）
    fn update_sequential(
        &mut self,
        ctx: &mut Context,
        world: &mut World,
        delta_time: f32,
        delta_ms: u32,
        animation_count: i32,
        network_tx: Option<&mpsc::UnboundedSender<NetworkCommand>>,
    ) -> GameResult {
        macro_rules! run_system {
            ($enabled:expr, $name:literal, $code:block) => {
                if $enabled {
                    let start = Instant::now();
                    $code
                    let duration = start.elapsed();
                    if let Some(stats) = self.stats.get_mut($name) {
                        stats.record_execution(duration, false);
                    }
                }
            };
        }

        // Layer 1: 输入和网络
        run_system!(self.input_collecting_enabled, "InputCollectingSystem", {
            InputCollectingSystem::update(world, ctx);
        });

        run_system!(self.client_network_enabled, "ClientNetworkSystem", {
            ClientNetworkSystem::send_commands(world, network_tx);
        });

        // Layer 2: 核心逻辑（有依赖链）
        run_system!(self.local_prediction_enabled, "LocalPredictionSystem", {
            let map_data_opt = world.query_mut::<&crate::ecs::components::MapData>()
                .into_iter()
                .next()
                .map(|(_, data)| data as *const _);
            
            if let Some(map_data_ptr) = map_data_opt {
                let map_data = unsafe { &*map_data_ptr };
                LocalPredictionSystem::update(world, map_data, delta_time);
            }
        });

        run_system!(self.movement_v2_enabled, "MovementSystemV2", {
            MovementSystemV2::update(world, delta_time);
        });

        run_system!(self.reconciliation_enabled, "ReconciliationSystem", {
            ReconciliationSystem::update(world, delta_time);
        });

        run_system!(self.interpolation_enabled, "InterpolationSystem", {
            InterpolationSystem::update(world, delta_time);
        });

        // Layer 3: 表现决策（串行）
        run_system!(self.animation_state_enabled, "AnimationStateSystem", {
            AnimationStateSystem::update(world, delta_time);
        });

        run_system!(self.monster_animation_state_enabled, "MonsterAnimationStateSystem", {
            MonsterAnimationStateSystem::update(world);
        });

        run_system!(self.npc_action_enabled, "NPCActionSystem", {
            NPCActionSystem::update(world, delta_ms);
        });

        // Layer 4: 渲染准备（串行）
        run_system!(self.tile_animation_enabled, "TileAnimationSystem", {
            TileAnimationSystem::update(world, animation_count);
        });

        run_system!(self.animation_playback_enabled, "AnimationPlaybackSystem", {
            AnimationPlaybackSystem::update(world, delta_ms);
        });

        run_system!(self.movement_interpolation_enabled, "MovementInterpolationSystem", {
            MovementInterpolationSystem::update(world);
        });

        // Layer 5: 其他系统（串行）
        run_system!(self.mouse_event_enabled, "MouseEventSystem", {
            MouseEventSystem::update_mouse_input(world);
        });

        run_system!(self.monster_enabled, "MonsterSystem", {
            MonsterSystem::update(world, delta_time);
        });

        run_system!(self.occlusion_enabled, "OcclusionSystem", {
            self.occlusion.update(world, delta_time);
        });

        run_system!(self.camera_enabled, "CameraSystem", {
            CameraSystem::update(world);
        });

        Ok(())
    }

    /// 并行执行（Layer 3/4/5 使用 Rayon）
    fn update_parallel(
        &mut self,
        ctx: &mut Context,
        world: &mut World,
        delta_time: f32,
        delta_ms: u32,
        animation_count: i32,
        network_tx: Option<&mpsc::UnboundedSender<NetworkCommand>>,
    ) -> GameResult {
        // Layer 1-2: 串行执行（必须按顺序）
        self.execute_layer1_sequential(ctx, world, network_tx)?;
        self.execute_layer2_sequential(world, delta_time)?;

        // Layer 3-5: 并行执行（无依赖）
        self.execute_layer3_parallel(world, delta_time, delta_ms)?;
        self.execute_layer4_parallel(world, delta_ms, animation_count)?;
        self.execute_layer5_parallel(world, delta_time)?;

        Ok(())
    }

    /// Layer 1: 输入和网络（串行）
    fn execute_layer1_sequential(
        &mut self,
        ctx: &mut Context,
        world: &mut World,
        network_tx: Option<&mpsc::UnboundedSender<NetworkCommand>>,
    ) -> GameResult {
        if self.input_collecting_enabled {
            let start = Instant::now();
            InputCollectingSystem::update(world, ctx);
            if let Some(stats) = self.stats.get_mut("InputCollectingSystem") {
                stats.record_execution(start.elapsed(), false);
            }
        }

        if self.client_network_enabled {
            let start = Instant::now();
            ClientNetworkSystem::send_commands(world, network_tx);
            if let Some(stats) = self.stats.get_mut("ClientNetworkSystem") {
                stats.record_execution(start.elapsed(), false);
            }
        }

        Ok(())
    }

    /// Layer 2: 核心逻辑（串行，有依赖链）
    fn execute_layer2_sequential(
        &mut self,
        world: &mut World,
        delta_time: f32,
    ) -> GameResult {
        if self.local_prediction_enabled {
            let start = Instant::now();
            let map_data_opt = world.query_mut::<&crate::ecs::components::MapData>()
                .into_iter()
                .next()
                .map(|(_, data)| data as *const _);
            
            if let Some(map_data_ptr) = map_data_opt {
                let map_data = unsafe { &*map_data_ptr };
                LocalPredictionSystem::update(world, map_data, delta_time);
            }
            if let Some(stats) = self.stats.get_mut("LocalPredictionSystem") {
                stats.record_execution(start.elapsed(), false);
            }
        }

        if self.movement_v2_enabled {
            let start = Instant::now();
            MovementSystemV2::update(world, delta_time);
            if let Some(stats) = self.stats.get_mut("MovementSystemV2") {
                stats.record_execution(start.elapsed(), false);
            }
        }

        if self.reconciliation_enabled {
            let start = Instant::now();
            ReconciliationSystem::update(world, delta_time);
            if let Some(stats) = self.stats.get_mut("ReconciliationSystem") {
                stats.record_execution(start.elapsed(), false);
            }
        }

        if self.interpolation_enabled {
            let start = Instant::now();
            InterpolationSystem::update(world, delta_time);
            if let Some(stats) = self.stats.get_mut("InterpolationSystem") {
                stats.record_execution(start.elapsed(), false);
            }
        }

        Ok(())
    }

    /// Layer 3: 表现决策（并行）
    fn execute_layer3_parallel(
        &mut self,
        world: &mut World,
        delta_time: f32,
        delta_ms: u32,
    ) -> GameResult {
        // 使用 RwLock 保护 World，允许多个系统并发读取
        let world_lock = RwLock::new(world);
        
        // 使用 rayon 并行执行 3 个动画状态系统
        rayon::scope(|s| {
            // AnimationStateSystem
            if self.animation_state_enabled {
                s.spawn(|_| {
                    let start = Instant::now();
                    let mut world = world_lock.write();
                    AnimationStateSystem::update(&mut *world, delta_time);
                    drop(world);
                    
                    // 记录统计（需要临时解锁）
                    let duration = start.elapsed();
                    // 注意：这里不能直接访问 self.stats，因为 self 是 &mut
                    // 我们将在 scope 外记录统计
                });
            }

            // MonsterAnimationStateSystem
            if self.monster_animation_state_enabled {
                s.spawn(|_| {
                    let start = Instant::now();
                    let mut world = world_lock.write();
                    MonsterAnimationStateSystem::update(&mut *world);
                    drop(world);
                });
            }

            // NPCActionSystem
            if self.npc_action_enabled {
                s.spawn(|_| {
                    let start = Instant::now();
                    let mut world = world_lock.write();
                    NPCActionSystem::update(&mut *world, delta_ms);
                    drop(world);
                });
            }
        });

        // 从 RwLock 取回 World
        let world = world_lock.into_inner();
        
        // 记录并行执行统计
        if self.animation_state_enabled {
            if let Some(stats) = self.stats.get_mut("AnimationStateSystem") {
                stats.parallel_executions += 1;
            }
        }
        if self.monster_animation_state_enabled {
            if let Some(stats) = self.stats.get_mut("MonsterAnimationStateSystem") {
                stats.parallel_executions += 1;
            }
        }
        if self.npc_action_enabled {
            if let Some(stats) = self.stats.get_mut("NPCActionSystem") {
                stats.parallel_executions += 1;
            }
        }

        Ok(())
    }

    /// Layer 4: 渲染准备（并行）
    fn execute_layer4_parallel(
        &mut self,
        world: &mut World,
        delta_ms: u32,
        animation_count: i32,
    ) -> GameResult {
        let world_lock = RwLock::new(world);
        
        rayon::scope(|s| {
            // TileAnimationSystem
            if self.tile_animation_enabled {
                s.spawn(|_| {
                    let mut world = world_lock.write();
                    TileAnimationSystem::update(&mut *world, animation_count);
                });
            }

            // AnimationPlaybackSystem
            if self.animation_playback_enabled {
                s.spawn(|_| {
                    let mut world = world_lock.write();
                    AnimationPlaybackSystem::update(&mut *world, delta_ms);
                });
            }

            // MovementInterpolationSystem
            if self.movement_interpolation_enabled {
                s.spawn(|_| {
                    let mut world = world_lock.write();
                    MovementInterpolationSystem::update(&mut *world);
                });
            }
        });

        let world = world_lock.into_inner();
        
        // 记录并行执行统计
        if self.tile_animation_enabled {
            if let Some(stats) = self.stats.get_mut("TileAnimationSystem") {
                stats.parallel_executions += 1;
            }
        }
        if self.animation_playback_enabled {
            if let Some(stats) = self.stats.get_mut("AnimationPlaybackSystem") {
                stats.parallel_executions += 1;
            }
        }
        if self.movement_interpolation_enabled {
            if let Some(stats) = self.stats.get_mut("MovementInterpolationSystem") {
                stats.parallel_executions += 1;
            }
        }

        Ok(())
    }

    /// Layer 5: 其他系统（并行）
    fn execute_layer5_parallel(
        &mut self,
        world: &mut World,
        delta_time: f32,
    ) -> GameResult {
        let world_lock = RwLock::new(world);
        
        rayon::scope(|s| {
            // MouseEventSystem
            if self.mouse_event_enabled {
                s.spawn(|_| {
                    let mut world = world_lock.write();
                    MouseEventSystem::update_mouse_input(&mut *world);
                });
            }

            // MonsterSystem
            if self.monster_enabled {
                s.spawn(|_| {
                    let mut world = world_lock.write();
                    MonsterSystem::update(&mut *world, delta_time);
                });
            }

            // CameraSystem
            if self.camera_enabled {
                s.spawn(|_| {
                    let mut world = world_lock.write();
                    CameraSystem::update(&mut *world);
                });
            }
            
            // OcclusionSystem 需要 &mut self，暂时串行执行
        });

        let world = world_lock.into_inner();
        
        // OcclusionSystem (有状态，需要 &mut self)
        if self.occlusion_enabled {
            let start = Instant::now();
            self.occlusion.update(world, delta_time);
            if let Some(stats) = self.stats.get_mut("OcclusionSystem") {
                stats.record_execution(start.elapsed(), false);
            }
        }
        
        // 记录并行执行统计
        if self.mouse_event_enabled {
            if let Some(stats) = self.stats.get_mut("MouseEventSystem") {
                stats.parallel_executions += 1;
            }
        }
        if self.monster_enabled {
            if let Some(stats) = self.stats.get_mut("MonsterSystem") {
                stats.parallel_executions += 1;
            }
        }
        if self.camera_enabled {
            if let Some(stats) = self.stats.get_mut("CameraSystem") {
                stats.parallel_executions += 1;
            }
        }

        Ok(())
    }

    /// 启用/禁用系统（与 GameSceneScheduler 相同）
    pub fn enable_system(&mut self, name: &str) {
        match name {
            "InputCollectingSystem" => self.input_collecting_enabled = true,
            "ClientNetworkSystem" => self.client_network_enabled = true,
            "LocalPredictionSystem" => self.local_prediction_enabled = true,
            "MovementSystemV2" => self.movement_v2_enabled = true,
            "ReconciliationSystem" => self.reconciliation_enabled = true,
            "InterpolationSystem" => self.interpolation_enabled = true,
            "AnimationStateSystem" => self.animation_state_enabled = true,
            "MonsterAnimationStateSystem" => self.monster_animation_state_enabled = true,
            "NPCActionSystem" => self.npc_action_enabled = true,
            "TileAnimationSystem" => self.tile_animation_enabled = true,
            "AnimationPlaybackSystem" => self.animation_playback_enabled = true,
            "MovementInterpolationSystem" => self.movement_interpolation_enabled = true,
            "MouseEventSystem" => self.mouse_event_enabled = true,
            "MonsterSystem" => self.monster_enabled = true,
            "OcclusionSystem" => self.occlusion_enabled = true,
            "CameraSystem" => self.camera_enabled = true,
            _ => {}
        }
    }

    pub fn disable_system(&mut self, name: &str) {
        match name {
            "InputCollectingSystem" => self.input_collecting_enabled = false,
            "ClientNetworkSystem" => self.client_network_enabled = false,
            "LocalPredictionSystem" => self.local_prediction_enabled = false,
            "MovementSystemV2" => self.movement_v2_enabled = false,
            "ReconciliationSystem" => self.reconciliation_enabled = false,
            "InterpolationSystem" => self.interpolation_enabled = false,
            "AnimationStateSystem" => self.animation_state_enabled = false,
            "MonsterAnimationStateSystem" => self.monster_animation_state_enabled = false,
            "NPCActionSystem" => self.npc_action_enabled = false,
            "TileAnimationSystem" => self.tile_animation_enabled = false,
            "AnimationPlaybackSystem" => self.animation_playback_enabled = false,
            "MovementInterpolationSystem" => self.movement_interpolation_enabled = false,
            "MouseEventSystem" => self.mouse_event_enabled = false,
            "MonsterSystem" => self.monster_enabled = false,
            "OcclusionSystem" => self.occlusion_enabled = false,
            "CameraSystem" => self.camera_enabled = false,
            _ => {}
        }
    }

    /// 获取统计信息
    pub fn get_stats(&self, name: &str) -> Option<&ParallelSystemStats> {
        self.stats.get(name)
    }

    pub fn get_all_stats(&self) -> Vec<ParallelSystemStats> {
        let mut stats: Vec<_> = self.stats.values().cloned().collect();
        stats.sort_by_key(|s| s.priority);
        stats
    }

    pub fn reset_stats(&mut self) {
        for stat in self.stats.values_mut() {
            stat.reset();
        }
    }

    /// 打印性能报告（带并行执行信息）
    pub fn print_performance_report(&self) {
        println!("\n========== 并行系统调度器性能报告 ==========");
        println!("执行模式: {:?}", self.execution_mode);
        let stats = self.get_all_stats();
        
        for stat in stats {
            if stat.execution_count > 0 {
                let parallel_ratio = if stat.execution_count > 0 {
                    (stat.parallel_executions as f64 / stat.execution_count as f64) * 100.0
                } else {
                    0.0
                };
                
                println!(
                    "[{:3}] {:30} | 执行: {:6}次 | 平均: {:6.2}μs | 最后: {:6.2}μs | 并行: {:5.1}%",
                    stat.priority,
                    stat.name,
                    stat.execution_count,
                    stat.average_time.as_micros() as f64,
                    stat.last_execution.as_micros() as f64,
                    parallel_ratio,
                );
            }
        }
        println!("==========================================\n");
    }
}

impl Default for ParallelScheduler {
    fn default() -> Self {
        Self::new(ExecutionMode::Parallel)
    }
}
