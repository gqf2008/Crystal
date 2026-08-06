// ============================================================================
// 对象状态表现层（#226）
// 网络驱动：ObjectHide/ObjectShow/ObjectSitDown/Pushed/ObjectPushed/
//           ObjectTeleportOut/ObjectTeleportIn → ServerEvent
// 绘制参考：Client-Macroquad/src（对象隐身/传送/击退表现）
// ============================================================================

use bevy::prelude::*;

use crate::actor::{ActorAnim, NetObjectId};
use crate::game::movement::tile_to_world;
use crate::network::server_event::ServerEvent;
use crate::scenes::AppState;

pub struct ObjectStatePlugin;

impl Plugin for ObjectStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            apply_object_state_events
                .after(crate::network::network_system)
                .run_if(in_state(AppState::Game)),
        );
    }
}

/// 消费对象状态事件：隐藏/显形/坐下/击退/传送进出
#[allow(clippy::too_many_arguments)]
fn apply_object_state_events(
    mut events: MessageReader<ServerEvent>,
    mut vis: Query<(&NetObjectId, &mut Visibility)>,
    mut anim: Query<(&NetObjectId, &mut ActorAnim)>,
    mut transforms: Query<(&NetObjectId, &mut Transform)>,
    mut effects: MessageWriter<crate::game::effects::PendingEffect>,
) {
    let pending: Vec<ServerEvent> = events.read().cloned().collect();
    if pending.is_empty() {
        return;
    }
    for ev in pending {
        match ev {
            ServerEvent::ObjectHidden { object_id } => {
                let found = vis.iter().any(|(id, _)| id.0 == object_id);
                tracing::debug!("[OBJSTATE] 隐藏 id={} found={}", object_id, found);
                for (id, mut v) in &mut vis {
                    if id.0 == object_id {
                        *v = Visibility::Hidden;
                        break;
                    }
                }
            }
            ServerEvent::ObjectShown { object_id } => {
                for (id, mut v) in &mut vis {
                    if id.0 == object_id {
                        *v = Visibility::Visible;
                        break;
                    }
                }
            }
            ServerEvent::ObjectSitDown {
                object_id,
                direction,
            } => {
                let found = anim.iter().any(|(id, _)| id.0 == object_id);
                tracing::debug!(
                    "[OBJSTATE] 坐下 id={} dir={} found={}",
                    object_id,
                    direction,
                    found
                );
                // 有 SitDown 帧表才切动作，否则只更新朝向（避免动画冻结）
                let has_sit = crate::objects::frames::get_player_frame(
                    mir2_shared::enums::MirAction::SitDown,
                )
                .is_some();
                for (id, mut a) in &mut anim {
                    if id.0 == object_id {
                        a.direction = direction;
                        if has_sit {
                            a.action = mir2_shared::enums::MirAction::SitDown;
                            a.frame_index = 0;
                        }
                        break;
                    }
                }
            }
            ServerEvent::ObjectPushed {
                object_id,
                x,
                y,
                direction,
            } => {
                let to = tile_to_world(x, y);
                for (id, mut tf) in &mut transforms {
                    if id.0 == object_id {
                        tf.translation.x = to.x;
                        tf.translation.y = to.y;
                        break;
                    }
                }
                for (id, mut a) in &mut anim {
                    if id.0 == object_id {
                        a.direction = direction;
                        break;
                    }
                }
            }
            ServerEvent::ObjectTeleportOut { object_id } => {
                // 传送消失：白紫色爆点 + 隐藏
                effects.write(crate::game::effects::PendingEffect::Burst {
                    target_id: object_id,
                    color: [0.8, 0.7, 1.0],
                });
                for (id, mut v) in &mut vis {
                    if id.0 == object_id {
                        *v = Visibility::Hidden;
                        break;
                    }
                }
            }
            ServerEvent::ObjectTeleportIn { object_id } => {
                effects.write(crate::game::effects::PendingEffect::Burst {
                    target_id: object_id,
                    color: [0.8, 0.7, 1.0],
                });
                for (id, mut v) in &mut vis {
                    if id.0 == object_id {
                        *v = Visibility::Visible;
                        break;
                    }
                }
            }
            _ => {}
        }
    }
}
