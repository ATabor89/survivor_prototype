//! A scripted player: it steers through a virtual gamepad and answers menus.
//!
//! The pilot only uses inputs a person has. It moves the left stick of a
//! gamepad entity and presses its South button, so movement, level-ups, and
//! treasure all go through the same systems as a real session.
use super::policy::{Caution, MovementStyle, UpgradePolicy};
use crate::arena::ArenaBounds;
use crate::components::{Enemy, Health, Player};
use crate::death::{MarkedForDeath, MarkedForDespawn};
use crate::enemy_behavior::HostileProjectile;
use crate::experience::{Experience, ExperienceOrb};
use crate::loot::LootKind;
use crate::menu::{MenuAction, MenuActionComponent, MenuItem, MenuRoot};
use crate::resources::{GameState, GameStats};
use crate::upgrade::UpgradeType;
use crate::waves::Boss;
use crate::weapons::{WeaponMeta, WeaponType};
use crate::GameplaySets;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rand::{rngs::StdRng, RngExt, SeedableRng};

/// Walls closer than this push the pilot back toward open ground.
const WALL_MARGIN: f32 = 140.0;
/// Pickups closer than this are worth walking to.
const LURE_RADIUS: f32 = 420.0;
const BOSS_WEIGHT: f32 = 3.0;
const PROJECTILE_WEIGHT: f32 = 1.5;
/// Circling keeps a kiting pilot from backing itself into a corner.
const ORBIT_BIAS: f32 = 0.35;
const WALL_WEIGHT: f32 = 2.5;

/// The scripted player's configuration and private state.
#[derive(Resource)]
pub struct Pilot {
    pub movement: MovementStyle,
    pub upgrades: UpgradePolicy,
    /// Weapons the run started with; `UpgradePolicy::Solo` invests in these.
    pub focus: Vec<WeaponType>,
    pub rng: StdRng,
    pad: Entity,
}

/// What the pilot observed, sampled while the player is alive.
#[derive(Resource, Default, Clone, Debug)]
pub struct FlightLog {
    pub level: u32,
    /// Level at the end of each completed minute of the run.
    pub level_by_minute: Vec<u32>,
    pub lowest_health: Option<i32>,
    /// Owned weapons and their levels, most recent observation.
    pub weapons: Vec<(WeaponType, u32)>,
}

pub struct AutoplayPlugin {
    pub movement: MovementStyle,
    pub upgrades: UpgradePolicy,
    pub focus: Vec<WeaponType>,
    pub seed: u64,
}

impl Plugin for AutoplayPlugin {
    fn build(&self, app: &mut App) {
        let pad = app.world_mut().spawn(Gamepad::default()).id();
        app.insert_resource(Pilot {
            movement: self.movement,
            upgrades: self.upgrades,
            focus: self.focus.clone(),
            // Keep the pilot's choices independent of the run's own streams.
            rng: StdRng::seed_from_u64(self.seed ^ 0x5049_4c4f_545f_524e),
            pad,
        })
        .init_resource::<FlightLog>()
        .add_systems(OnEnter(GameState::Restarting), forget_previous_run)
        .add_systems(PreUpdate, press_confirm)
        .add_systems(
            Update,
            (
                steer.before(GameplaySets::Movement),
                record.after(GameplaySets::Progress),
            )
                .run_if(in_state(GameState::Playing)),
        );
    }
}

/// A point the pilot wants distance from, and how badly.
#[derive(Clone, Copy, Debug)]
pub struct Threat {
    pub position: Vec2,
    pub weight: f32,
}

/// The stick direction for a pilot at `position` inside `bounds` (half-size).
///
/// Threats inside the pilot's caution radius repel with a strength that grows
/// as they close in, plus a sideways bias so the pilot circles a crowd. Walls
/// repel near the edges. The nearest pickup attracts.
pub fn steering(
    position: Vec2,
    bounds: Vec2,
    threats: &[Threat],
    lures: &[Vec2],
    caution: Caution,
) -> Vec2 {
    let Caution {
        threat_radius,
        lure_weight,
    } = caution;
    let mut flee = Vec2::ZERO;
    for threat in threats {
        let away = position - threat.position;
        let distance = away.length();
        if distance >= threat_radius {
            continue;
        }
        // A threat on top of the pilot has no direction; step out along +x.
        let direction = away.try_normalize().unwrap_or(Vec2::X);
        let urgency = ((threat_radius - distance) / threat_radius).powi(2);
        flee += direction * urgency * threat.weight;
    }
    let mut desire = flee + flee.perp() * ORBIT_BIAS;

    for axis in 0..2 {
        let gap = bounds[axis] - position[axis].abs();
        if gap < WALL_MARGIN {
            let inward = -position[axis].signum();
            let urgency = ((WALL_MARGIN - gap) / WALL_MARGIN).clamp(0.0, 1.0).powi(2);
            desire[axis] += inward * urgency * WALL_WEIGHT;
        }
    }

    let nearest_lure = lures
        .iter()
        .map(|lure| *lure - position)
        .filter(|offset| offset.length_squared() <= LURE_RADIUS * LURE_RADIUS)
        .min_by(|a, b| a.length_squared().total_cmp(&b.length_squared()));
    if let Some(direction) = nearest_lure.and_then(Vec2::try_normalize) {
        desire += direction * lure_weight;
    }

    if desire.length_squared() < 0.0025 {
        Vec2::ZERO
    } else {
        desire.normalize()
    }
}

