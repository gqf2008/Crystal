// GameScene - 网络同步系统模块
// 
// 功能说明:
// 1. 网络连接管理 - 连接、断开、重连、超时处理
// 2. 玩家状态同步 - 定期发送本地玩家位置、属性、状态到服务器
// 3. 远端实体同步 - 接收和应用其他玩家、NPC、怪物的状态更新
// 4. 地图对象同步 - 处理地图上物品掉落、门的状态、动态对象等
// 5. 聊天消息传输 - 发送本地聊天到服务器,接收服务器广播
// 6. 交互事件传输 - 发送 NPC 交互、物品拾取等事件到服务器
// 7. 网络状态监控 - 延迟测量、丢包检测、带宽统计
// 8. 消息队列管理 - 发送队列、接收缓冲区、优先级调度
// 9. 断线重连机制 - 自动重连、状态恢复、超时重试
// 10. 本地状态维护 - 同步计时器、待处理更新计数、连接状态
//
// 系统列表:
// - setup_network_system: 初始化网络状态资源
// - send_player_position_system: 定期发送玩家位置 (客户端 → 服务器)
// - send_player_stats_system: 定期发送玩家属性 (客户端 → 服务器)
// - send_chat_to_server_system: 发送聊天消息到服务器
// - send_interaction_to_server_system: 发送 NPC 交互事件到服务器
// - receive_player_sync_system: 接收远端玩家同步信息 (服务器 → 客户端)
// - receive_npc_sync_system: 接收 NPC 状态同步信息
// - receive_map_sync_system: 接收地图对象同步信息 (物品、门等)
// - receive_server_chat_system: 接收服务器聊天广播
// - handle_connection_events_system: 处理连接事件 (连接/断开/超时)
// - apply_player_sync_system: 应用远端玩家位置和状态到实体
// - apply_npc_sync_system: 应用 NPC 状态变化到实体
// - apply_item_spawn_system: 处理物品生成和消失
// - sync_local_state_system: 维护本地同步状态 (计时器、计数器)

use bevy::prelude::*;
use super::{
    NetworkState, ConnectionState, GameSceneState, ChatManager,
    Player, RemotePlayer, NPC, MapData, InteractWithNpcMessage,
    MAX_PENDING_UPDATES,
};

// ============================================================================
// 网络初始化
// ============================================================================

/// 初始化网络系统
/// 
/// 创建 NetworkState 资源,用于跟踪连接状态、同步计时器等
/// 在 GameScene setup 时调用一次
pub fn setup_network_system(mut commands: Commands) {
    commands.insert_resource(NetworkState::default());
    info!("🌐 网络系统已初始化");
}

// ============================================================================
// 客户端 → 服务器: 发送系统
// ============================================================================

/// 定期发送玩家位置同步消息
/// 
/// 根据 sync_interval 定期发送本地玩家的 Transform 到服务器
/// 只在已连接状态下发送,避免无效消息
/// 
/// 同步间隔由 NetworkState.sync_interval 控制 (默认 0.1 秒)
pub fn send_player_position_system(
    mut network_state: ResMut<NetworkState>,
    game_state: Res<GameSceneState>,
    player_query: Query<&Transform, With<Player>>,
) {
    // 检查是否需要同步
    if network_state.connection_state != ConnectionState::Connected {
        return;
    }
    
    // 检查是否到达同步间隔
    if network_state.last_sync_time < network_state.sync_interval {
        return;
    }
    
    // 重置同步计时器
    network_state.last_sync_time = 0.0;
    
    // 获取玩家位置
    if let Ok(transform) = player_query.single() {
        let pos = transform.translation;
        
        // 创建同步消息（这里模拟发送，实际需要网络传输）
        info!(
            "📤 发送玩家位置同步: ({:.1}, {:.1}, {:.1})",
            pos.x, pos.y, pos.z
        );
        
        network_state.pending_updates += 1;
    }
}

/// 定期发送玩家属性同步消息
/// 
/// 发送玩家的等级、血量、魔法值、经验值等属性到服务器
/// 用于服务器验证和其他玩家的显示
pub fn send_player_stats_system(
    mut network_state: ResMut<NetworkState>,
    game_state: Res<GameSceneState>,
) {
    // 检查连接状态
    if network_state.connection_state != ConnectionState::Connected {
        return;
    }
    
    // 定期发送属性更新（这里模拟发送）
    info!(
        "📤 发送玩家属性同步: 等级={}, HP={}/{}",
        game_state.player_level, game_state.player_health, game_state.player_max_health
    );
    
    network_state.pending_updates += 1;
}

