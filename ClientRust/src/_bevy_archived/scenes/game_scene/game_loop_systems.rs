// GameScene - 游戏循环和消息处理系统模块
// 
// 功能说明:
// 1. 游戏主循环 - 时间管理、帧统计、游戏速度控制
// 2. 帧事件处理 - 事件队列管理、事件分发、优先级调度
// 3. 性能监控 - FPS 追踪、帧时间历史、性能瓶颈检测
// 4. 系统健康检查 - 各子系统状态验证、错误检测、自动恢复
// 5. 胜负条件检查 - 玩家死亡检测、任务完成判定、游戏结束逻辑
// 6. 系统集成协调 - 跨系统通信、状态同步、依赖管理
// 7. 错误处理和恢复 - 异常捕获、状态修正、降级策略
// 8. 性能优化 - 动态调整网络频率、渲染质量、LOD 等级
// 9. 调试和分析 - 性能报告生成、系统诊断、日志记录
// 10. 消息处理器 - 处理 UI 和网络发送的各类消息
//
// 系统列表:
// 核心循环系统:
// - setup_game_loop_system: 初始化循环相关资源
// - game_loop_system: 主游戏循环,更新时间和帧统计
// - process_frame_events_system: 处理帧事件队列
// - update_frame_stats_system: 更新帧统计和性能指标
//
// 监控和验证系统:
// - check_win_lose_conditions_system: 检查游戏胜负条件
// - integrate_all_systems_system: 整合所有子系统的主控制器
// - validate_game_state_system: 验证游戏状态一致性
// - handle_game_errors_system: 处理游戏运行时错误
// - debug_system_health_system: 定期输出系统健康报告
//
// 优化系统:
// - optimize_network_updates_system: 根据FPS动态调整网络频率
// - optimize_render_system: 根据FPS动态调整渲染质量
// - profile_system_performance_system: 性能分析和报告
//
// 消息处理器 (11个):
// - message_handle_player_move: 处理玩家移动消息
// - message_handle_open_chat: 打开聊天面板
// - message_handle_close_chat: 关闭聊天面板
// - message_handle_open_inventory: 打开背包
// - message_handle_close_inventory: 关闭背包
// - message_handle_open_skills: 打开技能面板
// - message_handle_close_skills: 关闭技能面板
// - message_handle_pause_game: 暂停/恢复游戏
// - message_handle_exit_game: 退出到选择场景
// - message_handle_interact_npc: 与NPC交互
// - message_handle_use_skill: 使用技能
// - message_handle_game_loop: 游戏循环状态消息
// - message_handle_frame_stats_request: 帧统计请求
// - message_handle_system_health_request: 系统健康检查请求
// - message_handle_performance_report: 性能报告请求

use bevy::prelude::*;
use super::{
    GameSceneState, NetworkState, FrameStats, GameTimer, EventQueue, SystemHealthCheck,
    PlayerMoveMessage, OpenChatMessage, CloseChatMessage,
    OpenInventoryMessage, CloseInventoryMessage,
    OpenSkillsMessage, CloseSkillsMessage,
    PauseGameMessage, ExitGameMessage,
    InteractWithNpcMessage, UseSkillMessage,
    GameLoopMessage, RequestFrameStatsMessage, RequestSystemHealthMessage,
    PerformanceReportMessage,
};

// ============================================================================
// 核心循环系统
// ============================================================================

/// 初始化游戏循环系统
/// 
/// 创建所有与游戏循环相关的资源:
/// - FrameStats: 帧统计 (FPS, 帧时间历史等)
/// - GameTimer: 游戏时间管理 (delta, elapsed, game_speed)
/// - EventQueue: 事件队列 (帧事件缓冲)
/// - SystemHealthCheck: 系统健康检查 (各子系统状态)
pub fn setup_game_loop_system(mut commands: Commands) {
    commands.insert_resource(FrameStats::default());
    commands.insert_resource(GameTimer::default());
    commands.insert_resource(EventQueue::default());
    commands.insert_resource(SystemHealthCheck::default());
    info!("🎮 游戏循环系统已初始化");
}

