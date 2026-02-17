// ============================================================================
// BuffDialogHybrid - Buff 状态显示（对齐 C# BuffDialog）
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/BuffDialog.cs
// - 背景：Prguse2[20..33]（根据 buff 数量选择不同宽度）
// - Buff 图标：BuffIcon 库（部分 MagIcon/Prguse2，由 BuffImage 偏移决定）
// - 展开/收起按钮：Prguse2[7,8,9]
// - 淡入淡出效果：鼠标悬停时淡入，离开时淡出
// - 最后 5 秒闪烁：buff 即将过期时图标闪烁
// - 布局：右上角，从右向左排列，每行最多 10 个
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::*;

// ============================================================================
// 常量
// ============================================================================

/// 单个 buff 图标宽度
const BUFF_ICON_WIDTH: f32 = 23.0;
/// 单个 buff 图标高度
const BUFF_ICON_HEIGHT: f32 = 24.0;
/// 每行最多 buff 数
const BUFFS_PER_ROW: usize = 10;
/// 收起状态宽度
const COLLAPSED_WIDTH: f32 = 44.0;
/// 收起状态高度
const COLLAPSED_HEIGHT: f32 = 34.0;
/// 淡入淡出速率
const FADE_RATE: f32 = 0.2;
/// 闪烁阈值（最后 5 秒）
const BLINK_THRESHOLD_MS: u64 = 5000;

// ============================================================================
// 类型定义
// ============================================================================

/// Buff 类型（对应 C# BuffType 枚举的常用子集）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BuffType {
    None = 0,
    // 常见增益
    Haste = 1,
    SwiftFeet = 2,
    Fury = 3,
    LightBody = 4,
    SoulShield = 5,
    BlessedArmour = 6,
    ProtectionField = 7,
    Rage = 8,
    CounterAttack = 9,
    MagicBooster = 10,
    // 防御
    MagicShield = 11,
    EnergyShield = 12,
    ElementalBarrier = 13,
    // 变身/隐身
    Hiding = 14,
    MoonLight = 15,
    DarkBody = 16,
    // 召唤
    SummonSkeleton = 17,
    SummonShinsu = 18,
    SummonHolyDeva = 19,
    // 其他
    ImmortalSkin = 20,
    Concentration = 21,
    Meditation = 22,
    MentalState = 23,
}

impl BuffType {
    /// 获取 buff 名称
    pub fn name(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::Haste => "加速",
            Self::SwiftFeet => "神行",
            Self::Fury => "狂怒",
            Self::LightBody => "轻身",
            Self::SoulShield => "幽灵盾",
            Self::BlessedArmour => "神圣战甲",
            Self::ProtectionField => "护身气幕",
            Self::Rage => "暴怒",
            Self::CounterAttack => "反击",
            Self::MagicBooster => "魔法增强",
            Self::MagicShield => "魔法盾",
            Self::EnergyShield => "能量盾",
            Self::ElementalBarrier => "元素屏障",
            Self::Hiding => "隐身",
            Self::MoonLight => "月光",
            Self::DarkBody => "暗影",
            Self::SummonSkeleton => "骷髅召唤",
            Self::SummonShinsu => "神兽召唤",
            Self::SummonHolyDeva => "圣兽召唤",
            Self::ImmortalSkin => "不死之身",
            Self::Concentration => "集中",
            Self::Meditation => "冥想",
            Self::MentalState => "精神状态",
        }
    }

    /// 获取 buff 图标索引（BuffIcon 库）
    pub fn icon_index(&self) -> usize {
        // C# 中 BuffImage() 方法返回值，这里用简化映射
        *self as usize
    }
}

/// 客户端 Buff 数据
#[derive(Debug, Clone)]
pub struct ClientBuff {
    pub buff_type: BuffType,
    /// 过期时间戳（毫秒），0 表示永久
    pub expire_time_ms: u64,
    /// 是否暂停
    pub paused: bool,
    /// 是否永久
    pub infinite: bool,
    /// buff 值（用于特殊显示）
    pub values: Vec<i32>,
    /// 施放者名称
    pub caster: String,
}

impl ClientBuff {
    pub fn new(buff_type: BuffType, expire_time_ms: u64) -> Self {
        Self {
            buff_type,
            expire_time_ms,
            paused: false,
            infinite: false,
            values: Vec::new(),
            caster: String::new(),
        }
    }

    /// 获取剩余时间（毫秒）
    pub fn remaining_ms(&self, current_time_ms: u64) -> u64 {
        if self.infinite || self.expire_time_ms == 0 {
            return u64::MAX;
        }
        self.expire_time_ms.saturating_sub(current_time_ms)
    }

