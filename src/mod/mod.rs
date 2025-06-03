mod forge;
mod version;

use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use semver::Version;

pub use forge::parse_forge_mod_contents;
use crate::r#mod::version::VersionConstraint;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModMetadata {
    pub mod_id: String,
    pub version: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub file_name: String,

    pub platform: Platform,
    pub loader: String,
    pub loader_version: String,

    pub dependencies: Vec<ModDependency>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum Platform {
    Forge,
    Fabric,
    NeoForge,
    Quilt,
    Unknown(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModDependency {
    pub mod_id: String,
    pub version_range: String,
    pub mandatory: bool,
}

#[derive(Debug)]
pub enum DependencyError {
    UnsupportedPlatform(Platform, Vec<String>),
    MissingDependency(String),
    VersionConflict(String, String, String, String, String),
    CircularDependency(Vec<String>),
}

impl fmt::Display for DependencyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DependencyError::UnsupportedPlatform(platform, file_name) => {
                write!(f, "Unsupported platform: {:?}\n {:?}", platform, file_name)
            }
            DependencyError::MissingDependency(mod_id) => {
                write!(f, "Missing dependency: {}", mod_id)
            }
            DependencyError::VersionConflict(file_name, mod_id, required, found, found_name) => write!(
                f,
                "Version conflict for {}:\n    required {} {}, found {} ({}) ",
                file_name, mod_id, required, found, found_name
            ),
            DependencyError::CircularDependency(chain) => write!(
                f,
                "Circular dependency detected: {}",
                chain.join(" -> ")
            ),
        }
    }
}

pub fn analyze_dependencies(
    mods: &[ModMetadata],
) -> Result<Vec<ModMetadata>, DependencyError> {
    let mut platform_groups: HashMap<Platform, Vec<&ModMetadata>> = HashMap::new();
    for mod_ in mods {
        platform_groups
            .entry(mod_.platform.clone())
            .or_default()
            .push(mod_);
    }

    let mut result = Vec::new();
    for (platform, platform_mods) in platform_groups {
        match platform {
            Platform::Forge => {
                let resolved = resolve_forge_dependencies(platform_mods)?;
                result.extend(resolved.into_iter().cloned());
            }
            _ => return Err(DependencyError::UnsupportedPlatform(platform, platform_mods.iter().map(|m| m.file_name.clone()).collect())),
        }
    }

    Ok(result)
}

fn resolve_forge_dependencies(
    mods: Vec<&ModMetadata>,
) -> Result<Vec<&ModMetadata>, DependencyError> {
    let mod_map: HashMap<_, _> = mods
        .iter()
        .map(|m| (m.mod_id.as_str(), *m))
        .collect();

    let mut resolved = HashSet::new();
    let mut unresolved = HashSet::new();
    let mut ordered = Vec::new();

    for mod_ in mods.iter() {
        if !resolved.contains(&mod_.mod_id) {
            resolve_mod(
                mod_,
                &mod_map,
                &mut resolved,
                &mut unresolved,
                &mut ordered,
                &mut vec![mod_.mod_id.clone()],
            )?;
        }
    }

    Ok(ordered)
}

fn resolve_mod<'a>(
    mod_: &'a ModMetadata,
    mod_map: &HashMap<&str, &'a ModMetadata>,
    resolved: &mut HashSet<String>,
    unresolved: &mut HashSet<String>,
    ordered: &mut Vec<&'a ModMetadata>,
    path: &mut Vec<String>,
) -> Result<(), DependencyError> {
    unresolved.insert(mod_.mod_id.clone());

    for dep in &mod_.dependencies {
        if dep.mod_id == "minecraft" || dep.mod_id == "forge" {
            continue;
        }

        if !resolved.contains(&dep.mod_id) {
            if unresolved.contains(&dep.mod_id) {
                let mut cycle = path.clone();
                cycle.push(dep.mod_id.clone());
                return Err(DependencyError::CircularDependency(cycle));
            }

            let dep_mod = match mod_map.get(dep.mod_id.as_str()) {
                Some(m) => m,
                None => {
                    if dep.mandatory {
                        return Err(DependencyError::MissingDependency(dep.mod_id.clone()));
                    } else {
                        continue;
                    }
                }
            };

            let constraint: VersionConstraint = dep.version_range.parse().unwrap();
            if !constraint.matches(&Version::parse(&dep_mod.version).unwrap()) {
                return Err(DependencyError::VersionConflict(
                    mod_.file_name.clone(),
                    dep.mod_id.clone(),
                    dep.version_range.clone(),
                    dep_mod.version.clone(),
                    dep_mod.file_name.clone()
                ));
            }

            path.push(dep.mod_id.clone());
            resolve_mod(dep_mod, mod_map, resolved, unresolved, ordered, path)?;
            path.pop();
        }
    }

    resolved.insert(mod_.mod_id.clone());
    unresolved.remove(&mod_.mod_id);
    ordered.push(mod_);

    Ok(())
}