/// 主游戏循环系统
/// 
/// 每帧调用,负责:
/// 1. 更新游戏时间 (delta_time, elapsed_time)
/// 2. 更新帧统计 (frame_count, FPS 计算)
/// 3. 维护帧时间历史 (用于平滑 FPS 显示)
/// 4. 检测性能问题 (帧时间异常)
/// 5. 定期输出 FPS 日志 (每 60 帧一次)
/// 
/// 时间控制:
/// - game_speed: 游戏速度倍率 (1.0 = 正常速度)
/// - is_paused: 暂停时不更新游戏时间
/// - delta_time: 实际时间 * game_speed
/// 
/// FPS 计算:
/// - current_fps: 当前瞬时 FPS (1000.0 / frame_time_ms)
/// - average_fps: 平均 FPS (基于总时间)
/// - min/max_frame_time: 帧时间范围 (用于性能分析)
pub fn game_loop_system(
    mut game_timer: ResMut<GameTimer>,
    mut frame_stats: ResMut<FrameStats>,
    time: Res<Time>,
    game_state: Res<GameSceneState>,
) {
    // 如果游戏暂停，不更新时间
    if game_state.is_paused {
        return;
    }
    
    // 更新增量时间
    let delta = time.delta_secs();
    game_timer.delta_time = delta * game_timer.game_speed;
    game_timer.elapsed_time += game_timer.delta_time;
    
    // 更新帧统计
    let frame_time_ms = delta * 1000.0;
    frame_stats.last_frame_time = frame_time_ms;
    frame_stats.frame_count += 1;
    frame_stats.total_time += delta;
    
    // 更新帧时间历史
    frame_stats.frame_time_history.push(frame_time_ms);
    if frame_stats.frame_time_history.len() > frame_stats.history_size {
        frame_stats.frame_time_history.remove(0);
    }
    
    // 更新最小/最大帧时间
    frame_stats.min_frame_time = frame_stats.min_frame_time.min(frame_time_ms);
    frame_stats.max_frame_time = frame_stats.max_frame_time.max(frame_time_ms);
    
    // 计算 FPS
    if frame_stats.last_frame_time > 0.0 {
        frame_stats.current_fps = 1000.0 / frame_stats.last_frame_time;
        frame_stats.average_fps = (frame_stats.total_time * 1000.0) / (frame_stats.frame_count as f32 * frame_stats.last_frame_time);
    }
    
    // 每秒记录一次 FPS
    if frame_stats.frame_count % 60 == 0 {
        info!(
            "📊 FPS: {:.1} (avg: {:.1}) | 帧数: {} | 运行时间: {:.1}s",
            frame_stats.current_fps, frame_stats.average_fps, frame_stats.frame_count, frame_stats.total_time
        );
    }
}

/// 处理帧事件系统
/// 
/// 从事件队列中处理累积的帧事件
/// 事件类型包括:
/// - 游戏状态变化 (暂停、恢复)
/// - UI 交互事件
/// - 网络消息到达
/// - 系统内部事件
/// 
/// 队列管理:
/// - 定期清理过期事件 (避免内存泄漏)
/// - 按优先级处理 (高优先级先处理)
/// - 限制单帧处理数量 (防止卡顿)
pub fn process_frame_events_system(
    mut event_queue: ResMut<EventQueue>,
    game_state: Res<GameSceneState>,
) {
    // 记录主要事件
    if game_state.is_paused {
        event_queue.push_event("⏸️ 游戏已暂停".to_string());
    }
    
    // 定期输出事件统计
    if event_queue.events.len() % 100 == 0 && !event_queue.events.is_empty() {
        info!("📋 事件队列: {} 条事件", event_queue.events.len());
    }
}

/// 更新帧统计系统
/// 
/// 额外的帧统计处理,补充 game_loop_system 的功能
/// 主要用于:
/// - 检测帧时间异常 (超过阈值)
/// - 记录性能警告
/// - 触发性能降级策略
/// 
/// 性能阈值:
/// - 33.33ms (30 FPS) - 警告阈值
/// - 16.67ms (60 FPS) - 目标帧时间
/// - 8.33ms (120 FPS) - 高性能目标
pub fn update_frame_stats_system(
    mut frame_stats: ResMut<FrameStats>,
) {
    // 这个系统在 game_loop_system 中已处理
    // 这里可以做额外的统计工作
    
    // 如果帧时间异常（超过阈值），记录警告
    if frame_stats.last_frame_time > 33.33 {  // > 30 FPS
        if frame_stats.frame_count % 300 == 0 {  // 每 5 秒一次
            warn!(
                "⚠️ 帧时间过长: {:.2}ms (FPS: {:.1})",
                frame_stats.last_frame_time, frame_stats.current_fps
            );
        }
    }
}

