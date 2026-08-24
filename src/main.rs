mod cli;
mod report;
mod scaffold;
mod validate;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Command, Format};

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::New { name, dir } => run_new(&name, dir),
        Command::Check {
            path,
            format,
            verbose: _,
        } => run_check(path, format),
    }
}

fn run_new(name: &str, dir: Option<String>) -> ExitCode {
    let target = match dir {
        Some(d) => PathBuf::from(d),
        None => PathBuf::from(name),
    };

    match scaffold::scaffold(name, &target) {
        Ok(()) => {
            println!("Created plugin `{}` in {}", name, target.display());
            println!();
            println!("Next steps:");
            println!("  cd {}", target.display());
            println!("  plugcheck check");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("plugcheck: {}", err);
            ExitCode::FAILURE
        }
    }
}

fn run_check(path: Option<String>, format: Format) -> ExitCode {
    let root = match path {
        Some(p) => PathBuf::from(p),
        None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };

    let (report, skills_checked) = validate::check_plugin_dir(&root);

    let output = match format {
        Format::Human => report.render_human(skills_checked),
        Format::Json => report.render_json(skills_checked),
    };
    print!("{}", output);

    if report.ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
