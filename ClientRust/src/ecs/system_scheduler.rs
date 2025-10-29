// ============================================================================
// SystemScheduler - 统一系统调度器
// ============================================================================
//
// **职责**:
// - 管理所有18个ECS系统的生命周期
// - 按优先级顺序执行系统 (50-610)
// - 支持启用/禁用单个系统
// - 性能监控和统计
//
// **执行顺序**:
// Layer 1 (50-199): Input Processing
// Layer 2 (200-299): Decision Making
// Layer 3 (300-399): Combat & Skills
// Layer 4 (400-499): Physics & Movement
// Layer 5 (500-599): State Update
// Layer 6 (595-610): Network Sync
//
// ============================================================================

use hecs::World;
use ggez::GameResult;
use std::time::{Duration, Instant};
use std::collections::HashMap;

use crate::ecs::systems::{System, priority};

// 导入所有已实现的系统
use crate::ecs::systems::update::input::PlayerControlSystem;
use crate::ecs::systems::update::decision::{MonsterAISystem, NpcDialogueSystem};
use crate::ecs::systems::update::combat_skill::{SkillSystem, CombatSystem};
use crate::ecs::systems::update::physics_movement::{MovementSystem, CollisionSystem};
use crate::ecs::systems::update::state_update::{
    AnimationSystem, ParticleSystem, HealthRegenSystem, SoundSystem, CameraSystem,
};
use crate::ecs::systems::update::network_sync::{
    ClientPredictionSystem, NetworkSendSystem, SyncSystem,
};

/// 系统执行统计
#[derive(Debug, Clone)]
pub struct SystemStats {
    /// 系统名称
    pub name: String,
    /// 优先级
    pub priority: u32,
    /// 执行次数
    pub execution_count: u64,
    /// 总执行时间
    pub total_time: Duration,
    /// 平均执行时间
    pub average_time: Duration,
    /// 最后执行时间
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
        self.average_time = self.total_time / self.execution_count as u32;
        self.last_execution = duration;
    }
}

/// 系统调度器 - 管理18个已实现的ECS系统
pub struct SystemScheduler {
    // Layer 1: Input Processing
    player_control: PlayerControlSystem,
    player_control_enabled: bool,

    // Layer 2: Decision Making
    monster_ai: MonsterAISystem,
    monster_ai_enabled: bool,
    
    npc_dialogue: NpcDialogueSystem,
    npc_dialogue_enabled: bool,

    // Layer 3: Combat & Skills
    skill: SkillSystem,
    skill_enabled: bool,
    
    combat: CombatSystem,
    combat_enabled: bool,

    // Layer 4: Physics & Movement
    movement: MovementSystem,
    movement_enabled: bool,
    
    collision: CollisionSystem,
    collision_enabled: bool,

    // Layer 5: State Update
    animation: AnimationSystem,
    animation_enabled: bool,
    
    particle: ParticleSystem,
    particle_enabled: bool,
    
    health_regen: HealthRegenSystem,
    health_regen_enabled: bool,
    
    sound: SoundSystem,
    sound_enabled: bool,
    
    camera: CameraSystem,
    camera_enabled: bool,

    // Layer 6: Network Sync
    client_prediction: ClientPredictionSystem,
    client_prediction_enabled: bool,
    
    network_send: NetworkSendSystem,
    network_send_enabled: bool,
    
    sync: SyncSystem,
    sync_enabled: bool,

    // 性能统计
    stats: HashMap<String, SystemStats>,
}

impl SystemScheduler {
    /// 创建新的系统调度器
    pub fn new() -> Self {
        let mut scheduler = Self {
            // Layer 1
            player_control: PlayerControlSystem,
            player_control_enabled: true,

            // Layer 2
            monster_ai: MonsterAISystem,
            monster_ai_enabled: true,
            npc_dialogue: NpcDialogueSystem::new(),
            npc_dialogue_enabled: true,

            // Layer 3
            skill: SkillSystem,
            skill_enabled: true,
            combat: CombatSystem,
            combat_enabled: true,

            // Layer 4
            movement: MovementSystem,
            movement_enabled: true,
            collision: CollisionSystem::new(),
            collision_enabled: true,

            // Layer 5
            animation: AnimationSystem::new(),
            animation_enabled: true,
            particle: ParticleSystem,
            particle_enabled: true,
            health_regen: HealthRegenSystem,
            health_regen_enabled: true,
            sound: SoundSystem::new(),
            sound_enabled: true,
            camera: CameraSystem::new(),
            camera_enabled: true,

            // Layer 6
            client_prediction: ClientPredictionSystem,
            client_prediction_enabled: true,
            network_send: NetworkSendSystem,
            network_send_enabled: true,
            sync: SyncSystem,
            sync_enabled: true,

            stats: HashMap::new(),
        };

        // 初始化统计信息
        scheduler.initialize_stats();
        scheduler
    }

