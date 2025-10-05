// Character Deletion Dialog - 角色删除确认对话框
// 对应 C# 版本: Client/MirScenes/SelectScene.cs DeleteCharacter()

use crate::scenes::SelectCharacter;

/// 角色删除确认对话框状态
#[derive(Debug, Clone)]
pub struct CharacterDeletionDialog {
    /// 是否显示对话框
    pub visible: bool,
    
    /// 是否显示名称输入阶段（第二步）
    pub show_name_input: bool,
    
    /// 要删除的角色
    pub character_to_delete: Option<SelectCharacter>,
    
    /// 用户输入的角色名称
    pub input_name: String,
    
    /// 错误消息
    pub error_message: Option<String>,
    
    /// 是否正在删除（等待服务器响应）
    pub deleting: bool,
}

impl Default for CharacterDeletionDialog {
    fn default() -> Self {
        Self {
            visible: false,
            show_name_input: false,
            character_to_delete: None,
            input_name: String::new(),
            error_message: None,
            deleting: false,
        }
    }
}

impl CharacterDeletionDialog {
    /// 创建新对话框
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 显示对话框 - 第一步：确认删除
    pub fn show(&mut self, character: SelectCharacter) {
        self.visible = true;
        self.show_name_input = false;
        self.character_to_delete = Some(character);
        self.input_name.clear();
        self.error_message = None;
        self.deleting = false;
    }
    
    /// 显示名称输入阶段 - 第二步：输入名称确认
    pub fn show_name_input_stage(&mut self) {
        self.show_name_input = true;
        self.input_name.clear();
        self.error_message = None;
    }
    
    /// 隐藏对话框
    pub fn hide(&mut self) {
        self.visible = false;
        self.show_name_input = false;
        self.character_to_delete = None;
        self.input_name.clear();
        self.error_message = None;
        self.deleting = false;
    }
    
    /// 验证输入的名称是否与要删除的角色匹配
    pub fn validate_name(&self) -> Result<(), String> {
        if let Some(ref character) = self.character_to_delete {
            let input = self.input_name.trim();
            
            if input.is_empty() {
                return Err("请输入角色名称".to_string());
            }
            
            if input != character.name {
                return Err(format!("输入的名称不正确\n请输入: {}", character.name));
            }
            
            Ok(())
        } else {
            Err("未选择角色".to_string())
        }
    }
    
    /// 获取要删除的角色索引
    pub fn get_character_index(&self) -> Option<i32> {
        self.character_to_delete.as_ref().map(|c| c.index as i32)
    }
}
