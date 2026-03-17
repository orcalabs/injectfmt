use std::{
    env::current_dir,
    ffi::OsStr,
    process::ExitCode,
    sync::atomic::{AtomicBool, Ordering},
};

use anyhow::{Result, anyhow};
use clap::Parser;

mod cli;
mod config;
mod fmt;

use cli::Args;
use config::Config;
use fmt::*;
use ignore::{WalkBuilder, WalkState};

fn main() -> Result<ExitCode> {
    let Args {
        paths,
        config,
        check,
    } = Args::parse();

    let config_path = if let Some(path) = config {
        path
    } else {
        let mut dir = current_dir()?;
        loop {
            let path = dir.join("injectfmt.toml");
            if path.is_file() {
                break path;
            }
            dir = dir
                .parent()
                .ok_or_else(|| anyhow!("No config file found"))?
                .into();
        }
    };

    let config = Config::new(config_path)?;

    let mut builder = if paths.is_empty() {
        WalkBuilder::new(current_dir()?)
    } else {
        let mut builder = WalkBuilder::new(&paths[0]);
        for path in paths.into_iter().skip(1) {
            builder.add(path);
        }
        builder
    };

    builder.add_custom_ignore_filename(".injectfmtignore");

    let errored = AtomicBool::new(false);
    let modified = AtomicBool::new(false);

    builder.build_parallel().run(|| {
        Box::new(|entry| {
            let entry = match entry {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{e}");
                    return WalkState::Continue;
                }
            };

            if let Some(ft) = entry.file_type()
                && ft.is_file()
            {
                let path = entry.into_path();
                let Some(ext) = path.extension().and_then(OsStr::to_str) else {
                    return WalkState::Continue;
                };

                for cfg in &*config {
                    if ext == cfg.language.extension() {
                        match injectfmt_file(&path, cfg, check) {
                            Ok(true) => {
                                println!("{}", path.display());
                                modified.store(true, Ordering::Relaxed);
                            }
                            Ok(false) => {}
                            Err(e) => {
                                eprintln!("{e}");
                                errored.store(true, Ordering::Relaxed);
                            }
                        }
                    }
                }
            }

            WalkState::Continue
        })
    });

    Ok(
        if errored.load(Ordering::Relaxed) || (modified.load(Ordering::Relaxed) && check) {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        },
    )
}
