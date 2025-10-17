use bevy::prelude::*;
use bevy::window::WindowResolution;

// 引入 Bevy 模块
mod bevy_modules {
    pub use mir2_client::bevy::*;
}

use bevy_modules::{GameState, GameConfig, MLibraryAssets, MapAssets};
use bevy_modules::{MLibraryResource, load_mlibrary_system};
use bevy_modules::systems::{
    mouse_input_system, 
    keyboard_input_system,
    movement_system,
    render_offset_system,
    animation_system,
    camera_follow_system,
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
    cleanup_login_scene,
    update_background_animation,
    handle_text_input,
    update_input_display,
    handle_input_focus,
    handle_tab_focus,
    update_input_borders,
    update_cursor_blink,
    handle_button_hover,
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
    // Messages
    LoginButtonPressed,
    NewAccountButtonPressed,
    PasswordChangeButtonPressed,
    ViewKeyButtonPressed,
    CloseButtonPressed,
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
    println!("3.5. Message 类型已注册");
        
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
        
    // 启动系统
    app.add_systems(Startup, (setup, load_mlibrary_system, setup_debug_ui));
    println!("10. Startup 系统已添加");
        
    // 通用更新系统 (所有状态都运行)
    app.add_systems(Update, (
        keyboard_input_system,
        animation_system,
        update_fps_system,
        update_player_info_system,
    ));
    println!("11. 通用 Update 系统已添加");
        
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
        handle_button_hover,
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
        
    // 游戏中的系统 (仅在 Game 状态运行)
    app.add_systems(Update, (
        mouse_input_system,
        movement_system,
        render_offset_system,
        camera_follow_system,
        debug_info_system,
        map_culling_system,
    ).run_if(in_state(GameState::Game)));
    println!("13. Game 系统已添加");
        
    // 进入登录状态时设置 LoginScene
    app.add_systems(OnEnter(GameState::Login), setup_login_scene);
    println!("14. OnEnter(Login) 系统已添加");
        
    // 退出登录状态时清理 LoginScene
    app.add_systems(OnExit(GameState::Login), cleanup_login_scene);
    println!("15. OnExit(Login) 系统已添加");
        
    // 进入游戏状态时生成测试玩家和地图
    app.add_systems(OnEnter(GameState::Game), (spawn_test_player, setup_map_system));
    println!("16. OnEnter(Game) 系统已添加");
    
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
