// Combat Handler - 战斗相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct CombatHandler;

impl PacketHandler for CombatHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            // ObjectAttack - another object attacks
            x if x == ServerPacketIds::ObjectAttack as u16 => {
                if let Ok(packet) = server::ObjectAttack::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectAttack { packet });
                    tracing::trace!("⚔️ ObjectAttack received");
                }
            }

            // Struck - player was hit
            x if x == ServerPacketIds::Struck as u16 => {
                if let Ok(packet) = server::Struck::read_body(&mut cursor) {
                    events.push(NetworkEvent::PlayerStruck {
                        attacker_id: packet.attacker_id,
                        damage: 0,  // Struck包没有damage字段
                    });
                    tracing::debug!("⚔️ Player struck by {}", packet.attacker_id);
                }
            }
            
            // Death - player died
            x if x == ServerPacketIds::Death as u16 => {
                if let Ok(_packet) = server::Death::read_body(&mut cursor) {
                    events.push(NetworkEvent::PlayerDied);
                    tracing::warn!("💀 Player died");
                }
            }
            
            // ObjectStruck - another object was hit
            x if x == ServerPacketIds::ObjectStruck as u16 => {
                if let Ok(packet) = server::ObjectStruck::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectStruck {
                        object_id: packet.object_id,
                        attacker_id: packet.attacker_id,
                        damage: 0,  // ObjectStruck包没有damage字段
                    });
                    tracing::trace!("⚔️ Object {} struck by {}", 
                        packet.object_id, packet.attacker_id);
                }
            }

            // DamageIndicator - damage number & target id
            x if x == ServerPacketIds::DamageIndicator as u16 => {
                if let Ok(packet) = server::DamageIndicator::read_body(&mut cursor) {
                    events.push(NetworkEvent::DamageIndicator {
                        object_id: packet.object_id,
                        damage: packet.damage,
                        damage_type: packet.damage_type,
                    });
                    tracing::trace!(
                        "💥 DamageIndicator: object={} dmg={} type={}",
                        packet.object_id,
                        packet.damage,
                        packet.damage_type
                    );
                }
            }
            
            // ObjectDied - another object died
            x if x == ServerPacketIds::ObjectDied as u16 => {
                if let Ok(packet) = server::ObjectDied::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectDied {
                        object_id: packet.object_id,
                    });
                    tracing::trace!("💀 Object {} died", packet.object_id);
                }
            }

            // HealthChanged - local player hp/mp updated
            x if x == ServerPacketIds::HealthChanged as u16 => {
                if let Ok(packet) = server::HealthChanged::read_body(&mut cursor) {
                    // 协议只携带 hp/mp 当前值；max 由客户端已有状态决定。
                    // 用 max=0 作为“未知/不要覆盖 max”的标记，由落地层处理。
                    events.push(NetworkEvent::HealthChanged {
                        current: packet.hp,
                        max: 0,
                    });
                    events.push(NetworkEvent::ManaChanged {
                        current: packet.mp,
                        max: 0,
                    });
                    tracing::debug!("❤️ HealthChanged hp={} mp={}", packet.hp, packet.mp);
                }
            }

            // ObjectHealth - percent based health update
            x if x == ServerPacketIds::ObjectHealth as u16 => {
                if let Ok(packet) = server::ObjectHealth::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectHealthPercent {
                        object_id: packet.object_id,
                        percent: packet.percent,
                        expire: packet.expire,
                    });
                    tracing::trace!(
                        "🩸 ObjectHealthPercent object={} {}% expire={}",
                        packet.object_id,
                        packet.percent,
                        packet.expire
                    );
                }
            }
            
            // GainExperience
            x if x == ServerPacketIds::GainExperience as u16 => {
                if let Ok(packet) = server::GainExperience::read_body(&mut cursor) {
                    events.push(NetworkEvent::ExperienceGained {
                        amount: packet.amount as i64,  // u32→i64
                    });
                    tracing::debug!("✨ Experience gained: {}", packet.amount);
                }
            }
            
            // LevelChanged
            x if x == ServerPacketIds::LevelChanged as u16 => {
                if let Ok(packet) = server::LevelChanged::read_body(&mut cursor) {
                    events.push(NetworkEvent::LevelUp {
                        new_level: packet.level,
                    });
                    tracing::info!("🎉 Level up to {}!", packet.level);
                }
            }
            
            // ObjectMana - another object's mana percent
            x if x == ServerPacketIds::ObjectMana as u16 => {
                if let Ok(packet) = server::ObjectMana::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectManaPercent {
                        object_id: packet.object_id,
                        percent: packet.percent,
                    });
                    tracing::trace!(
                        "💙 ObjectManaPercent object={} {}%",
                        packet.object_id,
                        packet.percent
                    );
                }
            }

            // DuraChanged - item durability changed
            x if x == ServerPacketIds::DuraChanged as u16 => {
                if let Ok(packet) = server::DuraChanged::read_body(&mut cursor) {
                    events.push(NetworkEvent::DuraChanged {
                        unique_id: packet.unique_id,
                        durability: packet.current_dura as i32,
                    });
                    tracing::trace!("🔧 DuraChanged: item={} durability={}", packet.unique_id, packet.current_dura);
                }
            }

            // Poisoned - local player poisoned
            x if x == ServerPacketIds::Poisoned as u16 => {
                if let Ok(packet) = server::Poisoned::read_body(&mut cursor) {
                    events.push(NetworkEvent::PlayerPoisoned {
                        object_id: 0,
                        poison_type: packet.poison.bits() as u8,
                    });
                    tracing::debug!("☠️ Player poisoned: {:?}", packet.poison);
                }
            }

            // ObjectPoisoned - another object poisoned
            x if x == ServerPacketIds::ObjectPoisoned as u16 => {
                if let Ok(packet) = server::ObjectPoisoned::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectPoisonedEvent {
                        object_id: packet.object_id,
                        poison_type: packet.poison.bits() as u8,
                    });
                    tracing::trace!("☠️ Object {} poisoned: {:?}", packet.object_id, packet.poison);
                }
            }

            // RangeAttack - local player performed a ranged attack
            x if x == ServerPacketIds::RangeAttack as u16 => {
                if let Ok(packet) = server::RangeAttack::read_body(&mut cursor) {
                    events.push(NetworkEvent::RangeAttacked {
                        object_id: packet.target_id,
                    });
                    tracing::trace!("🏹 RangeAttack: target={}", packet.target_id);
                }
            }

            // ObjectRangeAttack - another object performed a ranged attack
            x if x == ServerPacketIds::ObjectRangeAttack as u16 => {
                if let Ok(packet) = server::ObjectRangeAttack::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectRangeAttacked {
                        object_id: packet.object_id,
                    });
                    tracing::trace!("🏹 Object {} range attacked target={}", packet.object_id, packet.target_id);
                }
            }

            // Pushed - local player pushed
            x if x == ServerPacketIds::Pushed as u16 => {
                if let Ok(packet) = server::Pushed::read_body(&mut cursor) {
                    events.push(NetworkEvent::PushedEvent {
                        object_id: 0,
                        x: packet.location_x as i32,
                        y: packet.location_y as i32,
                    });
                    tracing::trace!("🫸 Player pushed to ({}, {})", packet.location_x, packet.location_y);
                }
            }

            // ObjectPushed - another object pushed
            x if x == ServerPacketIds::ObjectPushed as u16 => {
                if let Ok(packet) = server::ObjectPushed::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectPushedEvent {
                        object_id: packet.object_id,
                        x: packet.location_x as i32,
                        y: packet.location_y as i32,
                    });
                    tracing::trace!("🫸 Object {} pushed to ({}, {})", packet.object_id, packet.location_x, packet.location_y);
                }
            }

            // UserDashAttack - local player dash attack
            x if x == ServerPacketIds::UserDashAttack as u16 => {
                if let Ok(_packet) = server::UserDashAttack::read_body(&mut cursor) {
                    events.push(NetworkEvent::UserDashAttacked);
                    tracing::trace!("⚡ User dash attack");
                }
            }

            // ObjectDashAttack - another object dash attack
            x if x == ServerPacketIds::ObjectDashAttack as u16 => {
                if let Ok(packet) = server::ObjectDashAttack::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectDashAttacked {
                        object_id: packet.object_id,
                    });
                    tracing::trace!("⚡ Object {} dash attack", packet.object_id);
                }
            }

            // UserAttackMove - local player attack move
            x if x == ServerPacketIds::UserAttackMove as u16 => {
                if let Ok(packet) = server::UserAttackMove::read_body(&mut cursor) {
                    events.push(NetworkEvent::UserAttackMoved {
                        x: packet.location_x,
                        y: packet.location_y,
                    });
                    tracing::trace!("💨 User attack move to ({}, {})", packet.location_x, packet.location_y);
                }
            }

            // Revived - local player revived
            x if x == ServerPacketIds::Revived as u16 => {
                if let Ok(_packet) = server::Revived::read_body(&mut cursor) {
                    events.push(NetworkEvent::PlayerRevived);
                    tracing::info!("🔄 Player revived");
                }
            }

            // ObjectRevived - another object revived
            x if x == ServerPacketIds::ObjectRevived as u16 => {
                if let Ok(packet) = server::ObjectRevived::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectRevivedEvent {
                        object_id: packet.object_id,
                    });
                    tracing::trace!("🔄 Object {} revived", packet.object_id);
                }
            }

            // ObjectLeveled - another object leveled up
            x if x == ServerPacketIds::ObjectLeveled as u16 => {
                if let Ok(packet) = server::ObjectLeveled::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectLeveled {
                        object_id: packet.object_id,
                    });
                    tracing::trace!("🎉 Object {} leveled up to {}", packet.object_id, packet.level);
                }
            }

            // ========================================================================
            // Hero Events
            // ========================================================================

            // HeroHealthChanged - hero hp/mp updated
            x if x == ServerPacketIds::HeroHealthChanged as u16 => {
                if let Ok(packet) = server::HeroHealthChanged::read_body(&mut cursor) {
                    tracing::trace!("🧡 HeroHealthChanged hp={} mp={}", packet.hp, packet.mp);
                }
            }

            // GainHeroExperience - hero gained experience
            x if x == ServerPacketIds::GainHeroExperience as u16 => {
                if let Ok(packet) = server::GainHeroExperience::read_body(&mut cursor) {
                    events.push(NetworkEvent::HeroExperienceGained {
                        amount: packet.amount as i64,
                    });
                    tracing::debug!("✨ Hero experience gained: {}", packet.amount);
                }
            }

            // HeroLevelChanged - hero level up
            x if x == ServerPacketIds::HeroLevelChanged as u16 => {
                if let Ok(packet) = server::HeroLevelChanged::read_body(&mut cursor) {
                    events.push(NetworkEvent::HeroLevelUp {
                        new_level: packet.level,
                    });
                    tracing::info!("🎉 Hero level up to {}!", packet.level);
                }
            }

            // ========================================================================
            // Magic/Spell Events
            // ========================================================================

            // NewMagic - player learned a new spell
            x if x == ServerPacketIds::NewMagic as u16 => {
                if let Ok(packet) = server::NewMagic::read_body(&mut cursor) {
                    events.push(NetworkEvent::MagicLearned {
                        spell: packet.magic.spell,
                        level: packet.magic.level,
                    });
                    tracing::debug!("✨ NewMagic learned: {:?} level={} (hero={})", packet.magic.spell, packet.magic.level, packet.hero);
                }
            }

            // RemoveMagic - spell removed from player
            x if x == ServerPacketIds::RemoveMagic as u16 => {
                if let Ok(packet) = server::RemoveMagic::read_body(&mut cursor) {
                    events.push(NetworkEvent::MagicRemoved { spell: packet.spell });
                    tracing::debug!("📜 Magic removed: {:?} (hero={})", packet.spell, packet.hero);
                }
            }

            // MagicLeveled - spell leveled up
            x if x == ServerPacketIds::MagicLeveled as u16 => {
                if let Ok(packet) = server::MagicLeveled::read_body(&mut cursor) {
                    events.push(NetworkEvent::MagicLeveledUp {
                        spell: packet.spell,
                        level: packet.level,
                    });
                    tracing::debug!("📈 Magic leveled up: {:?} level={} (hero={})", packet.spell, packet.level, packet.hero);
                }
            }

            // Magic - magic list / spell cast notification
            x if x == ServerPacketIds::Magic as u16 => {
                if let Ok(packet) = server::Magic::read_body(&mut cursor) {
                    events.push(NetworkEvent::MagicListReceived);
                    tracing::trace!("🔮 Magic: spell={:?} target={} cast={}", packet.spell, packet.target_id, packet.cast);
                }
            }

            // MagicDelay - spell cooldown
            x if x == ServerPacketIds::MagicDelay as u16 => {
                if let Ok(packet) = server::MagicDelay::read_body(&mut cursor) {
                    events.push(NetworkEvent::MagicDelayReceived {
                        object_id: packet.object_id,
                        spell: packet.spell,
                        delay: packet.delay as u32,
                    });
                    tracing::trace!("⏳ MagicDelay: object={} spell={:?} delay={}", packet.object_id, packet.spell, packet.delay);
                }
            }

            // MagicCast - spell cast confirmation
            x if x == ServerPacketIds::MagicCast as u16 => {
                if let Ok(packet) = server::MagicCast::read_body(&mut cursor) {
                    events.push(NetworkEvent::MagicCastEvent { spell: packet.spell });
                    tracing::trace!("🪄 MagicCast: {:?}", packet.spell);
                }
            }

            // ObjectMagic - another object casts a spell
            x if x == ServerPacketIds::ObjectMagic as u16 => {
                if let Ok(packet) = server::ObjectMagic::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectMagicCast {
                        object_id: packet.object_id,
                        spell: packet.spell,
                        target_id: packet.target_id,
                    });
                    tracing::trace!("🔮 Object {} magic: {:?} target={}", packet.object_id, packet.spell, packet.target_id);
                }
            }

            // ObjectEffect - object spell effect
            x if x == ServerPacketIds::ObjectEffect as u16 => {
                if let Ok(packet) = server::ObjectEffect::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectEffectReceived {
                        object_id: packet.object_id,
                        effect: packet.effect as u16,
                        effect_type: packet.effect_type as u8,
                    });
                    tracing::trace!(
                        "✨ ObjectEffect: object={} effect={:?} type={} delay={} duration={}",
                        packet.object_id, packet.effect, packet.effect_type, packet.delay_time, packet.time
                    );
                }
            }

            // ObjectProjectile - projectile from one object to another
            x if x == ServerPacketIds::ObjectProjectile as u16 => {
                if let Ok(packet) = server::ObjectProjectile::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectProjectileReceived {
                        spell: packet.spell,
                        source: packet.source,
                        destination: packet.destination,
                    });
                    tracing::trace!(
                        "🎯 ObjectProjectile: {:?} src={} dst={}",
                        packet.spell, packet.source, packet.destination
                    );
                }
            }

            // SpellToggle - spell toggle status changed
            x if x == ServerPacketIds::SpellToggle as u16 => {
                if let Ok(packet) = server::SpellToggle::read_body(&mut cursor) {
                    events.push(NetworkEvent::SpellToggled {
                        spell: packet.spell,
                        can_use: packet.can_use,
                    });
                    tracing::trace!("🔄 SpellToggle: {:?} can_use={} (hero={})", packet.spell, packet.can_use, packet.hero);
                }
            }

            // ========================================================================
            // Buff Events
            // ========================================================================

            // AddBuff - buff added to object
            x if x == ServerPacketIds::AddBuff as u16 => {
                if let Ok(packet) = server::AddBuff::read_body(&mut cursor) {
                    events.push(NetworkEvent::BuffAdded {
                        object_id: packet.buff.object_id,
                        buff_id: packet.buff.buff_type as u32,
                    });
                    tracing::trace!(
                        "➕ BuffAdded: object={} buff={:?} visible={} expire={} infinite={} paused={}",
                        packet.buff.object_id, packet.buff.buff_type, packet.buff.visible,
                        packet.buff.expire_time, packet.buff.infinite, packet.buff.paused
                    );
                }
            }

            // RemoveBuff - buff removed from object
            x if x == ServerPacketIds::RemoveBuff as u16 => {
                if let Ok(packet) = server::RemoveBuff::read_body(&mut cursor) {
                    events.push(NetworkEvent::BuffRemoved {
                        object_id: packet.object_id,
                        buff_id: packet.buff_type as u32,
                    });
                    tracing::trace!("➖ BuffRemoved: object={} buff={:?}", packet.object_id, packet.buff_type);
                }
            }

            // PauseBuff - buff paused/resumed
            x if x == ServerPacketIds::PauseBuff as u16 => {
                if let Ok(packet) = server::PauseBuff::read_body(&mut cursor) {
                    events.push(NetworkEvent::BuffPaused {
                        object_id: packet.object_id,
                        buff_id: packet.buff_type as u32,
                        paused: packet.paused,
                    });
                    tracing::trace!(
                        "⏸️ BuffPaused: object={} buff={:?} paused={}",
                        packet.object_id, packet.buff_type, packet.paused
                    );
                }
            }

            // ========================================================================
            // Misc Combat/Status Events
            // ========================================================================

            // SetConcentration
            x if x == ServerPacketIds::SetConcentration as u16 => {
                if let Ok(packet) = server::SetConcentration::read_body(&mut cursor) {
                    events.push(NetworkEvent::ConcentrationSet {
                        enabled: packet.enabled,
                    });
                    tracing::debug!("🎯 ConcentrationSet: enabled={}", packet.enabled);
                }
            }

            // SetElemental
            x if x == ServerPacketIds::SetElemental as u16 => {
                if let Ok(packet) = server::SetElemental::read_body(&mut cursor) {
                    events.push(NetworkEvent::ElementalSet {
                        element: packet.element,
                    });
                    tracing::debug!("🔥 ElementalSet: element={}", packet.element);
                }
            }

            // RemoveDelayedExplosion
            x if x == ServerPacketIds::RemoveDelayedExplosion as u16 => {
                if let Ok(_packet) = server::RemoveDelayedExplosion::read_body(&mut cursor) {
                    events.push(NetworkEvent::DelayedExplosionRemoved);
                    tracing::debug!("💣 DelayedExplosionRemoved");
                }
            }

            // ObjectDeco
            x if x == ServerPacketIds::ObjectDeco as u16 => {
                if let Ok(packet) = server::ObjectDeco::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectDecoReceived {
                        object_id: packet.object_id,
                    });
                    tracing::debug!("🎭 ObjectDecoReceived: object={}", packet.object_id);
                }
            }

            // ObjectSneaking
            x if x == ServerPacketIds::ObjectSneaking as u16 => {
                if let Ok(packet) = server::ObjectSneaking::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectSneakingReceived {
                        object_id: packet.object_id,
                    });
                    tracing::debug!("🥷 ObjectSneakingReceived: object={}", packet.object_id);
                }
            }

            // ObjectLevelEffects
            x if x == ServerPacketIds::ObjectLevelEffects as u16 => {
                if let Ok(packet) = server::ObjectLevelEffects::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectLevelEffectsReceived {
                        object_id: packet.object_id,
                    });
                    tracing::debug!("✨ ObjectLevelEffectsReceived: object={}", packet.object_id);
                }
            }

            // SetBindingShot
            x if x == ServerPacketIds::SetBindingShot as u16 => {
                if let Ok(packet) = server::SetBindingShot::read_body(&mut cursor) {
                    events.push(NetworkEvent::BindingShotSet {
                        enabled: packet.enabled,
                    });
                    tracing::debug!("🎯 BindingShotSet: enabled={}", packet.enabled);
                }
            }

            // SendOutputMessage
            x if x == ServerPacketIds::SendOutputMessage as u16 => {
                if let Ok(packet) = server::SendOutputMessage::read_body(&mut cursor) {
                    events.push(NetworkEvent::OutputMessageReceived {
                        message: packet.message.clone(),
                    });
                    tracing::debug!("📢 OutputMessageReceived: {}", packet.message);
                }
            }

            // InTrapRock
            x if x == ServerPacketIds::InTrapRock as u16 => {
                if let Ok(_packet) = server::InTrapRock::read_body(&mut cursor) {
                    events.push(NetworkEvent::TrapRockEntered { object_id: 0 });
                    tracing::debug!("🪨 TrapRockEntered");
                }
            }

            // BaseStatsInfo
            x if x == ServerPacketIds::BaseStatsInfo as u16 => {
                if let Ok(_packet) = server::BaseStatsInfo::read_body(&mut cursor) {
                    events.push(NetworkEvent::BaseStatsReceived);
                    tracing::debug!("📊 BaseStatsReceived");
                }
            }

            // ObjectHidden
            x if x == ServerPacketIds::ObjectHidden as u16 => {
                if let Ok(packet) = server::ObjectHidden::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectHiddenByName {
                        name: format!("{}", packet.object_id),
                    });
                    tracing::debug!("👻 ObjectHiddenByName: object={} hidden={}", packet.object_id, packet.hidden);
                }
            }

            // ObjectSpell
            x if x == ServerPacketIds::ObjectSpell as u16 => {
                if let Ok(packet) = server::ObjectSpell::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectSpellReceived {
                        object_id: packet.object_id,
                    });
                    tracing::debug!("🔮 ObjectSpellReceived: object={} spell={:?}", packet.object_id, packet.spell);
                }
            }

            // MapEffect
            x if x == ServerPacketIds::MapEffect as u16 => {
                if let Ok(packet) = server::MapEffect::read_body(&mut cursor) {
                    events.push(NetworkEvent::MapEffectReceived {
                        effect: packet.effect as u8,
                    });
                    tracing::debug!("🌟 MapEffectReceived: effect={:?}", packet.effect);
                }
            }

            // AllowObserve
            x if x == ServerPacketIds::AllowObserve as u16 => {
                if let Ok(_packet) = server::AllowObserve::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObserveAllowed { allowed: true });
                    tracing::debug!("👁️ ObserveAllowed");
                }
            }

            // UserStorage
            x if x == ServerPacketIds::UserStorage as u16 => {
                if let Ok(packet) = server::UserStorage::read_body(&mut cursor) {
                    let items: Vec<_> = packet.storage.into_iter().flatten().collect();
                    events.push(NetworkEvent::UserStorageReceived { items: items.clone() });
                    tracing::debug!("🏦 UserStorageReceived: {} items", items.len());
                }
            }

            _ => {
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }
        
        events
    }
}
