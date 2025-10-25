// ============================================================================
// 技能对话框 - SkillsDialog
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;

/// 技能槽位
#[derive(Debug, Clone)]
pub struct SkillSlot {
    /// 技能ID
    pub skill_id: Option<u16>,
    
    /// 技能等级
    pub level: u8,
    
    /// 技能图标索引
    pub icon_index: u16,
    
    /// 是否可用
    pub enabled: bool,
}

impl SkillSlot {
    pub fn new() -> Self {
        Self {
            skill_id: None,
            level: 0,
            icon_index: 0,
            enabled: false,
        }
    }
}

/// 技能对话框
pub struct SkillsDialog {
    /// 背景图像索引 (Prguse2)
    background_index: u16,
    
    /// 对话框位置 (屏幕坐标)
    position: (f32, f32),
    
    /// 对话框尺寸
    size: (f32, f32),
    
    /// 技能槽位 (最多64个)
    skill_slots: Vec<SkillSlot>,
    
    /// 当前选中的技能索引
    selected_skill: Option<usize>,
}

impl SkillsDialog {
    /// 创建新的技能对话框
    pub fn new() -> Self {
        // 初始化64个技能槽位
        let mut skill_slots = Vec::with_capacity(64);
        for _ in 0..64 {
            skill_slots.push(SkillSlot::new());
        }
        
        Self {
            background_index: 213, // 技能对话框背景 (需要从 C# 客户端确认)
            position: (100.0, 100.0),
            size: (300.0, 400.0),
            skill_slots,
            selected_skill: None,
        }
    }
    
    /// 添加技能
    pub fn add_skill(&mut self, skill_id: u16, level: u8, icon_index: u16) -> bool {
        // 查找空闲槽位
        for (i, slot) in self.skill_slots.iter_mut().enumerate() {
            if slot.skill_id.is_none() {
                slot.skill_id = Some(skill_id);
                slot.level = level;
                slot.icon_index = icon_index;
                slot.enabled = true;
                tracing::info!("✅ 添加技能到槽位 {}: ID={}, Level={}", i, skill_id, level);
                return true;
            }
        }
        tracing::warn!("❌ 技能槽位已满，无法添加技能 ID={}", skill_id);
        false
    }
    
    /// 移除技能
    pub fn remove_skill(&mut self, skill_id: u16) {
        for slot in &mut self.skill_slots {
            if slot.skill_id == Some(skill_id) {
                slot.skill_id = None;
                slot.level = 0;
                slot.enabled = false;
                tracing::info!("🗑️ 移除技能 ID={}", skill_id);
                break;
            }
        }
    }
    
    /// 获取技能槽位
    pub fn get_skill(&self, index: usize) -> Option<&SkillSlot> {
        self.skill_slots.get(index)
    }
    
    /// 选中技能
    pub fn select_skill(&mut self, index: usize) {
        if index < self.skill_slots.len() && self.skill_slots[index].skill_id.is_some() {
            self.selected_skill = Some(index);
            tracing::info!("🎯 选中技能槽位 {}", index);
        }
    }
    
    /// 绘制技能对话框
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        // TODO: 实现实际的技能对话框渲染
        // 1. 绘制背景
        // 2. 绘制技能格子 (8x8网格)
        // 3. 绘制技能图标
        // 4. 绘制技能等级
        // 5. 绘制选中高亮
        // 6. 绘制技能描述 (鼠标悬停时)
        
        Ok(())
    }
    
    /// 处理鼠标点击
    pub fn handle_click(&mut self, x: f32, y: f32) -> bool {
        // 检查点击是否在对话框范围内
        if x >= self.position.0 && x <= self.position.0 + self.size.0
            && y >= self.position.1 && y <= self.position.1 + self.size.1
        {
            // TODO: 计算点击的格子索引
            // 选中技能或拖拽到快捷栏
            return true;
        }
        false
    }
}

/// 技能对话框组件
pub struct SkillsDialogComp {
    pub dialog: SkillsDialog,
    pub is_open: bool,
}

impl SkillsDialogComp {
    pub fn new() -> Self {
        Self {
            dialog: SkillsDialog::new(),
            is_open: false,
        }
    }
    
    /// 检查是否打开
    pub fn is_open(&self) -> bool {
        self.is_open
    }
    
    /// 设置打开/关闭
    pub fn set_open(&mut self, open: bool) {
        self.is_open = open;
        // SkillsDialog 本身没有 visible 字段，由 SkillsDialogComp 管理
    }
}
