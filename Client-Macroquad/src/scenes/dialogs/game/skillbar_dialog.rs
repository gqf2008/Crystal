// ============================================================================
// SkillBarDialogHybrid - 技能快捷栏（对齐 C# SkillBarDialog）
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/MainDialogs.cs:1499-1744
// - 背景：Prguse[2190]，覆盖层：Prguse[2193]
// - 切换按钮：Prguse[2247]
// - 技能图标：MagIcon 库，index = magic.Icon * 2
// - 冷却动画：Prguse2[1260..1282]，共 22 帧
// - 快捷键标签：F1-F8（第 1 排），Ctrl+F1-F8（第 2 排）
// - 每排 8 个技能槽位，槽位宽度 25px，起始偏移 15px
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::*;

// ============================================================================
// 常量
// ============================================================================

/// 每排技能槽位数
const SLOTS_PER_BAR: usize = 8;
/// 槽位间距
const SLOT_SPACING: f32 = 25.0;
/// 槽位起始 X 偏移（按钮右侧）
const SLOT_OFFSET_X: f32 = 15.0;
/// 槽位 Y 偏移
const SLOT_OFFSET_Y: f32 = 3.0;
/// 槽位尺寸（图标 24x24）
const SLOT_SIZE: f32 = 24.0;
/// 冷却动画总帧数
const COOLDOWN_FRAMES: i32 = 22;
/// 冷却动画起始索引 (Prguse2)
const COOLDOWN_FRAME_START: usize = 1260;

// ============================================================================
// 类型定义
// ============================================================================

/// 技能槽位数据
#[derive(Debug, Clone)]
pub struct SkillSlot {
    /// 技能类型（对应 SpellType 枚举值）
    pub spell_id: u8,
    /// 技能名称
    pub name: String,
    /// 技能图标索引（MagIcon 库）
    pub icon_index: usize,
    /// MP 消耗
    pub mp_cost: u32,
    /// 冷却时间（毫秒）
    pub cooldown_ms: u64,
    /// 上次施放时间戳（毫秒）
    pub cast_time_ms: u64,
    /// 技能等级 (0-3)
    pub level: u8,
}

impl SkillSlot {
    pub fn new(spell_id: u8, name: &str, icon_index: usize, mp_cost: u32, cooldown_ms: u64) -> Self {
        Self {
            spell_id,
            name: name.to_string(),
            icon_index,
            mp_cost,
            cooldown_ms,
            cast_time_ms: 0,
            level: 0,
        }
    }

    /// 获取剩余冷却时间（毫秒）
    pub fn remaining_cooldown_ms(&self, current_time_ms: u64) -> u64 {
        if self.cooldown_ms == 0 || self.cast_time_ms == 0 {
            return 0;
        }
        let end_time = self.cast_time_ms.saturating_add(self.cooldown_ms);
        if current_time_ms >= end_time {
            0
        } else {
            end_time.saturating_sub(current_time_ms)
        }
    }

    /// 是否正在冷却
    pub fn is_cooling_down(&self, current_time_ms: u64) -> bool {
        self.remaining_cooldown_ms(current_time_ms) > 0
    }

    /// 获取冷却动画帧索引 (0..COOLDOWN_FRAMES)
    pub fn cooldown_frame(&self, current_time_ms: u64) -> Option<usize> {
        if self.cooldown_ms == 0 || self.cast_time_ms == 0 {
            return None;
        }
        let remaining = self.remaining_cooldown_ms(current_time_ms);
        if remaining == 0 {
            return None;
        }
        let delay_per_frame = self.cooldown_ms / COOLDOWN_FRAMES as u64;
        if delay_per_frame == 0 {
            return None;
        }
        let elapsed = current_time_ms.saturating_sub(self.cast_time_ms);
        let frame = (elapsed / delay_per_frame).min(COOLDOWN_FRAMES as u64 - 1) as usize;
        Some(COOLDOWN_FRAME_START + frame)
    }
}

/// 技能快捷栏使用事件
#[derive(Debug, Clone)]
pub enum SkillBarAction {
    /// 使用技能（槽位 0-7 + bar_index * 8）
    UseSkill { bar_index: u8, slot: usize },
}

