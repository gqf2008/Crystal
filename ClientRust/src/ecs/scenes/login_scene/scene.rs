// LoginScene - Login scene implementation (完全ECS架构)
// 移植自 Client/MirScenes/LoginScene.cs
// 
// 架构设计：
// - LoginScene: 场景状态管理器和ECS调度器
// - ECS World: 存储所有UI实体（背景、对话框、按钮、输入框等）
// - Systems: 处理渲染、输入、动画等逻辑
// - Components: 定义UI元素的属性和行为

use mir2_shared::packets::CharacterSummary;
use ggez::{Context, GameResult};
use ggez::graphics::Canvas;
use hecs::World;
use tokio::sync::mpsc;

use super::super::{Scene, SceneType};
use crate::network::{game_client::GameEvent, NetworkCommand};

// ✅ 只引用ECS模块
use super::{
    // ECS系统
    render_all,
    update_animations,
    handle_mouse_move,
    handle_mouse_click,
    handle_char_input,
    handle_backspace,
    handle_tab,
    handle_enter,
    handle_escape,
    handle_input_click,
    // 对话框创建函数
    create_login_dialog,
    create_new_account_dialog,
    create_connecting_box,
    create_message_box,
    create_change_password_dialog,
    // 对话框句柄
    LoginDialogHandle,
    NewAccountDialogHandle,
    ConnectingBoxHandle,
    MessageBoxHandle,
    ChangePasswordDialogHandle,
    // 辅助函数
    get_login_credentials,
    get_registration_data,
    get_change_password_data,
    set_login_dialog_visible,
    // 组件和动作
    ButtonAction,
};

#[derive(Debug, Clone)]
pub struct BanInfo {
    pub reason: String,
    pub expiry_date: i64,
}

/// Login scene state（完全ECS架构）
/// 
/// 职责：
/// 1. 管理场景级别的状态（网络连接、登录状态等）
/// 2. 调度ECS系统（渲染、输入、动画）
/// 3. 处理网络事件和场景切换
/// 4. 不直接处理UI逻辑，所有UI通过ECS系统管理
pub struct LoginScene {
    // ========== 网络层 ==========
    #[allow(dead_code)]
    game_client: Option<crate::network::game_client::SharedGameClient>,
    #[allow(dead_code)]
    command_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>>,
    
    // ========== 场景状态 ==========
    connecting: bool,
    connect_attempts: u32,
    version_checked: bool,
    version_valid: bool,
    login_enabled: bool,
    require_password_change: bool,
    ready_for_character_select: bool,
    
    // ========== 数据记录 ==========
    last_status: Option<String>,
    message_log: Vec<String>,
    last_login_result: Option<u8>,
    last_new_account_result: Option<u8>,
    last_change_password_result: Option<u8>,
    login_ban_info: Option<BanInfo>,
    password_change_ban_info: Option<BanInfo>,
    characters: Vec<CharacterSummary>,
    
    // ========== ECS核心 ==========
    /// ECS World - 存储所有UI实体和组件
    ecs_world: hecs::World,
    
    /// 对话框句柄 - 用于快速访问和操作特定对话框
    login_dialog: Option<LoginDialogHandle>,
    new_account_dialog: Option<NewAccountDialogHandle>,
    change_password_dialog: Option<ChangePasswordDialogHandle>,
    connecting_box: Option<ConnectingBoxHandle>,
    message_box: Option<MessageBoxHandle>,
    
    // ========== 标志位 ==========
    initialized: bool,  // 是否已初始化ECS实体
}

impl LoginScene {
    /// Create new login scene（纯ECS架构）
    pub fn new() -> Self {
        Self {
            // 网络
            game_client: None,
            command_tx: None,
            
            // 状态
            connecting: false,
            connect_attempts: 0,
            version_checked: false,
            version_valid: false,
            login_enabled: false,
            require_password_change: false,
            ready_for_character_select: false,
            
            // 记录
            last_status: None,
            message_log: Vec::new(),
            last_login_result: None,
            last_new_account_result: None,
            last_change_password_result: None,
            login_ban_info: None,
            password_change_ban_info: None,
            characters: Vec::new(),
            
            // ECS
            ecs_world: hecs::World::new(),
            login_dialog: None,
            new_account_dialog: None,
            change_password_dialog: None,
            connecting_box: None,
            message_box: None,
            
            // 标志
            initialized: false,
        }
    }
    
