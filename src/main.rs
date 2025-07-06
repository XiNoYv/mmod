mod jar;
mod r#mod;

use std::path::{Path, PathBuf};
use clap::Parser;
use anyhow::Result;
use crate::r#mod::{ModMetadata, parse_forge_mod_contents, parse_fabric_mod_contents, analyze_dependencies};

#[derive(Parser)]
#[command(name = "Minecraft MODs Dependency Analyzer")]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(default_value = "./")]
    dir: PathBuf,
    #[arg(long, action)]
    verbose: bool
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mods_dir = cli.dir.as_path();
    let verbose = cli.verbose;

    if !mods_dir.exists() {
        anyhow::bail!("Mods directory not found: {}", mods_dir.display());
    }

    let mods = load_mods_from_dir(mods_dir)?;

    println!("[✓] {} mods analyzed", mods.len());

    if verbose{
        for mod_data in mods.iter() {
            println!("  - {}", mod_data.mod_id);
        }
    }

    match analyze_dependencies(&mods) {
        Ok(ordered) => println!("All dependencies are satisfied!"),
        Err(e) => {
            eprintln!("Dependency error: {}", e);
        }
    }

    Ok(())
}

fn parse_mod_file(path: &Path) -> Result<Vec<ModMetadata>> {
    let file_name = path.file_name().unwrap().to_string_lossy().into_owned();

    let mut archive = jar::open_jar_file(path)?;

    if archive.by_name("fabric.mod.json").is_ok() {
        return Ok(vec![parse_fabric_mod_contents(&mut archive, &file_name)?]);
    }
    if archive.by_name("META-INF/mods.toml").is_ok() {
        return parse_forge_mod_contents(&mut archive, &file_name);
    }

    Err(anyhow::anyhow!(
        "Neither fabric.mod.json nor META-INF/mods.toml found in {}",
        file_name
    ))
}

fn load_mods_from_dir(dir: &Path) -> Result<Vec<ModMetadata>> {
    let mut mods = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();

        if path.is_file() && path.extension().map_or(false, |ext| ext == "jar") {
            let file_name = path.file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("unknown.jar");

            match parse_mod_file(&path) {
                Ok(mod_data_vec) => mods.extend(mod_data_vec),
                Err(e) => eprintln!("Skipping {}: {}", file_name, e),
            }
        }
    }

    Ok(mods)
}