    /// 是否即将过期（最后 5 秒）
    pub fn is_expiring_soon(&self, current_time_ms: u64) -> bool {
        if self.paused || self.infinite || self.expire_time_ms == 0 {
            return false;
        }
        let remaining = self.remaining_ms(current_time_ms);
        remaining <= BLINK_THRESHOLD_MS && remaining > 0
    }

    /// 是否已过期
    pub fn is_expired(&self, current_time_ms: u64) -> bool {
        if self.infinite || self.expire_time_ms == 0 {
            return false;
        }
        current_time_ms >= self.expire_time_ms
    }

    /// 获取显示文本
    pub fn display_text(&self, current_time_ms: u64) -> String {
        let name = self.buff_type.name();
        let remaining = self.remaining_ms(current_time_ms);
        if self.infinite {
            format!("{}\n持续: 永久", name)
        } else if remaining > 0 {
            let secs = remaining / 1000;
            let mins = secs / 60;
            if mins > 0 {
                format!("{}\n剩余: {}分{}秒", name, mins, secs % 60)
            } else {
                format!("{}\n剩余: {}秒", name, secs)
            }
        } else {
            format!("{}\n已过期", name)
        }
    }
}

/// Buff 对话框
pub struct BuffDialogHybrid {
    /// 是否可见
    visible: bool,
    /// 是否展开
    expanded: bool,
    /// 位置（右上角）
    position: Vec2,
    /// 当前透明度 (0.0 ~ 1.0)
    opacity: f32,
    /// 是否已淡出
    faded_out: bool,
    /// 是否已淡入
    faded_in: bool,

    // === 数据 ===
    /// 当前 buff 列表
    buffs: Vec<ClientBuff>,

    // === 纹理 ===
    /// 展开/收起按钮
    expand_btn: ButtonTextures,

    // === 交互 ===
    hovered_index: Option<usize>,
}

