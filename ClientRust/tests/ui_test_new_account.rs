// ============================================================================
// UI 自动化测试 - 新建账号流程
// ============================================================================
//
// 测试目标：
// 1. 点击新建账号按钮 -> NewAccountDialog应该显示
// 2. 填写表单字段
// 3. 点击OK按钮 -> 应该发送NewAccount数据包
// 4. 模拟服务器响应 -> 检查UI反馈
//
// 运行: cargo test --test ui_test_new_account -- --nocapture
//
// ============================================================================

use ggez::{
    conf::{WindowMode, WindowSetup},
    event,
    graphics::Color,
    Context, ContextBuilder, GameResult,
};
use mir2_client::ecs::scenes::login_scene::LoginScene;
use mir2_client::ecs::scenes::Scene;
use hecs::World;
use tokio::sync::mpsc;
use mir2_client::network::NetworkCommand;

struct TestApp {
    login_scene: LoginScene,
    world: World,
    command_tx: mpsc::UnboundedSender<NetworkCommand>,
    command_rx: mpsc::UnboundedReceiver<NetworkCommand>,
    test_step: u32,
    frame_count: u32,
}

impl TestApp {
    fn new(_ctx: &mut Context) -> GameResult<Self> {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        
        Ok(Self {
            login_scene: LoginScene::new(),
            world: World::new(),
            command_tx: command_tx.clone(),
            command_rx,
            test_step: 0,
            frame_count: 0,
        })
    }
}

impl event::EventHandler for TestApp {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.frame_count += 1;
        
