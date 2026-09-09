use crate::combat::DamageEvent;
use crate::components::{AreaMultiplier, CooldownReduction, DamageMultiplier, Enemy, Player};
use crate::death::MarkedForDeath;

use crate::resources::GameState;
use crate::weapons::magick_circle::{
    apply_magick_circle_weapon_upgrades, spawn_magick_circle, spawn_magick_circle_attack,
    MagickCircle, PatternType,
};
use crate::weapons::overlap::{overlap_detection_system, ActiveOverlaps, EnemyCircleHits};
use crate::weapons::weapon_upgrade::{
    apply_common_weapon_upgrades, update_weapon_level, WeaponUpgradeConfig,
};
use crate::GameplaySets;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;
use std::time::Duration;
use strum::EnumIter;

pub mod arsenal;
pub mod effects;
pub mod inscriptions;
pub mod magick_circle;
pub mod mobile;
pub mod rituals;
use effects::{CircleEffect, PULL_SPEED_CAP, ROOT_LINGER_SECONDS};
pub mod overlap;
pub mod projectiles;
pub mod weapon_upgrade;

/// Plugin to register all weapon-related systems
pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeaponUpgradeConfig>()
            .add_plugins((
                rituals::RitualWeaponsPlugin,
                inscriptions::InscriptionWeaponsPlugin,
                mobile::MobileWeaponsPlugin,
            ))
            .init_resource::<EnemyCircleHits>()
            .init_resource::<ActiveOverlaps>()
            .add_message::<AddWeaponEvent>()
            .add_message::<BindingEvent>()
            .add_systems(
                Update,
                (
                    update_weapon_level,
                    (
                        apply_common_weapon_upgrades,
                        apply_magick_circle_weapon_upgrades,
                    )
                        .after(update_weapon_level),
                )
                    .before(GameplaySets::Weapons),
            )
            .add_systems(
                Update,
                (
                    setup_player_inventory,
                    handle_new_weapons,
                    weapon_firing_system,
                    projectiles::resolve_projectiles,
                    invocation_cleanup_system,
                    area_effect_system,
                    magick_circle::animate_manifestation_charge,
                    overlap_detection_system,
                    handle_binding_events,
                    binding_expiration_system,
                    protection_system,
                    attack_lifetime_system,
                    orbital_movement_system,
                )
                    .chain()
                    .in_set(GameplaySets::Weapons)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Core weapon type identifier
#[derive(Debug, Component)]
pub struct WeaponMeta {
    pub weapon_type: WeaponType,
    pub level: u32,
}

// Component to define what weapon a player starts with
#[derive(Component)]
pub struct StartingWeapon(pub WeaponType);

// Event for when a new weapon should be added
#[derive(Message)]
pub struct AddWeaponEvent {
    pub player: Entity,
    pub weapon_type: WeaponType,
}

#[derive(Message)]
pub struct BindingEvent {
    pub target: Entity,
    pub strength: f32, // How strongly they're held
    pub source: Entity,
}

// This runs once on player spawn and handles the starting weapon
pub fn setup_player_inventory(
    query: Query<(Entity, &StartingWeapon), Added<Player>>,
    mut add_weapon_event: MessageWriter<AddWeaponEvent>,
) {
    for (player_entity, starting_weapon) in query.iter() {
        add_weapon_event.write(AddWeaponEvent {
            player: player_entity,
            weapon_type: starting_weapon.0,
        });
    }
}

// This system only runs when new weapons are added
pub fn handle_new_weapons(
    mut commands: Commands,
    mut events: MessageReader<AddWeaponEvent>,
    weapons: Query<(&ChildOf, &WeaponMeta)>,
    players: Query<(), With<Player>>,
) {
    let mut owned: HashMap<Entity, Vec<WeaponType>> = HashMap::default();
    for (parent, meta) in &weapons {
        owned
            .entry(parent.parent())
            .or_default()
            .push(meta.weapon_type);
    }
    for event in events.read() {
        if !players.contains(event.player) {
            continue;
        }
        let types = owned.entry(event.player).or_default();
        if types.contains(&event.weapon_type) || types.len() >= crate::inventory::MAX_WEAPONS {
            continue;
        }
        spawn_weapon(&mut commands, event.player, event.weapon_type);
        types.push(event.weapon_type);
    }
}

#[derive(
    Debug,
    Component,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    EnumIter,
    Deserialize,
    Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum WeaponType {
    MagickCircle,
    InscribedNeedles,
    AlchemicalVessel,
    KnifeOfTheArt,
    PlanetaryTalismans,
    AngelicInscription,
    CirculatingPelican,
    ForgottenSigil,
    BoundIntention,
    EmanationLattice,
    ResonantRecitation,
}

impl std::fmt::Display for WeaponType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MagickCircle => write!(f, "Magick Circle"),
            Self::InscribedNeedles => write!(f, "Inscribed Needles"),
            Self::AlchemicalVessel => write!(f, "Alchemical Vessel"),
            Self::KnifeOfTheArt => write!(f, "Knife of the Art"),
            Self::PlanetaryTalismans => write!(f, "Planetary Talismans"),
            Self::AngelicInscription => write!(f, "Angelic Inscription"),
            Self::CirculatingPelican => write!(f, "Circulating Pelican"),
            Self::ForgottenSigil => write!(f, "Forgotten Sigil"),
            Self::BoundIntention => write!(f, "Bound Intention"),
            Self::EmanationLattice => write!(f, "Emanation Lattice"),
            Self::ResonantRecitation => write!(f, "Resonant Recitation"),
        }
    }
}