    /// 初始化ECS实体（在第一次update时调用）
    fn initialize_ecs(&mut self) {
        if self.initialized {
            return;
        }
        
        tracing::info!("🎬 初始化LoginScene ECS实体...");
        
        // 创建登录对话框
        self.login_dialog = Some(create_login_dialog(&mut self.ecs_world));
        
        self.initialized = true;
        tracing::info!("✅ LoginScene ECS初始化完成，实体数: {}", self.ecs_world.len());
    }
    
    /// Set game client for network communication
    pub fn set_game_client(&mut self, client: Option<crate::network::game_client::SharedGameClient>) {
        self.game_client = client;
    }
    
    /// Set command sender for network commands
    pub fn set_command_sender(&mut self, tx: Option<tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>>) {
        self.command_tx = tx;
    }
    
    /// Send client version to server for verification
    fn send_client_version(&mut self) {
        use std::fs::File;
        use std::io::Read;
        
        tracing::info!("发送客户端版本验证...");
        self.record_status("Sending client version...");
        
        // 计算可执行文件的 MD5 哈希
        let version_hash = if let Ok(exe_path) = std::env::current_exe() {
            if let Ok(mut file) = File::open(exe_path) {
                let mut buffer = Vec::new();
                if file.read_to_end(&mut buffer).is_ok() {
                    use md5::compute;
                    let digest = compute(&buffer);
                    digest.0.to_vec()
                } else {
                    vec![0u8; 16]
                }
            } else {
                vec![0u8; 16]
            }
        } else {
            vec![0u8; 16]
        };
        
        tracing::info!("✅ 已连接到服务器,ClientVersion包准备完成");
        tracing::info!("版本哈希: {:?}", &version_hash[0..8.min(version_hash.len())]);
        
        // 临时方案: 直接启用登录对话框 (跳过版本验证)
        self.version_checked = true;
        self.version_valid = true;
        self.login_enabled = true;
        self.record_status("Connected! Ready to login.");
        tracing::info!("⚠ 临时跳过客户端版本验证,登录对话框已启用");
    }
    
    /// Load settings（ECS版本）
    pub fn load_settings(&mut self, account_id: String, password: String) {
        if let Some(handle) = &self.login_dialog {
            super::set_login_credentials(&mut self.ecs_world, handle, account_id, password);
        }
    }
    
    pub fn record_status<S: Into<String>>(&mut self, message: S) {
        let message = message.into();
        self.last_status = Some(message.clone());
        self.message_log.push(message);
    }
    
    /// Show a message box（ECS版本）
    pub fn show_message<S: Into<String>>(&mut self, message: S) {
        let msg = message.into();
        
        // 如果已有消息框，先销毁
        if let Some(handle) = self.message_box.take() {
            // 删除消息框相关的所有实体
            let _ = self.ecs_world.despawn(handle.background);
            let _ = self.ecs_world.despawn(handle.ok_button);
        }
        
        // 创建新消息框
        let handle = create_message_box(&mut self.ecs_world, msg.clone());
        self.message_box = Some(handle);
        
        tracing::debug!("📝 显示消息框: {}", msg);
    }
    
    /// Show a message box with custom title（ECS版本）
    pub fn show_message_with_title<S: Into<String>, T: Into<String>>(&mut self, message: S, _title: T) {
        self.show_message(message);
    }

    fn handle_client_version_response(&mut self, result: u8) {
        self.version_checked = true;
        self.connecting = false;
        match result {
            0 => {
                self.version_valid = false;
                self.login_enabled = false;
                self.record_status("Wrong version, please update your game. Connection closed.");
            }
            1 => {
                self.version_valid = true;
                self.login_enabled = true;
                self.record_status("Client version accepted by server. Login dialog unlocked.");
            }
            other => {
                self.version_valid = false;
                self.login_enabled = false;
                self.record_status(format!("Unknown client version response: {}", other));
            }
        }
    }

    fn handle_login_response(&mut self, result: u8) {
        self.last_login_result = Some(result);
        self.connecting = false;
        self.login_enabled = true;
        self.require_password_change = result == 5;
        self.ready_for_character_select = false;
        self.characters.clear();
        
        if let Some(message) = Self::login_result_message(result) {
            self.record_status(message);
            if result != 0 {
                self.show_message(message);
            }
        } else {
            let msg = format!("Unknown login result code {}", result);
            self.record_status(msg.clone());
            self.show_message(msg);
        }
    }