/// 技能快捷栏对话框
pub struct SkillBarDialogHybrid {
    /// 是否可见
    visible: bool,
    /// 当前排索引（0 或 1，对应 F1-F8 / Ctrl+F1-F8）
    bar_index: u8,
    /// 窗口位置
    position: Vec2,

    // === 纹理资源 ===
    /// 背景纹理 (Prguse[2190])
    bg_texture: BackgroundTexture,
    /// 覆盖层纹理 (Prguse[2193])
    overlay_texture: Option<Texture2D>,
    overlay_size: Vec2,
    /// 切换排按钮 (Prguse[2247])
    switch_btn: ButtonTextures,
    /// 冷却动画帧缓存 (Prguse2[1260..1282])
    cooldown_textures: Vec<Option<Texture2D>>,
    cooldown_size: Vec2,

    // === 窗口拖动 ===
    drag_helper: DragHelper,

    // === 槽位数据 ===
    slots: [Option<SkillSlot>; SLOTS_PER_BAR],

    // === 交互状态 ===
    hovered_slot: Option<usize>,
}

impl SkillBarDialogHybrid {
    pub fn new(bar_index: u8) -> Self {
        let y_offset = bar_index as f32 * 40.0;
        Self {
            visible: false,
            bar_index,
            position: vec2(200.0, 50.0 + y_offset),

            bg_texture: BackgroundTexture::new(),
            overlay_texture: None,
            overlay_size: Vec2::ZERO,
            switch_btn: ButtonTextures::new(),
            cooldown_textures: Vec::new(),
            cooldown_size: Vec2::ZERO,

            drag_helper: DragHelper::new(),

            slots: Default::default(),

            hovered_slot: None,
        }
    }

    /// 加载纹理资源
    pub fn load_textures(&mut self) {
        println!("⚔️ SkillBarDialog[{}]: 加载纹理...", self.bar_index);

        // 背景 (Prguse[2190])
        self.bg_texture = BackgroundTexture::load(LibraryName::Prguse, 2190, None);

        // 覆盖层 (Prguse[2193]) — 技能槽位底色
        if let Some(info) = LibraryName::Prguse.get_texture(2193) {
            self.overlay_size = vec2(info.width as f32, info.height as f32);
            self.overlay_texture = info.image;
        }

        // 切换排按钮 (Prguse[2247] 单态按钮)
        self.switch_btn = ButtonTextures::load_from_indices(LibraryName::Prguse, [2247, 2247, 2247]);

        // 冷却动画帧 (Prguse2[1260..1282])
        self.cooldown_textures.clear();
        for i in 0..COOLDOWN_FRAMES {
            let idx = COOLDOWN_FRAME_START + i as usize;
            if let Some(info) = LibraryName::Prguse2.get_texture(idx) {
                if i == 0 {
                    self.cooldown_size = vec2(info.width as f32, info.height as f32);
                }
                self.cooldown_textures.push(info.image);
            } else {
                self.cooldown_textures.push(None);
            }
        }

        println!("  ✅ 技能快捷栏[{}]纹理加载完成", self.bar_index);
    }

    // === 公共 API ===

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn bar_index(&self) -> u8 {
        self.bar_index
    }

    /// 设置技能到指定槽位
    pub fn set_slot(&mut self, slot: usize, skill: Option<SkillSlot>) {
        if slot < SLOTS_PER_BAR {
            self.slots[slot] = skill;
        }
    }

    /// 清空所有槽位
    pub fn clear_slots(&mut self) {
        for slot in &mut self.slots {
            *slot = None;
        }
    }

    /// 检查是否有任何技能
    pub fn has_skills(&self) -> bool {
        self.slots.iter().any(|s| s.is_some())
    }

    /// 记录技能施放时间（用于冷却计算）
    pub fn mark_cast(&mut self, slot: usize, cast_time_ms: u64) {
        if let Some(Some(skill)) = self.slots.get_mut(slot) {
            skill.cast_time_ms = cast_time_ms;
        }
    }

