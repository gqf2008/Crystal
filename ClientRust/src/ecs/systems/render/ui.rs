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
//   - draw_debug_ui(): 调试UI (FPS、操作提示)
//   - draw_game_ui(): 游戏UI (对话框、技能栏等)
//   - draw_overlay_ui(): 覆盖层UI (按键帮助面板)
//
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, Color, Text, DrawParam, TextFragment, PxScale};
use hecs::{World, Entity};

use crate::ecs::ui::{
    MainDialogComp, InventoryDialogComp, CharacterDialogComp,
    SkillBarComp, ChatDialogComp, MagicLearningDialogComp,
    QuestDialogComp, SkillsDialogComp, OptionsDialogComp, HotkeyHelpPanel,
};
use crate::ecs::components::TimeTracker;
use crate::ecs::Coordinates;

use super::RenderSystem;

impl RenderSystem {
    /// 渲染所有UI组件（分层管理）
    /// 
    /// # 参数
    /// - `time_entity`: TimeTracker实体引用(用于获取FPS数据)
    /// - `ui_font_name`: UI字体名称(用于中文显示)
    /// 
    /// # 渲染分层：
    /// - 第0层: 调试UI (FPS、操作提示) - draw_debug_ui()
    /// - 第1-15层: 游戏UI (对话框等) - draw_game_ui()
    /// - 第99层: 覆盖层UI (按键帮助) - draw_overlay_ui()
    pub fn draw_ui(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        current_time: u64,
        time_entity: Entity,
        hotkey_help: &HotkeyHelpPanel,
        ui_font_name: &str,
    ) -> GameResult {
        // 🎯 第0层: 调试UI (FPS、操作提示)
        Self::draw_debug_ui(ctx, canvas, world, time_entity, ui_font_name)?;
        
        // 🎯 第1-15层: 游戏UI (对话框、技能栏等)
        Self::draw_game_ui(ctx, canvas, world, current_time)?;
        
        // 🎯 第99层: 覆盖层UI (按键帮助面板)
        Self::draw_overlay_ui(ctx, canvas, hotkey_help)?;
        
        Ok(())
    }
    
    /// 渲染调试UI (FPS、操作提示)
    /// 
    /// # 第0层: 调试信息
    /// - FPS显示 (左上角, 绿色)
    /// - 操作提示 (右上角, 灰色)
    fn draw_debug_ui(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        time_entity: Entity,
        ui_font_name: &str,
    ) -> GameResult {
        // 绘制FPS (使用中文字体)
        if let Ok(time) = world.get::<&TimeTracker>(time_entity) {
            let fps_fragment = TextFragment::new(format!("FPS: {:.1}", time.fps))
                .font(ui_font_name)
                .scale(PxScale::from(16.0))
                .color(Color::from_rgb(0, 255, 0));
            let fps_text = Text::new(fps_fragment);
            canvas.draw(
                &fps_text,
                DrawParam::default().dest([10.0, 10.0]),
            );
        }
        
        // 绘制操作提示（使用中文字体）
        let hint_fragment = TextFragment::new("[WASD/方向键] 移动  [Shift+WASD] 跑动  [鼠标] 点击移动  [Esc] 返回")
            .font(ui_font_name)
            .scale(PxScale::from(14.0))
            .color(Color::from_rgb(200, 200, 200));
        let hint_text = Text::new(hint_fragment);
        canvas.draw(
            &hint_text,
            DrawParam::default().dest([Coordinates::DESIGN_WIDTH - 500.0, 10.0]),
        );
        
        Ok(())
    }
    
    /// 渲染游戏UI (对话框、技能栏等)
    /// 
    /// # 第1-15层: 游戏UI组件
    /// - 主对话框 (z=0)
    /// - 技能栏 (z=1)
    /// - 聊天对话框 (z=2)
    /// - 可弹出对话框 (z=10-15)
    fn draw_game_ui(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        current_time: u64,
    ) -> GameResult {
    // 🎯 第1层: 主对话框 (z=0, 最底层, 始终显示)
    for (_, dialog_comp) in world.query::<&MainDialogComp>().iter() {
        dialog_comp.dialog.draw(ctx, canvas)?;
    }
    
    // 🎯 第2层: 技能栏 (z=1, 固定UI)
    for (_, skill_bar_comp) in world.query::<&SkillBarComp>().iter() {
        skill_bar_comp.dialog.draw(ctx, canvas, current_time)?;
    }
    
    // 🎯 第3层: 聊天对话框 (z=2, 固定UI, 始终显示)
    for (_, dialog_comp) in world.query::<&ChatDialogComp>().iter() {
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
    for (_, dialog_comp) in world.query::<&InventoryDialogComp>().iter() {
        if dialog_comp.is_open {
            dialog_comp.dialog.draw(ctx, canvas)?;
        }
    }
    
    // 渲染角色对话框 (仅在打开时显示, z=11)
    for (_, dialog_comp) in world.query::<&CharacterDialogComp>().iter() {
        if dialog_comp.is_open {
            dialog_comp.dialog.draw(ctx, canvas)?;
        }
    }
    
    // 渲染技能学习对话框 (仅在打开时显示, z=12)
    for (_, dialog_comp) in world.query::<&MagicLearningDialogComp>().iter() {
        if dialog_comp.is_open {
            dialog_comp.dialog.draw(ctx, canvas)?;
        }
    }
    
    // 渲染任务对话框 (仅在打开时显示, z=13)
    for (_, dialog_comp) in world.query::<&QuestDialogComp>().iter() {
        if dialog_comp.is_open {
            dialog_comp.draw(ctx, canvas)?;
        }
    }
    
    // 渲染技能对话框 (仅在打开时显示, z=14)
    for (_, dialog_comp) in world.query::<&SkillsDialogComp>().iter() {
        if dialog_comp.is_open {
            dialog_comp.dialog.draw(ctx, canvas)?;
        }
    }
    
    // 渲染选项对话框 (仅在打开时显示, z=15, 最上层)
    for (_, dialog_comp) in world.query::<&OptionsDialogComp>().iter() {
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
    fn draw_overlay_ui(
        ctx: &mut Context,
        canvas: &mut Canvas,
        hotkey_help: &HotkeyHelpPanel,
    ) -> GameResult {
        hotkey_help.draw(ctx, canvas)?;
        Ok(())
    }
}
