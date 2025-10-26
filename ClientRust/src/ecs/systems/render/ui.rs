// ============================================================================
// UI 渲染模块 - RenderSystem UI渲染方法
// ============================================================================
//
// 职责：
// - RenderSystem::draw_ui() 负责渲染所有UI组件（按z-order排序）
// - 符合ECS设计原则：RenderSystem统一负责渲染，UISystem只处理数据
//
// 架构说明：
// - UISystem: 只处理UI状态管理（update/process_event/toggle_dialog等）
// - RenderSystem::draw_ui(): 负责所有UI组件的渲染
//
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, Color, Text, DrawParam};
use hecs::World;

use crate::ecs::ui::{
    MainDialogComp, InventoryDialogComp, CharacterDialogComp,
    SkillBarComp, ChatDialogComp, MagicLearningDialogComp,
    QuestDialogComp, SkillsDialogComp, OptionsDialogComp, HotkeyHelpPanel,
};
use crate::ecs::components::TimeTracker;
use crate::ecs::Coordinates;

use super::RenderSystem;

impl RenderSystem {
    /// 渲染所有UI组件（按z-order分层）
    /// 
    /// # 渲染顺序（从底层到顶层）：
    /// - 第0层: 调试UI (FPS、操作提示)
    /// - 第1层: 主对话框 (z=0, 最底层, 始终显示)
    /// - 第2层: 技能栏 (z=1, 固定UI)
    /// - 第3层: 聊天对话框 (z=2, 固定UI, 始终显示)
    /// - 第4层及以上: 可弹出对话框 (z=10+, 按打开顺序渲染)
    ///   - 背包对话框 (z=10)
    ///   - 角色对话框 (z=11)
    ///   - 技能学习对话框 (z=12)
    ///   - 任务对话框 (z=13)
    ///   - 技能对话框 (z=14)
    ///   - 选项对话框 (z=15, 最上层)
    /// - 第99层: 按键帮助面板 (覆盖层)
    pub fn draw_ui(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        current_time: u64,
        hotkey_help: &HotkeyHelpPanel,
    ) -> GameResult {
    // 调试输出 (只打印前3次)
    static mut DRAW_COUNT: u32 = 0;
    unsafe {
        if DRAW_COUNT < 3 {
            println!("🎨 [RenderSystem::draw_ui] 调用 #{}", DRAW_COUNT + 1);
            DRAW_COUNT += 1;
        }
    }
    
    // 🎯 第0层: 调试UI (FPS、操作提示)
    // 绘制FPS
    if let Some((_, time)) = world.query::<&TimeTracker>().iter().next() {
        let fps_text = Text::new(format!("FPS: {:.1}", time.fps));
        canvas.draw(
            &fps_text,
            DrawParam::default()
                .dest([10.0, 10.0])
                .color(Color::from_rgb(0, 255, 0)),
        );
    }
    
    // 绘制操作提示（移到右上角，使用设计坐标系）
    let hint_text = Text::new("[WASD/方向键] 移动  [Shift+WASD] 跑动  [鼠标] 点击移动  [Esc] 返回");
    canvas.draw(
        &hint_text,
        DrawParam::default()
            .dest([Coordinates::DESIGN_WIDTH - 500.0, 10.0])
            .color(Color::from_rgb(200, 200, 200)),
    );
    
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
    
    // 🎯 第99层: 按键帮助面板 (覆盖层, 最后绘制)
    hotkey_help.draw(ctx, canvas)?;
    
    Ok(())
    }
}
