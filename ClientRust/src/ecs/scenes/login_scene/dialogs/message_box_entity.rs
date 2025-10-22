//! MessageBox ECS实体工厂

use hecs::{Entity, World};
use crate::graphics::LibraryName;
use super::super::components::*;
use super::super::ui::ButtonBuilder;

/// MessageBox句柄
pub struct MessageBoxHandle {
    pub background: Entity,
    pub ok_button: Entity,
    pub message: String,
}

/// 创建MessageBox
pub fn create_message_box(world: &mut World, message: String) -> MessageBoxHandle {
    let dialog_x = 312.0;
    let dialog_y = 284.0;

    // 背景
    let background = world.spawn((
        DialogEntity,
        Position { x: dialog_x, y: dialog_y },
        Size { width: 400.0, height: 200.0 },
        Sprite {
            library: LibraryName::Prguse,
            index: 15,
            visible: true,
        },
        Visible(true),
    ));

    // OK按钮
    let ok_button = ButtonBuilder::new(LibraryName::Prguse, 4, ButtonAction::MessageBoxOk)
        .hover_index(5)
        .position(dialog_x + 160.0, dialog_y + 150.0)
        .size(80.0, 32.0)
        .build(world);

    MessageBoxHandle {
        background,
        ok_button,
        message,
    }
}

/// 销毁MessageBox
pub fn destroy_message_box(world: &mut World, handle: MessageBoxHandle) {
    let _ = world.despawn(handle.background);
    let _ = world.despawn(handle.ok_button);
}
