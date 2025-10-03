// Group Dialog - 组队对话框
// 管理队伍成员、邀请、踢出等

use super::Dialog;

/// 队伍成员信息
#[derive(Debug, Clone)]
pub struct GroupMember {
    pub name: String,
    pub level: u16,
    pub class: String,
    pub hp: i32,
    pub max_hp: i32,
    pub online: bool,
    pub is_leader: bool, // 是否队长
}

impl GroupMember {
    /// 创建新队员
    pub fn new(name: String, level: u16, class: String) -> Self {
        Self {
            name,
            level,
            class,
            hp: 100,
            max_hp: 100,
            online: true,
            is_leader: false,
        }
    }

    /// 计算HP百分比
    pub fn get_hp_percent(&self) -> f32 {
        if self.max_hp == 0 {
            return 0.0;
        }
        (self.hp as f32 / self.max_hp as f32) * 100.0
    }

    /// 更新HP
    pub fn update_hp(&mut self, hp: i32, max_hp: i32) {
        self.hp = hp;
        self.max_hp = max_hp;
    }
}

/// 组队对话框
pub struct GroupDialog {
    visible: bool,

    // 队伍设置
    pub allow_group: bool, // 是否允许组队邀请

    // 队员列表 (最多支持Globals.MaxGroup个成员)
    pub members: Vec<GroupMember>,
    pub max_members: usize,

    // 当前用户名称
    pub user_name: String,
}

impl GroupDialog {
    /// 创建新的组队对话框
    pub fn new() -> Self {
        Self {
            visible: false,
            allow_group: true,
            members: Vec::new(),
            max_members: 17, // Globals.MaxGroup
            user_name: String::new(),
        }
    }

    /// 切换允许组队状态
    pub fn toggle_allow_group(&mut self) {
        self.allow_group = !self.allow_group;
    }

    /// 设置允许组队
    pub fn set_allow_group(&mut self, allow: bool) {
        self.allow_group = allow;
    }

    /// 添加队员
    pub fn add_member(&mut self, member: GroupMember) -> bool {
        if self.members.len() >= self.max_members {
            return false;
        }
        if self.members.iter().any(|m| m.name == member.name) {
            return false;
        }
        self.members.push(member);
        true
    }

    /// 移除队员
    pub fn remove_member(&mut self, name: &str) -> bool {
        if let Some(index) = self.members.iter().position(|m| m.name == name) {
            self.members.remove(index);
            return true;
        }
        false
    }

    /// 查找队员
    pub fn find_member(&self, name: &str) -> Option<&GroupMember> {
        self.members.iter().find(|m| m.name == name)
    }

    /// 查找队员(可变)
    pub fn find_member_mut(&mut self, name: &str) -> Option<&mut GroupMember> {
        self.members.iter_mut().find(|m| m.name == name)
    }

    /// 更新队员HP
    pub fn update_member_hp(&mut self, name: &str, hp: i32, max_hp: i32) {
        if let Some(member) = self.find_member_mut(name) {
            member.update_hp(hp, max_hp);
        }
    }

    /// 更新队员在线状态
    pub fn set_member_online(&mut self, name: &str, online: bool) {
        if let Some(member) = self.find_member_mut(name) {
            member.online = online;
        }
    }

    /// 获取队长
    pub fn get_leader(&self) -> Option<&GroupMember> {
        self.members.iter().find(|m| m.is_leader)
    }

    /// 设置队长
    pub fn set_leader(&mut self, name: &str) -> bool {
        // 清除所有队长标记
        for member in &mut self.members {
            member.is_leader = false;
        }
        // 设置新队长
        if let Some(member) = self.find_member_mut(name) {
            member.is_leader = true;
            return true;
        }
        false
    }

    /// 检查是否是队长
    pub fn is_leader(&self, name: &str) -> bool {
        self.find_member(name).map_or(false, |m| m.is_leader)
    }

    /// 检查当前用户是否是队长
    pub fn is_user_leader(&self) -> bool {
        if self.user_name.is_empty() {
            return false;
        }
        self.is_leader(&self.user_name)
    }

    /// 获取在线成员数量
    pub fn online_count(&self) -> usize {
        self.members.iter().filter(|m| m.online).count()
    }

