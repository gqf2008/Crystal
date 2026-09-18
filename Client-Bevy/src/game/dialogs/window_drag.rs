// ============================================================================
// C# `MirControl.Movable` 窗口拖动（#2892 批D 单元①）
//
// C# 侧：`MirControl.Movable = true` 的控件在 `OnMouseMove` 里把 `Location` 加上鼠标位移；
// `Settings.Save/Load` **只持久化技能栏**（`[Game] Skillbar{i}X/Y`，`Settings.cs:163/269/380`）——
// 这 6 个窗口的位置原版**不落盘**，重进游戏回到默认位。本端因此同样只在会话内保留偏移，
// 不做 INI 持久化（与 C# 一致；技能栏有独立持久化，见 `game/skills.rs`）。
//
// 涉及的 5 个窗口（C# 基准）：
//   药水腰带 `BeltDialog`（`InventoryDialog.cs:610`）、英雄腰带 `HeroBeltDialog`（`HeroDialogs.cs:258`）、
//   好友备注 `MemoDialog`（`FriendDialog.cs:492`）、钓鱼状态 `FishingStatusDialog`（`FishingDialog.cs:176`）、
//   下拉框 `MirDropDownBox`（`MirDropDownBox.cs:196`）。
// 注意：聊天窗 **不可拖**——C# `ChatDialog` 是 `MirImageControl` 且不设 `Movable`
// （`MainDialogs.cs:697` 的 `Movable = true` 属于滚动滑块 `PositionBar`，拖动=滚历史，
// 本端已由 `chat_scroll_knob_system` 覆盖）。曾因误读该行实现整窗拖动，2026-09-18 移除。
//
// 用法：窗口系统每帧用 `WindowDragState::register` 登记**未加偏移**的矩形（UI 逻辑坐标），
// 绘制/命中时把 `WindowDragState::offset(w)` 加回去；本模块只负责拖动本身。
// ============================================================================

use bevy::prelude::*;
use std::collections::HashMap;

/// 可拖窗口标识
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DragWindow {
    /// 药水腰带（C# `BeltDialog`）
    PotionBelt,
    /// 英雄腰带（C# `HeroBeltDialog`）
    HeroBelt,
    /// 好友备注（C# `MemoDialog`）
    Memo,
    /// 钓鱼状态（C# `FishingStatusDialog`）
    FishingStatus,
    /// 下拉框（C# `MirDropDownBox`）
    DropDown,
}

/// 拖动命中优先级（后登记的窗口在上；下拉框/备注这类弹层优先）
pub const DRAG_ORDER: [DragWindow; 5] = [
    DragWindow::DropDown,
    DragWindow::Memo,
    DragWindow::FishingStatus,
    DragWindow::PotionBelt,
    DragWindow::HeroBelt,
];

/// UI 逻辑画布（C# `Settings.ScreenWidth/Height`）
pub const DRAG_SCREEN: (f32, f32) = (1024.0, 768.0);

#[derive(Resource, Default)]
pub struct WindowDragState {
    /// 未加拖动偏移时的窗口矩形（由各窗口系统每帧登记，UI 逻辑坐标）
    rects: HashMap<DragWindow, (f32, f32, f32, f32)>,
    /// 当前拖动偏移（相对基准位置的增量）
    offsets: HashMap<DragWindow, (f32, f32)>,
    /// 正在拖动的窗口 + 抓取点相对窗口左上角的偏移
    dragging: Option<(DragWindow, (f32, f32))>,
}

impl WindowDragState {
    /// 登记窗口的**基准**矩形（不含拖动偏移）
    pub fn register(&mut self, w: DragWindow, x: f32, y: f32, ww: f32, hh: f32) {
        self.rects.insert(w, (x, y, ww, hh));
    }

    /// 当前偏移
    pub fn offset(&self, w: DragWindow) -> (f32, f32) {
        self.offsets.get(&w).copied().unwrap_or((0.0, 0.0))
    }

    /// 直接设置偏移（供窗口系统/测试使用）
    pub fn set_offset(&mut self, w: DragWindow, dx: f32, dy: f32) {
        self.offsets.insert(w, (dx, dy));
    }

    /// 正在拖动的窗口
    pub fn dragging(&self) -> Option<DragWindow> {
        self.dragging.map(|(w, _)| w)
    }

    /// 光标（UI 逻辑坐标）是否落在任一已登记窗口的**当前显示**矩形（基准 + 拖动偏移）上。
    /// 供世界点击闸门使用：这些窗口不是 `DialogManager` 对话框，`blocks_world_click`
    /// 不覆盖；拖离底部常驻区后，落在其上的点击/按住不得穿透成寻路/移动。
    pub fn over_window(&self, c: Vec2) -> bool {
        DRAG_ORDER.iter().any(|w| {
            self.rects.get(w).is_some_and(|(x, y, ww, hh)| {
                let (dx, dy) = self.offset(*w);
                c.x >= x + dx && c.x <= x + dx + ww && c.y >= y + dy && c.y <= y + dy + hh
            })
        })
    }
}

