// ============================================================================
// 角色选择组件 - 用于角色选择界面
// ============================================================================

use mir2_shared::SelectInfo;

/// 角色列表组件
///
/// 🆕 **正确的ECS设计**:
/// - 整个角色列表作为一个组件存储在World中
/// - SelectScene直接引用这个组件,不复制数据
/// - 这是"当前账号的角色列表"这个概念的单一实体
/// - **包含选择状态**: selected_index也属于"角色列表"概念的一部分
///
/// 在登录成功后创建,在进入游戏或返回登录界面时清理
#[derive(Debug, Clone)]
pub struct CharacterList {
    /// 角色列表 (从服务器LoginSuccess获取)
    pub characters: Vec<SelectInfo>,

    /// 当前选中的角色索引 (-1 表示未选中)
    /// 🎯 这是角色列表状态的一部分,应该存在World中而不是UI层
    pub selected_index: i32,
}

impl CharacterList {
    pub fn new(characters: Vec<SelectInfo>) -> Self {
        // 默认选择最后登录的角色(已排序,第一个就是)
        let selected_index = if characters.is_empty() { -1 } else { 0 };

        Self {
            characters,
            selected_index,
        }
    }

    /// 获取当前选中的角色
    pub fn get_selected(&self) -> Option<&SelectInfo> {
        if self.selected_index >= 0 && (self.selected_index as usize) < self.characters.len() {
            Some(&self.characters[self.selected_index as usize])
        } else {
            None
        }
    }

    /// 设置选中的角色索引
    pub fn set_selected(&mut self, index: i32) {
        if index >= -1 && (index as usize) < self.characters.len() {
            self.selected_index = index;
        }
    }
}