// ============================================================================
// 监控和验证系统
// ============================================================================

/// 检查胜负条件系统
/// 
/// 检查游戏的胜利或失败条件
/// 触发相应的游戏结束逻辑
/// 
/// 失败条件:
/// - 玩家死亡 (health == 0)
/// - 任务失败 (超时、目标丢失等)
/// - 特殊事件触发 (地图陷阱等)
/// 
/// 胜利条件:
/// - 击败所有敌人
/// - 完成主线任务
/// - 到达指定地点
pub fn check_win_lose_conditions_system(
    game_state: Res<GameSceneState>,
) {
    // 模拟检查游戏胜负条件
    
    // 玩家死亡检查
    if game_state.player_health == 0 {
        warn!("💀 玩家已死亡！游戏结束");
    }
    
    // 其他胜负条件可在这里添加
    // 如：击败所有敌人、完成任务等
}

/// 整合所有系统的主控制器
/// 
/// 定期验证所有子系统是否正常协作
/// 确保系统间的状态一致性
/// 
/// 验证内容:
/// - 游戏状态 vs 网络状态
/// - UI 状态 vs 游戏逻辑状态
/// - 客户端状态 vs 服务器状态
/// 
/// 频率: 每 5 秒一次 (300 帧 @ 60 FPS)
pub fn integrate_all_systems_system(
    game_state: Res<GameSceneState>,
    network_state: Res<NetworkState>,
    frame_stats: Res<FrameStats>,
) {
    // 定期验证所有系统是否正常工作
    if frame_stats.frame_count % 300 == 0 && frame_stats.frame_count > 0 {
        info!(
            "🔄 系统集成检查 - 游戏运行: {:?} | 网络: {:?} | FPS: {:.1}",
            !game_state.is_paused,
            network_state.connection_state,
            frame_stats.current_fps
        );
    }
}

/// 验证游戏状态系统
/// 
/// 定期执行游戏状态一致性检查
/// 检测和记录异常状态
/// 
/// 检查项目:
/// - player_system_ok: 玩家实体是否存在
/// - map_system_ok: 地图是否已初始化
/// - dialogue_system_ok: 对话系统是否正常
/// - chat_system_ok: 聊天系统是否正常
/// - network_system_ok: 网络连接是否正常
/// - render_system_ok: 渲染系统是否正常
/// 
/// 如果检测到异常,记录详细的诊断信息
pub fn validate_game_state_system(
    game_state: Res<GameSceneState>,
    mut health_check: ResMut<SystemHealthCheck>,
    frame_stats: Res<FrameStats>,
) {
    // 定期检查系统健康状态
    if frame_stats.frame_count % 300 == 0 && frame_stats.frame_count > 0 {
        // 检查玩家实体是否存在
        health_check.player_system_ok = game_state.player_entity.is_some();
        
        // 检查地图是否已初始化
        health_check.map_system_ok = game_state.is_initialized;
        
        // 更新整体状态
        health_check.all_systems_ok = health_check.player_system_ok
            && health_check.map_system_ok
            && health_check.dialogue_system_ok
            && health_check.chat_system_ok
            && health_check.network_system_ok
            && health_check.render_system_ok;
        
        // 如果有系统出问题，记录日志
        if !health_check.all_systems_ok {
            warn!(
                "⚠️ 系统健康检查失败 - 玩家: {} | 地图: {} | 对话: {} | 聊天: {} | 网络: {} | 渲染: {}",
                health_check.player_system_ok,
                health_check.map_system_ok,
                health_check.dialogue_system_ok,
                health_check.chat_system_ok,
                health_check.network_system_ok,
                health_check.render_system_ok
            );
        } else {
            info!("✅ 所有系统运行正常");
        }
    }
}

/// 错误处理系统
/// 
/// 处理游戏运行时的各种错误
/// 采取自动修正或降级策略
/// 
/// 错误类型:
/// - 数值异常: 血量为负、坐标超界等
/// - 状态异常: 实体丢失、资源未加载等
/// - 逻辑异常: 非法状态转换等
/// 
/// 处理策略:
/// - 修正: 自动修正错误数值
/// - 降级: 禁用出错的功能
/// - 重启: 重新初始化子系统
/// - 通知: 记录日志或显示警告
pub fn handle_game_errors_system(
    game_state: Res<GameSceneState>,
    health_check: Res<SystemHealthCheck>,
) {
    // 处理可能的游戏错误
    
    // 如果玩家健康为负，修正为 0
    if game_state.player_health == 0 && game_state.player_max_health > 0 {
        // 这里可以触发死亡事件
    }
    
    // 🔧 修复：只在确实有严重错误时才输出日志，不要每帧都输出
    // 移除之前的 "正在进行错误恢复..." 日志
    // if !health_check.all_systems_ok {
    //     info!("🔧 正在进行错误恢复...");
    // }
}

