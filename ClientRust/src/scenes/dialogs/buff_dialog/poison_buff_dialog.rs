// Poison Buff Dialog - 毒Buff显示对话框
// 对应C#的PoisonBuffDialog类

/// 毒类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PoisonType {
    None = 0,
    Green = 1,
    Red = 2,
    Slow = 4,
    Frozen = 8,
    Stun = 16,
    Paralysis = 32,
    DelayedExplosion = 64,
    Bleeding = 128,
    LRParalysis = 256,
    Blindness = 512,
    Dazed = 1024,
}

/// 客户端毒Buff
#[derive(Debug, Clone)]
pub struct ClientPoisonBuff {
    /// 毒类型
    pub poison_type: PoisonType,
    /// 施法者名称
    pub caster: String,
    /// 毒值
    pub value: i32,
    /// 触发间隔 (毫秒)
    pub tick_speed: i32,
    /// 到期时间 (毫秒时间戳)
    pub expire_time: i64,
}

impl ClientPoisonBuff {
    /// 创建新的毒Buff
    pub fn new(poison_type: PoisonType, caster: String, value: i32, tick_speed: i32, expire_time: i64) -> Self {
        Self {
            poison_type,
            caster,
            value,
            tick_speed,
            expire_time,
        }
    }
}

/// 毒Buff对话框
/// 显示角色中毒状态的UI组件
#[derive(Debug, Clone)]
pub struct PoisonBuffDialog {
    /// 毒Buff列表
    pub buffs: Vec<ClientPoisonBuff>,
    /// Buff图像控件列表
    pub buff_images: Vec<BuffImageInfo>,
    /// 是否已淡出
    pub faded_out: bool,
    /// 是否已淡入
    pub faded_in: bool,
    /// Buff数量
    pub buff_count: i32,
    /// 下次淡化时间
    pub next_fade_time: i64,
    /// 是否可见
    pub visible: bool,
    /// 透明度
    pub opacity: f32,
    /// 位置 (x, y)
    pub position: (i32, i32),
    /// 大小 (width, height)
    pub size: (i32, i32),
    /// 扩展/折叠按钮位置
    pub expand_button_position: (i32, i32),
    /// Buff数量标签位置
    pub count_label_position: (i32, i32),
    /// Buff数量标签文本
    pub count_label_text: String,
    /// Buff数量标签是否可见
    pub count_label_visible: bool,
}

impl Default for PoisonBuffDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl PoisonBuffDialog {
    /// 创建新的毒Buff对话框
    pub fn new() -> Self {
        Self {
            buffs: Vec::new(),
            buff_images: Vec::new(),
            faded_out: true,
            faded_in: false,
            buff_count: 0,
            next_fade_time: 0,
            visible: false,
            opacity: 0.0,
            position: (1160, 0), // 假设1280x800屏幕，右侧位置
            size: (44, 34),
            expand_button_position: (29, 0),
            count_label_position: (0, 12),
            count_label_text: String::new(),
            count_label_visible: false,
        }
    }

    /// 添加毒Buff
    pub fn add_buff(&mut self, buff: ClientPoisonBuff) {
        self.buffs.insert(0, buff);
        self.update_window();
    }

    /// 获取毒Buff的图像索引
    pub fn get_buff_image_index(&self, poison_type: PoisonType) -> i32 {
        match poison_type {
            PoisonType::Green => 221,
            PoisonType::Red => 222,
            PoisonType::Slow => 225,
            PoisonType::Frozen => 223,
            PoisonType::Stun => 224,
            PoisonType::Paralysis => 233,
            PoisonType::DelayedExplosion => 229,
            PoisonType::Bleeding => 231,
            PoisonType::LRParalysis => 233,
            PoisonType::Blindness => 226,
            PoisonType::Dazed => 230,
            _ => 0,
        }
    }

