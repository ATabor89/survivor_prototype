import React, { useState } from 'react';
import { Sun, Moon } from 'lucide-react';

const categories = {
  majorTraditions: "Major Traditions",
  weaponSystems: "Weapon Systems",
  items: "Items",
  mechanics: "Game Mechanics",
  progression: "Progression Systems"
};

const systems = {
  majorTraditions: [
    {
      name: "Solomonic Magick",
      elements: [
        "Seal of Solomon - Creates binding circles and enhances divine authority for commanding spirits and angels",
        "Divine authority meter - Accumulates power for stronger angel summoning and bindings",
        "Angel summoning - Automatically summons different orders of angels based on authority level",
        "Circle magick - Creates automated circles with Venn diagram effects and combinations",
        "Command spirits - Control defeated enemies through divine authority"
      ],
      implementation: "Core system focusing on divine authority and binding. Automated circle creation with angel summoning based on authority level."
    },
    {
      name: "Alchemical Magick",
      elements: [
        "Nigredo - Decomposition effects slowing enemies and breaking reality barriers",
        "Albedo - Purification effects transmuting enemies into beneficial forms",
        "Citrinitas - Solar effects enabling consciousness expansion and reality manipulation",
        "Rubedo - Final transformation combining all stages for ultimate transmutation"
      ],
      implementation: "Progressive system serving as both weapon system and advancement path. Each stage unlocks deeper reality manipulation."
    },
    {
      name: "Enochian Magick",
      elements: [
        "Angelic alphabet - Floating glyph weapons that combine into powerful words",
        "Watchtower zones - Overlapping elemental fields with unique combinations",
        "30 Aethyrs - Progressive system from basic to reality-bending powers",
        "Word combinations - Letters auto-combine for enhanced effects"
      ],
      implementation: "Language-based system with automated combinations. Higher Aethyrs unlock more powerful effects."
    },
    {
      name: "Kabbalah",
      elements: [
        "Sephiroth weapons - Tree of Life-based attack types with unique effects",
        "Tree of Life mechanics - Interconnected upgrade paths following divine structure",
        "Divine names - Combine Hebrew letters for powerful effects",
        "Sacred geometry - Reality-affecting patterns based on divine principles"
      ],
      implementation: "Hierarchical system following Tree of Life structure. Powers enhance based on Sephirotic connections."
    },
    {
      name: "Hermetic",
      elements: [
        "As Above, So Below - Effects mirror across reality planes",
        "Reality mirroring - Creates opposite-functioning attacks",
        "Planetary forces - Time-based variations",
        "Element control - Classical element combinations",
        "Correspondence system - Linked effects across realities"
      ],
      implementation: "Reality manipulation through correspondences and mirroring effects."
    },
    {
      name: "Vedic/Hindu",
      elements: [
        "Yantra patterns - Geometric fields with persistent effects",
        "Mantra weapons - Sound-based resonance attacks",
        "Chakra system - Progressive energy unlocks",
        "Sanskrit powers - Word-based attack combinations"
      ],
      implementation: "Energy and sound-based system with geometric patterns and progressive unlocks."
    },
    {
      name: "Chinese",
      elements: [
        "Wu Xing - Five Elements cycle effects",
        "Feng Shui - Environmental energy manipulation",
        "Bagua powers - Eight trigram attack types",
        "Qi channeling - Energy-based combat"
      ],
      implementation: "Element and energy system with balance focus."
    },
    {
      name: "Voudon",
      elements: [
        "Loa aspects - Channel spirits for abilities",
        "Serviteur powers - Create autonomous allies",
        "Veve patterns - Auto-drawing effect zones",
        "Crossroads magic - Reality intersection effects"
      ]
    },
    {
      name: "Egyptian",
      elements: [
        "Heka system - Name-based power attacks",
        "Book spells - Chapter-based abilities",
        "Ka/Ba manipulation - Spirit essence effects",
        "Hieroglyphic magic - Symbol combinations"
      ]
    },
    {
      name: "Norse",
      elements: [
        "Seidr - Fate-weaving effects",
        "Galdr - Rune song powers",
        "Bind-runes - Rune combinations",
        "World-tree - Multi-realm effects"
      ]
    },
    {
      name: "Tibetan",
      elements: [
        "Tantric powers - Energy manipulation",
        "Mandala effects - Geometric powers",
        "Prayer wheels - Automatic effects",
        "Meditation - Consciousness expansion"
      ]
    },
    {
      name: "Gnostic",
      elements: [
        "Archon binding - Entity control",
        "Pleroma energy - Divine power",
        "Aeon manifestation - Deity aspects",
        "Reality understanding - Truth powers"
      ]
    }
  ],
  weaponSystems: [
    {
      name: "Circle Magick",
      elements: [
        "Multiple overlapping circles - Create Venn diagram effects",
        "Pattern effects - Different circle types combine uniquely",
        "Auto-positioning - Spawns like VS weapons",
        "Synergy system - Enhanced effects from combinations",
        "Circle upgrades - Size, duration, and interaction boosts"
      ]
    },
    {
      name: "Magick Square Weapons",
      elements: [
        "Numerical patterns - Different squares with properties",
        "Auto-filling - Squares charge during combat",
        "Completion effects - Pattern completion triggers",
        "Pattern synergies - Multiple square combinations"
      ]
    },
    {
      name: "Sigil Magick",
      elements: [
        "Auto-drawing - Creates floating sigils",
        "Intent-based - Different effects per type",
        "Layered effects - Combining enhances power",
        "Progressive complexity - Power scales with upgrades"
      ]
    },
    {
      name: "Mirror Weapons",
      elements: [
        "Reality inversion - Mirror world effects",
        "Cross-reality damage - Multi-plane impacts",
        "Symmetrical patterns - Balanced attacks",
        "Reflection mechanics - Inter-reality bouncing"
      ]
    },
    {
      name: "Quantum Weapons",
      elements: [
        "Probability manipulation - Multi-state attacks",
        "Superposition effects - Multi-location impacts",
        "Observer mechanics - Observation changes effects",
        "Quantum entanglement - Linked enemy damage"
      ]
    },
    {
      name: "Non-Euclidean",
      elements: [
        "Impossible geometry - Space-defying attacks",
        "Reality warping - Space distortion effects",
        "Spatial manipulation - Distance/direction control",
        "Geometric progression - Complex patterns"
      ]
    }
  ],
  items: [
    {
      name: "Magick Square Talismans",
      elements: [
        "Planetary squares - Different powers per square",
        "Permanent effects - Passive bonus generation",
        "Multi-square activation - Combined effects",
        "Progressive unlocks - Master simpler for complex",
        "Estate preparation - Pre-run charging"
      ]
    },
    {
      name: "Sacred Stones",
      elements: [
        "Gem properties - Unique passive effects",
        "Crystal grids - Pattern-based enhancements",
        "Chakra alignment - Energy center effects",
        "Combinations - Stone synergy bonuses"
      ]
    },
    {
      name: "Ritual Components",
      elements: [
        "Herbs/incense - Effect zones",
        "Sacred oils - Temporary buffs",
        "Candles - Color-based effects",
        "Woods - Burning effects"
      ]
    },
    {
      name: "Prayer Wheels",
      elements: [
        "Movement-based power - Generate while moving",
        "Energy accumulation - Store for release",
        "Wheel types - Various effects",
        "Auto-spinning - VS-style automation"
      ]
    }
  ],
  mechanics: [
    {
      name: "Magick Square Pattern System",
      elements: [
        "Grid-based power - Pattern-based layouts",
        "Numerical harmony - Square pattern effects",
        "Pattern discovery - Find new combinations",
        "Completion bonuses - Classical pattern rewards"
      ]
    },
    {
      name: "Estate Management",
      elements: [
        "Ritual prep - Pre-run modifications",
        "Loadout setup - Equipment customization",
        "Character building - Attribute allocation",
        "Resource control - Magical resource management"
      ]
    },
    {
      name: "Reality Manipulation",
      elements: [
        "Time dilation - Speed control",
        "Reality inversion - Mirror effects",
        "Layer access - Multiple planes",
        "Mirror worlds - Parallel realities"
      ]
    },
    {
      name: "Special Systems",
      elements: [
        "Unique upgrades - Tradition-specific paths",
        "Custom menus - System-specific interfaces",
        "Tradition locks - System-specific runs",
        "Post-enlightenment - Full system access"
      ]
    }
  ],
  progression: [
    {
      name: "Magnum Opus",
      elements: [
        "Nigredo - Basic reality manipulation",
        "Albedo - Purification abilities",
        "Citrinitas - Consciousness expansion",
        "Rubedo - Ultimate enlightenment"
      ]
    },
    {
      name: "Tradition Mastery",
      elements: [
        "System mastery - Tradition-specific growth",
        "Cross-synergies - Multi-tradition powers",
        "Mastery bonuses - High-level abilities",
        "Power mixing - Tradition combinations"
      ]
    },
    {
      name: "Reality Understanding",
      elements: [
        "Layer discovery - Reality plane revelation",
        "Truth uncovering - Reality nature insights",
        "Power mastery - Ability comprehension",
        "Final enlightenment - Complete understanding"
      ]
    }
  ]
};

