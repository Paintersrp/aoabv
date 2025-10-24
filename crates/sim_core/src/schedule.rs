use anyhow::Result;

use crate::diff::Diff;
use crate::io::frame::Highlight;
use crate::reduce::apply;
use crate::rng::{stream_label, Stream};
use crate::world::World;

#[derive(Clone, Debug)]
pub struct KernelRun {
    pub diff: Diff,
    pub chronicle: Vec<String>,
    pub highlights: Vec<Highlight>,
}

impl KernelRun {
    pub fn new(diff: Diff) -> Self {
        Self {
            diff,
            chronicle: Vec::new(),
            highlights: Vec::new(),
        }
    }
}

pub fn run_kernel<F>(
    world: &mut World,
    aggregate_diff: &mut Diff,
    parent_stream: &Stream,
    stage_label: &str,
    mut runner: F,
) -> Result<KernelRun>
where
    F: FnMut(&mut World, &mut Stream) -> Result<KernelRun>,
{
    let mut kernel_rng = parent_stream.derive(stream_label(stage_label));
    let run = runner(world, &mut kernel_rng)?;
    aggregate_diff.merge(&run.diff);
    apply(world, run.diff.clone());
    Ok(run)
}
