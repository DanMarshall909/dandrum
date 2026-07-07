//! Module package format and loading.
//!
//! A reusable module ships as a self-contained folder package: a directory
//! `<name>/` containing an entry YAML `<name>.yaml` whose file name mirrors the
//! folder name, plus any co-located resources. The entry YAML carries the same
//! fields as an inline `module_definitions` entry (public inputs/outputs,
//! parameter and asset bindings, internal modules and connections) minus the
//! `type`, which is supplied by the macro-qualified reference that names it.
//!
//! Loading a referenced package resolves it through [`crate::module_reference`],
//! parses its entry YAML, rebases its declared resources onto the package root
//! so the package is portable, and injects it as an inline defined-module so it
//! expands through the one existing expansion path in
//! [`crate::graph_module`] — there is no second expansion implementation.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::diagnostics::{Diagnostic, Severity, error_codes};
use crate::module_reference::{self, MacroRoots, ModuleReferenceError};
use crate::patch::{
    AssetDeclaration, ConnectionDeclaration, ModuleBindingDeclaration, ModuleDeclaration,
    ModuleDefinitionDeclaration, ModuleInputDeclaration, ModuleOutputDeclaration, PatchDocument,
};

/// Extension of a module package entry YAML file.
pub const PACKAGE_ENTRY_EXTENSION: &str = "yaml";

/// Parsed entry YAML of a module package.
///
/// Mirrors the body of a single `module_definitions` entry. The `type` is not
/// stored in the package; it is the macro-qualified reference that names the
/// package, so the same package can be pinned at any version or root.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct ModulePackageDocument {
    #[serde(default)]
    pub inputs: Vec<ModuleInputDeclaration>,
    #[serde(default)]
    pub outputs: Vec<ModuleOutputDeclaration>,
    #[serde(default)]
    pub parameters: Vec<ModuleBindingDeclaration>,
    #[serde(default)]
    pub asset_bindings: Vec<ModuleBindingDeclaration>,
    /// Package-owned resources whose paths are relative to the package root.
    #[serde(default)]
    pub assets: Vec<AssetDeclaration>,
    #[serde(default)]
    pub modules: Vec<ModuleDeclaration>,
    #[serde(default)]
    pub connections: Vec<ConnectionDeclaration>,
}

/// A loaded package together with the on-disk root its resources resolve against.
#[derive(Clone, Debug, PartialEq)]
pub struct LoadedPackage {
    document: ModulePackageDocument,
    root: PathBuf,
}

impl LoadedPackage {
    pub fn document(&self) -> &ModulePackageDocument {
        &self.document
    }

    /// Package root directory; co-located resources resolve relative to it.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Failure to load a module package.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModulePackageError {
    /// The reference could not be resolved to an on-disk path.
    Reference(ModuleReferenceError),
    /// The entry YAML could not be read.
    ReadFailed { path: PathBuf, message: String },
    /// The entry YAML could not be parsed.
    ParseFailed { path: PathBuf, message: String },
    /// The entry file name does not mirror its folder name.
    NameMismatch { path: PathBuf, expected: String },
    /// A package-internal resource path tries to escape the package root.
    ResourcePathEscape { reference: String, resource: String },
}

impl ModulePackageError {
    /// Renders the error as a validation diagnostic.
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            Self::Reference(error) => error.to_diagnostic(),
            Self::ReadFailed { path, message } => Diagnostic::new(
                error_codes::LIBRARY_PACKAGE_READ_FAILED,
                Severity::Error,
                format!("failed to read module package {}: {message}", path.display()),
            ),
            Self::ParseFailed { path, message } => Diagnostic::new(
                error_codes::LIBRARY_PACKAGE_PARSE_FAILED,
                Severity::Error,
                format!(
                    "failed to parse module package {}: {message}",
                    path.display()
                ),
            ),
            Self::NameMismatch { path, expected } => Diagnostic::new(
                error_codes::LIBRARY_PACKAGE_NAME_MISMATCH,
                Severity::Error,
                format!(
                    "module package entry {} must be named {expected}.{PACKAGE_ENTRY_EXTENSION} to mirror its folder",
                    path.display()
                ),
            ),
            Self::ResourcePathEscape {
                reference,
                resource,
            } => Diagnostic::new(
                error_codes::LIBRARY_PATH_ESCAPE,
                Severity::Error,
                format!(
                    "module package {reference} resource {resource} escapes its package root"
                ),
            ),
        }
    }
}

