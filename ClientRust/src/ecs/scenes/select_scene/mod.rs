// SelectScene - Character selection scene
// Mirrors Client/MirScenes/SelectScene.cs

// UI 组件子模块（每个组件负责自己的绘制和事件处理）
mod character_select;
pub mod credits_dialog;
pub mod delete_character_dialog;
mod message_box; // 🆕 SelectScene 专用消息框
pub mod new_character_dialog; // 🆕 角色选择主界面绘制

// 业务逻辑子模块（按照单一职责原则分离）
mod network_handler;
mod ui_actions; // UI 交互逻辑（按钮点击、对话框打开、游戏启动等） // 网络事件处理（服务器响应处理）
mod input_handler; // 输入事件处理（键盘、鼠标、IME等）

use crate::ecs::components::CharacterList;
use crate::ecs::{Coord, GameContext, WorldExt};
use character_select::CharacterSelect; // 🆕 导入CharacterList组件

pub use credits_dialog::CreditsDialog;
pub use delete_character_dialog::DeleteCharacterDialog;
pub use message_box::{MessageBox, MessageBoxButtons, MessageBoxResult};
pub use new_character_dialog::NewCharacterDialog; // 🆕 导出消息框

use ggez::graphics::Canvas;
use ggez::{Context, GameResult};
use hecs::World;
use std::sync::Arc;

use super::ui::InputBox; // 🆕 只导入 InputBox
use super::{Scene, SceneType};
use crate::ecs::ui::{ButtonGroup, ButtonWidget}; // ButtonGroup 从原路径导入
use crate::network::{handlers::GameEvent, NetContext};
use mir2_shared::SelectInfo;

pub struct SelectScene {
    // 🆕 角色选择UI状态(只维护选中索引、动画帧等UI状态)
    character_select_ui: CharacterSelect,

    // Dialogs
    /// Mirrors C# `private NewCharacterDialog _character`
    pub new_character_dialog: Option<NewCharacterDialog>,
    /// Delete character dialog
    pub delete_character_dialog: Option<DeleteCharacterDialog>,
    /// Credits dialog
    pub credits_dialog: Option<CreditsDialog>,

    // 🆕 消息框和输入框
    pub message_box: Option<MessageBox>,
    pub input_box: Option<InputBox>,

    // Scene transition
    pub pending_scene_change: Option<SceneType>,

    // 🆕 按钮组管理器(替代手动状态管理)
    bottom_buttons: ButtonGroup,

    // UI state (保留用于对话框)
    hovered_button: Option<BottomButton>, // TODO: 可以删除,由 ButtonGroup 管理
    pressed_button: Option<BottomButton>, // TODO: 可以删除,由 ButtonGroup 管理
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BottomButton {
    StartGame,
    NewCharacter,
    DeleteCharacter,
    Credits,
    ExitGame,
}

impl SelectScene {
    /// Create new select scene
    ///
    /// 🆕 ECS 架构: 角色数据从 World 查询,不再通过参数传递
    ///
    /// Mirrors C# constructor:
    /// ```csharp
    /// public SelectScene(List<SelectInfo> characters)
    /// {
    ///     Characters = characters;
    ///     SortList();
    ///     // ... initialize UI
    /// }
    /// ```
    pub fn new() -> Self {
        // 创建底部按钮组
        let mut bottom_buttons = ButtonGroup::new();

        // 按钮布局常量
        const BUTTON_Y: f32 = 736.0;
        const BUTTON_WIDTH: f32 = 96.0;
        const BUTTON_HEIGHT: f32 = 32.0;
        const BUTTON_SPACING: f32 = 150.0;
        const BUTTON_START_X: f32 = 100.0;

        // 添加5个底部按钮 (使用 Builder 模式添加工具提示)
        bottom_buttons.add(
            ButtonWidget::new(
                1,
                BUTTON_START_X,
                BUTTON_Y,
                BUTTON_WIDTH,
                BUTTON_HEIGHT,
                340,
            )
            .with_tooltip("开始游戏 (Enter)"),
        );
        bottom_buttons.add(
            ButtonWidget::new(
                2,
                BUTTON_START_X + BUTTON_SPACING,
                BUTTON_Y,
                BUTTON_WIDTH,
                BUTTON_HEIGHT,
                343,
            )
            .with_tooltip("新建角色"),
        );
        bottom_buttons.add(
            ButtonWidget::new(
                3,
                BUTTON_START_X + BUTTON_SPACING * 2.0,
                BUTTON_Y,
                BUTTON_WIDTH,
                BUTTON_HEIGHT,
                346,
            )
            .with_tooltip("删除角色 (Delete)"),
        );
        bottom_buttons.add(
            ButtonWidget::new(
                4,
                BUTTON_START_X + BUTTON_SPACING * 3.0,
                BUTTON_Y,
                BUTTON_WIDTH,
                BUTTON_HEIGHT,
                349,
            )
            .with_tooltip("制作人员"),
        );
        bottom_buttons.add(
            ButtonWidget::new(
                5,
                BUTTON_START_X + BUTTON_SPACING * 4.0,
                BUTTON_Y,
                BUTTON_WIDTH,
                BUTTON_HEIGHT,
                352,
            )
            .with_tooltip("退出游戏 (ESC)"),
        );

        // 🆕 ECS架构: CharacterSelect只维护UI状态
        let character_select_ui = CharacterSelect::new();

        Self {
            character_select_ui,
            new_character_dialog: None,
            delete_character_dialog: None,
            credits_dialog: None,
            message_box: None,
            input_box: None,
            pending_scene_change: None,
            bottom_buttons, // 🆕 使用 ButtonGroup
            hovered_button: None,
            pressed_button: None,
        }
    }