    fn handle_login_success(&mut self, characters: &[CharacterSummary]) {
        self.connecting = false;
        self.login_enabled = false;
        self.version_checked = true;
        self.version_valid = true;
        self.require_password_change = false;
        self.characters = characters.to_vec();
        
        // 隐藏连接提示框
        if let Some(handle) = self.connecting_box.take() {
            let _ = self.ecs_world.despawn(handle.background);
            let _ = self.ecs_world.despawn(handle.cancel_button);
        }
        
        // TODO: 开始播放登录成功动画
        // TODO: 播放登录音效
        
        let status_msg = format!(
            "Login successful. {} character(s) available.",
            self.characters.len()
        );
        self.record_status(status_msg.clone());
        
        tracing::info!("✅ {}", status_msg);
        
        // 暂时直接切换到角色选择场景
        self.ready_for_character_select = true;
    }

    fn handle_login_ban(&mut self, reason: &str, expiry_date: i64) {
        self.connecting = false;
        self.login_enabled = true;
        self.require_password_change = false;
        self.ready_for_character_select = false;
        let info = BanInfo {
            reason: reason.to_string(),
            expiry_date,
        };
        self.login_ban_info = Some(info.clone());
        self.record_status(Self::ban_message("Login", &info));
    }

    fn handle_new_account_response(&mut self, result: u8) {
        self.last_new_account_result = Some(result);
        let message = Self::new_account_result_message(result);
        
        match result {
            0 | 8 => {
                // 成功或失败都关闭对话框
                if let Some(msg) = message {
                    self.show_message(msg);
                }
                self.close_new_account_dialog();
            }
            _ => {
                // 其他错误显示消息但保持对话框打开
                if let Some(msg) = message {
                    self.show_message(msg);
                }
            }
        }
        
        if let Some(msg) = message {
            self.record_status(msg);
        }
    }

    fn handle_change_password_response(&mut self, result: u8) {
        self.last_change_password_result = Some(result);
        let message = Self::change_password_result_message(result);
        
        match result {
            0 | 6 => {
                // 成功或禁用都关闭对话框
                if let Some(msg) = message {
                    self.show_message(msg);
                }
                self.close_change_password_dialog();
            }
            _ => {
                // 其他错误显示消息但保持对话框打开
                if let Some(msg) = message {
                    self.show_message(msg);
                }
            }
        }
        
        if let Some(msg) = message {
            self.record_status(msg);
        }
    }

    fn handle_change_password_ban(&mut self, reason: &str, expiry_date: i64) {
        let info = BanInfo {
            reason: reason.to_string(),
            expiry_date,
        };
        self.password_change_ban_info = Some(info.clone());
        self.record_status(Self::ban_message("Password change", &info));
    }

