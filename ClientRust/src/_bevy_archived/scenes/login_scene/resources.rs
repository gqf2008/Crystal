// LoginScene Resources
// 场景状态和资源定义

use bevy::prelude::*;
use std::collections::HashMap;
use super::components::DialogFieldType;

// ============================================================================
// Dialog Types
// ============================================================================

/// 对话框类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogType {
    /// 无对话框
    None,
    /// 新账号对话框
    NewAccount,
    /// 修改密码对话框
    ChangePassword,
}

// ============================================================================
// Login State Resource
// ============================================================================

/// 登录场景主状态资源
/// 
/// 包含网络连接、版本检查、登录状态、动画状态、输入验证等所有状态
#[derive(Resource, Debug)]
pub struct LoginState {
    // ========== Network State ==========
    /// 是否正在连接服务器
    pub connecting: bool,
    
    /// 连接尝试次数
    pub connect_attempts: u32,
    
    // ========== Version Check State ==========
    /// 是否已完成版本检查
    pub version_checked: bool,
    
    /// 版本是否有效
    pub version_valid: bool,
    
    // ========== Login State ==========
    /// 登录按钮是否启用 (账号密码都有效时才启用)
    pub login_enabled: bool,
    
    /// 登录是否成功
    pub login_success: bool,
    
    /// 登录成功后经过的帧数 (用于延迟场景切换)
    pub frames_after_login: usize,
    
    // ========== Background Animation State ==========
    /// 当前背景动画帧索引 (0-18, 对应 Prguse 1-19)
    pub background_frame: usize,
    
    /// 动画计时器累计时间
    pub animation_timer: f32,
    
    /// 动画是否暂停 (登录前暂停,登录成功后播放)
    pub animation_paused: bool,
    
    // ========== Input Values ==========
    /// 账号ID输入值
    pub account_id: String,
    
    /// 密码输入值
    pub password: String,
    
    // ========== Input Validation ==========
    /// 账号ID是否有效 (长度符合要求)
    pub account_id_valid: bool,
    
    /// 密码是否有效 (长度符合要求)
    pub password_valid: bool,
    
    // ========== Dialog State ==========
    /// 当前显示的对话框类型
    pub dialog_visible: DialogType,
    
    /// 对话框输入字段值 (key: 字段类型, value: 输入值)
    pub dialog_inputs: HashMap<DialogFieldType, String>,
    
    // ========== Network Command Sender ==========
    /// 网络命令发送器 (用于向网络线程发送登录请求)
    pub command_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>>,
}

impl Default for LoginState {
    fn default() -> Self {
        Self {
            // Network
            connecting: false,
            connect_attempts: 0,
            
            // Version Check (跳过版本检查用于测试)
            version_checked: true,
            version_valid: true,
            
            // Login
            login_enabled: false,
            login_success: false,
            frames_after_login: 0,
            
            // Animation (启动时暂停,登录成功后才播放)
            background_frame: 0,
            animation_timer: 0.0,
            animation_paused: true,
            
            // Input
            account_id: String::new(),
            password: String::new(),
            
            // Validation
            account_id_valid: false,
            password_valid: false,
            
            // Dialog
            dialog_visible: DialogType::None,
            dialog_inputs: HashMap::new(),
            
            // Network
            command_tx: None,
        }
    }
}

impl LoginState {
    /// 设置网络命令发送器,用于向网络线程发送登录请求
    pub fn set_command_sender(&mut self, tx: tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>) {
        self.command_tx = Some(tx);
    }
}
