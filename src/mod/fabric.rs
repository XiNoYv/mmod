use std::collections::HashMap;
use std::fs::File;
use serde::Deserialize;
use serde_json;
use anyhow::{Context, Result};
use zip::ZipArchive;
use std::io::Read;
use crate::r#mod::{ModDependency, ModMetadata, Platform, DependencyVersionRange};

// https://docs.fabricmc.net/develop/getting-started/project-structure#fabric-mod-json

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum DependencyVersion {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FabricMod {
    pub schema_version: u32,
    /// The mod's ID, which should be unique.
    pub id: String,
    /// The mod's version
    pub version: String,
    /// The mod's name.
    pub name: Option<String>,
    pub description: Option<String>,
    pub authors: Option<Vec<Author>>,
    pub contact: Option<Contact>,
    pub license: Option<String>,
    pub icon: Option<String>,
    /// The environment that the mod runs in
    pub environment: Option<String>,
    /// The mods that the mod depends on.
    pub depends: Option<HashMap<String, DependencyVersion>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Author {
    Simple(String),
    Detailed {
        name: String,
        contact: Option<Contact>
    },
}

#[derive(Debug, Deserialize)]
pub struct Contact {
    pub homepage: Option<String>,
    pub sources: Option<String>,
    pub issues: Option<String>,
}

pub fn parse_fabric_mod_contents(jar_file: &mut ZipArchive<File>, file_name: &String) -> Result<ModMetadata> {
    let mut file = jar_file.by_name("fabric.mod.json")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let json: FabricMod = serde_json::from_str(contents.as_str())
        .with_context(|| format!("Failed to parse Fabric fabric.mod.json from {}", file_name))?;

    let mut metadata = ModMetadata::try_from(&json)
        .with_context(|| format!("Failed to convert Fabric fabric.mod.json to metadata for {}", file_name))?;
    
    metadata.file_name = file_name.clone();

    Ok(metadata)
}

impl TryFrom<&FabricMod> for ModMetadata {
    type Error = anyhow::Error;

    fn try_from(json: &FabricMod) -> Result<Self, Self::Error> {
        Ok(ModMetadata {
            mod_id: json.id.clone(),
            version: json.version.clone(),
            name: json.name.clone(),
            description: json.description.clone(),
            authors: parse_authors(&json.authors),
            platform: Platform::Fabric,
            dependencies: parse_fabric_dependencies(json),
            file_name: "".to_string(),
        })
    }
}

fn parse_authors(authors: &Option<Vec<Author>>) -> Vec<String> {
    let Some(authors) = authors else { return Vec::new() };
    authors.iter().map(|author| match author {
        Author::Simple(name) => name.clone(),
        Author::Detailed { name, .. } => name.clone(),
    }).collect()
}

fn parse_fabric_dependencies(json: &FabricMod) -> Vec<ModDependency> {
    let mut deps = Vec::new();

    let mut process_deps = |map: &Option<HashMap<String, DependencyVersion>>, mandatory: bool| {
        if let Some(dependencies) = map {
            for (id, version_spec) in dependencies {
                let version_range = match version_spec {
                    DependencyVersion::Single(s) => DependencyVersionRange::Single(s.clone()),
                    DependencyVersion::Multiple(v) => DependencyVersionRange::Multiple(v.clone()),
                };
                deps.push(ModDependency {
                    mod_id: id.clone(),
                    version_range,
                    mandatory,
                });
            }
        }
    };

    process_deps(&json.depends, true);

    deps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fabric_mod_contents() {
        let json_content = r#"{
            "schemaVersion": 1,
            "id": "my_mod",
            "version": "1.0.0",
            "name": "My Awesome Mod",
            "description": "This is a test mod.",
            "authors": [
                "Test Author"
            ],
            "contact": {
                "homepage": "https://example.com"
            },
            "license": "MIT",
            "environment": "*",
            "depends": {
                "fabricloader": ">=0.14.0",
                "minecraft": "1.19.x"
            }
        }"#;
        let file_name = "fabric.mod.json".to_string();
        let json: FabricMod = serde_json::from_str(json_content)
            .with_context(|| format!("Failed to parse Fabric fabric.mod.json from {}", file_name))?;

        let mut metadata = ModMetadata::try_from(&json)
            .with_context(|| format!("Failed to convert Fabric fabric.mod.json to metadata for {}", file_name))?;

        metadata.file_name = file_name.clone();

        assert_eq!(metadata.mod_id, "my_mod");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.name, Some("My Awesome Mod".to_string()));
        assert_eq!(metadata.authors, vec!["Test Author".to_string()]);
        assert_eq!(metadata.dependencies.len(), 2);
    }

    #[test]
    fn test_parse_fabric_mod_with_dependency_array() {
        let json_content = r#"{
            "schemaVersion": 1,
            "id": "my_mod",
            "version": "1.0.0",
            "depends": {
                "minecraft": ["1.16.2", "1.16.3", "1.16.4", "1.16.5"]
            }
        }"#;
        let file_name = "fabric.mod.json".to_string();
        let json: FabricMod = serde_json::from_str(json_content)
            .with_context(|| format!("Failed to parse Fabric fabric.mod.json from {}", file_name))?;

        let mut metadata = ModMetadata::try_from(&json)
            .with_context(|| format!("Failed to convert Fabric fabric.mod.json to metadata for {}", file_name))?;

        metadata.file_name = file_name.clone();

        assert_eq!(metadata.dependencies.len(), 1);
        assert_eq!(metadata.dependencies[0].mod_id, "minecraft");
        match &metadata.dependencies[0].version_range {
            DependencyVersionRange::Multiple(v) => {
                assert_eq!(v, &vec!["1.16.2", "1.16.3", "1.16.4", "1.16.5"]);
            },
            _ => panic!("Expected Multiple variant"),
        }
    }
}