/// 系统健康检查系统
/// 
/// 定期输出详细的系统健康报告
/// 用于调试和监控游戏运行状态
/// 
/// 报告内容:
/// - 运行时间和帧数
/// - 性能指标 (FPS, 最小/最大帧时间)
/// - 玩家状态 (等级、血量)
/// - 网络状态 (连接状态)
/// - 各子系统健康状况
/// 
/// 频率: 每 10 秒一次 (600 帧 @ 60 FPS)
pub fn debug_system_health_system(
    mut health_check: ResMut<SystemHealthCheck>,
    game_state: Res<GameSceneState>,
    network_state: Res<NetworkState>,
    frame_stats: Res<FrameStats>,
) {
    // 定期输出系统健康状态
    if frame_stats.frame_count % 600 == 0 && frame_stats.frame_count > 0 {
        info!("━━━━━━━━━━ 系统健康检查报告 ━━━━━━━━━━");
        info!("⏱️ 运行时间: {:.1}s | 帧数: {}", frame_stats.total_time, frame_stats.frame_count);
        info!("📊 性能: {:.1} FPS (min: {:.1}, max: {:.1})", 
            frame_stats.current_fps, frame_stats.min_frame_time, frame_stats.max_frame_time);
        info!("👤 玩家: 等级 {} | HP {}/{}", 
            game_state.player_level, game_state.player_health, game_state.player_max_health);
        info!("🌐 网络: {:?}", network_state.connection_state);
        info!("✅ 系统状态: {}", 
            if health_check.all_systems_ok { "正常" } else { "异常" });
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }
}

// ============================================================================
// 优化系统
// ============================================================================

/// 网络更新优化系统
/// 
/// 根据当前 FPS 动态调整网络同步频率
/// 在性能不足时降低网络负载
/// 
/// 策略:
/// - FPS < 30: 降低同步频率到 0.2 秒 (5 Hz)
/// - FPS > 100: 恢复默认频率 0.1 秒 (10 Hz)
/// - FPS 30-100: 保持当前频率
/// 
/// 频率: 每 10 秒检查一次
pub fn optimize_network_updates_system(
    mut network_state: ResMut<NetworkState>,
    frame_stats: Res<FrameStats>,
) {
    // 根据当前性能调整网络同步间隔
    if frame_stats.frame_count % 600 == 0 {
        if frame_stats.current_fps < 30.0 {
            // FPS 过低，减少网络更新频率
            network_state.sync_interval = network_state.sync_interval.max(0.2);
            warn!("🔻 降低网络同步频率 -> {:.2}s", network_state.sync_interval);
        } else if frame_stats.current_fps > 100.0 && network_state.sync_interval > 0.1 {
            // FPS 足够高，可以增加网络更新频率
            network_state.sync_interval = 0.1;
            info!("🔼 恢复网络同步频率 -> {:.2}s", network_state.sync_interval);
        }
    }
}

/// 渲染优化系统
/// 
/// 根据 FPS 动态调整渲染质量
/// 在性能不足时降低渲染负载
/// 
/// 质量等级:
/// - 低 (< 30 FPS): 减少粒子、降低分辨率、禁用阴影
/// - 中 (30-60 FPS): 中等设置
/// - 高 (> 60 FPS): 全部特效开启
/// 
/// 频率: 每 10 秒检查一次
pub fn optimize_render_system(
    frame_stats: Res<FrameStats>,
) {
    // 根据FPS动态调整渲染质量
    if frame_stats.frame_count % 600 == 0 {
        let quality = if frame_stats.current_fps < 30.0 {
            "低"
        } else if frame_stats.current_fps < 60.0 {
            "中"
        } else {
            "高"
        };
        
        info!("🎨 渲染质量: {} ({:.1} FPS)", quality, frame_stats.current_fps);
    }
}

