//! Headless autoplay: a scripted pilot plays whole runs so builds, stages, and
//! estate upgrades can be compared without a person at the controls.
//!
//! `simulate` runs one scenario to its end and reports what happened; `cli`
//! turns `survivors_prototype simulate ...` into a batch of scenarios and
//! `report` prints the results. See `design/AUTOPLAY.md`.
pub mod cli;
pub mod pilot;
pub mod policy;
pub mod report;
pub mod simulate;
