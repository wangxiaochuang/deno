use std::env;

use clap::Parser;

use crate::{utils::build_project, CmdExecutor};

#[derive(Debug, Parser)]
pub struct BuildOpts {}

impl CmdExecutor for BuildOpts {
    async fn execute(self) -> anyhow::Result<()> {
        let cur_dir = env::current_dir()?.display().to_string();
        let filename = build_project(&cur_dir)?;
        eprintln!("Build success: {}", filename);
        Ok(())
    }
}
