// ============================================================================
// NPC 对话框 — NPCDialog (对应 C# NPCDialogs.cs)
// ============================================================================
//
// NPC 对话系统，支持文本显示、滚动、超链接点击。
// 通过正则解析 NPC 对话文本中的链接标签。

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;
use std::time::Instant;

/// NPC 对话框中的文本行类型
#[derive(Debug, Clone)]
pub enum NpcTextLine {
    /// 普通文本
    Text(String),
    /// 可点击链接 (显示文本, 链接目标)
    Link { text: String, target: String },
    /// 大按钮 (显示文本, 链接目标)
    BigButton { text: String, target: String },
}

/// NPC 对话框动作
#[derive(Debug, Clone)]
pub enum NpcDialogAction {
    /// 点击链接
    ClickLink(String),
    /// 关闭对话框
    Close,
    /// 滚动到下一页
    ScrollDown,
    /// 滚动到上一页
    ScrollUp,
    /// 打开任务对话
    OpenQuest,
    /// 打开帮助
    OpenHelp,
}

/// NPC 对话框
pub struct NpcDialog {
    /// 是否可见
    pub visible: bool,
    /// 对话框位置
    pub position: (f32, f32),
    /// 对话框尺寸
    pub size: (f32, f32),
    /// NPC 名称
    pub npc_name: String,
    /// 当前对话文本行
    pub lines: Vec<NpcTextLine>,
    /// 文本链接按钮
    pub text_buttons: Vec<NpcTextLine>,
    /// 大按钮列表
    pub big_buttons: Vec<NpcTextLine>,
    /// 滚动偏移索引
    scroll_index: usize,
    /// 最大可见行数
    pub max_visible_lines: usize,
    /// 背景图像索引
    background_index: u16,
    /// 上次打开时间
    last_open_time: Option<Instant>,
}

