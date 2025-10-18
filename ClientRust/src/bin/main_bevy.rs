use bevy::prelude::*;
use bevy::window::WindowResolution;

// 引入 Bevy 模块
mod bevy_modules {
    pub use mir2_client::bevy::*;
}

use bevy_modules::{GameState, GameConfig, MLibraryAssets, MapAssets};
use bevy_modules::{MLibraryResource, load_mlibrary_system};
use bevy_modules::{
    init_global_network_manager,
    process_network_events,
    cleanup_network_manager,
};

// 引入调试系统
use bevy_modules::{debug_shortcuts_system, debug_info_overlay_system};

// 引入 GameScene 渲染系统 (新架构)
use bevy_modules::scenes::{
    MapRenderData, MapLoadRequest, GameCamera,
    render_map_system, update_animation_system, 
    camera_follow_system_new, camera_zoom_system,
    load_map_system_new,
    setup_game_rendering, cleanup_game_rendering,
};

use bevy_modules::systems::{
    mouse_input_system, 
    keyboard_input_system,
    movement_system,
    render_offset_system,
    animation_system,
    camera_follow_system as camera_follow_system_old,  // 旧的摄像机系统
    spawn_test_player,
    debug_info_system,
    setup_map_system,
    map_culling_system,
    setup_debug_ui,
    update_fps_system,
    update_player_info_system,
};

// 引入 LoginScene V2 (完整功能版本)
use bevy_modules::scenes::{
    setup_login_scene,
    init_network_channel,
    cleanup_login_scene,
    update_background_animation,
    handle_text_input,
    update_input_display,
    handle_input_focus,
    handle_tab_focus,
    update_input_borders,
    update_cursor_blink,
    handle_button_hover as login_handle_button_hover,
    handle_button_press,
    handle_button_clicks,
    handle_dialog_buttons,  // 对话框按钮处理
    handle_dialog_text_input,  // 对话框输入处理
    update_dialog_input_display,  // 对话框输入显示
    handle_dialog_tab_focus,  // 对话框Tab切换
    handle_dialog_input_click,  // 对话框输入点击
    update_dialog_input_borders,  // 对话框输入边框更新
    update_dialog_cursor_visibility,  // 对话框光标可见性
    handle_login_message,
    handle_close_message,
    handle_new_account_message,
    handle_password_change_message,
    handle_view_key_message,
    // LoginScene Messages
    LoginButtonPressed,
    NewAccountButtonPressed,
    PasswordChangeButtonPressed,
    ViewKeyButtonPressed,
    CloseButtonPressed,
    // SelectScene（新架构）
    setup_select_scene,
    cleanup_select_scene,
    init_select_network_channel,
    update_character_animation,
    update_character_slots,
    update_button_textures,
    handle_select_button_clicks,
    handle_slot_clicks,
    update_slot_texts,
    update_slot_text_colors,
    handle_slot_hover,
    DialogState,
    handle_dialog_button_clicks,
    handle_dialog_button_hover,
    update_dialog_character_preview,
    // GameScene
    setup_game_scene,
    cleanup_game_scene,
    update_game_time,
    handle_player_input,
    handle_player_movement,
    update_player_position,
    update_hud_display,
    handle_quickslot_hover,
    message_handle_player_move,
    message_handle_open_chat,
    message_handle_close_chat,
    message_handle_open_inventory,
    message_handle_close_inventory,
    message_handle_open_skills,
    message_handle_close_skills,
    message_handle_pause_game,
    message_handle_exit_game,
    message_handle_interact_npc,
    message_handle_use_skill,
    // Phase 1: 玩家实体管理
    update_player_stats_system,
    process_buffs_system,
    // Phase 2: 地图加载和渲染
    load_map_system,
    create_map_layers_system,
    spawn_map_objects_system,
    update_map_state_system,
    handle_map_collision_system,
    // Phase 3: NPC 和对象交互
    setup_dialogue_system,
    detect_interaction_system,
    handle_interaction_system,
    update_dialogue_display_system,
    handle_dialogue_choice_system,
    message_handle_npc_dialogue,
    // Phase 4: 聊天系统
    setup_chat_system,
    process_chat_input_system,
    process_chat_commands_system,
    receive_chat_messages_system,
    update_chat_display_system,
    manage_chat_history_system,
    message_handle_send_chat,
    // Phase 5: 网络同步系统
    setup_network_system,
    send_player_position_system,
    send_player_stats_system,
    send_chat_to_server_system,
    send_interaction_to_server_system,
    receive_player_sync_system,
    receive_npc_sync_system,
    receive_map_sync_system,
    receive_server_chat_system,
    handle_connection_events_system,
    apply_player_sync_system,
    apply_npc_sync_system,
    apply_item_spawn_system,
    sync_local_state_system,
    // Phase 6: 完整事件循环
    setup_game_loop_system,
    game_loop_system,
    process_frame_events_system,
    update_frame_stats_system,
    check_win_lose_conditions_system,
    integrate_all_systems_system,
    validate_game_state_system,
    handle_game_errors_system,
    debug_system_health_system,
    optimize_network_updates_system,
    optimize_render_system,
    profile_system_performance_system,
    message_handle_game_loop,
    message_handle_frame_stats_request,
    message_handle_system_health_request,
    message_handle_performance_report,
    // GameScene Messages
    PlayerMoveMessage,
    PlayerStopMessage,
    OpenChatMessage,
    CloseChatMessage,
    SendChatMessage,
    OpenInventoryMessage,
    CloseInventoryMessage,
    OpenSkillsMessage,
    CloseSkillsMessage,
    OpenCharacterMessage,
    CloseCharacterMessage,
    ExitGameMessage,
    InteractWithNpcMessage,
    UseSkillMessage,
    PauseGameMessage,
    // Phase 5: 网络同步消息
    PlayerSyncMessage,
    PlayerStatsSyncMessage,
    RemotePlayerSyncMessage,
    NPCSyncMessage,
    MapObjectSyncMessage,
    ChatSyncMessage,
    ItemSpawnMessage,
    ItemDespawnMessage,
    ConnectionEvent,
    NetworkErrorMessage,
    ServerTimeSyncMessage,
    // Phase 6: 游戏循环消息
    GameLoopMessage,
    RequestFrameStatsMessage,
    RequestSystemHealthMessage,
    PerformanceReportMessage,
};

