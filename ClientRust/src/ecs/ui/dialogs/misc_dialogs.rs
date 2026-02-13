// ============================================================================
// 低优先级对话框集合
// ============================================================================
// 包含 Phase 4 的所有剩余对话框的基础实现:
// - HeroDialog (英雄系统)
// - HelpDialog (帮助)
// - NoticeDialog (通知)
// - ChatNoticeDialog (聊天通知)
// - ReportDialog (举报)
// - KeyboardLayoutDialog (键盘布局)
// - RollDialog (抽奖)
// - TimerDialog (计时器)
// - CompassDialog (罗盘)
// - IntelligentCreatureDialog (智能生物)
// - ChatOptionDialog (聊天选项)
// - ItemRentalDialog (物品租借)

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;

// ─── 英雄系统对话框 ────────────────────────────────────────────

/// 英雄对话框 (对应 C# HeroDialogs.cs)
pub struct HeroDialog {
    pub visible: bool,
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub hero_name: String,
    pub hero_level: u16,
    pub hero_class: String,
}

impl HeroDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (200.0, 100.0),
            size: (350.0, 400.0),
            hero_name: String::new(),
            hero_level: 0,
            hero_class: String::new(),
        }
    }
    pub fn close(&mut self) { self.visible = false; }
    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult { Ok(()) }
}
impl Default for HeroDialog { fn default() -> Self { Self::new() } }

// ─── 帮助对话框 ────────────────────────────────────────────────

/// 帮助对话框 (对应 C# HelpDialog.cs)
pub struct HelpDialog {
    pub visible: bool,
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub current_page: usize,
    pub pages: Vec<String>,
}

impl HelpDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (100.0, 80.0),
            size: (450.0, 400.0),
            current_page: 0,
            pages: Vec::new(),
        }
    }
    pub fn close(&mut self) { self.visible = false; }
    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult { Ok(()) }
}
impl Default for HelpDialog { fn default() -> Self { Self::new() } }

// ─── 通知对话框 ────────────────────────────────────────────────

/// 通知对话框 (对应 C# NoticeDialog.cs)
pub struct NoticeDialog {
    pub visible: bool,
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub notices: Vec<String>,
    pub current_index: usize,
}

impl NoticeDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (150.0, 100.0),
            size: (400.0, 350.0),
            notices: Vec::new(),
            current_index: 0,
        }
    }
    pub fn close(&mut self) { self.visible = false; }
    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult { Ok(()) }
}
impl Default for NoticeDialog { fn default() -> Self { Self::new() } }

// ─── 聊天通知对话框 ────────────────────────────────────────────

/// 聊天通知对话框 (对应 C# ChatNoticeDialog.cs)
pub struct ChatNoticeDialog {
    pub visible: bool,
    pub position: (f32, f32),
    pub message: String,
}

impl ChatNoticeDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (0.0, 0.0),
            message: String::new(),
        }
    }
    pub fn show(&mut self, message: &str) {
        self.visible = true;
        self.message = message.to_string();
    }
    pub fn close(&mut self) { self.visible = false; }
    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult { Ok(()) }
}
impl Default for ChatNoticeDialog { fn default() -> Self { Self::new() } }

// ─── 举报对话框 ────────────────────────────────────────────────

/// 举报对话框 (对应 C# ReportDialog.cs)
pub struct ReportDialog {
    pub visible: bool,
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub target_name: String,
    pub reason: String,
}

impl ReportDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (200.0, 200.0),
            size: (300.0, 200.0),
            target_name: String::new(),
            reason: String::new(),
        }
    }
    pub fn close(&mut self) { self.visible = false; }
    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult { Ok(()) }
}
impl Default for ReportDialog { fn default() -> Self { Self::new() } }

// ─── 键盘布局对话框 ────────────────────────────────────────────

/// 键盘布局对话框 (对应 C# KeyboardLayoutDialog.cs)
pub struct KeyboardLayoutDialog {
    pub visible: bool,
    pub position: (f32, f32),
    pub size: (f32, f32),
}

impl KeyboardLayoutDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (50.0, 50.0),
            size: (600.0, 400.0),
        }
    }
    pub fn close(&mut self) { self.visible = false; }
    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult { Ok(()) }
}
impl Default for KeyboardLayoutDialog { fn default() -> Self { Self::new() } }

// ─── 抽奖对话框 ────────────────────────────────────────────────

/// 抽奖对话框 (对应 C# RollDialog.cs)
pub struct RollDialog {
    pub visible: bool,
    pub position: (f32, f32),
    pub size: (f32, f32),
    /// 抽奖结果
    pub result: Option<u32>,
    /// 是否正在抽奖
    pub is_rolling: bool,
}

impl RollDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (300.0, 250.0),
            size: (200.0, 150.0),
            result: None,
            is_rolling: false,
        }
    }
    pub fn close(&mut self) { self.visible = false; }
    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult { Ok(()) }
}
impl Default for RollDialog { fn default() -> Self { Self::new() } }

