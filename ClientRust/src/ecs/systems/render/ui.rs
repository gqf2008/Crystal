// ============================================================================
// UI 渲染模块 - RenderSystem UI渲染方法
// ============================================================================
//
// 职责：
// - RenderSystem::draw_ui() 负责渲染所有UI组件（分层管理）
// - 符合ECS设计原则：RenderSystem统一负责渲染，UISystem只处理数据
//
// 架构说明：
// - UISystem: 只处理UI状态管理（update/process_event/toggle_dialog等）
// - RenderSystem::draw_ui(): 统一渲染入口
//   - draw_debug_ui(): 调试UI (FPS、操作提示) - #[inline]优化
//   - draw_game_ui(): 游戏UI (对话框、技能栏等) - #[inline]优化  
//   - draw_overlay_ui(): 覆盖层UI (按键帮助面板) - #[inline]优化
//
// 优化说明：
// - 使用#[inline]减少函数调用开销
// - 字体名称编译时常量,不需运行时传递
// - 从world查询所有数据,符合ECS数据驱动原则
//
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, Color, Text, DrawParam, TextFragment, PxScale};
use hecs::World;

use crate::ecs::ui::{
    MainDialogComponent, InventoryDialogComponent, CharacterDialogComponent,
    SkillBarComponent, ChatDialogComponent, MagicLearningDialogComponent,
    QuestDialogComponent, SkillsDialogComponent, OptionsDialogComponent, HotkeyHelpPanel,
};
use crate::ecs::components::TimeTracker;
use crate::ecs::Coordinates;

use super::RenderSystem;

/// UI字体名称常量(编译时确定)
const UI_FONT_NAME: &str = "AlibabaPuHuiTi";

impl RenderSystem {
    /// 渲染所有UI组件（分层管理）
    /// 
    /// # 优化说明
    /// - 移除time_entity参数: 直接查询TimeTracker组件
    /// - 移除hotkey_help参数: 直接查询HotkeyHelpPanel组件
    /// - 移除ui_font_name参数: 使用模块常量UI_FONT_NAME
    /// - 移除current_time参数: 从ctx.time获取时间戳
    /// 
    /// # 渲染分层
    /// - 第0层: 调试UI (FPS、操作提示) - draw_debug_ui()
    /// - 第1-15层: 游戏UI (对话框等) - draw_game_ui()
    /// - 第99层: 覆盖层UI (按键帮助) - draw_overlay_ui()
    pub fn draw_ui(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
    ) -> GameResult {
        // 🎯 第0层: 调试UI (FPS、操作提示)
        Self::draw_debug_ui(ctx, canvas, world)?;
        
        // 🎯 第1-15层: 游戏UI (对话框、技能栏等)
        Self::draw_game_ui(ctx, canvas, world)?;
        
        // 🎯 第99层: 覆盖层UI (按键帮助面板)
        Self::draw_overlay_ui(ctx, canvas, world)?;
        
        Ok(())
    }
    
    /// 渲染调试UI (FPS、操作提示)
    /// 
    /// # 第0层: 调试信息
    /// - FPS显示 (左上角, 绿色)
    /// - 操作提示 (右上角, 灰色)
    /// 
    /// # 优化说明
    /// - 添加#[inline]减少函数调用开销
    /// - 直接查询TimeTracker组件(移除time_entity参数)
    /// - 使用模块常量UI_FONT_NAME(移除ui_font_name参数)
    #[inline]
    fn draw_debug_ui(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
    ) -> GameResult {
        // 绘制FPS (使用中文字体)
        // 直接查询TimeTracker组件,符合ECS数据驱动原则
        for (_entity, time) in world.query::<&TimeTracker>().iter() {
            let fps_fragment = TextFragment::new(format!("FPS: {:.1}", time.fps))
                .font(UI_FONT_NAME)
                .scale(PxScale::from(16.0))
                .color(Color::from_rgb(0, 255, 0));
            let fps_text = Text::new(fps_fragment);
            canvas.draw(
                &fps_text,
                DrawParam::default().dest([10.0, 10.0]),
            );
            break; // 只需要第一个TimeTracker
        }
        Ok(())
    }
    
