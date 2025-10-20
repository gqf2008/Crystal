// interaction_systems.rs - NPC 和对象交互系统
// 
// 功能说明:
// - 对话系统初始化: 创建对话树、对话节点、对话选项
// - 交互检测: 检测玩家附近的 NPC 和可交互对象
// - 交互处理: 处理玩家按 F 键触发交互
// - 对话显示: 更新 UI 显示当前对话内容和选项
// - 对话选择: 处理玩家数字键选择对话选项
// - 消息处理: 处理网络消息触发的对话
//
// 系统列表:
// 1. setup_dialogue_system - 初始化对话系统
// 2. detect_interaction_system - 检测交互系统
// 3. handle_interaction_system - 处理交互系统
// 4. update_dialogue_display_system - 显示对话UI系统
// 5. handle_dialogue_choice_system - 处理对话选择系统
// 6. message_handle_npc_dialogue - 处理NPC交互消息
//
// 辅助函数:
// - start_dialogue_with_npc - 启动对话

use bevy::prelude::*;
use super::{
    DialogueState, InteractionState, DialogueTree, DialogueNode, DialogueOption,
    Player, NPC, InteractiveObject, ChatMessageList, StartDialogueMessage
};

/// 初始化对话系统
/// 
/// 功能:
/// - 创建对话状态和交互状态资源
/// - 创建示例对话树 (村长的对话)
/// - 添加3个对话节点: 问候、介绍、世界说明
/// - 每个节点包含多个对话选项
pub fn setup_dialogue_system(
    mut commands: Commands,
) {
    // 创建对话状态资源
    commands.insert_resource(DialogueState::default());
    commands.insert_resource(InteractionState::default());
    
    // 创建一个示例对话树 (村长的对话)
    let mut dialogue_tree = DialogueTree::new(1, 1, 1);
    
    // 节点 1: 初次问候
    let greeting_options = vec![
        DialogueOption {
            option_id: 1,
            text: "你好，我是新手冒险者。".to_string(),
            next_dialogue_id: Some(2),
            action: String::new(),
            conditions: vec![],
        },
        DialogueOption {
            option_id: 2,
            text: "能告诉我关于这个世界吗？".to_string(),
            next_dialogue_id: Some(3),
            action: String::new(),
            conditions: vec![],
        },
    ];
    
    let greeting_node = DialogueNode {
        node_id: 1,
        npc_id: 1,
        text: "欢迎来到我们的村子！有什么我可以帮你的吗？".to_string(),
        speaker: "村长".to_string(),
        options: greeting_options,
        auto_next: None,
    };
    
    dialogue_tree.add_node(greeting_node);
    
    // 节点 2: 介绍自己
    let intro_node = DialogueNode {
        node_id: 2,
        npc_id: 1,
        text: "很高兴认识你！希望你在这里过得愉快。".to_string(),
        speaker: "村长".to_string(),
        options: vec![
            DialogueOption {
                option_id: 3,
                text: "谢谢你的欢迎。".to_string(),
                next_dialogue_id: None,
                action: String::new(),
                conditions: vec![],
            },
        ],
        auto_next: None,
    };
    
    dialogue_tree.add_node(intro_node);
    
    // 节点 3: 世界介绍
    let world_node = DialogueNode {
        node_id: 3,
        npc_id: 1,
        text: "这是一个充满魔法和冒险的世界。小心怪物和强大的敌人！".to_string(),
        speaker: "村长".to_string(),
        options: vec![
            DialogueOption {
                option_id: 4,
                text: "我会小心的。".to_string(),
                next_dialogue_id: None,
                action: String::new(),
                conditions: vec![],
            },
        ],
        auto_next: None,
    };
    
    dialogue_tree.add_node(world_node);
    
    commands.insert_resource(dialogue_tree);
    
    info!("🎭 对话系统已初始化");
}

/// 检测交互系统 - 检测玩家附近的可交互对象
/// 
/// 功能:
/// - 获取玩家位置
/// - 计算玩家与所有 NPC 的距离
/// - 计算玩家与所有交互对象的距离
/// - 范围内的对象加入 nearby_objects 列表
/// - 更新 can_interact 标志
pub fn detect_interaction_system(
    player_query: Query<&Transform, With<Player>>,
    npc_query: Query<(&NPC, &Transform)>,
    object_query: Query<(&InteractiveObject, &Transform)>,
    mut interaction_state: ResMut<InteractionState>,
) {
    let Some(player_transform) = player_query.iter().next() else {
        return;
    };
    
    let player_pos = player_transform.translation;
    let interaction_range = 100.0;  // 交互范围（像素）
    
    interaction_state.nearby_objects.clear();
    
    // 检查附近的 NPC
    for (npc, npc_transform) in npc_query.iter() {
        let distance = player_pos.distance(npc_transform.translation);
        
        if distance < interaction_range {
            interaction_state.nearby_objects.push(npc.npc_id);
            interaction_state.can_interact = true;
        }
    }
    
    // 检查附近的交互对象
    for (obj, obj_transform) in object_query.iter() {
        let distance = player_pos.distance(obj_transform.translation);
        
        if distance < interaction_range {
            interaction_state.nearby_objects.push(obj.object_id);
            interaction_state.can_interact = true;
        }
    }
    
    if !interaction_state.nearby_objects.is_empty() {
        info!("✨ 附近有 {} 个可交互对象", interaction_state.nearby_objects.len());
    }
}