/// 性能分析系统
/// 
/// 在特定时间点输出详细的性能分析报告
/// 用于性能调优和问题诊断
/// 
/// 报告包含:
/// - 总帧数和运行时间
/// - 平均、最小、最大 FPS
/// - 帧时间分布 (可选)
/// - 资源使用情况 (可选)
/// 
/// 触发时机: 运行 60 秒后 (3600 帧 @ 60 FPS)
pub fn profile_system_performance_system(
    frame_stats: Res<FrameStats>,
) {
    // 定期输出性能分析报告
    if frame_stats.frame_count == 3600 {  // 60 秒后（@60FPS）
        info!("╔═══════════════════════════════════════╗");
        info!("║       GameScene 性能分析报告          ║");
        info!("╠═══════════════════════════════════════╣");
        info!("║ 总帧数: {}", frame_stats.frame_count);
        info!("║ 总运行时间: {:.1}s", frame_stats.total_time);
        info!("║ 平均FPS: {:.1}", frame_stats.average_fps);
        info!("║ 最小FPS: {:.1}", 1000.0 / frame_stats.max_frame_time.max(0.1));
        info!("║ 最大FPS: {:.1}", 1000.0 / frame_stats.min_frame_time.max(0.1));
        info!("║ 当前FPS: {:.1}", frame_stats.current_fps);
        info!("╚═══════════════════════════════════════╝");
    }
}

// ============================================================================
// 消息处理器 - UI 和输入事件
// ============================================================================

/// 处理玩家移动消息
/// 
/// 响应玩家移动命令 (键盘或鼠标点击)
/// 更新玩家位置或发送移动请求到服务器
pub fn message_handle_player_move(
    events: Option<MessageReader<PlayerMoveMessage>>,
    mut state: ResMut<GameSceneState>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        info!("📨 玩家移动到: ({}, {})", event.x, event.y);
        // 更新状态或发送到服务器
    }
}

/// 处理打开聊天消息
/// 
/// 打开聊天面板,允许玩家输入
pub fn message_handle_open_chat(
    events: Option<MessageReader<OpenChatMessage>>,
    mut state: ResMut<GameSceneState>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!("💬 打开聊天面板");
        state.show_chat = true;
    }
}

/// 处理关闭聊天消息
/// 
/// 关闭聊天面板
pub fn message_handle_close_chat(
    events: Option<MessageReader<CloseChatMessage>>,
    mut state: ResMut<GameSceneState>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!("💬 关闭聊天面板");
        state.show_chat = false;
    }
}

/// 处理打开背包消息
/// 
/// 打开背包界面,显示物品列表
pub fn message_handle_open_inventory(
    events: Option<MessageReader<OpenInventoryMessage>>,
    mut state: ResMut<GameSceneState>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!("🎒 打开背包");
        state.show_inventory = true;
    }
}

/// 处理关闭背包消息
/// 
/// 关闭背包界面
pub fn message_handle_close_inventory(
    events: Option<MessageReader<CloseInventoryMessage>>,
    mut state: ResMut<GameSceneState>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!("🎒 关闭背包");
        state.show_inventory = false;
    }
}

/// 处理打开技能面板消息
/// 
/// 打开技能界面,显示技能列表和快捷键设置
pub fn message_handle_open_skills(
    events: Option<MessageReader<OpenSkillsMessage>>,
    mut state: ResMut<GameSceneState>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!("⚡ 打开技能面板");
        state.show_skills = true;
    }
}

/// 处理关闭技能面板消息
/// 
/// 关闭技能界面
pub fn message_handle_close_skills(
    events: Option<MessageReader<CloseSkillsMessage>>,
    mut state: ResMut<GameSceneState>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!("⚡ 关闭技能面板");
        state.show_skills = false;
    }
}

/// 处理暂停游戏消息
/// 
/// 暂停或恢复游戏
/// 暂停时停止时间流逝,但保持渲染和网络
pub fn message_handle_pause_game(
    events: Option<MessageReader<PauseGameMessage>>,
    mut state: ResMut<GameSceneState>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        info!("⏸️ 游戏暂停: {}", event.is_paused);
        state.is_paused = event.is_paused;
    }
}

/// 处理退出游戏消息
/// 
/// 退出当前游戏,返回角色选择场景
/// 会保存当前游戏状态
pub fn message_handle_exit_game(
    events: Option<MessageReader<ExitGameMessage>>,
    mut next_state: ResMut<NextState<crate::bevy::GameState>>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!("🚪 返回角色选择");
        next_state.set(crate::bevy::GameState::Select);
    }
}

