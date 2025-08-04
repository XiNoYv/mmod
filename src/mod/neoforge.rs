use std::fs::File;
use serde::Deserialize;
use anyhow::{Context, Result};
use zip::ZipArchive;
use std::io::Read;
use crate::r#mod::{DependencyVersionRange, ModDependency, ModMetadata, Platform};
use super::forge::{Authors, parse_authors};

// https://docs.neoforged.net/docs/gettingstarted/modfiles#neoforgemodstoml
#[derive(Debug, Deserialize)]
pub struct NeoForgeMod {
    /// The language loader used by the mod(s).
    /// Can be used to support alternative language structures,
    /// such as Kotlin objects for the main file,
    /// or different methods of determining the entrypoint,
    /// such as an interface or method.
    /// NeoForge provides the Java loader `javafml`.
    #[serde(rename = "modLoader")]
    pub mod_loader: String,
    /// The acceptable version range of the language loader,
    /// expressed as a Maven Version Range.
    /// For `javafml`, this is currently version 1.
    /// If no version is specified,then any version of the mod loader can be used.
    #[serde(rename = "loaderVersion")]
    pub loader_version: String,
    /// The license the mod(s) in this JAR are provided under.
    #[serde(rename = "license")]
    pub license: String,
    /// When `true`, the mod(s)’s resources will be displayed as
    /// a separate resource pack on the ‘Resource Packs’ menu,
    /// rather than being combined with the `Mod resources` pack.
    #[serde(rename = "showAsResourcePack")]
    pub show_as_resource_pack: Option<bool>,
    /// When `true`, the mod(s)’s data file will be displayed as
    /// a separate data pack on the ‘Data Packs’ menu,
    /// rather than being combined with the `Mod Data` pack.
    #[serde(rename = "showAsDataPack")]
    pub show_as_data_pack: Option<bool>,
    /// An array of services your mod uses.
    /// This is consumed as part of the created module for the mod
    /// from NeoForge's implementation of the Java Platform Module System.
    pub services: Option<Vec<String>>,
    /// A table of substitution properties.
    /// This is used by `StringSubstitutor` to replace `${file.<key>}` with its corresponding value.
    // pub properties,
    /// Mod-specific properties are tied to the specified mod using the `[[mods]]` header.
    /// This is an array of tables;
    /// A URL representing the place to report and track issues with the mod(s).
    #[serde(rename = "issueTrackerURL")]
    pub issue_tracker_url: Option<String>,
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
    /// Must also be a valid mod ID, but may additionally include dots or dashes.
    /// Currently unused.
    // #[serde(rename = "namespace")]
    // pub namespace: Option<String>,
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
    /// The location must be an absolute path starting from the root of the JAR or source set
    /// (e.g. ·src/main/resources` for the main source set).
    /// Valid filename characters are lowercase letters (a-z), digits (0-9), slashes, (/),
    /// underscores (_), periods (.) and hyphens (-). The complete character set is [a-z0-9_-.].
    #[serde(rename = "logoFile")]
    pub logo_file: Option<String>,
    /// Whether to use `GL_LINEAR*` (true) or `GL_NEAREST*` (false) to render the logoFile.
    #[serde(rename = "logoBlur")]
    pub logo_blur: Option<bool>,
    /// A URL to a JSON used by the [update checker](https://docs.neoforged.net/docs/misc/updatechecker/)
    /// to make sure the mod you are playing is the latest version.
    #[serde(rename = "updateJSONURL")]
    pub update_json_url: Option<String>,
    // pub features,
    /// A table of key/values associated with this mod.
    /// Unused by NeoForge, but is mainly for use by mods.
    // pub modproperties,
    /// A URL to the download page of the mod. Currently unused.
    // #[serde(rename = "modUrl")]
    // pub mod_url: Option<String>,
    /// Credits and acknowledges for the mod shown on the mod list screen.
    #[serde(rename = "credits")]
    pub credits: Option<String>,
    /// The authors of the mod shown on the mod list screen.
    #[serde(rename = "authors")]
    pub authors: Option<Authors>,
    /// A URL to the display page of the mod shown on the mod list screen.
    #[serde(rename = "displayURL")]
    pub display_url: Option<String>,
    /// The file path of a JSON file used for enum extension
    #[serde(rename = "enumExtensions")]
    pub enum_extensions: Option<String>,
    /// The file path of a JSON file used for feature flags
    #[serde(rename = "featureFlags")]
    pub feature_flags: Option<String>,
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
    /// Specifies the nature of this dependency:
    /// `"required"` is the default and prevents the mod from loading if this dependency is missing;
    /// `"optional"` will not prevent the mod from loading if the dependency is missing,
    /// but still validates that the dependency is compatible;
    /// `"incompatible"` prevents the mod from loading if this dependency is present;
    /// `"discouraged"` still allows the mod to load if the dependency is present, but presents a warning to the user.
    #[serde(rename = "type")]
    pub r#type: String,
    /// An optional user-facing message to describe why this dependency is required,
    /// or why it is incompatible.
    #[serde(rename = "reason")]
    pub reason: Option<String>,
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
    // A URL to the download page of the dependency. Currently unused.
    // #[serde(rename = "referralUrl")]
    // pub referral_url: Option<String>,
}

