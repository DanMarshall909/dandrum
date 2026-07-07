//! Resolution of macro-qualified module references.
//!
//! A defined-module instance may reference an external module package by setting
//! its `type` to a macro-qualified, version-pinned path such as
//! `$LIB/1.3.9/drum_voice/drum_voice.yaml`. The leading `$` macro is the
//! discriminator: any `type` beginning with it is an external reference, while
//! every other `type` is a built-in type name or an inline defined-module type,
//! exactly as before.
//!
//! The reserved `latest` version alias resolves to the highest numeric version
//! directory under the macro root, so `$LIB/latest/drum_voice/drum_voice.yaml`
//! can move forward while pinned references remain stable.
//!
//! This module owns the pure resolution layer only: detecting a reference,
//! mapping its macro root to a configured base directory, and rejecting unknown
//! macros and path escapes. Loading and expanding the resolved package lives with
//! the module-package loader.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::diagnostics::{Diagnostic, Severity, error_codes};

/// Leading character that marks a module `type` as an external, macro-qualified
/// reference rather than a built-in type name or an inline defined-module type.
pub const MACRO_PREFIX: char = '$';

/// Separator between segments of a macro-qualified reference.
pub const REFERENCE_SEPARATOR: char = '/';

/// Built-in name of the immutable, seeded standard-library macro root.
pub const LIB_MACRO: &str = "$LIB";

/// Built-in name of the mutable, user-owned macro root.
pub const USER_LIB_MACRO: &str = "$USER_LIB";

/// Reserved version path segment that resolves to the highest available version
/// directory under the configured macro root.
pub const LATEST_VERSION_ALIAS: &str = "latest";

/// Returns `true` when a module `type` is an external macro-qualified reference.
pub fn is_external_reference(module_type: &str) -> bool {
    module_type.starts_with(MACRO_PREFIX)
}

/// Configured macro roots, mapping a macro name (e.g. `$LIB`) to its absolute
/// base directory on disk. Roots are engine/host-provided configuration; an
/// unknown macro is a hard error rather than a silent fallback.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MacroRoots {
    roots: BTreeMap<String, PathBuf>,
}

impl MacroRoots {
    /// Creates an empty set of macro roots.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `root` as the base directory for `macro_name`, returning the
    /// updated set so registrations can be chained.
    pub fn with_root(mut self, macro_name: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        self.roots.insert(macro_name.into(), root.into());
        self
    }

    /// Returns the base directory configured for `macro_name`, if any.
    pub fn root(&self, macro_name: &str) -> Option<&Path> {
        self.roots.get(macro_name).map(PathBuf::as_path)
    }
}

/// Failure to resolve a macro-qualified module reference to an absolute path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModuleReferenceError {
    /// The `type` is not an external reference (does not begin with `$`).
    NotAReference { module_type: String },
    /// The reference names a macro root that is not configured.
    UnknownMacro { macro_name: String },
    /// The reference has no path after its macro root.
    Malformed { reference: String },
    /// The reference tries to escape its macro root (e.g. via `..` or an
    /// absolute segment).
    PathEscape { reference: String },
    /// The `latest` alias could not be resolved to a concrete version directory.
    LatestUnavailable {
        reference: String,
        root: PathBuf,
        message: String,
    },
}

impl ModuleReferenceError {
    /// Renders the error as a validation diagnostic.
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            Self::NotAReference { module_type } => Diagnostic::new(
                error_codes::LIBRARY_MALFORMED_REFERENCE,
                Severity::Error,
                format!("module type {module_type} is not an external macro reference"),
            ),
            Self::UnknownMacro { macro_name } => Diagnostic::new(
                error_codes::LIBRARY_UNKNOWN_MACRO,
                Severity::Error,
                format!("unknown module library macro {macro_name}"),
            ),
            Self::Malformed { reference } => Diagnostic::new(
                error_codes::LIBRARY_MALFORMED_REFERENCE,
                Severity::Error,
                format!("malformed module reference {reference}: missing package path"),
            ),
            Self::PathEscape { reference } => Diagnostic::new(
                error_codes::LIBRARY_PATH_ESCAPE,
                Severity::Error,
                format!("module reference {reference} escapes its library root"),
            ),
            Self::LatestUnavailable {
                reference,
                root,
                message,
            } => Diagnostic::new(
                error_codes::LIBRARY_LATEST_UNAVAILABLE,
                Severity::Error,
                format!(
                    "module reference {reference} cannot resolve latest under {}: {message}",
                    root.display()
                ),
            ),
        }
    }
}

