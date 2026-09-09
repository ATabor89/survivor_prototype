# Occult weapon research: first discussion draft

Research date: 2026-09-09. Proposals for discussion; no gameplay changes or final names are approved by this document.

Following the subsequent design discussion, first-pass implementations of all eight proposals are documented in [the occult arsenal guide](OCCULT-ARSENAL.md). This document preserves their research context and earlier proposal details.

## Direction

The setting treats all magick as real: different traditions provide different interfaces to the same reality. The most useful design move is to adapt an operation, instrument, or relationship into gameplay. Inscription, circulation, invocation, repetition, and correspondence offer more identity than assigning a different projectile color to every tradition.

Historical/source notes below describe texts, objects, or practitioners' accounts. All combat effects, targeting rules, upgrades, and flavor lines are original proposals. Evidence that a practice is documented is not evidence for our fictional combat behavior.

The original `project-summary.md` and `magic-dashboard-refined.tsx` already proposed angelic letters, planetary forces, mantra attacks, and interconnected diagrams. This draft develops those seeds into distinct weapons. It does not replace the broader village/expedition vision or turn the entire tradition catalogue into an immediate implementation list.

## Organizing the traditions

Keep a weapon's source tradition separate from its technique and battlefield role. For example, a Solomonic implement can use inscription and attack in an arc; an alchemical implement can use circulation and attack with returning projectiles. Shared targeting, collision, and stat behavior remain reusable.

For authoring, record a specific text, object, author, or lineage when possible. Use Solomonic, Enochian, alchemical, planetary, and other family labels for navigation, with room for overlapping influences. They need not be mutually exclusive classes or confer automatic same-family bonuses.

Refine the older umbrella labels during detailed design. Keep Jewish Kabbalah, Christian Cabala, and Hermetic Qabalah separately identifiable. Likewise, the mantra/yantra source used here concerns Tantric practice; it should not be filed indiscriminately as a description of all Vedic or Hindu practice. A fictional synthesis can be intentional, with its ingredients recorded.

## The existing arsenal