    fn login_result_message(result: u8) -> Option<&'static str> {
        match result {
            0 => Some("Logging in is currently disabled."),
            1 => Some("Your AccountID is not acceptable."),
            2 => Some("Your Password is not acceptable."),
            3 => Some("No account with that ID exists."),
            4 => Some("Incorrect password for that account ID."),
            5 => Some("The account's password must be changed before logging in."),
            _ => None,
        }
    }

    fn new_account_result_message(result: u8) -> Option<&'static str> {
        match result {
            0 => Some("Account creation is currently disabled."),
            1 => Some("Your AccountID is not acceptable."),
            2 => Some("Your Password is not acceptable."),
            3 => Some("Your E-Mail Address is not acceptable."),
            4 => Some("Your User Name is not acceptable."),
            5 => Some("Your Secret Question is not acceptable."),
            6 => Some("Your Secret Answer is not acceptable."),
            7 => Some("An account with this ID already exists."),
            8 => Some("Your account was created successfully."),
            _ => None,
        }
    }

    fn change_password_result_message(result: u8) -> Option<&'static str> {
        match result {
            0 => Some("Password changing is currently disabled."),
            1 => Some("Your AccountID is not acceptable."),
            2 => Some("The current password is not acceptable."),
            3 => Some("Your new password is not acceptable."),
            4 => Some("No account with that ID exists."),
            5 => Some("Incorrect password for that account ID."),
            6 => Some("Your password was changed successfully."),
            _ => None,
        }
    }

    fn ban_message(prefix: &str, info: &BanInfo) -> String {
        format!(
            "{} ban active. Reason: {}. Expiry ticks: {}.",
            prefix, info.reason, info.expiry_date
        )
    }
    
    /// Attempt to connect to server（ECS版本）
    pub fn connect_to_server(&mut self) {
        self.connecting = true;
        self.connect_attempts += 1;
        self.login_enabled = false;
        self.version_checked = false;
        self.version_valid = false;
        self.ready_for_character_select = false;
        
        // 如果已有连接框，先销毁
        if let Some(handle) = self.connecting_box.take() {
            let _ = self.ecs_world.despawn(handle.background);
            let _ = self.ecs_world.despawn(handle.cancel_button);
        }
        
        // 创建连接提示框（ECS）
        let handle = create_connecting_box(&mut self.ecs_world);
        self.connecting_box = Some(handle);
        
        let status = format!(
            "Attempting to connect to server (attempt {})",
            self.connect_attempts
        );
        self.record_status(status);
        
        tracing::info!("🔌 正在连接服务器（尝试 {}）", self.connect_attempts);
    }
    
    /// Submit login credentials（ECS版本）
    pub fn submit_login(&mut self) {
        let credentials = if let Some(handle) = &self.login_dialog {
            get_login_credentials(&self.ecs_world, handle)
        } else {
            None
        };
        
        if let Some((username, password)) = credentials {
            if let Some(tx) = &self.command_tx {
                let command = crate::network::NetworkCommand::Login {
                    username: username.clone(),
                    password: password.clone(),
                };
                
                if let Err(e) = tx.send(command) {
                    tracing::error!("发送登录命令失败: {}", e);
                    self.show_message("网络错误,无法发送登录请求");
                    return;
                }
                
                tracing::info!("✅ 已发送登录请求: {}", username);
                self.record_status(format!("正在提交登录请求: {}", username));
                self.connect_to_server();
            } else {
                self.show_message("网络未初始化,请稍后再试");
            }
            
            self.connecting = true;
            self.login_enabled = false;
        } else {
            self.show_message("请输入账号和密码");
        }
    }
    
    /// Open new account dialog（ECS版本）
    pub fn open_new_account_dialog(&mut self) {
        // 隐藏登录对话框
        if let Some(handle) = &self.login_dialog {
            set_login_dialog_visible(&mut self.ecs_world, handle, false);
        }
        
        // 如果已有新建账号对话框，先销毁
        if let Some(handle) = self.new_account_dialog.take() {
            let _ = self.ecs_world.despawn(handle.dialog_bg);
            let _ = self.ecs_world.despawn(handle.ok_button);
            let _ = self.ecs_world.despawn(handle.cancel_button);
        }
        
        // 创建新建账号对话框（ECS）
        tracing::info!("🎯 正在使用ECS版本创建NewAccountDialog...");
        let dialog_handle = create_new_account_dialog(&mut self.ecs_world);
        self.new_account_dialog = Some(dialog_handle);
        tracing::info!("✅ ECS NewAccountDialog创建成功，实体数: {}", self.ecs_world.len());
        
        self.record_status("Opening new account dialog");
    }
    
    /// Open change password dialog（ECS版本）
    pub fn open_change_password_dialog(&mut self, _autofill_id: Option<String>, _autofill_password: Option<String>) {
        // 隐藏登录对话框
        if let Some(handle) = &self.login_dialog {
            set_login_dialog_visible(&mut self.ecs_world, handle, false);
        }
        
        // 如果已有修改密码对话框，先销毁
        if let Some(handle) = self.change_password_dialog.take() {
            let _ = self.ecs_world.despawn(handle.background);
            let _ = self.ecs_world.despawn(handle.ok_button);
            let _ = self.ecs_world.despawn(handle.cancel_button);
        }
        
        // 创建修改密码对话框（ECS）
        let dialog_handle = create_change_password_dialog(&mut self.ecs_world);
        self.change_password_dialog = Some(dialog_handle);
        
        self.record_status("Opening change password dialog");
    }
    
    /// Close new account dialog（ECS版本）
    pub fn close_new_account_dialog(&mut self) {
        if let Some(handle) = self.new_account_dialog.take() {
            let _ = self.ecs_world.despawn(handle.dialog_bg);
            let _ = self.ecs_world.despawn(handle.ok_button);
            let _ = self.ecs_world.despawn(handle.cancel_button);
        }
        
        // 显示登录对话框
        if let Some(handle) = &self.login_dialog {
            set_login_dialog_visible(&mut self.ecs_world, handle, true);
        }
    }
    
    /// Close change password dialog（ECS版本）
    pub fn close_change_password_dialog(&mut self) {
        if let Some(handle) = self.change_password_dialog.take() {
            let _ = self.ecs_world.despawn(handle.background);
            let _ = self.ecs_world.despawn(handle.ok_button);
            let _ = self.ecs_world.despawn(handle.cancel_button);
        }
        
        // 显示登录对话框
        if let Some(handle) = &self.login_dialog {
            set_login_dialog_visible(&mut self.ecs_world, handle, true);
        }
    }
    
    /// Submit new account registration to server（ECS版本）
    pub fn submit_new_account(&mut self) {
        let data = if self.new_account_dialog.is_some() {
            get_registration_data(&self.ecs_world)
        } else {
            None
        };
        
        if let Some(data) = data {
            if let Some(tx) = &self.command_tx {
                let command = crate::network::NetworkCommand::NewAccount {
                    account_id: data.account_id.clone(),
                    password: data.password.clone(),
                    birth_date: 0i64,
                    username: data.username.clone(),
                    secret_question: data.secret_question.clone(),
                    secret_answer: data.secret_answer.clone(),
                    email: data.email.clone(),
                };
                
                if let Err(e) = tx.send(command) {
                    tracing::error!("❌ 发送新建账号命令失败: {}", e);
                    self.show_message("网络错误,无法发送注册请求");
                    return;
                }
                
                tracing::info!("✅ 已发送新建账号请求 (ECS): {}", data.account_id);
                self.record_status(format!("正在提交注册请求: {}", data.account_id));
                self.show_message("注册请求已提交,请等待服务器响应...");
            } else {
                self.show_message("网络未初始化,请稍后再试");
            }
        } else {
            self.show_message("请检查输入字段格式是否正确");
        }
    }
    
    /// Submit change password request to server（ECS版本）
    pub fn submit_change_password(&mut self) {
        let data = if let Some(handle) = &self.change_password_dialog {
            get_change_password_data(&self.ecs_world, handle)
        } else {
            None
        };
        
        if let Some((account_id, current_password, new_password)) = data {
            if let Some(tx) = &self.command_tx {
                let command = crate::network::NetworkCommand::ChangePassword {
                    account_id: account_id.clone(),
                    current_password: current_password.clone(),
                    new_password: new_password.clone(),
                };
                
                if let Err(e) = tx.send(command) {
                    tracing::error!("❌ 发送修改密码命令失败: {}", e);
                    self.show_message("网络错误,无法发送修改密码请求");
                    return;
                }
                
                tracing::info!("✅ 已发送修改密码请求: {}", account_id);
                self.record_status(format!("正在提交修改密码请求: {}", account_id));
                self.show_message("修改密码请求已提交,请等待服务器响应...");
            } else {
                self.show_message("网络未初始化,请稍后再试");
            }
        } else {
            self.show_message("请检查所有字段是否正确填写");
        }
    }
    
    /// 处理按钮动作（统一的按钮点击响应）
    fn handle_button_action(&mut self, action: ButtonAction) {
        match action {
            ButtonAction::Login => {
                tracing::info!("🔘 登录按钮");
                self.submit_login();
            }
            ButtonAction::NewAccount => {
                tracing::info!("🔘 新建账号按钮");
                self.open_new_account_dialog();
            }
            ButtonAction::ChangePassword => {
                tracing::info!("🔘 修改密码按钮");
                self.open_change_password_dialog(None, None);
            }
            ButtonAction::NewAccountOk => {
                tracing::info!("🔘 新建账号对话框 - OK");
                self.submit_new_account();
            }
            ButtonAction::NewAccountCancel => {
                tracing::info!("🔘 新建账号对话框 - Cancel");
                self.close_new_account_dialog();
            }
            ButtonAction::ChangePasswordOk => {
                tracing::info!("🔘 修改密码对话框 - OK");
                self.submit_change_password();
            }
            ButtonAction::ChangePasswordCancel => {
                tracing::info!("🔘 修改密码对话框 - Cancel");
                self.close_change_password_dialog();
            }
            ButtonAction::MessageBoxOk => {
                tracing::info!("🔘 消息框 - OK");
                // 关闭消息框
                if let Some(handle) = self.message_box.take() {
                    let _ = self.ecs_world.despawn(handle.background);
                    let _ = self.ecs_world.despawn(handle.ok_button);
                }
            }
            ButtonAction::CancelConnect => {
                tracing::info!("🔘 连接框 - Cancel");
                // 取消连接
                if let Some(handle) = self.connecting_box.take() {
                    let _ = self.ecs_world.despawn(handle.background);
                    let _ = self.ecs_world.despawn(handle.cancel_button);
                }
                self.connecting = false;
                self.login_enabled = true;
            }
            ButtonAction::CloseDialog => {
                tracing::info!("🔘 关闭对话框按钮");
                self.handle_escape_key();
            }
        }
    }
    
    /// 处理Escape键（关闭当前对话框）
    fn handle_escape_key(&mut self) {
        // 优先级：消息框 > 连接框 > 其他对话框
        if self.message_box.is_some() {
            if let Some(handle) = self.message_box.take() {
                let _ = self.ecs_world.despawn(handle.background);
                let _ = self.ecs_world.despawn(handle.ok_button);
            }
        } else if self.connecting_box.is_some() {
            if let Some(handle) = self.connecting_box.take() {
                let _ = self.ecs_world.despawn(handle.background);
                let _ = self.ecs_world.despawn(handle.cancel_button);
            }
            self.connecting = false;
            self.login_enabled = true;
        } else if self.new_account_dialog.is_some() {
            self.close_new_account_dialog();
        } else if self.change_password_dialog.is_some() {
            self.close_change_password_dialog();
        }
        
        tracing::debug!("⌨️ Escape键关闭对话框");
    }
}