impl BuffDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: true,
            expanded: false,
            position: vec2(screen_width() - 170.0, 0.0),
            opacity: 0.0,
            faded_out: true,
            faded_in: false,

            buffs: Vec::new(),

            expand_btn: ButtonTextures::new(),

            hovered_index: None,
        }
    }

    /// 加载纹理
    pub fn load_textures(&mut self) {
        println!("✨ BuffDialog: 加载纹理...");

        // 展开/收起按钮 (Prguse2[7,8,9])
        self.expand_btn = ButtonTextures::load_from_indices(LibraryName::Prguse2, [7, 8, 9]);

        println!("  ✅ Buff 状态栏纹理加载完成");
    }

    // === 公共 API ===

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// 添加 buff
    pub fn add_buff(&mut self, buff: ClientBuff) {
        // 检查是否已存在同类型 buff
        if let Some(existing) = self.buffs.iter_mut().find(|b| b.buff_type == buff.buff_type) {
            *existing = buff;
        } else {
            self.buffs.insert(0, buff);
        }
    }

    /// 移除 buff（按索引）
    pub fn remove_buff_at(&mut self, index: usize) {
        if index < self.buffs.len() {
            self.buffs.remove(index);
        }
    }

    /// 移除指定类型的 buff
    pub fn remove_buff_type(&mut self, buff_type: BuffType) {
        self.buffs.retain(|b| b.buff_type != buff_type);
    }

    /// 清除已过期的 buff
    pub fn clear_expired(&mut self, current_time_ms: u64) {
        self.buffs.retain(|b| !b.is_expired(current_time_ms));
    }

    /// Buff 总数
    pub fn buff_count(&self) -> usize {
        self.buffs.len()
    }

    // === 绘制 ===

    pub fn draw(&mut self, current_time_ms: u64) {
        if !self.visible {
            return;
        }

        // 更新位置（跟随屏幕右上角）
        self.position.x = screen_width() - 170.0;

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);
        let buff_count = self.buffs.len();

        if buff_count == 0 {
            return;
        }

        // 计算整体区域
        let (area_w, area_h) = if self.expanded {
            let cols = buff_count.min(BUFFS_PER_ROW) as f32;
            let rows = ((buff_count as f32 - 1.0) / BUFFS_PER_ROW as f32).floor() + 1.0;
            (cols * BUFF_ICON_WIDTH + 16.0, rows * BUFF_ICON_HEIGHT + 10.0)
        } else {
            (COLLAPSED_WIDTH, COLLAPSED_HEIGHT)
        };
        let area_rect = Rect::new(self.position.x, self.position.y, area_w, area_h);

        // 淡入淡出处理
        let is_hover = area_rect.contains(mouse_pos);
        if is_hover {
            self.opacity = (self.opacity + FADE_RATE).min(1.0);
            if self.opacity >= 1.0 {
                self.faded_in = true;
                self.faded_out = false;
            }
        } else {
            self.opacity = (self.opacity - FADE_RATE).max(0.0);
            if self.opacity <= 0.0 {
                self.faded_out = true;
                self.faded_in = false;
            }
        }

        // 绘制背景
        let bg_alpha = 0.3 + self.opacity * 0.5;
        draw_rectangle(
            self.position.x, self.position.y,
            area_w, area_h,
            Color::new(0.0, 0.0, 0.0, bg_alpha),
        );
        draw_rectangle_lines(
            self.position.x, self.position.y,
            area_w, area_h,
            1.0,
            Color::new(0.5, 0.5, 0.5, self.opacity),
        );

        // 绘制展开/收起按钮
        let btn_x = self.position.x + area_w - 15.0;
        let btn_y = self.position.y;
        let btn_rect = Rect::new(btn_x, btn_y, 15.0, 15.0);
        let btn_state = ButtonState::from_mouse(btn_rect, mouse_pos);
        if let Some(tex) = self.expand_btn.get_texture(btn_state) {
            draw_texture_ex(
                tex,
                btn_x, btn_y,
                Color::new(1.0, 1.0, 1.0, self.opacity.max(0.3)),
                DrawTextureParams::default(),
            );
        }
        if ButtonState::is_clicked(btn_rect, mouse_pos) {
            self.expanded = !self.expanded;
        }

        // 绘制 buff 图标
        self.hovered_index = None;

        if self.expanded {
            // 展开模式：显示所有 buff
            for (i, buff) in self.buffs.iter().enumerate() {
                let col = i % BUFFS_PER_ROW;
                let row = i / BUFFS_PER_ROW;
                let icon_x = self.position.x + area_w - 10.0 - BUFF_ICON_WIDTH - (col as f32 * BUFF_ICON_WIDTH);
                let icon_y = self.position.y + 6.0 + (row as f32 * BUFF_ICON_HEIGHT);
                let icon_rect = Rect::new(icon_x, icon_y, BUFF_ICON_WIDTH, BUFF_ICON_HEIGHT);

                // 闪烁效果（最后 5 秒）
                let should_draw = if buff.is_expiring_soon(current_time_ms) {
                    let time_frac = (current_time_ms % 1000) as f32 / 1000.0;
                    time_frac < 0.5  // 每秒闪烁一次
                } else {
                    true
                };

                if should_draw {
                    // 绘制 buff 图标 (BuffIcon 库)
                    let icon_idx = buff.buff_type.icon_index();
                    if let Some(info) = LibraryName::BuffIcon.get_texture(icon_idx) {
                        if let Some(tex) = &info.image {
                            draw_texture_ex(
                                tex,
                                icon_x, icon_y,
                                Color::new(1.0, 1.0, 1.0, self.opacity.max(0.5)),
                                DrawTextureParams {
                                    dest_size: Some(vec2(BUFF_ICON_WIDTH - 2.0, BUFF_ICON_HEIGHT - 2.0)),
                                    ..Default::default()
                                },
                            );
                        }
                    } else {
                        // Fallback: 纯色方块 + 名字首字
                        let color = Color::new(0.3, 0.6, 0.9, self.opacity.max(0.5));
                        draw_rectangle(icon_x + 1.0, icon_y + 1.0, BUFF_ICON_WIDTH - 3.0, BUFF_ICON_HEIGHT - 3.0, color);
                        let initial = &buff.buff_type.name().chars().next().map(|c| c.to_string()).unwrap_or_default();
                        draw_text_cn(initial, icon_x + 5.0, icon_y + 16.0, 10.0, WHITE);
                    }
                }

                // 悬停检测
                if icon_rect.contains(mouse_pos) {
                    self.hovered_index = Some(i);
                    draw_rectangle_lines(icon_x, icon_y, BUFF_ICON_WIDTH, BUFF_ICON_HEIGHT, 1.0, YELLOW);
                }
            }
        } else {
            // 收起模式：只显示第一个 buff + 计数
            if let Some(buff) = self.buffs.first() {
                let icon_x = self.position.x + 10.0;
                let icon_y = self.position.y + 6.0;
                let icon_rect = Rect::new(icon_x, icon_y, BUFF_ICON_WIDTH, BUFF_ICON_HEIGHT);

                let icon_idx = buff.buff_type.icon_index();
                if let Some(info) = LibraryName::BuffIcon.get_texture(icon_idx) {
                    if let Some(tex) = &info.image {
                        draw_texture_ex(
                            tex,
                            icon_x, icon_y,
                            Color::new(1.0, 1.0, 1.0, self.opacity.max(0.5)),
                            DrawTextureParams {
                                dest_size: Some(vec2(BUFF_ICON_WIDTH - 2.0, BUFF_ICON_HEIGHT - 2.0)),
                                ..Default::default()
                            },
                        );
                    }
                } else {
                    let color = Color::new(0.3, 0.6, 0.9, self.opacity.max(0.5));
                    draw_rectangle(icon_x + 1.0, icon_y + 1.0, BUFF_ICON_WIDTH - 3.0, BUFF_ICON_HEIGHT - 3.0, color);
                }

                if icon_rect.contains(mouse_pos) {
                    self.hovered_index = Some(0);
                }
            }

            // Buff 计数标签
            if buff_count > 1 {
                let count_text = format!("{}", buff_count);
                let cx = self.position.x + COLLAPSED_WIDTH / 2.0 - 4.0;
                let cy = self.position.y + COLLAPSED_HEIGHT / 2.0 + 4.0;
                draw_text_cn(&count_text, cx, cy, 10.0, YELLOW);
            }
        }

        // 工具提示
        if let Some(idx) = self.hovered_index {
            if let Some(buff) = self.buffs.get(idx) {
                let tooltip = buff.display_text(current_time_ms);
                let tip_x = mouse_pos.x + 15.0;
                let tip_y = mouse_pos.y + 15.0;
                let lines: Vec<&str> = tooltip.lines().collect();
                let tip_w = 180.0;
                let tip_h = lines.len() as f32 * 16.0 + 8.0;

                draw_rectangle(tip_x, tip_y, tip_w, tip_h, Color::new(0.0, 0.0, 0.0, 0.85));
                draw_rectangle_lines(tip_x, tip_y, tip_w, tip_h, 1.0, Color::new(0.6, 0.6, 0.6, 0.8));

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
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_buff_remaining() {
        let buff = ClientBuff::new(BuffType::Haste, 10000);
        assert_eq!(buff.remaining_ms(5000), 5000);
        assert_eq!(buff.remaining_ms(10000), 0);
        assert_eq!(buff.remaining_ms(15000), 0);
    }

    #[test]
    fn test_client_buff_infinite() {
        let mut buff = ClientBuff::new(BuffType::MagicShield, 0);
        buff.infinite = true;
        assert_eq!(buff.remaining_ms(999999), u64::MAX);
        assert!(!buff.is_expiring_soon(999999));
        assert!(!buff.is_expired(999999));
    }

    #[test]
    fn test_client_buff_expiring_soon() {
        let buff = ClientBuff::new(BuffType::Fury, 10000);
        assert!(!buff.is_expiring_soon(3000)); // 7s remaining
        assert!(buff.is_expiring_soon(6000));  // 4s remaining
        assert!(buff.is_expiring_soon(9500));  // 0.5s remaining
    }

    #[test]
    fn test_client_buff_expired() {
        let buff = ClientBuff::new(BuffType::Hiding, 5000);
        assert!(!buff.is_expired(3000));
        assert!(buff.is_expired(5000));
        assert!(buff.is_expired(6000));
    }

    #[test]
    fn test_buff_dialog_add_remove() {
        let mut dialog = BuffDialogHybrid::new();
        assert_eq!(dialog.buff_count(), 0);

        dialog.add_buff(ClientBuff::new(BuffType::Haste, 10000));
        assert_eq!(dialog.buff_count(), 1);

        dialog.add_buff(ClientBuff::new(BuffType::Fury, 15000));
        assert_eq!(dialog.buff_count(), 2);

        // 同类型替换而非新增
        dialog.add_buff(ClientBuff::new(BuffType::Haste, 20000));
        assert_eq!(dialog.buff_count(), 2);

        dialog.remove_buff_type(BuffType::Fury);
        assert_eq!(dialog.buff_count(), 1);
    }

    #[test]
    fn test_buff_dialog_clear_expired() {
        let mut dialog = BuffDialogHybrid::new();
        dialog.add_buff(ClientBuff::new(BuffType::Haste, 5000));
        dialog.add_buff(ClientBuff::new(BuffType::Fury, 10000));

        dialog.clear_expired(7000);
        assert_eq!(dialog.buff_count(), 1); // Fury still alive
        assert_eq!(dialog.buffs[0].buff_type, BuffType::Fury);
    }

    #[test]
    fn test_buff_display_text() {
        let buff = ClientBuff::new(BuffType::Haste, 65000);
        let text = buff.display_text(0);
        assert!(text.contains("加速"));
        assert!(text.contains("1分"));

        let mut infinite_buff = ClientBuff::new(BuffType::MagicShield, 0);
        infinite_buff.infinite = true;
        let text = infinite_buff.display_text(0);
        assert!(text.contains("永久"));
    }
}