**Inscribed Needles have a useful possible connection.** The *Key of Solomon* includes a consecrated needle and other iron instruments; needles can inscribe ritual materials and attach pentacles to clothing. That gives the weapon an implement to adapt, without claiming historical practitioners used homing needle volleys. Keep the existing neutral name unless we choose to establish a specific Solomonic origin. [Peterson's annotated edition, especially Book II, chapter XIX](https://www.esotericarchives.com/solomon/ksol.htm).

**Magick Circle remains one weapon.** Its existing compositional patterns can remain the game's fictional ceremonial practice. Other traditions using diagrams do not have to inherit Circle's overlap system. This avoids making every diagram weapon the same mechanic.

**Alchemical Vessel can remain a straightforward preparation.** Its thrown burst already fills a useful role. More alchemical weapons should explore processes rather than require that this one become a miniature crafting simulation.

## Eight new candidates

### 1. Knife of the Art

**Source anchor:** Solomonic ritual implements. The *Key of Solomon* describes knives used to trace the ritual circle. It is a better starting point for this concept than assuming every ritual blade is interchangeable. [Key of Solomon](https://www.esotericarchives.com/solomon/ksol.htm).

**Proposed weapon:** Automatically inscribe a short, bright arc in the last movement direction. The arc cuts nearby enemies and pushes them outward. It is the traced boundary that acts; the player does not need a manual melee button.

**Why take it:** Open a local route through a crowd. Strong in front, with poor rear coverage and no distant reach. Keep facing when stationary so it remains predictable.

**Progression:** Wider arc, longer reach, an additional closely timed stroke. A later upgrade could briefly retain the traced line, but that is optional and should not turn into a permanent collision wall.

**Visual/flavor:** A dark-handled implement draws a chalk-white crescent. “The boundary is drawn. The trespasser is denied.”

**Open decision:** Primarily damage or primarily displacement? Recommend damage with modest displacement so bosses remain useful targets.

### 2. Planetary Talismans

**Source anchor:** Renaissance planetary magic. Agrippa's *Three Books of Occult Philosophy*, Book II, chapter XXII discusses planetary number tables and associated seals and intelligences. [Agrippa, Book II, chapter XXII](https://www.esotericarchives.com/agrippa/agripp2b.htm).

**Proposed weapon:** Inscribed metal discs orbit at a fixed distance, damaging enemies on contact. Their paths leave clear gaps between passes. Orbiting is our visual/gameplay invention, not a historical talisman practice.

**Why take it:** A moving perimeter that rewards maintaining the right distance. Enemies inside the orbit remain dangerous. This differs from standing inside a Circle's territory.

**Progression:** Add discs, improve contact damage and angular speed. Treat orbit radius separately from attack-size scaling: a larger effect should not unexpectedly move the entire defense farther away.

**Visual/flavor:** Small suspended medals with readable metallic silhouettes. “The heavens keep their appointments.”

**Open decision:** One selected planetary association or a collection? Start with one weapon and one coherent source scheme; seven planet types with separate status effects would be an unnecessary first implementation.

### 3. Angelic Inscription

**Source anchor:** Dee's angelic practice uses carefully arranged letters, names, seals, and a ritual table. Peterson documents the Holy Table's construction and letter layout from the manuscripts. This is a specific anchor; the Holy Table is not simply another name for the Great Table. [John Dee's Holy Table and Lamen](https://www.esotericarchives.com/dee/holytab.htm).

**Proposed weapon:** A rectangular inscription appears automatically near a group of enemies. Its rows illuminate in sequence, then its columns discharge in a second pass. The origin stays fixed once placed.

**Why take it:** Delayed coverage of lanes and dense groups. Good against enemies following you into it; a charging enemy can escape before the second pass. Its spatial sequence distinguishes it from Vessel's burst and Circle's persistent effects.

**Progression:** Longer lanes, more active rows, another discharge. The reveal remains readable even with cooldown upgrades.

**Visual/flavor:** A framed tablet of luminous characters with an obvious scan direction. “Read in the order the world was written.”

**Open decision:** Whether the final design follows a particular historical table or uses an explicitly fictional inscription. Avoid placeholder text that purports to translate actual angelic names. The attack sequence is entirely invented.

### 4. Circulating Pelican

**Source anchor:** The pelican is an alchemical vessel whose arms permit continuous circulation. Princeton's exhibition connects surviving imagery with apparatus and chemical operations. [The Philosophers' Glass](https://dpul.princeton.edu/alchemy/feature/the-philosophers-glass).

**Proposed weapon:** A carried flask sends two streams of luminous condensate outward on curved paths, then draws them back. Enemies can be struck once on the outward leg and once on return. Return paths lead toward the player's current position, so ordinary movement changes the swept area.

**Why take it:** Repeated passes through a following crowd. A useful intermediate-range attack with gaps during its cycle, distinct from both orbiters and thrown explosions.

**Progression:** Stream width, outward reach, additional paired emissions. Returning material is automatically recovered; no catch timing or consumable reagent management.

**Visual/flavor:** A twin-armed glass flask; pale vapor condenses into bright droplets on return. “Nothing is spent. Everything returns changed.”

**Open decision:** Independent weapon or a possible future Vessel transformation? Both are viable; do not make implementation depend on an evolution system.

### 5. Forgotten Sigil

**Source anchor:** Spare's *Book of Pleasure* describes sigils formed from simplified letters and discusses removing the desire from conscious attention. This is a source anchor for a modern sigil-magic concept, not a claim that Spare authored every later Chaos Magic convention. [Book of Pleasure, sections on belief and sigils](https://www.tarrdaniel.com/documents/Thelemagick/zos/english/AOS_Book_of_Pleasure.html).

**Proposed weapon:** Automatically leave an inert sigil near your path. It arms when you move sufficiently far away, then erupts when an enemy crosses it. A slower fallback timer allows stationary builds to use it too. Armed status is visually explicit.

**Why take it:** Strong coverage of the space you leave behind. Supports travel and kiting, with less immediate rescue value than Knife. The distance trigger is a fictional metaphor for letting go, not a literal ritual instruction.

**Progression:** More simultaneously active marks, larger eruptions, shorter fallback arming time. Expire unused marks and cap population so the arena does not become unreadable.

**Visual/flavor:** An angular ink mark fades, then burns from beneath. “Leave the intention behind. Let it find its own way.”

**Open decision:** Is moving away satisfying or merely a chore? This is the highest-risk movement rule in the shortlist; compare it against a simple timed mine before committing.

### 6. Bound Intention

**Source anchor:** Chaos Magic servitors. Carroll's course describes a constructed agency given a task, identity, and sometimes a material representation. This offers a different fictional relationship from contacting a pre-existing angel or spirit. [Carroll, Arcanorium College, module on Evocation](https://www.specularium.org/arcanoriumcollege).

**Proposed weapon:** A small created familiar automatically selects a durable nearby enemy, pursues it, and attacks repeatedly until the enemy dies or leaves its operating radius. It then selects another. The player issues no commands.

**Why take it:** Persistent elite pressure while the rest of the arsenal handles crowds. Its limited retargeting speed and small target count are the tradeoffs.

**Progression:** Attack interval, pursuit range, and eventually a second familiar. Avoid a fragile pet health bar and resurrection chore in the first pass.

**Visual/flavor:** A folded paper or wax body animated by a single custom sigil. “One purpose. No distractions.”

**Open decision:** Keep its name visible as an individual creation? That could give players attachment without requiring multiple playable characters or a pet progression subsystem.

### 7. Emanation Lattice

**Source anchor:** Kabbalistic diagram traditions. The Ilanot Project studies richly varied tree diagrams as expressions of cosmology and objects of practice. The source supports interconnected structure; it does not describe a combat skill tree. [Maps of God / Ilanot Project](https://www.ilanot.org/pages?page=about).

**Proposed weapon:** A brief branching figure unfolds toward nearby enemies. A pulse travels along its paths, dividing at junctions to reach several targets. The layout is automatic, temporary, and bounded.

**Why take it:** Reach through a crowd and distribute damage among nearby groups. Divide a fixed attack budget among branches so additional targets do not automatically multiply single-target damage.

**Progression:** Additional endpoints, longer connections, faster traversal. Maintain a clear primary path and limited forks.

**Visual/flavor:** Points of light joined by ink-thin luminous paths. “What descends is divided, but not diminished.”

**Open decision:** Which lineage and diagram should inform the final form? This needs a narrower second research pass before assigning named sefirot or sacred text. It is currently the loosest historical-to-mechanical adaptation in the shortlist.

### 8. Resonant Recitation

**Source anchor:** Mantra and yantra in Tantric ritual. The British Museum's account describes sound and diagram as related ritual forms, with particular religious associations. [British Museum, Brass Tantric yantra](https://artsandculture.google.com/asset/brass-tantric-yantra/AQFvRm-m55quVw?hl=en).

**Proposed weapon:** Automatic recitation sends concentric pulses outward from the player. Every few pulses, a visibly anticipated accent produces stronger displacement. Movement remains free, and no rhythm input is required.

**Why take it:** Predictable, intermittent breathing room against encirclement. Broad reach, modest boss damage, and clear downtime distinguish it from Circle and orbiting talismans.

**Progression:** Pulse reach, width, damage, and cadence. Avoid upgrades that eliminate every gap and make crowd contact irrelevant.

**Visual/flavor:** Rings expand with a repeated geometric accent. “A sound repeated until the world answers.”

**Open decision:** Select a particular textual/traditional basis or write a fictional recitation. The pulse mechanic can be tested with nonverbal audio before choosing actual words; neither the name nor the damage should imply that all mantras are interchangeable sonic attacks.

## Comparison and suggested first batch

| Candidate | Primary role | Player influence | Main weakness | Initial priority |
| --- | --- | --- | --- | --- |
| Knife of the Art | Immediate frontal clearance | Last movement direction | Flanks and distant enemies | High |
| Planetary Talismans | Moving contact perimeter | Spacing and movement | Inside the orbit / gaps | High |
| Angelic Inscription | Delayed lane coverage | Lead enemies through fixed placement | Fast targets escape | High |
| Circulating Pelican | Outward/return crowd damage | Moving return origin | Cyclic downtime | Medium |
| Forgotten Sigil | Rearward traps | Leave marks behind | Delayed assistance | Experimental |
| Bound Intention | Elite pressure | Position within operating range | Limited crowd coverage | Medium |
| Emanation Lattice | Branching group damage | Group proximity | Damage divided among targets | Research further |
| Resonant Recitation | Periodic encirclement relief | Time movement around pulses | Weak sustained boss damage | Medium |

The first three supply the previously discussed arc, orbit, and delayed inscription roles. With the existing three weapons, that makes six, which still does not create weapon-slot scarcity. A subsequent seventh candidate should be selected deliberately: Bound Intention adds autonomous elite targeting; Forgotten Sigil adds a more experimental movement incentive. The complete eight-candidate list is an option pool, not a promise to implement all eight.

## Further uses of the research

Some practices belong in exploration, passive items, or the future hub as well as combat. Dee's scrying apparatus is a strong starting point for revealing hidden things; a manuscript or protective implement could supply an item identity. The British Museum documents wax seals associated with his scrying setup while explicitly qualifying their provenance. [British Museum, magical disc](https://www.britishmuseum.org/collection/object/H_1838-1232-90-a). Any hidden-object sensing would be our game adaptation.

The user’s original Abramelin interest deserves a separate close reading before we turn a Holy Guardian Angel relationship into an ordinary disposable summon. That is a design reservation, not a prohibition: it may be particularly valuable to the larger progression story.

Possible future evolutions should change behavior in a way grounded in the particular weapon: a returning stream might become a closed circulation network, or isolated marks might become a linked inscription. These are directions to discuss, not approved recipes, catalyst-consumption rules, or historical claims.

## Research limits and next decisions

This is a focused concept-development survey, not a comprehensive history or a final terminology review. Sources include annotated primary texts, museum/university descriptions, and a practitioner's account of his own system. Manuscript differences matter; a modern edition is not a timeless consensus. No source illustrations, scans, sacred-language text, or audio were copied into game assets.

Most useful feedback: which fantasies feel right, how closely final terminology should follow a specific source, and whether movement-linked mechanics such as Forgotten Sigil appeal. We can then develop the selected concepts into concrete level progressions, stat interactions, visuals, and playtest criteria.
