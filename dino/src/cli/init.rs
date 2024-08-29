use std::{fs, path::Path};

use askama::Template;
use clap::Parser;
use dialoguer::Input;
use git2::Repository;

use crate::CmdExecutor;

#[derive(Template)]
#[template(path = "config.yml.j2")]
struct ConfigFile {
    name: String,
}

#[derive(Template)]
#[template(path = "main.ts.j2")]
struct MainTsFile {}

#[derive(Template)]
#[template(path = ".gitignore.j2")]
struct GitIgnoreFile {}

#[derive(Debug, Parser)]
pub struct InitOpts {}

impl CmdExecutor for InitOpts {
    async fn execute(self) -> anyhow::Result<()> {
        let name: String = Input::new().with_prompt("Project name").interact_text()?;
        let cur = Path::new(".");
        if fs::read_dir(cur)?.next().is_none() {
            init_project(&name, cur)
        } else {
            let path = cur.join(&name);
            init_project(&name, &path)
        }
    }
}

fn init_project(name: &str, path: &Path) -> anyhow::Result<()> {
    Repository::init(path)?;
    let config = ConfigFile {
        name: name.to_string(),
    };
    fs::write(path.join("config.yml"), config.render()?)?;
    fs::write(path.join("main.ts"), MainTsFile {}.render()?)?;
    fs::write(path.join(".gitignore"), GitIgnoreFile {}.render()?)?;
    Ok(())
}
