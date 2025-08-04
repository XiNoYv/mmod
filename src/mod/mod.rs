mod forge;
mod fabric;
mod version;
mod neoforge;

pub use forge::parse_forge_mod_contents;
pub use fabric::parse_fabric_mod_contents;
pub use neoforge::parse_neoforge_mod_contents;
use crate::r#mod::version::VersionConstraint;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use semver::Version;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModMetadata {
    pub mod_id: String,
    pub version: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub file_name: String,
    pub platform: Platform,
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
#[serde(untagged)]
pub enum DependencyVersionRange {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModDependency {
    pub mod_id: String,
    pub version_range: DependencyVersionRange,
    pub mandatory: bool,
}

#[derive(Debug)]
pub enum DependencyError {
    UnsupportedPlatform(Platform, Vec<String>),
    MissingDependency(String, String, String),
    VersionConflict(String, String, String, String, String),
    CircularDependency(Vec<String>),
    InvalidVersionFormat(String, String, String),
}

#[derive(Debug)]
pub struct DependencyErrors(pub Vec<DependencyError>);

impl fmt::Display for DependencyErrors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, error) in self.0.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{}", error)?;
        }
        Ok(())
    }
}

impl fmt::Display for DependencyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DependencyError::UnsupportedPlatform(platform, file_name) => {
                write!(f, "Unsupported platform: {:?}\n {:?}", platform, file_name)
            }
            DependencyError::MissingDependency(mod_id, file_name, dependency_id) => {
                write!(f, "Missing dependency for {} ({}): {}", mod_id, file_name, dependency_id)
            }
            DependencyError::VersionConflict(file_name, mod_id, required, found, found_name) => write!(
                f,
                "Version conflict for {}:\n    required {} {}, found {} ({}) ",
                file_name, mod_id, required, found, found_name
            ),
            DependencyError::CircularDependency(chain) => {
                write!(f, "Circular dependency detected: {}", chain.join(" -> "))
            }
            DependencyError::InvalidVersionFormat(mod_id, file_name, version_str) => write!(
                f,
                "Invalid version format for {} ({}): \"{}\"",
                mod_id, file_name, version_str
            ),
        }
    }
}

pub fn analyze_dependencies(
    mods: &[ModMetadata],
) -> Result<Vec<ModMetadata>, DependencyErrors> {
    let mut platform_groups: HashMap<Platform, Vec<&ModMetadata>> = HashMap::new();
    for mod_ in mods {
        platform_groups
            .entry(mod_.platform.clone())
            .or_default()
            .push(mod_);
    }

    let mut result = Vec::new();
    let mut all_errors = Vec::new();

    for (platform, platform_mods) in platform_groups {
        match platform {
            Platform::Forge | Platform::Fabric | Platform::NeoForge => {
                match resolve_dependencies(platform_mods) {
                    Ok(resolved) => result.extend(resolved.into_iter().cloned()),
                    Err(errors) => all_errors.extend(errors.0),
                }
            }
            _ => {
                all_errors.push(DependencyError::UnsupportedPlatform(
                    platform,
                    platform_mods.iter().map(|m| m.file_name.clone()).collect(),
                ));
            }
        }
    }

    if all_errors.is_empty() {
        Ok(result)
    } else {
        Err(DependencyErrors(all_errors))
    }
}

fn resolve_dependencies(
    mods: Vec<&ModMetadata>,
) -> Result<Vec<&ModMetadata>, DependencyErrors> {
    let mod_map: HashMap<_, _> = mods
        .iter()
        .map(|m| (m.mod_id.as_str(), *m))
        .collect();

    let mut resolved = HashSet::new();
    let mut ordered = Vec::new();
    let mut errors = Vec::new();

    for mod_ in mods.iter() {
        if !resolved.contains(&mod_.mod_id) {
            resolve_mod(
                mod_,
                &mod_map,
                &mut resolved,
                &mut HashSet::new(),
                &mut ordered,
                &mut vec![mod_.mod_id.clone()],
                &mut errors,
            );
        }
    }

    if errors.is_empty() {
        Ok(ordered)
    } else {
        Err(DependencyErrors(errors))
    }
}

fn resolve_mod<'a>(
    mod_: &'a ModMetadata,
    mod_map: &HashMap<&str, &'a ModMetadata>,
    resolved: &mut HashSet<String>,
    unresolved: &mut HashSet<String>,
    ordered: &mut Vec<&'a ModMetadata>,
    path: &mut Vec<String>,
    errors: &mut Vec<DependencyError>,
) {
    unresolved.insert(mod_.mod_id.clone());

    for dep in &mod_.dependencies {
        if matches!(dep.mod_id.as_str(), "minecraft" | "forge" | "fabricloader" | "fabric-resource-loader-v0" | "java" | "neoforge") {
            continue;
        }

        if resolved.contains(&dep.mod_id) {
            continue;
        }

        if unresolved.contains(&dep.mod_id) {
            let mut cycle = path.clone();
            cycle.push(dep.mod_id.clone());
            errors.push(DependencyError::CircularDependency(cycle));
            continue;
        }

        let dep_mod = match mod_map.get(dep.mod_id.as_str()) {
            Some(m) => m,
            None => {
                if dep.mandatory {
                    errors.push(DependencyError::MissingDependency(
                        mod_.mod_id.clone(),
                        mod_.file_name.clone(),
                        dep.mod_id.clone(),
                    ));
                }
                continue;
            }
        };

        let current_mod_version = match Version::parse(&dep_mod.version) {
            Ok(v) => v,
            Err(_) => {
                errors.push(DependencyError::InvalidVersionFormat(
                    dep_mod.mod_id.clone(),
                    dep_mod.file_name.clone(),
                    dep_mod.version.clone(),
                ));
                continue;
            }
        };

        let mut version_matched = false;

        match &dep.version_range {
            DependencyVersionRange::Single(required_version_str) => {
                match required_version_str.parse::<VersionConstraint>() {
                    Ok(constraint) => {
                        if constraint.matches(&current_mod_version) {
                            version_matched = true;
                        }
                    }
                    Err(_) => {
                        errors.push(DependencyError::InvalidVersionFormat(
                            dep.mod_id.clone(),
                            mod_.file_name.clone(),
                            required_version_str.clone(),
                        ));
                    }
                }
            },
            DependencyVersionRange::Multiple(required_versions_vec) => {
                for req_ver_str in required_versions_vec {
                    match req_ver_str.parse::<VersionConstraint>() {
                        Ok(constraint) => {
                            if constraint.matches(&current_mod_version) {
                                version_matched = true;
                                break;
                            }
                        }
                        Err(_) => {
                            errors.push(DependencyError::InvalidVersionFormat(
                                dep.mod_id.clone(),
                                mod_.file_name.clone(),
                                req_ver_str.clone(),
                            ));
                        }
                    }
                }
            },
        }

        if !version_matched {
            let required_display = match &dep.version_range {
                DependencyVersionRange::Single(s) => s.clone(),
                DependencyVersionRange::Multiple(v) => v.join(" || "),
            };
            errors.push(DependencyError::VersionConflict(
                mod_.file_name.clone(),
                dep.mod_id.clone(),
                required_display,
                dep_mod.version.clone(),
                dep_mod.file_name.clone(),
            ));
        }

        path.push(dep.mod_id.clone());
        resolve_mod(dep_mod, mod_map, resolved, unresolved, ordered, path, errors);
        path.pop();
    }

    resolved.insert(mod_.mod_id.clone());
    unresolved.remove(&mod_.mod_id);
    ordered.push(mod_);
}
