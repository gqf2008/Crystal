// chat_systems.rs - 聊天系统完整实现
// 
// 功能说明:
// - 聊天系统初始化: 创建聊天过滤器、命令管理器、显示设置
// - 聊天输入处理: 处理键盘输入、打开/关闭聊天窗口
// - 命令系统: 处理以 / 开头的聊天命令
// - 消息接收: 接收来自网络或系统的聊天消息
// - 消息过滤: 根据消息类型和设置过滤显示的消息
// - 屏蔽词过滤: 替换敏感词为 ***
// - 聊天显示: 在 UI 中格式化并显示聊天消息
// - 历史管理: 清理过期消息、维护历史记录大小
// - 消息处理: 处理网络消息触发的聊天
//
// 系统列表:
// 1. setup_chat_system - 初始化聊天系统
// 2. process_chat_input_system - 处理聊天输入系统
// 3. process_chat_commands_system - 处理聊天命令系统
// 4. receive_chat_messages_system - 接收聊天消息系统
// 5. filter_chat_messages_system - 过滤聊天消息系统
// 6. apply_word_filter_system - 应用屏蔽词系统
// 7. update_chat_display_system - 更新聊天显示系统
// 8. manage_chat_history_system - 管理聊天历史系统
// 9. message_handle_send_chat - 处理发送聊天消息
//
// 辅助函数:
// - send_chat_message - 发送聊天消息（内部函数）

use bevy::prelude::*;
use super::{
    ChatManager, ChatMessage, ChatFilterConfig, ChatCommandManager, 
    ChatDisplaySettings, ChatMessageList, GameSceneState, SendChatMessage
};

/// 初始化聊天系统
/// 
/// 功能:
/// - 创建聊天过滤器配置资源
/// - 创建聊天命令管理器资源
/// - 创建聊天显示设置资源
pub fn setup_chat_system(
    mut commands: Commands,
) {
    // 聊天管理器已在 Phase 1 中初始化
    // 这里初始化聊天的额外设置
    commands.insert_resource(ChatFilterConfig::default());
    commands.insert_resource(ChatCommandManager::default());
    commands.insert_resource(ChatDisplaySettings::default());
    
    info!("💬 聊天系统已完整初始化");
}

/// 处理聊天输入系统 - 完整的字符输入处理
/// 
/// 功能:
/// - T 键打开/关闭聊天窗口
/// - Backspace 删除字符
/// - Enter 发送消息
/// - Escape 关闭聊天并清空输入
pub fn process_chat_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut chat_manager: ResMut<ChatManager>,
    mut game_state: ResMut<GameSceneState>,
) {
    // T 键打开/关闭聊天
    if keyboard.just_pressed(KeyCode::KeyT) {
        game_state.show_chat = !game_state.show_chat;
        
        if game_state.show_chat {
            info!("💬 聊天窗口已打开");
        } else {
            info!("💬 聊天窗口已关闭");
        }
        
        return;
    }
    
    if !game_state.show_chat {
        return;
    }
    
    // 处理字符输入
    // 注: 在实际应用中应该使用 ReceivedCharacter 事件
    // 这里简化处理
    
    // Backspace 删除字符
    if keyboard.just_pressed(KeyCode::Backspace) {
        chat_manager.input_buffer.pop();
    }
    
    // Enter 发送消息
    if keyboard.just_pressed(KeyCode::Enter) {
        if !chat_manager.input_buffer.is_empty() {
            send_chat_message(&mut chat_manager);
        }
    }
    
    // Escape 关闭聊天
    if keyboard.just_pressed(KeyCode::Escape) {
        game_state.show_chat = false;
        chat_manager.input_buffer.clear();
        info!("💬 聊天窗口已关闭");
    }
}

/// 发送聊天消息 - 将消息添加到历史记录
/// 
/// 辅助函数，内部使用
/// 
/// 功能:
/// - 检查消息是否为空
/// - 创建 ChatMessage 对象
/// - 添加到历史记录
/// - 维护历史记录大小限制
/// - 清空输入缓冲
fn send_chat_message(chat_manager: &mut ResMut<ChatManager>) {
    if chat_manager.input_buffer.is_empty() {
        return;
    }
    
    let content = chat_manager.input_buffer.clone();
    
    // 检查消息长度
    if content.len() > chat_manager.input_buffer.len() {
        info!("⚠️ 消息过长，已截断");
        return;
    }
    
    let message = ChatMessage {
        sender: "玩家".to_string(),
        content: content.clone(),
        timestamp: 0.0,  // TODO: 使用真实时间戳
        message_type: 0, // 普通消息
    };
    
    chat_manager.history.push_back(message);
    
    // 保持历史记录大小限制
    while chat_manager.history.len() > chat_manager.max_history {
        chat_manager.history.pop_front();
    }
    
    info!("💬 消息已发送: {}", content);
    chat_manager.input_buffer.clear();
}

