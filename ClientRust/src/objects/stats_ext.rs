// stats_ext.rs - Stats system extensions for UserObject
// Convenience methods for accessing specific stats

use mir2_shared::{data::stats::Stats, enums::Stat};

/// Extension trait for Stats to provide convenient getter/setter methods
pub trait StatsExt {
    // HP/MP getters
    fn get_max_hp(&self) -> i32;
    fn get_max_mp(&self) -> i32;
    fn get_hp_rate_percent(&self) -> i32;
    fn get_mp_rate_percent(&self) -> i32;
    
    // AC/MAC getters
    fn get_max_ac(&self) -> i32;
    fn get_max_ac_rate_percent(&self) -> i32;
    fn get_max_mac(&self) -> i32;
    fn get_max_mac_rate_percent(&self) -> i32;
    
    // DC/MC/SC getters
    fn get_max_dc(&self) -> i32;
    fn get_max_dc_rate_percent(&self) -> i32;
    fn get_max_mc(&self) -> i32;
    fn get_max_mc_rate_percent(&self) -> i32;
    fn get_max_sc(&self) -> i32;
    fn get_max_sc_rate_percent(&self) -> i32;
    
    // Attack speed
    fn get_attack_speed(&self) -> i32;
    fn get_attack_speed_rate_percent(&self) -> i32;
    
    // Min stats getters
    fn get_min_ac(&self) -> i32;
    fn get_min_mac(&self) -> i32;
    fn get_min_dc(&self) -> i32;
    fn get_min_mc(&self) -> i32;
    fn get_min_sc(&self) -> i32;
    
    // Other stats getters
    fn get_accuracy(&self) -> i32;
    fn get_agility(&self) -> i32;
    fn get_luck(&self) -> i32;
    fn get_holy(&self) -> i32;
    fn get_bag_weight(&self) -> i32;
    fn get_hand_weight(&self) -> i32;
    fn get_wear_weight(&self) -> i32;
    fn get_magic_resist(&self) -> i32;
    fn get_poison_resist(&self) -> i32;
    
    // Adder methods (HP/MP)
    fn add_max_hp(&mut self, value: i32);
    fn add_max_mp(&mut self, value: i32);
    
    // Adder methods (AC/MAC)
    fn add_max_ac(&mut self, value: i32);
    fn add_max_mac(&mut self, value: i32);
    
    // Adder methods (DC/MC/SC)
    fn add_max_dc(&mut self, value: i32);
    fn add_max_mc(&mut self, value: i32);
    fn add_max_sc(&mut self, value: i32);
    
    // Adder methods (Attack speed)
    fn add_attack_speed(&mut self, value: i32);
    
    // Adder methods (Min stats)
    fn add_min_ac(&mut self, value: i32);
    fn add_min_mac(&mut self, value: i32);
    fn add_min_dc(&mut self, value: i32);
    fn add_min_mc(&mut self, value: i32);
    fn add_min_sc(&mut self, value: i32);
    
    // Adder methods (Other stats)
    fn add_accuracy(&mut self, value: i32);
    fn add_agility(&mut self, value: i32);
    fn add_luck(&mut self, value: i32);
    fn add_holy(&mut self, value: i32);
    fn add_bag_weight(&mut self, value: i32);
    fn add_hand_weight(&mut self, value: i32);
    fn add_wear_weight(&mut self, value: i32);
    fn add_magic_resist(&mut self, value: i32);
    fn add_poison_resist(&mut self, value: i32);
}

impl StatsExt for Stats {
    // ==================== HP/MP ====================
    
    fn get_max_hp(&self) -> i32 {
        self.get(Stat::HP)
    }
    
    fn get_max_mp(&self) -> i32 {
        self.get(Stat::MP)
    }
    
    fn get_hp_rate_percent(&self) -> i32 {
        self.get(Stat::HPRatePercent)
    }
    
