// ============================================================================
// Layer 2: 核心逻辑层 - 校正系统
// ============================================================================
// 职责：比较客户端预测与服务器权威状态，校正误差
// 
// 工作流程：
// 1. 读取 ServerStateComponent（由 ClientNetworkSystem 写入）
// 2. 读取 PredictionComponent（由 LocalPredictionSystem 写入）
// 3. 计算误差：|predicted_pos - server_pos|
// 4. 如果误差 > 阈值（50px），则平滑插值到正确位置
//
// 设计原则：
// - 服务器权威：Server says truth
// - 误差容忍：小误差忽略（避免抖动）
// - 平滑校正：大误差用插值（避免瞬移）
// ============================================================================

use hecs::World;
use crate::ecs::components::{
    LocalPlayer,
    Position,
    prediction::{PredictionComponent, ServerStateComponent, InterpolationComponent},
};

pub struct ReconciliationSystem;

impl ReconciliationSystem {
    pub fn new() -> Self {
        Self
    }

    /// 🎯 Layer 2 核心：服务器校正系统
    /// 
    /// 执行顺序：在 ClientNetworkSystem 接收服务器数据之后
    /// 
    /// 数据流：
    /// - 读取：ServerStateComponent（服务器权威位置）、PredictionComponent（客户端预测）
    /// - 写入：Position（校正后的位置）、InterpolationComponent（平滑插值）
    pub fn update(world: &mut World, _dt: f32) {
        for (_entity, (position, server_state, prediction, mut interpolation)) in world
            .query_mut::<(
                &mut Position,
                &ServerStateComponent,
                &mut PredictionComponent,
                Option<&mut InterpolationComponent>,
            )>()
            .with::<&LocalPlayer>() // 只校正本地玩家
        {
            // 1️⃣ 计算预测误差
            let error_x = server_state.position.x - prediction.predicted_position.x;
            let error_y = server_state.position.y - prediction.predicted_position.y;
            let error_distance = (error_x * error_x + error_y * error_y).sqrt();

            // 2️⃣ 检查是否超过误差阈值
            if error_distance > prediction.error_threshold {
                tracing::warn!(
                    "[ReconciliationSystem] ⚠️ 预测误差过大: {:.2}px (阈值: {:.2}px)",
                    error_distance,
                    prediction.error_threshold
                );

                // 3️⃣ 立即校正位置（或启动平滑插值）
                if error_distance > 200.0 {
                    // 误差太大，直接瞬移（可能是网络卡顿或作弊检测）
                    *position = server_state.position.clone();
                    tracing::error!(
                        "[ReconciliationSystem] 🚨 误差超过200px，强制校正到服务器位置: ({:.1}, {:.1})",
                        server_state.position.x,
                        server_state.position.y
                    );
                } else {
                    // 启动平滑插值（200ms内校正）
                    if let Some(interpolation) = interpolation.as_deref_mut() {
                        interpolation.start_interpolation(
                            position.clone(),
                            server_state.position.clone(),
                            0.2, // 200ms平滑校正
                        );
                        tracing::info!(
                            "[ReconciliationSystem] 🔄 启动平滑校正: {:.2}px误差, 耗时200ms",
                            error_distance
                        );
                    }
                }

                // 4️⃣ 更新预测基准
                prediction.predicted_position = server_state.position.clone();
                // needs_reconciliation 是一个方法，不需要设置
            } else {
                // 误差在可接受范围内，无需校正
                if error_distance > 10.0 {
                    tracing::debug!(
                        "[ReconciliationSystem] ✅ 预测准确: {:.2}px误差 (低于阈值)",
                        error_distance
                    );
                }
            }

            // 5️⃣ 检查序列号（防止过时数据覆盖）
            if server_state.sequence_number < prediction.last_input_sequence {
                tracing::warn!(
                    "[ReconciliationSystem] 🕰️ 收到过时的服务器数据: server_seq={}, client_seq={}",
                    server_state.sequence_number,
                    prediction.last_input_sequence
                );
                // 忽略过时数据
                continue;
            }
        }
    }

    /// 辅助方法：强制将玩家位置同步到服务器位置（用于传送等特殊情况）
    #[allow(dead_code)]
    pub fn force_sync_to_server(world: &mut World, entity: hecs::Entity) {
        if let Ok((mut position, server_state, mut prediction)) = world.query_one_mut::<(
            &mut Position,
            &ServerStateComponent,
            &mut PredictionComponent,
        )>(entity) {
            *position = server_state.position.clone();
            prediction.predicted_position = server_state.position.clone();
            // needs_reconciliation 是一个方法，不需要设置
            
            tracing::info!(
                "[ReconciliationSystem] 🔄 强制同步到服务器位置: ({:.1}, {:.1})",
                server_state.position.x,
                server_state.position.y
            );
        }
    }
}
