// ============================================================================
// Session / Entry Flow State - 会话与进场状态
// ============================================================================
//
// 说明：
// - StartGame* 相关包是“进场链路”关键环节，但 EventBus 是帧内有效。
// - 这里用一个持久化状态把关键结果保留下来，供场景/系统在后续帧读取。

/// 会话状态（持久化）
#[derive(Debug, Default, Clone)]
pub struct SessionState {
    /// StartGameDelay: 服务器要求延迟登录（毫秒）
    pub start_game_delay_ms: Option<i64>,

    /// StartGameBanned: (reason, expiry_date_binary)
    pub start_game_banned: Option<(String, i64)>,

    /// StartGame: (result, resolution)
    pub start_game_result: Option<(u8, i32)>,

    /// 是否启用本地玩家完全服务器权威移动：
    /// - 客户端只发 Walk/Run/Turn 请求
    /// - 位置以服务器回包（UserLocation → PlayerLocationChanged）为准
    pub server_authoritative_movement: bool,

    /// 是否启用战斗结果服务器权威：
    /// - 客户端仅发送 AttackRequest/MagicRequest 意图
    /// - 命中/死亡等结果由服务器（或 MockServer）推送
    pub server_authoritative_combat: bool,
}
