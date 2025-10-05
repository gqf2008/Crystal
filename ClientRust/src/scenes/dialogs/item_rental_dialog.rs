//! Item Rental Dialog
//!
//! Displays rented items and allows players to rent out items to other players.
//! Corresponds to Client/MirScenes/Dialogs/ItemRentalDialog.cs

use std::time::{Duration, Instant};

use mir2_shared::data::item::ItemRentalInformation;
use mir2_shared::packets::client::item::{
    GetRentedItems, ItemRentalRequest,
};

/// Item rental dialog showing rented items list
#[derive(Debug, Clone)]
pub struct ItemRentalDialog {
    pub visible: bool,
    pub location: (i32, i32),
    pub item_rows: Vec<ItemRow>,
    last_request_time: Option<Instant>,
}

impl Default for ItemRentalDialog {
    fn default() -> Self {
        Self {
            visible: false,
            location: (400, 200), // Centered on 800x600 screen
            item_rows: vec![ItemRow::default(); 3],
            last_request_time: None,
        }
    }
}

impl ItemRentalDialog {
    /// Create a new item rental dialog
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle dialog visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.request_rented_items();
        }
    }

    /// Update the rented items display
    pub fn receive_rented_items(&mut self, rented_items: Vec<Option<ItemRentalInformation>>) {
        for (i, item_row) in self.item_rows.iter_mut().enumerate() {
            item_row.clear();
            if let Some(Some(item)) = rented_items.get(i) {
                item_row.update(
                    &item.item_name,
                    &item.renting_player_name,
                    &format_date(item.item_return_date_binary),
                );
            }
        }
    }

    /// Request rented items from server (rate limited to once per 60 seconds)
    pub fn request_rented_items(&mut self) {
        let now = Instant::now();
        if let Some(last_time) = self.last_request_time {
            if now.duration_since(last_time) < Duration::from_secs(60) {
                return;
            }
        }

        self.last_request_time = Some(now);
        // TODO: Send GetRentedItems packet to server
        // network.send(GetRentedItems);
    }

    /// Handle rent item button click
    pub fn rent_item(&self) {
        // TODO: Send ItemRentalRequest packet to server
        // network.send(ItemRentalRequest);
    }
}

/// Individual row displaying rental item information
#[derive(Debug, Clone, Default)]
pub struct ItemRow {
    pub visible: bool,
    pub item_name: String,
    pub renting_player_name: String,
    pub return_date: String,
}

impl ItemRow {
    /// Clear the row data
    pub fn clear(&mut self) {
        self.visible = false;
        self.item_name.clear();
        self.renting_player_name.clear();
        self.return_date.clear();
    }

    /// Update the row with rental information
    pub fn update(&mut self, item_name: &str, renting_player_name: &str, return_date: &str) {
        self.item_name = item_name.to_string();
        self.renting_player_name = renting_player_name.to_string();
        self.return_date = return_date.to_string();
        self.visible = true;
    }
}

/// Format binary date to string (simplified for now)
fn format_date(binary_date: i64) -> String {
    // TODO: Implement proper date formatting
    // For now, just return the binary value as string
    binary_date.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_rental_dialog_creation() {
        let dialog = ItemRentalDialog::new();
        assert!(!dialog.visible);
        assert_eq!(dialog.location, (400, 200));
        assert_eq!(dialog.item_rows.len(), 3);
    }

    #[test]
    fn test_toggle_visibility() {
        let mut dialog = ItemRentalDialog::new();
        assert!(!dialog.visible);

        dialog.toggle();
        assert!(dialog.visible);

        dialog.toggle();
        assert!(!dialog.visible);
    }

    #[test]
    fn test_item_row_operations() {
        let mut row = ItemRow::default();
        assert!(!row.visible);

        row.update("Dragon Sword", "PlayerOne", "2025-01-01");
        assert!(row.visible);
        assert_eq!(row.item_name, "Dragon Sword");
        assert_eq!(row.renting_player_name, "PlayerOne");
        assert_eq!(row.return_date, "2025-01-01");

        row.clear();
        assert!(!row.visible);
        assert!(row.item_name.is_empty());
        assert!(row.renting_player_name.is_empty());
        assert!(row.return_date.is_empty());
    }

    #[test]
    fn test_receive_rented_items() {
        let mut dialog = ItemRentalDialog::new();

        let rented_items = vec![
            Some(ItemRentalInformation {
                item_id: 1,
                item_name: "Sword".to_string(),
                renting_player_name: "Alice".to_string(),
                item_return_date_binary: 123456789,
            }),
            None,
            Some(ItemRentalInformation {
                item_id: 2,
                item_name: "Shield".to_string(),
                renting_player_name: "Bob".to_string(),
                item_return_date_binary: 987654321,
            }),
        ];

        dialog.receive_rented_items(rented_items);

        assert!(dialog.item_rows[0].visible);
        assert_eq!(dialog.item_rows[0].item_name, "Sword");
        assert_eq!(dialog.item_rows[0].renting_player_name, "Alice");

        assert!(!dialog.item_rows[1].visible);

        assert!(dialog.item_rows[2].visible);
        assert_eq!(dialog.item_rows[2].item_name, "Shield");
        assert_eq!(dialog.item_rows[2].renting_player_name, "Bob");
    }

    #[test]
    fn test_request_rate_limiting() {
        let mut dialog = ItemRentalDialog::new();

        // First request should work
        dialog.request_rented_items();
        assert!(dialog.last_request_time.is_some());

        // Second request immediately after should be ignored
        let last_time = dialog.last_request_time.unwrap();
        dialog.request_rented_items();
        assert_eq!(dialog.last_request_time.unwrap(), last_time);
    }
}