    /// 获取快捷键文本
    fn key_label(&self, slot: usize) -> &'static str {
        if self.bar_index == 0 {
            match slot {
                0 => "F1", 1 => "F2", 2 => "F3", 3 => "F4",
                4 => "F5", 5 => "F6", 6 => "F7", 7 => "F8",
                _ => "",
            }
        } else {
            match slot {
                0 => "^F1", 1 => "^F2", 2 => "^F3", 3 => "^F4",
                4 => "^F5", 5 => "^F6", 6 => "^F7", 7 => "^F8",
                _ => "",
            }
        }
    }

    // === 绘制 ===

    /// 绘制并处理输入，返回可能的动作
    pub fn draw(&mut self, current_time_ms: u64) -> Option<SkillBarAction> {
        if !self.visible {
            return None;
        }

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);
        let mut action = None;

        // 1. 窗口拖动
        let bg_size = self.bg_texture.size;
        let drag_rect = Rect::new(self.position.x, self.position.y, bg_size.x, bg_size.y);
        self.position = self.drag_helper.update(drag_rect, self.position, mouse_pos);

        // 2. 绘制背景
        self.bg_texture.draw(self.position);

        // 3. 绘制覆盖层（技能槽位底色）
        if let Some(tex) = &self.overlay_texture {
            draw_texture_ex(
                tex,
                self.position.x + SLOT_OFFSET_X - 2.0,
                self.position.y,
                Color::new(1.0, 1.0, 1.0, 0.5),
                DrawTextureParams::default(),
            );
        }

        // 4. 排号标签
        let bar_label = format!("{}", self.bar_index + 1);
        draw_text_cn(
            &bar_label,
            self.position.x + 3.0,
            self.position.y + 14.0,
            8.0,
            WHITE,
        );

        // 5. 切换排按钮
        let switch_rect = Rect::new(self.position.x, self.position.y, 14.0, 28.0);
        let switch_state = ButtonState::from_mouse(switch_rect, mouse_pos);
        self.switch_btn.draw(self.position, switch_state);
        if ButtonState::is_clicked(switch_rect, mouse_pos) {
            // 切换排号目前只更新标签显示
        }

        // 6. 绘制技能槽位
        self.hovered_slot = None;
        for i in 0..SLOTS_PER_BAR {
            let slot_x = self.position.x + SLOT_OFFSET_X + (i as f32 * SLOT_SPACING);
            let slot_y = self.position.y + SLOT_OFFSET_Y;
            let slot_rect = Rect::new(slot_x, slot_y, SLOT_SIZE, SLOT_SIZE);

            if let Some(skill) = &self.slots[i] {
                // 绘制技能图标 (MagIcon[icon_index * 2])
                let tex_index = skill.icon_index * 2;
                if let Some(info) = LibraryName::MagIcon.get_texture(tex_index) {
                    if let Some(tex) = &info.image {
                        draw_texture_ex(
                            tex,
                            slot_x,
                            slot_y,
                            WHITE,
                            DrawTextureParams {
                                dest_size: Some(vec2(SLOT_SIZE, SLOT_SIZE)),
                                ..Default::default()
                            },
                        );
                    }
                }

                // 绘制冷却叠加
                if let Some(frame_idx) = skill.cooldown_frame(current_time_ms) {
                    let local_idx = frame_idx.saturating_sub(COOLDOWN_FRAME_START);
                    if let Some(Some(cd_tex)) = self.cooldown_textures.get(local_idx) {
                        draw_texture_ex(
                            cd_tex,
                            slot_x,
                            slot_y,
                            Color::new(1.0, 1.0, 1.0, 0.6),
                            DrawTextureParams {
                                dest_size: Some(vec2(SLOT_SIZE, SLOT_SIZE)),
                                ..Default::default()
                            },
                        );
                    }
                }

                // 悬停检测
                if slot_rect.contains(mouse_pos) {
                    self.hovered_slot = Some(i);
                    // 悬停高亮
                    draw_rectangle(slot_x, slot_y, SLOT_SIZE, SLOT_SIZE, Color::new(1.0, 1.0, 1.0, 0.15));
                }

                // 点击使用技能
                if slot_rect.contains(mouse_pos) && is_mouse_button_pressed(MouseButton::Left) {
                    action = Some(SkillBarAction::UseSkill {
                        bar_index: self.bar_index,
                        slot: i,
                    });
                }
            } else {
                // 空槽位 - 绘制快捷键标签
                let key = self.key_label(i);
                if !key.is_empty() {
                    draw_text_cn(
                        key,
                        slot_x + 2.0,
                        slot_y + 14.0,
                        8.0,
                        Color::new(1.0, 1.0, 1.0, 0.6),
                    );
                }
            }
        }

        // 7. 绘制工具提示（悬停时）
        if let Some(slot_idx) = self.hovered_slot {
            if let Some(skill) = &self.slots[slot_idx] {
                let key = self.key_label(slot_idx);
                let remaining = skill.remaining_cooldown_ms(current_time_ms);
                let cooldown_text = if remaining > 0 {
                    format!("{}s", remaining / 1000)
                } else {
                    "就绪".to_string()
                };
                let tooltip = format!(
                    "{}\nMP: {}\n冷却: {}\n快捷键: {}",
                    skill.name, skill.mp_cost, cooldown_text, key
                );

                let tip_x = mouse_pos.x + 15.0;
                let tip_y = mouse_pos.y + 15.0;
                let lines: Vec<&str> = tooltip.lines().collect();
                let tip_w = 160.0;
                let tip_h = lines.len() as f32 * 16.0 + 8.0;

                // 背景
                draw_rectangle(tip_x, tip_y, tip_w, tip_h, Color::new(0.0, 0.0, 0.0, 0.85));
                draw_rectangle_lines(tip_x, tip_y, tip_w, tip_h, 1.0, Color::new(0.6, 0.6, 0.6, 0.8));

                // 文字
                for (j, line) in lines.iter().enumerate() {
                    draw_text_cn(
                        line,
                        tip_x + 6.0,
                        tip_y + 14.0 + j as f32 * 16.0,
                        12.0,
                        WHITE,
                    );
                }
            }
        }

        action
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_slot_cooldown() {
        let mut slot = SkillSlot::new(31, "FireBall", 0, 10, 3000);
        // 未施放 — 无冷却
        assert_eq!(slot.remaining_cooldown_ms(1000), 0);
        assert!(!slot.is_cooling_down(1000));
        assert!(slot.cooldown_frame(1000).is_none());

        // 施放后冷却中
        slot.cast_time_ms = 1000;
        assert_eq!(slot.remaining_cooldown_ms(2000), 2000);
        assert!(slot.is_cooling_down(2000));
        assert!(slot.cooldown_frame(2000).is_some());

        // 冷却结束
        assert_eq!(slot.remaining_cooldown_ms(4000), 0);
        assert!(!slot.is_cooling_down(4000));
        assert!(slot.cooldown_frame(4000).is_none());
    }

    #[test]
    fn test_skill_slot_zero_cooldown() {
        let slot = SkillSlot::new(1, "Fencing", 0, 0, 0);
        assert_eq!(slot.remaining_cooldown_ms(1000), 0);
        assert!(!slot.is_cooling_down(1000));
        assert!(slot.cooldown_frame(1000).is_none());
    }

    #[test]
    fn test_skillbar_has_skills() {
        let mut bar = SkillBarDialogHybrid::new(0);
        assert!(!bar.has_skills());

        bar.set_slot(0, Some(SkillSlot::new(31, "FireBall", 0, 10, 3000)));
        assert!(bar.has_skills());

        bar.clear_slots();
        assert!(!bar.has_skills());
    }

    #[test]
    fn test_key_labels() {
        let bar0 = SkillBarDialogHybrid::new(0);
        assert_eq!(bar0.key_label(0), "F1");
        assert_eq!(bar0.key_label(7), "F8");

        let bar1 = SkillBarDialogHybrid::new(1);
        assert_eq!(bar1.key_label(0), "^F1");
        assert_eq!(bar1.key_label(7), "^F8");
    }

    #[test]
    fn test_mark_cast() {
        let mut bar = SkillBarDialogHybrid::new(0);
        bar.set_slot(0, Some(SkillSlot::new(31, "FireBall", 0, 10, 3000)));
        bar.mark_cast(0, 5000);
        assert_eq!(bar.slots[0].as_ref().unwrap().cast_time_ms, 5000);
    }
}
