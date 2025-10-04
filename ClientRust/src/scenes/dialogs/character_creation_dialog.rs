// Character Creation Dialog - 角色创建对话框
// 对应 C# 版本: Client/MirScenes/Dialogs/NewCharacterDialog.cs

use mir2_shared::enums::{MirClass, MirGender};

/// 角色创建对话框状态
#[derive(Debug, Clone)]
pub struct CharacterCreationDialog {
    /// 是否显示对话框
    pub visible: bool,
    
    /// 角色名称
    pub name: String,
    
    /// 选择的职业
    pub selected_class: MirClass,
    
    /// 选择的性别
    pub selected_gender: MirGender,
    
    /// 错误消息
    pub error_message: Option<String>,
    
    /// 是否正在创建（等待服务器响应）
    pub creating: bool,
}

impl Default for CharacterCreationDialog {
    fn default() -> Self {
        Self {
            visible: false,
            name: String::new(),
            selected_class: MirClass::Warrior,
            selected_gender: MirGender::Male,
            error_message: None,
            creating: false,
        }
    }
}

impl CharacterCreationDialog {
    /// 创建新对话框
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 显示对话框
    pub fn show(&mut self) {
        self.visible = true;
        self.name.clear();
        self.selected_class = MirClass::Warrior;
        self.selected_gender = MirGender::Male;
        self.error_message = None;
        self.creating = false;
    }
    
    /// 隐藏对话框
    pub fn hide(&mut self) {
        self.visible = false;
    }
    
    /// 验证角色名称
    pub fn validate_name(&self) -> Result<(), String> {
        let name = self.name.trim();
        
        if name.is_empty() {
            return Err("角色名称不能为空".to_string());
        }
        
        if name.len() < 2 {
            return Err("角色名称至少需要2个字符".to_string());
        }
        
        if name.len() > 16 {
            return Err("角色名称最多16个字符".to_string());
        }
        
        // 检查字符是否合法（字母、数字、中文）
        let valid_chars = name.chars().all(|c| {
            c.is_ascii_alphanumeric() || 
            ('\u{4e00}'..='\u{9fa5}').contains(&c) // 中文字符
        });
        
        if !valid_chars {
            return Err("角色名称只能包含字母、数字和中文".to_string());
        }
        
        Ok(())
    }
    
    /// 获取职业描述
    pub fn get_class_description(&self) -> &'static str {
        match self.selected_class {
            MirClass::Warrior => WARRIOR_DESCRIPTION,
            MirClass::Wizard => WIZARD_DESCRIPTION,
            MirClass::Taoist => TAOIST_DESCRIPTION,
            MirClass::Assassin => ASSASSIN_DESCRIPTION,
            MirClass::Archer => ARCHER_DESCRIPTION,
        }
    }
    
    /// 获取职业图标emoji
    pub fn get_class_icon(&self) -> &'static str {
        match self.selected_class {
            MirClass::Warrior => "⚔️",
            MirClass::Wizard => "🔮",
            MirClass::Taoist => "☯️",
            MirClass::Assassin => "🗡️",
            MirClass::Archer => "🏹",
        }
    }
    
    /// 获取性别图标emoji
    pub fn get_gender_icon(&self) -> &'static str {
        match self.selected_gender {
            MirGender::Male => "♂️",
            MirGender::Female => "♀️",
        }
    }
}

// 职业描述文本
const WARRIOR_DESCRIPTION: &str = 
    "战士是力量和体力的化身。他们不容易在战斗中被杀死，并且能够使用各种重型武器和盔甲。\
    战士偏好基于近战物理伤害的攻击。他们的远程攻击较弱，但是专为战士开发的各种装备弥补了他们在远程战斗中的弱点。";

const WIZARD_DESCRIPTION: &str = 
    "法师是力量和耐力较低的职业，但拥有使用强大法术的能力。他们的攻击性法术非常有效，\
    但由于施放这些法术需要时间，因此很容易让自己暴露在敌人的攻击之下。因此，身体虚弱的法师必须在安全距离攻击敌人。";

const TAOIST_DESCRIPTION: &str = 
    "道士除了武功外，还精通天文学、医学等学科。他们的专长不在于直接与敌人交战，\
    而在于用辅助技能协助盟友。道士可以召唤强大的生物，对魔法有很高的抵抗力，是攻守兼备的职业。";

const ASSASSIN_DESCRIPTION: &str = 
    "刺客是秘密组织的成员，他们的历史相对不为人知。他们能够隐藏自己，在别人看不见的情况下进行攻击，\
    这自然使他们擅长快速击杀。由于体力和力量较弱，他们需要避免与多个敌人作战。";

const ARCHER_DESCRIPTION: &str = 
    "弓箭手是精准和力量兼备的职业，使用弓箭的强大技能从远处造成非凡的伤害。\
    就像法师一样，他们依靠敏锐的直觉来躲避迎面而来的攻击，因为他们往往会让自己暴露在正面攻击之下。\
    然而，他们的身体素质和致命的准确性使他们能够让任何被击中的人感到恐惧。";

