// Big Button - 大按钮
// 对应C#的BigButton类

/// 大按钮
pub struct BigButton {
    text: String,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    enabled: bool,
    visible: bool,
}

impl BigButton {
    /// 创建新的大按钮
    pub fn new(text: String) -> Self {
        Self {
            text,
            x: 0,
            y: 0,
            width: 237,
            height: 40,
            enabled: true,
            visible: true,
        }
    }

    /// 设置位置
    pub fn set_position(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    /// 设置大小
    pub fn set_size(&mut self, width: i32, height: i32) {
        self.width = width;
        self.height = height;
    }

    /// 设置文本
    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }

    /// 获取文本
    pub fn get_text(&self) -> &str {
        &self.text
    }

    /// 启用按钮
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// 禁用按钮
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 显示按钮
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// 隐藏按钮
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// 检查是否可见
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// 检查点是否在按钮内
    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        self.visible && x >= self.x && x < self.x + self.width &&
        y >= self.y && y < self.y + self.height
    }

    /// 获取位置
    pub fn position(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    /// 获取大小
    pub fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }
}

impl Default for BigButton {
    fn default() -> Self {
        Self::new(String::new())
    }
}