    /// 获取队伍人数
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// 检查队伍是否已满
    pub fn is_full(&self) -> bool {
        self.members.len() >= self.max_members
    }

    /// 检查是否在队伍中
    pub fn is_in_group(&self) -> bool {
        !self.members.is_empty()
    }

    /// 清空队伍
    pub fn clear(&mut self) {
        self.members.clear();
    }

    /// 离开队伍
    pub fn leave_group(&mut self) {
        self.clear();
    }

    /// 解散队伍 (仅队长可用)
    pub fn disband_group(&mut self) -> bool {
        if self.is_user_leader() {
            self.clear();
            return true;
        }
        false
    }

    /// 创建队伍 (当前用户成为队长)
    pub fn create_group(&mut self) {
        self.clear();
        let mut leader = GroupMember::new(
            self.user_name.clone(),
            1,
            String::from("Warrior"),
        );
        leader.is_leader = true;
        self.members.push(leader);
    }

    /// 获取队员名称列表
    pub fn get_member_names(&self) -> Vec<String> {
        self.members.iter().map(|m| m.name.clone()).collect()
    }

    /// 按位置获取队员 (用于UI显示)
    pub fn get_member_by_position(&self, position: usize) -> Option<&GroupMember> {
        self.members.get(position)
    }
}

impl Default for GroupDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Dialog for GroupDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn update(&mut self, _delta_time: f32) {
        // 更新逻辑 (如HP动画等)
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // TODO: 实际渲染逻辑
        // 绘制队伍列表、HP条、队长标记等
    }

    fn is_visible(&self) -> bool {
        self.visible
    }
    
    fn name(&self) -> &str { "GroupDialog" }
    fn contains_point(&self, x: i32, y: i32) -> bool { x >= 0 && x < 350 && y >= 0 && y < 400 }
    fn position(&self) -> (i32, i32) { (0, 0) }
    fn size(&self) -> (i32, i32) { (350, 400) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_member_creation() {
        let member = GroupMember::new("Player1".to_string(), 50, "Warrior".to_string());
        assert_eq!(member.name, "Player1");
        assert_eq!(member.level, 50);
        assert!(!member.is_leader);
    }

    #[test]
    fn test_group_member_hp() {
        let mut member = GroupMember::new("Test".to_string(), 1, "Wizard".to_string());
        member.update_hp(50, 100);
        assert_eq!(member.get_hp_percent(), 50.0);
    }

    #[test]
    fn test_group_dialog_creation() {
        let dialog = GroupDialog::new();
        assert!(!dialog.is_visible());
        assert!(dialog.allow_group);
        assert_eq!(dialog.max_members, 17);
    }

    #[test]
    fn test_allow_group() {
        let mut dialog = GroupDialog::new();
        assert!(dialog.allow_group);
        
        dialog.toggle_allow_group();
        assert!(!dialog.allow_group);
        
        dialog.set_allow_group(true);
        assert!(dialog.allow_group);
    }

    #[test]
    fn test_add_remove_member() {
        let mut dialog = GroupDialog::new();
        
        let member = GroupMember::new("Alice".to_string(), 30, "Taoist".to_string());
        assert!(dialog.add_member(member));
        assert_eq!(dialog.member_count(), 1);
        
        assert!(dialog.remove_member("Alice"));
        assert_eq!(dialog.member_count(), 0);
    }

    #[test]
    fn test_duplicate_member() {
        let mut dialog = GroupDialog::new();
        
        let member1 = GroupMember::new("Bob".to_string(), 25, "Warrior".to_string());
        let member2 = GroupMember::new("Bob".to_string(), 30, "Warrior".to_string());
        
        assert!(dialog.add_member(member1));
        assert!(!dialog.add_member(member2)); // 重复添加失败
        assert_eq!(dialog.member_count(), 1);
    }

    #[test]
    fn test_full_group() {
        let mut dialog = GroupDialog::new();
        dialog.max_members = 3;
        
        for i in 0..3 {
            let member = GroupMember::new(format!("Player{}", i), 1, "Warrior".to_string());
            assert!(dialog.add_member(member));
        }
        
        assert!(dialog.is_full());
        
        let extra = GroupMember::new("Extra".to_string(), 1, "Warrior".to_string());
        assert!(!dialog.add_member(extra));
    }

    #[test]
    fn test_leader() {
        let mut dialog = GroupDialog::new();
        
        let mut member1 = GroupMember::new("Leader".to_string(), 50, "Warrior".to_string());
        member1.is_leader = true;
        let member2 = GroupMember::new("Member".to_string(), 40, "Wizard".to_string());
        
        dialog.add_member(member1);
        dialog.add_member(member2);
        
        assert!(dialog.is_leader("Leader"));
        assert!(!dialog.is_leader("Member"));
        
        let leader = dialog.get_leader();
        assert!(leader.is_some());
        assert_eq!(leader.unwrap().name, "Leader");
    }

    #[test]
    fn test_set_leader() {
        let mut dialog = GroupDialog::new();
        
        dialog.add_member(GroupMember::new("Alice".to_string(), 50, "Warrior".to_string()));
        dialog.add_member(GroupMember::new("Bob".to_string(), 40, "Wizard".to_string()));
        
        assert!(dialog.set_leader("Alice"));
        assert!(dialog.is_leader("Alice"));
        
        assert!(dialog.set_leader("Bob"));
        assert!(!dialog.is_leader("Alice"));
        assert!(dialog.is_leader("Bob"));
    }

    #[test]
    fn test_update_member_hp() {
        let mut dialog = GroupDialog::new();
        
        dialog.add_member(GroupMember::new("Charlie".to_string(), 30, "Taoist".to_string()));
        
        dialog.update_member_hp("Charlie", 200, 500);
        
        let member = dialog.find_member("Charlie").unwrap();
        assert_eq!(member.hp, 200);
        assert_eq!(member.max_hp, 500);
        assert_eq!(member.get_hp_percent(), 40.0);
    }

    #[test]
    fn test_online_status() {
        let mut dialog = GroupDialog::new();
        
        dialog.add_member(GroupMember::new("Dave".to_string(), 25, "Warrior".to_string()));
        assert_eq!(dialog.online_count(), 1);
        
        dialog.set_member_online("Dave", false);
        assert_eq!(dialog.online_count(), 0);
    }

    #[test]
    fn test_is_user_leader() {
        let mut dialog = GroupDialog::new();
        dialog.user_name = "UserPlayer".to_string();
        
        let mut member = GroupMember::new("UserPlayer".to_string(), 50, "Warrior".to_string());
        member.is_leader = true;
        dialog.add_member(member);
        
        assert!(dialog.is_user_leader());
    }

    #[test]
    fn test_create_group() {
        let mut dialog = GroupDialog::new();
        dialog.user_name = "NewLeader".to_string();
        
        dialog.create_group();
        
        assert_eq!(dialog.member_count(), 1);
        assert!(dialog.is_leader("NewLeader"));
        assert!(dialog.is_user_leader());
    }

    #[test]
    fn test_disband_group() {
        let mut dialog = GroupDialog::new();
        dialog.user_name = "Leader".to_string();
        
        dialog.create_group();
        dialog.add_member(GroupMember::new("Member".to_string(), 30, "Wizard".to_string()));
        
        assert!(dialog.disband_group());
        assert_eq!(dialog.member_count(), 0);
    }

    #[test]
    fn test_leave_group() {
        let mut dialog = GroupDialog::new();
        
        dialog.add_member(GroupMember::new("Player1".to_string(), 30, "Warrior".to_string()));
        dialog.add_member(GroupMember::new("Player2".to_string(), 25, "Wizard".to_string()));
        
        dialog.leave_group();
        assert_eq!(dialog.member_count(), 0);
    }

    #[test]
    fn test_get_member_names() {
        let mut dialog = GroupDialog::new();
        
        dialog.add_member(GroupMember::new("Eve".to_string(), 40, "Taoist".to_string()));
        dialog.add_member(GroupMember::new("Frank".to_string(), 35, "Warrior".to_string()));
        
        let names = dialog.get_member_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"Eve".to_string()));
        assert!(names.contains(&"Frank".to_string()));
    }
}
