//! Module package format and loading.
//!
//! A reusable module ships as a self-contained folder package: a directory
//! `<name>/` containing an entry YAML `<name>.yaml` whose file name mirrors the
//! folder name, plus any co-located resources. Package entries are kernel graph
//! definitions loaded with package-root resource provenance. Nested package
//! references resolve into the same [`crate::kernel::DefinitionRegistry`].

use std::collections::{BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostics::{Diagnostic, Severity, error_codes};
use crate::kernel::document::load_kernel_definition_str;
use crate::kernel::{DefinitionRegistry, GraphDefinition, ResourceOrigin};
use crate::module_reference::{self, ModuleReferenceError};
use crate::preparation::PreparationContext;

/// Extension of a module package entry YAML file.
pub const PACKAGE_ENTRY_EXTENSION: &str = "yaml";

/// A package loaded through the unified kernel parser, together with all inline
/// and recursively referenced definitions needed to validate and flatten it.
#[derive(Clone, Debug)]
pub struct LoadedKernelPackage {
    definition: GraphDefinition,
    registry: DefinitionRegistry,
    root: PathBuf,
}

impl LoadedKernelPackage {
    pub fn definition(&self) -> &GraphDefinition {
        &self.definition
    }

    pub fn registry(&self) -> &DefinitionRegistry {
        &self.registry
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Failure to load a module package.
#[derive(Clone, Debug, PartialEq)]
pub enum ModulePackageError {
    /// The reference could not be resolved to an on-disk path.
    Reference(ModuleReferenceError),
    /// The entry YAML could not be read.
    ReadFailed { path: PathBuf, message: String },
    /// The entry file name does not mirror its folder name.
    NameMismatch { path: PathBuf, expected: String },
    /// Kernel graph-definition parsing or validation failed.
    Kernel(Diagnostic),
}

impl ModulePackageError {
    /// Renders the error as a validation diagnostic.
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            Self::Reference(error) => error.to_diagnostic(),
            Self::ReadFailed { path, message } => Diagnostic::new(
                error_codes::LIBRARY_PACKAGE_READ_FAILED,
                Severity::Error,
                format!(
                    "failed to read module package {}: {message}",
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
            Self::Kernel(diagnostic) => diagnostic.clone(),
        }
    }
}

impl From<ModuleReferenceError> for ModulePackageError {
    fn from(error: ModuleReferenceError) -> Self {
        Self::Reference(error)
    }
}

/// Resolves and loads a package entry directly as a kernel graph definition.
/// External references found in that definition or its inline definitions are
/// recursively loaded through the same preparation context.
pub fn load_referenced_kernel_package(
    reference: &str,
    context: &PreparationContext,
) -> Result<LoadedKernelPackage, ModulePackageError> {
    let entry = module_reference::resolve(reference, context.macro_roots())?;
    let root = validate_package_entry_path(&entry)?;
    let (definition, mut registry) = load_kernel_entry(reference, &entry, &root)?;
    registry = registry.with_definition(definition.clone());

    let mut references = external_references(&definition, &registry);
    while let Some(nested_reference) = references.pop_front() {
        if registry.get(&nested_reference).is_some() {
            continue;
        }
        let nested_entry = module_reference::resolve(&nested_reference, context.macro_roots())?;
        let nested_root = validate_package_entry_path(&nested_entry)?;
        let (nested, nested_registry) =
            load_kernel_entry(&nested_reference, &nested_entry, &nested_root)?;
        for inline in nested_registry.definitions() {
            registry = registry.with_definition(inline.clone());
        }
        references.extend(external_references(&nested, &nested_registry));
        registry = registry.with_definition(nested);
    }

    Ok(LoadedKernelPackage {
        definition,
        registry,
        root,
    })
}

fn validate_package_entry_path(path: &Path) -> Result<PathBuf, ModulePackageError> {
    let root = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    if let Some(folder_name) = root.file_name().and_then(|name| name.to_str()) {
        let expected = format!("{folder_name}.{PACKAGE_ENTRY_EXTENSION}");
        if path.file_name().and_then(|name| name.to_str()) != Some(expected.as_str()) {
            return Err(ModulePackageError::NameMismatch {
                path: path.to_path_buf(),
                expected: folder_name.to_string(),
            });
        }
    }
    Ok(root)
}

fn load_kernel_entry(
    reference: &str,
    entry: &Path,
    root: &Path,
) -> Result<(GraphDefinition, DefinitionRegistry), ModulePackageError> {
    let yaml = fs::read_to_string(entry).map_err(|error| ModulePackageError::ReadFailed {
        path: entry.to_path_buf(),
        message: error.to_string(),
    })?;
    let package = load_kernel_definition_str(
        &yaml,
        reference,
        ResourceOrigin::Package(root.to_path_buf()),
    )
    .map_err(|diagnostics| {
        ModulePackageError::Kernel(
            diagnostics
                .all()
                .first()
                .cloned()
                .expect("kernel parse failures always include a diagnostic"),
        )
    })?;
    Ok((package.root().clone(), package.registry().clone()))
}

fn external_references(root: &GraphDefinition, registry: &DefinitionRegistry) -> VecDeque<String> {
    std::iter::once(root)
        .chain(registry.definitions())
        .flat_map(|definition| definition.nodes())
        .map(|node| node.definition_ref())
        .filter(|reference| module_reference::is_external_reference(reference))
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::builtins::SAMPLE_RESOURCE_PARAM;
    use crate::kernel::document::load_kernel_patch_str;
    use crate::kernel::{ResourceKind, ResourceOrigin, StaticValue};
    use crate::module_reference::{LIB_MACRO, MacroRoots};
    use crate::preparation::PreparationContext;
    use std::fs;
    use std::path::PathBuf;

    fn seed_kernel_package(lib_root: &Path, version: &str, name: &str, yaml: &str) -> PathBuf {
        let package_dir = lib_root.join(version).join(name);
        fs::create_dir_all(&package_dir).expect("kernel package dir should be created");
        let entry = package_dir.join(format!("{name}.yaml"));
        fs::write(&entry, yaml).expect("kernel package entry should be written");
        entry
    }

    fn kernel_package_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "dandrum-kernel-package-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn packaged_kernel_definition_is_equivalent_to_inline_definition() {
        const DEFINITION: &str = r#"
ports:
  - { name: audio_in, direction: input, signal: audio, channels: 1, maps_to: amp.audio_in }
  - { name: audio_out, direction: output, signal: audio, channels: 1, maps_from: amp.audio_out }
modules:
  - { id: amp, type: gain, defaults: { gain: 0.5 } }
connections: []
"#;
        let lib_root = kernel_package_root("equivalent");
        seed_kernel_package(&lib_root, "1.0.0", "half_gain", DEFINITION);
        let reference = "$LIB/1.0.0/half_gain/half_gain.yaml";
        let context = PreparationContext::new(&lib_root, 48_000)
            .with_macro_roots(MacroRoots::new().with_root(LIB_MACRO, &lib_root));

        let packaged = load_referenced_kernel_package(reference, &context)
            .expect("kernel package should load directly");
        let inline =
            load_kernel_patch_str(&format!("metadata: {{ name: {reference} }}\n{DEFINITION}"))
                .expect("equivalent inline definition should load");

        assert_eq!(packaged.definition(), inline.root());
        assert_eq!(
            packaged
                .definition()
                .flatten(packaged.registry())
                .expect("packaged definition should flatten"),
            inline
                .root()
                .flatten(inline.registry())
                .expect("inline definition should flatten")
        );
    }

    #[test]
    fn nested_kernel_package_references_resolve_recursively() {
        const INNER: &str = r#"
static_params:
  - name: sample
    type: resource
    resource_kind: sample
    default: { kind: sample, path: samples/inner.wav }
ports:
  - { name: audio_out, direction: output, signal: audio, channels: 1, maps_from: player.audio }
modules:
  - { id: player, type: sampler, static: { sample: $sample } }
connections: []
"#;
        const OUTER: &str = r#"
ports:
  - { name: audio_out, direction: output, signal: audio, channels: 1, maps_from: inner.audio_out }
modules:
  - { id: inner, type: $USER_LIB/2.0.0/inner/inner.yaml }
connections: []
"#;
        let lib_root = kernel_package_root("nested-lib");
        let user_root = kernel_package_root("nested-user");
        seed_kernel_package(&lib_root, "1.0.0", "outer", OUTER);
        seed_kernel_package(&user_root, "2.0.0", "inner", INNER);
        let reference = "$LIB/1.0.0/outer/outer.yaml";
        let context = PreparationContext::new(&lib_root, 48_000).with_macro_roots(
            MacroRoots::new()
                .with_root(LIB_MACRO, &lib_root)
                .with_root(crate::module_reference::USER_LIB_MACRO, &user_root),
        );

        let package = load_referenced_kernel_package(reference, &context)
            .expect("nested kernel packages should resolve");
        let flattened = package
            .definition()
            .flatten(package.registry())
            .expect("nested package should flatten");

        assert_eq!(flattened.nodes().len(), 1);
        assert_eq!(flattened.nodes()[0].id().as_str(), "inner::player");
        assert_eq!(flattened.nodes()[0].definition(), "sampler");
        let StaticValue::Resource(resource) =
            &flattened.nodes()[0].static_args()[SAMPLE_RESOURCE_PARAM]
        else {
            panic!("nested sampler should retain its resource")
        };
        assert_eq!(
            resource.origin(),
            &ResourceOrigin::Package(user_root.join("2.0.0").join("inner"))
        );
    }

    #[test]
    fn package_resource_defaults_and_literals_retain_concrete_origins() {
        const PACKAGE: &str = r#"
static_params:
  - name: default_sample
    type: resource
    resource_kind: sample
    default: { kind: sample, path: samples/default.wav }
ports:
  - { name: trigger, direction: input, signal: event, channels: 1, maps_to: default_player.trigger }
  - { name: audio, direction: output, signal: audio, channels: 1, maps_from: default_player.audio }
modules:
  - id: default_player
    type: sampler
    static:
      sample: $default_sample
  - id: literal_player
    type: sampler
    static:
      sample: { kind: sample, path: samples/literal.wav }
connections: []
"#;
        let lib_root = kernel_package_root("origins");
        let entry = seed_kernel_package(&lib_root, "1.2.3", "kit", PACKAGE);
        let package_root = entry.parent().unwrap().to_path_buf();
        let reference = "$LIB/1.2.3/kit/kit.yaml";
        let context = PreparationContext::new(&lib_root, 48_000)
            .with_macro_roots(MacroRoots::new().with_root(LIB_MACRO, &lib_root));

        let package = load_referenced_kernel_package(reference, &context)
            .expect("resource-bearing kernel package should load");
        let flattened = package
            .definition()
            .flatten(package.registry())
            .expect("resource-bearing package should flatten");

        for (node_id, expected_path) in [
            ("default_player", "samples/default.wav"),
            ("literal_player", "samples/literal.wav"),
        ] {
            let StaticValue::Resource(resource) = &flattened
                .node(&crate::kernel::NodeId::new(node_id))
                .unwrap()
                .static_args()[SAMPLE_RESOURCE_PARAM]
            else {
                panic!("{node_id} should retain a typed sample resource")
            };
            assert_eq!(resource.kind(), ResourceKind::Sample);
            assert_eq!(resource.path(), Path::new(expected_path));
            assert_eq!(
                resource.origin(),
                &ResourceOrigin::Package(package_root.clone())
            );
        }
    }

    #[test]
    fn pinned_package_version_provenance_survives_flattening() {
        const PACKAGE: &str = r#"
static_params:
  - name: sample
    type: resource
    resource_kind: sample
    default: { kind: sample, path: samples/hit.wav }
ports:
  - { name: audio, direction: output, signal: audio, channels: 1, maps_from: player.audio }
modules:
  - { id: player, type: sampler, static: { sample: $sample } }
connections: []
"#;
        let lib_root = kernel_package_root("pinned");
        let pinned_entry = seed_kernel_package(&lib_root, "1.0.0", "voice", PACKAGE);
        seed_kernel_package(&lib_root, "2.0.0", "voice", PACKAGE);
        let context = PreparationContext::new(&lib_root, 48_000)
            .with_macro_roots(MacroRoots::new().with_root(LIB_MACRO, &lib_root));

        let package = load_referenced_kernel_package("$LIB/1.0.0/voice/voice.yaml", &context)
            .expect("pinned package should load");
        let flattened = package
            .definition()
            .flatten(package.registry())
            .expect("pinned package should flatten");
        let StaticValue::Resource(resource) =
            &flattened.nodes()[0].static_args()[SAMPLE_RESOURCE_PARAM]
        else {
            panic!("sampler should carry its package resource")
        };

        assert_eq!(
            resource.origin(),
            &ResourceOrigin::Package(pinned_entry.parent().unwrap().to_path_buf())
        );
    }

    #[test]
    fn package_entry_rejects_asset_bindings() {
        let lib_root = kernel_package_root("no-legacy-expansion");
        seed_kernel_package(
            &lib_root,
            "1.0.0",
            "legacy",
            r#"
asset_bindings:
  - { name: sample, maps_to: player.asset }
ports:
  - { name: audio, direction: output, signal: audio, channels: 1, maps_from: player.audio }
modules:
  - { id: player, type: sampler }
connections: []
"#,
        );
        let context = PreparationContext::new(&lib_root, 48_000)
            .with_macro_roots(MacroRoots::new().with_root(LIB_MACRO, &lib_root));

        let error = load_referenced_kernel_package("$LIB/1.0.0/legacy/legacy.yaml", &context)
            .expect_err("package entries must reject asset_bindings");

        assert_eq!(
            error.to_diagnostic().error_code(),
            crate::diagnostics::error_codes::KERNEL_DOCUMENT_LEGACY_ASSET_BINDINGS
        );
    }
}
