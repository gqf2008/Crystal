//! Item Rent Dialog
//!
//! Dialog for renting out items to other players.
//! Corresponds to Client/MirScenes/Dialogs/ItemRentDialog.cs

use mir2_shared::packets::client::item::{
    CancelItemRental, ItemRentalFee, ItemRentalLockFee,
};

/// Item rent dialog for setting rental fees
#[derive(Debug, Clone)]
pub struct ItemRentDialog {
    pub visible: bool,
    pub location: (i32, i32),
    pub locked: bool,
}

impl Default for ItemRentDialog {
    fn default() -> Self {
        Self {
            visible: false,
            location: (400, 200), // Centered on 800x600 screen
            locked: false,
        }
    }
}

impl ItemRentDialog {
    /// Create a new item rent dialog
    pub fn new() -> Self {
        Self::default()
    }

    /// Refresh the dialog interface
    pub fn refresh_interface(&self, user_name: &str, rental_gold_amount: u32) {
        // Update display with current values
        // This would be called when rental amount changes
        // Display: user_name and rental_gold_amount
    }

    /// Open the item rent dialog
    pub fn open_item_rent_dialog(&mut self, user_name: &str, rental_gold_amount: u32) {
        // TODO: Show inventory dialog
        // TODO: Show guest item renting dialog
        self.show();
        self.refresh_interface(user_name, rental_gold_amount);
    }

    /// Reset the dialog to initial state
    pub fn reset(&mut self) {
        // Reset would be handled by caller updating global state
        self.unlock();
        self.hide();
    }

    /// Lock the rental fee (prevent further changes)
    pub fn lock(&mut self) {
        self.locked = true;
        // refresh_interface would be called by caller
    }

    /// Unlock the rental fee (allow changes)
    pub fn unlock(&mut self) {
        self.locked = false;
    }

    /// Handle rental fee button click - returns the amount to add
    pub fn set_rental_fee(&self, current_amount: u32, amount_to_add: u32) -> u32 {
        if amount_to_add == 0 {
            return current_amount;
        }
        current_amount + amount_to_add
    }

    /// Handle lock fee button click
    pub fn lock_fee(&self, rental_gold_amount: u32) {
        if rental_gold_amount < 1 {
            return;
        }
        // TODO: Send ItemRentalLockFee packet
        // network.send(ItemRentalLockFee);
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

/// Guest item rent dialog (shows guest's rental settings)
#[derive(Debug, Clone)]
pub struct GuestItemRentDialog {
    pub visible: bool,
    pub location: (i32, i32),
    pub guest_name: String,
    pub guest_gold: u32,
    pub guest_gold_locked: bool,
}

impl Default for GuestItemRentDialog {
    fn default() -> Self {
        Self {
            visible: false,
            location: (400, 200), // Centered on 800x600 screen
            guest_name: String::new(),
            guest_gold: 0,
            guest_gold_locked: false,
        }
    }
}

impl GuestItemRentDialog {
    /// Create a new guest item rent dialog
    pub fn new() -> Self {
        Self::default()
    }

    /// Refresh the dialog interface
    pub fn refresh_interface(&mut self) {
        // Update display with current values
    }

    /// Set the guest name
    pub fn set_guest_name(&mut self, name: &str) {
        self.guest_name = name.to_string();
    }

    /// Set the guest rental fee
    pub fn set_guest_fee(&mut self, amount: u32) {
        self.guest_gold = amount;
    }

    /// Reset the dialog
    pub fn reset(&mut self) {
        self.unlock();
        self.guest_name.clear();
        self.guest_gold = 0;
        self.hide();
    }

    /// Lock the guest rental fee
    pub fn lock(&mut self) {
        self.guest_gold_locked = true;
        self.refresh_interface();
    }

    /// Unlock the guest rental fee
    pub fn unlock(&mut self) {
        self.guest_gold_locked = false;
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
    fn test_item_rent_dialog_creation() {
        let dialog = ItemRentDialog::new();
        assert!(!dialog.visible);
        assert_eq!(dialog.location, (400, 200));
        assert!(!dialog.locked);
    }

    #[test]
    fn test_item_rent_dialog_operations() {
        let mut dialog = ItemRentDialog::new();

        let new_amount = dialog.set_rental_fee(0, 1000);
        assert_eq!(new_amount, 1000);

        let new_amount = dialog.set_rental_fee(1000, 500);
        assert_eq!(new_amount, 1500);

        dialog.lock();
        assert!(dialog.locked);

        dialog.unlock();
        assert!(!dialog.locked);
    }

    #[test]
    fn test_item_rent_dialog_reset() {
        let mut dialog = ItemRentDialog::new();
        dialog.locked = true;
        dialog.visible = true;

        dialog.reset();
        assert!(!dialog.locked);
        assert!(!dialog.visible);
    }

    #[test]
    fn test_guest_item_rent_dialog_creation() {
        let dialog = GuestItemRentDialog::new();
        assert!(!dialog.visible);
        assert_eq!(dialog.location, (400, 200));
        assert!(dialog.guest_name.is_empty());
        assert_eq!(dialog.guest_gold, 0);
        assert!(!dialog.guest_gold_locked);
    }

    #[test]
    fn test_guest_item_rent_dialog_operations() {
        let mut dialog = GuestItemRentDialog::new();

        dialog.set_guest_name("GuestUser");
        dialog.set_guest_fee(2000);

        assert_eq!(dialog.guest_name, "GuestUser");
        assert_eq!(dialog.guest_gold, 2000);

        dialog.lock();
        assert!(dialog.guest_gold_locked);

        dialog.unlock();
        assert!(!dialog.guest_gold_locked);
    }

    #[test]
    fn test_guest_item_rent_dialog_reset() {
        let mut dialog = GuestItemRentDialog::new();
        dialog.guest_name = "GuestUser".to_string();
        dialog.guest_gold = 2000;
        dialog.guest_gold_locked = true;
        dialog.visible = true;

        dialog.reset();
        assert!(dialog.guest_name.is_empty());
        assert_eq!(dialog.guest_gold, 0);
        assert!(!dialog.guest_gold_locked);
        assert!(!dialog.visible);
    }
}