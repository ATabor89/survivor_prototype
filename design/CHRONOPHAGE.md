# Chronophage — deferred enemy concept

Recorded September 20, 2026. **Concept only; not scheduled for implementation.**
All example timings are illustrative, not locked balance values.

## Core idea

A chronophage eats the player's remaining opportunity to develop their build.
When it successfully damages the player, the elapsed run clock instantly jumps
forward. The wave state and its displayed information update to match the new
time, but scheduled bosses crossed by the jump are missed, along with the chests
they would have dropped. Those missed bosses do not spawn afterward as catch-up
encounters merely because the clock jumped. A possible time-recovery mechanic
is recorded below; its interaction with missed encounters remains undecided.

For example, a hit at **5:45** might consume **30 seconds**, moving the run clock
immediately to **6:15**. A boss scheduled for **6:00** is skipped. The player is now
fighting the wave appropriate to 6:15, without having had those 30 seconds to
earn experience, collect resources, or improve their build.

The threat compounds: later waves arrive against an underdeveloped build, while
missed boss chests remove valuable upgrade opportunities. Reduced experience is
the result of lost fighting time; the concept does not call for subtracting XP
already earned.

## Identity and availability

- **Very recognizable.** Players must be able to identify this enemy and learn
  what makes being hit by it unusually costly, including in a crowded fight.
  Its final appearance, silhouette, animation, and sound are still to be designed.
- **Rare and conditional.** It should not be a routine member of every map's
  enemy pool. Possible conditions include particular maps, rituals the player
  has performed, or enabled modifiers. These are alternatives or possible
  combinations, not a settled unlock rule.
- **Readable time loss.** Presentation should clearly connect the damaging hit
  to the clock jump and changed wave information. Exact feedback is undecided.

## Possible extensions

These are optional ideas to explore, not committed features.

- **Variants:** different chronophages could steal more or less time per
  damaging hit. Amounts and variant identities are undecided; variants should
  remain recognizable as chronophages and communicate their relative threat.
- **Time-recovery grace window:** after a chronophage steals time, killing that
  chronophage quickly enough could return some or all of the time it stole.
  This could turn a damaging hit into an urgent choice between pursuing the
  thief and surviving the surrounding wave. The window length, recoverable
  amount, and whether recovery is partial or complete remain open.

Recovery needs its own rules before implementation: when the grace window
starts, whether further hits refresh it, how multiple thieves track stolen
time, and how the remaining opportunity is communicated. It is also undecided
whether returned time moves the run clock backward or is represented another
way. If the clock rewinds, decide how wave selection responds, whether missed
bosses become available again, and how to prevent duplicate encounters or
rewards when timestamps are crossed again. Returning time does not yet imply
undoing everything that happened after the hit.

## Behavior to preserve when revisiting

- Time is consumed on a hit that actually damages the player.
- Elapsed run time advances instantly; this is a loss of preparation time.
- Ordinary wave selection and wave information reflect the destination time.
- Scheduled reward bosses crossed by the jump are genuinely missed, including
  their potential chest rewards. Do not silently queue them for later.
- The resulting difficulty comes from facing later threats with fewer earned
  resources and upgrades.

## Open questions before implementation

- How much time does each hit consume? Can repeated hits compound without a
  limit, or should there be a cooldown or cap?
- Which maps, rituals, or modifiers permit an appearance, and how rare is it?
- What movement and attack pattern gives players recognizable counterplay?
- What happens when the jump crosses a **required final boss** or the end of a
  **timed-survival stage**? Existing stages use both objective types. The concept
  establishes missed reward bosses but does not yet decide finale behavior.
- How are events exactly at the destination timestamp handled? What happens to
  an encounter already telegraphing or a boss already present?
- Which other scheduled events are skipped, and which must still occur?
- Which timers advance with the run clock, and which retain normal combat time
  (for example, attack cooldowns, status effects, and encounter lifetimes)?
- How should the HUD communicate consumed time and missed encounters? Should
  run results distinguish actual play time from the advanced run clock?

These questions are deliberately unresolved. This note preserves the enemy's
identity and intended penalty without committing to tuning or implementation.

Related context: [combat encounters and map objectives](COMBAT-ENCOUNTERS.md)
and [authored stages, encounters, and rewards](STAGE-CONTENT.md).