impl From<ModuleReferenceError> for ModulePackageError {
    fn from(error: ModuleReferenceError) -> Self {
        Self::Reference(error)
    }
}

/// Loads the module package at an already-resolved entry-YAML `path`.
///
/// Enforces the packaging invariant that the entry file name mirrors its folder
/// name, so a package is a single self-describing directory.
pub fn load_package(path: &Path) -> Result<LoadedPackage, ModulePackageError> {
    let root = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    if let Some(folder_name) = root.file_name().and_then(|name| name.to_str()) {
        let stem = path.file_stem().and_then(|stem| stem.to_str());
        if stem != Some(folder_name) {
            return Err(ModulePackageError::NameMismatch {
                path: path.to_path_buf(),
                expected: folder_name.to_string(),
            });
        }
    }

    let yaml = fs::read_to_string(path).map_err(|error| ModulePackageError::ReadFailed {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;

    let document =
        serde_yaml::from_str(&yaml).map_err(|error| ModulePackageError::ParseFailed {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

    Ok(LoadedPackage { document, root })
}

/// Resolves `reference` against `roots` and loads the package it names.
pub fn load_referenced_package(
    reference: &str,
    roots: &MacroRoots,
) -> Result<LoadedPackage, ModulePackageError> {
    let path = module_reference::resolve(reference, roots)?;
    load_package(&path)
}

/// Returns a copy of `patch` with every external module reference resolved,
/// loaded, and injected as an inline defined module.
///
/// Each distinct `$`-qualified module `type` referenced by an instance is loaded
/// once, its resources rebased onto its package root, and appended as a
/// `module_definitions` entry keyed by the reference string, so the existing
/// defined-module expansion turns instances of it into their constituent
/// modules and connections — behaving identically to an inline definition.
pub fn expand_external_references(
    patch: &PatchDocument,
    roots: &MacroRoots,
) -> Result<PatchDocument, ModulePackageError> {
    let references = patch
        .modules
        .iter()
        .map(|module| module.module_type.as_str())
        .filter(|module_type| module_reference::is_external_reference(module_type))
        .collect::<BTreeSet<_>>();

    if references.is_empty() {
        return Ok(patch.clone());
    }

    let mut expanded = patch.clone();

    for reference in references {
        let loaded = load_referenced_package(reference, roots)?;
        let LoadedPackage { document, root } = loaded;

        for asset in &document.assets {
            reject_resource_path_escape(reference, &asset.path)?;
            if expanded.assets.iter().any(|existing| existing.id == asset.id) {
                continue;
            }
            let mut rebased = asset.clone();
            rebased.path = root.join(&asset.path).to_string_lossy().into_owned();
            expanded.assets.push(rebased);
        }

        expanded
            .module_definitions
            .push(document.into_definition(reference.to_string()));
    }

    Ok(expanded)
}

fn reject_resource_path_escape(
    reference: &str,
    resource: &str,
) -> Result<(), ModulePackageError> {
    let is_escape = Path::new(resource)
        .components()
        .any(|component| !matches!(component, Component::Normal(_)));

    if is_escape {
        return Err(ModulePackageError::ResourcePathEscape {
            reference: reference.to_string(),
            resource: resource.to_string(),
        });
    }

    Ok(())
}

impl ModulePackageDocument {
    /// Converts the package into an inline defined module keyed by `module_type`.
    pub fn into_definition(self, module_type: String) -> ModuleDefinitionDeclaration {
        ModuleDefinitionDeclaration {
            module_type,
            inputs: self.inputs,
            outputs: self.outputs,
            parameters: self.parameters,
            asset_bindings: self.asset_bindings,
            modules: self.modules,
            connections: self.connections,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module_reference::LIB_MACRO;
    use crate::patch;
    use std::fs;
    use std::path::PathBuf;

    const DRUM_VOICE_PACKAGE: &str = r#"
inputs:
  - name: trigger
    signal_type: event
    maps_to:
      - env.gate
outputs:
  - name: audio
    signal_type: audio
    maps_from:
      - vca.audio_out
modules:
  - id: osc
    type: oscillator
  - id: env
    type: adsr
  - id: vca
    type: gain
connections:
  - from: osc.audio
    to: vca.audio_in
  - from: env.value
    to: vca.gain
"#;

    const HOST_PATCH: &str = r#"
metadata:
  name: External Reference Host
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 4800
modules:
  - id: midi
    type: midi_input
  - id: voice
    type: $LIB/1.3.9/drum_voice/drum_voice.yaml
  - id: mixer
    type: audio_mixer
  - id: out
    type: audio_output
connections:
  - from: midi.events
    to: voice.trigger
  - from: voice.audio
    to: mixer.inputs
  - from: mixer.mix
    to: out.left
  - from: mixer.mix
    to: out.right
"#;

    const INLINE_EQUIVALENT_PATCH: &str = r#"
metadata:
  name: External Reference Host
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 4800
module_definitions:
  - type: drum_voice
    inputs:
      - name: trigger
        signal_type: event
        maps_to:
          - env.gate
    outputs:
      - name: audio
        signal_type: audio
        maps_from:
          - vca.audio_out
    modules:
      - id: osc
        type: oscillator
      - id: env
        type: adsr
      - id: vca
        type: gain
    connections:
      - from: osc.audio
        to: vca.audio_in
      - from: env.value
        to: vca.gain
modules:
  - id: midi
    type: midi_input
  - id: voice
    type: drum_voice
  - id: mixer
    type: audio_mixer
  - id: out
    type: audio_output
connections:
  - from: midi.events
    to: voice.trigger
  - from: voice.audio
    to: mixer.inputs
  - from: mixer.mix
    to: out.left
  - from: mixer.mix
    to: out.right
"#;

    /// Writes `drum_voice/drum_voice.yaml` under a fresh temp `$LIB` root and
    /// returns `(lib_root, entry_path)`. The version segment is included so the
    /// reference in `HOST_PATCH` resolves.
    fn seed_drum_voice_package(tag: &str) -> (PathBuf, PathBuf) {
        let lib_root = std::env::temp_dir().join(format!(
            "dandrum-package-{tag}-{}",
            std::process::id()
        ));
        let package_dir = lib_root.join("1.3.9").join("drum_voice");
        fs::create_dir_all(&package_dir).expect("package dir should be created");
        let entry = package_dir.join("drum_voice.yaml");
        fs::write(&entry, DRUM_VOICE_PACKAGE).expect("entry yaml should be written");
        (lib_root, entry)
    }

    #[test]
    fn package_entry_loads_with_its_root() {
        let (_lib_root, entry) = seed_drum_voice_package("load");

        let loaded = load_package(&entry).expect("a well-formed package should load");

        assert_eq!(
            loaded.root(),
            entry.parent().expect("entry has a parent"),
            "the package root should be the entry file's directory"
        );
        assert_eq!(
            loaded.document().modules.len(),
            3,
            "the drum_voice package declares three internal modules"
        );
    }

    #[test]
    fn entry_file_not_mirroring_its_folder_is_rejected() {
        let lib_root = std::env::temp_dir().join(format!(
            "dandrum-package-mismatch-{}",
            std::process::id()
        ));
        let package_dir = lib_root.join("drum_voice");
        fs::create_dir_all(&package_dir).expect("package dir should be created");
        let entry = package_dir.join("voice.yaml");
        fs::write(&entry, DRUM_VOICE_PACKAGE).expect("entry yaml should be written");

        let error = load_package(&entry)
            .expect_err("an entry that does not mirror its folder should be rejected");

        assert!(
            matches!(error, ModulePackageError::NameMismatch { .. }),
            "a mismatched entry file name should report NameMismatch, got {error:?}"
        );
    }

    #[test]
    fn unknown_macro_reference_fails_to_load() {
        let roots = MacroRoots::new();

        let error = load_referenced_package("$LIB/1.3.9/drum_voice/drum_voice.yaml", &roots)
            .expect_err("an unconfigured macro should fail to load");

        assert!(
            matches!(
                error,
                ModulePackageError::Reference(ModuleReferenceError::UnknownMacro { .. })
            ),
            "an unknown macro should surface as a reference error, got {error:?}"
        );
    }

    #[test]
    fn external_reference_expands_identically_to_inline_definition() {
        let (lib_root, _entry) = seed_drum_voice_package("identical");
        let roots = MacroRoots::new().with_root(LIB_MACRO, &lib_root);

        let host = patch::load_patch_str(HOST_PATCH).expect("host patch should parse");
        let expanded =
            expand_external_references(&host, &roots).expect("external reference should expand");

        let inline =
            patch::load_patch_str(INLINE_EQUIVALENT_PATCH).expect("inline patch should parse");

        patch::validate_patch_schema(&expanded).expect("expanded patch should validate");
        let external_graph = crate::graph::Graph::from_patch_declarations(&expanded);
        let inline_graph = crate::graph::Graph::from_patch_declarations(&inline);

        assert_eq!(
            external_graph, inline_graph,
            "an external reference should build the same graph as the equivalent inline definition"
        );
    }

    #[test]
    fn patch_without_external_references_is_unchanged() {
        let inline =
            patch::load_patch_str(INLINE_EQUIVALENT_PATCH).expect("inline patch should parse");
        let roots = MacroRoots::new();

        let expanded = expand_external_references(&inline, &roots)
            .expect("a patch with no external references should pass through");

        assert_eq!(
            expanded, inline,
            "a patch with no external references should be returned unchanged"
        );
    }

    #[test]
    fn package_resources_rebase_onto_the_package_root() {
        let lib_root = std::env::temp_dir().join(format!(
            "dandrum-package-assets-{}",
            std::process::id()
        ));
        let package_dir = lib_root.join("1.0.0").join("clap");
        fs::create_dir_all(&package_dir).expect("package dir should be created");
        fs::write(
            package_dir.join("clap.yaml"),
            r#"
assets:
  - id: clap_sample
    kind: sample
    path: samples/clap.wav
modules:
  - id: player
    type: sampler
    parameters:
      asset: clap_sample
outputs:
  - name: audio
    signal_type: audio
    maps_from:
      - player.audio_out
"#,
        )
        .expect("entry yaml should be written");

        let reference = "$LIB/1.0.0/clap/clap.yaml";
        let host = patch::load_patch_str(&format!(
            r#"
metadata:
  name: Sampler Host
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 4800
modules:
  - id: clap
    type: {reference}
  - id: out
    type: audio_output
connections:
  - from: clap.audio
    to: out.left
"#
        ))
        .expect("host patch should parse");
        let roots = MacroRoots::new().with_root(LIB_MACRO, &lib_root);

        let expanded =
            expand_external_references(&host, &roots).expect("package should expand");

        let asset = expanded
            .assets
            .iter()
            .find(|asset| asset.id == "clap_sample")
            .expect("the package asset should be injected into the patch");
        assert_eq!(
            asset.path,
            package_dir
                .join("samples/clap.wav")
                .to_string_lossy()
                .into_owned(),
            "a package resource path should resolve relative to the package root"
        );
    }

    #[test]
    fn package_resource_escaping_its_root_is_rejected() {
        let lib_root = std::env::temp_dir().join(format!(
            "dandrum-package-escape-{}",
            std::process::id()
        ));
        let package_dir = lib_root.join("1.0.0").join("evil");
        fs::create_dir_all(&package_dir).expect("package dir should be created");
        fs::write(
            package_dir.join("evil.yaml"),
            r#"
assets:
  - id: stolen
    kind: sample
    path: ../../../../etc/passwd
modules:
  - id: player
    type: sampler
    parameters:
      asset: stolen
"#,
        )
        .expect("entry yaml should be written");

        let host = patch::load_patch_str(
            r#"
metadata:
  name: Escape Host
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 4800
modules:
  - id: evil
    type: $LIB/1.0.0/evil/evil.yaml
  - id: out
    type: audio_output
"#,
        )
        .expect("host patch should parse");
        let roots = MacroRoots::new().with_root(LIB_MACRO, &lib_root);

        let error = expand_external_references(&host, &roots)
            .expect_err("a resource that escapes the package root should be rejected");

        assert!(
            matches!(error, ModulePackageError::ResourcePathEscape { .. }),
            "an escaping resource path should report ResourcePathEscape, got {error:?}"
        );
    }
}