/// 光标 → UI 逻辑坐标（UI 相机 `Fixed{1024,768}`，需 `viewport_to_world_2d` 换算，见 #2517）
pub fn ui_cursor(
    window: &Window,
    cameras: &Query<(&Camera, &GlobalTransform), With<crate::ui::sprite_ui::UiEntity>>,
) -> Option<Vec2> {
    let raw = window.cursor_position()?;
    let (cam, gtf) = cameras.single().ok()?;
    let world = cam.viewport_to_world_2d(gtf, raw).ok()?;
    Some(Vec2::new(world.x, -world.y))
}

/// 拖动系统：按下命中窗口 → 跟随光标（钳在画布内）→ 松开结束
pub fn window_drag_system(
    mut state: ResMut<WindowDragState>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<crate::ui::sprite_ui::UiEntity>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let cursor = ui_cursor(window, &cameras);
    if mouse.just_pressed(MouseButton::Left) {
        if let Some(c) = cursor {
            // 命中优先级固定（窗口重叠概率低；弹层优先）
            let hit = DRAG_ORDER.iter().copied().find(|w| {
                state.rects.get(w).is_some_and(|(x, y, ww, hh)| {
                    c.x >= *x && c.x <= x + ww && c.y >= *y && c.y <= y + hh
                })
            });
            if let Some(w) = hit {
                if let Some(&(rx, ry, _, _)) = state.rects.get(&w) {
                    let off = state.offset(w);
                    // 抓取点按「当前显示位置」算（基准 + 已有偏移），否则拖第二次会跳
                    state.dragging = Some((w, (c.x - (rx + off.0), c.y - (ry + off.1))));
                }
            }
        }
    }
    let Some((w, grab)) = state.dragging else {
        return;
    };
    if mouse.pressed(MouseButton::Left) {
        if let (Some(c), Some(&(rx, ry, ww, hh))) = (cursor, state.rects.get(&w)) {
            // C# `OnMouseMove` 把控件钳在父容器（全屏）内：窗口永远不会被拖出屏幕
            let nx = (c.x - grab.0).clamp(0.0, (DRAG_SCREEN.0 - ww).max(0.0));
            let ny = (c.y - grab.1).clamp(0.0, (DRAG_SCREEN.1 - hh).max(0.0));
            state.offsets.insert(w, (nx - rx, ny - ry));
        }
    } else {
        state.dragging = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 偏移登记/读取：默认 0，写入后按窗口各自返回
    #[test]
    fn offsets_are_per_window() {
        let mut st = WindowDragState::default();
        assert_eq!(st.offset(DragWindow::PotionBelt), (0.0, 0.0));
        st.register(DragWindow::PotionBelt, 230.0, 618.0, 240.0, 38.0);
        st.offsets.insert(DragWindow::PotionBelt, (10.0, -20.0));
        assert_eq!(st.offset(DragWindow::PotionBelt), (10.0, -20.0));
        // 另一个窗口不受影响
        assert_eq!(st.offset(DragWindow::HeroBelt), (0.0, 0.0));
    }

    /// 拖动钳位公式：窗口四边不得越出 1024x768（C# `MirControl.OnMouseMove` 钳在父容器内）
    #[test]
    fn drag_clamp_stays_on_screen() {
        let (ww, hh) = (240.0f32, 38.0f32);
        let clamp = |x: f32, y: f32| {
            (
                x.clamp(0.0, (DRAG_SCREEN.0 - ww).max(0.0)),
                y.clamp(0.0, (DRAG_SCREEN.1 - hh).max(0.0)),
            )
        };
        assert_eq!(clamp(-50.0, -50.0), (0.0, 0.0));
        assert_eq!(clamp(2000.0, 2000.0), (1024.0 - ww, 768.0 - hh));
        assert_eq!(clamp(230.0, 618.0), (230.0, 618.0));
        // 命中优先级：弹层（下拉框/备注）排在最前
        assert_eq!(DRAG_ORDER[0], DragWindow::DropDown);
        assert_eq!(DRAG_ORDER[1], DragWindow::Memo);
    }

    /// over_window：按「基准+偏移」的当前显示位置命中——拖走后原位不再算、新位算
    /// （世界点击闸门依赖此判定防穿透）
    #[test]
    fn over_window_follows_drag_offset() {
        let mut st = WindowDragState::default();
        // 未登记任何窗口 → 不命中
        assert!(!st.over_window(Vec2::new(240.0, 620.0)));
        st.register(DragWindow::PotionBelt, 230.0, 618.0, 240.0, 38.0);
        // 基准位置命中（边含边界）
        assert!(st.over_window(Vec2::new(230.0, 618.0)));
        assert!(st.over_window(Vec2::new(470.0, 656.0)));
        assert!(!st.over_window(Vec2::new(471.0, 618.0)));
        assert!(!st.over_window(Vec2::new(230.0, 657.0)));
        // 拖走 100,-100 后：原位不命中，新位置命中
        st.set_offset(DragWindow::PotionBelt, 100.0, -100.0);
        assert!(!st.over_window(Vec2::new(240.0, 620.0)));
        assert!(st.over_window(Vec2::new(340.0, 520.0)));
        // 其它窗口（英雄腰带）独立判定
        st.register(DragWindow::HeroBelt, 475.0, 618.0, 100.0, 38.0);
        assert!(st.over_window(Vec2::new(500.0, 630.0)));
    }
}
