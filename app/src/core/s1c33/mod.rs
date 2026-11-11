// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros (Rust translation and integration)
// Derived from maragotchi/tamaemu; retain upstream GPL notices and provenance.
//! Native S1C33 PE CPU/MCU and P's backend; other hardware maps remain unverified.
pub mod cpu;
pub mod lcd;
pub mod machine;
pub mod memory;
pub mod peripherals;
pub mod profile;
mod rom;
pub mod state;
#[cfg(test)]
mod tests;

mod pn512;