impl WeaponType {
    pub const fn id(self) -> &'static str {
        match self {
            Self::MagickCircle => "magick_circle",
            Self::InscribedNeedles => "inscribed_needles",
            Self::AlchemicalVessel => "alchemical_vessel",
            Self::KnifeOfTheArt => "knife_of_the_art",
            Self::PlanetaryTalismans => "planetary_talismans",
            Self::AngelicInscription => "angelic_inscription",
            Self::CirculatingPelican => "circulating_pelican",
            Self::ForgottenSigil => "forgotten_sigil",
            Self::BoundIntention => "bound_intention",
            Self::EmanationLattice => "emanation_lattice",
            Self::ResonantRecitation => "resonant_recitation",
        }
    }

    /// Fictional build categories, not claims about historical practices.
    pub const fn family(self) -> &'static str {
        match self {
            Self::MagickCircle => "Ceremonial",
            Self::InscribedNeedles => "Talismanic",
            Self::AlchemicalVessel => "Alchemy",
            Self::KnifeOfTheArt => "Solomonic",
            Self::PlanetaryTalismans => "Planetary",
            Self::AngelicInscription => "Enochian",
            Self::CirculatingPelican => "Alchemy",
            Self::ForgottenSigil | Self::BoundIntention => "Chaos",
            Self::EmanationLattice => "Kabbalistic inspiration",
            Self::ResonantRecitation => "Mantra inspiration",
        }
    }

    pub const fn summary(self) -> &'static str {
        match self {
            Self::MagickCircle => {
                "Stationary circles compose damage, roots, pulls, and protection."
            }
            Self::InscribedNeedles => {
                "Auto-aimed needles pierce up to three targets; walls stop them."
            }
            Self::AlchemicalVessel => {
                "Throw a vessel toward the nearest foe; it bursts on impact or landing."
            }
            Self::KnifeOfTheArt => "Trace a damaging arc in your last movement direction.",
            Self::PlanetaryTalismans => {
                "Orbiting medals strike a close perimeter; each cast renews their attack."
            }
            Self::AngelicInscription => {
                "Target a foe with a fixed inscription that strikes in rows, then columns."
            }
            Self::CirculatingPelican => {
                "Paired streams curve outward and return, striking on both journeys."
            }
            Self::ForgottenSigil => {
                "Leave automatic traps that arm, trigger on enemies, or erupt at expiry."
            }
            Self::BoundIntention => {
                "An autonomous familiar pursues durable enemies; cooldown controls its attacks."
            }
            Self::EmanationLattice => "A branching pulse divides its damage among nearby enemies.",
            Self::ResonantRecitation => {
                "Expanding pulses clear space; every fourth recitation is stronger."
            }
        }
    }

    pub const fn icon(self) -> usize {
        match self {
            Self::MagickCircle => 0,
            Self::InscribedNeedles => 1,
            Self::AlchemicalVessel => 2,
            Self::KnifeOfTheArt => 12,
            Self::PlanetaryTalismans => 13,
            Self::AngelicInscription => 14,
            Self::CirculatingPelican => 15,
            Self::ForgottenSigil => 16,
            Self::BoundIntention => 17,
            Self::EmanationLattice => 18,
            Self::ResonantRecitation => 19,
        }
    }

    pub const fn level_bonus(self, level: u32) -> &'static str {
        match (self, level) {
            (Self::PlanetaryTalismans, 3 | 5 | 7) => "; +1 orbiting talisman",
            (Self::EmanationLattice, 2 | 4 | 6) => {
                "; +1 maximum branch target (shared damage budget)"
            }
            _ => "",
        }
    }
}