/// 处理交互系统 - 处理玩家交互
/// 
/// 功能:
/// - 检测 F 键按下
/// - 如果附近有可交互对象，启动对话
/// - 使用第一个附近对象的 ID
pub fn handle_interaction_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    interaction_state: Res<InteractionState>,
    mut dialogue_state: ResMut<DialogueState>,
    dialogue_tree: Res<DialogueTree>,
) {
    // 按 F 键交互
    if keyboard.just_pressed(KeyCode::KeyF) && interaction_state.can_interact {
        if let Some(object_id) = interaction_state.nearby_objects.first() {
            start_dialogue_with_npc(&mut dialogue_state, dialogue_tree.as_ref(), *object_id as i32);
        }
    }
}

/// 启动对话
/// 
/// 辅助函数，用于初始化对话状态
/// 
/// 参数:
/// - dialogue_state: 对话状态（可变引用）
/// - dialogue_tree: 对话树（不可变引用）
/// - npc_id: NPC ID
fn start_dialogue_with_npc(
    dialogue_state: &mut DialogueState,
    dialogue_tree: &DialogueTree,
    npc_id: i32,
) {
    dialogue_state.is_in_dialogue = true;
    dialogue_state.current_npc_id = Some(npc_id);
    dialogue_state.current_node_id = dialogue_tree.start_node_id;
    dialogue_state.tree_id = dialogue_tree.tree_id;
    
    info!("🎭 开始与 NPC {} 对话", npc_id);
    
    if let Some(node) = dialogue_tree.get_node(dialogue_state.current_node_id) {
        info!("💬 [{}]: {}", node.speaker, node.text);
    }
}

/// 显示对话UI系统 - 更新对话显示
/// 
/// 功能:
/// - 检查是否在对话中
/// - 获取当前对话节点
/// - 格式化对话文本和选项
/// - 更新聊天消息列表显示
pub fn update_dialogue_display_system(
    dialogue_state: Res<DialogueState>,
    dialogue_tree: Res<DialogueTree>,
    mut ui_query: Query<&mut Text, With<ChatMessageList>>,
) {
    if !dialogue_state.is_in_dialogue {
        return;
    }
    
    if let Some(node) = dialogue_tree.get_node(dialogue_state.current_node_id) {
        for mut text in ui_query.iter_mut() {
            let mut display = format!("【对话】\n");
            display.push_str(&format!("[{}]: {}\n\n", node.speaker, node.text));
            
            // 显示选项
            for (idx, option) in node.options.iter().enumerate() {
                display.push_str(&format!("{}. {}\n", idx + 1, option.text));
            }
            
            display.push_str("\n[按数字键选择选项, ESC 关闭对话]");
            text.0 = display;
        }
    }
}

/// 处理对话选择系统
/// 
/// 功能:
/// - ESC 键关闭对话
/// - 数字键 1-4 选择对话选项
/// - 执行选项关联的动作
/// - 跳转到下一个对话节点
/// - 如果没有下一个节点，结束对话
pub fn handle_dialogue_choice_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut dialogue_state: ResMut<DialogueState>,
    mut dialogue_tree: ResMut<DialogueTree>,
) {
    if !dialogue_state.is_in_dialogue {
        return;
    }
    
    // ESC 键关闭对话
    if keyboard.just_pressed(KeyCode::Escape) {
        dialogue_state.is_in_dialogue = false;
        dialogue_state.current_npc_id = None;
        info!("🎭 对话已结束");
        return;
    }
    
    // 处理数字键选择
    let choice_keys = [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
    ];
    
    if let Some(current_node) = dialogue_tree.get_node(dialogue_state.current_node_id) {
        for (key, index) in choice_keys {
            if keyboard.just_pressed(key) && index < current_node.options.len() {
                let option = &current_node.options[index];
                
                info!("玩家选择: {}", option.text);
                
                // 执行动作
                if !option.action.is_empty() {
                    info!("执行动作: {}", option.action);
                }
                
                // 进行到下一个对话
                if let Some(next_id) = option.next_dialogue_id {
                    dialogue_state.current_node_id = next_id;
                    
                    if let Some(next_node) = dialogue_tree.get_node(next_id) {
                        info!("💬 [{}]: {}", next_node.speaker, next_node.text);
                    }
                } else {
                    // 对话结束
                    dialogue_state.is_in_dialogue = false;
                    dialogue_state.current_npc_id = None;
                    info!("🎭 对话已结束");
                }
                
                break;
            }
        }
    }
}

/// 处理 NPC 交互消息
/// 
/// 功能:
/// - 监听网络消息 StartDialogueMessage
/// - 根据消息中的 npc_id 启动对话
/// - 用于网络同步的对话触发
pub fn message_handle_npc_dialogue(
    events: Option<MessageReader<StartDialogueMessage>>,
    mut dialogue_state: ResMut<DialogueState>,
    dialogue_tree: Res<DialogueTree>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        start_dialogue_with_npc(&mut dialogue_state, dialogue_tree.as_ref(), event.npc_id);
    }
}