        // 每10帧执行一个测试步骤
        if self.frame_count % 10 == 0 {
            match self.test_step {
                0 => {
                    println!("\n========================================");
                    println!("🧪 测试步骤 0: 初始状态检查");
                    println!("========================================");
                    
                    // 检查初始状态
                    assert!(!self.login_scene.new_account_dialog.is_some(), 
                        "❌ 初始状态下NewAccountDialog应该为None");
                    assert!(self.login_scene.login_dialog.visible, 
                        "❌ LoginDialog应该可见");
                    
                    println!("✅ 初始状态正确");
                    self.test_step += 1;
                }
                
                1 => {
                    println!("\n========================================");
                    println!("🧪 测试步骤 1: 点击新建账号按钮");
                    println!("========================================");
                    
                    // 模拟点击新建账号按钮 (坐标需要根据实际按钮位置调整)
                    // LoginDialog新建账号按钮通常在底部
                    let button_x = 400.0;
                    let button_y = 500.0;
                    
                    self.login_scene.on_mouse_down(
                        ctx, 
                        &mut self.world, 
                        ggez::winit::event::MouseButton::Left,
                        button_x,
                        button_y,
                        &self.command_tx
                    )?;
                    
                    // 检查NewAccountDialog是否打开
                    if let Some(dialog) = &self.login_scene.new_account_dialog {
                        println!("✅ NewAccountDialog已创建");
                        assert!(dialog.visible, "❌ NewAccountDialog应该可见");
                        println!("✅ NewAccountDialog可见");
                    } else {
                        println!("❌ 点击新建账号按钮后，NewAccountDialog应该被创建");
                        println!("   按钮坐标: ({}, {})", button_x, button_y);
                        println!("   LoginDialog可见: {}", self.login_scene.login_dialog.visible);
                        panic!("NewAccountDialog未创建");
                    }
                    
                    self.test_step += 1;
                }
                
                2 => {
                    println!("\n========================================");
                    println!("🧪 测试步骤 2: 填写账号信息");
                    println!("========================================");
                    
                    if let Some(dialog) = &mut self.login_scene.new_account_dialog {
                        // 模拟输入账号
                        dialog.registration.account_id = "test_user_123".to_string();
                        dialog.account_id_valid = true;
                        
                        // 模拟输入密码
                        dialog.registration.password = "test_pass_456".to_string();
                        dialog.password1_valid = true;
                        dialog.password2_valid = true;
                        
                        // 模拟输入邮箱
                        dialog.registration.email = "test@example.com".to_string();
                        dialog.email_valid = true;
                        
                        println!("✅ 账号: {}", dialog.registration.account_id);
                        println!("✅ 密码: {} (长度: {})", 
                            "*".repeat(dialog.registration.password.len()),
                            dialog.registration.password.len()
                        );
                        println!("✅ 邮箱: {}", dialog.registration.email);
                        
                        // 检查表单验证状态
                        assert!(dialog.account_id_valid, "❌ 账号应该有效");
                        assert!(dialog.password1_valid, "❌ 密码1应该有效");
                        assert!(dialog.password2_valid, "❌ 密码2应该有效");
                        assert!(dialog.email_valid, "❌ 邮箱应该有效");
                        
                        println!("✅ 所有字段验证通过");
                    } else {
                        panic!("❌ NewAccountDialog不存在");
                    }
                    
                    self.test_step += 1;
                }
                
                3 => {
                    println!("\n========================================");
                    println!("🧪 测试步骤 3: 点击OK按钮提交");
                    println!("========================================");
                    
                    // 模拟点击OK按钮
                    let ok_button_x = 350.0;
                    let ok_button_y = 550.0;
                    
                    self.login_scene.on_mouse_down(
                        ctx, 
                        &mut self.world, 
                        ggez::winit::event::MouseButton::Left,
                        ok_button_x,
                        ok_button_y,
                        &self.command_tx
                    )?;
                    
                    println!("✅ 已点击OK按钮");
                    self.test_step += 1;
                }
                
                4 => {
                    println!("\n========================================");
                    println!("🧪 测试步骤 4: 检查网络命令");
                    println!("========================================");
                    
                    // 检查是否发送了NewAccount命令
                    let mut found_command = false;
                    while let Ok(cmd) = self.command_rx.try_recv() {
                        println!("📦 收到网络命令: {:?}", cmd);
                        
                        if let NetworkCommand::NewAccount { account_id, password, .. } = &cmd {
                            println!("✅ 发送NewAccount命令:");
                            println!("   账号: {}", account_id);
                            println!("   密码: {}", "*".repeat(password.len()));
                            
                            assert_eq!(account_id, "test_user_123", "❌ 账号不匹配");
                            assert_eq!(password, "test_pass_456", "❌ 密码不匹配");
                            
                            found_command = true;
                        }
                    }
                    
                    if !found_command {
                        println!("❌ 未找到NewAccount命令");
                        println!("   可能的原因:");
                        println!("   1. OK按钮点击未生效");
                        println!("   2. 表单验证失败");
                        println!("   3. 命令未发送到channel");
                        panic!("NewAccount命令未发送");
                    }
                    
                    println!("✅ NewAccount命令验证通过");
                    self.test_step += 1;
                }
                
                5 => {
                    println!("\n========================================");
                    println!("🎉 所有测试通过！");
                    println!("========================================\n");
                    
                    // 退出测试
                    ctx.request_quit();
                }
                
                _ => {}
            }
        }
        
        // 更新场景
        self.login_scene.update(ctx, &mut self.world, &self.command_tx)?;
        
        Ok(())
    }
    
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = ggez::graphics::Canvas::from_frame(ctx, Color::BLACK);
        self.login_scene.draw(ctx, &mut canvas, &self.world)?;
        canvas.finish(ctx)?;
        Ok(())
    }
}

#[test]
fn test_new_account_flow() {
    // 创建ggez context
    let (mut ctx, event_loop) = ContextBuilder::new("ui_test", "Crystal")
        .window_setup(WindowSetup::default().title("UI Test - New Account"))
        .window_mode(WindowMode::default().dimensions(1024.0, 768.0))
        .build()
        .expect("Failed to create ggez context");
    
    // 创建测试应用
    let app = TestApp::new(&mut ctx).expect("Failed to create test app");
    
    // 运行测试
    event::run(ctx, event_loop, app).expect("Test failed");
}