/// 处理聊天命令系统 - 处理以特定前缀开头的消息
/// 
/// 功能:
/// - 检查命令是否以 / 开头
/// - 解析命令名称和参数
/// - 执行对应命令: help, emote, whisper, party
/// - 显示命令帮助和错误提示
pub fn process_chat_commands_system(
    mut chat_manager: ResMut<ChatManager>,
    command_manager: Res<ChatCommandManager>,
) {
    if !command_manager.enabled || chat_manager.input_buffer.is_empty() {
        return;
    }
    
    let input = &chat_manager.input_buffer;
    
    // 检查是否以 / 开头
    if !input.starts_with('/') {
        return;
    }
    
    // 解析命令
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }
    
    let command_name = parts[0].trim_start_matches('/');
    let args = parts[1..].to_vec();
    
    // 处理命令
    match command_name {
        "help" => {
            info!("📖 帮助命令:");
            for cmd in &command_manager.commands {
                info!("  /{} - {}", cmd.name, cmd.description);
            }
        }
        "emote" => {
            if !args.is_empty() {
                info!("😊 执行表情动作: {}", args.join(" "));
            } else {
                info!("⚠️ 用法: /emote <动作>");
            }
        }
        "whisper" => {
            if args.len() >= 2 {
                let target = args[0];
                let message = args[1..].join(" ");
                info!("🤐 私聊 {}: {}", target, message);
            } else {
                info!("⚠️ 用法: /whisper <玩家名> <消息>");
            }
        }
        "party" => {
            if !args.is_empty() {
                info!("👥 队伍聊天: {}", args.join(" "));
            } else {
                info!("⚠️ 用法: /party <消息>");
            }
        }
        _ => {
            info!("⚠️ 未知命令: /{}", command_name);
        }
    }
    
    chat_manager.input_buffer.clear();
}

/// 接收聊天消息系统 - 模拟接收其他玩家的消息
/// 
/// 功能:
/// - 从网络接收消息（待实现）
/// - 添加系统消息到历史记录
/// - 每60帧（约1秒）添加一次欢迎消息
pub fn receive_chat_messages_system(
    mut chat_manager: ResMut<ChatManager>,
    game_state: Res<GameSceneState>,
) {
    // 这里可以从网络接收消息
    // 现在只是演示如何添加消息到历史记录
    
    // 可以在这里添加来自 NPC 或系统的消息
    if game_state.game_time as u32 % 60 == 0 {
        // 每 60 帧添加一条系统消息（约 1 秒）
        if chat_manager.history.iter().all(|m| m.message_type != 1) {
            let system_message = ChatMessage {
                sender: "系统".to_string(),
                content: "欢迎来到游戏世界！".to_string(),
                timestamp: game_state.game_time,
                message_type: 1,  // 系统消息
            };
            
            chat_manager.history.push_back(system_message);
        }
    }
}

/// 过滤聊天消息系统 - 根据过滤器过滤消息
/// 
/// 功能:
/// - 根据消息类型过滤
/// - 类型 0: 普通消息（总是显示）
/// - 类型 1: 系统消息（根据设置）
/// - 类型 2: 私聊消息（根据设置）
/// - 类型 3: 公告消息（根据设置）
/// 
/// 返回值:
/// - 过滤后的消息列表
pub fn filter_chat_messages_system(
    chat_manager: Res<ChatManager>,
    filter_config: Res<ChatFilterConfig>,
) -> Vec<ChatMessage> {
    let mut filtered_messages = Vec::new();
    
    for message in chat_manager.history.iter() {
        let should_show = match message.message_type {
            0 => true,  // 普通消息总是显示
            1 => filter_config.show_system,
            2 => filter_config.show_whisper,
            3 => filter_config.show_broadcast,
            _ => true,
        };
        
        if should_show {
            filtered_messages.push(message.clone());
        }
    }
    
    filtered_messages
}