const MagicDashboard = () => {
  const [selectedCategory, setSelectedCategory] = useState('majorTraditions');
  const [theme, setTheme] = useState('dark');

  const bgColor = theme === 'dark' ? 'bg-gray-900' : 'bg-white';
  const textColor = theme === 'dark' ? 'text-gray-100' : 'text-gray-900';
  const cardBg = theme === 'dark' ? 'bg-gray-800' : 'bg-gray-100';
  const buttonBg = theme === 'dark' ? 'bg-gray-700 hover:bg-gray-600' : 'bg-gray-200 hover:bg-gray-300';
  const cardBorder = theme === 'dark' ? 'border-gray-700' : 'border-gray-300';

  return (
    <div className={`${bgColor} ${textColor} min-h-screen p-6`}>
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-3xl font-bold">Magical Systems Dashboard</h1>
        <button
          onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
          className={`${buttonBg} p-3 rounded-full flex items-center justify-center`}
          aria-label="Toggle theme"
        >
          {theme === 'dark' ? <Sun className="h-5 w-5" /> : <Moon className="h-5 w-5" />}
        </button>
      </div>

      <div className="grid grid-cols-5 gap-4 mb-6">
        {Object.entries(categories).map(([key, value]) => (
          <button
            key={key}
            onClick={() => setSelectedCategory(key)}
            className={`${selectedCategory === key ? 'bg-purple-600' : buttonBg} p-2 rounded`}
          >
            {value}
          </button>
        ))}
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {systems[selectedCategory].map((system, index) => (
          <div key={index} className={`${cardBg} p-6 rounded-lg border ${cardBorder}`}>
            <h2 className="text-xl font-bold mb-4">{system.name}</h2>
            <ul className="list-disc list-inside space-y-2">
              {system.elements.map((element, idx) => (
                <li key={idx}>{element}</li>
              ))}
            </ul>
            {system.implementation && (
              <p className="mt-4 text-sm opacity-80">{system.implementation}</p>
            )}
          </div>
        ))}
      </div>
    </div>
  );
};

export default MagicDashboard;