/// Base weapon statistics
#[derive(Component, Clone)]
pub struct WeaponCooldown {
    pub timer: Timer,
    pub base_duration: f32,
    pub cooldown_bonus: i32, // Negative numbers speed it up
}

impl Default for WeaponCooldown {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            base_duration: 1.0,
            cooldown_bonus: 0,
        }
    }
}

impl WeaponCooldown {
    /// Limit breaks may exceed 100% reduction, but firing always has a cadence.
    pub const MIN_SECONDS: f32 = 0.1;

    pub(crate) fn effective_seconds(&self, player_reduction: f32) -> f32 {
        let weapon_factor = ((100.0 + self.cooldown_bonus as f32) / 100.0).max(0.0);
        (self.base_duration * weapon_factor * (1.0 - player_reduction.clamp(0.0, 1.0)))
            .max(Self::MIN_SECONDS)
    }
}

#[derive(Component, Clone, Copy)]
pub struct WeaponDamage {
    pub base_amount: i32,
    pub damage_bonus: i32, // Positive numbers increase damage
}

impl WeaponDamage {
    pub(crate) fn effective_amount(&self, player_multiplier: f32) -> i32 {
        let weapon_factor = ((100.0 + self.damage_bonus as f32) / 100.0).max(0.0);
        (self.base_amount as f32 * weapon_factor * player_multiplier.max(0.0))
            .max(0.0)
            .floor() as i32
    }
}

#[derive(Component)]
pub struct Damage {
    pub amount: i32,
}

#[derive(Component, Clone, Copy)]
pub struct WeaponArea {
    pub base_radius: f32,
    pub area_bonus: i32, // Positive numbers increase area
}

impl WeaponArea {
    pub(crate) fn effective_radius(&self, player_multiplier: f32) -> f32 {
        let weapon_factor = ((100.0 + self.area_bonus as f32) / 100.0).max(0.0);
        (self.base_radius * weapon_factor * player_multiplier.max(0.0)).max(0.0)
    }
}

/// Attack-specific components
#[derive(Component)]
pub struct Attack; // Formerly held an AttackType enum

/// Kept on the attack until damage is resolved so telemetry can attribute hits.
#[derive(Component, Clone, Copy, Debug)]
pub struct AttackSource(pub WeaponType);

#[derive(Component)]
pub struct Lifetime {
    pub timer: Timer,
}

#[derive(Component)]
pub struct Sigil;

#[derive(Component)]
pub struct Orbits {
    pub radius: f32,
    pub speed: f32,
    pub current_angle: f32,
}

#[derive(Component)]
pub struct AreaEffect {
    /// Spawn-relative cadence: new circles always wait for their first effect.
    pub timer: Timer,
}

impl AreaEffect {
    pub fn new(pattern: PatternType) -> Self {
        Self {
            timer: Timer::from_seconds(pattern.effect().interval(), TimerMode::Repeating),
        }
    }
}

/// Spawns a weapon for the player based on weapon type
pub fn spawn_weapon(commands: &mut Commands, player_entity: Entity, weapon_type: WeaponType) {
    match weapon_type {
        WeaponType::MagickCircle => spawn_magick_circle(commands, player_entity),
        WeaponType::InscribedNeedles | WeaponType::AlchemicalVessel => {
            projectiles::spawn_projectile_weapon(commands, player_entity, weapon_type);
        }
        _ => arsenal::spawn_weapon(commands, player_entity, weapon_type),
    }
}

