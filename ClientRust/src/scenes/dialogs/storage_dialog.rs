// Storage Dialog - 仓库对话框// Storage Dialog - 仓库对话框// Storage Dialog - 仓库对话框// Storage Dialog - 仓库对话框

// 显示玩家的仓库物品 (10x16网格布局，总共160个槽位)

// 显示玩家的仓库物品 (10x16网格布局，总共160个槽位)

use super::Dialog;

use crate::network::protocol::UserItem;// 显示玩家的仓库物品 (10x16网格布局，总共160个槽位)// 显示玩家的仓库物品 (10x16网格布局，总共160个槽位)



/// 仓库类型use super::Dialog;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]

pub enum StorageType {use crate::network::protocol::UserItem;

    Storage1,  // 仓库1 (基础存储)

    Storage2,  // 仓库2 (扩展存储，需要租赁)

}

/// 仓库类型use super::Dialog;use super::Dialog;

/// 仓库对话框

pub struct StorageDialog {#[derive(Debug, Clone, Copy, PartialEq, Eq)]

    visible: bool,

    x: i32,pub enum StorageType {use crate::network::protocol::UserItem;use crate::network::protocol::UserItem;

    y: i32,

    width: i32,    Storage1,  // 仓库1 (基础存储)

    height: i32,

    current_storage: StorageType,    Storage2,  // 仓库2 (扩展存储，需要租赁)



    // 仓库网格 (10x16 = 160个槽位)}

    pub grid: Vec<Option<UserItem>>,

/// 仓库类型/// 仓库类型

    // UI状态

    pub storage1_selected: bool,  // Storage1按钮选中状态/// 仓库对话框

    pub storage2_selected: bool,  // Storage2按钮选中状态

    pub rent_button_visible: bool, // 租赁按钮可见性pub struct StorageDialog {#[derive(Debug, Clone, Copy, PartialEq, Eq)]#[derive(Debug, Clone, Copy, PartialEq, Eq)]

    pub protect_button_visible: bool, // 保护按钮可见性

    pub locked_page_visible: bool, // 锁定页面可见性    visible: bool,



    // 扩展仓库状态    x: i32,pub enum StorageType {pub enum StorageType {

    pub has_expanded_storage: bool, // 是否已租赁扩展仓库

    pub rental_expiry: Option<i64>, // 租赁到期时间(Unix时间戳)    y: i32,

    pub rental_label_text: String, // 租赁状态文本

    pub rental_label_color: (u8, u8, u8), // 租赁标签颜色 (RGB)    width: i32,    Storage1,  // 仓库1 (基础存储)    Storage1,  // 仓库1 (基础存储)



    // 选中的槽位    height: i32,

    pub selected_slot: Option<usize>,

    pub selected_item: Option<UserItem>,    current_storage: StorageType,    Storage2,  // 仓库2 (扩展存储，需要租赁)    Storage2,  // 仓库2 (扩展存储，需要租赁)



    // 保护模式

    pub protect_mode: bool, // 是否开启保护模式

}    // 仓库网格 (10x16 = 160个槽位)}}



impl StorageDialog {    pub grid: Vec<Option<UserItem>>,

    /// 创建新的仓库对话框

    pub fn new() -> Self {

        Self {

            visible: false,    // UI状态

            x: 100,

            y: 100,    pub storage1_selected: bool,  // Storage1按钮选中状态/// 仓库对话框/// 仓库对话框

            width: 450,

            height: 550,    pub storage2_selected: bool,  // Storage2按钮选中状态

            current_storage: StorageType::Storage1,

            grid: vec![None; 160], // 10x16网格    pub rent_button_visible: bool, // 租赁按钮可见性pub struct StorageDialog {pub struct StorageDialog {

            storage1_selected: true,

            storage2_selected: false,    pub protect_button_visible: bool, // 保护按钮可见性

            rent_button_visible: false,

            protect_button_visible: true,    pub locked_page_visible: bool, // 锁定页面可见性    visible: bool,    visible: bool,

            locked_page_visible: false,

            has_expanded_storage: false,

            rental_expiry: None,

            rental_label_text: "扩展仓库已锁定".to_string(),    // 扩展仓库状态    x: i32,    x: i32,

            rental_label_color: (255, 0, 0), // 红色

            selected_slot: None,    pub has_expanded_storage: bool, // 是否已租赁扩展仓库

            selected_item: None,

            protect_mode: false,    pub rental_expiry: Option<i64>, // 租赁到期时间(Unix时间戳)    y: i32,    y: i32,

        }

    }    pub rental_label_text: String, // 租赁状态文本



    /// 切换到仓库1    pub rental_label_color: (u8, u8, u8), // 租赁标签颜色 (RGB)    width: i32,    width: i32,

    pub fn refresh_storage1(&mut self) {

        self.current_storage = StorageType::Storage1;

        self.storage1_selected = true;

        self.storage2_selected = false;    // 选中的槽位    height: i32,    height: i32,

        self.rent_button_visible = false;

        self.locked_page_visible = false;    pub selected_slot: Option<usize>,

        // 租赁标签不可见

    }    pub selected_item: Option<UserItem>,    current_storage: StorageType,    current_storage: StorageType,



    /// 切换到仓库2 (扩展存储)

    pub fn refresh_storage2(&mut self) {

        self.current_storage = StorageType::Storage2;    // 保护模式

        self.storage1_selected = false;

        self.storage2_selected = true;    pub protect_mode: bool, // 是否开启保护模式



        if self.has_expanded_storage {}    // 仓库网格 (10x16 = 160个槽位)    // 仓库网格 (10x16 = 160个槽位)

            self.rent_button_visible = true;

            self.locked_page_visible = false;

            self.rental_label_text = format!("扩展仓库到期时间: {}",

                self.rental_expiry.map_or("未知".to_string(), |t| t.to_string()));impl StorageDialog {    pub grid: Vec<Option<UserItem>>,    pub grid: Vec<Option<UserItem>>,

            self.rental_label_color = (255, 255, 255); // 白色

        } else {    /// 创建新的仓库对话框

            self.rental_label_text = "扩展仓库已锁定".to_string();

            self.rental_label_color = (255, 0, 0); // 红色    pub fn new() -> Self {

            self.rent_button_visible = true;

            self.locked_page_visible = true;        Self {

        }

    }            visible: false,    // UI状态    // UI状态



    /// 获取当前仓库类型            x: 100,

    pub fn get_current_storage(&self) -> StorageType {

        self.current_storage            y: 100,    pub storage1_selected: bool,  // Storage1按钮选中状态    pub storage1_selected: bool,  // Storage1按钮选中状态

    }

            width: 450,

    /// 获取网格中的物品 (通过槽位索引)

    pub fn get_item(&self, slot: usize) -> Option<&UserItem> {            height: 550,    pub storage2_selected: bool,  // Storage2按钮选中状态    pub storage2_selected: bool,  // Storage2按钮选中状态

        self.grid.get(slot)?.as_ref()

    }            current_storage: StorageType::Storage1,



    /// 设置网格中的物品            grid: vec![None; 160], // 10x16网格    pub rent_button_visible: bool, // 租赁按钮可见性    pub rent_button_visible: bool, // 租赁按钮可见性

    pub fn set_item(&mut self, slot: usize, item: Option<UserItem>) {

        if slot < self.grid.len() {            storage1_selected: true,

            self.grid[slot] = item;

        }            storage2_selected: false,    pub protect_button_visible: bool, // 保护按钮可见性    pub protect_button_visible: bool, // 保护按钮可见性

    }

            rent_button_visible: false,

    /// 通过物品ID获取单元格

    pub fn get_cell(&self, unique_id: u64) -> Option<usize> {            protect_button_visible: true,    pub locked_page_visible: bool, // 锁定页面可见性    pub locked_page_visible: bool, // 锁定页面可见性

        self.grid.iter().position(|item| {

            item.as_ref().map_or(false, |i| i.unique_id == unique_id)            locked_page_visible: false,

        })

    }            has_expanded_storage: false,



    /// 选中槽位            rental_expiry: None,

    pub fn select_slot(&mut self, slot: usize) {

        if slot < self.grid.len() {            rental_label_text: "扩展仓库已锁定".to_string(),    // 扩展仓库状态    // 扩展仓库状态

            self.selected_slot = Some(slot);

            self.selected_item = self.grid[slot].clone();            rental_label_color: (255, 0, 0), // 红色

        }

    }            selected_slot: None,    pub has_expanded_storage: bool, // 是否已租赁扩展仓库    pub has_expanded_storage: bool, // 是否已租赁扩展仓库



    /// 取消选中            selected_item: None,

    pub fn deselect(&mut self) {

        self.selected_slot = None;            protect_mode: false,    pub rental_expiry: Option<i64>, // 租赁到期时间(Unix时间戳)    pub rental_expiry: Option<i64>, // 租赁到期时间(Unix时间戳)

        self.selected_item = None;

    }        }



