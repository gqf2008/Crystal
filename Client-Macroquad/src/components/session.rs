// ============================================================================
// Session / Entry Flow State - 会话与进场状态
// ============================================================================
//
// 说明：
// - StartGame* 相关包是“进场链路”关键环节，但 EventBus 是帧内有效。
// - 这里用一个持久化状态把关键结果保留下来，供场景/系统在后续帧读取。

/// 会话状态（持久化）
#[derive(Debug, Clone)]
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

    /// 是否将本地移动“意图”同步到服务器（不代表服务器权威移动）。
    ///
    /// 用途：
    /// - MockServer 需要知道本地玩家格子位置，才能基于 AttackRequest 的方向判定命中。
    /// - 但我们又不想开启 server_authoritative_movement 造成“本地移动 + 服务器纠偏”的双驱动抖动。
    pub sync_movement_intent_to_server: bool,

    /// 是否启用战斗结果服务器权威：
    /// - 客户端仅发送 AttackRequest/MagicRequest 意图
    /// - 命中/死亡等结果由服务器（或 MockServer）推送
    pub server_authoritative_combat: bool,

    /// 是否启用本地玩家 AI 控制（自动找怪/靠近/攻击）。
    ///
    /// 说明：
    /// - 该开关只影响 LocalPlayerAiSystem 是否写入 PlayerInput。
    /// - 关闭时会尽量清理 AI 留下的 move_to/attack_target，避免“关了还在走/追砍”。
    pub local_player_ai_enabled: bool,

    /// 远程玩家走路插值时长（秒）
    pub remote_player_walk_interp_secs: f32,

    /// 远程玩家跑路插值时长（秒）
    pub remote_player_run_interp_secs: f32,
}

impl Default for SessionState {
    fn default() -> Self {
        let env_truthy = |key: &str| -> bool {
            std::env::var(key)
                .ok()
                .map(|v| {
                    let v = v.trim().to_ascii_lowercase();
                    v == "1" || v == "true" || v == "yes" || v == "on"
                })
                .unwrap_or(false)
        };

        Self {
            start_game_delay_ms: None,
            start_game_banned: None,
            start_game_result: None,

            server_authoritative_movement: false,
            sync_movement_intent_to_server: false,
            server_authoritative_combat: false,

            // 默认手动控制；F8 可切到“挂机/自动战斗(AT/BT)”模式。
            // 若需要录制/调试 AI，可用环境变量自动开启：
            // - CRYSTAL_AI_AUTO=1
            // - 或 CRYSTAL_AI_LOG=1（通常配合日志一起用）
            local_player_ai_enabled: env_truthy("CRYSTAL_AI_AUTO") || env_truthy("CRYSTAL_AI_LOG"),

            // 默认值与当前手感保持一致
            remote_player_walk_interp_secs: 0.16,
            remote_player_run_interp_secs: 0.11,
        }
    }
}
