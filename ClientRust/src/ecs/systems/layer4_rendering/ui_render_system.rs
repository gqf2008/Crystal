// ============================================================================
// UI Render System - UI渲染系统
// ============================================================================
//
// 🎯 Layer 4 - Rendering & Playback Layer（渲染与播放层）
//
// 职责：
// - 渲染UI对话框（背包、角色、技能树、聊天等）
// - 从Layer 5的UI组件读取数据并渲染到屏幕
// - UI对话框是"弹出的菜单和交互界面"
//
// 不负责：
// - HUD渲染（由HUDRenderSystem负责）
// - UI逻辑和事件处理（由Layer 5的UISystem负责）
// - 游戏逻辑
//
// 与HUDRenderSystem的区别：
// - HUDRenderSystem: 固定在屏幕上的游戏信息（血条、地图、buff等）
// - UIRenderSystem: 可打开/关闭的对话框（背包、技能、聊天等）
//
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, Color, Text, DrawParam, TextFragment, PxScale};
use hecs::World;
use crate::ecs::ui::{
    MainDialog, InventoryDialog, CharacterDialog,
    SkillBarDialog, ChatDialog, MagicLearningDialog,
    QuestDialog, SkillsDialog, OptionsDialog, HotkeyHelpPanel,
};

/// UI渲染系统（Layer 4）
/// 
/// # 设计原则
/// - 仅负责渲染UI对话框
/// - 不处理点击、拖拽等交互（由Layer 5负责）
/// - 只读取UI组件数据，不修改游戏状态
/// 
/// # 渲染分层
/// - 第1层: 固定UI（技能栏、聊天窗口）- 始终显示
/// - 第2层: 主对话框（最底层的可弹出对话框）
/// - 第3-10层: 弹出对话框（背包、角色、技能等）- 按打开顺序叠加
pub struct UIRenderSystem;

impl UIRenderSystem {
    /// 渲染所有UI组件
    /// 
    /// # 参数
    /// - `ctx`: ggez上下文
    /// - `canvas`: 渲染目标画布
    /// - `world`: ECS世界（用于查询UI组件）
    pub fn render(ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult {
        // 🎯 第1层: 固定UI（技能栏、聊天窗口）
        Self::render_fixed_ui(ctx, canvas, world)?;
        
        // 🎯 第2层: 主对话框
        Self::render_main_dialog(ctx, canvas, world)?;
        
        // 🎯 第3-10层: 弹出对话框（按打开顺序渲染）
        Self::render_popup_dialogs(ctx, canvas, world)?;
        
        // 🎯 第99层: 覆盖层UI（按键帮助面板）
        Self::render_overlay_ui(ctx, canvas, world)?;
        
        Ok(())
    }
    
    // ========================================================================
    // 固定UI渲染
    // ========================================================================
    
    /// 渲染固定UI（始终显示的UI元素）
    fn render_fixed_ui(ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult {
        let current_time = ctx.time.ticks() as u64;
        
        // 渲染技能栏
        for (_, skill_bar) in world.query::<&SkillBarDialog>().iter() {
            skill_bar.draw(ctx, canvas, current_time)?;
        }
        
        // 渲染聊天对话框
        for (_, chat) in world.query::<&ChatDialog>().iter() {
            chat.draw(ctx, canvas)?;
        }
        
        Ok(())
    }
    
    // ========================================================================
    // 主对话框渲染
    // ========================================================================
    
    /// 渲染主对话框（最底层的可弹出对话框）
    fn render_main_dialog(ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult {
        for (_, dialog) in world.query::<&MainDialog>().iter() {
            dialog.draw(ctx, canvas)?;
        }
        Ok(())
    }
    
    // ========================================================================
    // 弹出对话框渲染
    // ========================================================================
    
    /// 渲染弹出对话框（按固定顺序，后打开的覆盖在上层）
    fn render_popup_dialogs(ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult {
        // 固定渲染顺序（从底层到顶层）：
        // 1. 背包对话框 (z=3)
        // 2. 角色对话框 (z=4)
        // 3. 技能学习对话框 (z=5)
        // 4. 任务对话框 (z=6)
        // 5. 技能对话框 (z=7)
        // 6. 选项对话框 (z=8, 最上层)
        
        // 渲染背包对话框
        for (_, dialog) in world.query::<&InventoryDialog>().iter() {
            if dialog.is_open() {
                dialog.draw(ctx, canvas)?;
            }
        }
        
        // 渲染角色对话框
        for (_, dialog) in world.query::<&CharacterDialog>().iter() {
            if dialog.is_open() {
                dialog.draw(ctx, canvas)?;
            }
        }
        
        // 渲染技能学习对话框
        for (_, dialog) in world.query::<&MagicLearningDialog>().iter() {
            if dialog.is_open() {
                dialog.draw(ctx, canvas)?;
            }
        }
        
        // 渲染任务对话框
        for (_, dialog) in world.query::<&QuestDialog>().iter() {
            if dialog.is_open {
                dialog.draw(ctx, canvas)?;
            }
        }
        
        // 渲染技能对话框
        for (_, dialog) in world.query::<&SkillsDialog>().iter() {
            if dialog.is_open() {
                dialog.draw(ctx, canvas)?;
            }
        }
        
        // 渲染选项对话框（最上层）
        for (_, dialog) in world.query::<&OptionsDialog>().iter() {
            if dialog.is_open() {
                dialog.draw(ctx, canvas)?;
            }
        }
        
        Ok(())
    }
    
    // ========================================================================
    // 覆盖层UI渲染
    // ========================================================================
    
    /// 渲染覆盖层UI（最上层，如按键帮助面板）
    fn render_overlay_ui(ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult {
        // 渲染按键帮助面板
        for (_, hotkey_help) in world.query::<&HotkeyHelpPanel>().iter() {
            hotkey_help.draw(ctx, canvas)?;
            break; // 只需要第一个
        }
        
        Ok(())
    }
}

// ============================================================================
// 使用说明
// ============================================================================
//
// 这个系统从RenderSystem::draw_ui()中提取出来，专注于UI对话框渲染
// 
// 调用顺序（在game_scene.rs中）：
// 1. RenderSystem::draw_game_world() - 渲染游戏世界
// 2. HUDRenderSystem::render() - 渲染HUD（血条、地图等）
// 3. UIRenderSystem::render() - 渲染UI对话框（背包、技能等）
//
// Layer 4和Layer 5的分工：
// - Layer 4 (UIRenderSystem): 只负责"画"，读取UI组件数据并渲染
// - Layer 5 (UISystem): 负责"逻辑"，处理点击、打开/关闭对话框、更新数据
//