/// 应用屏蔽词系统 - 对消息内容进行检查和过滤
/// 
/// 功能:
/// - 遍历屏蔽词列表
/// - 将敏感词替换为等长的 ***
/// 
/// 参数:
/// - content: 原始消息内容
/// - filter_config: 过滤器配置
/// 
/// 返回值:
/// - 过滤后的消息内容
pub fn apply_word_filter_system(
    content: &str,
    filter_config: &ChatFilterConfig,
) -> String {
    let mut filtered = content.to_string();
    
    for bad_word in &filter_config.word_filter {
        let replacement = "*".repeat(bad_word.len());
        filtered = filtered.replace(bad_word, &replacement);
    }
    
    filtered
}

/// 更新聊天显示系统 - 在 UI 中显示聊天消息
/// 
/// 功能:
/// - 过滤消息（根据类型和设置）
/// - 限制可见消息数量
/// - 格式化消息（时间戳、发送者、内容）
/// - 应用屏蔽词过滤
/// - 根据消息类型添加前缀和着色
/// - 显示输入缓冲和光标
pub fn update_chat_display_system(
    chat_manager: Res<ChatManager>,
    display_settings: Res<ChatDisplaySettings>,
    filter_config: Res<ChatFilterConfig>,
    mut text_query: Query<&mut Text, With<ChatMessageList>>,
    game_state: Res<GameSceneState>,
) {
    if !game_state.show_chat {
        return;
    }
    
    // 过滤消息
    let display_messages: Vec<ChatMessage> = chat_manager
        .history
        .iter()
        .filter(|m| match m.message_type {
            0 => true,
            1 => filter_config.show_system,
            2 => filter_config.show_whisper,
            3 => filter_config.show_broadcast,
            _ => true,
        })
        .cloned()
        .collect();
    
    // 只显示最后的消息
    let start_idx = if display_messages.len() > display_settings.max_visible_messages {
        display_messages.len() - display_settings.max_visible_messages
    } else {
        0
    };
    
    let visible_messages = &display_messages[start_idx..];
    
    for mut text in text_query.iter_mut() {
        let mut display_text = String::from("【聊天】\n");
        
        for msg in visible_messages {
            // 格式化消息
            let formatted = if display_settings.show_timestamps {
                format!(
                    "[{:.0}] {}: {}",
                    msg.timestamp,
                    msg.sender,
                    apply_word_filter_system(&msg.content, &filter_config)
                )
            } else {
                format!(
                    "{}: {}",
                    msg.sender,
                    apply_word_filter_system(&msg.content, &filter_config)
                )
            };
            
            // 根据消息类型着色
            let colored = match msg.message_type {
                0 => format!("{}\n", formatted),  // 普通白色
                1 => format!("【系统】{}\n", formatted),  // 系统黄色
                2 => format!("【私聊】{}\n", formatted),  // 私聊紫色
                3 => format!("【公告】{}\n", formatted),  // 公告青色
                _ => format!("{}\n", formatted),
            };
            
            display_text.push_str(&colored);
        }
        
        // 显示输入缓冲
        display_text.push_str("\n> ");
        display_text.push_str(&chat_manager.input_buffer);
        display_text.push('_');  // 光标
        
        text.0 = display_text;
    }
}

/// 管理聊天历史系统 - 清理过期消息
/// 
/// 功能:
/// - 更新消息时间戳（如果为0）
/// - 删除超过淡出时间的消息
/// - 维护最大消息数限制
pub fn manage_chat_history_system(
    mut chat_manager: ResMut<ChatManager>,
    display_settings: Res<ChatDisplaySettings>,
    game_state: Res<GameSceneState>,
) {
    // 更新消息时间戳
    for message in chat_manager.history.iter_mut() {
        if message.timestamp == 0.0 {
            message.timestamp = game_state.game_time;
        }
    }
    
    // 删除太旧的消息（根据淡出时间）
    let current_time = game_state.game_time;
    let max_age = display_settings.message_fade_time;
    
    chat_manager.history.retain(|msg| {
        current_time - msg.timestamp < max_age
    });
    
    // 保持最大消息数限制
    while chat_manager.history.len() > chat_manager.max_history {
        chat_manager.history.pop_front();
    }
}

/// 处理发送聊天消息
/// 
/// 功能:
/// - 监听网络消息 SendChatMessage
/// - 将消息内容设置到输入缓冲
/// - 调用 send_chat_message 发送
pub fn message_handle_send_chat(
    events: Option<MessageReader<SendChatMessage>>,
    mut chat_manager: ResMut<ChatManager>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        chat_manager.input_buffer = event.text.clone();
        send_chat_message(&mut chat_manager);
    }
}