    /// 初始化所有系统的统计信息 (14个已实现系统)
    fn initialize_stats(&mut self) {
        let systems = vec![
            ("PlayerControlSystem", priority::PLAYER_CONTROL),
            ("MonsterAISystem", priority::MONSTER_AI),
            ("NpcDialogueSystem", priority::DIALOGUE),
            ("SkillSystem", priority::SKILL),
            ("CombatSystem", priority::COMBAT),
            ("MovementSystem", priority::MOVEMENT),
            ("CollisionSystem", priority::COLLISION),
            ("AnimationSystem", priority::ANIMATION),
            ("ParticleSystem", priority::PARTICLE),
            ("HealthRegenSystem", priority::PARTICLE), // 与Particle同级510
            ("SoundSystem", priority::SOUND),
            ("CameraSystem", priority::CAMERA),
            ("ClientPredictionSystem", priority::SYNC), // 595,接近sync
            ("NetworkSendSystem", priority::NETWORK_SEND),
            ("SyncSystem", priority::SYNC),
        ];

        for (name, priority) in systems {
            self.stats.insert(
                name.to_string(),
                SystemStats::new(name.to_string(), priority),
            );
        }
    }

    /// 执行所有启用的系统 (按优先级顺序)
    pub fn update(&mut self, world: &mut World, delta_time: f32) -> GameResult {
        // 宏简化系统执行和统计记录
        macro_rules! run_system {
            ($enabled:expr, $name:literal, $system:expr) => {
                if $enabled {
                    let start = Instant::now();
                    $system.update(world, delta_time)?;
                    let duration = start.elapsed();
                    if let Some(stats) = self.stats.get_mut($name) {
                        stats.record_execution(duration);
                    }
                }
            };
        }

        // Layer 1: Input Processing
        run_system!(self.player_control_enabled, "PlayerControlSystem", self.player_control);

        // Layer 2: Decision Making
        run_system!(self.monster_ai_enabled, "MonsterAISystem", self.monster_ai);
        run_system!(self.npc_dialogue_enabled, "NpcDialogueSystem", self.npc_dialogue);

        // Layer 3: Combat & Skills
        run_system!(self.skill_enabled, "SkillSystem", self.skill);
        run_system!(self.combat_enabled, "CombatSystem", self.combat);

        // Layer 4: Physics & Movement
        run_system!(self.movement_enabled, "MovementSystem", self.movement);
        run_system!(self.collision_enabled, "CollisionSystem", self.collision);

        // Layer 5: State Update
        run_system!(self.animation_enabled, "AnimationSystem", self.animation);
        run_system!(self.particle_enabled, "ParticleSystem", self.particle);
        run_system!(self.health_regen_enabled, "HealthRegenSystem", self.health_regen);
        run_system!(self.sound_enabled, "SoundSystem", self.sound);
        run_system!(self.camera_enabled, "CameraSystem", self.camera);

        // Layer 6: Network Sync
        run_system!(self.client_prediction_enabled, "ClientPredictionSystem", self.client_prediction);
        run_system!(self.network_send_enabled, "NetworkSendSystem", self.network_send);
        run_system!(self.sync_enabled, "SyncSystem", self.sync);

        Ok(())
    }

    /// 启用系统
    pub fn enable_system(&mut self, name: &str) {
        match name {
            "PlayerControlSystem" => self.player_control_enabled = true,
            "MonsterAISystem" => self.monster_ai_enabled = true,
            "NpcDialogueSystem" => self.npc_dialogue_enabled = true,
            "SkillSystem" => self.skill_enabled = true,
            "CombatSystem" => self.combat_enabled = true,
            "MovementSystem" => self.movement_enabled = true,
            "CollisionSystem" => self.collision_enabled = true,
            "AnimationSystem" => self.animation_enabled = true,
            "ParticleSystem" => self.particle_enabled = true,
            "HealthRegenSystem" => self.health_regen_enabled = true,
            "SoundSystem" => self.sound_enabled = true,
            "CameraSystem" => self.camera_enabled = true,
            "ClientPredictionSystem" => self.client_prediction_enabled = true,
            "NetworkSendSystem" => self.network_send_enabled = true,
            "SyncSystem" => self.sync_enabled = true,
            _ => tracing::warn!("未知系统: {}", name),
        }
    }