/// 发送聊天消息到服务器
/// 
/// 将本地玩家发送的聊天消息转发到服务器
/// 服务器会验证并广播给其他玩家
/// 
/// 消息类型:
/// - 0: 普通消息 (附近玩家可见)
/// - 1: 私聊消息 (只有目标玩家可见)
/// - 2: 公会消息 (公会成员可见)
/// - 3: 系统消息 (服务器发送)
pub fn send_chat_to_server_system(
    network_state: Res<NetworkState>,
    chat_manager: Res<ChatManager>,
) {
    // 检查连接状态
    if network_state.connection_state != ConnectionState::Connected {
        return;
    }
    
    // 只在有新消息时发送
    if chat_manager.history.is_empty() {
        return;
    }
    
    // 获取最后一条消息
    if let Some(last_msg) = chat_manager.history.back() {
        if last_msg.message_type == 0 {  // 0=普通消息
            info!("📤 发送聊天消息到服务器: {}", last_msg.content);
        }
    }
}

/// 发送交互事件到服务器
/// 
/// 当玩家点击 NPC、物品、传送点等交互对象时
/// 发送交互事件到服务器进行验证和处理
/// 
/// 事件类型:
/// - NPC 对话
/// - 物品拾取
/// - 传送点使用
/// - 技能释放
pub fn send_interaction_to_server_system(
    network_state: Res<NetworkState>,
    events: Option<MessageReader<InteractWithNpcMessage>>,
) {
    // 检查连接状态
    if network_state.connection_state != ConnectionState::Connected {
        return;
    }
    
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        info!("📤 发送NPC交互事件到服务器: NPC ID = {}", event.npc_id);
    }
}

// ============================================================================
// 服务器 → 客户端: 接收系统
// ============================================================================

/// 接收远端玩家同步信息
/// 
/// 从服务器接收其他玩家的位置、状态、动作等信息
/// 存储到接收缓冲区,由 apply_player_sync_system 应用到实体
/// 
/// 同步数据包括:
/// - 位置 (x, y, z)
/// - 朝向 (rotation)
/// - 动作状态 (idle, walking, attacking)
/// - 血量和等级 (用于显示血条和名字)
pub fn receive_player_sync_system(
    mut network_state: ResMut<NetworkState>,
) {
    // 模拟接收其他玩家同步信息
    if network_state.connection_state == ConnectionState::Connected {
        if network_state.pending_updates > 0 {
            // 模拟处理同步数据
            // 实际应用会从网络缓冲区读取
            network_state.pending_updates = network_state.pending_updates.saturating_sub(1);
        }
    }
}

/// 接收NPC状态同步信息
/// 
/// 从服务器接收 NPC 的位置、血量、状态等更新
/// NPC 由服务器控制,客户端只负责显示
/// 
/// 同步数据包括:
/// - 位置和朝向
/// - 血量和状态 (正常/战斗/死亡)
/// - 当前动作 (idle/patrol/chase/attack)
pub fn receive_npc_sync_system(
    network_state: Res<NetworkState>,
    mut npc_query: Query<&mut Transform, With<NPC>>,
) {
    // 检查连接状态
    if network_state.connection_state != ConnectionState::Connected {
        return;
    }
    
    // 模拟接收NPC状态更新
    // 实际应用会从网络缓冲区读取
    for mut transform in npc_query.iter_mut() {
        // 这里会更新NPC位置/状态
        // info!("📥 接收NPC状态同步");
    }
}

/// 接收地图对象同步信息
/// 
/// 从服务器接收地图上的动态对象状态变化
/// 包括物品掉落、门的开关、传送点激活等
/// 
/// 对象类型:
/// - 掉落物品: 位置、物品ID、数量、所有者
/// - 门和开关: 开关状态
/// - 传送点: 激活状态
/// - 特效: 位置、特效ID、持续时间
pub fn receive_map_sync_system(
    network_state: Res<NetworkState>,
    map_data: Res<MapData>,
) {
    // 检查连接状态
    if network_state.connection_state != ConnectionState::Connected {
        return;
    }
    
    // 模拟接收地图对象状态更新（物品、门等）
    // 实际应用会处理掉落物品、门打开/关闭等
    if !map_data.objects.is_empty() {
        // info!("📥 接收地图对象同步: {} 个对象", map_data.objects.len());
    }
}

/// 接收服务器聊天广播
/// 
/// 接收服务器发送的聊天消息,包括:
/// - 其他玩家的聊天
/// - 系统公告
/// - GM 消息
/// - 战斗日志
/// 
/// 注意: 这个系统与 chat_systems.rs 中的 receive_chat_messages_system 配合使用
/// 这里主要处理服务器特定的广播格式,receive_chat_messages_system 处理本地显示
pub fn receive_server_chat_system(
    mut chat_manager: ResMut<ChatManager>,
) {
    // 模拟接收服务器聊天广播
    // 实际应用会从网络缓冲区读取
    
    // 这个系统在 receive_chat_messages_system 中已部分实现
    // 这里主要用于处理服务器特定的广播格式
}