    /// Select character by index
    ///
    /// Mirrors C# CharacterButton click handler:
    /// ```csharp
    /// _selected = index;
    /// UpdateInterface();
    /// ```
    pub fn select_character(&mut self, index: i32, world: &mut hecs::World) {
        // 更新World中的选择状态
        if let Some((_, mut char_list)) = world.query::<&mut CharacterList>().iter().next() {
            char_list.set_selected(index);
            tracing::debug!("🎯 选择角色索引: {}", index);
        }

        // 同步到UI层 (用于动画等)
        self.character_select_ui.select_character(index);
        if let Some(character) = self.character_select_ui.get_selected_character() {
            println!("Selected character: {}", character.name);
        }
    }

    /// 获取当前选中的角色索引 (从World读取)
    pub fn get_selected_character_index(&self, world: &hecs::World) -> i32 {
        if let Some((_, char_list)) = world.query::<&CharacterList>().iter().next() {
            char_list.selected_index
        } else {
            -1
        }
    }

    fn window_to_design_coords(&self, ctx: &Context, window_x: f32, window_y: f32) -> (f32, f32) {
        let (window_width, window_height) = ctx.gfx.drawable_size();
        // 计算4:3视口
        let aspect_ratio = 4.0 / 3.0;
        let current_ratio = window_width / window_height;

        let (viewport_width, viewport_height) = if current_ratio > aspect_ratio {
            (window_height * aspect_ratio, window_height)
        } else {
            (window_width, window_width / aspect_ratio)
        };

        let offset_x = (window_width - viewport_width) / 2.0;
        let offset_y = (window_height - viewport_height) / 2.0;

        // 转换：窗口坐标 -> 视口坐标 -> 设计坐标
        let viewport_x = window_x - offset_x;
        let viewport_y = window_y - offset_y;

        let design_x = (viewport_x / viewport_width) * Coord::DESIGN_WIDTH;
        let design_y = (viewport_y / viewport_height) * Coord::DESIGN_HEIGHT;

        (design_x, design_y)
    }
}

impl Default for SelectScene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene for SelectScene {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, game_ctx: &mut crate::ecs::GameContext) -> GameResult<Option<SceneType>> {
        self.handle_input_event(game_ctx)?;

        // 🔄 从World同步角色列表到UI缓存
        // 🆕 正确架构: World存储CharacterList组件(单一实体)
        if self.character_select_ui.is_empty() {
            // 📦 Step 1: 先提取数据(只在借用作用域内)
            let (should_select, first_name) = {
                if let Some((entity, char_list)) = game_ctx.world
                    .query::<&crate::ecs::components::CharacterList>()
                    .iter()
                    .next()
                {
                    tracing::info!(
                        "💾 从World加载CharacterList: Entity({:?}), {} 个角色",
                        entity,
                        char_list.characters.len()
                    );

                    // 同步到UI缓存(排序后)
                    let mut characters = char_list.characters.clone();
                    characters.sort_by(|a, b| b.last_access.cmp(&a.last_access));

                    for character in &characters {
                        self.character_select_ui.add_character(character.clone());
                    }

                    tracing::info!(
                        "✅ UI缓存同步完成: {} 个角色",
                        self.character_select_ui.len()
                    );

                    // 准备默认选中数据
                    let should_select = !characters.is_empty();
                    let first_name = if should_select {
                        Some(characters[0].name.clone())
                    } else {
                        None
                    };

                    (should_select, first_name)
                } else {
                    tracing::warn!("⚠️ World中没有CharacterList组件");
                    (false, None)
                }
            }; // ← 查询借用在这里结束
            self.handle_network_event(game_ctx);
            // ✅ Step 2: 在借用释放后修改 World
            if should_select {
                self.select_character(0, game_ctx.world);
                tracing::info!("🎯 默认选中第一个角色: {}", first_name.unwrap());
            }
        }

      

