//! Item Renting Dialog
//!
//! Dialog for renting items from other players.
//! Corresponds to Client/MirScenes/Dialogs/ItemRentingDialog.cs

use mir2_shared::data::item::UserItem;
use mir2_shared::packets::client::item::{
    CancelItemRental, ConfirmItemRental, ItemRentalLockItem, ItemRentalPeriod,
};

/// Item renting dialog for renting items from other players
#[derive(Debug, Clone)]
pub struct ItemRentingDialog {
    pub visible: bool,
    pub location: (i32, i32),
    pub rental_period_days: u32,
    pub locked: bool,
    pub confirm_enabled: bool,
}

impl Default for ItemRentingDialog {
    fn default() -> Self {
        Self {
            visible: false,
            location: (400, 250), // Slightly below center on 800x600 screen
            rental_period_days: 0,
            locked: false,
            confirm_enabled: false,
        }
    }
}

impl ItemRentingDialog {
    /// Create a new item renting dialog
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable the confirm button
    pub fn enable_confirm_button(&mut self) {
        self.confirm_enabled = true;
    }

    /// Input rental period from user
    pub fn input_rental_period(&mut self, _item_name: &str, _guest_name: &str) -> Option<u32> {
        // TODO: Show input dialog for rental period
        // For now, return a default value for testing
        Some(7) // Default to 7 days
    }

    /// Refresh the dialog interface
    pub fn refresh_interface(&self, user_name: &str) {
        // Update display with current values
        // Display: user_name and self.rental_period_days
    }

    /// Open the item renting dialog
    pub fn open_item_rental_dialog(&mut self, user_name: &str) {
        // TODO: Show inventory dialog
        self.show();
        self.refresh_interface(user_name);
    }

    /// Reset the dialog to initial state
    pub fn reset(&mut self) {
        self.rental_period_days = 0;
        self.confirm_enabled = false;
        self.unlock();
        self.hide();
    }

    /// Lock the rental item (prevent further changes)
    pub fn lock(&mut self) {
        self.locked = true;
        // refresh_interface would be called by caller
    }

    /// Unlock the rental item (allow changes)
    pub fn unlock(&mut self) {
        self.locked = false;
    }

    /// Set the rental period
    pub fn set_rental_period(&mut self, days: u32) {
        if days < 1 || days > 30 {
            return;
        }
        self.rental_period_days = days;
        // TODO: Send ItemRentalPeriod packet
        // network.send(ItemRentalPeriod { days });
        // refresh_interface would be called by caller
    }

    /// Handle lock item button click
    pub fn lock_item(&self) {
        if self.rental_period_days < 1 || self.rental_period_days > 30 {
            return;
        }
        // TODO: Send ItemRentalLockItem packet
        // network.send(ItemRentalLockItem);
    }

    /// Handle confirm button click
    pub fn confirm_rental(&self) {
        // TODO: Send ConfirmItemRental packet
        // network.send(ConfirmItemRental);
    }

    /// Cancel item rental
    pub fn cancel_item_rental(&self) {
        // TODO: Send CancelItemRental packet
        // network.send(CancelItemRental);
    }

    /// Show the dialog
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the dialog
    pub fn hide(&mut self) {
        self.visible = false;
    }
}

/// Guest item renting dialog (shows guest's rental item)
#[derive(Debug, Clone)]
pub struct GuestItemRentingDialog {
    pub visible: bool,
    pub location: (i32, i32),
    pub guest_name: String,
    pub guest_rental_period: u32,
    pub guest_loan_item: Option<UserItem>,
    pub locked: bool,
}

impl Default for GuestItemRentingDialog {
    fn default() -> Self {
        Self {
            visible: false,
            location: (400, 250), // Slightly below center on 800x600 screen
            guest_name: String::new(),
            guest_rental_period: 0,
            guest_loan_item: None,
            locked: false,
        }
    }
}

impl GuestItemRentingDialog {
    /// Create a new guest item renting dialog
    pub fn new() -> Self {
        Self::default()
    }

    /// Refresh the dialog interface
    pub fn refresh_interface(&self) {
        // Update display with current values
        // TODO: Bind guest loan item if it exists
    }

    /// Reset the dialog
    pub fn reset(&mut self) {
        self.unlock();
        self.guest_name.clear();
        self.guest_loan_item = None;
        self.hide();
    }

    /// Set the guest name
    pub fn set_guest_name(&mut self, name: &str) {
        self.guest_name = name.to_string();
    }

    /// Lock the dialog
    pub fn lock(&mut self) {
        self.locked = true;
        self.refresh_interface();
    }

    /// Unlock the dialog
    pub fn unlock(&mut self) {
        self.locked = false;
    }

    /// Show the dialog
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the dialog
    pub fn hide(&mut self) {
        self.visible = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_renting_dialog_creation() {
        let dialog = ItemRentingDialog::new();
        assert!(!dialog.visible);
        assert_eq!(dialog.location, (400, 250));
        assert_eq!(dialog.rental_period_days, 0);
        assert!(!dialog.locked);
        assert!(!dialog.confirm_enabled);
    }

    #[test]
    fn test_item_renting_dialog_operations() {
        let mut dialog = ItemRentingDialog::new();

        dialog.set_rental_period(7);
        assert_eq!(dialog.rental_period_days, 7);

        dialog.enable_confirm_button();
        assert!(dialog.confirm_enabled);

        dialog.lock();
        assert!(dialog.locked);

        dialog.unlock();
        assert!(!dialog.locked);
    }

    #[test]
    fn test_item_renting_dialog_reset() {
        let mut dialog = ItemRentingDialog::new();
        dialog.rental_period_days = 7;
        dialog.locked = true;
        dialog.confirm_enabled = true;
        dialog.visible = true;

        dialog.reset();
        assert_eq!(dialog.rental_period_days, 0);
        assert!(!dialog.locked);
        assert!(!dialog.confirm_enabled);
        assert!(!dialog.visible);
    }

    #[test]
    fn test_rental_period_validation() {
        let mut dialog = ItemRentingDialog::new();

        // Valid period
        dialog.set_rental_period(15);
        assert_eq!(dialog.rental_period_days, 15);

        // Invalid periods should be ignored
        dialog.set_rental_period(0);
        assert_eq!(dialog.rental_period_days, 15); // Should remain unchanged

        dialog.set_rental_period(31);
        assert_eq!(dialog.rental_period_days, 15); // Should remain unchanged
    }

    #[test]
    fn test_guest_item_renting_dialog_creation() {
        let dialog = GuestItemRentingDialog::new();
        assert!(!dialog.visible);
        assert_eq!(dialog.location, (400, 250));
        assert!(dialog.guest_name.is_empty());
        assert_eq!(dialog.guest_rental_period, 0);
        assert!(dialog.guest_loan_item.is_none());
        assert!(!dialog.locked);
    }

    #[test]
    fn test_guest_item_renting_dialog_operations() {
        let mut dialog = GuestItemRentingDialog::new();

        dialog.set_guest_name("GuestUser");
        assert_eq!(dialog.guest_name, "GuestUser");

        dialog.lock();
        assert!(dialog.locked);

        dialog.unlock();
        assert!(!dialog.locked);
    }

    #[test]
    fn test_guest_item_renting_dialog_reset() {
        let mut dialog = GuestItemRentingDialog::new();
        dialog.guest_name = "GuestUser".to_string();
        dialog.locked = true;
        dialog.visible = true;

        dialog.reset();
        assert!(dialog.guest_name.is_empty());
        assert!(!dialog.locked);
        assert!(!dialog.visible);
    }
}