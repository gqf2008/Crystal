// ============================================================================
// 邮件系统对话框 — MailDialogs (对应 C# MailDialogs.cs)
// ============================================================================
//
// 邮件系统包含三个子组件：邮件列表、邮件阅读、邮件撰写。

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;

/// 邮件摘要信息
#[derive(Debug, Clone)]
pub struct MailSummary {
    /// 邮件ID
    pub mail_id: u64,
    /// 发件人
    pub sender: String,
    /// 主题
    pub subject: String,
    /// 是否已读
    pub is_read: bool,
    /// 是否有附件 (物品/金币)
    pub has_parcel: bool,
    /// 发送时间
    pub date: String,
}

/// 邮件详细内容
#[derive(Debug, Clone)]
pub struct MailContent {
    /// 邮件摘要
    pub summary: MailSummary,
    /// 邮件正文
    pub message: String,
    /// 附件金币数量
    pub gold: u32,
}

/// 邮件系统当前视图
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailView {
    /// 邮件列表
    List,
    /// 阅读邮件
    Read,
    /// 撰写邮件
    Compose,
}

/// 邮件系统动作
#[derive(Debug, Clone)]
pub enum MailAction {
    /// 刷新列表
    Refresh,
    /// 阅读邮件
    ReadMail(u64),
    /// 删除邮件
    DeleteMail(u64),
    /// 领取附件
    CollectParcel(u64),
    /// 发送邮件
    SendMail {
        to: String,
        subject: String,
        message: String,
        gold: u32,
    },
    /// 关闭
    Close,
}

/// 邮件系统对话框
pub struct MailDialog {
    /// 是否可见
    pub visible: bool,
    /// 当前视图
    pub current_view: MailView,
    /// 位置
    pub position: (f32, f32),
    /// 尺寸
    pub size: (f32, f32),
    /// 邮件列表
    pub mails: Vec<MailSummary>,
    /// 当前阅读的邮件
    pub current_mail: Option<MailContent>,
    /// 撰写：收件人
    pub compose_to: String,
    /// 撰写：主题
    pub compose_subject: String,
    /// 撰写：正文
    pub compose_message: String,
    /// 撰写：金币
    pub compose_gold: u32,
    /// 选中的邮件索引
    pub selected_index: Option<usize>,
}

impl MailDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            current_view: MailView::List,
            position: (150.0, 100.0),
            size: (350.0, 380.0),
            mails: Vec::new(),
            current_mail: None,
            compose_to: String::new(),
            compose_subject: String::new(),
            compose_message: String::new(),
            compose_gold: 0,
            selected_index: None,
        }
    }

    /// 打开邮件列表
    pub fn open_list(&mut self) {
        self.visible = true;
        self.current_view = MailView::List;
        tracing::info!("📬 打开邮件列表: {} 封邮件", self.mails.len());
    }

    /// 打开邮件阅读
    pub fn open_read(&mut self, mail: MailContent) {
        self.current_view = MailView::Read;
        tracing::info!("📖 阅读邮件: {} - {}", mail.summary.sender, mail.summary.subject);
        self.current_mail = Some(mail);
    }

    /// 打开邮件撰写
    pub fn open_compose(&mut self) {
        self.current_view = MailView::Compose;
        self.compose_to.clear();
        self.compose_subject.clear();
        self.compose_message.clear();
        self.compose_gold = 0;
    }

    /// 关闭
    pub fn close(&mut self) {
        self.visible = false;
        self.current_mail = None;
    }

    /// 未读邮件数量
    pub fn unread_count(&self) -> usize {
        self.mails.iter().filter(|m| !m.is_read).count()
    }

    /// 绘制
    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult {
        if !self.visible {
            return Ok(());
        }
        // TODO: 根据 current_view 绘制不同界面
        Ok(())
    }

    pub fn handle_click(&mut self, _x: f32, _y: f32) -> Option<MailAction> {
        if !self.visible {
            return None;
        }
        // TODO: 处理按钮/列表点击
        None
    }
}

impl Default for MailDialog {
    fn default() -> Self {
        Self::new()
    }
}