impl Default for LoginScene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene for LoginScene {
    /// 更新场景逻辑（ECS版本）
    fn update(
        &mut self, 
        ctx: &mut Context, 
        _world: &mut World,
        _network_tx: &mpsc::UnboundedSender<NetworkCommand>
    ) -> GameResult<Option<SceneType>> {
        // 初始化ECS实体
        self.initialize_ecs();
        
        // 更新ECS动画系统
        let delta_time = ctx.time.delta().as_secs_f32();
        update_animations(&mut self.ecs_world, delta_time);
        
        // 检查是否应该切换到角色选择场景
        if self.ready_for_character_select {
            return Ok(Some(SceneType::Select));
        }
        
        Ok(None)
    }
    
    /// 绘制场景（ECS版本）
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, _world: &World) -> GameResult {
        // 调用ECS渲染系统绘制所有实体
        if let Err(e) = render_all(&self.ecs_world, ctx, canvas) {
            tracing::error!("ECS渲染失败: {}", e);
        }
        
        // 绘制状态信息（临时，后续也应该ECS化）
        use ggez::graphics::{Text, TextFragment, DrawParam, Color as GgezColor};
        
        // 版本信息
        let version_text = Text::new(
            TextFragment::new("Crystal v1.0 - ECS Edition")
                .font("AlibabaPuHuiTi")
                .scale(21.0)
        );
        canvas.draw(&version_text, DrawParam::default()
            .dest([10.0, 10.0])
            .color(GgezColor::from_rgb(200, 200, 255)));
        
        // 连接状态
        if self.connecting {
            let status_text = Text::new(
                TextFragment::new(format!("正在连接服务器... (尝试 {})", self.connect_attempts))
                    .font("AlibabaPuHuiTi")
                    .scale(21.0)
            );
            canvas.draw(&status_text, DrawParam::default()
                .dest([10.0, 740.0])
                .color(GgezColor::from_rgb(255, 255, 100)));
        } else if let Some(status) = &self.last_status {
            let status_text = Text::new(
                TextFragment::new(status.as_str())
                    .font("AlibabaPuHuiTi")
                    .scale(21.0)
            );
            canvas.draw(&status_text, DrawParam::default()
                .dest([10.0, 740.0])
                .color(GgezColor::from_rgb(100, 255, 100)));
        }
        
        // FPS
        let fps = ctx.time.fps();
        let debug_text = Text::new(
            TextFragment::new(format!("FPS: {:.1}", fps))
                .font("AlibabaPuHuiTi")
                .scale(21.0)
        );
        canvas.draw(&debug_text, DrawParam::default()
            .dest([950.0, 10.0])
            .color(GgezColor::from_rgb(255, 255, 255)));
        
        Ok(())
    }
    
    /// 鼠标按下事件（ECS版本）
    fn on_mouse_down(
        &mut self,
        _ctx: &mut Context,
        _world: &mut World,
        button: ggez::winit::event::MouseButton,
        x: f32,
        y: f32,
        _network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult {
        use ggez::winit::event::MouseButton;
        
        if button != MouseButton::Left {
            return Ok(());
        }
        
        // 1. 先检查是否点击了输入框
        if handle_input_click(&mut self.ecs_world, x, y) {
            tracing::debug!("🖱️ 输入框获得焦点");
            return Ok(());
        }
        
        // 2. 检查是否点击了按钮
        if let Some(action) = handle_mouse_click(&self.ecs_world, x, y) {
            tracing::debug!("🖱️ 按钮点击: {:?}", action);
            self.handle_button_action(action);
        }
        
        Ok(())
    }
    
    /// 鼠标移动事件（ECS版本）
    fn on_mouse_move(
        &mut self,
        _ctx: &mut Context,
        _world: &mut World,
        x: f32,
        y: f32,
    ) -> GameResult {
        // 更新按钮悬停状态
        handle_mouse_move(&mut self.ecs_world, x, y);
        Ok(())
    }
    
    /// 键盘按下事件（ECS版本）
    fn on_key_down(
        &mut self,
        _ctx: &mut Context,
        _world: &mut World,
        input: ggez::input::keyboard::KeyInput,
        _network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult<Option<SceneType>> {
        use ggez::winit::keyboard::KeyCode;
        
        if let ggez::winit::event::KeyEvent {
            physical_key: ggez::winit::keyboard::PhysicalKey::Code(keycode),
            text,
            ..
        } = &input.event {
            match keycode {
                KeyCode::Backspace => {
                    handle_backspace(&mut self.ecs_world);
                }
                KeyCode::Tab => {
                    handle_tab(&mut self.ecs_world);
                }
                KeyCode::Enter => {
                    // 处理Enter键（提交表单）
                    if let Some(action) = handle_enter(&self.ecs_world) {
                        self.handle_button_action(action);
                    }
                }
                KeyCode::Escape => {
                    // 处理Escape键（关闭对话框）
                    if handle_escape() {
                        self.handle_escape_key();
                    }
                }
                _ => {
                    // 处理文本输入
                    if let Some(text_str) = text {
                        for ch in text_str.chars() {
                            handle_char_input(&mut self.ecs_world, ch);
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// ============================================================================
// 网络事件处理
// ============================================================================

impl LoginScene {
    /// 处理网络事件（由GameApp调用）
    pub fn handle_network_event(&mut self, event: &GameEvent) {
        match event {
            GameEvent::Connected => {
                let status = "Connected to server successfully!";
                self.connecting = false;
                self.record_status(status);
                self.send_client_version();
            }
            GameEvent::Disconnected { reason } => {
                let status = format!("Disconnected: {}", reason);
                self.connecting = false;
                self.login_enabled = false;
                self.ready_for_character_select = false;
                self.record_status(status);
            }
            GameEvent::SystemMessage { message } => {
                self.record_status(message.clone());
            }
            GameEvent::ClientVersionResponse { result } => {
                self.handle_client_version_response(*result);
            }
            GameEvent::LoginResponse { result } => {
                self.handle_login_response(*result);
            }
            GameEvent::LoginBanned { reason, expiry_date } => {
                self.handle_login_ban(reason, *expiry_date);
            }
            GameEvent::LoginSuccess { characters } => {
                self.handle_login_success(characters);
            }
            GameEvent::NewAccountResponse { result } => {
                self.handle_new_account_response(*result);
            }
            GameEvent::ChangePasswordResponse { result } => {
                self.handle_change_password_response(*result);
            }
            GameEvent::ChangePasswordBanned { reason, expiry_date } => {
                self.handle_change_password_ban(reason, *expiry_date);
            }
            _ => {}
        }
    }
}