/// Restores an authored loadout through the same stat changes as normal upgrades.
pub(crate) fn grant_configured_weapon(
    world: &mut World,
    player: Entity,
    kind: WeaponType,
    level: u32,
    patterns: &[PatternType],
) -> Result<(), String> {
    use weapon_upgrade::{apply_stat_change, WeaponUpgradeChange};
    if world.get::<Player>(player).is_none() {
        return Err("Weapon owner is not a player".into());
    }
    let config = world.resource::<WeaponUpgradeConfig>();
    if level == 0 || level > config.max_level(kind) {
        return Err(format!(
            "{kind} level must be between 1 and {}",
            config.max_level(kind)
        ));
    }
    let progression: Vec<_> = (1..level)
        .flat_map(|current| config.get_next_upgrades(kind, current))
        .collect();
    let circle_count = 1 + progression
        .iter()
        .flat_map(|spec| &spec.changes)
        .filter(|change| matches!(change, WeaponUpgradeChange::AddCircle { .. }))
        .count();
    if !patterns.is_empty() && (kind != WeaponType::MagickCircle || patterns.len() > 7) {
        return Err(format!(
            "{kind} accepts an explicit circle loadout of 1..=7 patterns, or an empty default list"
        ));
    }
    let owned: Vec<_> = world
        .query::<(&ChildOf, &WeaponMeta)>()
        .iter(world)
        .filter(|(parent, _)| parent.parent() == player)
        .map(|(_, meta)| meta.weapon_type)
        .collect();
    if owned.contains(&kind) {
        return Err(format!("{kind} is already owned"));
    }
    if owned.len() >= crate::inventory::MAX_WEAPONS {
        return Err("Weapon slots are full".into());
    }
    spawn_weapon(&mut world.commands(), player, kind);
    world.flush();
    let entity = world
        .query::<(Entity, &ChildOf, &WeaponMeta)>()
        .iter(world)
        .find(|(_, parent, meta)| parent.parent() == player && meta.weapon_type == kind)
        .map(|(entity, _, _)| entity)
        .ok_or_else(|| format!("Failed to create {kind}"))?;
    let mut query = world.query::<(
        &mut WeaponMeta,
        &mut WeaponDamage,
        &mut WeaponArea,
        &mut WeaponCooldown,
        Option<&mut MagickCircle>,
    )>();
    let (mut meta, mut damage, mut area, mut cooldown, circle) = query
        .get_mut(world, entity)
        .map_err(|error| error.to_string())?;
    for change in progression.iter().flat_map(|spec| &spec.changes) {
        apply_stat_change(change, &mut damage, &mut area, &mut cooldown);
    }
    meta.level = level;
    if let Some(mut circle) = circle {
        circle.patterns = if patterns.is_empty() {
            vec![PatternType::Banishment; circle_count]
        } else {
            patterns.to_vec()
        };
    }
    Ok(())
}