// ─── 计时器对话框 ────────────────────────────────────────────────

/// 计时器对话框 (对应 C# TimerDialog.cs)
pub struct TimerDialog {
    pub visible: bool,
    pub position: (f32, f32),
    /// 剩余秒数
    pub remaining_seconds: u32,
    /// 标题
    pub title: String,
    /// 类型 (普通/紧急)
    pub timer_type: u8,
}

impl TimerDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (0.0, 0.0),
            remaining_seconds: 0,
            title: String::new(),
            timer_type: 0,
        }
    }
    pub fn start(&mut self, title: &str, seconds: u32) {
        self.visible = true;
        self.title = title.to_string();
        self.remaining_seconds = seconds;
    }
    pub fn close(&mut self) { self.visible = false; }
    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult { Ok(()) }
}
impl Default for TimerDialog { fn default() -> Self { Self::new() } }

// ─── 罗盘对话框 ────────────────────────────────────────────────

/// 罗盘对话框 (对应 C# CompassDialog.cs)
pub struct CompassDialog {
    pub visible: bool,
    pub position: (f32, f32),
    /// 当前朝向角度
    pub angle: f32,
}

impl CompassDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (0.0, 0.0),
            angle: 0.0,
        }
    }
    pub fn close(&mut self) { self.visible = false; }
    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult { Ok(()) }
}
impl Default for CompassDialog { fn default() -> Self { Self::new() } }

// ─── 智能生物对话框 ────────────────────────────────────────────

/// 智能生物 (宠物) 对话框 (对应 C# IntelligentCreatureDialogs.cs)
pub struct IntelligentCreatureDialog {
    pub visible: bool,
    pub position: (f32, f32),
    pub size: (f32, f32),
    /// 宠物名称
    pub creature_name: String,
    /// 宠物等级
    pub creature_level: u16,
    /// 亲密度
    pub loyalty: u32,
}

impl IntelligentCreatureDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (150.0, 100.0),
            size: (350.0, 380.0),
            creature_name: String::new(),
            creature_level: 0,
            loyalty: 0,
        }
    }
    pub fn close(&mut self) { self.visible = false; }
    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult { Ok(()) }
}
impl Default for IntelligentCreatureDialog { fn default() -> Self { Self::new() } }

// ─── 聊天选项对话框 ────────────────────────────────────────────

/// 聊天选项对话框 (对应 C# ChatOptionDialog.cs)
pub struct ChatOptionDialog {
    pub visible: bool,
    pub position: (f32, f32),
    pub size: (f32, f32),
    /// 聊天过滤选项
    pub show_normal: bool,
    pub show_shout: bool,
    pub show_whisper: bool,
    pub show_group: bool,
    pub show_guild: bool,
    pub show_system: bool,
}

impl ChatOptionDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (100.0, 300.0),
            size: (200.0, 200.0),
            show_normal: true,
            show_shout: true,
            show_whisper: true,
            show_group: true,
            show_guild: true,
            show_system: true,
        }
    }
    pub fn close(&mut self) { self.visible = false; }
    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult { Ok(()) }
}
impl Default for ChatOptionDialog { fn default() -> Self { Self::new() } }

// ─── 物品租借对话框 ────────────────────────────────────────────

/// 物品租借对话框 (对应 C# ItemRentalDialog.cs)
pub struct ItemRentalDialog {
    pub visible: bool,
    pub position: (f32, f32),
    pub size: (f32, f32),
    /// 租借方名称
    pub renter_name: String,
    /// 租金
    pub rental_fee: u32,
}

impl ItemRentalDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (200.0, 150.0),
            size: (280.0, 300.0),
            renter_name: String::new(),
            rental_fee: 0,
        }
    }
    pub fn close(&mut self) { self.visible = false; }
    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult { Ok(()) }
}
impl Default for ItemRentalDialog { fn default() -> Self { Self::new() } }

/// 物品出租对话框 (对应 C# ItemRentDialog.cs)
pub struct ItemRentDialog {
    pub visible: bool,
    pub position: (f32, f32),
    pub size: (f32, f32),
}

impl ItemRentDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (200.0, 150.0),
            size: (280.0, 250.0),
        }
    }
    pub fn close(&mut self) { self.visible = false; }
    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult { Ok(()) }
}
impl Default for ItemRentDialog { fn default() -> Self { Self::new() } }

/// 物品租借进行中对话框 (对应 C# ItemRentingDialog.cs)
pub struct ItemRentingDialog {
    pub visible: bool,
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub rental_item: Option<super::controls::CellItemInfo>,
}

impl ItemRentingDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (200.0, 150.0),
            size: (280.0, 300.0),
            rental_item: None,
        }
    }
    pub fn close(&mut self) { self.visible = false; }
    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult { Ok(()) }
}
impl Default for ItemRentingDialog { fn default() -> Self { Self::new() } }
