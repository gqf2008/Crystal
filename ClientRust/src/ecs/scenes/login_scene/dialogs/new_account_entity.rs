//! NewAccountDialog ECS实体工厂
//! 
//! 将NewAccountDialog从700行的结构体重构为ECS实体集合

use hecs::{Entity, World};
use crate::graphics::LibraryName;
use super::super::components::*;
use super::super::ui::{ButtonBuilder, TextInputBuilder};

/// 对话框句柄（持有所有相关实体ID）
pub struct NewAccountDialogHandle {
    pub dialog_bg: Entity,
    pub ok_button: Entity,
    pub cancel_button: Entity,
    pub account_input: Entity,
    pub password1_input: Entity,
    pub password2_input: Entity,
    pub email_input: Entity,
    pub name_input: Entity,
    pub question_input: Entity,
    pub answer_input: Entity,
    pub birthday_input: Entity,
}

/// 创建NewAccountDialog所有实体
pub fn create_new_account_dialog(world: &mut World) -> NewAccountDialogHandle {
    // 对话框尺寸和位置（417x440，居中）
    let dialog_width = 417.0;
    let dialog_height = 440.0;
    let dialog_x = (1024.0 - dialog_width) / 2.0;  // 303.5
    let dialog_y = (768.0 - dialog_height) / 2.0;  // 164.0

    tracing::info!("📦 创建NewAccountDialog实体: 位置({}, {})", dialog_x, dialog_y);

    // 1. 创建对话框背景（C# Index = 63, Library = Prguse）
    let dialog_bg = world.spawn((
        DialogEntity,
        Position { x: dialog_x, y: dialog_y },
        Size { width: dialog_width, height: dialog_height },
        Sprite {
            library: LibraryName::Prguse,
            index: 63,  // ✅ 修正：使用C#中的Index 63
            visible: true,
        },
        Visible(true),
    ));

    // 2. 创建OK按钮（C# Location: 135, 425）
    let ok_button = ButtonBuilder::new(
        LibraryName::Title,
        200,  // normal
        ButtonAction::NewAccountOk,
    )
    .hover_index(201)
    .pressed_index(202)
    .position(dialog_x + 135.0, dialog_y + 425.0)  // ✅ 对话框相对坐标
    .size(80.0, 23.0)  // 按钮尺寸
    .enabled(false)  // 默认禁用，等字段验证通过后启用
    .build(world);

    tracing::debug!("  ✅ OK按钮: ({}, {})", dialog_x + 135.0, dialog_y + 425.0);

    // 3. 创建Cancel按钮（C# Location: 409, 425）
    let cancel_button = ButtonBuilder::new(
        LibraryName::Title,
        203,  // normal
        ButtonAction::NewAccountCancel,
    )
    .hover_index(204)
    .pressed_index(205)
    .position(dialog_x + 409.0, dialog_y + 425.0)  // ✅ 对话框相对坐标
    .size(80.0, 23.0)
    .enabled(true)
    .build(world);

    tracing::debug!("  ✅ Cancel按钮: ({}, {})", dialog_x + 409.0, dialog_y + 425.0);

    // 4. 创建所有输入框（基于C#的Location）
    // Account ID TextBox: Location(226, 78), Size(136, 18)
    let account_input = TextInputBuilder::new(InputFieldType::NewAccountId)
        .position(dialog_x + 226.0, dialog_y + 78.0)
        .size(136.0, 18.0)
        .max_length(15)  // Globals.MaxAccountIDLength
        .validation(InputValidation::MinLength(3))
        .build(world);

    // Password TextBox: Location(226, 129), Size(136, 18)
    let password1_input = TextInputBuilder::new(InputFieldType::NewAccountPassword)
        .position(dialog_x + 226.0, dialog_y + 129.0)
        .size(136.0, 18.0)
        .max_length(20)  // Globals.MaxPasswordLength
        .password(true)
        .validation(InputValidation::MinLength(5))
        .build(world);

    // Confirm Password TextBox: Location(226, 176), Size(136, 18)
    let password2_input = TextInputBuilder::new(InputFieldType::NewAccountConfirmPassword)
        .position(dialog_x + 226.0, dialog_y + 176.0)
        .size(136.0, 18.0)
        .max_length(20)
        .password(true)
        .build(world);

    // E-Mail TextBox: Location(226, 223), Size(136, 18)
    let email_input = TextInputBuilder::new(InputFieldType::NewAccountEmail)
        .position(dialog_x + 226.0, dialog_y + 223.0)
        .size(136.0, 18.0)
        .max_length(50)  // Globals.MaxEMailLength
        .validation(InputValidation::EmailFormat)
        .build(world);

    // User Name TextBox: Location(226, 270), Size(136, 18) - 可选字段
    let name_input = TextInputBuilder::new(InputFieldType::NewAccountName)
        .position(dialog_x + 226.0, dialog_y + 270.0)
        .size(136.0, 18.0)
        .max_length(20)  // Globals.MaxCharacterNameLength
        .build(world);

    // Question TextBox: Location(226, 317), Size(136, 18) - 可选字段
    let question_input = TextInputBuilder::new(InputFieldType::NewAccountQuestion)
        .position(dialog_x + 226.0, dialog_y + 317.0)
        .size(136.0, 18.0)
        .max_length(30)  // Globals.MaxQuestionLength
        .build(world);

    // Answer TextBox: Location(226, 364), Size(136, 18) - 可选字段
    let answer_input = TextInputBuilder::new(InputFieldType::NewAccountAnswer)
        .position(dialog_x + 226.0, dialog_y + 364.0)
        .size(136.0, 18.0)
        .max_length(30)  // Globals.MaxAnswerLength
        .build(world);

    // Birthday TextBox: Location(226, 390), Size(136, 18) - 可选字段
    let birthday_input = TextInputBuilder::new(InputFieldType::NewAccountBirthday)
        .position(dialog_x + 226.0, dialog_y + 390.0)
        .size(136.0, 18.0)
        .max_length(10)
        .build(world);

    NewAccountDialogHandle {
        dialog_bg,
        ok_button,
        cancel_button,
        account_input,
        password1_input,
        password2_input,
        email_input,
        name_input,
        question_input,
        answer_input,
        birthday_input,
    }
}

