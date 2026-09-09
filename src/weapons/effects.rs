//! Each pattern contributes one effect; overlap never dispatches a special combo.
use super::magick_circle::PatternType;

pub const PULL_SPEED_CAP: f32 = 180.0;
pub const ROOT_LINGER_SECONDS: f32 = 1.0;

#[derive(Clone, Copy)]
pub enum CircleEffect {
    Damage { interval: f32, multiplier: f32 },
    Root { strength: f32 },
    Pull { speed: f32 },
    Ward { reduction: f32 },
}

impl PatternType {
    pub fn effect(self) -> CircleEffect {
        match self {
            Self::Banishment => CircleEffect::Damage {
                interval: 0.5,
                multiplier: 1.0,
            },
            Self::Binding => CircleEffect::Root { strength: 1.0 },
            Self::Invocation => CircleEffect::Pull { speed: 120.0 },
            Self::Manifestation => CircleEffect::Damage {
                interval: 1.25,
                multiplier: 3.0,
            },
            Self::Protection => CircleEffect::Ward { reduction: 0.5 },
        }
    }
}

impl CircleEffect {
    pub fn interval(self) -> f32 {
        match self {
            Self::Damage { interval, .. } => interval,
            _ => 0.5,
        }
    }
}