    fn get_mp_rate_percent(&self) -> i32 {
        self.get(Stat::MPRatePercent)
    }
    
    fn add_max_hp(&mut self, value: i32) {
        let current = self.get(Stat::HP);
        self.set(Stat::HP, current + value);
    }
    
    fn add_max_mp(&mut self, value: i32) {
        let current = self.get(Stat::MP);
        self.set(Stat::MP, current + value);
    }
    
    // ==================== AC/MAC ====================
    
    fn get_max_ac(&self) -> i32 {
        self.get(Stat::MaxAC)
    }
    
    fn get_max_ac_rate_percent(&self) -> i32 {
        self.get(Stat::MaxACRatePercent)
    }
    
    fn get_max_mac(&self) -> i32 {
        self.get(Stat::MaxMAC)
    }
    
    fn get_max_mac_rate_percent(&self) -> i32 {
        self.get(Stat::MaxMACRatePercent)
    }
    
    fn add_max_ac(&mut self, value: i32) {
        let current = self.get(Stat::MaxAC);
        self.set(Stat::MaxAC, current + value);
    }
    
    fn add_max_mac(&mut self, value: i32) {
        let current = self.get(Stat::MaxMAC);
        self.set(Stat::MaxMAC, current + value);
    }
    
    // ==================== DC/MC/SC ====================
    
    fn get_max_dc(&self) -> i32 {
        self.get(Stat::MaxDC)
    }
    
    fn get_max_dc_rate_percent(&self) -> i32 {
        self.get(Stat::MaxDCRatePercent)
    }
    
    fn get_max_mc(&self) -> i32 {
        self.get(Stat::MaxMC)
    }
    
    fn get_max_mc_rate_percent(&self) -> i32 {
        self.get(Stat::MaxMCRatePercent)
    }
    
    fn get_max_sc(&self) -> i32 {
        self.get(Stat::MaxSC)
    }
    
    fn get_max_sc_rate_percent(&self) -> i32 {
        self.get(Stat::MaxSCRatePercent)
    }
    
    fn add_max_dc(&mut self, value: i32) {
        let current = self.get(Stat::MaxDC);
        self.set(Stat::MaxDC, current + value);
    }
    
    fn add_max_mc(&mut self, value: i32) {
        let current = self.get(Stat::MaxMC);
        self.set(Stat::MaxMC, current + value);
    }
    
    fn add_max_sc(&mut self, value: i32) {
        let current = self.get(Stat::MaxSC);
        self.set(Stat::MaxSC, current + value);
    }
    
    // ==================== Attack Speed ====================
    
    fn get_attack_speed(&self) -> i32 {
        self.get(Stat::AttackSpeed)
    }
    
    fn get_attack_speed_rate_percent(&self) -> i32 {
        self.get(Stat::AttackSpeedRatePercent)
    }
    
    fn add_attack_speed(&mut self, value: i32) {
        let current = self.get(Stat::AttackSpeed);
        self.set(Stat::AttackSpeed, current + value);
    }
    
    // ==================== Min Stats ====================
    
    fn get_min_ac(&self) -> i32 {
        self.get(Stat::MinAC)
    }
    
    fn get_min_mac(&self) -> i32 {
        self.get(Stat::MinMAC)
    }
    
    fn get_min_dc(&self) -> i32 {
        self.get(Stat::MinDC)
    }
    
    fn get_min_mc(&self) -> i32 {
        self.get(Stat::MinMC)
    }
    
    fn get_min_sc(&self) -> i32 {
        self.get(Stat::MinSC)
    }
    
    fn add_min_ac(&mut self, value: i32) {
        let current = self.get(Stat::MinAC);
        self.set(Stat::MinAC, current + value);
    }
    
    fn add_min_mac(&mut self, value: i32) {
        let current = self.get(Stat::MinMAC);
        self.set(Stat::MinMAC, current + value);
    }
    
    fn add_min_dc(&mut self, value: i32) {
        let current = self.get(Stat::MinDC);
        self.set(Stat::MinDC, current + value);
    }
    