        let delta = game_ctx.ctx.time.delta().as_secs_f32();

        // 更新 NewCharacterDialog 动画和计时器
        if let Some(dialog) = &mut self.new_character_dialog {
            dialog.update(delta);
        }

        // ✅ 更新 InputBox (光标闪烁)
        if let Some(ref mut input_box) = self.input_box {
            input_box.update(delta);
        }

        // 🆕 更新角色选择组件（动画）
        self.character_select_ui.update(delta);

        // 🆕 处理消息框结果
        if let Some(ref mut message_box) = self.message_box {
            if message_box.has_result() {
                let result = message_box.result;
                self.message_box = None; // 关闭消息框

                // 根据结果执行相应操作
                match result {
                    MessageBoxResult::Ok => {
                        tracing::debug!("✅ 消息框: 用户点击OK");
                    }
                    MessageBoxResult::Yes => {
                        tracing::debug!("✅ 消息框: 用户点击Yes - 显示输入框");
                        // 删除角色确认后，显示输入框验证
                        if self.character_select_ui.get_selected_character().is_some() {
                            let mut input_box =
                                InputBox::new("Please enter the character's name.".to_string());
                            input_box.show(game_ctx.ctx); // ✅ 传入 ctx 启用 IME
                            self.input_box = Some(input_box);
                        }
                    }
                    MessageBoxResult::No => {
                        tracing::debug!("❌ 消息框: 用户点击No");
                    }
                    _ => {}
                }
            }
        }

        // 🆕 处理输入框结果
        if let Some(ref mut input_box) = self.input_box {
            if input_box.confirmed {
                let input_text = input_box.get_input().to_string();
                tracing::info!("✅ 输入框确认: {}", input_text);

                // 验证角色名并发送删除请求
                if let Some(character) = self.character_select_ui.get_selected_character() {
                    if input_text == character.name {
                        tracing::info!("🗑️ 发送删除角色请求: index={}", character.index);
                        // 发送 DeleteCharacter 包
                        use crate::ecs::WorldExt;
                        let _ = game_ctx.world.network().send(GameEvent::DeleteCharacterRequest {
                            index: character.index,
                        });
                    } else {
                        // 名称不匹配，显示错误消息
                        let mut msg = MessageBox::new(
                            "Incorrect Entry.".to_string(),
                            MessageBoxButtons::Ok,
                            Coord::DESIGN_WIDTH,
                            Coord::DESIGN_HEIGHT,
                        );
                        msg.show();
                        self.message_box = Some(msg);
                    }
                }

                // ✅ 关闭输入框时禁用 IME
                input_box.hide(game_ctx.ctx);
                self.input_box = None;
            } else if input_box.cancelled {
                tracing::info!("❌ 输入框取消");
                // ✅ 关闭输入框时禁用 IME
                input_box.hide(game_ctx.ctx);
                self.input_box = None;
            }
        }

        // 检查场景切换
        if let Some(scene_type) = self.pending_scene_change.take() {
            return Ok(Some(scene_type));
        }

        Ok(None)
    }

    fn draw(&mut self, ctx: &mut GameContext, canvas: &mut Canvas) -> GameResult {
        canvas.set_screen_coordinates(ggez::graphics::Rect::new(
            0.0,
            0.0,
            Coord::DESIGN_WIDTH,
            Coord::DESIGN_HEIGHT,
        ));

        // 🆕 使用 CharacterSelect 组件绘制主界面
        self.character_select_ui
            .draw(ctx.ctx, canvas, &self.bottom_buttons)?;

        // TODO: 8. 绘制 NewCharacterDialog (最上层)
        if let Some(ref mut dialog) = self.new_character_dialog {
            if dialog.visible {
                dialog.draw(ctx.ctx, canvas)?;
            }
        }

        // TODO: 9. 绘制 DeleteCharacterDialog (最上层)
        // 暂时禁用对话框绘制

        // 10. 🆕 绘制 CreditsDialog (最上层)
        if let Some(ref dialog) = self.credits_dialog {
            if dialog.visible {
                let _ = dialog.draw(ctx.ctx, canvas);
            }
        }

        // 11. 🆕 绘制消息框 (最上层)
        if let Some(ref message_box) = self.message_box {
            let _ = message_box.draw(ctx.ctx, canvas); // 使用正确的 MessageBox
        }

        // 12. 🆕 绘制输入框 (最上层)
        if let Some(ref mut input_box) = self.input_box {
            input_box.draw(ctx.ctx, canvas)?;
        }

        Ok(())
    }
}
