// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Observational counters; none of these values control emulated time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CycleCounters {
    pub instructions: u64,
    pub halt: u64,
    pub interrupts: u64,
    pub reset: u64,
    pub fast: u64,
    pub slow: u64,
    pub osc1_edges: u64,
    pub audio_samples: u64,
    pub clock_calls: u64,
}
impl CycleCounters {
    pub fn total(&self) -> u64 {
        self.instructions + self.halt + self.interrupts + self.reset
    }
}
/// Detailed timing is opt-in because taking host timestamps has a measurable cost.
#[derive(Clone, Copy, Debug, Default)]
pub struct CoreProfile {
    pub cpu_ns: u64,
    pub irq_ns: u64,
    pub timers_ns: u64,
    pub pcm_ns: u64,
}

impl CycleCounters {
    pub(crate) fn encode_state(&self, e: &mut crate::core::state::Encoder) {
        e.u64(self.instructions);
        e.u64(self.halt);
        e.u64(self.interrupts);
        e.u64(self.reset);
        e.u64(self.fast);
        e.u64(self.slow);
        e.u64(self.osc1_edges);
        e.u64(self.audio_samples);
        e.u64(self.clock_calls);
    }
    pub(crate) fn decode_state(
        &mut self,
        d: &mut crate::core::state::Decoder<'_>,
    ) -> crate::core::state::Result<()> {
        self.instructions = d.u64()?;
        self.halt = d.u64()?;
        self.interrupts = d.u64()?;
        self.reset = d.u64()?;
        self.fast = d.u64()?;
        self.slow = d.u64()?;
        self.osc1_edges = d.u64()?;
        self.audio_samples = d.u64()?;
        self.clock_calls = d.u64()?;

        Ok(())
    }
}
