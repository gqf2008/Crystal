// ============================================================================
// 角色选择场景 - 纯 Native 版本 (无 egui)
// ============================================================================
// 
// 【渲染架构说明】
// 本场景采用纯 macroquad 原生渲染，无 egui 依赖
//
// ============================================================================

use crate::game::GameResult;
use crate::network::{NetContext, NetworkBuilder, NetworkEvent};
use crate::scenes::{Scene, SceneTransition};
mod credits_dialog;
mod delete_character_dialog;
mod input;
mod new_character_dialog;
mod view;
use credits_dialog::CreditsDialog;
use macroquad::prelude::*;
use mir2_shared::SelectInfo;

/// 角色信息
#[derive(Debug, Clone)]
pub struct CharacterInfo {
    pub index: i32,
    pub name: String,
    pub level: u16,
    pub class: u8,  // 0=Warrior, 1=Wizard, 2=Taoist
    pub gender: u8, // 0=Male, 1=Female
    pub last_access: String,
}

/// 角色选择场景
pub struct SelectScene {
    // 角色数据
    characters: Vec<CharacterInfo>,
    selected_index: Option<usize>,
    
    // 对话框状态
    show_new_character: bool,
    show_delete_character: bool,
    show_message_box: bool,
    message_text: String,
    
    // 新建角色表单
    new_char_name: String,
    new_char_class: u8,
    new_char_gender: u8,
    
    // 删除角色确认
    delete_char_name: String,
    delete_char_index: i32,
    delete_confirm_input: String,
    
    // 角色预览动画
    animation_frame: usize,
    animation_timer: f32,
    animation_delay: f32,
    
    // 光标闪烁
    cursor_visible: bool,
    cursor_timer: f32,

    // 网络 / 进场
    net: Option<NetContext>,
    start_game_requested: bool,
    start_game_in_flight: bool,
    start_game_character_index: Option<i32>,

    // 角色操作网络状态
    character_op_in_flight: bool,

    // Credits
    credits_dialog: CreditsDialog,
}

impl SelectScene {
    pub fn new(characters: Vec<CharacterInfo>) -> GameResult<Self> {
        let selected_index = if !characters.is_empty() { Some(0) } else { None };
        
        Ok(Self {
            characters,
            selected_index,
            
            show_new_character: false,
            show_delete_character: false,
            show_message_box: false,
            message_text: String::new(),
            
            new_char_name: String::new(),
            new_char_class: 0,
            new_char_gender: 0,
            
            delete_char_name: String::new(),
            delete_char_index: -1,
            delete_confirm_input: String::new(),
            
            animation_frame: 0,
            animation_timer: 0.0,
            animation_delay: 0.25,
            
            cursor_visible: true,
            cursor_timer: 0.0,

            net: None,
            start_game_requested: false,
            start_game_in_flight: false,
            start_game_character_index: None,

            character_op_in_flight: false,

            credits_dialog: CreditsDialog::new(),
        })
    }

    fn apply_server_character_list(&mut self, list: Vec<SelectInfo>) {
        // 同步到全局：返回选角时仍能复用
        crate::network::set_global_characters(list.clone());

        self.characters = list
            .into_iter()
            .map(|c| CharacterInfo {
                index: c.index,
                name: c.name,
                level: c.level,
                class: c.class as u8,
                gender: c.gender as u8,
                last_access: c.last_access.format("%Y-%m-%d %H:%M").to_string(),
            })
            .collect();

        self.selected_index = if self.characters.is_empty() {
            None
        } else {
            Some(self.selected_index.unwrap_or(0).min(self.characters.len() - 1))
        };
    }

    fn request_start_game(&mut self) {
        let Some(selected_idx) = self.selected_index else {
            self.show_message("请先选择一个角色");
            return;
        };
        if selected_idx >= self.characters.len() {
            self.show_message("角色索引无效");
            return;
        }

        let character = &self.characters[selected_idx];
        self.start_game_character_index = Some(character.index);
        self.start_game_requested = true;
    }

    fn ensure_network(&mut self) {
        if self.net.is_some() {
            return;
        }

        // 优先接管 LoginScene/GameScene 放入的全局连接
        if let Some(net) = crate::network::take_global_net() {
            self.net = Some(net);
            return;
        }

        // 读取 config.ini（默认 mock=true，保证离线可跑）
        let cfg = crate::network::load_network_runtime_config();
        let builder = NetworkBuilder::new(cfg.server_addr)
            .with_mock(cfg.use_mock)
            .with_client_version_hash(cfg.client_version_hash);
        match builder.build() {
            Ok(net) => {
                self.net = Some(net);
            }
            Err(e) => {
                self.show_message(&format!("网络初始化失败: {e}"));
            }
        }
    }