    /// 获取毒Buff的描述文本
    pub fn get_buff_description(&self, buff: &ClientPoisonBuff, current_time: i64) -> String {
        let mut text = format!("{:?}\n", buff.poison_type);

        match buff.poison_type {
            PoisonType::Green => {
                let tick_seconds = buff.tick_speed / 1000;
                let tick_name = if tick_seconds > 1 { "seconds" } else { "second" };
                text.push_str(&format!("Receive {} damage every {} {}.\n", buff.value, tick_seconds, tick_name));
            }
            PoisonType::Red => {
                let tick_seconds = buff.tick_speed / 1000;
                let tick_name = if tick_seconds > 1 { "seconds" } else { "second" };
                text.push_str(&format!("Reduces armour rate by 10% every {} {}.\n", tick_seconds, tick_name));
            }
            PoisonType::Slow => {
                text.push_str("Reduces movement speed.\n");
            }
            PoisonType::Frozen => {
                text.push_str("Prevents casting, moving and attacking.\n");
            }
            PoisonType::Stun => {
                let tick_seconds = buff.tick_speed / 1000;
                let tick_name = if tick_seconds > 1 { "seconds" } else { "second" };
                text.push_str(&format!("Increases damage received by 20% every {} {}.\n", tick_seconds, tick_name));
            }
            PoisonType::Paralysis => {
                text.push_str("Prevents moving and attacking.\n");
            }
            PoisonType::DelayedExplosion => {
                text.push_str("Ticking time bomb.\n");
            }
            PoisonType::Bleeding => {
                let tick_seconds = buff.tick_speed / 1000;
                let tick_name = if tick_seconds > 1 { "seconds" } else { "second" };
                text.push_str(&format!("Receive {} damage every {} {}.\n", buff.value, tick_seconds, tick_name));
            }
            PoisonType::LRParalysis => {
                text.push_str("Prevents moving and attacking.\nCancels when attacked\n");
            }
            PoisonType::Blindness => {
                text.push_str("Causes temporary blindness.\n");
            }
            PoisonType::Dazed => {
                text.push_str("Prevents attacking.\n");
            }
            _ => {}
        }

        // 添加剩余时间
        let remaining_seconds = ((buff.expire_time - current_time) / 1000) as i32;
        text.push_str(&format!("Expire: {}\n", format_duration(remaining_seconds.max(0))));

        // 添加施法者信息
        if !buff.caster.is_empty() {
            text.push_str(&format!("Caster: {}", buff.caster));
        }

        text
    }

    /// 获取组合Buff文本
    pub fn get_combined_buff_text(&self) -> String {
        "Active Poisons\n".to_string()
    }

    /// 处理毒Buff对话框逻辑
    pub fn process(&mut self, current_time: i64, mouse_position: (i32, i32), expanded_buff_window: bool) {
        if !self.visible {
            return;
        }

        // 更新Buff图像
        self.update_buff_images(current_time, expanded_buff_window);

        // 处理淡入淡出效果
        self.process_fade_effect(current_time, mouse_position);
    }

    /// 更新Buff图像
    fn update_buff_images(&mut self, current_time: i64, expanded_buff_window: bool) {
        // 确保图像数量与Buff数量匹配
        while self.buff_images.len() < self.buffs.len() {
            self.buff_images.push(BuffImageInfo::default());
        }
        while self.buff_images.len() > self.buffs.len() {
            self.buff_images.pop();
        }

        for (i, (image, buff)) in self.buff_images.iter_mut().zip(&self.buffs).enumerate() {
            // 计算位置
            let i_i32 = i as i32;
            let location_x = self.size.0 - 10 - 23 - (i_i32 * 23) + ((10 * 23) * (i_i32 / 10));
            let location_y = 6 + ((i_i32 / 10) * 24);

            image.location = (location_x, location_y as i32);
            image.visible = expanded_buff_window || (!expanded_buff_window && i == 0);
            image.opacity = if image.visible { 1.0 } else { 0.6 };

            // 处理即将到期的Buff闪烁效果
            let remaining_seconds = ((buff.expire_time - current_time) / 1000) as f64;
            if remaining_seconds <= 5.0 {
                let time = (buff.expire_time - current_time) / 100;
                if (time as f64 / 10.0 % 10.0) < 5.0 {
                    image.index = -1;
                }
            }
        }

        // 预先计算所有需要的索引和描述，避免借用冲突
        let mut indices_and_hints = Vec::new();
        for (_i, buff) in self.buffs.iter().enumerate() {
            let index = self.get_buff_image_index(buff.poison_type);
            let hint = if expanded_buff_window {
                self.get_buff_description(buff, current_time)
            } else {
                self.get_combined_buff_text()
            };
            indices_and_hints.push((index, hint));
        }

        // 单独设置图像索引和hint，避免借用冲突
        for (i, image) in self.buff_images.iter_mut().enumerate() {
            if let Some((index, hint)) = indices_and_hints.get(i) {
                image.index = *index;
                image.hint = hint.clone();
            }
        }
    }