fn main() {
    println!("=== 程序启动 ===");
    println!("1. 准备创建 App...");
    
    let mut app = App::new();
    println!("2. App 已创建");
    
    app.add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "传奇客户端 - Bevy 0.17.2".to_string(),
                        resolution: WindowResolution::new(1024, 768),
                        resizable: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()), // 像素风格游戏使用最近邻插值
        );
    println!("3. 插件已添加");
    
    // 注册 LoginScene 使用的所有 Message 类型
    app.add_message::<LoginButtonPressed>();
    app.add_message::<NewAccountButtonPressed>();
    app.add_message::<PasswordChangeButtonPressed>();
    app.add_message::<ViewKeyButtonPressed>();
    app.add_message::<CloseButtonPressed>();
    
    // SelectScene 新架构不使用 Message 系统（直接通过 Interaction 处理）
    
    // 注册 GameScene 使用的所有 Message 类型
    app.add_message::<PlayerMoveMessage>();
    app.add_message::<PlayerStopMessage>();
    app.add_message::<OpenChatMessage>();
    app.add_message::<CloseChatMessage>();
    app.add_message::<SendChatMessage>();
    app.add_message::<OpenInventoryMessage>();
    app.add_message::<CloseInventoryMessage>();
    app.add_message::<OpenSkillsMessage>();
    app.add_message::<CloseSkillsMessage>();
    app.add_message::<OpenCharacterMessage>();
    app.add_message::<CloseCharacterMessage>();
    app.add_message::<ExitGameMessage>();
    app.add_message::<InteractWithNpcMessage>();
    app.add_message::<UseSkillMessage>();
    app.add_message::<PauseGameMessage>();
    println!("3.5. Message 类型已注册");
    
    // 注册 Phase 5 网络同步相关的 Message 类型
    app.add_message::<PlayerSyncMessage>();
    app.add_message::<PlayerStatsSyncMessage>();
    app.add_message::<RemotePlayerSyncMessage>();
    app.add_message::<NPCSyncMessage>();
    app.add_message::<MapObjectSyncMessage>();
    app.add_message::<ChatSyncMessage>();
    app.add_message::<ItemSpawnMessage>();
    app.add_message::<ItemDespawnMessage>();
    app.add_message::<ConnectionEvent>();
    app.add_message::<NetworkErrorMessage>();
    app.add_message::<ServerTimeSyncMessage>();
    println!("3.6. Phase 5 网络消息类型已注册");
    
    // 注册 Phase 6 游戏循环相关的 Message 类型
    app.add_message::<GameLoopMessage>();
    app.add_message::<RequestFrameStatsMessage>();
    app.add_message::<RequestSystemHealthMessage>();
    app.add_message::<PerformanceReportMessage>();
    println!("3.7. Phase 6 游戏循环消息类型已注册");
        
    // 初始化状态
    app.init_state::<GameState>();
    println!("4. 状态已初始化");
        
    // 初始化资源
    app.insert_resource(GameConfig::default());
    println!("6. GameConfig 已插入");
    
    app.insert_resource(MLibraryAssets::new());
    println!("7. MLibraryAssets 已插入");
    
    app.insert_resource(MapAssets::new());
    println!("8. MapAssets 已插入");
    
    app.insert_resource(MLibraryResource::new());
    println!("9. MLibraryResource 已插入");
    
    // GameScene 渲染资源 (新架构)
    app.insert_resource(MapRenderData::default());
    println!("9.1. MapRenderData 已插入");
    
    app.insert_resource(MapLoadRequest::default());
    println!("9.2. MapLoadRequest 已插入");
        
    // 启动系统
    app.add_systems(Startup, (
        setup, 
        load_mlibrary_system, 
        setup_debug_ui,
        init_global_network_manager,  // 初始化全局 NetworkManager
    ));
    println!("10. Startup 系统已添加 (包含 NetworkManager)");
        
    // 通用更新系统 (所有状态都运行)
    app.add_systems(Update, (
        keyboard_input_system,
        animation_system,
        update_fps_system,
        update_player_info_system,
        process_network_events,  // 处理网络事件
        debug_shortcuts_system,  // 调试快捷键 (F1-F5, ESC)
    ));
    println!("11. 通用 Update 系统已添加 (包含网络事件处理和调试快捷键)");
        
    // LoginScene V2 系统 - 输入处理 (仅在 Login 状态运行)
    app.add_systems(Update, (
        // 背景动画
        update_background_animation,
        // 登录输入处理
        handle_text_input,
        update_input_display,
        handle_input_focus,
        handle_tab_focus,
        update_input_borders,
        update_cursor_blink,
    ).run_if(in_state(GameState::Login)));
    
    // LoginScene V2 系统 - 对话框处理 (仅在 Login 状态运行)
    app.add_systems(Update, (
        // 对话框输入处理
        handle_dialog_text_input,
        update_dialog_input_display,
        handle_dialog_tab_focus,
        handle_dialog_input_click,
        update_dialog_input_borders,
        update_dialog_cursor_visibility,
    ).run_if(in_state(GameState::Login)));
    
    // LoginScene V2 系统 - 按钮和消息 (仅在 Login 状态运行)
    app.add_systems(Update, (
        // 按钮交互
        login_handle_button_hover,
        handle_button_press,
        handle_button_clicks,
        handle_dialog_buttons,  // 对话框按钮处理
        // 消息处理
        handle_login_message,
        handle_close_message,
        handle_new_account_message,
        handle_password_change_message,
        handle_view_key_message,
    ).run_if(in_state(GameState::Login)));
    println!("12. LoginScene V2 系统已添加");
    
    // SelectScene 系统 - 新架构（参考 ggez 版本）
    app.add_systems(Update, (
        update_character_animation,      // 角色预览动画
        update_character_slots,          // 角色槽位纹理
        update_button_textures,          // 按钮悬停/按下状态
        handle_select_button_clicks,     // 按钮点击处理
        handle_slot_clicks,              // 角色槽点击选择
        update_slot_texts,               // 角色槽文本更新
        update_slot_text_colors,         // 角色槽文本颜色高亮
        handle_slot_hover,               // 角色槽悬停效果
        handle_dialog_button_clicks,     // 对话框按钮点击
        handle_dialog_button_hover,      // 对话框按钮悬停
        update_dialog_character_preview, // 对话框角色预览动画
    ).run_if(in_state(GameState::Select)));
    println!("12.5. SelectScene 系统已添加（纹理系统重构版 + 角色槽交互 + 对话框）");
    
    // GameScene 系统 - UI 更新 (仅在 Game 状态运行)
    app.add_systems(Update, (
        update_game_time,
        update_hud_display,
    ).run_if(in_state(GameState::Game)));
    
    // GameScene 系统 - 玩家控制 (仅在 Game 状态运行)
    app.add_systems(Update, (
        handle_player_input,
        handle_player_movement,
        update_player_position,
    ).run_if(in_state(GameState::Game)));
    
    // GameScene 系统 - 消息处理和 UI 交互 (仅在 Game 状态运行)
    app.add_systems(Update, (
        handle_quickslot_hover,
        // 消息处理
        message_handle_player_move,
        message_handle_open_chat,
        message_handle_close_chat,
        message_handle_open_inventory,
        message_handle_close_inventory,
        message_handle_open_skills,
        message_handle_close_skills,
        message_handle_pause_game,
        message_handle_exit_game,
        message_handle_interact_npc,
        message_handle_use_skill,
    ).run_if(in_state(GameState::Game)));
    
    // GameScene 系统 - Phase 1 玩家管理
    app.add_systems(Update, (
        update_player_stats_system,
        process_buffs_system,
        process_chat_input_system,
        update_chat_display_system,
    ).run_if(in_state(GameState::Game)));
    
    // GameScene 系统 - Phase 2 地图系统
    app.add_systems(Update, (
        update_map_state_system,
        handle_map_collision_system,
    ).run_if(in_state(GameState::Game)));
    
    // GameScene 系统 - Phase 3 交互系统
    app.add_systems(Update, (
        detect_interaction_system,
        handle_interaction_system,
        handle_dialogue_choice_system,
        update_dialogue_display_system,
        message_handle_npc_dialogue,
    ).run_if(in_state(GameState::Game)));
    
    // GameScene 系统 - Phase 4 聊天系统
    app.add_systems(Update, (
        process_chat_input_system,
        process_chat_commands_system,
        receive_chat_messages_system,
        manage_chat_history_system,
        update_chat_display_system,
        message_handle_send_chat,
    ).run_if(in_state(GameState::Game)));
    
    // GameScene 系统 - Phase 5 网络同步
    app.add_systems(Update, (
        // 网络发送系统
        send_player_position_system,
        send_player_stats_system,
        send_chat_to_server_system,
        send_interaction_to_server_system,
        // 网络接收系统
        receive_player_sync_system,
        receive_npc_sync_system,
        receive_map_sync_system,
        receive_server_chat_system,
        handle_connection_events_system,
        // 同步应用系统
        apply_player_sync_system,
        apply_npc_sync_system,
        apply_item_spawn_system,
        sync_local_state_system,
    ).run_if(in_state(GameState::Game)));
    
    // GameScene 系统 - Phase 6 完整事件循环
    app.add_systems(Update, (
        // 游戏循环核心
        game_loop_system,
        process_frame_events_system,
        update_frame_stats_system,
        check_win_lose_conditions_system,
        // 系统整合和验证
        integrate_all_systems_system,
        validate_game_state_system,
        handle_game_errors_system,
        debug_system_health_system,
        // 性能优化
        optimize_network_updates_system,
        optimize_render_system,
        profile_system_performance_system,
        // 消息处理
        message_handle_game_loop,
        message_handle_frame_stats_request,
        message_handle_system_health_request,
        message_handle_performance_report,
    ).run_if(in_state(GameState::Game)));
    
    println!("13. GameScene 系统已添加 (Phase 1-6, 分组注册)");
    
    // GameScene 新渲染系统 (Phase 4: 摄像机 + Phase 5: 地图加载)
    app.add_systems(Update, (
        update_animation_system,     // 更新地图动画
        load_map_system_new,         // 地图加载系统 (新)
        render_map_system,           // 地图渲染系统 (新)
        camera_follow_system_new,    // 摄像机跟随系统 (新)
        camera_zoom_system,          // 摄像机缩放系统
    ).run_if(in_state(GameState::Game)));
    println!("13.1. GameScene 新渲染系统已添加 (Phase 4-5)");
        
    // 游戏中的系统 (仅在 Game 状态运行,原有的地图/渲染系统)
    app.add_systems(Update, (
        mouse_input_system,
        movement_system,
        render_offset_system,
        camera_follow_system_old,  // 使用旧的摄像机系统
        debug_info_system,
        map_culling_system,
    ).run_if(in_state(GameState::Game)));
    println!("13.5. 原有 Game 系统已添加");
        
    // 进入登录状态时设置 LoginScene
    app.add_systems(OnEnter(GameState::Login), (setup_login_scene, init_network_channel));
    println!("14. OnEnter(Login) 系统已添加");
        
    // 退出登录状态时清理 LoginScene
    app.add_systems(OnExit(GameState::Login), cleanup_login_scene);
    println!("15. OnExit(Login) 系统已添加");
    
    // 进入角色选择状态时设置 SelectScene
    app.add_systems(OnEnter(GameState::Select), (setup_select_scene, init_select_network_channel));
    println!("15.5. OnEnter(Select) 系统已添加");
    
    // 退出角色选择状态时清理 SelectScene
    app.add_systems(OnExit(GameState::Select), cleanup_select_scene);
    println!("15.7. OnExit(Select) 系统已添加");
    
    // 进入游戏状态时设置 GameScene HUD
    app.add_systems(OnEnter(GameState::Game), (
        setup_game_scene, 
        setup_game_rendering,  // 新架构: 初始化摄像机和加载地图
        spawn_test_player, 
        setup_map_system,
        load_map_system,  // Phase 2: 加载地图
        create_map_layers_system,  // Phase 2: 创建地图图层
        spawn_map_objects_system,  // Phase 2: 生成地图对象
        setup_dialogue_system,  // Phase 3: 初始化对话系统
        setup_chat_system,  // Phase 4: 初始化聊天系统
        setup_network_system,  // Phase 5: 初始化网络系统
        setup_game_loop_system,  // Phase 6: 初始化游戏循环系统
    ));
    println!("16. OnEnter(Game) 系统已添加 (包含 GameScene 和 Phase 2-6 系统)");
    
    // 退出游戏状态时清理 GameScene
    app.add_systems(OnExit(GameState::Game), (cleanup_game_scene, cleanup_game_rendering));
    println!("16.5. OnExit(Game) 系统已添加");
    
    println!("=== 准备运行 App.run() ===");
    app.run();
    println!("=== App.run() 已结束 (不应该看到这条消息) ===");
}

/// 启动系统 - 创建摄像机和基础设置
fn setup(mut commands: Commands, mut next_state: ResMut<NextState<GameState>>) {
    // 生成 2D 摄像机
    commands.spawn(Camera2d::default());
    
    println!("✅ Bevy 原型启动成功!");
    println!("🎮 窗口大小: 1024x768");
    println!("📦 插件: DefaultPlugins + 最近邻插值");
    println!("🏗️ ECS 架构初始化完成");
    println!("📊 状态机: Loading -> Login -> Select -> Game");
    
    // 进入登录状态
    next_state.set(GameState::Login);
    println!("🎮 进入登录状态");
}
