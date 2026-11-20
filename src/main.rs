mod cli;
mod compat;
mod pack;
mod report;
mod scaffold;
mod validate;
mod watch;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;

use crate::cli::{Cli, Command, Format};

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::New {
            name,
            dir,
            template,
        } => run_new(&name, dir, template.as_deref()),
        Command::Check {
            path,
            format,
            verbose,
        } => run_check(path, format, verbose),
        Command::Pack { path, output } => run_pack(path, output),
        Command::Watch { path, interval } => run_watch(path, interval),
        Command::Compat { path, format } => run_compat(path, format),
    }
}

fn resolve_root(path: Option<String>) -> PathBuf {
    match path {
        Some(p) => PathBuf::from(p),
        None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    }
}

fn run_new(name: &str, dir: Option<String>, template: Option<&str>) -> ExitCode {
    let target = match dir {
        Some(d) => PathBuf::from(d),
        None => PathBuf::from(name),
    };

    let result = match template {
        Some(t) => scaffold::scaffold_from_template(name, &target, Path::new(t)),
        None => scaffold::scaffold(name, &target),
    };
    match result {
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

fn run_check(path: Option<String>, format: Format, verbose: bool) -> ExitCode {
    let root = resolve_root(path);
    let (report, skills_checked) = validate::check_plugin_dir(&root);

    let output = match format {
        Format::Human => report.render_human(skills_checked, verbose),
        Format::Json => report.render_json(skills_checked, verbose),
    };
    print!("{}", output);

    if report.ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn run_pack(path: Option<String>, output: Option<String>) -> ExitCode {
    let root = resolve_root(path);
    let target = match output {
        Some(o) => PathBuf::from(o),
        None => {
            let name = root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "plugin".to_string());
            PathBuf::from(format!("{}.zip", name))
        }
    };

    match pack::pack_plugin(&root, &target) {
        Ok(result) => {
            println!(
                "packed {} file(s) into {} ({} bytes)",
                result.files,
                target.display(),
                result.bytes
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("plugcheck: {}", err);
            ExitCode::FAILURE
        }
    }
}

fn run_watch(path: Option<String>, interval: u64) -> ExitCode {
    let root = resolve_root(path);
    if !root.is_dir() {
        eprintln!("plugcheck: not a directory: {}", root.display());
        return ExitCode::FAILURE;
    }
    if interval == 0 {
        eprintln!("plugcheck: --interval must be at least 1 second");
        return ExitCode::FAILURE;
    }
    println!("plugcheck: watching {} (Ctrl-C to stop)", root.display());
    let check = || {
        let (report, skills) = validate::check_plugin_dir(&root);
        print!("{}", report.render_human(skills, false));
    };
    match watch::watch(&root, Duration::from_secs(interval), check) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("plugcheck: {}", err);
            ExitCode::FAILURE
        }
    }
}

fn run_compat(path: Option<String>, format: Format) -> ExitCode {
    let root = resolve_root(path);
    match compat::check_compat(&root) {
        Ok(report) => {
            let output = match format {
                Format::Human => compat::render_human(&report),
                Format::Json => {
                    serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
                }
            };
            print!("{}", output);
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("plugcheck: {}", err);
            ExitCode::FAILURE
        }
    }
}
