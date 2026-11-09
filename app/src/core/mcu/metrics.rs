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
