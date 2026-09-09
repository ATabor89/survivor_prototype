use bevy::prelude::*;
use bevy::platform::collections::{HashMap, HashSet};

use crate::weapons::magick_circle::PatternType;

/// Frame-scoped intermediate data produced by `area_effect_system`.
/// Maps each enemy to the list of circles currently affecting it.
/// Read by `overlap_detection_system` to find enemies in 2+ circles.
#[derive(Resource, Debug, Default)]
pub struct EnemyCircleHits {
    pub hits: HashMap<Entity, Vec<(Entity, PatternType, Vec2)>>,
}

/// A canonical pair of circle entities, ordered so that (A,B) and (B,A)
/// hash to the same key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CirclePair(pub Entity, pub Entity);

impl CirclePair {
    pub fn new(a: Entity, b: Entity) -> Self {
        if a < b {
            Self(a, b)
        } else {
            Self(b, a)
        }
    }
}

/// Describes one overlap between two circles: the pattern types involved
/// and which enemies are inside both circles.
#[derive(Debug, Clone)]
pub struct OverlapInfo {
    pub pattern_a: PatternType,
    pub pattern_b: PatternType,
    /// Enemy entities that are inside BOTH circles of this pair.
    pub enemies_in_overlap: HashSet<Entity>,
}

/// Frame-scoped resource rebuilt every tick by `overlap_detection_system`.
/// Contains all active overlaps (circle pairs that share at least one enemy).
/// Future combo effect systems read from this.
#[derive(Resource, Debug, Default)]
pub struct ActiveOverlaps {
    pub overlaps: HashMap<CirclePair, OverlapInfo>,
}

impl ActiveOverlaps {
    /// Returns all pattern-pair combinations affecting a given enemy.
    pub fn patterns_for_enemy(&self, enemy: Entity) -> Vec<(PatternType, PatternType)> {
        self.overlaps
            .values()
            .filter(|info| info.enemies_in_overlap.contains(&enemy))
            .map(|info| (info.pattern_a, info.pattern_b))
            .collect()
    }

    /// Returns all unique overlapping pattern pairs, regardless of enemy.
    pub fn all_pattern_pairs(&self) -> Vec<(PatternType, PatternType)> {
        self.overlaps
            .values()
            .map(|info| (info.pattern_a, info.pattern_b))
            .collect()
    }
}

/// Component attached to enemies that are inside 2+ circles simultaneously.
/// Rebuilt every frame by `overlap_detection_system`.
#[derive(Component, Debug, Clone)]
pub struct OverlappingPatterns {
    /// Every (PatternType, PatternType) pair this enemy is inside.
    /// For an enemy inside circles A, B, C this contains:
    /// [(A,B), (A,C), (B,C)]
    pub pattern_pairs: Vec<(PatternType, PatternType)>,
    /// The circle entities affecting this enemy.
    pub circle_entities: Vec<Entity>,
}

/// Reads `EnemyCircleHits` (populated by `area_effect_system`) and extracts
/// overlap data: which circle pairs share enemies, and which enemies are in
/// multiple circles. Writes to `ActiveOverlaps` resource and attaches
/// `OverlappingPatterns` components to affected enemies.
pub fn overlap_detection_system(
    mut commands: Commands,
    enemy_circle_hits: Res<EnemyCircleHits>,
    mut active_overlaps: ResMut<ActiveOverlaps>,
    existing_overlap_enemies: Query<Entity, With<OverlappingPatterns>>,
) {
    // Clear previous frame's data
    active_overlaps.overlaps.clear();

    // Remove OverlappingPatterns from all enemies (clean slate each frame)
    for entity in existing_overlap_enemies.iter() {
        commands.entity(entity).remove::<OverlappingPatterns>();
    }

    // For each enemy affected by 2+ circles, extract all unique pairs
    for (enemy_entity, circles) in enemy_circle_hits.hits.iter() {
        if circles.len() < 2 {
            continue;
        }

        let mut pattern_pairs = Vec::new();
        let circle_entities: Vec<Entity> = circles.iter().map(|(e, _, _)| *e).collect();

        // Generate all unique pairs from the circles affecting this enemy
        for i in 0..circles.len() {
            for j in (i + 1)..circles.len() {
                let (circle_a, pattern_a, _) = circles[i];
                let (circle_b, pattern_b, _) = circles[j];

                let pair = CirclePair::new(circle_a, circle_b);
                pattern_pairs.push((pattern_a, pattern_b));

                // Update the global resource
                active_overlaps
                    .overlaps
                    .entry(pair)
                    .and_modify(|info| {
                        info.enemies_in_overlap.insert(*enemy_entity);
                    })
                    .or_insert_with(|| {
                        let mut enemies = HashSet::new();
                        enemies.insert(*enemy_entity);
                        OverlapInfo {
                            pattern_a,
                            pattern_b,
                            enemies_in_overlap: enemies,
                        }
                    });
            }
        }

        // Attach component to this enemy
        commands.entity(*enemy_entity).insert(OverlappingPatterns {
            pattern_pairs,
            circle_entities,
        });
    }

    // Log detected overlaps in debug builds
    #[cfg(debug_assertions)]
    if !active_overlaps.overlaps.is_empty() {
        for (_pair, info) in active_overlaps.overlaps.iter() {
            info!(
                "Overlap detected: {:?} + {:?} affecting {} enemies",
                info.pattern_a,
                info.pattern_b,
                info.enemies_in_overlap.len()
            );
        }
    }
}
