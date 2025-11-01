// ============================================================================
// Game Event Dispatcher - 游戏事件分发系统
// ============================================================================
//
// ## 职责说明
//
// **重要**: 本系统不再维护事件队列！所有事件统一存储在 GlobalEvents 组件中。
//
// ### 职责
//
// 1. 从 GlobalEvents.game_events 读取游戏事件
// 2. 根据事件类型分发到相应的处理逻辑
// 3. 记录事件处理日志（调试用）
//
// ### 与 GlobalEvents 的关系
//
// ```
// GlobalEvents (事件总线)
// ├─ keyboard_events    - 由 InputSystem 写入
// ├─ mouse_events       - 由 InputSystem 写入
// ├─ game_events        - 由各系统写入 ⬅ 本系统读取
// ├─ network_incoming   - ❌ 当前未使用 (NetworkSyncSystem 已废弃)
// └─ network_commands   - 由各系统发送网络命令 (通过 Channel)
// ```
//
// ### 网络事件说明
//
// **重要**: 网络事件当前不通过 GlobalEvents 传递！
// - SelectScene/LoginScene: 直接从 `NetContext.try_recv()` 读取
// - GameScene: 网络同步尚未实现，需要重新设计架构
//
// ### 设计理念
//
// - **只读不写**: 只读取 GlobalEvents.game_events，不写入
// - **无状态**: 不维护内部事件队列，所有状态在 GlobalEvents 中
// - **分发逻辑**: 根据事件类型调用相应处理函数
// - **日志记录**: 可选的事件处理日志（调试用）
//
// ### 执行时机
//
// - 优先级: 120 (Layer 1 最后)
// - 在 InputSystem(100) 和 PlayerControlSystem(110) 之后执行
// - 确保输入事件已经写入 GlobalEvents
//
// ### 注意事项
//
// ⚠️ **不要在本系统维护事件队列**，这会与 GlobalEvents 功能重叠！
// ⚠️ **事件清理由 EventCleanupSystem 负责**，本系统不清理事件
//
// ============================================================================

use crate::ecs::components::{GlobalEvents, GameEvent};
use crate::ecs::systems::System;
use ggez::GameResult;
use hecs::World;

/// 游戏内部事件类型
/// 
/// 这些事件用于系统间通信，存储在 GlobalEvents.game_events 中
#[derive(Debug, Clone)]
pub enum InternalEvent {
    PlayerStateChanged { entity: hecs::Entity },
    MonsterStateChanged { entity: hecs::Entity },
    UIEvent { event_type: String, data: String },
    SoundTriggered { sound_id: u32, position: Option<(f32, f32)> },
    ParticleTriggered { effect_id: u32, position: (f32, f32) },
    CameraShake { intensity: f32, duration: f32 },
}

/// 游戏事件分发系统
/// 
/// 从 GlobalEvents 读取事件并分发处理
pub struct GameEventDispatcher {
    events_processed_this_frame: usize,
    enable_logging: bool,
}

impl GameEventDispatcher {
    pub fn new() -> Self {
        Self {
            events_processed_this_frame: 0,
            enable_logging: false,
        }
    }
    
    /// 启用/禁用事件日志
    pub fn set_logging(&mut self, enabled: bool) {
        self.enable_logging = enabled;
    }
    
    /// 处理所有游戏事件
    fn process_all_events(&mut self, world: &mut World) {
        self.events_processed_this_frame = 0;
        
        // 先收集事件（避免借用冲突）
        let events_to_process: Vec<_> = {
            let mut query = world.query::<&GlobalEvents>();
            if let Some((_, events)) = query.iter().next() {
                events.net_events.iter().cloned().collect()
            } else {
                Vec::new()
            }
        };
        
        // 处理收集的事件
        for event in events_to_process.iter() {
            self.handle_game_event(world, event);
            self.events_processed_this_frame += 1;
        }
        
        if self.enable_logging && self.events_processed_this_frame > 0 {
            tracing::debug!(
                "🎮 GameEventDispatcher: 处理了 {} 个游戏事件",
                self.events_processed_this_frame
            );
        }
    }
    
    /// 处理单个游戏事件
    fn handle_game_event(&mut self, _world: &mut World, event: &GameEvent) {
        if self.enable_logging {
            match event {
                // 角色事件
                GameEvent::StartGameRequest { character_index } => {
                    tracing::info!("📡 事件分发: 开始游戏请求 - 角色索引: {}", character_index);
                }
                GameEvent::StartGame { delay } => {
                    tracing::info!("📡 事件分发: 开始游戏 - 延迟: {}ms", delay);
                }
                
                // 移动事件
                GameEvent::WalkRequest { direction } => {
                    tracing::debug!("📡 事件分发: 行走请求 -> 方向: {:?}", direction);
                }
                GameEvent::RunRequest { direction } => {
                    tracing::debug!("📡 事件分发: 奔跑请求 -> 方向: {:?}", direction);
                }
                GameEvent::PlayerLocationChanged { x, y } => {
                    tracing::debug!("📡 事件分发: 玩家位置变化 -> ({}, {})", x, y);
                }
                
                // 战斗事件
                GameEvent::AttackRequest { direction, spell } => {
                    tracing::debug!("📡 事件分发: 攻击请求 -> 方向: {:?}, 技能: {}", direction, spell);
                }
                GameEvent::MagicRequest { spell, target_id, .. } => {
                    tracing::debug!("📡 事件分发: 魔法请求 -> 技能: {}, 目标: {}", spell, target_id);
                }
                
                // NPC 事件
                GameEvent::NPCCallRequest { npc_object_id } => {
                    tracing::info!("📡 事件分发: 呼叫 NPC - ID: {}", npc_object_id);
                }
                GameEvent::NpcDialog { npc_id, dialog } => {
                    tracing::info!("📡 事件分发: NPC 对话 - ID: {}, 内容长度: {}", npc_id, dialog.len());
                }
                
                // 地图事件
                GameEvent::MapChanged { file_name, title, .. } => {
                    tracing::info!("📡 事件分发: 地图切换 -> {} ({})", title, file_name);
                }
                
                // 聊天事件
                GameEvent::ChatMessage { sender, message, chat_type } => {
                    tracing::debug!("📡 事件分发: 聊天消息 - [{:?}] {}: {}", chat_type, sender, message);
                }
                
                _ => {
                    // 其他事件不记录日志
                }
            }
        }
    }
}

impl Default for GameEventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl System for GameEventDispatcher {
    fn priority(&self) -> u32 {
        120 // Layer 1 最后，在 InputSystem(100) 和 PlayerControlSystem(110) 之后
    }

    fn update(&mut self, world: &mut hecs::World, _delay_time: f32) -> GameResult {
        self.process_all_events(world);
        Ok(())
    }
}

// 为了向后兼容，保留 GameEventSystem 别名
pub type GameEventSystem = GameEventDispatcher;
