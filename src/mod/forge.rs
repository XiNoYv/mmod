use std::fs::File;
use serde::Deserialize;
use anyhow::{Context, Result};
use zip::ZipArchive;
use std::io::Read;
use crate::r#mod::{DependencyVersionRange, ModDependency, ModMetadata, Platform};

// https://docs.minecraftforge.net/en/latest/gettingstarted/modfiles/#modstoml
#[derive(Debug, Deserialize)]
pub struct ForgeMod {
    /// The language loader used by the mod(s).
    /// Can be used to support alternative language structures,
    /// such as Kotlin objects for the main file,
    /// or different methods of determining the entrypoint,
    /// such as an interface or method.
    /// Forge provides the Java loader `javafml`
    /// and low/no code loader `lowcodefml`.
    #[serde(rename = "modLoader")]
    pub mod_loader: String,
    /// The acceptable version range of the language loader,
    /// expressed as a Maven Version Range.
    /// For `javafml` and `lowcodefml`,
    /// the version is the major version of the Forge version.
    #[serde(rename = "loaderVersion")]
    pub loader_version: String,
    /// The license the mod(s) in this JAR are provided under.
    #[serde(rename = "license")]
    pub license: Option<String>,
    /// A URL representing the place to report and track issues with the mod(s).
    #[serde(rename = "issueTrackerURL")]
    pub issue_tracker_url: Option<String>,
    /// When `true`, the mod(s)’s resources will be displayed as
    /// a separate resource pack on the ‘Resource Packs’ menu,
    /// rather than being combined with the `Mod resources` pack.
    #[serde(rename = "showAsResourcePack")]
    pub show_as_resource_pack: Option<bool>,
    /// Weather the mod is only needed on client side or not.
    #[serde(rename = "clientSideOnly")]
    pub client_side_only: Option<bool>,
    /// Mod-specific properties are tied to the specified mod using the `[[mods]]` header.
    /// This is an array of tables;
    /// all key/value properties will be attached to that mod until the next header.
    #[serde(rename = "mods")]
    pub mods: Vec<ModEntry>,
    /// Mods can specify their dependencies, which are checked by Forge before loading the mods.
    /// These configurations are created using the array of tables `[[dependencies.<modid>]]`
    /// where `modid` is the identifier of the mod the dependency is for.
    #[serde(rename = "dependencies")]
    pub dependencies: Option<Dependencies>,
}

#[derive(Debug, Deserialize)]
pub struct ModEntry {
    /// The unique identifier representing this mod.
    #[serde(rename = "modId")]
    pub mod_id: String,
    /// An override namespace for the mod.
    #[serde(rename = "namespace")]
    pub namespace: Option<String>,
    /// The version of the mod, preferably in a variation of Maven versioning.
    /// When set to `${file.jarVersion}`, it will be replaced with the value of the
    /// `Implementation-Version` property in the JAR’s manifest
    /// (displays as `0.0NONE` in a development environment).
    #[serde(rename = "version")]
    pub version: String,
    /// The pretty name of the mod.
    /// Used when representing the mod on a screen (e.g., mod list, mod mismatch).
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    /// The description of the mod shown in the mod list screen.
    #[serde(rename = "description")]
    pub description: Option<String>,
    /// The name and extension of an image file used on the mods list screen.
    /// The logo must be in the root of the JAR or directly in the root of the source set
    /// (e.g., `src/main/resources` for the main source set).
    #[serde(rename = "logoFile")]
    pub logo_file: Option<String>,
    /// Whether to use `GL_LINEAR*` (true) or `GL_NEAREST*` (false) to render the logoFile.
    #[serde(rename = "logoBlur")]
    pub logo_blur: Option<bool>,
    /// A URL to a JSON used by the [update checker](https://docs.minecraftforge.net/en/latest/misc/updatechecker/)
    /// to make sure the mod you are playing is the latest version.
    #[serde(rename = "updateJSONURL")]
    pub update_json_url: Option<String>,
    /// Credits and acknowledges for the mod shown on the mod list screen.
    #[serde(rename = "credits")]
    pub credits: Option<String>,
    /// The authors of the mod shown on the mod list screen.
    #[serde(rename = "authors")]
    pub authors: Option<Authors>,
    /// A URL to the display page of the mod shown on the mod list screen.
    #[serde(rename = "displayURL")]
    pub display_url: Option<String>,
    /// See [sides](https://docs.minecraftforge.net/en/latest/concepts/sides/#writing-one-sided-mods).
    #[serde(rename = "displayTest")]
    pub display_test: Option<String>,
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
    /// The identifier of the mod added as a dependency.
    #[serde(rename = "modId")]
    pub mod_id: String,
    /// Whether the game should crash when this dependency is not met.
    pub mandatory: bool,
    /// The acceptable version range of the language loader,
    /// expressed as a Maven Version Range.
    /// An empty string matches any version.
    #[serde(rename = "versionRange")]
    pub version_range: String,
    /// Defines if the mod must load before ("BEFORE") or after ("AFTER") this dependency.
    /// If the ordering does not matter, return "NONE"
    pub ordering: String,
    /// The physical side the dependency must be present on: "CLIENT", "SERVER", or "BOTH".
    pub side: String,
}

