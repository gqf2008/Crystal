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
use std::collections::VecDeque;
use crate::ecs::components::Position;

/// 预测的位置记录
#[derive(Debug, Clone)]
pub struct PredictedPosition {
    /// 位置
    pub position: (i32, i32),
    /// 时间戳(毫秒)
    pub timestamp: u64,
    /// 输入序列号
    pub sequence: u32,
}

impl PredictedPosition {
    pub fn new(position: (i32, i32), timestamp: u64, sequence: u32) -> Self {
        Self {
            position,
            timestamp,
            sequence,
        }
    }
}

/// 预测状态组件 (用于客户端预测系统)
#[derive(Debug, Clone)]
pub struct PredictionState {
    /// 预测位置历史
    pub predicted_positions: VecDeque<PredictedPosition>,
    /// 当前修正目标
    pub correction_target: Option<(i32, i32)>,
    /// 预测权重 (0.0-1.0, 根据延迟调整)
    pub prediction_weight: f32,
    /// 最大历史记录数
    pub max_history: usize,
    /// 修正速度 (0.0-1.0)
    pub correction_speed: f32,
}

impl PredictionState {
    pub fn new() -> Self {
        Self {
            predicted_positions: VecDeque::new(),
            correction_target: None,
            prediction_weight: 1.0,
            max_history: 10,
            correction_speed: 0.3,
        }
    }

    /// 添加预测位置
    pub fn add_predicted_position(&mut self, position: (i32, i32), timestamp: u64, sequence: u32) {
        // 添加新记录
        self.predicted_positions.push_back(PredictedPosition::new(position, timestamp, sequence));
        
        // 限制历史记录数量
        while self.predicted_positions.len() > self.max_history {
            self.predicted_positions.pop_front();
        }
    }

    /// 获取最近的预测位置
    pub fn latest_predicted_position(&self) -> Option<&PredictedPosition> {
        self.predicted_positions.back()
    }

    /// 清理旧的预测记录
    pub fn cleanup_old_predictions(&mut self, current_time: u64, max_age_ms: u64) {
        self.predicted_positions.retain(|p| current_time - p.timestamp <= max_age_ms);
    }

    /// 设置修正目标
    pub fn set_correction_target(&mut self, target: (i32, i32)) {
        self.correction_target = Some(target);
    }

    /// 清除修正目标
    pub fn clear_correction_target(&mut self) {
        self.correction_target = None;
    }

    /// 根据延迟调整预测权重
    pub fn adjust_prediction_weight(&mut self, latency_ms: u64) {
        self.prediction_weight = if latency_ms > 200 {
            0.5 // 高延迟,降低预测权重
        } else if latency_ms > 100 {
            0.7
        } else {
            1.0 // 低延迟,完全信任预测
        };
    }
}

impl Default for PredictionState {
    fn default() -> Self {
        Self::new()
    }
}

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

/// 服务器状态组件 - 存储服务器权威状态
#[derive(Debug, Clone)]
pub struct ServerState {
    /// 服务器位置
    pub position: Position,
    
    /// 服务器方向
    pub direction: u8,
    
    /// 最后更新时间
    pub last_update_time: Instant,
    
    /// 接收序列号（用于检测过期数据）
    pub sequence_number: u32,
}

impl ServerState {
    pub fn new(position: Position, direction: u8) -> Self {
        Self {
            position,
            direction,
            last_update_time: Instant::now(),
            sequence_number: 0,
        }
    }
    
    /// 更新服务器状态
    pub fn update(&mut self, position: Position, direction: u8, sequence: u32) {
        // 只接受更新的状态（防止过期数据）
        if sequence >= self.sequence_number {
            self.position = position;
            self.direction = direction;
            self.sequence_number = sequence;
            self.last_update_time = Instant::now();
        }
    }
    
    /// 获取延迟（毫秒）
    pub fn get_latency_ms(&self) -> u128 {
        self.last_update_time.elapsed().as_millis()
    }
}

/// 插值组件 - 用于平滑其他玩家的移动
#[derive(Debug, Clone)]
pub struct Interpolation {
    /// 起始位置
    pub from_position: Position,
    
    /// 目标位置
    pub to_position: Position,
    
    /// 插值进度 (0.0 - 1.0)
    pub progress: f32,
    
    /// 插值持续时间（秒）
    pub duration: f32,
    
    /// 插值开始时间
    pub start_time: Instant,
    
    /// 是否正在插值
    pub is_active: bool,
}

impl Interpolation {
    pub fn new() -> Self {
        Self {
            from_position: Position { x: 0.0, y: 0.0 },
            to_position: Position { x: 0.0, y: 0.0 },
            progress: 0.0,
            duration: 0.1, // 默认100ms插值
            start_time: Instant::now(),
            is_active: false,
        }
    }
    
    /// 开始新的插值
    pub fn start_interpolation(&mut self, from: Position, to: Position, duration: f32) {
        self.from_position = from;
        self.to_position = to;
        self.progress = 0.0;
        self.duration = duration;
        self.start_time = Instant::now();
        self.is_active = true;
    }
    
    /// 更新插值进度
    pub fn update(&mut self, delta_time: f32) -> Option<Position> {
        if !self.is_active {
            return None;
        }
        
        self.progress += delta_time / self.duration;
        
        if self.progress >= 1.0 {
            self.progress = 1.0;
            self.is_active = false;
            return Some(self.to_position.clone());
        }
        
        // 线性插值
        let t = self.progress;
        Some(Position {
            x: self.from_position.x + (self.to_position.x - self.from_position.x) * t,
            y: self.from_position.y + (self.to_position.y - self.from_position.y) * t,
        })
    }
    
    /// 停止插值
    pub fn stop(&mut self) {
        self.is_active = false;
        self.progress = 1.0;
    }
}

impl Default for Interpolation {
    fn default() -> Self {
        Self::new()
    }
}