/// Resolves a macro-qualified `reference` to an absolute path under its
/// configured macro root.
///
/// The reference is `$MACRO/<relative>`; the macro root is looked up in `roots`
/// and the relative portion is joined onto it after rejecting any component that
/// would escape the root (`..`, absolute, or drive-prefixed segments). If the
/// first relative segment is `latest`, it is replaced with the highest numeric
/// version directory currently present under the macro root.
pub fn resolve(reference: &str, roots: &MacroRoots) -> Result<PathBuf, ModuleReferenceError> {
    if !is_external_reference(reference) {
        return Err(ModuleReferenceError::NotAReference {
            module_type: reference.to_string(),
        });
    }

    let mut segments = reference.splitn(2, REFERENCE_SEPARATOR);
    let macro_name = segments.next().unwrap_or_default();
    let relative = segments.next().unwrap_or_default();

    let root = roots
        .root(macro_name)
        .ok_or_else(|| ModuleReferenceError::UnknownMacro {
            macro_name: macro_name.to_string(),
        })?;

    if relative.is_empty() {
        return Err(ModuleReferenceError::Malformed {
            reference: reference.to_string(),
        });
    }

    let relative_path = Path::new(relative);
    for component in relative_path.components() {
        match component {
            Component::Normal(_) => {}
            _ => {
                return Err(ModuleReferenceError::PathEscape {
                    reference: reference.to_string(),
                });
            }
        }
    }

    let resolved_relative = resolve_latest_alias(relative_path, root, reference)?;
    Ok(root.join(resolved_relative))
}

fn resolve_latest_alias(
    relative_path: &Path,
    root: &Path,
    reference: &str,
) -> Result<PathBuf, ModuleReferenceError> {
    let mut components = relative_path.components();
    let Some(Component::Normal(first)) = components.next() else {
        return Ok(relative_path.to_path_buf());
    };

    if first != LATEST_VERSION_ALIAS {
        return Ok(relative_path.to_path_buf());
    }

    let latest = latest_version_directory_name(root, reference)?;
    let mut resolved = PathBuf::from(latest);
    for component in components {
        if let Component::Normal(segment) = component {
            resolved.push(segment);
        }
    }
    Ok(resolved)
}

