//! ConnectingBox ECS实体工厂

use hecs::{Entity, World};
use crate::graphics::LibraryName;
use super::super::components::*;
use super::super::ui::ButtonBuilder;

/// ConnectingBox句柄
pub struct ConnectingBoxHandle {
    pub background: Entity,
    pub cancel_button: Entity,
}

/// 创建ConnectingBox
pub fn create_connecting_box(world: &mut World) -> ConnectingBoxHandle {
    let dialog_x = 362.0;
    let dialog_y = 334.0;

    // 背景
    let background = world.spawn((
        DialogEntity,
        Position { x: dialog_x, y: dialog_y },
        Size { width: 300.0, height: 100.0 },
        Sprite {
            library: LibraryName::Prguse,
            index: 12,
            visible: false,  // 默认隐藏
        },
        Visible(false),
    ));

    // Cancel按钮
    let cancel_button = ButtonBuilder::new(LibraryName::Prguse, 20, ButtonAction::CancelConnect)
        .hover_index(21)
        .position(dialog_x + 110.0, dialog_y + 60.0)
        .size(80.0, 32.0)
        .build(world);

    ConnectingBoxHandle {
        background,
        cancel_button,
    }
}

/// 销毁ConnectingBox
pub fn destroy_connecting_box(world: &mut World, handle: ConnectingBoxHandle) {
    let _ = world.despawn(handle.background);
    let _ = world.despawn(handle.cancel_button);
}

/// 显示/隐藏
pub fn set_connecting_box_visible(world: &mut World, handle: &ConnectingBoxHandle, visible: bool) {
    for entity in [handle.background, handle.cancel_button] {
        if let Ok(mut vis) = world.get::<&mut Visible>(entity) {
            vis.0 = visible;
        }
    }
}
