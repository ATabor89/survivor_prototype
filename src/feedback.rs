//! Bounded presentation effects driven by actual damage; no gameplay collision.
use crate::{combat::DamageApplied, resources::GameState, run::RunEntity, GameplaySets};
use bevy::prelude::*;

const MAX_PARTICLES: usize = 96;

#[derive(Component)]
struct HitFlash {
    original: Color,
    remaining: f32,
}

#[derive(Component)]
pub(crate) struct ImpactParticle {
    velocity: Vec2,
    remaining: f32,
    duration: f32,
}

pub struct FeedbackPlugin;
impl Plugin for FeedbackPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (animate_feedback, show_damage)
                .chain()
                .after(GameplaySets::Combat)
                .before(GameplaySets::Cleanup)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

fn show_damage(
    mut commands: Commands,
    mut damage: MessageReader<DamageApplied>,
    mut sprites: Query<(&mut Sprite, Option<&mut HitFlash>)>,
    particles: Query<(), With<ImpactParticle>>,
) {
    let mut budget = MAX_PARTICLES.saturating_sub(particles.iter().count());
    let mut flashed: bevy::platform::collections::HashSet<Entity> = Default::default();
    for hit in damage.read() {
        if let Some((mut sprite, flash)) = sprites
            .get_mut(hit.target)
            .ok()
            .filter(|_| flashed.insert(hit.target))
        {
            if let Some(mut flash) = flash {
                flash.remaining = 0.09;
            } else {
                commands.entity(hit.target).insert(HitFlash {
                    original: sprite.color,
                    remaining: 0.09,
                });
            }
            sprite.color = if hit.is_player {
                Color::srgb(1.0, 0.3, 0.25)
            } else {
                Color::srgb(2.2, 2.0, 1.8)
            };
        }
        let count = (if hit.lethal { 5 } else { 2 }).min(budget);
        budget -= count;
        let duration = if hit.lethal { 0.28 } else { 0.16 };
        for index in 0..count {
            let angle = index as f32 * std::f32::consts::TAU / count as f32;
            let color = if hit.is_player {
                Color::srgb(1.0, 0.35, 0.25)
            } else if hit.lethal {
                Color::srgb(0.85, 0.6, 0.95)
            } else {
                Color::srgb(1.0, 0.85, 0.55)
            };
            commands.spawn((
                RunEntity,
                ImpactParticle {
                    velocity: Vec2::from_angle(angle) * if hit.lethal { 90.0 } else { 50.0 },
                    remaining: duration,
                    duration,
                },
                Sprite::from_color(color, Vec2::splat(if hit.lethal { 5.0 } else { 3.0 })),
                Transform::from_translation(hit.position.extend(5.0)),
            ));
        }
    }
}

fn animate_feedback(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mut flashes: Query<(Entity, &mut HitFlash, &mut Sprite), Without<ImpactParticle>>,
    mut particles: Query<
        (Entity, &mut ImpactParticle, &mut Transform, &mut Sprite),
        Without<HitFlash>,
    >,
) {
    let dt = time.delta_secs();
    for (entity, mut flash, mut sprite) in &mut flashes {
        flash.remaining -= dt;
        if flash.remaining <= 0.0 {
            sprite.color = flash.original;
            commands.entity(entity).remove::<HitFlash>();
        }
    }
    for (entity, mut particle, mut transform, mut sprite) in &mut particles {
        particle.remaining -= dt;
        if particle.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation += (particle.velocity * dt).extend(0.0);
        sprite
            .color
            .set_alpha(particle.remaining / particle.duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        combat::DamageEvent,
        components::{Enemy, Health},
        tests::{count, game, step},
    };

    #[test]
    fn simultaneous_hits_restore_original_tint_and_do_not_exceed_particle_budget() {
        let mut app = game();
        let original = Color::srgb(0.3, 0.7, 0.9);
        let enemy = app
            .world_mut()
            .spawn((
                Enemy {
                    speed: 0.0,
                    experience_value: 0,
                },
                Health {
                    current: 1000,
                    maximum: 1000,
                },
                Sprite::from_color(original, Vec2::splat(40.0)),
                Transform::from_xyz(500.0, 500.0, 0.0),
            ))
            .id();
        for _ in 0..100 {
            app.world_mut().write_message(DamageEvent {
                target: enemy,
                amount: 1,
                source: None,
            });
        }
        step(&mut app, 1);
        assert_eq!(count::<ImpactParticle>(&mut app), MAX_PARTICLES);
        assert_eq!(
            app.world().get::<HitFlash>(enemy).unwrap().original,
            original
        );
        step(&mut app, 20);
        assert!(app.world().get::<HitFlash>(enemy).is_none());
        assert_eq!(app.world().get::<Sprite>(enemy).unwrap().color, original);
        assert_eq!(count::<ImpactParticle>(&mut app), 0);
    }

    #[test]
    fn feedback_pauses_and_cleans_up_with_run() {
        let mut app = game();
        let target = crate::tests::player(&mut app);
        app.world_mut().write_message(DamageApplied {
            target,
            amount: 5,
            is_player: true,
            position: Vec2::ZERO,
            lethal: false,
        });
        step(&mut app, 1);
        let remaining = app
            .world_mut()
            .query::<&ImpactParticle>()
            .iter(app.world())
            .next()
            .unwrap()
            .remaining;
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Paused);
        step(&mut app, 30);
        assert_eq!(
            app.world_mut()
                .query::<&ImpactParticle>()
                .iter(app.world())
                .next()
                .unwrap()
                .remaining,
            remaining
        );
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::MainMenu);
        step(&mut app, 2);
        assert_eq!(count::<ImpactParticle>(&mut app), 0);
    }
}