    /// 查找空槽位    }    pub rental_label_text: String, // 租赁状态文本    pub rental_label_text: String, // 租赁状态文本

    pub fn find_empty_slot(&self) -> Option<usize> {

        self.grid.iter().position(|slot| slot.is_none())

    }

    /// 切换到仓库1    pub rental_label_color: (u8, u8, u8), // 租赁标签颜色 (RGB)    pub rental_label_color: (u8, u8, u8), // 租赁标签颜色 (RGB)

    /// 检查仓库是否已满

    pub fn is_full(&self) -> bool {    pub fn refresh_storage1(&mut self) {

        self.find_empty_slot().is_none()

    }        self.current_storage = StorageType::Storage1;



    /// 移动物品        self.storage1_selected = true;

    pub fn move_item(&mut self, from: usize, to: usize) -> bool {

        if from < self.grid.len() && to < self.grid.len() {        self.storage2_selected = false;    // 选中的槽位    // 选中的槽位

            // 交换物品

            let temp = self.grid[from].take();        self.rent_button_visible = false;

            self.grid[from] = self.grid[to].take();

            self.grid[to] = temp;        self.locked_page_visible = false;    pub selected_slot: Option<usize>,    pub selected_slot: Option<usize>,

            return true;

        }        // 租赁标签不可见

        false

    }    }    pub selected_item: Option<UserItem>,    pub selected_item: Option<UserItem>,



    /// 存入物品 (从背包到仓库)

    pub fn store_item(&mut self, item: UserItem) -> bool {

        if let Some(empty_slot) = self.find_empty_slot() {    /// 切换到仓库2 (扩展存储)

            self.set_item(empty_slot, Some(item));

            true    pub fn refresh_storage2(&mut self) {

        } else {

            false        self.current_storage = StorageType::Storage2;    // 保护模式    // 保护模式

        }

    }        self.storage1_selected = false;



    /// 取出物品 (从仓库到背包)        self.storage2_selected = true;    pub protect_mode: bool, // 是否开启保护模式    pub protect_mode: bool, // 是否开启保护模式

    pub fn retrieve_item(&mut self, slot: usize) -> Option<UserItem> {

        if slot < self.grid.len() {

            self.grid[slot].take()

        } else {        if self.has_expanded_storage {}}

            None

        }            self.rent_button_visible = true;

    }

            self.locked_page_visible = false;

    /// 清空仓库

    pub fn clear_storage(&mut self) {            self.rental_label_text = format!("扩展仓库到期时间: {}",

        self.grid.iter_mut().for_each(|slot| *slot = None);

    }                self.rental_expiry.map_or("未知".to_string(), |t| t.to_string()));impl StorageDialog {impl StorageDialog {



    /// 启用扩展仓库            self.rental_label_color = (255, 255, 255); // 白色

    pub fn enable_expanded_storage(&mut self, expiry_time: Option<i64>) {

        self.has_expanded_storage = true;        } else {    /// 创建新的仓库对话框    /// 创建新的仓库对话框

        self.rental_expiry = expiry_time;

        self.refresh_storage2();            self.rental_label_text = "扩展仓库已锁定".to_string();

    }

            self.rental_label_color = (255, 0, 0); // 红色    pub fn new() -> Self {    pub fn new() -> Self {

    /// 禁用扩展仓库

    pub fn disable_expanded_storage(&mut self) {            self.rent_button_visible = true;

        self.has_expanded_storage = false;

        self.rental_expiry = None;            self.locked_page_visible = true;        Self {        Self {

        // 清空扩展存储区域 (假设后80个槽位是扩展存储)

        for i in 80..160 {        }

            self.grid[i] = None;

        }    }            visible: false,            visible: false,

        if self.current_storage == StorageType::Storage2 {

            self.refresh_storage1();

        }

    }    /// 获取当前仓库类型            x: 100,            x: 100,



    /// 检查扩展仓库是否已过期    pub fn get_current_storage(&self) -> StorageType {

    pub fn is_expanded_storage_expired(&self, current_time: i64) -> bool {

        if !self.has_expanded_storage {        self.current_storage            y: 100,            y: 100,

            return false;

        }    }

        if let Some(expiry) = self.rental_expiry {

            current_time > expiry            width: 450,            width: 450,

        } else {

            false    /// 获取网格中的物品 (通过槽位索引)

        }

    }    pub fn get_item(&self, slot: usize) -> Option<&UserItem> {            height: 550,            height: 550,



    /// 获取扩展仓库剩余时间(秒)        self.grid.get(slot)?.as_ref()

    pub fn get_rental_time_remaining(&self, current_time: i64) -> Option<i64> {

        if let Some(expiry) = self.rental_expiry {    }            current_storage: StorageType::Storage1,            current_storage: StorageType::Storage1,

            Some((expiry - current_time).max(0))

        } else {

            None

        }    /// 设置网格中的物品            grid: vec![None; 160], // 10x16网格            grid: vec![None; 160], // 10x16网格

    }

    pub fn set_item(&mut self, slot: usize, item: Option<UserItem>) {

    /// 切换保护模式

    pub fn toggle_protect_mode(&mut self) {        if slot < self.grid.len() {            storage1_selected: true,            storage1_selected: true,

        self.protect_mode = !self.protect_mode;

    }            self.grid[slot] = item;



    /// 统计仓库中的物品数量        }            storage2_selected: false,            storage2_selected: false,

    pub fn count_items(&self) -> usize {

        self.grid.iter().filter(|slot| slot.is_some()).count()    }

    }

            rent_button_visible: false,            rent_button_visible: false,

    /// 统计总槽位数量

    pub fn total_slots(&self) -> usize {    /// 通过物品ID获取单元格

        self.grid.len()

    }    pub fn get_cell(&self, unique_id: u64) -> Option<usize> {            protect_button_visible: true,            protect_button_visible: true,



    /// 统计空槽位数量        self.grid.iter().position(|item| {

    pub fn count_empty_slots(&self) -> usize {

        self.total_slots() - self.count_items()            item.as_ref().map_or(false, |i| i.unique_id == unique_id)            locked_page_visible: false,            locked_page_visible: false,

    }

        })

    /// 获取网格位置 (将槽位索引转换为x,y坐标)

    pub fn get_grid_position(&self, slot: usize) -> Option<(i32, i32)> {    }            has_expanded_storage: false,            has_expanded_storage: false,

        if slot >= self.grid.len() {

            return None;

        }

        let x = (slot % 10) as i32;    /// 选中槽位            rental_expiry: None,            rental_expiry: None,

        let y = (slot / 10) as i32;

        Some((x, y))    pub fn select_slot(&mut self, slot: usize) {

    }

        if slot < self.grid.len() {            rental_label_text: "扩展仓库已锁定".to_string(),            rental_label_text: "扩展仓库已锁定".to_string(),

    /// 获取槽位索引 (将x,y坐标转换为槽位索引)

    pub fn get_slot_index(&self, x: i32, y: i32) -> Option<usize> {            self.selected_slot = Some(slot);

        if x >= 0 && x < 10 && y >= 0 && y < 16 {

            Some((y * 10 + x) as usize)            self.selected_item = self.grid[slot].clone();            rental_label_color: (255, 0, 0), // 红色            rental_label_color: (255, 0, 0), // 红色

        } else {

            None        }

        }

    }    }            selected_slot: None,            selected_slot: None,

}



impl Default for StorageDialog {

