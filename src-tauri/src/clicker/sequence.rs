//! Multi-point sequence planning.

use crate::state::SequenceStep;

#[derive(Clone, Debug)]
pub struct SequencePlan {
    pub steps: Vec<SequenceStep>,
    pub loop_enabled: bool,
}

impl SequencePlan {
    pub fn new(steps: Vec<SequenceStep>, loop_enabled: bool) -> Option<Self> {
        if steps.is_empty() {
            return None;
        }
        Some(Self {
            steps,
            loop_enabled,
        })
    }

}