    /// 渲染游戏UI (对话框、技能栏等)
    /// 
    /// # 第1-15层: 游戏UI组件
    /// - 主对话框 (z=0)
    /// - 技能栏 (z=1)
    /// - 聊天对话框 (z=2)
    /// - 可弹出对话框 (z=10-15)
    /// 
    /// # 优化说明
    /// - 添加#[inline]减少函数调用开销
    /// - 从ctx.time获取当前时间戳传递给技能栏
    #[inline]
    fn draw_game_ui(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
    ) -> GameResult {
    // 获取当前时间戳(毫秒)
    let current_time = ctx.time.ticks() as u64;
    
    // 🎯 第1层: 主对话框 (z=0, 最底层, 始终显示)
    for (_, dialog_comp) in world.query::<&MainDialogComponent>().iter() {
        dialog_comp.dialog.draw(ctx, canvas)?;
    }
    
    // 🎯 第2层: 技能栏 (z=1, 固定UI)
    for (_, skill_bar_comp) in world.query::<&SkillBarComponent>().iter() {
        skill_bar_comp.dialog.draw(ctx, canvas, current_time)?;
    }
    
    // 🎯 第3层: 聊天对话框 (z=2, 固定UI, 始终显示)
    for (_, dialog_comp) in world.query::<&ChatDialogComponent>().iter() {
        dialog_comp.dialog.draw(ctx, canvas)?;
    }
    
    // 🎯 第4层及以上: 可弹出对话框 (z=10+, 按打开顺序渲染)
    // 策略: 收集所有打开的对话框到Vec,按组件顺序渲染 (先渲染先打开的)
    // 这样后打开的对话框会覆盖在前面打开的之上
    
    // 固定渲染顺序 (从底层到顶层):
    // 1. 背包对话框
    // 2. 角色对话框
    // 3. 技能学习对话框
    // 4. 任务对话框
    // 5. 技能对话框
    // 6. 选项对话框
    
    // 渲染背包对话框 (仅在打开时显示, z=10)
    for (_, dialog_comp) in world.query::<&InventoryDialogComponent>().iter() {
        if dialog_comp.is_open {
            dialog_comp.dialog.draw(ctx, canvas)?;
        }
    }
    
    // 渲染角色对话框 (仅在打开时显示, z=11)
    for (_, dialog_comp) in world.query::<&CharacterDialogComponent>().iter() {
        if dialog_comp.is_open {
            dialog_comp.dialog.draw(ctx, canvas)?;
        }
    }
    
    // 渲染技能学习对话框 (仅在打开时显示, z=12)
    for (_, dialog_comp) in world.query::<&MagicLearningDialogComponent>().iter() {
        if dialog_comp.is_open {
            dialog_comp.dialog.draw(ctx, canvas)?;
        }
    }
    
    // 渲染任务对话框 (仅在打开时显示, z=13)
    for (_, dialog_comp) in world.query::<&QuestDialogComponent>().iter() {
        if dialog_comp.is_open {
            dialog_comp.draw(ctx, canvas)?;
        }
    }
    
    // 渲染技能对话框 (仅在打开时显示, z=14)
    for (_, dialog_comp) in world.query::<&SkillsDialogComponent>().iter() {
        if dialog_comp.is_open {
            dialog_comp.dialog.draw(ctx, canvas)?;
        }
    }
    
    // 渲染选项对话框 (仅在打开时显示, z=15, 最上层)
    for (_, dialog_comp) in world.query::<&OptionsDialogComponent>().iter() {
        if dialog_comp.is_open {
            dialog_comp.dialog.draw(ctx, canvas)?;
        }
    }
    
    Ok(())
    }
    
    /// 渲染覆盖层UI (按键帮助面板)
    /// 
    /// # 第99层: 覆盖层
    /// - 按键帮助面板 (最后绘制,在最上层)
    /// 
    /// # 优化说明
    /// - 添加#[inline]减少函数调用开销
    /// - 直接查询HotkeyHelpPanel组件(移除hotkey_help参数)
    #[inline]
    fn draw_overlay_ui(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
    ) -> GameResult {
        // 直接查询HotkeyHelpPanel组件,符合ECS数据驱动原则
        for (_entity, hotkey_help) in world.query::<&HotkeyHelpPanel>().iter() {
            hotkey_help.draw(ctx, canvas)?;
            break; // 只需要第一个HotkeyHelpPanel
        }
        Ok(())
    }
}