pub fn parse_neoforge_mod_contents(jar_file: &mut ZipArchive<File>, file_name: &String) -> Result<Vec<ModMetadata>> {
    let mut file = jar_file.by_name("META-INF/neoforge.mods.toml")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    drop(file);
    let toml: NeoForgeMod = toml::from_str(contents.as_str())
        .with_context(|| format!("Failed to parse NeoForge mods.toml from {}", file_name))?;

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
            platform: Platform::NeoForge,
            dependencies: parse_neoforge_dependencies(&toml),
            file_name: file_name.clone(),
        };
        all_metadata.push(metadata);
    }

    Ok(all_metadata)
}

fn parse_neoforge_dependencies(toml: &NeoForgeMod) -> Vec<ModDependency> {
    let Some(deps) = &toml.dependencies else { return Vec::new() };

    let entries: Vec<_> = match deps {
        Dependencies::SingleMod(entries) => entries.iter().collect(),
        Dependencies::MultiMod(map) => map.values().flatten().collect(),
    };

    entries.iter().map(|entry| ModDependency {
        mod_id: entry.mod_id.clone(),
        version_range: DependencyVersionRange::Single(entry.version_range.clone()),
        mandatory: entry.r#type == "required"
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_forge_mod_contents() {
        let toml_content = r#"
modLoader="javafml"
loaderVersion="[1,)"
license="${mod_license}"
issueTrackerURL="https://change.me.to.your.issue.tracker.example.invalid/"

[[mods]]
modId="examplemod"
version="1.8.2"
displayName="Example Mod"
displayURL="https://minecraftforge.net"
logoFile="icon.png"
authors="Author"
description='''
Lets you craft dirt into diamonds. This is a traditional mod that has existed for eons. It is ancient. The holy Notch created it. Jeb rainbowfied it. Dinnerbone made it upside down. Etc.
'''
displayTest="IGNORE_ALL_VERSION"
[[mixins]]
config="entityculling.mixins.json"
[[dependencies.examplemod]]
    modId="minecraft"
    type="required"
    versionRange="[1.21]"
    ordering="NONE"
    side="BOTH"
[[dependencies.examplemod]]
    modId="neoforge"
    type="required"
    versionRange="[20.2,)"
    ordering="NONE"
    side="BOTH"

"#;
        let file_name = "test.toml".to_string();
        let toml: NeoForgeMod = toml::from_str(toml_content)
            .with_context(|| format!("Failed to parse Forge mods.toml from {}", file_name))
            .unwrap();

        let mut all_metadata = Vec::new();

        for mod_entry in &toml.mods {
            let metadata = ModMetadata {
                mod_id: mod_entry.mod_id.clone(),
                version: mod_entry.version.clone(),
                name: mod_entry.display_name.clone(),
                description: mod_entry.description.clone(),
                authors: parse_authors(&mod_entry.authors),
                platform: Platform::Forge,
                dependencies: parse_neoforge_dependencies(&toml),
                file_name: file_name.clone(),
            };
            all_metadata.push(metadata);
        }

        assert_eq!(all_metadata.len(), 1);
        let first_mod = &all_metadata[0];
        assert_eq!(first_mod.mod_id, "examplemod");
        assert_eq!(first_mod.version, "1.8.2");
    }
}