# Combat encounters and map objectives

## Scope and tuning intent

This pass supplies different threats to an accumulated build. With three weapons and six weapon slots, owning all three is expected; starting-weapon choice is not being treated as an exclusive build commitment. Damage, area coverage, and Circle control should each make useful contributions against a mixed crowd. These are playable prototype values, not a finished difficulty curve.

## Map win conditions

`assets/data/stages.ron` owns each stage's objective:

```ron
// The default when omitted. End in victory when duration elapses.
win_condition: TimedSurvival,

// At duration, stop ordinary spawning and warn/spawn this required boss.
// The run clock continues; killing that boss wins the run.
win_condition: DefeatBoss(enemy: "final_warden"),
```

The Courtyard uses the final-boss objective at 10:00. Ash Processional (9:00) and Bell Descent (8:00) retain timed survival, demonstrating both paths. Existing enemies remain when the final encounter begins; the normal wave supply stops. The final boss does not drop a chest immediately before the victory screen. Earlier chest-bearing minibosses still do.

Boss objectives track the required actor and its actual death event. Despawning the actor cannot win a run; a missing required actor is scheduled again. Occupied spawn locations are retried through Rapier queries with a fresh visible warning before arrival. Pauses freeze the warning and all combat. A simultaneous lethal hit to the player takes precedence over victory. Restart clears objective progress and pending encounters.

The enum is an explicit extension point for future authored objectives. No hub progression, endless mode, generic scripting system, or arbitrary quest graph is implied by these two supported conditions.

## Enemy behaviors

Enemy definitions default to `Pursuer`. The new definitions opt into:

- `Charger`: approach within range, telegraph a locked direction, dash, recover. The Horned Penitent introduces it in the first minute.
- `Ranged`: approach into range, telegraph locked shot directions, fire, recover. The Censer Adept fires one shot; the larger Hollow Cantor fires a three-shot spread.
- `Warden`: alternate the shared ranged cycle and charge cycle. The Warden of the Last Seal uses a three-shot volley and a longer charge warning.

Enemy data controls trigger/preferred range, windup, recovery/cooldown, dash speed/duration, projectile speed/lifetime/damage, and volley count/spread. Validation rejects non-finite/invalid values and excessive counts/speeds. A ranged definition can use zero movement speed for a stationary enemy, provided its spawn is authored within useful attack range; the new ordinary caster is mobile so offscreen wave placement remains useful.

Rapier owns body movement, wall contact, and projectile shape casts. Fast dashes enable CCD. Hostile projectiles use native sensor sweeps against the player and world, with finite lifetimes; they do not hit fellow enemies or the player's contact-sensor children. Actual player damage still passes through the common cooldown, Protection, and debug-invulnerability rules. Binding affects voluntary movement; Invocation overrides voluntary velocity. Neither effect is defined as a silence, so attack timers continue while rooted.

## Courtyard pacing

Twenty authored phases use 30-second boundaries for this map; the director still supports arbitrary timestamps. Pursuers remain the large majority of each pool. The schedule introduces charges around 1:00, introduces ordinary casters around 2:30, supplies a ranged miniboss at 4:30, then mixes threats. Lower spawn pressure around boss introductions and at 6:30 provides breathing room without deleting existing enemies. The final two minutes increase density to a 144-enemy ordinary cap and a 40-enemy replenishment minimum, followed by the final boss objective.

Health scaling tops out at 1.6 in the Courtyard. These numbers intentionally leave room to test a small roster; they are not an attempt to reproduce another game's end-of-run saturation. The other two maps retain their existing spawn schedules as comparison stages.

## Feedback, sound, and tools

The old boss stripe was decorative. Bosses now have a dark background and left-anchored yellow fill derived from current/maximum health, including paused debug changes. The existing numeric boss HUD remains.

Resolved `DamageApplied` messages drive short hit flashes and hit/death particles, so blocked hits do not pretend to deal damage. Particle count is bounded at 96, repeated same-frame hits preserve the original sprite tint, and effects pause/reset with the run. These entities have no colliders and do not affect gameplay.

Eight short Kenney CC0 cues use Bevy audio: impact, defeat, player damage, upgrade, treasure, encounter warning, victory, and loss. The mixer limits simultaneous voices and throttles crowded hit/death cues, reserving capacity for important feedback. Mute/volume controls appear on setup, pause, and result screens and inside the persistent Debug menu. Preferences survive run changes for the current application session. Sources, original licenses, and exact file checksums are in `assets/audio/README.md`.

The Debug Run tab can spawn any authored enemy, start the map's final boss encounter, or explicitly bypass the objective with Test Victory. Starting a finale while debug-paused waits for simulation resume. Clear Arena preserves the required final boss. These separate controls let encounter testing use the real objective path while result-screen testing remains quick.

## Next weapon design questions

The next additions should expand coverage patterns and tradeoffs before filling out names. Candidates for a focused design discussion:

| Working concept | Battlefield role | Decision to resolve |
| --- | --- | --- |
| Consecrated blade | Short forward arc, strong through a nearby crowd | Facing-based attack versus automatic nearest-threat orientation |
| Orbiting seals | Reliable close perimeter with openings between passes | Moving protection versus stationary Circle territory |
| Delayed inscription | Marks a location, then strikes after a readable delay | Reward grouping enemies without duplicating Vessel's thrown burst |

These are fictional working concepts, not claims about historical practices or assignments to exclusive traditions. Research and family placement should precede final names and descriptions. The existing worldbuilding contract remains: all traditions work, with different interfaces to reality.

Evolutions remain an open design topic. Decisions include whether catalysts are retained or consumed, whether a transformation releases a slot, whether eligibility requires a particular reward source, and whether transformations alter attack behavior or only its power. No evolution rules or passive consumption are implemented in this pass.

## Verification

Validation on Windows: 89 default-feature tests and 104 dev-tools tests passed,
along with formatting and strict Clippy checks for both configurations. Coverage
includes real Rapier collisions, locked aim, pause/reset, bounded feedback/audio,
boss health presentation, and objective lifecycle failures as well as success.

A native 1920 × 1080 playtest imported the existing three-weapon recipe, started
the final encounter from the paused debug panel, and resumed into its warning.
The clock continued past 10:00; the yellow fill visibly tracked 2,190 and then
810 of 2,400 HP. Ordinary weapon damage killed the boss and opened victory at
10:53. Natural spawning was disabled and invulnerability enabled for this focused
check, so it does not establish full-run difficulty. Setup and result layouts,
mute/unmute controls, and the final-objective HUD were also inspected. Runtime
logs showed no asset, audio, or application errors. Ogg decoding and audio limits
were tested automatically; subjective listening balance remains unverified.
