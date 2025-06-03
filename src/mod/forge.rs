use serde::Deserialize;
use anyhow::{Context, Result};
use crate::r#mod::{ModDependency, ModMetadata, Platform};

#[derive(Debug, Deserialize)]
pub struct ModsToml {
    #[serde(rename = "modLoader")]
    pub mod_loader: String,
    #[serde(rename = "loaderVersion")]
    pub loader_version: String,
    #[serde(rename = "license")]
    pub license: Option<String>,
    #[serde(rename = "issueTrackerURL")]
    pub issue_tracker_url: Option<String>,
    #[serde(rename = "showAsResourcePack")]
    pub show_as_resource_pack: Option<bool>,
    #[serde(rename = "clientSideOnly")]
    pub client_side_only: Option<bool>,
    #[serde(rename = "mods")]
    pub mods: Vec<ModEntry>,
    #[serde(rename = "dependencies")]
    pub dependencies: Option<Dependencies>,
}

#[derive(Debug, Deserialize)]
pub struct ModEntry {
    #[serde(rename = "modId")]
    pub mod_id: String,
    #[serde(rename = "version")]
    pub version: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "updateJSONURL")]
    pub update_json_url: Option<String>,
    #[serde(rename = "displayURL")]
    pub display_url: Option<String>,
    #[serde(rename = "logoFile")]
    pub logo_file: Option<String>,
    #[serde(rename = "credits")]
    pub credits: Option<String>,
    #[serde(rename = "description")]
    pub description: Option<String>,
    #[serde(rename = "authors")]
    pub authors: Option<Authors>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Authors {
    String(String),
    Array(Vec<String>),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Dependencies {
    SingleMod(Vec<DependencyEntry>),
    MultiMod(std::collections::HashMap<String, Vec<DependencyEntry>>),
}

#[derive(Debug, Deserialize)]
pub struct DependencyEntry {
    #[serde(rename = "modId")]
    pub mod_id: String,
    pub mandatory: bool,
    #[serde(rename = "versionRange")]
    pub version_range: String,
    pub ordering: String,
    pub side: String,
}

pub fn parse_forge_mod_contents(contents: &str, file_name: &String) -> Result<ModMetadata> {
    let toml: ModsToml = toml::from_str(contents)
        .with_context(|| "Failed to parse Forge mods.toml")?;

    let mut metadata = ModMetadata::from(&toml);

    metadata.file_name = file_name.clone();

    Ok(metadata)
}

impl From<&ModsToml> for ModMetadata {
    fn from(toml: &ModsToml) -> Self {
        let primary_mod = toml.mods.first().expect("At least one mod entry");

        ModMetadata {
            mod_id: primary_mod.mod_id.clone(),
            version: primary_mod.version.clone(),
            name: primary_mod.display_name.clone(),
            description: primary_mod.description.clone(),
            authors: parse_authors(&primary_mod.authors),
            platform: Platform::Forge,
            loader: toml.mod_loader.clone(),
            loader_version: toml.loader_version.clone(),

            dependencies: parse_forge_dependencies(&toml),
            file_name: "".to_string(),
        }
    }
}

fn parse_authors(authors: &Option<Authors>) -> Vec<String> {
    match authors {
        Some(Authors::String(s)) => s.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Some(Authors::Array(arr)) => arr.clone(),
        None => Vec::new(),
    }
}

fn parse_forge_dependencies(toml: &ModsToml) -> Vec<ModDependency> {
    let mut dependencies = Vec::new();

    if let Some(deps) = &toml.dependencies {
        match deps {
            Dependencies::SingleMod(entries) => {
                for entry in entries {
                    dependencies.push(ModDependency {
                        mod_id: entry.mod_id.clone(),
                        version_range: entry.version_range.clone(),
                        mandatory: entry.mandatory,
                    });
                }
            },
            Dependencies::MultiMod(map) => {
                for (_, entries) in map {
                    for entry in entries {
                        dependencies.push(ModDependency {
                            mod_id: entry.mod_id.clone(),
                            version_range: entry.version_range.clone(),
                            mandatory: entry.mandatory,
                        });
                    }
                }
            }
        }
    }

    dependencies
}