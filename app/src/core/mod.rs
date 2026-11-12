// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Deterministic emulation; no UI, filesystem, host clock, or native audio dependencies.
pub mod cpu;
pub mod device;
pub mod mcu;
pub mod p1;
pub mod profile;
pub mod tamagotchi;

pub mod state;

pub mod s1c33;

pub mod splb32;