    fn pump_network_for_start_game(&mut self) -> Option<SceneTransition> {
        // 临时取走 net，避免在持有 `&NetContext` 时又去 `&mut self` 触发借用冲突。
        let net = self.net.take()?;

        // 重要：这里不能用 recv_all() 把队列一次性“清空”。
        // 真服通常会在 StartGame 成功后紧跟发送 UserInformation/MapInformation/Object* 等关键数据。
        // 如果在选角场景把这些事件都 drain 掉，GameScene 就收不到，会表现为：
        // - 看不到玩家
        // - 相机停在地图左上角（默认 0,0）
        //
        // 因此这里按事件逐个处理：一旦收到 StartGame 成功，立刻切场景，
        // 让剩余事件留在队列里由 GameScene 的 NetworkSystem 消费。
        let mut saw_any = false;
        while let Some(ev) = net.try_recv() {
            saw_any = true;
            match ev {
                NetworkEvent::LoginSuccess { characters } => {
                    // 角色列表刷新（登录后/创建删除后可能都会推）
                    self.apply_server_character_list(characters);
                    self.character_op_in_flight = false;
                }
                NetworkEvent::CharacterCreated { character } => {
                    // 对齐 C#：关闭创建窗口、提示成功、并把新角色插到列表开头并选中
                    self.show_new_character = false;

                    let info = CharacterInfo {
                        index: character.index,
                        name: character.name.clone(),
                        level: character.level,
                        class: character.class as u8,
                        gender: character.gender as u8,
                        last_access: character.last_access.format("%Y-%m-%d %H:%M").to_string(),
                    };

                    self.characters.retain(|c| c.index != info.index);
                    self.characters.insert(0, info);
                    self.selected_index = Some(0);

                    self.show_message("Your character was created successfully.");
                    self.character_op_in_flight = false;
                }
                NetworkEvent::CharacterDeleted { index } => {
                    // 服务器可能不会立即发全量列表，这里先本地移除；若后续收到 LoginSuccess 会再覆盖
                    if let Some(pos) = self.characters.iter().position(|c| c.index == index as i32) {
                        self.characters.remove(pos);
                    }
                    self.selected_index = if self.characters.is_empty() {
                        None
                    } else {
                        Some(self.selected_index.unwrap_or(0).min(self.characters.len() - 1))
                    };
                    self.show_message(&format!("角色已删除: index={index}"));
                    self.character_op_in_flight = false;
                }
                NetworkEvent::StartGame { packet } => {
                    // C#：Result=4 表示 Success，带 Resolution。
                    if packet.result == 4 {
                        // 场景间移交连接：Select -> Game
                        crate::network::set_global_net(net);
                        return Some(SceneTransition::Game);
                    }
                    self.show_message(&format!("StartGame 失败: result={}", packet.result));
                    self.start_game_in_flight = false;
                }
                NetworkEvent::StartGameDelay { packet } => {
                    self.show_message(&format!("开始游戏延迟: {}ms", packet.milliseconds));
                    self.start_game_in_flight = false;
                }
                NetworkEvent::StartGameBanned { packet } => {
                    self.show_message(&format!("开始游戏被禁止: {}", packet.reason));
                    self.start_game_in_flight = false;
                }
                NetworkEvent::Disconnected { reason } => {
                    self.show_message(&format!("已断开连接: {reason}"));
                    self.start_game_in_flight = false;
                    self.character_op_in_flight = false;
                }
                NetworkEvent::SystemMessage { message } => {
                    // 角色创建/删除失败等会走这里；统一用 MirMessageBox(OK) 弹出
                    self.show_message(&message);
                    self.character_op_in_flight = false;
                }
                _ => {}
            }
        }

        // 未切场景：把 net 放回去
        self.net = Some(net);

        if !saw_any {
            return None;
        }

        None
    }
    
    /// 显示消息框
    fn show_message(&mut self, message: &str) {
        self.message_text = message.to_string();
        self.show_message_box = true;
    }
}

impl Scene for SelectScene {
    fn name(&self) -> &str {
        "CharacterSelect"
    }

    fn on_enter(&mut self) -> GameResult {
        // 优先接管场景间移交的连接
        if self.net.is_none() {
            if let Some(net) = crate::network::take_global_net() {
                self.net = Some(net);
            }
        }

        Ok(())
    }
    
    fn on_exit(&mut self) -> GameResult {
        Ok(())
    }
    
    fn update(&mut self, dt: f32) -> GameResult<SceneTransition> {
        // 更新角色预览动画
        self.animation_timer += dt;
        if self.animation_timer >= self.animation_delay {
            self.animation_timer = 0.0;
            self.animation_frame = (self.animation_frame + 1) % 16;
        }
        
        // 更新光标闪烁
        self.cursor_timer += dt;
        if self.cursor_timer >= 0.5 {
            self.cursor_timer = 0.0;
            self.cursor_visible = !self.cursor_visible;
        }

        // Credits（静态内容）
        self.credits_dialog.update(dt);

        // 处理“开始游戏”请求（由 render 中的按钮点击触发，下一帧在 update 中发送）
        if self.start_game_requested {
            self.start_game_requested = false;
            self.ensure_network();

            if let (Some(net), Some(character_index)) = (self.net.as_ref(), self.start_game_character_index) {
                if !self.start_game_in_flight {
                    if let Err(e) = net.send(NetworkEvent::StartGameRequest { character_index }) {
                        self.show_message(&format!("发送 StartGameRequest 失败: {e}"));
                    } else {
                        self.start_game_in_flight = true;
                    }
                }
            }
        }

        // 轮询网络：等待 StartGame* 回包
        if let Some(t) = self.pump_network_for_start_game() {
            return Ok(t);
        }
        
        Ok(SceneTransition::None)
    }
    
    fn render(&mut self) -> GameResult {
        self.render_scene()
    }

    fn handle_input(&mut self) -> GameResult {
        self.handle_scene_input()
    }
}
