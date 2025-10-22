// CharacterDialog - 角色状态对话框
// 参考: Client/MirScenes/Dialogs/CharacterDialog.cs

use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, Color, DrawParam, Rect, Text};

/// 角色对话框标签页
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CharacterTab {
    Equipment,  // 装备页 (角色模型 + 装备栏)
    Status,     // 属性页 (攻击/防御等)
    Stats,      // 状态页 (负重/恢复等)
    Skills,     // 技能页 (已废弃，使用独立MagicDialog)
}

/// 装备槽位
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EquipmentSlot {
    Weapon = 0,      // 武器
    Armour = 1,      // 衣服
    Helmet = 2,      // 头盔
    Torch = 3,       // 火把
    Necklace = 4,    // 项链
    BraceletL = 5,   // 左手镯
    BraceletR = 6,   // 右手镯
    RingL = 7,       // 左戒指
    RingR = 8,       // 右戒指
    Amulet = 9,      // 护身符
    Belt = 10,       // 腰带
    Boots = 11,      // 鞋子
    Stone = 12,      // 宝石
    Mount = 13,      // 坐骑
}

impl EquipmentSlot {
    pub fn all() -> [EquipmentSlot; 14] {
        [
            EquipmentSlot::Weapon,
            EquipmentSlot::Armour,
            EquipmentSlot::Helmet,
            EquipmentSlot::Torch,
            EquipmentSlot::Necklace,
            EquipmentSlot::BraceletL,
            EquipmentSlot::BraceletR,
            EquipmentSlot::RingL,
            EquipmentSlot::RingR,
            EquipmentSlot::Amulet,
            EquipmentSlot::Belt,
            EquipmentSlot::Boots,
            EquipmentSlot::Stone,
            EquipmentSlot::Mount,
        ]
    }
    
    /// 获取装备槽位的屏幕位置 (相对于对话框)
    pub fn get_position(&self) -> (f32, f32) {
        match self {
            EquipmentSlot::Weapon => (123.0, 268.0),
            EquipmentSlot::Armour => (163.0, 268.0),
            EquipmentSlot::Helmet => (203.0, 268.0),
            EquipmentSlot::Torch => (203.0, 308.0),
            EquipmentSlot::Necklace => (203.0, 348.0),
            EquipmentSlot::BraceletL => (8.0, 348.0),
            EquipmentSlot::BraceletR => (123.0, 348.0),
            EquipmentSlot::RingL => (8.0, 308.0),
            EquipmentSlot::RingR => (123.0, 308.0),
            EquipmentSlot::Amulet => (8.0, 268.0),
            EquipmentSlot::Belt => (48.0, 268.0),
            EquipmentSlot::Boots => (48.0, 308.0),
            EquipmentSlot::Stone => (48.0, 348.0),
            EquipmentSlot::Mount => (163.0, 348.0),
        }
    }
}

/// 角色对话框
pub struct CharacterDialog {
    pub visible: bool,
    pub current_tab: CharacterTab,
    
    // 对话框位置和尺寸
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    
    // 玩家属性数据
    pub name: String,
    pub level: u16,
    pub class: String,
    
    // 攻击/防御属性
    pub ac_min: i32,
    pub ac_max: i32,
    pub mac_min: i32,
    pub mac_max: i32,
    pub dc_min: i32,
    pub dc_max: i32,
    pub mc_min: i32,
    pub mc_max: i32,
    pub sc_min: i32,
    pub sc_max: i32,
    
    // 生命/魔法
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    
    // 其他属性
    pub accuracy: i32,
    pub agility: i32,
    pub luck: i32,
    pub attack_speed: i32,
    pub crit_rate: i32,
    pub crit_damage: i32,
    
    // 负重
    pub bag_weight: i32,
    pub max_bag_weight: i32,
    pub wear_weight: i32,
    pub max_wear_weight: i32,
    pub hand_weight: i32,
    pub max_hand_weight: i32,
    
    // 经验
    pub experience: i64,
    pub max_experience: i64,
    
    // 抗性
    pub magic_resist: i32,
    pub poison_resist: i32,
    pub health_recovery: i32,
    pub mana_recovery: i32,
    pub poison_recovery: i32,
    pub holy: i32,
    pub freezing: i32,
    pub poison_attack: i32,
    
    // 装备槽位
    pub equipment: [Option<mir2_shared::data::item::UserItem>; 14],
    
    // UI状态
    hover_slot: Option<EquipmentSlot>,
}

