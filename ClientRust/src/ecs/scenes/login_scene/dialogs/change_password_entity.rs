//! ChangePasswordDialog ECS实体工厂

use hecs::{Entity, World};
use crate::graphics::LibraryName;
use super::super::components::*;
use super::super::ui::{ButtonBuilder, TextInputBuilder};

/// ChangePasswordDialog句柄
pub struct ChangePasswordDialogHandle {
    pub background: Entity,
    pub account_input: Entity,
    pub current_password_input: Entity,
    pub new_password_input: Entity,
    pub confirm_password_input: Entity,
    pub ok_button: Entity,
    pub cancel_button: Entity,
}

/// 创建ChangePasswordDialog
pub fn create_change_password_dialog(world: &mut World) -> ChangePasswordDialogHandle {
    let dialog_x = 274.0;
    let dialog_y = 194.0;

    // 背景
    let background = world.spawn((
        DialogEntity,
        Position { x: dialog_x, y: dialog_y },
        Size { width: 476.0, height: 380.0 },
        Sprite {
            library: LibraryName::Prguse,
            index: 18,
            visible: true,
        },
        Visible(true),
    ));

    // 账号输入框
    let account_input = TextInputBuilder::new(InputFieldType::ChangePasswordAccount)
        .position(dialog_x + 178.0, dialog_y + 75.0)
        .size(200.0, 20.0)
        .max_length(20)
        .build(world);

    // 当前密码
    let current_password_input = TextInputBuilder::new(InputFieldType::ChangePasswordCurrent)
        .position(dialog_x + 178.0, dialog_y + 113.0)
        .size(200.0, 20.0)
        .max_length(20)
        .password(true)
        .build(world);

    // 新密码
    let new_password_input = TextInputBuilder::new(InputFieldType::ChangePasswordNew)
        .position(dialog_x + 178.0, dialog_y + 151.0)
        .size(200.0, 20.0)
        .max_length(20)
        .password(true)
        .build(world);

    // 确认新密码
    let confirm_password_input = TextInputBuilder::new(InputFieldType::ChangePasswordConfirm)
        .position(dialog_x + 178.0, dialog_y + 188.0)
        .size(200.0, 20.0)
        .max_length(20)
        .password(true)
        .build(world);

    // OK按钮
    let ok_button = ButtonBuilder::new(LibraryName::Prguse, 4, ButtonAction::ChangePasswordOk)
        .hover_index(5)
        .position(dialog_x + 135.0, dialog_y + 325.0)
        .size(80.0, 32.0)
        .build(world);

    // Cancel按钮
    let cancel_button = ButtonBuilder::new(LibraryName::Prguse, 20, ButtonAction::ChangePasswordCancel)
        .hover_index(21)
        .position(dialog_x + 261.0, dialog_y + 325.0)
        .size(80.0, 32.0)
        .build(world);

    ChangePasswordDialogHandle {
        background,
        account_input,
        current_password_input,
        new_password_input,
        confirm_password_input,
        ok_button,
        cancel_button,
    }
}

/// 销毁ChangePasswordDialog
pub fn destroy_change_password_dialog(world: &mut World, handle: ChangePasswordDialogHandle) {
    let _ = world.despawn(handle.background);
    let _ = world.despawn(handle.account_input);
    let _ = world.despawn(handle.current_password_input);
    let _ = world.despawn(handle.new_password_input);
    let _ = world.despawn(handle.confirm_password_input);
    let _ = world.despawn(handle.ok_button);
    let _ = world.despawn(handle.cancel_button);
}

/// 获取修改密码数据
pub fn get_change_password_data(world: &World, handle: &ChangePasswordDialogHandle) -> Option<(String, String, String)> {
    let account = world.get::<&TextInput>(handle.account_input).ok()?.text.clone();
    let current_pwd = world.get::<&TextInput>(handle.current_password_input).ok()?.text.clone();
    let new_pwd = world.get::<&TextInput>(handle.new_password_input).ok()?.text.clone();
    let confirm_pwd = world.get::<&TextInput>(handle.confirm_password_input).ok()?.text.clone();
    
    if !account.is_empty() && !current_pwd.is_empty() && !new_pwd.is_empty() && new_pwd == confirm_pwd {
        Some((account, current_pwd, new_pwd))
    } else {
        None
    }
}