/// System to handle weapon firing logic
#[expect(
    clippy::type_complexity,
    reason = "Auto-targeting excludes dead actors in the ECS query."
)]
pub fn weapon_firing_system(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    // Query player stats
    player_query: Query<
        (
            &CooldownReduction,
            &DamageMultiplier,
            &AreaMultiplier,
            &Transform,
            Option<&arsenal::Facing>,
        ),
        With<Player>,
    >,
    // Query weapons and their stats
    mut weapon_query: Query<(
        Entity,
        &ChildOf,
        &mut WeaponCooldown,
        &WeaponDamage,
        &WeaponArea,
        &WeaponMeta,
    )>,
    // Query specific weapon types for their unique properties
    magick_circle_query: Query<&MagickCircle>,
    targets: Query<(Entity, &Transform), (With<Enemy>, Without<MarkedForDeath>)>,
) {
    for (weapon_entity, parent, mut cooldown, damage, area, weapon) in weapon_query.iter_mut() {
        if let Ok((
            cooldown_reduction,
            damage_multiplier,
            area_multiplier,
            player_transform,
            facing,
        )) = player_query.get(parent.parent())
        {
            let effective_cooldown = cooldown.effective_seconds(cooldown_reduction.percent);

            cooldown
                .timer
                .set_duration(Duration::from_secs_f32(effective_cooldown));
            cooldown.timer.tick(time.delta());

            let effective_damage = damage.effective_amount(damage_multiplier.factor);
            let effective_radius = area.effective_radius(area_multiplier.factor);

            if cooldown.timer.just_finished() {
                match weapon.weapon_type {
                    WeaponType::MagickCircle => {
                        if let Ok(magick_circle) = magick_circle_query.get(weapon_entity) {
                            let Some((&first, remaining)) = magick_circle.patterns.split_first()
                            else {
                                continue;
                            };
                            // First circle always spawns centered
                            spawn_magick_circle_attack(
                                &mut commands,
                                player_transform.translation,
                                effective_damage,
                                effective_radius,
                                first,
                                None, // No offset for first circle
                            );

                            // Additional circles are evenly spaced
                            if !remaining.is_empty() {
                                let angle_step = std::f32::consts::TAU / remaining.len() as f32;
                                for (i, pattern) in remaining.iter().enumerate() {
                                    let angle = angle_step * i as f32;
                                    spawn_magick_circle_attack(
                                        &mut commands,
                                        player_transform.translation,
                                        effective_damage,
                                        effective_radius,
                                        *pattern,
                                        Some(angle),
                                    );
                                }
                            }
                        } else {
                            warn!("Missing MagickCircle component on weapon {weapon_entity:?}");
                        }
                    }
                    WeaponType::InscribedNeedles | WeaponType::AlchemicalVessel => {
                        let origin = player_transform.translation.truncate();
                        if let Some((_, target)) = targets
                            .iter()
                            .filter(|(_, transform)| {
                                transform.translation.truncate().distance_squared(origin)
                                    <= 720.0_f32.powi(2)
                            })
                            .min_by(|(entity_a, a), (entity_b, b)| {
                                a.translation
                                    .truncate()
                                    .distance_squared(origin)
                                    .total_cmp(&b.translation.truncate().distance_squared(origin))
                                    .then_with(|| entity_a.cmp(entity_b))
                            })
                        {
                            projectiles::spawn_projectile(
                                &mut commands,
                                origin,
                                target.translation.truncate(),
                                weapon.weapon_type,
                                effective_damage,
                                effective_radius,
                            );
                        }
                    }
                    kind => {
                        let origin = player_transform.translation.truncate();
                        let target = targets
                            .iter()
                            .filter(|(_, t)| {
                                t.translation.truncate().distance_squared(origin)
                                    <= 720.0_f32.powi(2)
                            })
                            .min_by(|(a, ta), (b, tb)| {
                                ta.translation
                                    .truncate()
                                    .distance_squared(origin)
                                    .total_cmp(&tb.translation.truncate().distance_squared(origin))
                                    .then_with(|| a.cmp(b))
                            })
                            .map(|(entity, t)| (entity, t.translation.truncate()));
                        arsenal::cast(
                            &mut commands,
                            arsenal::Cast {
                                owner: parent.parent(),
                                weapon: weapon_entity,
                                kind,
                                origin,
                                facing: facing.map_or(Vec2::X, |f| f.0),
                                target,
                                damage: effective_damage,
                                radius: effective_radius,
                                level: weapon.level,
                            },
                        );
                    }
                }
            }
        } else {
            warn!("Missing player stats for weapon {weapon_entity:?}");
        }
    }
}

