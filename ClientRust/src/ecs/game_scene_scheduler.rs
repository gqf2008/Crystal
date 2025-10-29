// ============================================================================
// GameScene系统调度器 - 管理游戏场景中实际使用的系统
// ============================================================================
//
// 职责：
// - 统一管理GameScene中的所有ECS系统
// - 按正确的优先级顺序执行系统
// - 提供性能统计和监控
// - 支持运行时启用/禁用系统
//
// 系统列表（按优先级）：
// Layer 1 (100-199): 输入和网络
//   - InputCollectingSystem (100)
//   - ClientNetworkSystem (150)
//
// Layer 2 (200-299): 核心逻辑
//   - LocalPredictionSystem (200)
//   - MovementSystemV2 (210)
//   - ReconciliationSystem (220)
//   - InterpolationSystem (230)
//
// Layer 3 (300-399): 表现决策
//   - AnimationStateSystem (300)
//   - MonsterAnimationStateSystem (310)
//   - NPCActionSystem (320)
//
// Layer 4 (400-499): 渲染准备
//   - TileAnimationSystem (400)
//   - AnimationPlaybackSystem (410)
//   - MovementInterpolationSystem (420)
//
// Layer 5 (500-599): 其他系统
//   - MouseEventSystem (500)
//   - MonsterSystem (510)
//   - OcclusionSystem (520)
//   - CameraSystem (530)
//
// ============================================================================

use hecs::World;
use ggez::GameResult;
use ggez::Context;
use std::time::{Duration, Instant};
use std::collections::HashMap;
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

/// 系统优先级常量
pub mod priority {
    // Layer 1: 输入和网络
    pub const INPUT_COLLECTING: u32 = 100;
    pub const CLIENT_NETWORK: u32 = 150;
    
    // Layer 2: 核心逻辑
    pub const LOCAL_PREDICTION: u32 = 200;
    pub const MOVEMENT_V2: u32 = 210;
    pub const RECONCILIATION: u32 = 220;
    pub const INTERPOLATION: u32 = 230;
    
    // Layer 3: 表现决策
    pub const ANIMATION_STATE: u32 = 300;
    pub const MONSTER_ANIMATION_STATE: u32 = 310;
    pub const NPC_ACTION: u32 = 320;
    
    // Layer 4: 渲染准备
    pub const TILE_ANIMATION: u32 = 400;
    pub const ANIMATION_PLAYBACK: u32 = 410;
    pub const MOVEMENT_INTERPOLATION: u32 = 420;
    
    // Layer 5: 其他系统
    pub const MOUSE_EVENT: u32 = 500;
    pub const MONSTER: u32 = 510;
    pub const OCCLUSION: u32 = 520;
    pub const CAMERA: u32 = 530;
}

/// 系统性能统计
#[derive(Debug, Clone)]
pub struct SystemStats {
    pub name: String,
    pub priority: u32,
    pub execution_count: u64,
    pub total_time: Duration,
    pub average_time: Duration,
    pub last_execution: Duration,
}

impl SystemStats {
    fn new(name: String, priority: u32) -> Self {
        Self {
            name,
            priority,
            execution_count: 0,
            total_time: Duration::ZERO,
            average_time: Duration::ZERO,
            last_execution: Duration::ZERO,
        }
    }

    fn record_execution(&mut self, duration: Duration) {
        self.execution_count += 1;
        self.total_time += duration;
        self.last_execution = duration;
        self.average_time = self.total_time / self.execution_count as u32;
    }

    fn reset(&mut self) {
        self.execution_count = 0;
        self.total_time = Duration::ZERO;
        self.average_time = Duration::ZERO;
        self.last_execution = Duration::ZERO;
    }
}

/// GameScene系统调度器
pub struct GameSceneScheduler {
    // Layer 1
    input_collecting_enabled: bool,
    client_network_enabled: bool,

    // Layer 2
    local_prediction_enabled: bool,
    movement_v2_enabled: bool,
    reconciliation_enabled: bool,
    interpolation_enabled: bool,

    // Layer 3
    animation_state_enabled: bool,
    monster_animation_state_enabled: bool,
    npc_action_enabled: bool,

    // Layer 4
    tile_animation_enabled: bool,
    animation_playback_enabled: bool,
    movement_interpolation_enabled: bool,

    // Layer 5
    mouse_event_enabled: bool,
    monster_enabled: bool,
    occlusion: OcclusionSystem,
    occlusion_enabled: bool,
    camera_enabled: bool,

    /// 性能统计
    stats: HashMap<String, SystemStats>,
}

