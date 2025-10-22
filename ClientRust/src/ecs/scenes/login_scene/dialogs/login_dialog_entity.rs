//! LoginDialog ECS实体工厂

use hecs::{Entity, World};
use crate::graphics::LibraryName;
use super::super::components::*;
use super::super::ui::{ButtonBuilder, TextInputBuilder};

/// LoginDialog句柄
pub struct LoginDialogHandle {
    pub background: Entity,
    pub account_input: Entity,
    pub password_input: Entity,
    pub ok_button: Entity,
    pub new_account_button: Entity,
    pub change_password_button: Entity,
    pub exit_button: Entity,
}

/// 创建LoginDialog
pub fn create_login_dialog(world: &mut World) -> LoginDialogHandle {
    let dialog_x = 274.0;
    let dialog_y = 244.0;

    // 背景
    let background = world.spawn((
        DialogEntity,
        Position { x: dialog_x, y: dialog_y },
        Size { width: 476.0, height: 280.0 },
        Sprite {
            library: LibraryName::Prguse,
            index: 1,
            visible: true,
        },
        Visible(true),
    ));

    // 账号输入框
    let account_input = TextInputBuilder::new(InputFieldType::LoginAccount)
        .position(dialog_x + 76.0, dialog_y + 73.0)
        .size(200.0, 20.0)
        .max_length(20)
        .build(world);

    // 密码输入框
    let password_input = TextInputBuilder::new(InputFieldType::LoginPassword)
        .position(dialog_x + 76.0, dialog_y + 98.0)
        .size(200.0, 20.0)
        .max_length(20)
        .password(true)
        .build(world);

    // Login按钮
    let ok_button = ButtonBuilder::new(LibraryName::Prguse, 4, ButtonAction::Login)
        .hover_index(5)
        .position(dialog_x + 60.0, dialog_y + 240.0)
        .size(80.0, 32.0)
        .build(world);

    // New Account按钮
    let new_account_button = ButtonBuilder::new(LibraryName::Prguse, 6, ButtonAction::NewAccount)
        .hover_index(7)
        .position(dialog_x + 150.0, dialog_y + 240.0)
        .size(80.0, 32.0)
        .build(world);

    // Change Password按钮
    let change_password_button = ButtonBuilder::new(LibraryName::Prguse, 8, ButtonAction::ChangePassword)
        .hover_index(9)
        .position(dialog_x + 240.0, dialog_y + 240.0)
        .size(80.0, 32.0)
        .build(world);

    // Exit按钮
    let exit_button = ButtonBuilder::new(LibraryName::Prguse, 10, ButtonAction::CloseDialog)
        .hover_index(11)
        .position(dialog_x + 330.0, dialog_y + 240.0)
        .size(80.0, 32.0)
        .build(world);

    LoginDialogHandle {
        background,
        account_input,
        password_input,
        ok_button,
        new_account_button,
        change_password_button,
        exit_button,
    }
}

/// 销毁LoginDialog
pub fn destroy_login_dialog(world: &mut World, handle: LoginDialogHandle) {
    let _ = world.despawn(handle.background);
    let _ = world.despawn(handle.account_input);
    let _ = world.despawn(handle.password_input);
    let _ = world.despawn(handle.ok_button);
    let _ = world.despawn(handle.new_account_button);
    let _ = world.despawn(handle.change_password_button);
    let _ = world.despawn(handle.exit_button);
}

/// 获取登录凭据
pub fn get_login_credentials(world: &World, handle: &LoginDialogHandle) -> Option<(String, String)> {
    let account = world.get::<&TextInput>(handle.account_input).ok()?.text.clone();
    let password = world.get::<&TextInput>(handle.password_input).ok()?.text.clone();
    
    if !account.is_empty() && !password.is_empty() {
        Some((account, password))
    } else {
        None
    }
}

/// 设置凭据
pub fn set_login_credentials(world: &mut World, handle: &LoginDialogHandle, account: String, password: String) {
    if let Ok(mut input) = world.get::<&mut TextInput>(handle.account_input) {
        input.text = account;
    }
    if let Ok(mut input) = world.get::<&mut TextInput>(handle.password_input) {
        input.text = password;
    }
}

/// 显示/隐藏
pub fn set_login_dialog_visible(world: &mut World, handle: &LoginDialogHandle, visible: bool) {
    for entity in [
        handle.background,
        handle.account_input,
        handle.password_input,
        handle.ok_button,
        handle.new_account_button,
        handle.change_password_button,
        handle.exit_button,
    ] {
        if let Ok(mut vis) = world.get::<&mut Visible>(entity) {
            vis.0 = visible;
        }
    }
}
