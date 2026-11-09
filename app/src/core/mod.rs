//! Platform-independent CPU components for future device machines.
//!
//! The partial CPU core is not yet connected to the user interface.
//! Do not introduce dependencies on windows, egui, or native audio output.
//! The planned contract is described in `docs/ARCHITECTURE.md`.

pub mod cpu;
