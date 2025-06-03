mod jar;
mod r#mod;

use std::path::{Path, PathBuf};
use std::io::Read;
use clap::Parser;
use anyhow::Result;
use crate::r#mod::{ModMetadata, parse_forge_mod_contents, analyze_dependencies};

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

fn parse_mod_file(path: &Path) -> Result<ModMetadata> {
    let file_name = path.file_name().unwrap().to_string_lossy().into_owned();

    let mut archive = jar::open_jar_file(path)?;

    if let Ok(mut file) = archive.by_name("fabric.mod.json") {
        Err("Not Implemented").unwrap_or_else(|_| {
            //panic!("Fabric mod parsing not implemented for file: {}", file_name);
        });
    }
    if let Ok(mut file) = archive.by_name("META-INF/mods.toml") {
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        return parse_forge_mod_contents(&contents, &file_name);
    }

    Err(anyhow::anyhow!(
        "Neither fabric.mod.json nor META-INF/mods.toml found in {}",
        file_name
    ))
}

fn load_mods_from_dir(dir: &Path) -> Result<Vec<ModMetadata>> {
    let mut mods = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |ext| ext == "jar") {
            let file_name = path.file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("unknown.jar");

            match parse_mod_file(&path) {
                Ok(mod_data) => mods.push(mod_data),
                Err(e) => eprintln!("Skipping {}: {}", file_name, e),
            }
        }
    }

    Ok(mods)
}