// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Host services: worker scheduling, local ROM files, native dialogs and PCM output.
pub mod audio;
pub mod emulator;
pub mod platform;
pub mod roms;
pub mod runner;

pub mod performance;

pub mod saves;
pub mod settings;
pub mod storage;

pub mod fingerprints;
pub mod library;

pub mod pending_devices;
