//! 文本输入框UI组件
//! 
//! 将输入框抽象为ECS实体，包含位置、文本、验证、聚焦状态

use hecs::{Entity, World};
use crate::graphics::LibraryName;
use super::super::components::*;

/// 输入框构建器
pub struct TextInputBuilder {
    position: Position,
    size: Size,
    max_length: usize,
    password: bool,
    validation: InputValidation,
    field_type: InputFieldType,
}

impl TextInputBuilder {
    pub fn new(field_type: InputFieldType) -> Self {
        Self {
            position: Position { x: 0.0, y: 0.0 },
            size: Size { width: 150.0, height: 20.0 },
            max_length: 50,
            password: false,
            validation: InputValidation::None,
            field_type,
        }
    }

    pub fn position(mut self, x: f32, y: f32) -> Self {
        self.position = Position { x, y };
        self
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.size = Size { width, height };
        self
    }

    pub fn max_length(mut self, max_length: usize) -> Self {
        self.max_length = max_length;
        self
    }

    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    pub fn validation(mut self, validation: InputValidation) -> Self {
        self.validation = validation;
        self
    }

    /// 创建输入框实体
    pub fn build(self, world: &mut World) -> Entity {
        let bounds = Bounds {
            x: self.position.x,
            y: self.position.y,
            width: self.size.width,
            height: self.size.height,
        };

        let mut text_input = TextInput::new(self.max_length, self.password);
        text_input.validation = self.validation;

        world.spawn((
            TextInputEntity,
            self.position,
            self.size,
            bounds,
            text_input,
            InputField { field_type: self.field_type },
            Focused(false),
            Visible(true),
        ))
    }
}

/// 输入框辅助函数
pub mod input_helpers {
    use super::*;

    /// 聚焦输入框
    pub fn focus_field(world: &mut World, field_type: InputFieldType) {
        // 先取消所有输入框的聚焦
        for (_entity, (mut text_input, mut focused)) in world.query_mut::<(&mut TextInput, &mut Focused)>() {
            text_input.focused = false;
            focused.0 = false;
        }

        // 聚焦目标输入框
        for (_entity, (input_field, mut text_input, mut focused)) in world.query_mut::<(&InputField, &mut TextInput, &mut Focused)>() {
            if input_field.field_type == field_type {
                text_input.focused = true;
                focused.0 = true;
                tracing::debug!("🎯 输入框聚焦: {:?}", field_type);
                break;
            }
        }
    }

    /// 清空输入框
    pub fn clear_field(world: &mut World, field_type: InputFieldType) {
        for (_entity, (input_field, mut text_input)) in world.query_mut::<(&InputField, &mut TextInput)>() {
            if input_field.field_type == field_type {
                text_input.text.clear();
                text_input.validate();
                tracing::debug!("🧹 输入框已清空: {:?}", field_type);
                break;
            }
        }
    }

    /// 获取输入框文本
    pub fn get_text(world: &World, field_type: InputFieldType) -> Option<String> {
        for (_entity, (input_field, text_input)) in world.query::<(&InputField, &TextInput)>().iter() {
            if input_field.field_type == field_type {
                return Some(text_input.text.clone());
            }
        }
        None
    }

    /// 设置输入框文本
    pub fn set_text(world: &mut World, field_type: InputFieldType, text: String) {
        for (_entity, (input_field, mut text_input)) in world.query_mut::<(&InputField, &mut TextInput)>() {
            if input_field.field_type == field_type {
                text_input.text = text;
                text_input.validate();
                break;
            }
        }
    }

    /// 处理键盘输入
    pub fn handle_char_input(world: &mut World, ch: char) {
        for (_entity, (mut text_input, focused)) in world.query_mut::<(&mut TextInput, &Focused)>() {
            if focused.0 && text_input.text.len() < text_input.max_length {
                text_input.text.push(ch);
                text_input.validate();
                break;
            }
        }
    }

    /// 处理退格键
    pub fn handle_backspace(world: &mut World) {
        for (_entity, (mut text_input, focused)) in world.query_mut::<(&mut TextInput, &Focused)>() {
            if focused.0 {
                text_input.text.pop();
                text_input.validate();
                break;
            }
        }
    }

    /// 处理Tab键（切换到下一个输入框）
    pub fn handle_tab(world: &mut World) {
        // 找到当前聚焦的输入框类型
        let mut current_field = None;
        for (_entity, (input_field, focused)) in world.query::<(&InputField, &Focused)>().iter() {
            if focused.0 {
                current_field = Some(input_field.field_type);
                break;
            }
        }

        // 根据当前输入框确定下一个输入框
        if let Some(current) = current_field {
            let next = get_next_field(current);
            if let Some(next_field) = next {
                focus_field(world, next_field);
            }
        }
    }

    /// 检查所有必填字段是否有效
    pub fn validate_required_fields(world: &World, required_fields: &[InputFieldType]) -> bool {
        for field_type in required_fields {
            let mut found = false;
            let mut valid = false;

            for (_entity, (input_field, text_input)) in world.query::<(&InputField, &TextInput)>().iter() {
                if input_field.field_type == *field_type {
                    found = true;
                    valid = text_input.valid && !text_input.text.is_empty();
                    break;
                }
            }

            if !found || !valid {
                return false;
            }
        }
        true
    }
}

/// 获取下一个输入框（Tab键顺序）
fn get_next_field(current: InputFieldType) -> Option<InputFieldType> {
    use InputFieldType::*;
    match current {
        // LoginDialog
        LoginAccount => Some(LoginPassword),
        LoginPassword => Some(LoginAccount),

        // NewAccountDialog
        NewAccountId => Some(NewAccountPassword),
        NewAccountPassword => Some(NewAccountConfirmPassword),
        NewAccountConfirmPassword => Some(NewAccountEmail),
        NewAccountEmail => Some(NewAccountName),
        NewAccountName => Some(NewAccountQuestion),
        NewAccountQuestion => Some(NewAccountAnswer),
        NewAccountAnswer => Some(NewAccountBirthday),
        NewAccountBirthday => Some(NewAccountId),

        // ChangePasswordDialog
        ChangePasswordAccount => Some(ChangePasswordCurrent),
        ChangePasswordCurrent => Some(ChangePasswordNew),
        ChangePasswordNew => Some(ChangePasswordConfirm),
        ChangePasswordConfirm => Some(ChangePasswordAccount),
    }
}
