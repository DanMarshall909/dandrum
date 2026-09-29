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
    let (entry, root) = resolve_contained_entry(reference, context)?;
    let (definition, mut registry) = load_kernel_entry(reference, &entry, &root)?;
    registry = registry.with_definition(definition.clone());

    let mut references = external_references(&definition, &registry);
    while let Some(nested_reference) = references.pop_front() {
        if registry.get(&nested_reference).is_some() {
            continue;
        }
        let (nested_entry, nested_root) = resolve_contained_entry(&nested_reference, context)?;
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

fn resolve_contained_entry(
    reference: &str,
    context: &PreparationContext,
) -> Result<(PathBuf, PathBuf), ModulePackageError> {
    let entry = module_reference::resolve(reference, context.macro_roots())?;
    validate_package_entry_path(&entry)?;
    let macro_name = reference
        .split(module_reference::REFERENCE_SEPARATOR)
        .next()
        .expect("validated reference has a macro name");
    let configured_root = context
        .macro_roots()
        .root(macro_name)
        .expect("resolved reference has a configured macro root");
    let canonical_root =
        fs::canonicalize(configured_root).map_err(|error| ModulePackageError::ReadFailed {
            path: configured_root.to_path_buf(),
            message: error.to_string(),
        })?;
    let canonical_entry =
        fs::canonicalize(&entry).map_err(|error| ModulePackageError::ReadFailed {
            path: entry.clone(),
            message: error.to_string(),
        })?;
    if !canonical_entry.starts_with(&canonical_root) {
        return Err(ModulePackageError::Reference(
            ModuleReferenceError::PathEscape {
                reference: reference.to_string(),
            },
        ));
    }
    let package_root = canonical_entry
        .parent()
        .expect("canonical package entry has a parent")
        .to_path_buf();
    Ok((canonical_entry, package_root))
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
    let local_names = package
        .local_definition_names()
        .iter()
        .map(|name| (name.clone(), format!("{reference}::{name}")))
        .collect::<std::collections::BTreeMap<_, _>>();
    let root = package
        .root()
        .with_scoped_definition_refs(reference, &local_names);
    let mut registry = DefinitionRegistry::new();
    for definition in package.registry().definitions() {
        registry = registry.with_definition(match local_names.get(definition.name()) {
            Some(qualified) => definition.with_scoped_definition_refs(qualified, &local_names),
            None => definition.clone(),
        });
    }
    Ok((root, registry))
}

pub(crate) fn external_references(
    root: &GraphDefinition,
    registry: &DefinitionRegistry,
) -> VecDeque<String> {
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
    fn package_entry_reports_name_and_read_failures() {
        let directory = tempfile::tempdir().expect("temporary package root");
        let folder = directory.path().join("voice");
        fs::create_dir_all(&folder).expect("create package folder");
        let mismatched = folder.join("other.yaml");
        let error =
            validate_package_entry_path(&mismatched).expect_err("entry must mirror folder name");
        assert_eq!(
            error.to_diagnostic().error_code(),
            error_codes::LIBRARY_PACKAGE_NAME_MISMATCH
        );
        assert!(error.to_diagnostic().message().contains("voice.yaml"));

        let missing = folder.join("voice.yaml");
        let error = load_kernel_entry("$LIB/voice/voice.yaml", &missing, &folder)
            .expect_err("missing entry cannot be loaded");
        assert_eq!(
            error.to_diagnostic().error_code(),
            error_codes::LIBRARY_PACKAGE_READ_FAILED
        );
        assert!(error.to_diagnostic().message().contains("voice.yaml"));
    }

    #[test]
    fn package_resolution_reports_missing_root_and_entry() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let missing_root = directory.path().join("missing");
        let context = PreparationContext::new(directory.path(), 48_000)
            .with_macro_roots(MacroRoots::new().with_root(LIB_MACRO, &missing_root));
        let reference = "$LIB/1.0.0/voice/voice.yaml";
        let error =
            resolve_contained_entry(reference, &context).expect_err("missing library root fails");
        assert_eq!(
            error.to_diagnostic().error_code(),
            error_codes::LIBRARY_PACKAGE_READ_FAILED
        );
        assert!(error.to_diagnostic().message().contains("missing"));

        fs::create_dir_all(&missing_root).expect("create library root");
        let error =
            resolve_contained_entry(reference, &context).expect_err("missing package entry fails");
        assert_eq!(
            error.to_diagnostic().error_code(),
            error_codes::LIBRARY_PACKAGE_READ_FAILED
        );
        assert!(error.to_diagnostic().message().contains("voice.yaml"));
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

        let packaged_metadata = packaged.registry().discover(reference).unwrap();
        assert_eq!(packaged_metadata, inline.root().metadata());
        assert_eq!(packaged_metadata.ports()[0].name(), "audio_in");

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

    #[test]
    fn external_library_and_user_modules_render_identically_to_inline_definition() {
        use crate::graph_processor::render_kernel_offline_named;
        use crate::patch::RenderSettings;
        use crate::preparation::{HostBuses, prepare_kernel_graph_with_buses_and_context};
        use crate::sample::PreparedSamplerAssets;

        const DEFINITION: &str = "ports:\n  - { name: audio, direction: output, signal: audio, channels: 1, maps_from: source.out }\nmodules:\n  - { id: source, type: control_to_audio, defaults: { in: -0.5 } }\nconnections: []\n";
        let directory = tempfile::tempdir().unwrap();
        let lib_root = directory.path().join("lib");
        let user_root = directory.path().join("user");
        seed_kernel_package(&lib_root, "1.0.0", "voice", DEFINITION);
        fs::create_dir_all(user_root.join("voice")).unwrap();
        fs::write(user_root.join("voice/voice.yaml"), DEFINITION).unwrap();
        let roots = MacroRoots::new()
            .with_root(LIB_MACRO, &lib_root)
            .with_root(crate::module_reference::USER_LIB_MACRO, &user_root);
        let context = PreparationContext::new(directory.path(), 48_000).with_macro_roots(roots);
        let settings = RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 8,
            duration_frames: 12,
        };
        let buses = HostBuses::new().with_output("master", 1);
        let indented_definition = DEFINITION
            .lines()
            .map(|line| format!("    {line}\n"))
            .collect::<String>();
        let inline_yaml = format!(
            "metadata: {{ name: inline }}\nports:\n  - {{ name: master, direction: output, signal: audio, channels: 1, maps_from: voice.audio }}\nmodule_definitions:\n  - type: voice\n{}modules:\n  - {{ id: voice, type: voice }}\nconnections: []\n",
            indented_definition
        );
        let inline = load_kernel_patch_str(&inline_yaml).unwrap();
        let render = |patch: &crate::kernel::document::KernelPatch| {
            let prepared = prepare_kernel_graph_with_buses_and_context(
                patch.root(),
                patch.registry(),
                &settings,
                &buses,
                &context,
            )
            .expect("external and inline modules should prepare");
            let outputs =
                render_kernel_offline_named(&prepared, vec![], &PreparedSamplerAssets::empty())
                    .expect("prepared module should render");
            let planes: Vec<_> = outputs[0].1.iter().map(Vec::as_slice).collect();
            let mut bytes = Vec::new();
            crate::wav::write_wav_channels_i16(&mut bytes, 48_000, &planes).unwrap();
            bytes
        };
        let inline_bytes = render(&inline);
        for reference in ["$LIB/1.0.0/voice/voice.yaml", "$USER_LIB/voice/voice.yaml"] {
            let external = load_kernel_patch_str(&format!(
                "metadata: {{ name: external }}\nports:\n  - {{ name: master, direction: output, signal: audio, channels: 1, maps_from: voice.audio }}\nmodules:\n  - {{ id: voice, type: {reference} }}\nconnections: []\n"
            ))
            .unwrap();
            assert_eq!(render(&external), inline_bytes, "{reference}");
        }
    }

    #[test]
    fn independent_packages_keep_same_named_private_definitions_and_root_helper_distinct() {
        use crate::graph_processor::render_kernel_offline_named;
        use crate::patch::RenderSettings;
        use crate::preparation::{HostBuses, prepare_kernel_graph_with_buses_and_context};
        use crate::sample::PreparedSamplerAssets;

        let directory = tempfile::tempdir().unwrap();
        let lib_root = directory.path().join("lib");
        for (name, value) in [("one", 0.25), ("two", -0.5)] {
            let package = format!(
                "ports:\n  - {{ name: audio, direction: output, signal: audio, channels: 1, maps_from: inner.audio }}\nmodule_definitions:\n  - type: helper\n    ports:\n      - {{ name: audio, direction: output, signal: audio, channels: 1, maps_from: source.out }}\n    modules:\n      - {{ id: source, type: control_to_audio, defaults: {{ in: {value} }} }}\n    connections: []\nmodules:\n  - {{ id: inner, type: helper }}\nconnections: []\n"
            );
            seed_kernel_package(&lib_root, "1.0.0", name, &package);
        }
        let patch = load_kernel_patch_str(
            "ports:\n  - { name: one, direction: output, signal: audio, channels: 1, maps_from: first.audio }\n  - { name: two, direction: output, signal: audio, channels: 1, maps_from: second.audio }\n  - { name: root, direction: output, signal: audio, channels: 1, maps_from: local.audio }\nmodule_definitions:\n  - type: helper\n    ports:\n      - { name: audio, direction: output, signal: audio, channels: 1, maps_from: source.out }\n    modules:\n      - { id: source, type: control_to_audio, defaults: { in: 0.75 } }\n    connections: []\nmodules:\n  - { id: first, type: $LIB/1.0.0/one/one.yaml }\n  - { id: second, type: $LIB/1.0.0/two/two.yaml }\n  - { id: local, type: helper }\nconnections: []\n",
        )
        .unwrap();
        let context = PreparationContext::new(directory.path(), 48_000)
            .with_macro_roots(MacroRoots::new().with_root(LIB_MACRO, &lib_root));
        let settings = RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 8,
            duration_frames: 8,
        };
        let prepared = prepare_kernel_graph_with_buses_and_context(
            patch.root(),
            patch.registry(),
            &settings,
            &HostBuses::new()
                .with_output("one", 1)
                .with_output("two", 1)
                .with_output("root", 1),
            &context,
        )
        .unwrap();
        let outputs =
            render_kernel_offline_named(&prepared, vec![], &PreparedSamplerAssets::empty())
                .unwrap();
        assert_eq!(outputs[0], ("one".to_string(), vec![vec![0.25; 8]]));
        assert_eq!(outputs[1], ("two".to_string(), vec![vec![-0.5; 8]]));
        assert_eq!(outputs[2], ("root".to_string(), vec![vec![0.75; 8]]));
    }

    #[test]
    fn packaged_poly_resolves_its_private_voice_definition() {
        use crate::graph_processor::RealtimeGraphProcessor;
        use crate::patch::RenderSettings;
        use crate::preparation::{HostBuses, prepare_kernel_graph_with_buses_and_context};
        use crate::sample::PreparedSamplerAssets;

        let directory = tempfile::tempdir().unwrap();
        let lib_root = directory.path().join("lib");
        seed_kernel_package(
            &lib_root,
            "1.0.0",
            "poly_voice",
            "ports:\n  - { name: master, direction: output, signal: audio, channels: 1, maps_from: voices.audio }\nmodule_definitions:\n  - type: helper\n    ports:\n      - { name: audio, direction: output, signal: audio, channels: 1, maps_from: source.out }\n    modules:\n      - { id: source, type: control_to_audio, defaults: { in: -0.25 } }\n    connections: []\nmodules:\n  - { id: voices, type: poly, static: { definition: helper, max_voices: 1, allocation: reject-new } }\nconnections: []\n",
        );
        let context = PreparationContext::new(directory.path(), 48_000)
            .with_macro_roots(MacroRoots::new().with_root(LIB_MACRO, &lib_root));
        let package =
            load_referenced_kernel_package("$LIB/1.0.0/poly_voice/poly_voice.yaml", &context)
                .unwrap();
        let prepared = prepare_kernel_graph_with_buses_and_context(
            package.definition(),
            package.registry(),
            &RenderSettings {
                sample_rate_hz: 48_000,
                block_size_frames: 8,
                duration_frames: 8,
            },
            &HostBuses::new().with_output("master", 1),
            &context,
        )
        .unwrap();
        let mut runtime = RealtimeGraphProcessor::from_compiled_patch(
            prepared.compiled_patch().clone(),
            48_000.0,
            &PreparedSamplerAssets::empty(),
            8,
        );
        let mut outputs = vec![vec![vec![0.0; 8]]];
        runtime.note_on_at(60, 100, 0);
        assert_eq!(runtime.render_root_outputs(&mut outputs), 8);
        assert_eq!(outputs[0][0], [-0.25; 8]);
    }

    #[test]
    fn caller_resource_override_keeps_document_origin_through_package_pass_through() {
        use crate::graph_processor::render_kernel_offline_named;
        use crate::patch::RenderSettings;
        use crate::preparation::{HostBuses, prepare_kernel_graph_with_buses_and_context};
        use crate::sample::PreparedSamplerAssets;

        const PACKAGE: &str = "static_params:\n  - name: sample\n    type: resource\n    resource_kind: sample\n    default: { kind: sample, path: samples/default.wav }\nports:\n  - { name: audio, direction: output, signal: audio, channels: 1, maps_from: player.audio }\nmodules:\n  - { id: midi, type: midi_input }\n  - { id: player, type: sampler, static: { sample: $sample } }\nconnections:\n  - { from: midi.events, to: player.trigger }\n";
        let directory = tempfile::tempdir().expect("isolated document and library roots");
        let lib_root = directory.path().join("lib");
        let entry = seed_kernel_package(&lib_root, "1.0.0", "voice", PACKAGE);
        let package_sample = entry.parent().unwrap().join("samples/default.wav");
        fs::create_dir_all(package_sample.parent().unwrap()).unwrap();
        for (path, value) in [
            (package_sample, 0.25),
            (directory.path().join("caller.wav"), -0.5),
        ] {
            crate::wav::write_wav_stereo_i16(
                fs::File::create(path).unwrap(),
                48_000,
                &[value],
                &[value],
            )
            .unwrap();
        }
        let patch = load_kernel_patch_str(
            "ports:\n  - { name: master, direction: output, signal: audio, channels: 1, maps_from: voice.audio }\nmodules:\n  - { id: voice, type: $LIB/1.0.0/voice/voice.yaml, static: { sample: { kind: sample, path: caller.wav } } }\nconnections: []\n",
        )
        .expect("caller patch loads");
        let context = PreparationContext::new(directory.path(), 48_000)
            .with_macro_roots(MacroRoots::new().with_root(LIB_MACRO, &lib_root));
        let settings = RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 8,
            duration_frames: 8,
        };
        let prepared = prepare_kernel_graph_with_buses_and_context(
            patch.root(),
            patch.registry(),
            &settings,
            &HostBuses::new().with_output("master", 1),
            &context,
        )
        .expect("caller resource prepares through package pass-through");
        let sampler = prepared
            .flattened_graph()
            .nodes()
            .iter()
            .find(|node| node.definition() == "sampler")
            .expect("packaged sampler is flattened");
        assert_eq!(
            sampler.static_args()[SAMPLE_RESOURCE_PARAM],
            StaticValue::Resource(crate::kernel::ResourceRef::new(
                ResourceKind::Sample,
                "caller.wav",
                ResourceOrigin::Document,
            ))
        );
        let rendered = render_kernel_offline_named(
            &prepared,
            vec![crate::core::TimedInputEvent::new(
                0,
                crate::script::ScriptEvent::NoteOn {
                    note: 60,
                    velocity: 100,
                },
            )],
            &PreparedSamplerAssets::empty(),
        )
        .expect("caller sample renders");
        assert!((rendered[0].1[0][0] + 0.5).abs() < 0.0001);
    }

    #[test]
    fn external_reference_errors_are_reported_during_preparation() {
        use crate::patch::RenderSettings;
        use crate::preparation::{HostBuses, prepare_kernel_graph_with_buses_and_context};

        let directory = tempfile::tempdir().unwrap();
        let context = PreparationContext::new(directory.path(), 48_000)
            .with_macro_roots(MacroRoots::new().with_root(LIB_MACRO, directory.path()));
        let settings = RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 8,
            duration_frames: 8,
        };
        for (reference, code) in [
            (
                "$NOPE/voice/voice.yaml",
                crate::diagnostics::error_codes::LIBRARY_UNKNOWN_MACRO,
            ),
            (
                "$LIB/../voice/voice.yaml",
                crate::diagnostics::error_codes::LIBRARY_PATH_ESCAPE,
            ),
        ] {
            let patch = load_kernel_patch_str(&format!(
                "metadata: {{ name: external-error }}\nports:\n  - {{ name: master, direction: output, signal: audio, channels: 1, maps_from: voice.audio }}\nmodules:\n  - {{ id: voice, type: {reference} }}\nconnections: []\n"
            ))
            .unwrap();
            let error = prepare_kernel_graph_with_buses_and_context(
                patch.root(),
                patch.registry(),
                &settings,
                &HostBuses::new().with_output("master", 1),
                &context,
            )
            .expect_err("invalid module reference must fail preparation");
            assert_eq!(
                error.diagnostics().all()[0].error_code(),
                code,
                "{reference}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn package_entry_symlinks_cannot_escape_the_selected_library_root() {
        use std::os::unix::fs::symlink;

        const PACKAGE: &str = "ports:\n  - { name: audio, direction: output, signal: audio, channels: 1, maps_from: source.out }\nmodules:\n  - { id: source, type: control_to_audio, defaults: { in: 0.25 } }\nconnections: []\n";
        let directory = tempfile::tempdir().unwrap();
        let lib_root = directory.path().join("lib");
        let outside = directory.path().join("outside");
        fs::create_dir_all(&lib_root).unwrap();
        let escaped_folder = seed_kernel_package(&outside, "1.0.0", "folder", PACKAGE);
        let escaped_file = seed_kernel_package(&outside, "1.0.0", "file", PACKAGE);
        let version = lib_root.join("1.0.0");
        fs::create_dir_all(version.join("file")).unwrap();
        symlink(escaped_folder.parent().unwrap(), version.join("folder")).unwrap();
        symlink(&escaped_file, version.join("file/file.yaml")).unwrap();
        let context = PreparationContext::new(directory.path(), 48_000)
            .with_macro_roots(MacroRoots::new().with_root(LIB_MACRO, &lib_root));

        for reference in ["$LIB/1.0.0/folder/folder.yaml", "$LIB/1.0.0/file/file.yaml"] {
            let error = match load_referenced_kernel_package(reference, &context) {
                Ok(_) => panic!("{reference} escaped its selected library root"),
                Err(error) => error,
            };
            assert_eq!(
                error.to_diagnostic().error_code(),
                crate::diagnostics::error_codes::LIBRARY_PATH_ESCAPE,
                "{reference}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn package_directory_symlink_inside_library_uses_its_canonical_resource_root() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let lib_root = directory.path().join("lib");
        let target = seed_kernel_package(
            &lib_root.join("contained"),
            "1.0.0",
            "linked",
            "ports:\n  - { name: audio, direction: output, signal: audio, channels: 1, maps_from: source.out }\nmodules:\n  - { id: source, type: control_to_audio, defaults: { in: -0.5 } }\nconnections: []\n",
        );
        let version = lib_root.join("1.0.0");
        fs::create_dir_all(&version).unwrap();
        symlink(target.parent().unwrap(), version.join("linked")).unwrap();
        let context = PreparationContext::new(directory.path(), 48_000)
            .with_macro_roots(MacroRoots::new().with_root(LIB_MACRO, &lib_root));

        let package = load_referenced_kernel_package("$LIB/1.0.0/linked/linked.yaml", &context)
            .expect("a contained package symlink should load");
        assert_eq!(
            package.root(),
            target.parent().unwrap().canonicalize().unwrap()
        );
    }
}
