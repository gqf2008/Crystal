// ============================================================================
// MapViewer系统调度器 - 专为地图查看器设计的串行调度器
// ============================================================================
//
// 职责：
// - 统一管理地图查看器中的所有ECS系统
// - 按正确的优先级顺序执行系统（串行）
// - 提供性能统计和监控
// - 支持运行时启用/禁用系统
//
// 系统列表（按优先级）：
// Layer 1 (100-199): 输入
//   - InputCollectingSystem (100)
//
// Layer 2 (200-299): 核心逻辑
//   - LocalPredictionSystem (200)
//   - MovementSystemV2 (210)
//
// Layer 3 (300-399): 表现
//   - PlayerAnimationSystem (300)
//
// Layer 5 (500-599): 其他
//   - CameraSystem (530)
//
// ============================================================================

use hecs::World;
use ggez::GameResult;
use ggez::Context;
use std::time::{Duration, Instant};
use std::collections::HashMap;

use crate::ecs::systems::{
    CameraSystem,
    InputCollectingSystem,
    LocalPredictionSystem,
    MovementSystemV2,
};
use crate::ecs::systems::layer3_presentation::player_animation_system::PlayerAnimationSystem;

/// 系统优先级常量
pub mod priority {
    // Layer 1: 输入
    pub const INPUT_COLLECTING: u32 = 100;
    
    // Layer 2: 核心逻辑
    pub const LOCAL_PREDICTION: u32 = 200;
    pub const MOVEMENT_V2: u32 = 210;
    
    // Layer 3: 表现
    pub const PLAYER_ANIMATION: u32 = 300;
    
    // Layer 5: 其他
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

/// MapViewer系统调度器
pub struct MapViewerScheduler {
    // Layer 1
    input_collecting_enabled: bool,

    // Layer 2
    local_prediction_enabled: bool,
    movement_v2_enabled: bool,

    // Layer 3
    player_animation_enabled: bool,

    // Layer 5
    camera_enabled: bool,

    /// 性能统计
    stats: HashMap<String, SystemStats>,
}

impl MapViewerScheduler {
    /// 创建新的调度器
    pub fn new() -> Self {
        let mut scheduler = Self {
            // Layer 1
            input_collecting_enabled: true,

            // Layer 2
            local_prediction_enabled: true,
            movement_v2_enabled: true,

            // Layer 3
            player_animation_enabled: true,

            // Layer 5
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
            ("LocalPredictionSystem", priority::LOCAL_PREDICTION),
            ("MovementSystemV2", priority::MOVEMENT_V2),
            ("PlayerAnimationSystem", priority::PLAYER_ANIMATION),
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

        // Layer 1: 输入
        run_system!(self.input_collecting_enabled, "InputCollectingSystem", {
            InputCollectingSystem::update(world, ctx);
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

        // Layer 3: 表现
        run_system!(self.player_animation_enabled, "PlayerAnimationSystem", {
            PlayerAnimationSystem::update(world);
        });

        // Layer 5: 其他
        run_system!(self.camera_enabled, "CameraSystem", {
            CameraSystem::update(world);
        });

        Ok(())
    }

    /// 启用系统
    pub fn enable_system(&mut self, name: &str) {
        match name {
            "InputCollectingSystem" => self.input_collecting_enabled = true,
            "LocalPredictionSystem" => self.local_prediction_enabled = true,
            "MovementSystemV2" => self.movement_v2_enabled = true,
            "PlayerAnimationSystem" => self.player_animation_enabled = true,
            "CameraSystem" => self.camera_enabled = true,
            _ => {}
        }
    }

    /// 禁用系统
    pub fn disable_system(&mut self, name: &str) {
        match name {
            "InputCollectingSystem" => self.input_collecting_enabled = false,
            "LocalPredictionSystem" => self.local_prediction_enabled = false,
            "MovementSystemV2" => self.movement_v2_enabled = false,
            "PlayerAnimationSystem" => self.player_animation_enabled = false,
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
        println!("\n========== MapViewer系统性能报告 ==========");
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

impl Default for MapViewerScheduler {
    fn default() -> Self {
        Self::new()
    }
}