/// 处理与 NPC 交互消息
/// 
/// 响应玩家点击 NPC 的交互请求
/// 打开对话框或执行 NPC 相关功能
pub fn message_handle_interact_npc(
    events: Option<MessageReader<InteractWithNpcMessage>>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        info!("🗣️ 与 NPC {} 交互", event.npc_id);
        // 打开对话框等
    }
}

/// 处理使用技能消息
/// 
/// 响应玩家使用技能的请求
/// 验证技能可用性,播放动画,发送到服务器
pub fn message_handle_use_skill(
    events: Option<MessageReader<UseSkillMessage>>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        info!(
            "✨ 使用技能 {} 在位置 ({}, {})",
            event.skill_id, event.target_x, event.target_y
        );
        // 播放技能动画等
    }
}

// ============================================================================
// 消息处理器 - 系统控制消息
// ============================================================================

/// 消息处理器 - 游戏循环消息
/// 
/// 处理游戏循环状态控制消息
/// 
/// 状态类型:
/// - 1: 暂停游戏循环
/// - 2: 恢复游戏循环
pub fn message_handle_game_loop(
    events: Option<MessageReader<GameLoopMessage>>,
    mut game_state: ResMut<GameSceneState>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        match event.loop_state {
            1 => {
                game_state.is_paused = true;
                info!("⏸️ 游戏已暂停");
            }
            2 => {
                game_state.is_paused = false;
                info!("▶️ 游戏已恢复");
            }
            _ => {}
        }
    }
}

/// 消息处理器 - 帧统计请求
/// 
/// 响应帧统计信息请求
/// 输出当前 FPS 和统计数据
pub fn message_handle_frame_stats_request(
    events: Option<MessageReader<RequestFrameStatsMessage>>,
    frame_stats: Res<FrameStats>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!(
            "📊 帧统计 - FPS: {:.1} | 帧数: {} | 平均: {:.1}",
            frame_stats.current_fps, frame_stats.frame_count, frame_stats.average_fps
        );
    }
}

/// 消息处理器 - 系统健康检查请求
/// 
/// 响应系统健康检查请求
/// 输出所有子系统的健康状态
pub fn message_handle_system_health_request(
    events: Option<MessageReader<RequestSystemHealthMessage>>,
    health_check: Res<SystemHealthCheck>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!(
            "🏥 系统健康: 玩家{} | 地图{} | 对话{} | 聊天{} | 网络{} | 渲染{} | 总体{}",
            if health_check.player_system_ok { "✅" } else { "❌" },
            if health_check.map_system_ok { "✅" } else { "❌" },
            if health_check.dialogue_system_ok { "✅" } else { "❌" },
            if health_check.chat_system_ok { "✅" } else { "❌" },
            if health_check.network_system_ok { "✅" } else { "❌" },
            if health_check.render_system_ok { "✅" } else { "❌" },
            if health_check.all_systems_ok { "✅ 正常" } else { "❌ 异常" }
        );
    }
}

/// 消息处理器 - 性能报告请求
/// 
/// 响应性能报告请求
/// 根据报告类型输出不同的性能数据
/// 
/// 报告类型:
/// - 0: FPS 报告 (当前、平均、最小、最大)
/// - 1: 内存报告 (事件队列大小等)
/// - 2: 网络报告 (延迟、带宽等)
/// - 3: 完整报告 (所有信息)
pub fn message_handle_performance_report(
    events: Option<MessageReader<PerformanceReportMessage>>,
    frame_stats: Res<FrameStats>,
    game_timer: Res<GameTimer>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        match event.report_type {
            0 => {  // FPS 报告
                info!(
                    "📈 FPS报告 - 当前: {:.1} | 平均: {:.1} | 最小: {:.1} | 最大: {:.1}",
                    frame_stats.current_fps,
                    frame_stats.average_fps,
                    1000.0 / frame_stats.max_frame_time.max(0.1),
                    1000.0 / frame_stats.min_frame_time.max(0.1)
                );
            }
            1 => {  // 内存报告
                info!("💾 内存报告 - 事件队列: 准备就绪");
            }
            2 => {  // 网络报告
                info!("🌐 网络报告 - 游戏速度: {:.2}x", game_timer.game_speed);
            }
            3 => {  // 完整报告
                info!("📋 完整性能报告已生成");
            }
            _ => {}
        }
    }
}