    fn default() -> Self {    /// 取消选中            selected_item: None,            selected_item: None,

        Self::new()

    }    pub fn deselect(&mut self) {

}

        self.selected_slot = None;            protect_mode: false,            protect_mode: false,

impl Dialog for StorageDialog {

    fn show(&mut self) {        self.selected_item = None;

        self.visible = true;

        // 默认显示Storage1    }        }        }

        self.refresh_storage1();

    }



    fn hide(&mut self) {    /// 查找空槽位    }    }

        self.visible = false;

        self.deselect();    pub fn find_empty_slot(&self) -> Option<usize> {

    }

        self.grid.iter().position(|slot| slot.is_none())

    fn update(&mut self, _delta_time: f32) {

        // 更新逻辑 (如检查租赁到期等)    }

    }

    /// 切换到仓库1impl StorageDialog {

    fn draw(&self) {

        if !self.visible {    /// 检查仓库是否已满

            return;

        }    pub fn is_full(&self) -> bool {    pub fn refresh_storage1(&mut self) {    /// 切换到仓库1

        // TODO: 实际渲染逻辑

        // 绘制仓库对话框背景、物品格子、标签页按钮等        self.find_empty_slot().is_none()

    }

    }        self.current_storage = StorageType::Storage1;    pub fn refresh_storage1(&mut self) {

    fn is_visible(&self) -> bool {

        self.visible

    }

    /// 移动物品        self.storage1_selected = true;        self.current_storage = StorageType::Storage1;

    fn name(&self) -> &str {

        "StorageDialog"    pub fn move_item(&mut self, from: usize, to: usize) -> bool {

    }

        if from < self.grid.len() && to < self.grid.len() {        self.storage2_selected = false;        self.storage1_selected = true;

    fn contains_point(&self, x: i32, y: i32) -> bool {

        x >= self.x && x < self.x + self.width &&            // 交换物品

        y >= self.y && y < self.y + self.height

    }            let temp = self.grid[from].take();        self.rent_button_visible = false;        self.storage2_selected = false;



    fn position(&self) -> (i32, i32) {            self.grid[from] = self.grid[to].take();

        (self.x, self.y)

    }            self.grid[to] = temp;        self.locked_page_visible = false;        self.rent_button_visible = false;



    fn size(&self) -> (i32, i32) {            return true;

        (self.width, self.height)

    }        }        // 租赁标签不可见        self.locked_page_visible = false;

}

        false

#[cfg(test)]

mod tests {    }    }        // 租赁标签不可见

    use super::*;



    #[test]

    fn test_storage_dialog_creation() {    /// 存入物品 (从背包到仓库)    }

        let dialog = StorageDialog::new();

        assert!(!dialog.is_visible());    pub fn store_item(&mut self, item: UserItem) -> bool {

        assert_eq!(dialog.grid.len(), 160);

        assert!(!dialog.has_expanded_storage);        if let Some(empty_slot) = self.find_empty_slot() {    /// 切换到仓库2 (扩展存储)

    }

            self.set_item(empty_slot, Some(item));

    #[test]

    fn test_storage_switch() {            true    pub fn refresh_storage2(&mut self) {    /// 切换到仓库2 (扩展存储)

        let mut dialog = StorageDialog::new();

        assert_eq!(dialog.get_current_storage(), StorageType::Storage1);        } else {



        dialog.refresh_storage2();            false        self.current_storage = StorageType::Storage2;    pub fn refresh_storage2(&mut self) {

        assert_eq!(dialog.get_current_storage(), StorageType::Storage2);

    }        }



    #[test]    }        self.storage1_selected = false;        self.current_storage = StorageType::Storage2;

    fn test_storage_set_get_item() {

        let mut dialog = StorageDialog::new();



        let item = UserItem {    /// 取出物品 (从仓库到背包)        self.storage2_selected = true;        self.storage1_selected = false;

            unique_id: 1001,

            item_index: 42,    pub fn retrieve_item(&mut self, slot: usize) -> Option<UserItem> {

            current_dura: 1000,

            max_dura: 1000,        if slot < self.grid.len() {        self.storage2_selected = true;

            count: 1,

            ..Default::default()            self.grid[slot].take()

        };

        } else {        if self.has_expanded_storage {

        dialog.set_item(0, Some(item.clone()));

            None

        let stored = dialog.get_item(0);

        assert!(stored.is_some());        }            self.rent_button_visible = true;        if self.has_expanded_storage {

        assert_eq!(stored.unwrap().unique_id, 1001);

    }    }



    #[test]            self.locked_page_visible = false;            self.rent_button_visible = true;

    fn test_storage_find_empty_slot() {

        let mut dialog = StorageDialog::new();    /// 清空仓库



        let item = UserItem::default();    pub fn clear_storage(&mut self) {            self.rental_label_text = format!("扩展仓库到期时间: {}",            self.locked_page_visible = false;

        for i in 0..5 {

            dialog.set_item(i, Some(item.clone()));        self.grid.iter_mut().for_each(|slot| *slot = None);

        }

    }                self.rental_expiry.map_or("未知".to_string(), |t| t.to_string()));            self.rental_label_text = format!("扩展仓库到期时间: {}",

        let empty = dialog.find_empty_slot();

        assert_eq!(empty, Some(5));

    }

    /// 启用扩展仓库            self.rental_label_color = (255, 255, 255); // 白色                self.rental_expiry.map_or("未知".to_string(), |t| t.to_string()));

    #[test]

    fn test_storage_is_full() {    pub fn enable_expanded_storage(&mut self, expiry_time: Option<i64>) {

        let mut dialog = StorageDialog::new();

        assert!(!dialog.is_full());        self.has_expanded_storage = true;        } else {            self.rental_label_color = (255, 255, 255); // 白色



        let item = UserItem::default();        self.rental_expiry = expiry_time;

        for i in 0..160 {

            dialog.set_item(i, Some(item.clone()));        self.refresh_storage2();            self.rental_label_text = "扩展仓库已锁定".to_string();        } else {

        }

    }

        assert!(dialog.is_full());

    }            self.rental_label_color = (255, 0, 0); // 红色            self.rental_label_text = "扩展仓库已锁定".to_string();



    #[test]    /// 禁用扩展仓库

    fn test_storage_store_retrieve() {

        let mut dialog = StorageDialog::new();    pub fn disable_expanded_storage(&mut self) {            self.rent_button_visible = true;            self.rental_label_color = (255, 0, 0); // 红色



        let item = UserItem {        self.has_expanded_storage = false;

            unique_id: 2001,

            item_index: 55,        self.rental_expiry = None;            self.locked_page_visible = true;            self.rent_button_visible = true;

            current_dura: 500,

            max_dura: 1000,        // 清空扩展存储区域 (假设后80个槽位是扩展存储)

            count: 10,

            ..Default::default()        for i in 80..160 {        }            self.locked_page_visible = true;

        };

            self.grid[i] = None;

        // 存入物品

        let success = dialog.store_item(item.clone());        }    }        }

        assert!(success);

        assert_eq!(dialog.count_items(), 1);        if self.current_storage == StorageType::Storage2 {



        // 取出物品            self.refresh_storage1();    }

        let retrieved = dialog.retrieve_item(0);

        assert!(retrieved.is_some());        }

        assert_eq!(retrieved.unwrap().unique_id, 2001);

        assert_eq!(dialog.count_items(), 0);    }    /// 获取当前仓库类型

    }



    #[test]

    fn test_storage_move_item() {    /// 检查扩展仓库是否已过期    pub fn get_current_storage(&self) -> StorageType {    /// 获取当前仓库类型

        let mut dialog = StorageDialog::new();

    pub fn is_expanded_storage_expired(&self, current_time: i64) -> bool {

        let item1 = UserItem { unique_id: 1001, ..Default::default() };

        let item2 = UserItem { unique_id: 2002, ..Default::default() };        if !self.has_expanded_storage {        self.current_storage    pub fn get_current_storage(&self) -> StorageType {



        dialog.set_item(0, Some(item1));            return false;

        dialog.set_item(5, Some(item2));

        }    }        self.current_storage

        // 移动物品

        dialog.move_item(0, 10);        if let Some(expiry) = self.rental_expiry {



        assert!(dialog.get_item(0).is_none());            current_time > expiry    }

        assert!(dialog.get_item(10).is_some());

        assert_eq!(dialog.get_item(10).unwrap().unique_id, 1001);        } else {

    }

            false    /// 获取网格中的物品 (通过槽位索引)

    #[test]

    fn test_storage_select_slot() {        }

        let mut dialog = StorageDialog::new();

    }    pub fn get_item(&self, slot: usize) -> Option<&UserItem> {    /// 获取网格中的物品 (通过槽位索引)

        let item = UserItem { unique_id: 3001, ..Default::default() };

        dialog.set_item(5, Some(item.clone()));



        dialog.select_slot(5);    /// 获取扩展仓库剩余时间(秒)        self.grid.get(slot)?.as_ref()    pub fn get_item(&self, slot: usize) -> Option<&UserItem> {

        assert_eq!(dialog.selected_slot, Some(5));

        assert!(dialog.selected_item.is_some());    pub fn get_rental_time_remaining(&self, current_time: i64) -> Option<i64> {

        assert_eq!(dialog.selected_item.as_ref().unwrap().unique_id, 3001);

        if let Some(expiry) = self.rental_expiry {    }        self.grid.get(slot)?.as_ref()

        dialog.deselect();

        assert!(dialog.selected_slot.is_none());            Some((expiry - current_time).max(0))

        assert!(dialog.selected_item.is_none());

    }        } else {    }



    #[test]            None

    fn test_storage_clear() {

        let mut dialog = StorageDialog::new();        }    /// 设置网格中的物品



        let item = UserItem::default();    }

        for i in 0..10 {

            dialog.set_item(i, Some(item.clone()));    pub fn set_item(&mut self, slot: usize, item: Option<UserItem>) {    /// 设置网格中的物品

        }

    /// 切换保护模式

        assert_eq!(dialog.count_items(), 10);

    pub fn toggle_protect_mode(&mut self) {        if slot < self.grid.len() {    pub fn set_item(&mut self, slot: usize, item: Option<UserItem>) {

        dialog.clear_storage();

        assert_eq!(dialog.count_items(), 0);        self.protect_mode = !self.protect_mode;

    }

    }            self.grid[slot] = item;        if slot < self.grid.len() {

    #[test]

    fn test_expanded_storage() {

        let mut dialog = StorageDialog::new();

    /// 统计仓库中的物品数量        }            self.grid[slot] = item;

        assert!(!dialog.has_expanded_storage);

        assert_eq!(dialog.total_slots(), 160);    pub fn count_items(&self) -> usize {



        // 启用扩展仓库        self.grid.iter().filter(|slot| slot.is_some()).count()    }        }

        dialog.enable_expanded_storage(Some(1000000));

        assert!(dialog.has_expanded_storage);    }



        // 禁用扩展仓库    }

        dialog.disable_expanded_storage();

        assert!(!dialog.has_expanded_storage);    /// 统计总槽位数量

    }

    pub fn total_slots(&self) -> usize {    /// 通过物品ID获取单元格

    #[test]

    fn test_rental_expiry() {        self.grid.len()

        let mut dialog = StorageDialog::new();

    }    pub fn get_cell(&self, unique_id: u64) -> Option<usize> {    /// 通过物品ID获取单元格

        let expiry_time = 1000000;

        dialog.enable_expanded_storage(Some(expiry_time));



        // 未过期    /// 统计空槽位数量        self.grid.iter().position(|item| {    pub fn get_cell(&self, unique_id: u64) -> Option<usize> {

        assert!(!dialog.is_expanded_storage_expired(999000));

    pub fn count_empty_slots(&self) -> usize {

        // 已过期

        assert!(dialog.is_expanded_storage_expired(1000001));        self.total_slots() - self.count_items()            item.as_ref().map_or(false, |i| i.unique_id == unique_id)        self.grid.iter().position(|item| {



        // 剩余时间    }

        let remaining = dialog.get_rental_time_remaining(999500);

        assert_eq!(remaining, Some(500));        })            item.as_ref().map_or(false, |i| i.unique_id == unique_id)

    }

    /// 获取网格位置 (将槽位索引转换为x,y坐标)

    #[test]

    fn test_storage_counting() {    pub fn get_grid_position(&self, slot: usize) -> Option<(i32, i32)> {    }        })

        let mut dialog = StorageDialog::new();

        if slot >= self.grid.len() {

        let item = UserItem::default();

            return None;    }

        // 添加一些物品

        for i in 0..8 {        }

            dialog.grid[i] = Some(item.clone());

        }        let x = (slot % 10) as i32;    /// 选中槽位



        assert_eq!(dialog.count_items(), 8);        let y = (slot / 10) as i32;

        assert_eq!(dialog.count_empty_slots(), 152); // 160 - 8

    }        Some((x, y))    pub fn select_slot(&mut self, slot: usize) {    /// 选中槽位



    #[test]    }

    fn test_protect_mode() {

        let mut dialog = StorageDialog::new();        if slot < self.grid.len() {    pub fn select_slot(&mut self, slot: usize) {

        assert!(!dialog.protect_mode);

    /// 获取槽位索引 (将x,y坐标转换为槽位索引)

        dialog.toggle_protect_mode();

        assert!(dialog.protect_mode);    pub fn get_slot_index(&self, x: i32, y: i32) -> Option<usize> {            self.selected_slot = Some(slot);        if slot < self.grid.len() {



        dialog.toggle_protect_mode();        if x >= 0 && x < 10 && y >= 0 && y < 16 {

        assert!(!dialog.protect_mode);

    }            Some((y * 10 + x) as usize)            self.selected_item = self.grid[slot].clone();            self.selected_slot = Some(slot);



    #[test]        } else {

    fn test_grid_position_conversion() {

        let dialog = StorageDialog::new();            None        }            self.selected_item = self.grid[slot].clone();



        // 测试位置转换        }

        assert_eq!(dialog.get_grid_position(0), Some((0, 0)));

        assert_eq!(dialog.get_grid_position(9), Some((9, 0)));    }    }        }

        assert_eq!(dialog.get_grid_position(10), Some((0, 1)));

        assert_eq!(dialog.get_grid_position(159), Some((9, 15)));}

        assert_eq!(dialog.get_grid_position(160), None);

    }

        // 测试索引转换

        assert_eq!(dialog.get_slot_index(0, 0), Some(0));impl Default for StorageDialog {

        assert_eq!(dialog.get_slot_index(9, 0), Some(9));

        assert_eq!(dialog.get_slot_index(0, 1), Some(10));    fn default() -> Self {    /// 取消选中

        assert_eq!(dialog.get_slot_index(9, 15), Some(159));

        assert_eq!(dialog.get_slot_index(10, 0), None);        Self::new()

        assert_eq!(dialog.get_slot_index(0, 16), None);

    }    }    pub fn deselect(&mut self) {    /// 取消选中



    #[test]}

    fn test_get_cell_by_id() {

        let mut dialog = StorageDialog::new();        self.selected_slot = None;    pub fn deselect(&mut self) {



        let item1 = UserItem { unique_id: 1001, ..Default::default() };impl Dialog for StorageDialog {

        let item2 = UserItem { unique_id: 2002, ..Default::default() };

    fn show(&mut self) {        self.selected_item = None;        self.selected_slot = None;

        dialog.set_item(5, Some(item1));

        dialog.set_item(10, Some(item2));        self.visible = true;



        assert_eq!(dialog.get_cell(1001), Some(5));        // 默认显示Storage1    }        self.selected_item = None;

        assert_eq!(dialog.get_cell(2002), Some(10));

        assert_eq!(dialog.get_cell(3003), None);        self.refresh_storage1();

    }

    }    }

    #[test]

    fn test_refresh_storage_ui() {

        let mut dialog = StorageDialog::new();

    fn hide(&mut self) {    /// 查找空槽位

        // 测试Storage1

        dialog.refresh_storage1();        self.visible = false;

        assert!(dialog.storage1_selected);

        assert!(!dialog.storage2_selected);        self.deselect();    pub fn find_empty_slot(&self) -> Option<usize> {    /// 查找空槽位

        assert!(!dialog.rent_button_visible);

        assert!(!dialog.locked_page_visible);    }



        // 测试Storage2 (未扩展)        self.grid.iter().position(|slot| slot.is_none())    pub fn find_empty_slot(&self) -> Option<usize> {

        dialog.refresh_storage2();

        assert!(!dialog.storage1_selected);    fn update(&mut self, _delta_time: f32) {

        assert!(dialog.storage2_selected);

        assert!(dialog.rent_button_visible);        // 更新逻辑 (如检查租赁到期等)    }        self.grid.iter().position(|slot| slot.is_none())

        assert!(dialog.locked_page_visible);

        assert_eq!(dialog.rental_label_text, "扩展仓库已锁定");    }

        assert_eq!(dialog.rental_label_color, (255, 0, 0));

    }

        // 测试Storage2 (已扩展)

        dialog.enable_expanded_storage(Some(1234567890));    fn draw(&self) {

        dialog.refresh_storage2();

        assert!(!dialog.locked_page_visible);        if !self.visible {    /// 检查仓库是否已满

        assert_eq!(dialog.rental_label_color, (255, 255, 255));

    }            return;

}
        }    pub fn is_full(&self) -> bool {    /// 检查仓库是否已满

        // TODO: 实际渲染逻辑

        // 绘制仓库对话框背景、物品格子、标签页按钮等        self.find_empty_slot().is_none()    pub fn is_full(&self) -> bool {

    }

    }        self.find_empty_slot().is_none()

    fn is_visible(&self) -> bool {

        self.visible    }

    }

    /// 移动物品

    fn name(&self) -> &str {

        "StorageDialog"    pub fn move_item(&mut self, from: usize, to: usize) -> bool {    /// 移动物品

    }

        if from < self.grid.len() && to < self.grid.len() {    pub fn move_item(&mut self, from: usize, to: usize) -> bool {

    fn contains_point(&self, x: i32, y: i32) -> bool {

        x >= self.x && x < self.x + self.width &&            // 交换物品        if from < self.grid.len() && to < self.grid.len() {

        y >= self.y && y < self.y + self.height

    }            let temp = self.grid[from].take();            // 交换物品



    fn position(&self) -> (i32, i32) {            self.grid[from] = self.grid[to].take();            let temp = self.grid[from].take();

        (self.x, self.y)

    }            self.grid[to] = temp;            self.grid[from] = self.grid[to].take();



    fn size(&self) -> (i32, i32) {            return true;            self.grid[to] = temp;

        (self.width, self.height)

    }        }            return true;

}

        false        }

#[cfg(test)]

mod tests {    }        false

    use super::*;

    }

    #[test]

    fn test_storage_dialog_creation() {    /// 存入物品 (从背包到仓库)

        let dialog = StorageDialog::new();

        assert!(!dialog.is_visible());    pub fn store_item(&mut self, item: UserItem) -> bool {    /// 存入物品 (从背包到仓库)

        assert_eq!(dialog.grid.len(), 160);

        assert!(!dialog.has_expanded_storage);        if let Some(empty_slot) = self.find_empty_slot() {    pub fn store_item(&mut self, item: UserItem) -> bool {

    }

            self.set_item(empty_slot, Some(item));        if let Some(empty_slot) = self.find_empty_slot() {

    #[test]

    fn test_storage_switch() {            true            self.set_item(empty_slot, Some(item));

        let mut dialog = StorageDialog::new();

        assert_eq!(dialog.get_current_storage(), StorageType::Storage1);        } else {            true



        dialog.refresh_storage2();            false        } else {

        assert_eq!(dialog.get_current_storage(), StorageType::Storage2);

    }        }            false



    #[test]    }        }

    fn test_storage_set_get_item() {

        let mut dialog = StorageDialog::new();    }



        let item = UserItem {    /// 取出物品 (从仓库到背包)

            unique_id: 1001,

            item_index: 42,    pub fn retrieve_item(&mut self, slot: usize) -> Option<UserItem> {    /// 取出物品 (从仓库到背包)

            current_dura: 1000,

            max_dura: 1000,        if slot < self.grid.len() {    pub fn retrieve_item(&mut self, slot: usize) -> Option<UserItem> {

            count: 1,

            ..Default::default()            self.grid[slot].take()        if slot < self.grid.len() {

        };

        } else {            self.grid[slot].take()

        dialog.set_item(0, Some(item.clone()));

            None        } else {

        let stored = dialog.get_item(0);

        assert!(stored.is_some());        }            None

        assert_eq!(stored.unwrap().unique_id, 1001);

    }    }        }



    #[test]    }

    fn test_storage_find_empty_slot() {

        let mut dialog = StorageDialog::new();    /// 清空仓库



        let item = UserItem::default();    pub fn clear_storage(&mut self) {    /// 清空仓库

        for i in 0..5 {

            dialog.set_item(i, Some(item.clone()));        self.grid.iter_mut().for_each(|slot| *slot = None);    pub fn clear_storage(&mut self) {

        }

    }        self.grid.iter_mut().for_each(|slot| *slot = None);

        let empty = dialog.find_empty_slot();

        assert_eq!(empty, Some(5));    }

    }

    /// 启用扩展仓库

    #[test]

    fn test_storage_is_full() {    pub fn enable_expanded_storage(&mut self, expiry_time: Option<i64>) {    /// 启用扩展仓库

        let mut dialog = StorageDialog::new();

        assert!(!dialog.is_full());        self.has_expanded_storage = true;    pub fn enable_expanded_storage(&mut self, expiry_time: Option<i64>) {



        let item = UserItem::default();        self.rental_expiry = expiry_time;        self.has_expanded_storage = true;

        for i in 0..160 {

            dialog.set_item(i, Some(item.clone()));        self.refresh_storage2();        self.rental_expiry = expiry_time;

        }

    }        self.refresh_storage2();

        assert!(dialog.is_full());

    }    }



    #[test]    /// 禁用扩展仓库

    fn test_storage_store_retrieve() {

        let mut dialog = StorageDialog::new();    pub fn disable_expanded_storage(&mut self) {    /// 禁用扩展仓库



        let item = UserItem {        self.has_expanded_storage = false;    pub fn disable_expanded_storage(&mut self) {

            unique_id: 2001,

            item_index: 55,        self.rental_expiry = None;        self.has_expanded_storage = false;

            current_dura: 500,

            max_dura: 1000,        // 清空扩展存储区域 (假设后80个槽位是扩展存储)        self.rental_expiry = None;

            count: 10,

            ..Default::default()        for i in 80..160 {        // 清空扩展存储区域 (假设后80个槽位是扩展存储)

        };

            self.grid[i] = None;        for i in 80..160 {

        // 存入物品

        let success = dialog.store_item(item.clone());        }            self.grid[i] = None;

        assert!(success);

        assert_eq!(dialog.count_items(), 1);        if self.current_storage == StorageType::Storage2 {        }



        // 取出物品            self.refresh_storage1();        if self.current_storage == StorageType::Storage2 {

        let retrieved = dialog.retrieve_item(0);

        assert!(retrieved.is_some());        }            self.refresh_storage1();

        assert_eq!(retrieved.unwrap().unique_id, 2001);

        assert_eq!(dialog.count_items(), 0);    }        }

    }

    }

    #[test]

    fn test_storage_move_item() {    /// 检查扩展仓库是否已过期

        let mut dialog = StorageDialog::new();

    pub fn is_expanded_storage_expired(&self, current_time: i64) -> bool {    /// 检查扩展仓库是否已过期

        let item1 = UserItem { unique_id: 1001, ..Default::default() };

        let item2 = UserItem { unique_id: 2002, ..Default::default() };        if !self.has_expanded_storage {    pub fn is_expanded_storage_expired(&self, current_time: i64) -> bool {



        dialog.set_item(0, Some(item1));            return false;        if !self.has_expanded_storage {

        dialog.set_item(5, Some(item2));

        }            return false;

        // 移动物品

        dialog.move_item(0, 10);        if let Some(expiry) = self.rental_expiry {        }



        assert!(dialog.get_item(0).is_none());            current_time > expiry        if let Some(expiry) = self.rental_expiry {

        assert!(dialog.get_item(10).is_some());

        assert_eq!(dialog.get_item(10).unwrap().unique_id, 1001);        } else {            current_time > expiry

    }

            false        } else {

    #[test]

    fn test_storage_select_slot() {        }            false

        let mut dialog = StorageDialog::new();

    }        }

        let item = UserItem { unique_id: 3001, ..Default::default() };

        dialog.set_item(5, Some(item.clone()));    }



        dialog.select_slot(5);    /// 获取扩展仓库剩余时间(秒)

        assert_eq!(dialog.selected_slot, Some(5));

        assert!(dialog.selected_item.is_some());    pub fn get_rental_time_remaining(&self, current_time: i64) -> Option<i64> {    /// 获取扩展仓库剩余时间(秒)

        assert_eq!(dialog.selected_item.as_ref().unwrap().unique_id, 3001);

        if let Some(expiry) = self.rental_expiry {    pub fn get_rental_time_remaining(&self, current_time: i64) -> Option<i64> {

        dialog.deselect();

        assert!(dialog.selected_slot.is_none());            Some((expiry - current_time).max(0))        if let Some(expiry) = self.rental_expiry {

        assert!(dialog.selected_item.is_none());

    }        } else {            Some((expiry - current_time).max(0))



    #[test]            None        } else {

    fn test_storage_clear() {

        let mut dialog = StorageDialog::new();        }            None



        let item = UserItem::default();    }        }

        for i in 0..10 {

            dialog.set_item(i, Some(item.clone()));    }

        }

    /// 切换保护模式

        assert_eq!(dialog.count_items(), 10);

    pub fn toggle_protect_mode(&mut self) {    /// 切换保护模式

        dialog.clear_storage();

        assert_eq!(dialog.count_items(), 0);        self.protect_mode = !self.protect_mode;    pub fn toggle_protect_mode(&mut self) {

    }

    }        self.protect_mode = !self.protect_mode;

    #[test]

    fn test_expanded_storage() {    }

        let mut dialog = StorageDialog::new();

    /// 统计仓库中的物品数量

        assert!(!dialog.has_expanded_storage);

        assert_eq!(dialog.total_slots(), 160);    pub fn count_items(&self) -> usize {    /// 统计仓库中的物品数量



        // 启用扩展仓库        self.grid.iter().filter(|slot| slot.is_some()).count()    pub fn count_items(&self) -> usize {

        dialog.enable_expanded_storage(Some(1000000));

        assert!(dialog.has_expanded_storage);    }        self.grid.iter().filter(|slot| slot.is_some()).count()



        // 禁用扩展仓库    }

        dialog.disable_expanded_storage();

        assert!(!dialog.has_expanded_storage);    /// 统计总槽位数量

    }

    pub fn total_slots(&self) -> usize {    /// 统计总槽位数量

    #[test]

    fn test_rental_expiry() {        self.grid.len()    pub fn total_slots(&self) -> usize {

        let mut dialog = StorageDialog::new();

    }        self.grid.len()

        let expiry_time = 1000000;

        dialog.enable_expanded_storage(Some(expiry_time));    }



        // 未过期    /// 统计空槽位数量

        assert!(!dialog.is_expanded_storage_expired(999000));

    pub fn count_empty_slots(&self) -> usize {    /// 统计空槽位数量

        // 已过期

        assert!(dialog.is_expanded_storage_expired(1000001));        self.total_slots() - self.count_items()    pub fn count_empty_slots(&self) -> usize {



        // 剩余时间    }        self.total_slots() - self.count_items()

        let remaining = dialog.get_rental_time_remaining(999500);

        assert_eq!(remaining, Some(500));    }

    }

    /// 获取网格位置 (将槽位索引转换为x,y坐标)

    #[test]

    fn test_storage_counting() {    pub fn get_grid_position(&self, slot: usize) -> Option<(i32, i32)> {    /// 获取网格位置 (将槽位索引转换为x,y坐标)

        let mut dialog = StorageDialog::new();

        if slot >= self.grid.len() {    pub fn get_grid_position(&self, slot: usize) -> Option<(i32, i32)> {

        let item = UserItem::default();

            return None;        if slot >= self.grid.len() {

        // 添加一些物品

        for i in 0..8 {        }            return None;

            dialog.grid[i] = Some(item.clone());

        }        let x = (slot % 10) as i32;        }



        assert_eq!(dialog.count_items(), 8);        let y = (slot / 10) as i32;        let x = (slot % 10) as i32;

        assert_eq!(dialog.count_empty_slots(), 152); // 160 - 8

    }        Some((x, y))        let y = (slot / 10) as i32;



    #[test]    }        Some((x, y))

    fn test_protect_mode() {

        let mut dialog = StorageDialog::new();    }

        assert!(!dialog.protect_mode);

    /// 获取槽位索引 (将x,y坐标转换为槽位索引)

        dialog.toggle_protect_mode();

        assert!(dialog.protect_mode);    pub fn get_slot_index(&self, x: i32, y: i32) -> Option<usize> {    /// 获取槽位索引 (将x,y坐标转换为槽位索引)



        dialog.toggle_protect_mode();        if x >= 0 && x < 10 && y >= 0 && y < 16 {    pub fn get_slot_index(&self, x: i32, y: i32) -> Option<usize> {

        assert!(!dialog.protect_mode);

    }            Some((y * 10 + x) as usize)        if x >= 0 && x < 10 && y >= 0 && y < 16 {



    #[test]        } else {            Some((y * 10 + x) as usize)

    fn test_grid_position_conversion() {

        let dialog = StorageDialog::new();            None        } else {



        // 测试位置转换        }            None

        assert_eq!(dialog.get_grid_position(0), Some((0, 0)));

        assert_eq!(dialog.get_grid_position(9), Some((9, 0)));    }        }

        assert_eq!(dialog.get_grid_position(10), Some((0, 1)));

        assert_eq!(dialog.get_grid_position(159), Some((9, 15)));}    }

        assert_eq!(dialog.get_grid_position(160), None);

}

        // 测试索引转换

        assert_eq!(dialog.get_slot_index(0, 0), Some(0));impl Default for StorageDialog {}

        assert_eq!(dialog.get_slot_index(9, 0), Some(9));

        assert_eq!(dialog.get_slot_index(0, 1), Some(10));    fn default() -> Self {

        assert_eq!(dialog.get_slot_index(9, 15), Some(159));

        assert_eq!(dialog.get_slot_index(10, 0), None);        Self::new()impl Default for StorageDialog {

        assert_eq!(dialog.get_slot_index(0, 16), None);

    }    }    fn default() -> Self {



    #[test]}        Self::new()

    fn test_get_cell_by_id() {

        let mut dialog = StorageDialog::new();    }



        let item1 = UserItem { unique_id: 1001, ..Default::default() };impl Dialog for StorageDialog {}

        let item2 = UserItem { unique_id: 2002, ..Default::default() };

    fn show(&mut self) {

        dialog.set_item(5, Some(item1));

        dialog.set_item(10, Some(item2));        self.visible = true;impl Dialog for StorageDialog {



        assert_eq!(dialog.get_cell(1001), Some(5));        // 默认显示Storage1    fn show(&mut self) {

        assert_eq!(dialog.get_cell(2002), Some(10));

        assert_eq!(dialog.get_cell(3003), None);        self.refresh_storage1();        self.visible = true;

    }

    }        // 默认显示Storage1

    #[test]

    fn test_refresh_storage_ui() {        self.refresh_storage1();

        let mut dialog = StorageDialog::new();

    fn hide(&mut self) {    }

        // 测试Storage1

        dialog.refresh_storage1();        self.visible = false;

        assert!(dialog.storage1_selected);

        assert!(!dialog.storage2_selected);        self.deselect();    fn hide(&mut self) {

        assert!(!dialog.rent_button_visible);

        assert!(!dialog.locked_page_visible);    }        self.visible = false;



        // 测试Storage2 (未扩展)        self.deselect();

        dialog.refresh_storage2();

        assert!(!dialog.storage1_selected);    fn update(&mut self, _delta_time: f32) {    }

        assert!(dialog.storage2_selected);

        assert!(dialog.rent_button_visible);        // 更新逻辑 (如检查租赁到期等)

        assert!(dialog.locked_page_visible);

        assert_eq!(dialog.rental_label_text, "扩展仓库已锁定");    }    fn update(&mut self, _delta_time: f32) {

        assert_eq!(dialog.rental_label_color, (255, 0, 0));

        // 更新逻辑 (如检查租赁到期等)

        // 测试Storage2 (已扩展)

        dialog.enable_expanded_storage(Some(1234567890));    fn draw(&self) {    }

        dialog.refresh_storage2();

        assert!(!dialog.locked_page_visible);        if !self.visible {

        assert_eq!(dialog.rental_label_color, (255, 255, 255));

    }            return;    fn draw(&self) {

}
        }        if !self.visible {

        // TODO: 实际渲染逻辑            return;

        // 绘制仓库对话框背景、物品格子、标签页按钮等        }

    }        // TODO: 实际渲染逻辑

        // 绘制仓库对话框背景、物品格子、标签页按钮等

    fn is_visible(&self) -> bool {    }

        self.visible

    }    fn is_visible(&self) -> bool {

        self.visible

    fn name(&self) -> &str {    }

        "StorageDialog"    

    }    fn name(&self) -> &str {

        "StorageDialog"

    fn contains_point(&self, x: i32, y: i32) -> bool {    }

        x >= self.x && x < self.x + self.width &&    

        y >= self.y && y < self.y + self.height    fn contains_point(&self, x: i32, y: i32) -> bool {

    }        x >= self.x && x < self.x + self.width &&

        y >= self.y && y < self.y + self.height

    fn position(&self) -> (i32, i32) {    }

        (self.x, self.y)    

    }    fn position(&self) -> (i32, i32) {

        (self.x, self.y)

    fn size(&self) -> (i32, i32) {    }

        (self.width, self.height)    

    }    fn size(&self) -> (i32, i32) {

}        (self.width, self.height)

    }

#[cfg(test)]}

mod tests {

    use super::*;#[cfg(test)]

mod tests {

    #[test]    use super::*;

    fn test_storage_dialog_creation() {

        let dialog = StorageDialog::new();    #[test]

        assert!(!dialog.is_visible());    fn test_storage_dialog_creation() {

        assert_eq!(dialog.grid.len(), 160);        let dialog = StorageDialog::new();

        assert!(!dialog.has_expanded_storage);        assert!(!dialog.is_visible());

    }        assert_eq!(dialog.storage1.len(), 80);

        assert_eq!(dialog.storage2.len(), 80);

    #[test]        assert!(!dialog.has_expanded_storage);

    fn test_storage_switch() {    }

        let mut dialog = StorageDialog::new();

        assert_eq!(dialog.get_current_storage(), StorageType::Storage1);    #[test]

    fn test_storage_switch() {

        dialog.refresh_storage2();        let mut dialog = StorageDialog::new();

        assert_eq!(dialog.get_current_storage(), StorageType::Storage2);        assert_eq!(dialog.get_current_storage(), StorageType::Storage1);

    }        

        dialog.switch_storage(StorageType::Storage2);

    #[test]        assert_eq!(dialog.get_current_storage(), StorageType::Storage2);

    fn test_storage_set_get_item() {    }

        let mut dialog = StorageDialog::new();

    #[test]

        let item = UserItem {    fn test_storage_set_get_item() {

            unique_id: 1001,        let mut dialog = StorageDialog::new();

            item_index: 42,        

            current_dura: 1000,        let item = UserItem {

            max_dura: 1000,            unique_id: 1001,

            count: 1,            item_index: 42,

            ..Default::default()            current_dura: 1000,

        };            max_dura: 1000,

            count: 1,

        dialog.set_item(0, Some(item.clone()));            ..Default::default()

        };

        let stored = dialog.get_item(0);        

        assert!(stored.is_some());        dialog.set_item(0, Some(item.clone()));

        assert_eq!(stored.unwrap().unique_id, 1001);        

    }        let stored = dialog.get_item(0);

        assert!(stored.is_some());

    #[test]        assert_eq!(stored.unwrap().unique_id, 1001);

    fn test_storage_find_empty_slot() {    }

        let mut dialog = StorageDialog::new();

    #[test]

        let item = UserItem::default();    fn test_storage_find_empty_slot() {

        for i in 0..5 {        let mut dialog = StorageDialog::new();

            dialog.set_item(i, Some(item.clone()));        

        }        let item = UserItem::default();

        for i in 0..5 {

        let empty = dialog.find_empty_slot();            dialog.set_item(i, Some(item.clone()));

        assert_eq!(empty, Some(5));        }

    }        

        let empty = dialog.find_empty_slot();

    #[test]        assert_eq!(empty, Some(5));

    fn test_storage_is_full() {    }

        let mut dialog = StorageDialog::new();

        assert!(!dialog.is_full());    #[test]

    fn test_storage_is_full() {

        let item = UserItem::default();        let mut dialog = StorageDialog::new();

        for i in 0..160 {        assert!(!dialog.is_full());

            dialog.set_item(i, Some(item.clone()));        

        }        let item = UserItem::default();

        for i in 0..80 {

        assert!(dialog.is_full());            dialog.set_item(i, Some(item.clone()));

    }        }

        

    #[test]        assert!(dialog.is_full());

    fn test_storage_store_retrieve() {    }

        let mut dialog = StorageDialog::new();

    #[test]

        let item = UserItem {    fn test_storage_store_retrieve() {

            unique_id: 2001,        let mut dialog = StorageDialog::new();

            item_index: 55,        

            current_dura: 500,        let item = UserItem {

            max_dura: 1000,            unique_id: 2001,

            count: 10,            item_index: 55,

            ..Default::default()            current_dura: 500,

        };            max_dura: 1000,

            count: 10,

        // 存入物品            ..Default::default()

        let success = dialog.store_item(item.clone());        };

        assert!(success);        

        assert_eq!(dialog.count_items(), 1);        // 存入物品

        let success = dialog.store_item(item.clone());

        // 取出物品        assert!(success);

        let retrieved = dialog.retrieve_item(0);        assert_eq!(dialog.count_items(), 1);

        assert!(retrieved.is_some());        

        assert_eq!(retrieved.unwrap().unique_id, 2001);        // 取出物品

        assert_eq!(dialog.count_items(), 0);        let retrieved = dialog.retrieve_item(0);

    }        assert!(retrieved.is_some());

        assert_eq!(retrieved.unwrap().unique_id, 2001);

    #[test]        assert_eq!(dialog.count_items(), 0);

    fn test_storage_move_item() {    }

        let mut dialog = StorageDialog::new();

    #[test]

        let item1 = UserItem { unique_id: 1001, ..Default::default() };    fn test_storage_move_item() {

        let item2 = UserItem { unique_id: 2002, ..Default::default() };        let mut dialog = StorageDialog::new();

        

        dialog.set_item(0, Some(item1));        let item1 = UserItem { unique_id: 1001, ..Default::default() };

        dialog.set_item(5, Some(item2));        let item2 = UserItem { unique_id: 2002, ..Default::default() };

        

        // 移动物品        dialog.set_item(0, Some(item1));

        dialog.move_item(0, 10);        dialog.set_item(5, Some(item2));

        

        assert!(dialog.get_item(0).is_none());        // 移动物品

        assert!(dialog.get_item(10).is_some());        dialog.move_item(0, 10);

        assert_eq!(dialog.get_item(10).unwrap().unique_id, 1001);        

    }        assert!(dialog.get_item(0).is_none());

        assert!(dialog.get_item(10).is_some());

    #[test]        assert_eq!(dialog.get_item(10).unwrap().unique_id, 1001);

    fn test_storage_select_slot() {    }

        let mut dialog = StorageDialog::new();

    #[test]

        let item = UserItem { unique_id: 3001, ..Default::default() };    fn test_storage_select_slot() {

        dialog.set_item(5, Some(item.clone()));        let mut dialog = StorageDialog::new();

        

        dialog.select_slot(5);        let item = UserItem { unique_id: 3001, ..Default::default() };

        assert_eq!(dialog.selected_slot, Some(5));        dialog.set_item(5, Some(item.clone()));

        assert!(dialog.selected_item.is_some());        

        assert_eq!(dialog.selected_item.as_ref().unwrap().unique_id, 3001);        dialog.select_slot(5);

        assert_eq!(dialog.selected_slot, Some(5));

        dialog.deselect();        assert!(dialog.selected_item.is_some());

        assert!(dialog.selected_slot.is_none());        assert_eq!(dialog.selected_item.as_ref().unwrap().unique_id, 3001);

        assert!(dialog.selected_item.is_none());        

    }        dialog.deselect();

        assert!(dialog.selected_slot.is_none());

    #[test]        assert!(dialog.selected_item.is_none());

    fn test_storage_clear() {    }

        let mut dialog = StorageDialog::new();

    #[test]

        let item = UserItem::default();    fn test_storage_clear() {

        for i in 0..10 {        let mut dialog = StorageDialog::new();

            dialog.set_item(i, Some(item.clone()));        

        }        let item = UserItem::default();

        for i in 0..10 {

        assert_eq!(dialog.count_items(), 10);            dialog.set_item(i, Some(item.clone()));

        }

        dialog.clear_storage();        

        assert_eq!(dialog.count_items(), 0);        assert_eq!(dialog.count_items(), 10);

    }        

        dialog.clear_storage(StorageType::Storage1);

    #[test]        assert_eq!(dialog.count_items(), 0);

    fn test_expanded_storage() {    }

        let mut dialog = StorageDialog::new();

    #[test]

        assert!(!dialog.has_expanded_storage);    fn test_expanded_storage() {

        assert_eq!(dialog.total_slots(), 160);        let mut dialog = StorageDialog::new();

        

        // 启用扩展仓库        assert!(!dialog.has_expanded_storage);

        dialog.enable_expanded_storage(Some(1000000));        assert_eq!(dialog.total_slots(), 80);

        assert!(dialog.has_expanded_storage);        

        // 启用扩展仓库

        // 禁用扩展仓库        dialog.enable_expanded_storage(Some(1000000));

        dialog.disable_expanded_storage();        assert!(dialog.has_expanded_storage);

        assert!(!dialog.has_expanded_storage);        assert_eq!(dialog.total_slots(), 160);

    }        

        // 禁用扩展仓库

    #[test]        dialog.disable_expanded_storage();

    fn test_rental_expiry() {        assert!(!dialog.has_expanded_storage);

        let mut dialog = StorageDialog::new();        assert_eq!(dialog.total_slots(), 80);

    }

        let expiry_time = 1000000;

        dialog.enable_expanded_storage(Some(expiry_time));    #[test]

    fn test_rental_expiry() {

        // 未过期        let mut dialog = StorageDialog::new();

        assert!(!dialog.is_expanded_storage_expired(999000));        

        let expiry_time = 1000000;

        // 已过期        dialog.enable_expanded_storage(Some(expiry_time));

        assert!(dialog.is_expanded_storage_expired(1000001));        

        // 未过期

        // 剩余时间        assert!(!dialog.is_expanded_storage_expired(999000));

        let remaining = dialog.get_rental_time_remaining(999500);        

        assert_eq!(remaining, Some(500));        // 已过期

    }        assert!(dialog.is_expanded_storage_expired(1000001));

        

    #[test]        // 剩余时间

    fn test_storage_counting() {        let remaining = dialog.get_rental_time_remaining(999500);

        let mut dialog = StorageDialog::new();        assert_eq!(remaining, Some(500));

    }

        let item = UserItem::default();

    #[test]

        // 添加一些物品    fn test_storage_counting() {

        for i in 0..8 {        let mut dialog = StorageDialog::new();

            dialog.grid[i] = Some(item.clone());        

        }        let item = UserItem::default();

        

        assert_eq!(dialog.count_items(), 8);        // Storage1: 5个物品

        assert_eq!(dialog.count_empty_slots(), 152); // 160 - 8        for i in 0..5 {

    }            dialog.storage1[i] = Some(item.clone());

        }

    #[test]        

    fn test_protect_mode() {        assert_eq!(dialog.count_items(), 5);

        let mut dialog = StorageDialog::new();        assert_eq!(dialog.count_empty_slots(), 75);

        assert!(!dialog.protect_mode);        

        // 启用扩展仓库

        dialog.toggle_protect_mode();        dialog.enable_expanded_storage(None);

        assert!(dialog.protect_mode);        

        // Storage2: 3个物品

        dialog.toggle_protect_mode();        for i in 0..3 {

        assert!(!dialog.protect_mode);            dialog.storage2[i] = Some(item.clone());

    }        }

        

    #[test]        assert_eq!(dialog.count_items(), 8);

    fn test_grid_position_conversion() {        assert_eq!(dialog.count_empty_slots(), 152); // 160 - 8

        let dialog = StorageDialog::new();    }



        // 测试位置转换    #[test]

        assert_eq!(dialog.get_grid_position(0), Some((0, 0)));    fn test_protect_mode() {

        assert_eq!(dialog.get_grid_position(9), Some((9, 0)));        let mut dialog = StorageDialog::new();

        assert_eq!(dialog.get_grid_position(10), Some((0, 1)));        assert!(!dialog.protect_mode);

        assert_eq!(dialog.get_grid_position(159), Some((9, 15)));        

        assert_eq!(dialog.get_grid_position(160), None);        dialog.toggle_protect_mode();

        assert!(dialog.protect_mode);

        // 测试索引转换        

        assert_eq!(dialog.get_slot_index(0, 0), Some(0));        dialog.toggle_protect_mode();

        assert_eq!(dialog.get_slot_index(9, 0), Some(9));        assert!(!dialog.protect_mode);

        assert_eq!(dialog.get_slot_index(0, 1), Some(10));    }

        assert_eq!(dialog.get_slot_index(9, 15), Some(159));}

        assert_eq!(dialog.get_slot_index(10, 0), None);
        assert_eq!(dialog.get_slot_index(0, 16), None);
    }

    #[test]
    fn test_get_cell_by_id() {
        let mut dialog = StorageDialog::new();

        let item1 = UserItem { unique_id: 1001, ..Default::default() };
        let item2 = UserItem { unique_id: 2002, ..Default::default() };

        dialog.set_item(5, Some(item1));
        dialog.set_item(10, Some(item2));

        assert_eq!(dialog.get_cell(1001), Some(5));
        assert_eq!(dialog.get_cell(2002), Some(10));
        assert_eq!(dialog.get_cell(3003), None);
    }

    #[test]
    fn test_refresh_storage_ui() {
        let mut dialog = StorageDialog::new();

        // 测试Storage1
        dialog.refresh_storage1();
        assert!(dialog.storage1_selected);
        assert!(!dialog.storage2_selected);
        assert!(!dialog.rent_button_visible);
        assert!(!dialog.locked_page_visible);

        // 测试Storage2 (未扩展)
        dialog.refresh_storage2();
        assert!(!dialog.storage1_selected);
        assert!(dialog.storage2_selected);
        assert!(dialog.rent_button_visible);
        assert!(dialog.locked_page_visible);
        assert_eq!(dialog.rental_label_text, "扩展仓库已锁定");
        assert_eq!(dialog.rental_label_color, (255, 0, 0));

        // 测试Storage2 (已扩展)
        dialog.enable_expanded_storage(Some(1234567890));
        dialog.refresh_storage2();
        assert!(!dialog.locked_page_visible);
        assert_eq!(dialog.rental_label_color, (255, 255, 255));
    }
}