/// System to manage area effects for weapons that have them.
/// Populates `EnemyCircleHits` as a side effect so `overlap_detection_system`
/// can identify enemies inside multiple circles.
#[expect(
    clippy::too_many_arguments,
    reason = "Bevy injects separate queries, messages, and resources to track ECS access."
)]
pub fn area_effect_system(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mut effect_query: Query<
        (Entity, &mut AreaEffect, &Damage, &PatternType, &Transform),
        With<Attack>,
    >,
    mut damage_events: MessageWriter<DamageEvent>,
    mut binding_events: MessageWriter<BindingEvent>,
    context_query: ReadRapierContext,
    enemy_query: Query<&Transform, (With<Enemy>, Without<MarkedForDeath>)>,
    breakables: Query<(), (With<crate::loot::Breakable>, Without<MarkedForDeath>)>,
    mut enemy_circle_hits: ResMut<EnemyCircleHits>,
) {
    let rapier_context = context_query
        .single()
        .expect("Expected one initialized Rapier context");

    // Clear previous frame's data; this resource is also read by overlap_detection_system
    enemy_circle_hits.hits.clear();
    let mut ticking_circles: HashMap<Entity, u32> = HashMap::default();

    // First pass: collect all circle effects affecting each enemy
    for (circle_entity, mut area_effect, damage, pattern, circle_transform) in
        effect_query.iter_mut()
    {
        area_effect.timer.tick(time.delta());
        let ticks = area_effect.timer.times_finished_this_tick();
        if ticks > 0 {
            ticking_circles.insert(circle_entity, ticks);
        }
        let circle_pos = circle_transform.translation.truncate();

        // Spatial membership is continuous; only effect application is timed.
        for (collider1, collider2, intersecting) in
            rapier_context.intersection_pairs_with(circle_entity)
        {
            if !intersecting {
                continue;
            }

            let enemy_entity = if collider1 == circle_entity {
                collider2
            } else {
                collider1
            };

            if enemy_query.contains(enemy_entity) {
                enemy_circle_hits
                    .hits
                    .entry(enemy_entity)
                    .or_default()
                    .push((circle_entity, *pattern, circle_pos));
            } else if ticks > 0 && breakables.contains(enemy_entity) {
                // Props participate in Rapier intersections and damage, but never
                // in enemy-only root, pull, overlap, kill, or XP behavior.
                if let CircleEffect::Damage { multiplier, .. } = pattern.effect() {
                    damage_events.write(DamageEvent {
                        target: enemy_entity,
                        amount: ((damage.amount as f32 * multiplier).round() as i32)
                            .saturating_mul(i32::try_from(ticks).unwrap_or(i32::MAX)),
                        source: Some(circle_entity),
                    });
                }
            }
        }
    }

    // Second pass: apply per-pattern effects for each enemy
    for (enemy_entity, affecting_circles) in enemy_circle_hits.hits.iter_mut() {
        // Stable accumulation order, independent of query/archetype iteration.
        affecting_circles.sort_unstable_by_key(|(entity, _, _)| *entity);
        let enemy_pos = enemy_query
            .get(*enemy_entity)
            .unwrap()
            .translation
            .truncate();
        let mut pull_velocity = Vec2::ZERO;
        let mut has_pull = false;
        for (circle_entity, pattern, circle_pos) in affecting_circles {
            if let CircleEffect::Pull { speed } = pattern.effect() {
                let offset = *circle_pos - enemy_pos;
                // Slow near the center to avoid overshooting and oscillating.
                pull_velocity += offset.normalize_or_zero()
                    * speed.min(offset.length() / time.delta_secs().max(0.0001));
                has_pull = true;
                continue;
            }
            let Some(&ticks) = ticking_circles.get(circle_entity) else {
                continue;
            };
            match pattern.effect() {
                CircleEffect::Damage { multiplier, .. } => {
                    if let Ok((_, _, damage, _, _)) = effect_query.get(*circle_entity) {
                        damage_events.write(DamageEvent {
                            target: *enemy_entity,
                            amount: ((damage.amount as f32 * multiplier).round() as i32)
                                .saturating_mul(i32::try_from(ticks).unwrap_or(i32::MAX)),
                            source: Some(*circle_entity),
                        });
                    }
                }
                CircleEffect::Root { strength } => {
                    binding_events.write(BindingEvent {
                        target: *enemy_entity,
                        strength,
                        source: *circle_entity,
                    });
                }
                CircleEffect::Pull { .. } | CircleEffect::Ward { .. } => {}
            }
        }
        if has_pull {
            commands.entity(*enemy_entity).insert(InvocationPull {
                velocity: pull_velocity.clamp_length_max(PULL_SPEED_CAP),
            });
        }
    }
}

#[derive(Component)]
pub struct BindingEffect {
    pub strength: f32,
    pub source: Entity,
    pub timer: Timer,
}

pub fn handle_binding_events(
    mut commands: Commands,
    mut binding_events: MessageReader<BindingEvent>,
    mut bindings: Query<&mut BindingEffect>,
) {
    let mut roots: HashMap<Entity, (f32, Entity)> = HashMap::default();
    for event in binding_events.read() {
        let root = roots
            .entry(event.target)
            .or_insert((event.strength, event.source));
        if event.strength > root.0 || (event.strength == root.0 && event.source < root.1) {
            *root = (event.strength, event.source);
        }
    }
    for (target, (strength, source)) in roots {
        if let Ok(mut binding) = bindings.get_mut(target) {
            if strength >= binding.strength {
                binding.strength = strength;
                binding.source = source;
            }
            binding.timer.reset();
        } else {
            commands.entity(target).insert(BindingEffect {
                strength,
                source,
                timer: Timer::from_seconds(ROOT_LINGER_SECONDS, TimerMode::Once),
            });
        }
    }
}