impl NpcDialog {
    /// 创建新的 NPC 对话框
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (100.0, 60.0),
            size: (440.0, 200.0),
            npc_name: String::new(),
            lines: Vec::new(),
            text_buttons: Vec::new(),
            big_buttons: Vec::new(),
            scroll_index: 0,
            max_visible_lines: 8,
            background_index: 995,
            last_open_time: None,
        }
    }

    /// 显示 NPC 对话 (解析文本内容)
    pub fn show(&mut self, npc_name: &str, text: &str) {
        self.npc_name = npc_name.to_string();
        self.visible = true;
        self.scroll_index = 0;
        self.last_open_time = Some(Instant::now());
        self.parse_text(text);
        tracing::info!("💬 NPC 对话: {} - {} 行", npc_name, self.lines.len());
    }

    /// 解析 NPC 对话文本
    ///
    /// 支持的标签格式 (对应 C# 正则):
    /// - `<显示文本/@链接目标>` — 普通链接
    /// - `<<显示文本/@链接目标>>` — 大按钮
    /// - `{颜色文本/颜色代码}` — 彩色文本
    /// - `(图片文本/图片路径)` — 内联图片
    fn parse_text(&mut self, text: &str) {
        self.lines.clear();
        self.text_buttons.clear();
        self.big_buttons.clear();

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                self.lines.push(NpcTextLine::Text(String::new()));
                continue;
            }

            // 解析大按钮: <<text/@target>>
            if trimmed.starts_with("<<") && trimmed.ends_with(">>") {
                let inner = &trimmed[2..trimmed.len() - 2];
                if let Some(slash_pos) = inner.find('/') {
                    let display = &inner[..slash_pos];
                    let target = &inner[slash_pos + 1..];
                    let btn = NpcTextLine::BigButton {
                        text: display.to_string(),
                        target: target.to_string(),
                    };
                    self.big_buttons.push(btn.clone());
                    self.lines.push(btn);
                    continue;
                }
            }

            // 解析普通链接: <text/@target>
            let mut remaining = trimmed.to_string();
            let mut has_link = false;
            while let Some(start) = remaining.find('<') {
                if let Some(end) = remaining[start..].find('>') {
                    let inner = &remaining[start + 1..start + end];
                    if let Some(slash_pos) = inner.find('/') {
                        let display = &inner[..slash_pos];
                        let target = &inner[slash_pos + 1..];

                        // 添加链接前的文本
                        let before = &remaining[..start];
                        if !before.is_empty() {
                            self.lines.push(NpcTextLine::Text(before.to_string()));
                        }

                        let link = NpcTextLine::Link {
                            text: display.to_string(),
                            target: target.to_string(),
                        };
                        self.text_buttons.push(link.clone());
                        self.lines.push(link);
                        remaining = remaining[start + end + 1..].to_string();
                        has_link = true;
                        continue;
                    }
                    // '<' found with '>' but no '/' — skip past this '>'
                    remaining = remaining[start + end + 1..].to_string();
                    continue;
                }
                // '<' found but no '>' — treat rest as plain text
                break;
            }

            if !has_link {
                self.lines.push(NpcTextLine::Text(trimmed.to_string()));
            } else if !remaining.is_empty() {
                self.lines.push(NpcTextLine::Text(remaining));
            }
        }
    }

    /// 关闭对话框
    pub fn close(&mut self) {
        self.visible = false;
        self.lines.clear();
        self.text_buttons.clear();
        self.big_buttons.clear();
        self.npc_name.clear();
    }

    /// 向上滚动
    pub fn scroll_up(&mut self) {
        if self.scroll_index > 0 {
            self.scroll_index -= 1;
        }
    }

    /// 向下滚动
    pub fn scroll_down(&mut self) {
        if self.scroll_index + self.max_visible_lines < self.lines.len() {
            self.scroll_index += 1;
        }
    }

    /// 处理鼠标滚轮
    pub fn handle_scroll(&mut self, delta: f32) {
        if delta > 0.0 {
            self.scroll_up();
        } else if delta < 0.0 {
            self.scroll_down();
        }
    }

    /// 是否需要滚动条
    pub fn needs_scrollbar(&self) -> bool {
        self.lines.len() > self.max_visible_lines
    }

    /// 获取当前可见的行
    pub fn visible_lines(&self) -> &[NpcTextLine] {
        let end = (self.scroll_index + self.max_visible_lines).min(self.lines.len());
        &self.lines[self.scroll_index..end]
    }

    /// 处理鼠标点击
    pub fn handle_click(&mut self, x: f32, y: f32) -> Option<NpcDialogAction> {
        if !self.visible {
            return None;
        }

        // 检查是否在对话框范围内
        if x < self.position.0
            || x > self.position.0 + self.size.0
            || y < self.position.1
            || y > self.position.1 + self.size.1
        {
            return None;
        }

        // 检查关闭按钮 (右上角)
        let close_x = self.position.0 + self.size.0 - 25.0;
        let close_y = self.position.1 + 5.0;
        if x >= close_x && x <= close_x + 20.0 && y >= close_y && y <= close_y + 20.0 {
            self.close();
            return Some(NpcDialogAction::Close);
        }

        // 检查滚动按钮
        let scroll_x = self.position.0 + self.size.0 - 20.0;
        if x >= scroll_x {
            if y >= self.position.1 + 30.0 && y <= self.position.1 + 50.0 {
                self.scroll_up();
                return Some(NpcDialogAction::ScrollUp);
            }
            if y >= self.position.1 + self.size.1 - 30.0 {
                self.scroll_down();
                return Some(NpcDialogAction::ScrollDown);
            }
        }

        // 检查文本链接点击
        let text_y_start = self.position.1 + 35.0;
        let line_height = 20.0;
        let visible = self.visible_lines();
        for (i, line) in visible.iter().enumerate() {
            let line_y = text_y_start + (i as f32) * line_height;
            if y >= line_y && y <= line_y + line_height {
                if let NpcTextLine::Link { target, .. } | NpcTextLine::BigButton { target, .. } =
                    line
                {
                    return Some(NpcDialogAction::ClickLink(target.clone()));
                }
            }
        }

        None
    }

    /// 绘制 NPC 对话框
    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult {
        if !self.visible {
            return Ok(());
        }
        // TODO: 绘制对话框背景 (Libraries.Prguse, index 995)
        // TODO: 绘制 NPC 名称
        // TODO: 绘制关闭/滚动按钮
        // TODO: 绘制文本行 (区分普通文本和链接)
        // TODO: 绘制大按钮
        Ok(())
    }
}

impl Default for NpcDialog {
    fn default() -> Self {
        Self::new()
    }
}

/// NPC 掉落面板 (简化版 NPCDropDialog)
pub struct NpcDropDialog {
    /// 是否可见
    pub visible: bool,
    /// 目标物品
    pub target_item: Option<super::controls::CellItemInfo>,
}

impl NpcDropDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            target_item: None,
        }
    }
}

impl Default for NpcDropDialog {
    fn default() -> Self {
        Self::new()
    }
}