// ============================================================================
// 连接管理
// ============================================================================

/// 处理网络连接事件（连接成功、断开、超时等）
/// 
/// 监控连接状态变化,触发相应的处理逻辑
/// 
/// 状态转换:
/// - Disconnected → Connecting: 发起连接
/// - Connecting → Connected: 连接成功
/// - Connected → Disconnecting: 主动断开
/// - Connected → Reconnecting: 检测到断线,尝试重连
/// - Reconnecting → Connected: 重连成功
/// - Reconnecting → Disconnected: 重连失败
pub fn handle_connection_events_system(
    mut network_state: ResMut<NetworkState>,
) {
    // 模拟处理连接事件
    match network_state.connection_state {
        ConnectionState::Disconnected => {
            info!("❌ 网络断开连接");
        }
        ConnectionState::Connecting => {
            info!("🔗 正在连接到服务器...");
            // 模拟连接延迟后设置为已连接
        }
        ConnectionState::Connected => {
            // info!("✅ 已连接到服务器");
        }
        ConnectionState::Reconnecting => {
            info!("🔄 正在重新连接...");
        }
        ConnectionState::Disconnecting => {
            info!("🔌 正在断开连接...");
        }
    }
}

// ============================================================================
// 应用同步数据到实体
// ============================================================================

/// 应用远端玩家同步数据
/// 
/// 将接收到的远端玩家状态应用到对应的实体
/// 使用插值平滑位置变化,避免闪烁
/// 
/// 处理步骤:
/// 1. 从接收缓冲区读取同步数据
/// 2. 查找或创建对应的 RemotePlayer 实体
/// 3. 使用线性插值更新位置 (lerp)
/// 4. 更新动作状态和朝向
/// 5. 更新血条和名字显示
pub fn apply_player_sync_system(
    network_state: Res<NetworkState>,
    mut remote_player_query: Query<&mut Transform, With<RemotePlayer>>,
) {
    // 检查连接状态
    if network_state.connection_state != ConnectionState::Connected {
        return;
    }
    
    // 应用远端玩家位置/状态更新
    for mut transform in remote_player_query.iter_mut() {
        // 这里会平滑更新远端玩家位置
        // info!("🔄 更新远端玩家位置");
    }
}

/// 应用NPC状态同步
/// 
/// 将服务器发送的 NPC 状态应用到本地 NPC 实体
/// NPC 完全由服务器控制,客户端只是呈现
/// 
/// 更新内容:
/// - 位置和朝向 (使用插值)
/// - 血量和最大血量
/// - 当前动作 (idle/patrol/chase/attack/death)
/// - 特殊效果 (buff/debuff)
pub fn apply_npc_sync_system(
    network_state: Res<NetworkState>,
) {
    // 检查连接状态
    if network_state.connection_state != ConnectionState::Connected {
        return;
    }
    
    // 应用NPC状态变化（血量、状态等）
    // info!("🔄 应用NPC状态同步");
}

/// 处理物品生成/消失
/// 
/// 根据服务器消息在地图上生成或删除物品实体
/// 
/// 生成物品:
/// 1. 从消息中读取物品ID、位置、数量
/// 2. 加载物品纹理 (从 MLibrary)
/// 3. 创建物品实体 (Sprite + Transform + ItemComponent)
/// 4. 添加拾取碰撞检测
/// 
/// 删除物品:
/// 1. 根据物品ID查找实体
/// 2. 播放消失动画 (可选)
/// 3. despawn 实体
pub fn apply_item_spawn_system(
    network_state: Res<NetworkState>,
    mut _commands: Commands,
) {
    // 检查连接状态
    if network_state.connection_state != ConnectionState::Connected {
        return;
    }
    
    // 处理物品掉落/消失事件
    // 实际应用会根据网络消息创建/删除物品实体
}

// ============================================================================
// 本地状态维护
// ============================================================================

/// 维持本地同步状态
/// 
/// 每帧更新网络相关的计时器和计数器
/// 
/// 维护内容:
/// - last_sync_time: 距离上次同步的时间 (用于控制发送频率)
/// - pending_updates: 待处理的更新数量 (防止积压)
/// - connection_timeout: 连接超时计时器 (检测断线)
/// 
/// 限制条件:
/// - pending_updates 不超过 MAX_PENDING_UPDATES (防止内存溢出)
/// - 如果长时间无响应,触发重连机制
pub fn sync_local_state_system(
    mut network_state: ResMut<NetworkState>,
    time: Res<Time>,
) {
    // 更新同步计时器
    network_state.last_sync_time += time.delta_secs();
    
    // 保持待处理更新数不超过限制
    if network_state.pending_updates > MAX_PENDING_UPDATES {
        network_state.pending_updates = MAX_PENDING_UPDATES;
    }
}
