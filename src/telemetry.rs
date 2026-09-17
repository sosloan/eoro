use crate::contracts::{PipelineDiagnostics, StageDiagnostic};
use std::time::Instant;

pub struct StageTimer {
    name: &'static str,
    started: Instant,
    input_count: usize,
}

impl StageTimer {
    pub fn start(name: &'static str, input_count: usize) -> Self {
        Self {
            name,
            started: Instant::now(),
            input_count,
        }
    }

    pub fn finish(
        self,
        diagnostics: &mut PipelineDiagnostics,
        output_count: usize,
        rejected_count: usize,
    ) {
        diagnostics.stages.push(StageDiagnostic {
            stage: self.name.to_string(),
            duration_micros: self.started.elapsed().as_micros(),
            input_count: self.input_count,
            output_count,
            rejected_count,
        });
    }
}