impl CharacterDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            current_tab: CharacterTab::Equipment,
            
            x: 800.0,
            y: 0.0,
            width: 264.0,
            height: 450.0,
            
            name: "未知".to_string(),
            level: 1,
            class: "战士".to_string(),
            
            ac_min: 0,
            ac_max: 0,
            mac_min: 0,
            mac_max: 0,
            dc_min: 0,
            dc_max: 0,
            mc_min: 0,
            mc_max: 0,
            sc_min: 0,
            sc_max: 0,
            
            hp: 100,
            max_hp: 100,
            mp: 50,
            max_mp: 50,
            
            accuracy: 0,
            agility: 0,
            luck: 0,
            attack_speed: 0,
            crit_rate: 0,
            crit_damage: 0,
            
            bag_weight: 0,
            max_bag_weight: 100,
            wear_weight: 0,
            max_wear_weight: 50,
            hand_weight: 0,
            max_hand_weight: 20,
            
            experience: 0,
            max_experience: 100,
            
            magic_resist: 0,
            poison_resist: 0,
            health_recovery: 0,
            mana_recovery: 0,
            poison_recovery: 0,
            holy: 0,
            freezing: 0,
            poison_attack: 0,
            
            equipment: [None, None, None, None, None, None, None, 
                       None, None, None, None, None, None, None],
            
            hover_slot: None,
        }
    }
    
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }
    
    pub fn show(&mut self) {
        self.visible = true;
    }
    
    pub fn hide(&mut self) {
        self.visible = false;
    }
    
    pub fn switch_tab(&mut self, tab: CharacterTab) {
        self.current_tab = tab;
    }
    
    /// 检查点击是否在关闭按钮上
    pub fn is_close_button(&self, x: f32, y: f32) -> bool {
        if !self.visible {
            return false;
        }
        
        let close_x = self.x + self.width - 30.0;
        let close_y = self.y + 5.0;
        
        x >= close_x && x < close_x + 25.0 && y >= close_y && y < close_y + 25.0
    }
    
    /// 检查点击是否在标签按钮上
    pub fn get_tab_at(&self, x: f32, y: f32) -> Option<CharacterTab> {
        if !self.visible {
            return None;
        }
        
        let tab_y = self.y + 70.0;
        let tab_height = 20.0;
        
        if y < tab_y || y > tab_y + tab_height {
            return None;
        }
        
        // 4个标签按钮，每个宽度约60像素
        let relative_x = x - self.x;
        if relative_x >= 8.0 && relative_x < 68.0 {
            Some(CharacterTab::Equipment)
        } else if relative_x >= 68.0 && relative_x < 128.0 {
            Some(CharacterTab::Status)
        } else if relative_x >= 128.0 && relative_x < 188.0 {
            Some(CharacterTab::Stats)
        } else if relative_x >= 188.0 && relative_x < 248.0 {
            Some(CharacterTab::Skills)
        } else {
            None
        }
    }
    
    /// 检查点击是否在装备槽位上
    pub fn get_equipment_slot_at(&self, x: f32, y: f32) -> Option<EquipmentSlot> {
        if !self.visible || self.current_tab != CharacterTab::Equipment {
            return None;
        }
        
        const SLOT_SIZE: f32 = 36.0;
        
        for slot in EquipmentSlot::all() {
            let (slot_x, slot_y) = slot.get_position();
            let abs_x = self.x + slot_x;
            let abs_y = self.y + slot_y;
            
            if x >= abs_x && x < abs_x + SLOT_SIZE &&
               y >= abs_y && y < abs_y + SLOT_SIZE {
                return Some(slot);
            }
        }
        
        None
    }
    
    pub fn update_hover(&mut self, x: f32, y: f32) {
        self.hover_slot = self.get_equipment_slot_at(x, y);
    }
    
    pub fn on_mouse_down(&mut self, x: f32, y: f32) -> Option<CharacterAction> {
        if !self.visible {
            return None;
        }
        
        // 检查关闭按钮
        if self.is_close_button(x, y) {
            return Some(CharacterAction::Close);
        }
        
        // 检查标签切换
        if let Some(tab) = self.get_tab_at(x, y) {
            return Some(CharacterAction::SwitchTab(tab));
        }
        
        // 检查装备槽位点击
        if let Some(slot) = self.get_equipment_slot_at(x, y) {
            return Some(CharacterAction::EquipmentClick(slot));
        }
        
        None
    }
    
    pub fn draw(&self, _ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        if !self.visible {
            return Ok(());
        }
        
        // 绘制对话框背景
        let bg_rect = Rect::new(self.x, self.y, self.width, self.height);
        let bg_mesh = ggez::graphics::Mesh::new_rectangle(
            _ctx,
            ggez::graphics::DrawMode::fill(),
            bg_rect,
            Color::from_rgba(40, 40, 50, 230),
        )?;
        canvas.draw(&bg_mesh, DrawParam::default());
        
        // 绘制边框
        let border_mesh = ggez::graphics::Mesh::new_rectangle(
            _ctx,
            ggez::graphics::DrawMode::stroke(2.0),
            bg_rect,
            Color::from_rgb(100, 100, 120),
        )?;
        canvas.draw(&border_mesh, DrawParam::default());
        
        // 绘制标题
        let title = Text::new(format!("角色 - {}", self.name));
        canvas.draw(
            &title,
            DrawParam::default()
                .dest([self.x + 10.0, self.y + 10.0])
                .color(Color::from_rgb(255, 255, 200)),
        );
        
        // 绘制关闭按钮
        let close_text = Text::new("✕");
        canvas.draw(
            &close_text,
            DrawParam::default()
                .dest([self.x + self.width - 25.0, self.y + 8.0])
                .color(Color::from_rgb(255, 100, 100)),
        );
        
        // 绘制标签按钮
        self.draw_tabs(_ctx, canvas)?;
        
        // 根据当前标签绘制内容
        match self.current_tab {
            CharacterTab::Equipment => self.draw_equipment_page(_ctx, canvas)?,
            CharacterTab::Status => self.draw_status_page(_ctx, canvas)?,
            CharacterTab::Stats => self.draw_stats_page(_ctx, canvas)?,
            CharacterTab::Skills => self.draw_skills_page(_ctx, canvas)?,
        }
        
        Ok(())
    }
    
    fn draw_tabs(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        let tab_y = self.y + 70.0;
        let tab_height = 20.0;
        
        let tabs = [
            ("装备", CharacterTab::Equipment),
            ("属性", CharacterTab::Status),
            ("状态", CharacterTab::Stats),
            ("技能", CharacterTab::Skills),
        ];
        
        for (i, (name, tab)) in tabs.iter().enumerate() {
            let tab_x = self.x + 8.0 + i as f32 * 60.0;
            let is_active = *tab == self.current_tab;
            
            let tab_rect = Rect::new(tab_x, tab_y, 60.0, tab_height);
            let tab_color = if is_active {
                Color::from_rgba(80, 80, 100, 200)
            } else {
                Color::from_rgba(50, 50, 60, 150)
            };
            
            let tab_mesh = ggez::graphics::Mesh::new_rectangle(
                ctx,
                ggez::graphics::DrawMode::fill(),
                tab_rect,
                tab_color,
            )?;
            canvas.draw(&tab_mesh, DrawParam::default());
            
            let tab_text = Text::new(*name);
            canvas.draw(
                &tab_text,
                DrawParam::default()
                    .dest([tab_x + 5.0, tab_y + 2.0])
                    .color(if is_active {
                        Color::WHITE
                    } else {
                        Color::from_rgb(180, 180, 180)
                    }),
            );
        }
        
        Ok(())
    }
    
    fn draw_equipment_page(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        let page_y = self.y + 100.0;
        
        // 绘制角色模型区域 (简化版 - 用矩形表示)
        let model_rect = Rect::new(self.x + 80.0, page_y, 104.0, 150.0);
        let model_mesh = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::stroke(1.0),
            model_rect,
            Color::from_rgb(80, 80, 80),
        )?;
        canvas.draw(&model_mesh, DrawParam::default());
        
        // 绘制"角色预览"文字
        let preview_text = Text::new("角色预览");
        canvas.draw(
            &preview_text,
            DrawParam::default()
                .dest([self.x + 100.0, page_y + 60.0])
                .color(Color::from_rgb(150, 150, 150)),
        );
        
        // 绘制装备槽位
        const SLOT_SIZE: f32 = 36.0;
        
        for slot in EquipmentSlot::all() {
            let (slot_x, slot_y) = slot.get_position();
            let abs_x = self.x + slot_x;
            let abs_y = self.y + slot_y;
            
            // 判断是否悬停
            let is_hover = self.hover_slot == Some(slot);
            
            let slot_rect = Rect::new(abs_x, abs_y, SLOT_SIZE, SLOT_SIZE);
            let slot_color = if is_hover {
                Color::from_rgba(100, 100, 120, 180)
            } else {
                Color::from_rgba(60, 60, 70, 180)
            };
            
            let slot_mesh = ggez::graphics::Mesh::new_rectangle(
                ctx,
                ggez::graphics::DrawMode::fill(),
                slot_rect,
                slot_color,
            )?;
            canvas.draw(&slot_mesh, DrawParam::default());
            
            // 绘制槽位边框
            let border_mesh = ggez::graphics::Mesh::new_rectangle(
                ctx,
                ggez::graphics::DrawMode::stroke(1.0),
                slot_rect,
                Color::from_rgb(100, 100, 100),
            )?;
            canvas.draw(&border_mesh, DrawParam::default());
            
            // 如果有装备，显示物品名称
            if let Some(item) = &self.equipment[slot as usize] {
                let item_name = item.info.as_ref()
                    .map(|i| i.name.as_str())
                    .unwrap_or("?");
                
                // 只显示首字符
                let display_text = item_name.chars().next().unwrap_or('?').to_string();
                let item_text = Text::new(display_text);
                canvas.draw(
                    &item_text,
                    DrawParam::default()
                        .dest([abs_x + 12.0, abs_y + 10.0])
                        .color(Color::from_rgb(255, 200, 100)),
                );
            }
        }
        
        Ok(())
    }
    
    fn draw_status_page(&self, _ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        let page_y = self.y + 100.0;
        let line_height = 18.0;
        
        let stats = [
            ("攻击", format!("{}-{}", self.dc_min, self.dc_max)),
            ("魔法", format!("{}-{}", self.mc_min, self.mc_max)),
            ("道术", format!("{}-{}", self.sc_min, self.sc_max)),
            ("防御", format!("{}-{}", self.ac_min, self.ac_max)),
            ("魔防", format!("{}-{}", self.mac_min, self.mac_max)),
            ("生命", format!("{}/{}", self.hp, self.max_hp)),
            ("魔法", format!("{}/{}", self.mp, self.max_mp)),
            ("准确", format!("+{}", self.accuracy)),
            ("敏捷", format!("+{}", self.agility)),
            ("幸运", format!("{}", self.luck)),
            ("攻速", format!("{}", self.attack_speed)),
            ("暴击率", format!("{}%", self.crit_rate)),
            ("暴击伤", format!("{}", self.crit_damage)),
        ];
        
        for (i, (name, value)) in stats.iter().enumerate() {
            let y = page_y + i as f32 * line_height;
            
            let name_text = Text::new(format!("{}:", name));
            canvas.draw(
                &name_text,
                DrawParam::default()
                    .dest([self.x + 20.0, y])
                    .color(Color::from_rgb(200, 200, 200)),
            );
            
            let value_text = Text::new(value.clone());
            canvas.draw(
                &value_text,
                DrawParam::default()
                    .dest([self.x + 120.0, y])
                    .color(Color::from_rgb(255, 255, 100)),
            );
        }
        
        Ok(())
    }
    
    fn draw_stats_page(&self, _ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        let page_y = self.y + 100.0;
        let line_height = 18.0;
        
        let exp_percent = if self.max_experience > 0 {
            (self.experience as f64 / self.max_experience as f64 * 100.0) as i32
        } else {
            0
        };
        
        let stats = [
            ("经验值", format!("{}%", exp_percent)),
            ("背包负重", format!("{}/{}", self.bag_weight, self.max_bag_weight)),
            ("装备负重", format!("{}/{}", self.wear_weight, self.max_wear_weight)),
            ("腕力负重", format!("{}/{}", self.hand_weight, self.max_hand_weight)),
            ("魔法抗性", format!("+{}", self.magic_resist)),
            ("毒素抗性", format!("+{}", self.poison_resist)),
            ("生命恢复", format!("+{}", self.health_recovery)),
            ("魔法恢复", format!("+{}", self.mana_recovery)),
            ("毒素恢复", format!("+{}", self.poison_recovery)),
            ("神圣", format!("+{}", self.holy)),
            ("冰冻", format!("+{}", self.freezing)),
            ("毒素攻击", format!("+{}", self.poison_attack)),
        ];
        
        for (i, (name, value)) in stats.iter().enumerate() {
            let y = page_y + i as f32 * line_height;
            
            let name_text = Text::new(format!("{}:", name));
            canvas.draw(
                &name_text,
                DrawParam::default()
                    .dest([self.x + 20.0, y])
                    .color(Color::from_rgb(200, 200, 200)),
            );
            
            let value_text = Text::new(value.clone());
            canvas.draw(
                &value_text,
                DrawParam::default()
                    .dest([self.x + 120.0, y])
                    .color(Color::from_rgb(255, 255, 100)),
            );
        }
        
        Ok(())
    }
    
    fn draw_skills_page(&self, _ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        let page_y = self.y + 120.0;
        
        let hint_text = Text::new("请使用 [M] 键打开技能栏");
        canvas.draw(
            &hint_text,
            DrawParam::default()
                .dest([self.x + 40.0, page_y])
                .color(Color::from_rgb(150, 150, 150)),
        );
        
        Ok(())
    }
}

/// 角色对话框操作
#[derive(Debug, Clone)]
pub enum CharacterAction {
    Close,
    SwitchTab(CharacterTab),
    EquipmentClick(EquipmentSlot),
}
