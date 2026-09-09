use crate::components::*;
use crate::death::{MarkedForDeath, MarkedForDespawn};
use crate::events::EntityDeathEvent;
use crate::resources::GameState;
use crate::GameplaySets;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

pub struct ExperiencePlugin;

impl Plugin for ExperiencePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_experience_orbs,
                vacuum_system,
                collect_experience_orbs,
                check_level_up,
            )
                .chain()
                .in_set(GameplaySets::Experience)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

#[derive(Component)]
pub struct Experience {
    pub current: u32,
    pub level: u32,
}

#[derive(Component)]
pub struct ExperienceOrb {
    pub value: u32,
}

#[derive(Component)]
pub struct Vacuumable {
    pub base_speed: f32,
}

impl Default for Vacuumable {
    fn default() -> Self {
        Self {
            base_speed: 300.0, // Some reasonable default speed
        }
    }
}

// Could move this to a config resource if we want to make it data-driven
pub(crate) fn calculate_experience_needed(level: u32) -> u32 {
    // Simple exponential scaling: each level needs 25% more XP than the last
    // Level 1->2: 100 XP
    // Level 2->3: 125 XP
    // Level 3->4: 156 XP
    // etc.
    let base_xp = 100;
    let scaling = 1.25f32;
    (base_xp as f32 * scaling.powi((level - 1) as i32)) as u32
}

#[expect(
    clippy::type_complexity,
    reason = "The ECS query explicitly restricts progression to living players."
)]
pub fn check_level_up(
    mut player_query: Query<
        (&mut Experience, &Health),
        (
            With<Player>,
            Without<MarkedForDeath>,
            Without<MarkedForDespawn>,
        ),
    >,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if let Ok((mut experience, health)) = player_query.single_mut() {
        // Combat resolves first. A lethal hit takes priority over a level earned
        // in the same frame, so leveling cannot replace the GameOver transition.
        if health.current <= 0 {
            return;
        }
        let xp_needed = calculate_experience_needed(experience.level);

        if experience.current >= xp_needed {
            // Bank the leftover XP
            experience.current -= xp_needed;
            experience.level += 1;

            // Trigger level up menu
            next_state.set(GameState::LevelUp);
        }
    }
}

fn spawn_experience_orbs(
    mut commands: Commands,
    mut death_events: MessageReader<EntityDeathEvent>,
) {
    for event in death_events.read() {
        if let Some(exp_value) = event.exp_value.filter(|value| *value > 0) {
            debug!(
                "Dropping {} experience from {:?} at {:?}",
                exp_value, event.entity, event.position
            );
            commands.spawn((
                ExperienceOrb { value: exp_value },
                Vacuumable::default(),
                Sprite {
                    color: Color::srgb(0.5, 0.8, 1.0),
                    custom_size: Some(Vec2::new(8.0, 8.0)),
                    ..default()
                },
                Transform::from_translation(event.position.extend(0.0)),
                // Add Rapier components
                RigidBody::Dynamic,
                Velocity::zero(),
                Collider::ball(4.0), // Smaller collision radius than visual
                Sensor,              // Make it a sensor so it doesn't affect physics
                ActiveEvents::COLLISION_EVENTS,
                CollisionGroups::new(
                    Group::GROUP_4, // Experience orb group
                    Group::GROUP_1, // Player group
                ),
                LockedAxes::ROTATION_LOCKED,
                Damping {
                    linear_damping: 2.0,
                    angular_damping: 1.0,
                },
            ));
        }
    }
}

fn vacuum_system(
    player_query: Query<(&Transform, &Player)>,
    mut items: Query<(&Transform, &Vacuumable, &mut Velocity), Without<MarkedForDespawn>>,
) {
    let Ok((player_transform, player)) = player_query.single() else {
        return;
    };

    for (item_transform, vacuumable, mut velocity) in &mut items {
        let to_player = (player_transform.translation - item_transform.translation).truncate();
        let distance = to_player.length();

        if distance < player.magnet_strength {
            let vacuum_influence = 1.0 - (distance / player.magnet_strength).powi(2);
            // An orb can spawn exactly under the player. Keep Rapier's velocity
            // finite until its sensor intersection is available for collection.
            let vacuum_direction = to_player.normalize_or_zero();

            let speed = vacuumable.base_speed
                * (vacuum_influence * 2.0 + vacuum_influence.powi(3))
                * player.magnet_speed;

            velocity.linear = vacuum_direction * speed;
        }
    }
}

#[expect(
    clippy::type_complexity,
    reason = "The ECS query explicitly excludes dying players from pickup collection."
)]
fn collect_experience_orbs(
    mut commands: Commands,
    mut player_query: Query<
        (Entity, &mut Experience),
        (
            With<Player>,
            Without<MarkedForDeath>,
            Without<MarkedForDespawn>,
        ),
    >,
    orb_query: Query<(Entity, &ExperienceOrb), Without<MarkedForDespawn>>,
    context_query: ReadRapierContext,
) {
    let Ok((player_entity, mut player_exp)) = player_query.single_mut() else {
        return;
    };

    let Ok(context) = context_query.single() else {
        return;
    };

    // Read the physics engine's current overlaps, rather than transient Started
    // messages which can expire while the gameplay systems are paused.
    for (first, second, intersecting) in context.intersection_pairs_with(player_entity) {
        if !intersecting {
            continue;
        }
        let orb = if first == player_entity {
            second
        } else {
            first
        };
        if let Ok((orb_entity, exp_orb)) = orb_query.get(orb) {
            debug!("Collected {} experience", exp_orb.value);
            player_exp.current += exp_orb.value;
            commands.entity(orb_entity).insert(MarkedForDespawn);
        }
    }
}