/// 销毁NewAccountDialog所有实体
pub fn destroy_new_account_dialog(world: &mut World, handle: NewAccountDialogHandle) {
    let _ = world.despawn(handle.dialog_bg);
    let _ = world.despawn(handle.ok_button);
    let _ = world.despawn(handle.cancel_button);
    let _ = world.despawn(handle.account_input);
    let _ = world.despawn(handle.password1_input);
    let _ = world.despawn(handle.password2_input);
    let _ = world.despawn(handle.email_input);
    let _ = world.despawn(handle.name_input);
    let _ = world.despawn(handle.question_input);
    let _ = world.despawn(handle.answer_input);
    let _ = world.despawn(handle.birthday_input);
    
    tracing::info!("🗑️ NewAccountDialog实体已销毁");
}

/// 更新OK按钮状态（基于必填字段验证）
pub fn update_ok_button_state(world: &mut World) {
    use super::super::ui::input_helpers;
    use super::super::ui::button_helpers;

    // 检查必填字段
    let required_fields = vec![
        InputFieldType::NewAccountId,
        InputFieldType::NewAccountPassword,
        InputFieldType::NewAccountConfirmPassword,
        InputFieldType::NewAccountEmail,
    ];

    let all_valid = input_helpers::validate_required_fields(world, &required_fields);

    // 更新OK按钮状态
    button_helpers::set_enabled(world, ButtonAction::NewAccountOk, all_valid);
}

/// 从ECS世界获取注册数据
pub fn get_registration_data(world: &World) -> Option<RegistrationData> {
    let mut data = RegistrationData::default();

    // 遍历所有InputField实体
    for (_entity, (input_field, text_input)) in world.query::<(&InputField, &TextInput)>().iter() {
        match input_field.field_type {
            InputFieldType::NewAccountId => {
                data.account_id = text_input.text.clone();
            }
            InputFieldType::NewAccountPassword => {
                data.password = text_input.text.clone();
            }
            InputFieldType::NewAccountConfirmPassword => {
                data.confirm_password = text_input.text.clone();
            }
            InputFieldType::NewAccountEmail => {
                data.email = text_input.text.clone();
            }
            InputFieldType::NewAccountName => {
                data.username = text_input.text.clone();
            }
            InputFieldType::NewAccountQuestion => {
                data.secret_question = text_input.text.clone();
            }
            InputFieldType::NewAccountAnswer => {
                data.secret_answer = text_input.text.clone();
            }
            InputFieldType::NewAccountBirthday => {
                data.birth_date = text_input.text.clone();
            }
            _ => {}
        }
    }

    // 验证必填字段
    if data.account_id.is_empty() || data.password.is_empty() || data.email.is_empty() {
        return None;
    }

    Some(data)
}

/// 注册数据结构
#[derive(Default, Clone)]
pub struct RegistrationData {
    pub account_id: String,
    pub password: String,
    pub confirm_password: String,
    pub email: String,
    pub username: String,
    pub secret_question: String,
    pub secret_answer: String,
    pub birth_date: String,
}