    /// 禁用系统
    pub fn disable_system(&mut self, name: &str) {
        match name {
            "PlayerControlSystem" => self.player_control_enabled = false,
            "MonsterAISystem" => self.monster_ai_enabled = false,
            "NpcDialogueSystem" => self.npc_dialogue_enabled = false,
            "SkillSystem" => self.skill_enabled = false,
            "CombatSystem" => self.combat_enabled = false,
            "MovementSystem" => self.movement_enabled = false,
            "CollisionSystem" => self.collision_enabled = false,
            "AnimationSystem" => self.animation_enabled = false,
            "ParticleSystem" => self.particle_enabled = false,
            "HealthRegenSystem" => self.health_regen_enabled = false,
            "SoundSystem" => self.sound_enabled = false,
            "CameraSystem" => self.camera_enabled = false,
            "ClientPredictionSystem" => self.client_prediction_enabled = false,
            "NetworkSendSystem" => self.network_send_enabled = false,
            "SyncSystem" => self.sync_enabled = false,
            _ => tracing::warn!("未知系统: {}", name),
        }
    }

    /// 获取系统统计信息
    pub fn get_stats(&self, name: &str) -> Option<&SystemStats> {
        self.stats.get(name)
    }

    /// 获取所有系统统计信息 (按优先级排序)
    pub fn get_all_stats(&self) -> Vec<&SystemStats> {
        let mut stats: Vec<&SystemStats> = self.stats.values().collect();
        stats.sort_by_key(|s| s.priority);
        stats
    }

    /// 重置所有统计信息
    pub fn reset_stats(&mut self) {
        for stats in self.stats.values_mut() {
            stats.execution_count = 0;
            stats.total_time = Duration::ZERO;
            stats.average_time = Duration::ZERO;
            stats.last_execution = Duration::ZERO;
        }
    }

    /// 打印性能报告
    pub fn print_performance_report(&self) {
        tracing::info!("=== ECS System Performance Report ===");
        tracing::info!("{:<30} {:>8} {:>15} {:>15} {:>15}", 
            "System", "Priority", "Exec Count", "Avg Time (μs)", "Last Time (μs)");
        tracing::info!("{:-<80}", "");

        for stats in self.get_all_stats() {
            tracing::info!("{:<30} {:>8} {:>15} {:>15} {:>15}",
                stats.name,
                stats.priority,
                stats.execution_count,
                stats.average_time.as_micros(),
                stats.last_execution.as_micros(),
            );
        }

        let total_time: Duration = self.stats.values()
            .map(|s| s.total_time)
            .sum();
        tracing::info!("{:-<80}", "");
        tracing::info!("Total Time: {:?}", total_time);
    }
}

impl Default for SystemScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let scheduler = SystemScheduler::new();
        
        // 验证所有系统默认启用
        assert!(scheduler.player_control_enabled);
        assert!(scheduler.movement_enabled);
        assert!(scheduler.combat_enabled);
        
        // 验证统计信息已初始化 (15个系统)
        assert_eq!(scheduler.stats.len(), 15);
    }

    #[test]
    fn test_enable_disable_system() {
        let mut scheduler = SystemScheduler::new();
        
        // 禁用系统
        scheduler.disable_system("MovementSystem");
        assert!(!scheduler.movement_enabled);
        
        // 启用系统
        scheduler.enable_system("MovementSystem");
        assert!(scheduler.movement_enabled);
    }

    #[test]
    fn test_system_execution() {
        let mut scheduler = SystemScheduler::new();
        let mut world = World::new();
        
        // 执行一帧
        scheduler.update(&mut world, 0.016).unwrap();
        
        // 验证统计信息更新
        let stats = scheduler.get_stats("MovementSystem").unwrap();
        assert_eq!(stats.execution_count, 1);
        assert!(stats.total_time > Duration::ZERO);
    }

    #[test]
    fn test_stats_ordering() {
        let scheduler = SystemScheduler::new();
        let all_stats = scheduler.get_all_stats();
        
        // 验证按优先级排序
        for i in 1..all_stats.len() {
            assert!(all_stats[i - 1].priority <= all_stats[i].priority);
        }
    }

    #[test]
    fn test_reset_stats() {
        let mut scheduler = SystemScheduler::new();
        let mut world = World::new();
        
        // 执行几帧
        for _ in 0..5 {
            scheduler.update(&mut world, 0.016).unwrap();
        }
        
        // 验证统计信息
        let stats_before = scheduler.get_stats("MovementSystem").unwrap();
        assert_eq!(stats_before.execution_count, 5);
        
        // 重置统计
        scheduler.reset_stats();
        
        let stats_after = scheduler.get_stats("MovementSystem").unwrap();
        assert_eq!(stats_after.execution_count, 0);
        assert_eq!(stats_after.total_time, Duration::ZERO);
    }
}