fn latest_version_directory_name(
    root: &Path,
    reference: &str,
) -> Result<String, ModuleReferenceError> {
    let entries = fs::read_dir(root).map_err(|error| ModuleReferenceError::LatestUnavailable {
        reference: reference.to_string(),
        root: root.to_path_buf(),
        message: error.to_string(),
    })?;

    let mut latest: Option<(LibraryVersion, String)> = None;
    for entry in entries {
        let entry = entry.map_err(|error| ModuleReferenceError::LatestUnavailable {
            reference: reference.to_string(),
            root: root.to_path_buf(),
            message: error.to_string(),
        })?;
        if !entry
            .file_type()
            .map(|file_type| file_type.is_dir())
            .unwrap_or(false)
        {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(ToOwned::to_owned) else {
            continue;
        };
        let Some(version) = LibraryVersion::parse(&name) else {
            continue;
        };

        if latest
            .as_ref()
            .is_none_or(|(current, _)| version > *current)
        {
            latest = Some((version, name));
        }
    }

    latest
        .map(|(_, name)| name)
        .ok_or_else(|| ModuleReferenceError::LatestUnavailable {
            reference: reference.to_string(),
            root: root.to_path_buf(),
            message: "no numeric version directories exist".to_string(),
        })
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct LibraryVersion(Vec<u64>);

impl LibraryVersion {
    fn parse(name: &str) -> Option<Self> {
        let parts = name
            .split('.')
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()
            .ok()?;

        if parts.is_empty() || parts.iter().all(|part| *part == 0) {
            return None;
        }

        Some(Self(parts))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PINNED_REFERENCE: &str = "$LIB/1.3.9/drum_voice/drum_voice.yaml";

    fn roots() -> MacroRoots {
        MacroRoots::new()
            .with_root(LIB_MACRO, "/opt/dandrum/lib")
            .with_root(USER_LIB_MACRO, "/home/user/.dandrum/modules")
    }

    #[test]
    fn dollar_prefixed_type_is_an_external_reference() {
        assert!(
            is_external_reference(PINNED_REFERENCE),
            "a $-prefixed type should be detected as an external reference"
        );
    }

    #[test]
    fn built_in_and_inline_types_are_not_external_references() {
        assert!(
            !is_external_reference("oscillator"),
            "a built-in type name should not be an external reference"
        );
        assert!(
            !is_external_reference("my_drum_voice"),
            "an inline defined-module type should not be an external reference"
        );
    }

    #[test]
    fn pinned_reference_resolves_under_its_macro_root() {
        let resolved = resolve(PINNED_REFERENCE, &roots())
            .expect("a well-formed $LIB reference should resolve");
        assert_eq!(
            resolved,
            PathBuf::from("/opt/dandrum/lib/1.3.9/drum_voice/drum_voice.yaml"),
            "the relative path should be joined onto the configured $LIB root"
        );
    }

    #[test]
    fn user_lib_reference_resolves_under_its_own_root() {
        let resolved = resolve("$USER_LIB/my_kit/my_kit.yaml", &roots())
            .expect("a well-formed $USER_LIB reference should resolve");
        assert_eq!(
            resolved,
            PathBuf::from("/home/user/.dandrum/modules/my_kit/my_kit.yaml"),
            "the reference should resolve under the $USER_LIB root, not $LIB"
        );
    }

    #[test]
    fn latest_alias_resolves_to_the_highest_version_directory() {
        let root = temp_library_root("latest");
        fs::create_dir_all(root.join("1.3.9")).expect("older version directory should be created");
        fs::create_dir_all(root.join("1.10.0")).expect("newer version directory should be created");
        fs::create_dir_all(root.join("latest")).expect("literal latest directory should be ignored");
        fs::write(root.join("not-a-version"), "ignored").expect("file should be created");
        let roots = MacroRoots::new().with_root(LIB_MACRO, &root);

        let resolved = resolve("$LIB/latest/drum_voice/drum_voice.yaml", &roots)
            .expect("latest should resolve when version directories exist");

        assert_eq!(
            resolved,
            root.join("1.10.0").join("drum_voice").join("drum_voice.yaml"),
            "latest should follow the highest numeric version directory"
        );
    }

    #[test]
    fn pinned_versions_remain_resolvable_when_latest_exists() {
        let root = temp_library_root("pinned");
        fs::create_dir_all(root.join("1.3.9")).expect("older version directory should be created");
        fs::create_dir_all(root.join("1.10.0")).expect("newer version directory should be created");
        let roots = MacroRoots::new().with_root(LIB_MACRO, &root);

        let resolved = resolve("$LIB/1.3.9/drum_voice/drum_voice.yaml", &roots)
            .expect("a pinned version should not require latest lookup");

        assert_eq!(
            resolved,
            root.join("1.3.9").join("drum_voice").join("drum_voice.yaml"),
            "pinned references should stay on their explicit version"
        );
    }

    #[test]
    fn latest_alias_without_version_directories_is_a_hard_error() {
        let root = temp_library_root("empty-latest");
        fs::create_dir_all(&root).expect("library root should be created");
        let roots = MacroRoots::new().with_root(LIB_MACRO, &root);

        let error = resolve("$LIB/latest/drum_voice/drum_voice.yaml", &roots)
            .expect_err("latest should fail when no concrete version exists");

        assert_eq!(
            error.to_diagnostic().error_code(),
            error_codes::LIBRARY_LATEST_UNAVAILABLE,
            "an unavailable latest alias should map to a stable library diagnostic"
        );
    }

    #[test]
    fn unknown_macro_is_a_hard_error() {
        let error = resolve("$NOPE/1.0.0/thing/thing.yaml", &roots())
            .expect_err("an unconfigured macro should fail rather than fall back");
        assert_eq!(
            error,
            ModuleReferenceError::UnknownMacro {
                macro_name: "$NOPE".to_string(),
            },
            "the error should name the unknown macro without resolving elsewhere"
        );
        assert_eq!(
            error.to_diagnostic().error_code(),
            error_codes::LIBRARY_UNKNOWN_MACRO,
            "an unknown macro should map to the library.unknown_macro code"
        );
    }

    #[test]
    fn parent_dir_segment_is_rejected_as_a_path_escape() {
        let reference = "$LIB/1.3.9/../../etc/passwd";
        let error = resolve(reference, &roots())
            .expect_err("a reference with .. segments should be rejected");
        assert_eq!(
            error,
            ModuleReferenceError::PathEscape {
                reference: reference.to_string(),
            },
            "a .. segment must not be allowed to escape the library root"
        );
        assert_eq!(
            error.to_diagnostic().error_code(),
            error_codes::LIBRARY_PATH_ESCAPE,
            "a path escape should map to the library.path_escape code"
        );
    }

    fn temp_library_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "dandrum-module-reference-{tag}-{}",
            std::process::id()
        ))
    }
}