pub fn parse_forge_mod_contents(jar_file: &mut ZipArchive<File>, file_name: &String) -> Result<Vec<ModMetadata>> {
    let mut file = jar_file.by_name("META-INF/mods.toml")?; // 重新打开文件
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    drop(file);
    let toml: ForgeMod = toml::from_str(contents.as_str())
        .with_context(|| format!("Failed to parse Forge mods.toml from {}", file_name))?;

    let mut all_metadata = Vec::new();

    for mod_entry in &toml.mods {
        let version = if mod_entry.version == "${file.jarVersion}" {
            let mut manifest_file = jar_file.by_name("META-INF/MANIFEST.MF")
                .with_context(|| "META-INF/MANIFEST.MF not found in JAR")?;
            let mut manifest_contents = String::new();
            manifest_file.read_to_string(&mut manifest_contents)?;

            let version_line = manifest_contents.lines()
                .find(|line| line.starts_with("Implementation-Version:"))
                .with_context(|| "Implementation-Version not found in MANIFEST.MF")?;
            version_line.split(": ").nth(1).unwrap_or("unknown").to_string()
        } else {
            mod_entry.version.clone()
        };

        let metadata = ModMetadata {
            mod_id: mod_entry.mod_id.clone(),
            version,
            name: mod_entry.display_name.clone(),
            description: mod_entry.description.clone(),
            authors: parse_authors(&mod_entry.authors),
            platform: Platform::Forge,
            dependencies: parse_forge_dependencies(&toml),
            file_name: file_name.clone(),
        };
        all_metadata.push(metadata);
    }

    Ok(all_metadata)
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

fn parse_forge_dependencies(toml: &ForgeMod) -> Vec<ModDependency> {
    let Some(deps) = &toml.dependencies else { return Vec::new() };

    let entries: Vec<_> = match deps {
        Dependencies::SingleMod(entries) => entries.iter().collect(),
        Dependencies::MultiMod(map) => map.values().flatten().collect(),
    };

    entries.iter().map(|entry| ModDependency {
        mod_id: entry.mod_id.clone(),
        version_range: DependencyVersionRange::Single(entry.version_range.clone()),
        mandatory: entry.mandatory,
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_forge_mod_contents() {
        let toml_content = r#"
modLoader="javafml"
loaderVersion="[52,)"

license="All Rights Reserved"
issueTrackerURL="https://github.com/MinecraftForge/MinecraftForge/issues"
showAsResourcePack=false
clientSideOnly=false

[[mods]]
  modId="examplemod"
  version="1.0.0.0"
  displayName="Example Mod"
  updateJSONURL="https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json"
  displayURL="https://minecraftforge.net"
  logoFile="logo.png"
  credits="I'd like to thank my mother and father."
  authors="Author"
  description='''
  Lets you craft dirt into diamonds. This is a traditional mod that has existed for eons. It is ancient. The holy Notch created it. Jeb rainbowfied it. Dinnerbone made it upside down. Etc.
  '''
  displayTest="MATCH_VERSION"

[[dependencies.examplemod]]
  modId="forge"
  mandatory=true
  versionRange="[52,)"
  ordering="NONE"
  side="BOTH"

[[dependencies.examplemod]]
  modId="minecraft"
  mandatory=true
  versionRange="[1.21.1,)"
  ordering="NONE"
  side="BOTH"
"#;
        let file_name = "test.toml".to_string();
        let toml: ForgeMod = toml::from_str(toml_content)
            .with_context(|| format!("Failed to parse Forge mods.toml from {}", file_name))?;

        let mut all_metadata = Vec::new();

        for mod_entry in &toml.mods {
            let metadata = ModMetadata {
                mod_id: mod_entry.mod_id.clone(),
                version: mod_entry.version.clone(),
                name: mod_entry.display_name.clone(),
                description: mod_entry.description.clone(),
                authors: parse_authors(&mod_entry.authors),
                platform: Platform::Forge,
                dependencies: parse_forge_dependencies(&toml),
                file_name: file_name.clone(),
            };
            all_metadata.push(metadata);
        }

        assert_eq!(all_metadata.len(), 1);
        let first_mod = &all_metadata[0];
        assert_eq!(first_mod.mod_id, "examplemod");
        assert_eq!(first_mod.version, "1.0.0.0");
    }
}