/// Enemies that can still hurt the player.
type LiveEnemy = (
    With<Enemy>,
    Without<MarkedForDeath>,
    Without<MarkedForDespawn>,
);

/// Everything on the field the pilot reacts to.
#[derive(SystemParam)]
struct Surroundings<'w, 's> {
    enemies: Query<'w, 's, (&'static Transform, Has<Boss>), LiveEnemy>,
    projectiles: Query<'w, 's, &'static Transform, With<HostileProjectile>>,
    orbs: Query<'w, 's, &'static Transform, With<ExperienceOrb>>,
    loot: Query<'w, 's, &'static Transform, With<LootKind>>,
}

impl Surroundings<'_, '_> {
    fn threats(&self) -> Vec<Threat> {
        let enemies = self.enemies.iter().map(|(transform, boss)| Threat {
            position: transform.translation.truncate(),
            weight: if boss { BOSS_WEIGHT } else { 1.0 },
        });
        let projectiles = self.projectiles.iter().map(|transform| Threat {
            position: transform.translation.truncate(),
            weight: PROJECTILE_WEIGHT,
        });
        enemies.chain(projectiles).collect()
    }

    fn lures(&self) -> Vec<Vec2> {
        self.orbs
            .iter()
            .chain(&self.loot)
            .map(|transform| transform.translation.truncate())
            .collect()
    }
}

fn steer(
    pilot: Res<Pilot>,
    bounds: Res<ArenaBounds>,
    player: Query<&Transform, With<Player>>,
    surroundings: Surroundings,
    mut pads: Query<&mut Gamepad>,
) {
    let (Ok(player), Ok(mut pad)) = (player.single(), pads.get_mut(pilot.pad)) else {
        return;
    };
    let stick = pilot.movement.caution().map_or(Vec2::ZERO, |caution| {
        steering(
            player.translation.truncate(),
            bounds.half_size,
            &surroundings.threats(),
            &surroundings.lures(),
            caution,
        )
    });
    pad.analog_mut().set(GamepadAxis::LeftStickX, stick.x);
    pad.analog_mut().set(GamepadAxis::LeftStickY, stick.y);
}

/// Tap South every other frame while a level-up or treasure screen is open,
/// after marking the offer the policy prefers.
///
/// Menus ignore the input that opened them, so a single press is not always
/// enough; tapping until the screen closes is what a player would do.
fn press_confirm(
    mut pilot: ResMut<Pilot>,
    state: Res<State<GameState>>,
    roots: Query<(), With<MenuRoot>>,
    mut items: Query<(&mut MenuItem, &MenuActionComponent)>,
    mut pads: Query<&mut Gamepad>,
) {
    let Ok(mut pad) = pads.get_mut(pilot.pad) else {
        return;
    };
    // No gamepad backend runs headless, so do its per-frame bookkeeping here.
    pad.digital_mut().clear();
    if pad.pressed(GamepadButton::South) {
        pad.digital_mut().release(GamepadButton::South);
        return;
    }
    match state.get() {
        GameState::Treasure => {}
        GameState::LevelUp if !roots.is_empty() => mark_choice(&mut pilot, &mut items),
        _ => return,
    }
    pad.digital_mut().press(GamepadButton::South);
}

/// Select the menu item the pilot wants before it confirms.
fn mark_choice(pilot: &mut Pilot, items: &mut Query<(&mut MenuItem, &MenuActionComponent)>) {
    let offers: Vec<UpgradeType> = items
        .iter()
        .filter_map(|(_, action)| match &action.action {
            MenuAction::SelectUpgrade(choice) => Some(choice.upgrade_type.clone()),
            _ => None,
        })
        .collect();
    let patterns = items
        .iter()
        .filter(|(_, action)| matches!(action.action, MenuAction::SelectPattern(_)))
        .count();
    // Upgrade offers and circle patterns never share a menu.
    let wanted = if offers.is_empty() {
        (patterns > 0).then(|| pilot.rng.random_range(0..patterns))
    } else {
        let Pilot {
            upgrades,
            focus,
            rng,
            ..
        } = pilot;
        upgrades.choose(&offers, focus, rng)
    };
    let Some(wanted) = wanted else {
        return;
    };
    let choices = items.iter_mut().filter(|(_, action)| {
        matches!(
            action.action,
            MenuAction::SelectUpgrade(_) | MenuAction::SelectPattern(_)
        )
    });
    for (index, (mut item, _)) in choices.enumerate() {
        item.selected = index == wanted;
    }
}

fn forget_previous_run(mut log: ResMut<FlightLog>) {
    *log = FlightLog::default();
}

fn record(
    stats: Res<GameStats>,
    player: Query<(&Experience, &Health, &Children), With<Player>>,
    weapons: Query<&WeaponMeta>,
    mut log: ResMut<FlightLog>,
) {
    let Ok((experience, health, children)) = player.single() else {
        return;
    };
    log.level = experience.level;
    log.lowest_health = Some(
        log.lowest_health
            .map_or(health.current, |lowest| lowest.min(health.current)),
    );
    let minutes = (stats.time_elapsed / 60.0) as usize;
    while log.level_by_minute.len() < minutes {
        log.level_by_minute.push(experience.level);
    }
    let mut owned: Vec<_> = children
        .iter()
        .filter_map(|child| weapons.get(child).ok())
        .map(|meta| (meta.weapon_type, meta.level))
        .collect();
    owned.sort();
    if log.weapons != owned {
        log.weapons = owned;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPEN: Vec2 = Vec2::new(1600.0, 1200.0);
    const CAREFUL: Caution = MovementStyle::Evasive.caution().unwrap();
    const THREAT_RADIUS: f32 = CAREFUL.threat_radius;

    fn threat(x: f32, y: f32) -> Threat {
        Threat {
            position: Vec2::new(x, y),
            weight: 1.0,
        }
    }

    #[test]
    fn a_quiet_field_needs_no_movement() {
        assert_eq!(steering(Vec2::ZERO, OPEN, &[], &[], CAREFUL), Vec2::ZERO);
        // Distant threats and distant pickups are ignored.
        let far = [threat(THREAT_RADIUS + 50.0, 0.0)];
        let lure = [Vec2::new(LURE_RADIUS + 50.0, 0.0)];
        assert_eq!(steering(Vec2::ZERO, OPEN, &far, &lure, CAREFUL), Vec2::ZERO);
    }

    #[test]
    fn the_pilot_moves_away_from_threats_and_nearer_ones_matter_more() {
        let stick = steering(Vec2::ZERO, OPEN, &[threat(60.0, 0.0)], &[], CAREFUL);
        assert!(stick.x < -0.8, "{stick}");
        assert!((stick.length() - 1.0).abs() < 1e-4);

        // A close threat on the left outweighs a distant one on the right.
        let both = [threat(-40.0, 0.0), threat(200.0, 0.0)];
        assert!(steering(Vec2::ZERO, OPEN, &both, &[], CAREFUL).x > 0.5);
        // Even a threat exactly on top of the pilot produces a direction.
        assert!(steering(Vec2::ZERO, OPEN, &[threat(0.0, 0.0)], &[], CAREFUL).length() > 0.9);
    }

    #[test]
    fn walls_push_inward_and_pickups_attract_when_safe() {
        let near_east = Vec2::new(OPEN.x - 20.0, 0.0);
        assert!(steering(near_east, OPEN, &[], &[], CAREFUL).x < -0.9);
        let near_south = Vec2::new(0.0, -OPEN.y + 20.0);
        assert!(steering(near_south, OPEN, &[], &[], CAREFUL).y > 0.9);

        let stick = steering(Vec2::ZERO, OPEN, &[], &[Vec2::new(0.0, 100.0)], CAREFUL);
        assert!(stick.y > 0.9, "{stick}");
        // A threat between the pilot and a pickup wins.
        let stick = steering(
            Vec2::ZERO,
            OPEN,
            &[threat(0.0, 30.0)],
            &[Vec2::new(0.0, 100.0)],
            CAREFUL,
        );
        assert!(stick.y < 0.0, "{stick}");
    }

    #[test]
    fn a_bold_pilot_ignores_what_a_careful_one_flees() {
        let bold = MovementStyle::Bold.caution().unwrap();
        let nearby = [threat(0.0, 50.0)];
        let pickup = [Vec2::new(0.0, 100.0)];
        assert!(steering(Vec2::ZERO, OPEN, &nearby, &pickup, CAREFUL).y < 0.0);
        assert!(steering(Vec2::ZERO, OPEN, &nearby, &pickup, bold).y > 0.9);
        // It still steps away from contact.
        assert!(steering(Vec2::ZERO, OPEN, &[threat(0.0, 30.0)], &[], bold).y < -0.8);
    }
}
