//! The implemented device contract; complete save-state support is a later milestone.
use super::cpu::e0c6200::{CpuError, CpuState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    A,
    B,
    C,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplaySnapshot {
    pub nibbles: [u8; 160],
}
impl Default for DisplaySnapshot {
    fn default() -> Self {
        Self { nibbles: [0; 160] }
    }
}
impl DisplaySnapshot {
    pub fn segment(&self, index: usize, bit: u8) -> bool {
        self.nibbles
            .get(index)
            .is_some_and(|value| value & (1 << bit) != 0)
    }
}

pub struct Descriptor {
    pub id: &'static str,
    pub name: &'static str,
}

pub trait Device: Send {
    fn descriptor(&self) -> Descriptor;
    fn advance_to(&mut self, master_tick: u64) -> Result<(), CpuError>;
    fn time(&self) -> u64;
    fn cpu_state(&self) -> &CpuState;
    fn set_button(&mut self, button: Button, pressed: bool);
    fn release_inputs(&mut self);
    fn reset(&mut self);
    fn display(&self) -> DisplaySnapshot;
    fn drain_audio(&mut self) -> Vec<i16>;
}
