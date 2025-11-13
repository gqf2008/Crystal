// ============================================================================
// GameState - 游戏状态管理器
// ============================================================================
//
// 职责：
// - 管理当前场景
// - 游戏主循环（run）
// - 监听场景切换请求

use crate::compat::GameResult;
use crate::core::GameError;
use crate::scenes::*;
use macroquad::prelude::*;

/// 游戏主状态
pub struct GameState {
    /// 当前场景
    current_scene: Scene,
}

impl GameState {
    /// 创建游戏状态
    pub async fn new() -> GameResult<Self> {
        // 加载字体
        let font_data = include_bytes!("../assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf");
        let _font = load_ttf_font_from_bytes(font_data)
            .map_err(|e| GameError::ResourceLoadError(format!("字体加载失败: {}", e)))?;
        
        // 创建初始场景（登录）
        let mut initial_scene = Scene::Login(LoginScene::new());
        initial_scene.on_enter()?;
        
        Ok(Self {
            current_scene: initial_scene,
        })
    }
    
    /// 游戏主循环
    pub async fn run(mut self) -> GameResult {
        println!("🎮 游戏启动: {}", self.current_scene.name());
        
        loop {
            let dt = get_frame_time();
            
            // 处理输入
            self.current_scene.handle_input()?;
            
            // 更新场景，获取切换请求
            let transition = self.current_scene.update(dt)?;
            
            // 渲染场景
            self.current_scene.render()?;
            
            // 处理场景切换
            match transition {
                SceneTransition::None => {
                    // 继续当前场景
                }
                SceneTransition::Exit => {
                    println!("👋 游戏退出");
                    break;
                }
                other => {
                    // 切换场景
                    self.switch_to(other)?;
                }
            }
            
            next_frame().await;
        }
        
        Ok(())
    }
    
    /// 切换场景
    fn switch_to(&mut self, transition: SceneTransition) -> GameResult {
        // 离开当前场景
        self.current_scene.on_exit()?;
        
        // 创建新场景
        let mut new_scene = match transition {
            SceneTransition::Login => Scene::Login(LoginScene::new()),
            SceneTransition::CharacterSelect => Scene::CharacterSelect(CharacterSelectScene::new()),
            SceneTransition::Game => Scene::Game(GameScene::new()),
            SceneTransition::Loading => Scene::Loading(LoadingScene::new()),
            SceneTransition::None | SceneTransition::Exit => {
                return Ok(());
            }
        };
        
        println!("🎬 场景切换: {} → {}", self.current_scene.name(), new_scene.name());
        
        // 进入新场景
        new_scene.on_enter()?;
        
        // 替换场景
        self.current_scene = new_scene;
        
        Ok(())
    }
}

