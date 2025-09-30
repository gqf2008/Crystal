use std::collections::BTreeMap;
use std::io::{self, Read, Write};
use std::string::FromUtf8Error;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::enums::{MirClass, Stat, StatFormula};

#[derive(Debug, Error)]
pub enum SharedError {
    #[error("unknown stat discriminant {0}")]
    UnknownStat(u8),
    #[error("unknown stat formula discriminant {0}")]
    UnknownStatFormula(u8),
    #[error("unknown class discriminant {0}")]
    UnknownClass(u8),
    #[error("unknown {name} discriminant {value}")]
    UnknownEnum { name: &'static str, value: u32 },
    #[error("missing required field {0}")]
    MissingField(&'static str),
    #[error("packet length {0} shorter than header")]
    InvalidPacketLength(u16),
    #[error("packet body too large ({0} bytes)")]
    PacketTooLarge(usize),
    #[error("opcode mismatch: expected {expected}, got {actual}")]
    OpcodeMismatch { expected: i16, actual: i16 },
    #[error("invalid 7-bit encoded integer")]
    Invalid7BitEncodedInt,
    #[error("string length {length} exceeds maximum supported size")]
    StringTooLong { length: usize },
    #[error("invalid UTF-8 string data")]
    InvalidUtf8(#[from] FromUtf8Error),
    #[error(transparent)]
    Io(#[from] io::Error),
}

pub type SharedResult<T> = Result<T, SharedError>;

impl SharedError {
    pub fn unknown_enum(name: &'static str, value: u32) -> Self {
        SharedError::UnknownEnum { name, value }
    }

    pub fn missing_field(name: &'static str) -> Self {
        SharedError::MissingField(name)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    values: BTreeMap<Stat, i32>,
}

impl Stats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn total_magnitude(&self) -> i32 {
        self.values.values().map(|value| value.abs()).sum()
    }

    pub fn get(&self, stat: Stat) -> i32 {
        *self.values.get(&stat).unwrap_or(&0)
    }

    pub fn set(&mut self, stat: Stat, value: i32) {
        if value == 0 {
            self.values.remove(&stat);
        } else {
            self.values.insert(stat, value);
        }
    }

    pub fn add_assign(&mut self, other: &Stats) {
        for (stat, value) in other.values.iter() {
            let current = self.get(*stat);
            self.set(*stat, current + value);
        }
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (Stat, i32)> + '_ {
        self.values.iter().map(|(stat, value)| (*stat, *value))
    }

    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()?;
        let mut stats = Stats::new();

        for _ in 0..count {
            let stat_value = reader.read_u8()?;
            let stat =
                Stat::try_from(stat_value).map_err(|_| SharedError::UnknownStat(stat_value))?;
            let value = reader.read_i32::<LittleEndian>()?;
            stats.set(stat, value);
        }

        Ok(stats)
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.values.len() as i32)?;
        for (stat, value) in self.values.iter() {
            writer.write_u8(u8::from(*stat))?;
            writer.write_i32::<LittleEndian>(*value)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaseStat {
    pub formula_type: StatFormula,
    pub stat: Stat,
    pub base: i32,
    pub gain: f32,
    pub gain_rate: f32,
    pub max: i32,
}

impl BaseStat {
    pub fn new(stat: Stat) -> Self {
        Self {
            formula_type: StatFormula::Stat,
            stat,
            base: 0,
            gain: 0.0,
            gain_rate: 0.0,
            max: 0,
        }
    }

    pub fn formula(mut self, formula: StatFormula) -> Self {
        self.formula_type = formula;
        self
    }

    pub fn base(mut self, base: i32) -> Self {
        self.base = base;
        self
    }

    pub fn gain(mut self, gain: f32) -> Self {
        self.gain = gain;
        self
    }

    pub fn gain_rate(mut self, gain_rate: f32) -> Self {
        self.gain_rate = gain_rate;
        self
    }

    pub fn max(mut self, max: i32) -> Self {
        self.max = max;
        self
    }

    pub fn calculate(&self, job: MirClass, level: i32) -> i32 {
        if self.gain == 0.0 {
            return self.base;
        }

        let level_f = level as f32;
        let cap = if self.max > 0 {
            self.max as f32
        } else {
            i32::MAX as f32
        };

        let value = match self.formula_type {
            StatFormula::Health => match job {
                MirClass::Warrior => {
                    self.base as f32
                        + ((level_f / self.gain) + self.gain_rate + (level_f / 20.0)) * level_f
                }
                _ => self.base as f32 + ((level_f / self.gain) + self.gain_rate) * level_f,
            },
            StatFormula::Mana => match job {
                MirClass::Wizard => {
                    self.base as f32
                        + ((level_f / self.gain) + 2.0) * 2.2 * level_f
                        + (level_f * self.gain_rate)
                }
                MirClass::Taoist => {
                    (self.base as f32 + (level_f / self.gain) * 2.2 * level_f)
                        + (level_f * self.gain_rate)
                }
                _ => self.base as f32 + (level_f * self.gain) + (level_f * self.gain_rate),
            },
            StatFormula::Weight => self.base as f32 + ((level_f / self.gain) * level_f),
            StatFormula::Stat => self.base as f32 + (level_f / self.gain),
        };

        value.min(cap) as i32
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaseStats {
    pub job: MirClass,
    pub stats: Vec<BaseStat>,
    pub caps: Stats,
}

impl BaseStats {
    pub fn new(job: MirClass) -> Self {
        let stats = base_stats_for_class(job);
        let caps = default_caps();
        Self { job, stats, caps }
    }

    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let job_value = reader.read_u8()?;
        let job =
            MirClass::try_from(job_value).map_err(|_| SharedError::UnknownClass(job_value))?;
        let count = reader.read_i32::<LittleEndian>()?;

        let mut stats = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let stat_value = reader.read_u8()?;
            let stat =
                Stat::try_from(stat_value).map_err(|_| SharedError::UnknownStat(stat_value))?;
            let formula_value = reader.read_u8()?;
            let formula = StatFormula::try_from(formula_value)
                .map_err(|_| SharedError::UnknownStatFormula(formula_value))?;
            let base = reader.read_i32::<LittleEndian>()?;
            let gain = reader.read_f32::<LittleEndian>()?;
            let gain_rate = reader.read_f32::<LittleEndian>()?;
            let max = reader.read_i32::<LittleEndian>()?;

            stats.push(BaseStat {
                formula_type: formula,
                stat,
                base,
                gain,
                gain_rate,
                max,
            });
        }

        let caps = Stats::read_from(reader)?;

        Ok(Self { job, stats, caps })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(u8::from(self.job))?;
        writer.write_i32::<LittleEndian>(self.stats.len() as i32)?;

        for stat in &self.stats {
            writer.write_u8(u8::from(stat.stat))?;
            writer.write_u8(u8::from(stat.formula_type))?;
            writer.write_i32::<LittleEndian>(stat.base)?;
            writer.write_f32::<LittleEndian>(stat.gain)?;
            writer.write_f32::<LittleEndian>(stat.gain_rate)?;
            writer.write_i32::<LittleEndian>(stat.max)?;
        }

        self.caps.write_to(writer)?;
        Ok(())
    }
}

fn base_stats_for_class(job: MirClass) -> Vec<BaseStat> {
    match job {
        MirClass::Warrior => vec![
            BaseStat::new(Stat::HP)
                .formula(StatFormula::Health)
                .base(14)
                .gain(4.0)
                .gain_rate(4.5)
                .max(0),
            BaseStat::new(Stat::MP)
                .formula(StatFormula::Mana)
                .base(11)
                .gain(3.5)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::BagWeight)
                .formula(StatFormula::Weight)
                .base(50)
                .gain(3.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::WearWeight)
                .formula(StatFormula::Weight)
                .base(15)
                .gain(20.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::HandWeight)
                .formula(StatFormula::Weight)
                .base(12)
                .gain(13.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::MinAC)
                .formula(StatFormula::Stat)
                .base(0)
                .gain(0.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::MaxAC)
                .formula(StatFormula::Stat)
                .base(0)
                .gain(7.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::MinDC)
                .formula(StatFormula::Stat)
                .base(0)
                .gain(5.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::MaxDC)
                .formula(StatFormula::Stat)
                .base(0)
                .gain(5.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::Agility)
                .formula(StatFormula::Stat)
                .base(15)
                .gain(0.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::Accuracy)
                .formula(StatFormula::Stat)
                .base(5)
                .gain(0.0)
                .gain_rate(0.0)
                .max(0),
        ],
        MirClass::Wizard => vec![
            BaseStat::new(Stat::HP)
                .formula(StatFormula::Health)
                .base(14)
                .gain(15.0)
                .gain_rate(1.8)
                .max(0),
            BaseStat::new(Stat::MP)
                .formula(StatFormula::Mana)
                .base(13)
                .gain(5.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::BagWeight)
                .formula(StatFormula::Weight)
                .base(50)
                .gain(5.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::WearWeight)
                .formula(StatFormula::Weight)
                .base(15)
                .gain(100.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::HandWeight)
                .formula(StatFormula::Weight)
                .base(12)
                .gain(90.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::MinDC)
                .formula(StatFormula::Stat)
                .base(0)
                .gain(7.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::MaxDC)
                .formula(StatFormula::Stat)
                .base(0)
                .gain(7.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::MinMC)
                .formula(StatFormula::Stat)
                .base(0)
                .gain(7.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::MaxMC)
                .formula(StatFormula::Stat)
                .base(0)
                .gain(7.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::Agility)
                .formula(StatFormula::Stat)
                .base(15)
                .gain(0.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::Accuracy)
                .formula(StatFormula::Stat)
                .base(5)
                .gain(0.0)
                .gain_rate(0.0)
                .max(0),
        ],
        MirClass::Taoist => vec![
            BaseStat::new(Stat::HP)
                .formula(StatFormula::Health)
                .base(14)
                .gain(6.0)
                .gain_rate(2.5)
                .max(0),
            BaseStat::new(Stat::MP)
                .formula(StatFormula::Mana)
                .base(13)
                .gain(8.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::BagWeight)
                .formula(StatFormula::Weight)
                .base(50)
                .gain(4.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::WearWeight)
                .formula(StatFormula::Weight)
                .base(15)
                .gain(50.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::HandWeight)
                .formula(StatFormula::Weight)
                .base(12)
                .gain(42.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::MinMAC)
                .formula(StatFormula::Stat)
                .base(0)
                .gain(12.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::MaxMAC)
                .formula(StatFormula::Stat)
                .base(0)
                .gain(6.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::MinDC)
                .formula(StatFormula::Stat)
                .base(0)
                .gain(7.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::MaxDC)
                .formula(StatFormula::Stat)
                .base(0)
                .gain(7.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::MinSC)
                .formula(StatFormula::Stat)
                .base(0)
                .gain(7.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::MaxSC)
                .formula(StatFormula::Stat)
                .base(0)
                .gain(7.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::Agility)
                .formula(StatFormula::Stat)
                .base(18)
                .gain(0.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::Accuracy)
                .formula(StatFormula::Stat)
                .base(5)
                .gain(0.0)
                .gain_rate(0.0)
                .max(0),
        ],
        MirClass::Assassin => vec![
            BaseStat::new(Stat::HP)
                .formula(StatFormula::Health)
                .base(14)
                .gain(4.0)
                .gain_rate(3.25)
                .max(0),
            BaseStat::new(Stat::MP)
                .formula(StatFormula::Mana)
                .base(11)
                .gain(5.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::BagWeight)
                .formula(StatFormula::Weight)
                .base(50)
                .gain(3.5)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::WearWeight)
                .formula(StatFormula::Weight)
                .base(15)
                .gain(33.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::HandWeight)
                .formula(StatFormula::Weight)
                .base(12)
                .gain(30.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::MinDC)
                .formula(StatFormula::Stat)
                .base(0)
                .gain(8.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::MaxDC)
                .formula(StatFormula::Stat)
                .base(0)
                .gain(8.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::Agility)
                .formula(StatFormula::Stat)
                .base(20)
                .gain(0.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::Accuracy)
                .formula(StatFormula::Stat)
                .base(5)
                .gain(0.0)
                .gain_rate(0.0)
                .max(0),
        ],
        MirClass::Archer => vec![
            BaseStat::new(Stat::HP)
                .formula(StatFormula::Health)
                .base(14)
                .gain(4.0)
                .gain_rate(3.25)
                .max(0),
            BaseStat::new(Stat::MP)
                .formula(StatFormula::Mana)
                .base(11)
                .gain(4.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::BagWeight)
                .formula(StatFormula::Weight)
                .base(50)
                .gain(4.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::WearWeight)
                .formula(StatFormula::Weight)
                .base(15)
                .gain(33.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::HandWeight)
                .formula(StatFormula::Weight)
                .base(12)
                .gain(30.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::MinDC)
                .formula(StatFormula::Stat)
                .base(0)
                .gain(8.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::MaxDC)
                .formula(StatFormula::Stat)
                .base(0)
                .gain(8.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::MinMC)
                .formula(StatFormula::Stat)
                .base(0)
                .gain(8.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::MaxMC)
                .formula(StatFormula::Stat)
                .base(0)
                .gain(8.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::Agility)
                .formula(StatFormula::Stat)
                .base(15)
                .gain(0.0)
                .gain_rate(0.0)
                .max(0),
            BaseStat::new(Stat::Accuracy)
                .formula(StatFormula::Stat)
                .base(8)
                .gain(0.0)
                .gain_rate(0.0)
                .max(0),
        ],
    }
}

fn default_caps() -> Stats {
    let mut caps = Stats::new();
    caps.set(Stat::MagicResist, 2);
    caps.set(Stat::PoisonResist, 6);
    caps.set(Stat::CriticalRate, 18);
    caps.set(Stat::CriticalDamage, 10);
    caps.set(Stat::Freezing, 6);
    caps.set(Stat::PoisonAttack, 6);
    caps.set(Stat::HealthRecovery, 8);
    caps.set(Stat::SpellRecovery, 8);
    caps.set(Stat::PoisonRecovery, 6);
    caps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_roundtrip_binary() {
        let mut stats = Stats::new();
        stats.set(Stat::HP, 100);
        stats.set(Stat::MP, 50);
        stats.set(Stat::MinDC, 7);

        let mut buffer = Vec::new();
        stats.write_to(&mut buffer).unwrap();

        let mut cursor = std::io::Cursor::new(buffer);
        let decoded = Stats::read_from(&mut cursor).unwrap();

        assert_eq!(decoded.get(Stat::HP), 100);
        assert_eq!(decoded.get(Stat::MP), 50);
        assert_eq!(decoded.get(Stat::MinDC), 7);
        assert_eq!(decoded.len(), 3);
    }

    #[test]
    fn warrior_hp_scaling_matches_reference() {
        let base_stats = BaseStats::new(MirClass::Warrior);
        let hp_entry = base_stats
            .stats
            .iter()
            .find(|stat| stat.stat == Stat::HP)
            .expect("warrior hp stat");

        let level_1 = hp_entry.calculate(MirClass::Warrior, 1);
        let level_40 = hp_entry.calculate(MirClass::Warrior, 40);

        assert!(level_1 >= 14);
        assert_eq!(level_40, 674);
    }
}