    fn add_min_mc(&mut self, value: i32) {
        let current = self.get(Stat::MinMC);
        self.set(Stat::MinMC, current + value);
    }
    
    fn add_min_sc(&mut self, value: i32) {
        let current = self.get(Stat::MinSC);
        self.set(Stat::MinSC, current + value);
    }
    
    // ==================== Other Stats ====================
    
    fn get_accuracy(&self) -> i32 {
        self.get(Stat::Accuracy)
    }
    
    fn get_agility(&self) -> i32 {
        self.get(Stat::Agility)
    }
    
    fn get_luck(&self) -> i32 {
        self.get(Stat::Luck)
    }
    
    fn get_holy(&self) -> i32 {
        self.get(Stat::Holy)
    }
    
    fn get_bag_weight(&self) -> i32 {
        self.get(Stat::BagWeight)
    }
    
    fn get_hand_weight(&self) -> i32 {
        self.get(Stat::HandWeight)
    }
    
    fn get_wear_weight(&self) -> i32 {
        self.get(Stat::WearWeight)
    }
    
    fn get_magic_resist(&self) -> i32 {
        self.get(Stat::MagicResist)
    }
    
    fn get_poison_resist(&self) -> i32 {
        self.get(Stat::PoisonResist)
    }
    
    fn add_accuracy(&mut self, value: i32) {
        let current = self.get(Stat::Accuracy);
        self.set(Stat::Accuracy, current + value);
    }
    
    fn add_agility(&mut self, value: i32) {
        let current = self.get(Stat::Agility);
        self.set(Stat::Agility, current + value);
    }
    
    fn add_luck(&mut self, value: i32) {
        let current = self.get(Stat::Luck);
        self.set(Stat::Luck, current + value);
    }
    
    fn add_holy(&mut self, value: i32) {
        let current = self.get(Stat::Holy);
        self.set(Stat::Holy, current + value);
    }
    
    fn add_bag_weight(&mut self, value: i32) {
        let current = self.get(Stat::BagWeight);
        self.set(Stat::BagWeight, current + value);
    }
    
    fn add_hand_weight(&mut self, value: i32) {
        let current = self.get(Stat::HandWeight);
        self.set(Stat::HandWeight, current + value);
    }
    
    fn add_wear_weight(&mut self, value: i32) {
        let current = self.get(Stat::WearWeight);
        self.set(Stat::WearWeight, current + value);
    }
    
    fn add_magic_resist(&mut self, value: i32) {
        let current = self.get(Stat::MagicResist);
        self.set(Stat::MagicResist, current + value);
    }
    
    fn add_poison_resist(&mut self, value: i32) {
        let current = self.get(Stat::PoisonResist);
        self.set(Stat::PoisonResist, current + value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_stats_ext_hp() {
        let mut stats = Stats::new();
        stats.set(Stat::HP, 100);
        
        assert_eq!(stats.get_max_hp(), 100);
        
        stats.add_max_hp(50);
        assert_eq!(stats.get_max_hp(), 150);
    }
    
    #[test]
    fn test_stats_ext_attack_speed() {
        let mut stats = Stats::new();
        stats.set(Stat::AttackSpeed, 10);
        
        assert_eq!(stats.get_attack_speed(), 10);
        
        stats.add_attack_speed(5);
        assert_eq!(stats.get_attack_speed(), 15);
    }
    
    #[test]
    fn test_stats_ext_percentage() {
        let mut stats = Stats::new();
        stats.set(Stat::HP, 1000);
        stats.set(Stat::HPRatePercent, 20);
        
        // Apply percentage bonus
        let bonus = (stats.get_max_hp() * stats.get_hp_rate_percent()) / 100;
        stats.add_max_hp(bonus);
        
        assert_eq!(stats.get_max_hp(), 1200); // 1000 + 20% = 1200
    }
}
