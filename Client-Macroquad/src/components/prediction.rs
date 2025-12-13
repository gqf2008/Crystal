// ============================================================================
// Prediction Components - 客户端预测相关组件
// ============================================================================
//
// 用于实现客户端预测和状态调和
// - Prediction: 预测状态
// - ServerState: 服务器权威状态
//
// ============================================================================

use std::time::Instant;
use crate::components::Position;

/// 客户端预测组件
#[derive(Debug, Clone)]
pub struct Prediction {
    /// 客户端预测的位置
    pub predicted_position: Position,
    
    /// 最后一次服务器确认的位置
    pub server_position: Position,
    
    /// 最后一次同步时间
    pub last_sync_time: Instant,
    
    /// 是否正在预测
    pub is_predicting: bool,
    
    /// 预测误差（用于判断是否需要纠正）
    pub error_threshold: f32,
    
    /// 最后一次输入序列号（用于防止过期数据）
    pub last_input_sequence: u32,
}

impl Prediction {
    pub fn new(initial_pos: Position) -> Self {
        Self {
            predicted_position: initial_pos.clone(),
            server_position: initial_pos,
            last_sync_time: Instant::now(),
            is_predicting: false,
            error_threshold: 50.0, // 误差超过50像素则纠正
            last_input_sequence: 0,
        }
    }
    
    /// 计算预测误差
    pub fn calculate_error(&self) -> f32 {
        let dx = self.predicted_position.x - self.server_position.x;
        let dy = self.predicted_position.y - self.server_position.y;
        (dx * dx + dy * dy).sqrt()
    }
    
    /// 是否需要纠正
    pub fn needs_reconciliation(&self) -> bool {
        self.calculate_error() > self.error_threshold
    }
    
    /// 更新服务器权威位置
    pub fn update_server_position(&mut self, pos: Position) {
        self.server_position = pos;
        self.last_sync_time = Instant::now();
    }
    
    /// 更新预测位置
    pub fn update_predicted_position(&mut self, pos: Position) {
        self.predicted_position = pos;
    }
}