    /// 处理淡入淡出效果
    fn process_fade_effect(&mut self, current_time: i64, mouse_position: (i32, i32)) {
        const FADE_DELAY: i64 = 55;
        const FADE_RATE: f32 = 0.2;

        let mouse_over = self.is_mouse_over(mouse_position);

        if mouse_over {
            if self.buff_count == 0 || (!self.faded_in && current_time <= self.next_fade_time) {
                return;
            }

            self.opacity += FADE_RATE;

            if self.opacity > 1.0 {
                self.opacity = 1.0;
                self.faded_in = true;
                self.faded_out = false;
            }

            self.next_fade_time = current_time + FADE_DELAY;
        } else {
            if !self.faded_out && current_time <= self.next_fade_time {
                return;
            }

            self.opacity -= FADE_RATE;

            if self.opacity < 0.0 {
                self.opacity = 0.0;
                self.faded_out = true;
                self.faded_in = false;
            }

            self.next_fade_time = current_time + FADE_DELAY;
        }
    }

    /// 检查鼠标是否悬停在对话框上
    fn is_mouse_over(&self, mouse_position: (i32, i32)) -> bool {
        mouse_position.0 >= self.position.0 &&
        mouse_position.0 <= self.position.0 + self.size.0 &&
        mouse_position.1 >= self.position.1 &&
        mouse_position.1 <= self.position.1 + self.size.1
    }

    /// 更新窗口布局
    fn update_window(&mut self) {
        self.buff_count = self.buffs.len() as i32;

        if self.buff_count > 0 {
            if self.buff_count <= 10 {
                // Index = baseImage + _buffCount - 1; (baseImage = 20)
                // 这里我们简化处理，实际应该根据图像资源设置
            } else if self.buff_count > 10 {
                // 处理更多Buff的情况
            }

            // 更新位置和大小
            if self.buff_count <= 10 {
                self.size = ((self.buff_count * 23) as i32, 24);
            } else {
                self.size = (230, 24 + ((self.buff_count / 10) * 24) as i32);
            }

            self.count_label_visible = false;
            self.expand_button_position = (self.size.0 - 15, 0);
        } else {
            self.size = (44, 34);
            self.count_label_visible = true;
            self.count_label_text = self.buff_count.to_string();
            self.count_label_position = (self.size.0 / 2 - 5, self.size.1 / 2 - 10);
            self.expand_button_position = (self.size.0 - 15, 0);
        }
    }

    /// 切换展开/折叠状态
    pub fn toggle_expanded(&mut self, _expanded_buff_window: bool) {
        // 这里应该更新Settings.ExpandedBuffWindow
        // 暂时只是调用update_window
        self.update_window();
    }
}

/// Buff图像信息
#[derive(Debug, Clone, Default)]
pub struct BuffImageInfo {
    /// 位置 (x, y)
    pub location: (i32, i32),
    /// 提示文本
    pub hint: String,
    /// 图像索引
    pub index: i32,
    /// 是否可见
    pub visible: bool,
    /// 透明度
    pub opacity: f32,
}

/// 格式化持续时间显示
fn format_duration(seconds: i32) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{:02}:{:02}", minutes, secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_poison_buff_new() {
        let buff = ClientPoisonBuff::new(PoisonType::Green, "TestCaster".to_string(), 10, 1000, 10000);
        assert_eq!(buff.poison_type, PoisonType::Green);
        assert_eq!(buff.caster, "TestCaster");
        assert_eq!(buff.value, 10);
        assert_eq!(buff.tick_speed, 1000);
        assert_eq!(buff.expire_time, 10000);
    }

    #[test]
    fn test_poison_buff_dialog_new() {
        let dialog = PoisonBuffDialog::new();
        assert!(!dialog.visible);
        assert_eq!(dialog.opacity, 0.0);
        assert!(dialog.buffs.is_empty());
    }

    #[test]
    fn test_add_buff() {
        let mut dialog = PoisonBuffDialog::new();
        let buff = ClientPoisonBuff::new(PoisonType::Green, "Test".to_string(), 5, 1000, 10000);

        dialog.add_buff(buff);
        assert_eq!(dialog.buffs.len(), 1);
        assert_eq!(dialog.buff_count, 1);
    }

    #[test]
    fn test_get_buff_image_index() {
        let dialog = PoisonBuffDialog::new();

        assert_eq!(dialog.get_buff_image_index(PoisonType::Green), 221);
        assert_eq!(dialog.get_buff_image_index(PoisonType::Red), 222);
        assert_eq!(dialog.get_buff_image_index(PoisonType::Slow), 225);
        assert_eq!(dialog.get_buff_image_index(PoisonType::Frozen), 223);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(5), "00:05");
        assert_eq!(format_duration(65), "01:05");
        assert_eq!(format_duration(3665), "01:01:05");
    }
}