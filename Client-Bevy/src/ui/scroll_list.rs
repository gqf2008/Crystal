// ============================================================================
// scroll_list - 通用可滚动列表（#89）
// 参考：C# MirListBox + ScrollBar（滚轮滚动 + 滚动条滑块指示）
// 设计：
//   - ScrollList 组件挂在对话框容器实体（背景/面板）上，rect 与 track 均为
//     相对容器左上角的坐标 → 弹窗拖动（dialog_drag_system）后仍正确。
//   - scroll_list_system 处理 MouseWheel：光标在列表可视区内 → 滚动最上层
//     （z 最大）列表；并每帧把滑块移到 offset 对应位置。
//   - 滑块实体由 spawn_scroll_bar 生成（不挂 DialogRoot，避免被拖动系统直接
//     位移；位置由本系统按容器 Transform 推算）。
// ============================================================================

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use crate::map_renderer::make_image;
use crate::ui::sprite_ui::UiEntity;

/// 可滚动列表状态（挂在对话框容器实体上）
#[derive(Component, Debug, Clone)]
pub struct ScrollList {
    /// 列表可视区（相对容器左上角，屏幕坐标 x,y,w,h）
    pub rect_rel: (f32, f32, f32, f32),
    /// 行高（px）
    pub row_h: f32,
    /// 可视行数
    pub visible: usize,
    /// 数据总行数（对话框每帧 set_total）
    pub total: usize,
    /// 当前滚动偏移（首行下标）
    pub offset: usize,
    /// 滚轮每格滚动行数
    pub step: usize,
    /// 滚动条轨道（相对容器左上角，屏幕坐标 x,y,w,h）
    pub track_rel: (f32, f32, f32, f32),
    /// 滚动条滑块实体（spawn_scroll_bar 返回）
    pub thumb: Option<Entity>,
    /// z 排序（多个列表重叠时滚动最上层）
    pub z: f32,
}

impl ScrollList {
    /// 最大可用偏移（数据不满一屏时为 0）
    pub fn max_offset(&self) -> usize {
        self.total.saturating_sub(self.visible)
    }

    /// 更新数据行数并夹紧偏移
    pub fn set_total(&mut self, total: usize) {
        self.total = total;
        self.offset = self.offset.min(self.max_offset());
    }
}

/// 滚动条滑块标记
#[derive(Component)]
pub struct ScrollThumb;

/// 生成滚动条（轨道 + 滑块），返回滑块实体。
/// 轨道挂 DialogRoot 由拖动系统整体移动；滑块不挂，由 scroll_list_system 定位。
/// images 参数用于创建 1x1 白色纹理（轨道/滑块着色矩形）。
pub fn spawn_scroll_bar(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    track_abs: (f32, f32, f32, f32),
    z: f32,
) -> (Entity, Entity) {
    let white = images.add(make_image(vec![255, 255, 255, 255], 1, 1));
    // 轨道（半透明深色）
    let track = commands
        .spawn((
            UiEntity,
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.0, 0.0, 0.0, 0.35),
                custom_size: Some(Vec2::new(track_abs.2, track_abs.3)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(track_abs.0, -track_abs.1, z),
            Visibility::Visible,
        ))
        .id();
    // 滑块（浅色）
    let thumb = commands
        .spawn((
            UiEntity,
            ScrollThumb,
            Sprite {
                image: white,
                color: Color::srgba(0.85, 0.85, 0.9, 0.9),
                custom_size: Some(Vec2::new(track_abs.2, 40.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(track_abs.0, -track_abs.1, z + 0.1),
            Visibility::Visible,
        ))
        .id();
    (track, thumb)
}

/// 滚轮滚动 + 滑块定位
pub fn scroll_list_system(
    mut wheels: MessageReader<MouseWheel>,
    windows: Query<&Window>,
    mut lists: Query<(Entity, &mut ScrollList, &Transform), Without<ScrollThumb>>,
    mut thumbs: Query<(&mut Transform, &mut Sprite, &ScrollThumb)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    // 汇总本帧滚轮增量（行）。像素滚动按 ~20px/行折算。
    let mut scroll_y = 0.0f32;
    for ev in wheels.read() {
        match ev.unit {
            MouseScrollUnit::Line => scroll_y += ev.y,
            MouseScrollUnit::Pixel => scroll_y += ev.y / 20.0,
        }
    }

    if scroll_y.abs() > 0.0 {
        // 找光标下 z 最大的列表（重叠时滚动最上层）
        let mut best: Option<(f32, Entity)> = None;
        for (e, list, tf) in lists.iter() {
            let (rx, ry, rw, rh) = list.rect_rel;
            let ax = tf.translation.x + rx;
            let ay = -tf.translation.y + ry;
            if cursor.x >= ax && cursor.x <= ax + rw && cursor.y >= ay && cursor.y <= ay + rh {
                if best.map_or(true, |(bz, _)| list.z > bz) {
                    best = Some((list.z, e));
                }
            }
        }
        if let Some((_, e)) = best {
            if let Ok((_, mut list, _)) = lists.get_mut(e) {
                let rows = (scroll_y * list.step.max(1) as f32).round() as i32;
                let max = list.max_offset() as i32;
                list.offset = (list.offset as i32 + rows).clamp(0, max) as usize;
            }
        }
    }

    // 每帧把滑块移到 offset 对应位置（跟随容器拖动）
    for (_, list, tf) in lists.iter() {
        let Some(thumb) = list.thumb else {
            continue;
        };
        let Ok((mut tt, mut ts, _)) = thumbs.get_mut(thumb) else {
            continue;
        };
        let (tx, ty, tw, th) = list.track_rel;
        let total = list.total.max(list.visible);
        let thumb_h = (th * (list.visible as f32 / total as f32)).clamp(14.0, th);
        let max_off = list.max_offset();
        let ratio = if max_off == 0 {
            0.0
        } else {
            list.offset as f32 / max_off as f32
        };
        let thumb_y = ty + ratio * (th - thumb_h);
        let cx = tf.translation.x;
        let cy = -tf.translation.y; // 容器左上角屏幕 y
        tt.translation.x = cx + tx;
        tt.translation.y = -(cy + thumb_y);
        ts.custom_size = Some(Vec2::new(tw, thumb_h));
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn list(total: usize, visible: usize, offset: usize) -> ScrollList {
        ScrollList {
            rect_rel: (0.0, 0.0, 100.0, 100.0),
            row_h: 20.0,
            visible,
            total,
            offset,
            step: 3,
            track_rel: (100.0, 0.0, 4.0, 100.0),
            thumb: None,
            z: 1.0,
        }
    }

    #[test]
    fn max_offset_clamps() {
        // 数据不满一屏 → 不能滚动
        let mut l = list(5, 10, 0);
        assert_eq!(l.max_offset(), 0);
        l.set_total(5);
        assert_eq!(l.offset, 0);
        // 数据超过一屏 → 可滚动到 total-visible
        l.set_total(25);
        assert_eq!(l.max_offset(), 15);
        l.offset = 99;
        l.set_total(25);
        assert_eq!(l.offset, 15);
    }

    #[test]
    fn offset_follows_total_shrink() {
        let mut l = list(30, 10, 20);
        assert_eq!(l.max_offset(), 20);
        l.set_total(12);
        assert_eq!(l.offset, 2);
    }
}