impl GameSceneScheduler {
    /// 创建新的调度器
    pub fn new() -> Self {
        let mut scheduler = Self {
            // Layer 1
            input_collecting_enabled: true,
            client_network_enabled: true,

            // Layer 2
            local_prediction_enabled: true,
            movement_v2_enabled: true,
            reconciliation_enabled: true,
            interpolation_enabled: true,

            // Layer 3
            animation_state_enabled: true,
            monster_animation_state_enabled: true,
            npc_action_enabled: true,

            // Layer 4
            tile_animation_enabled: true,
            animation_playback_enabled: true,
            movement_interpolation_enabled: true,

            // Layer 5
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
            ("InputCollectingSystem", priority::INPUT_COLLECTING),
            ("ClientNetworkSystem", priority::CLIENT_NETWORK),
            ("LocalPredictionSystem", priority::LOCAL_PREDICTION),
            ("MovementSystemV2", priority::MOVEMENT_V2),
            ("ReconciliationSystem", priority::RECONCILIATION),
            ("InterpolationSystem", priority::INTERPOLATION),
            ("AnimationStateSystem", priority::ANIMATION_STATE),
            ("MonsterAnimationStateSystem", priority::MONSTER_ANIMATION_STATE),
            ("NPCActionSystem", priority::NPC_ACTION),
            ("TileAnimationSystem", priority::TILE_ANIMATION),
            ("AnimationPlaybackSystem", priority::ANIMATION_PLAYBACK),
            ("MovementInterpolationSystem", priority::MOVEMENT_INTERPOLATION),
            ("MouseEventSystem", priority::MOUSE_EVENT),
            ("MonsterSystem", priority::MONSTER),
            ("OcclusionSystem", priority::OCCLUSION),
            ("CameraSystem", priority::CAMERA),
        ];

        for (name, priority) in systems {
            self.stats.insert(
                name.to_string(),
                SystemStats::new(name.to_string(), priority),
            );
        }
    }

    /// 执行所有系统
    pub fn update(
        &mut self,
        ctx: &mut Context,
        world: &mut World,
        delta_time: f32,
        delta_ms: u32,
        animation_count: i32,
        network_tx: Option<&mpsc::UnboundedSender<NetworkCommand>>,
    ) -> GameResult {
        // 宏：简化系统执行和统计
        macro_rules! run_system {
            ($enabled:expr, $name:literal, $code:block) => {
                if $enabled {
                    let start = Instant::now();
                    $code
                    let duration = start.elapsed();
                    if let Some(stats) = self.stats.get_mut($name) {
                        stats.record_execution(duration);
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

        // Layer 2: 核心逻辑
        run_system!(self.local_prediction_enabled, "LocalPredictionSystem", {
            // 获取 MapData 引用
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

        // Layer 3: 表现决策
        run_system!(self.animation_state_enabled, "AnimationStateSystem", {
            AnimationStateSystem::update(world, delta_time);
        });

        run_system!(self.monster_animation_state_enabled, "MonsterAnimationStateSystem", {
            MonsterAnimationStateSystem::update(world);
        });

        run_system!(self.npc_action_enabled, "NPCActionSystem", {
            NPCActionSystem::update(world, delta_ms);
        });

        // Layer 4: 渲染准备
        run_system!(self.tile_animation_enabled, "TileAnimationSystem", {
            TileAnimationSystem::update(world, animation_count);
        });

        run_system!(self.animation_playback_enabled, "AnimationPlaybackSystem", {
            AnimationPlaybackSystem::update(world, delta_ms);
        });

        run_system!(self.movement_interpolation_enabled, "MovementInterpolationSystem", {
            MovementInterpolationSystem::update(world);
        });

        // Layer 5: 其他系统
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

    /// 启用系统
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

    /// 禁用系统
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

    /// 获取系统统计
    pub fn get_stats(&self, name: &str) -> Option<&SystemStats> {
        self.stats.get(name)
    }

    /// 获取所有系统统计
    pub fn get_all_stats(&self) -> Vec<SystemStats> {
        let mut stats: Vec<_> = self.stats.values().cloned().collect();
        stats.sort_by_key(|s| s.priority);
        stats
    }

    /// 重置统计
    pub fn reset_stats(&mut self) {
        for stat in self.stats.values_mut() {
            stat.reset();
        }
    }

    /// 打印性能报告
    pub fn print_performance_report(&self) {
        println!("\n========== GameScene系统性能报告 ==========");
        let stats = self.get_all_stats();
        
        for stat in stats {
            if stat.execution_count > 0 {
                println!(
                    "[{:3}] {:30} | 执行: {:6}次 | 平均: {:6.2}μs | 最后: {:6.2}μs | 总计: {:8.2}ms",
                    stat.priority,
                    stat.name,
                    stat.execution_count,
                    stat.average_time.as_micros() as f64 / 1.0,
                    stat.last_execution.as_micros() as f64 / 1.0,
                    stat.total_time.as_micros() as f64 / 1000.0,
                );
            }
        }
        println!("==========================================\n");
    }
}

impl Default for GameSceneScheduler {
    fn default() -> Self {
        Self::new()
    }
}