/// Ticks binding timers and removes expired binding effects so enemies
/// regain movement after leaving a Binding circle.
pub fn binding_expiration_system(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mut query: Query<(Entity, &mut BindingEffect)>,
) {
    for (entity, mut binding) in query.iter_mut() {
        binding.timer.tick(time.delta());
        if binding.timer.is_finished() {
            commands.entity(entity).remove::<BindingEffect>();
        }
    }
}

/// Applied to enemies inside an Invocation circle. Cleared every frame
/// by `invocation_cleanup_system` and re-applied by `area_effect_system`
/// while the enemy remains inside the circle.
#[derive(Component)]
pub struct InvocationPull {
    pub velocity: Vec2,
}

/// Clears all `InvocationPull` components so stale pulls are removed.
/// Must run BEFORE `area_effect_system` which re-inserts them for
/// enemies still inside Invocation circles.
pub fn invocation_cleanup_system(
    mut commands: Commands,
    query: Query<Entity, With<InvocationPull>>,
) {
    for entity in query.iter() {
        commands.entity(entity).remove::<InvocationPull>();
    }
}

/// Applied to the player when they're standing inside a Protection circle.
/// Reduces incoming damage by `damage_reduction` (0.0–1.0).
#[derive(Component)]
pub struct ProtectionBuff {
    pub damage_reduction: f32,
}

/// Checks whether the player is inside any active Protection circle and
/// inserts or removes `ProtectionBuff` accordingly.
/// Uses Rapier intersection pairs (circles are GROUP_3 filtering for
/// GROUP_1 | GROUP_2, player is GROUP_1 filtering for GROUP_3).
pub fn protection_system(
    mut commands: Commands,
    context_query: ReadRapierContext,
    player_query: Query<Entity, With<Player>>,
    protection_circles: Query<(Entity, &PatternType), With<Attack>>,
) {
    let Ok(player_entity) = player_query.single() else {
        return;
    };
    let rapier_context = context_query
        .single()
        .expect("Expected one initialized Rapier context");

    let mut reduction: f32 = 0.0;

    for (circle_entity, pattern) in protection_circles.iter() {
        let CircleEffect::Ward {
            reduction: circle_reduction,
        } = pattern.effect()
        else {
            continue;
        };
        for (collider1, collider2, intersecting) in
            rapier_context.intersection_pairs_with(circle_entity)
        {
            if !intersecting {
                continue;
            }
            let other = if collider1 == circle_entity {
                collider2
            } else {
                collider1
            };
            if other == player_entity {
                reduction = reduction.max(circle_reduction);
                break;
            }
        }
    }

    if reduction > 0.0 {
        commands.entity(player_entity).insert(ProtectionBuff {
            damage_reduction: reduction,
        });
    } else {
        commands.entity(player_entity).remove::<ProtectionBuff>();
    }
}

/// Handles lifetime of attacks and marks them for death when expired
#[expect(
    clippy::type_complexity,
    reason = "The Bevy query filters expired attacks without revisiting entities already marked for death."
)]
pub fn attack_lifetime_system(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mut query: Query<(Entity, &mut Lifetime), (With<Attack>, Without<MarkedForDeath>)>,
) {
    for (entity, mut lifetime) in query.iter_mut() {
        lifetime.timer.tick(time.delta());
        if lifetime.timer.is_finished() {
            // First check if the entity still exists
            if commands.get_entity(entity).is_ok() {
                commands.entity(entity).insert(MarkedForDeath);
            }
        }
    }
}

fn orbital_movement_system(
    time: Res<Time<Virtual>>,
    mut query: Query<(&mut Transform, &mut Orbits)>,
) {
    for (mut transform, mut orbits) in &mut query {
        orbits.current_angle = (orbits.current_angle + orbits.speed * time.delta_secs())
            .rem_euclid(std::f32::consts::TAU);

        let offset = Vec2::new(
            orbits.current_angle.cos() * orbits.radius,
            orbits.current_angle.sin() * orbits.radius,
        );

        // Since we're using parent-relative transforms, this will work
        // whether the entity is parented to the player or to a circle
        transform.translation = Vec3::new(offset.x, offset.y, transform.translation.z);
    }
}
