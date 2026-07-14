//! Seeded standard-library extraction for reusable module packages.
//!
//! The render path must not create or mutate library files. Hosts call this
//! during preparation/startup to seed a versioned standard library under the
//! configured `$LIB` root. Extraction is CRC-gated and writes into a sibling
//! staging directory before publishing the completed version directory, so a
//! package version is never observed half-written.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::diagnostics::{Diagnostic, Severity, error_codes};

/// Environment variable that overrides the default seeded `$LIB` storage root.
pub const STANDARD_LIBRARY_ROOT_ENV_VAR: &str = "DANDRUM_MODULE_LIBRARY_ROOT";

/// Manifest file written inside each seeded version directory.
pub const STANDARD_LIBRARY_CRC_FILENAME: &str = ".dandrum-library.crc";

/// Current bundled standard-library version.
pub const BUNDLED_STANDARD_LIBRARY_VERSION: &str = "1.0.0";

const BUNDLED_DRUM_VOICE_PATH: &str = "drum_voice/drum_voice.yaml";
const BUNDLED_DRUM_VOICE_YAML: &[u8] =
    include_bytes!("../module-library/1.0.0/drum_voice/drum_voice.yaml");
const BUNDLED_DRUM_MACHINE_PATH: &str = "drum_machine/drum_machine.yaml";
const BUNDLED_DRUM_MACHINE_YAML: &[u8] =
    include_bytes!("../module-library/1.0.0/drum_machine/drum_machine.yaml");

/// One file bundled into a seeded module-library version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeededLibraryFile {
    pub path: String,
    pub contents: Vec<u8>,
}

/// A complete seeded module-library version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeededLibrary {
    pub version: String,
    pub files: Vec<SeededLibraryFile>,
}

/// Outcome of a seed attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SeedResult {
    /// The target version already had the same recorded CRC, so no extraction ran.
    SkippedUnchanged { version: String, crc: u32 },
    /// A version directory was extracted or replaced.
    Extracted { version: String, crc: u32 },
}

/// Failure to seed the module library.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModuleLibrarySeedError {
    MissingHomeDirectory,
    InvalidVersion { version: String },
    PathEscape { path: String },
    Io { path: PathBuf, message: String },
}

impl ModuleLibrarySeedError {
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            Self::MissingHomeDirectory => Diagnostic::new(
                error_codes::LIBRARY_SEED_FAILED,
                Severity::Error,
                "cannot determine a default module library root; set DANDRUM_MODULE_LIBRARY_ROOT",
            ),
            Self::InvalidVersion { version } => Diagnostic::new(
                error_codes::LIBRARY_SEED_FAILED,
                Severity::Error,
                format!("invalid seeded module library version {version}"),
            ),
            Self::PathEscape { path } => Diagnostic::new(
                error_codes::LIBRARY_PATH_ESCAPE,
                Severity::Error,
                format!("seeded module library file path {path} escapes its version root"),
            ),
            Self::Io { path, message } => Diagnostic::new(
                error_codes::LIBRARY_SEED_FAILED,
                Severity::Error,
                format!(
                    "failed to seed module library at {}: {message}",
                    path.display()
                ),
            ),
        }
    }
}

/// Returns the canonical storage root used for seeded `$LIB` content.
///
/// Hosts may override it with `DANDRUM_MODULE_LIBRARY_ROOT`; otherwise it
/// defaults to `<home>/.dandrum/lib` without creating directories.
pub fn default_standard_library_root() -> Result<PathBuf, ModuleLibrarySeedError> {
    if let Some(root) = std::env::var_os(STANDARD_LIBRARY_ROOT_ENV_VAR) {
        return Ok(PathBuf::from(root));
    }

    home_directory()
        .map(|home| home.join(".dandrum").join("lib"))
        .ok_or(ModuleLibrarySeedError::MissingHomeDirectory)
}

/// Returns the immutable standard module-library bundle shipped with the engine.
pub fn bundled_standard_library() -> SeededLibrary {
    SeededLibrary {
        version: BUNDLED_STANDARD_LIBRARY_VERSION.to_string(),
        files: vec![
            SeededLibraryFile {
                path: BUNDLED_DRUM_VOICE_PATH.to_string(),
                contents: BUNDLED_DRUM_VOICE_YAML.to_vec(),
            },
            SeededLibraryFile {
                path: BUNDLED_DRUM_MACHINE_PATH.to_string(),
                contents: BUNDLED_DRUM_MACHINE_YAML.to_vec(),
            },
        ],
    }
}

/// Seeds the bundled standard module-library version under `root`.
pub fn seed_bundled_standard_library(
    root: impl AsRef<Path>,
) -> Result<SeedResult, ModuleLibrarySeedError> {
    seed_standard_library(root, &bundled_standard_library())
}

/// Seeds `library.version` under `root`, skipping extraction when the recorded
/// CRC already matches and replacing only that version directory when it differs.
pub fn seed_standard_library(
    root: impl AsRef<Path>,
    library: &SeededLibrary,
) -> Result<SeedResult, ModuleLibrarySeedError> {
    validate_version(&library.version)?;
    for file in &library.files {
        reject_file_path_escape(&file.path)?;
    }

    let root = root.as_ref();
    let crc = seeded_library_crc(library);
    let version_root = root.join(&library.version);
    let manifest_path = version_root.join(STANDARD_LIBRARY_CRC_FILENAME);

    if recorded_crc(&manifest_path)? == Some(crc) {
        return Ok(SeedResult::SkippedUnchanged {
            version: library.version.clone(),
            crc,
        });
    }

    fs::create_dir_all(root).map_err(|error| io_error(root, error))?;

    let staging_root = root.join(format!(
        ".{}.extracting.{}",
        library.version,
        std::process::id()
    ));
    if staging_root.exists() {
        fs::remove_dir_all(&staging_root).map_err(|error| io_error(&staging_root, error))?;
    }
    fs::create_dir_all(&staging_root).map_err(|error| io_error(&staging_root, error))?;

    for file in &library.files {
        let target = staging_root.join(&file.path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| io_error(parent, error))?;
        }
        fs::write(&target, &file.contents).map_err(|error| io_error(&target, error))?;
    }

    let staging_manifest = staging_root.join(STANDARD_LIBRARY_CRC_FILENAME);
    fs::write(&staging_manifest, format_crc(crc))
        .map_err(|error| io_error(&staging_manifest, error))?;

    publish_version_directory(&version_root, &staging_root)?;

    Ok(SeedResult::Extracted {
        version: library.version.clone(),
        crc,
    })
}

fn publish_version_directory(
    version_root: &Path,
    staging_root: &Path,
) -> Result<(), ModuleLibrarySeedError> {
    if !version_root.exists() {
        return fs::rename(staging_root, version_root)
            .map_err(|error| io_error(version_root, error));
    }

    let backup_root = version_root.with_extension(format!("replacing.{}", std::process::id()));
    if backup_root.exists() {
        fs::remove_dir_all(&backup_root).map_err(|error| io_error(&backup_root, error))?;
    }

    fs::rename(version_root, &backup_root).map_err(|error| io_error(version_root, error))?;
    match fs::rename(staging_root, version_root) {
        Ok(()) => {
            fs::remove_dir_all(&backup_root).map_err(|error| io_error(&backup_root, error))?;
            Ok(())
        }
        Err(error) => {
            let _ = fs::rename(&backup_root, version_root);
            Err(io_error(version_root, error))
        }
    }
}

fn recorded_crc(path: &Path) -> Result<Option<u32>, ModuleLibrarySeedError> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(u32::from_str_radix(text.trim(), 16).ok()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_error(path, error)),
    }
}

fn validate_version(version: &str) -> Result<(), ModuleLibrarySeedError> {
    let is_valid = !version.is_empty()
        && version
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()));

    if is_valid {
        Ok(())
    } else {
        Err(ModuleLibrarySeedError::InvalidVersion {
            version: version.to_string(),
        })
    }
}

fn reject_file_path_escape(path: &str) -> Result<(), ModuleLibrarySeedError> {
    let is_escape = Path::new(path)
        .components()
        .any(|component| !matches!(component, Component::Normal(_)));

    if is_escape {
        Err(ModuleLibrarySeedError::PathEscape {
            path: path.to_string(),
        })
    } else {
        Ok(())
    }
}

fn seeded_library_crc(library: &SeededLibrary) -> u32 {
    let mut files = library.files.clone();
    files.sort_by(|left, right| left.path.cmp(&right.path));

    let mut crc = crc32_update(0xffff_ffff, library.version.as_bytes());
    for file in files {
        crc = crc32_update(crc, b"\0");
        crc = crc32_update(crc, file.path.as_bytes());
        crc = crc32_update(crc, b"\0");
        crc = crc32_update(crc, &file.contents);
    }
    !crc
}

fn crc32_update(mut crc: u32, bytes: &[u8]) -> u32 {
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = if crc & 1 == 1 { 0xedb8_8320 } else { 0 };
            crc = (crc >> 1) ^ mask;
        }
    }
    crc
}

fn format_crc(crc: u32) -> String {
    format!("{crc:08x}\n")
}

fn home_directory() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn io_error(path: &Path, error: io::Error) -> ModuleLibrarySeedError {
    ModuleLibrarySeedError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{PortDirection, SignalType};
    use crate::graph_processor::RealtimeGraphProcessor;
    use crate::module_package::load_referenced_kernel_package;
    use crate::module_reference::{self, LIB_MACRO, MacroRoots};
    use crate::patch::{RenderSettings, VoiceAllocation};
    use crate::preparation::{
        HostBuses, PreparationContext, prepare_kernel_graph_with_buses_and_context,
    };
    use crate::sample::PreparedSamplerAssets;
    use std::collections::BTreeSet;

    const DRUM_VOICE_REFERENCE: &str = "$LIB/1.0.0/drum_voice/drum_voice.yaml";
    const DRUM_MACHINE_REFERENCE: &str = "$LIB/1.0.0/drum_machine/drum_machine.yaml";
    const RESOURCE_PACKAGE_PATH: &str = "sample_voice/sample_voice.yaml";
    const RESOURCE_PACKAGE_REFERENCE: &str = "$LIB/1.0.0/sample_voice/sample_voice.yaml";
    const RESOURCE_PACKAGE_YAML: &[u8] = br#"
ports:
  - { name: left, direction: output, signal: audio, channels: 1, maps_from: player.audio }
  - { name: right, direction: output, signal: audio, channels: 1, maps_from: player.audio }
modules:
  - { id: midi, type: midi_input }
  - id: player
    type: sampler
    static:
      sample: { kind: sample, path: samples/hit.wav }
connections:
  - { from: midi.events, to: player.trigger }
"#;
    const TEST_SAMPLE_RATE_HZ: u32 = 48_000;
    const TEST_BLOCK_SIZE_FRAMES: usize = 8;

    fn file(path: &str, contents: &[u8]) -> SeededLibraryFile {
        SeededLibraryFile {
            path: path.to_string(),
            contents: contents.to_vec(),
        }
    }

    fn library(version: &str, contents: &[u8]) -> SeededLibrary {
        SeededLibrary {
            version: version.to_string(),
            files: vec![file("drum_voice/drum_voice.yaml", contents)],
        }
    }

    #[test]
    fn bundled_standard_library_contains_the_drum_voice_and_drum_machine_packages() {
        let bundled = bundled_standard_library();
        let paths = bundled
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(bundled.version, BUNDLED_STANDARD_LIBRARY_VERSION);
        assert!(
            paths.contains(BUNDLED_DRUM_VOICE_PATH),
            "the bundled standard library should carry the drum_voice package entry YAML"
        );
        assert!(
            paths.contains(BUNDLED_DRUM_MACHINE_PATH),
            "the bundled standard library should carry the drum_machine package entry YAML"
        );
    }

    #[test]
    fn bundled_standard_library_seeds_under_the_requested_root() {
        let root = temp_root("bundled");

        seed_bundled_standard_library(&root).expect("bundled library should seed");

        assert!(
            root.join(BUNDLED_STANDARD_LIBRARY_VERSION)
                .join(BUNDLED_DRUM_VOICE_PATH)
                .exists(),
            "the bundled drum_voice package should be extracted under the version-first layout"
        );
        assert!(
            root.join(BUNDLED_STANDARD_LIBRARY_VERSION)
                .join(BUNDLED_DRUM_MACHINE_PATH)
                .exists(),
            "the bundled drum_machine package should be extracted under the version-first layout"
        );
    }

    #[test]
    fn bundled_packages_load_through_the_kernel_parser_with_characterized_public_ports() {
        let root = temp_root("drum-machine");
        seed_bundled_standard_library(&root).expect("bundled library should seed");
        let context = preparation_context(&root);
        let voice = load_referenced_kernel_package(DRUM_VOICE_REFERENCE, &context)
            .expect("bundled drum_voice should load through the kernel parser");
        let machine = load_referenced_kernel_package(DRUM_MACHINE_REFERENCE, &context)
            .expect("bundled drum_machine should load through the kernel parser");

        let voice_ports = voice
            .definition()
            .ports()
            .iter()
            .map(|port| (port.name(), port.direction(), port.signal_type()))
            .collect::<Vec<_>>();
        assert_eq!(
            voice_ports,
            vec![
                ("trigger", PortDirection::Input, SignalType::Event),
                ("audio", PortDirection::Output, SignalType::Audio),
            ]
        );

        let outputs = machine
            .definition()
            .ports()
            .iter()
            .filter(|port| port.direction() == PortDirection::Output)
            .map(|port| port.name())
            .collect::<BTreeSet<_>>();

        for expected in [
            "main_left",
            "main_right",
            "kick_left",
            "kick_right",
            "snare_left",
            "snare_right",
            "hat_left",
            "hat_right",
        ] {
            assert!(
                outputs.contains(expected),
                "the bundled drum_machine should expose the {expected} output"
            );
        }
    }

    #[test]
    fn bundled_packages_retain_characterized_public_routing() {
        let root = temp_root("drum-machine-routes");
        seed_bundled_standard_library(&root).expect("bundled library should seed");
        let context = preparation_context(&root);
        let voice = load_referenced_kernel_package(DRUM_VOICE_REFERENCE, &context)
            .expect("bundled drum_voice should load through the kernel parser");
        let machine = load_referenced_kernel_package(DRUM_MACHINE_REFERENCE, &context)
            .expect("bundled drum_machine should load through the kernel parser");

        assert_eq!(
            voice.definition().ports()[0].internal_targets()[0]
                .node()
                .as_str(),
            "env"
        );
        assert_eq!(
            voice.definition().ports()[1].internal_sources()[0]
                .node()
                .as_str(),
            "vca"
        );

        let routed_sources = machine
            .definition()
            .ports()
            .iter()
            .filter_map(|port| {
                port.internal_sources()
                    .first()
                    .map(|source| (port.name(), source.node().as_str()))
            })
            .collect::<BTreeSet<_>>();

        assert!(routed_sources.contains(&("kick_left", "kick_vca")));
        assert!(routed_sources.contains(&("snare_left", "snare_vca")));
        assert!(routed_sources.contains(&("hat_left", "hat_vca")));
    }

    #[test]
    fn pinned_resource_package_prepares_and_renders_its_package_relative_sample() {
        let root = temp_root("resource-package");
        for (version, sample) in [("1.0.0", 0.25), ("2.0.0", 0.75)] {
            seed_standard_library(
                &root,
                &SeededLibrary {
                    version: version.to_string(),
                    files: vec![file(RESOURCE_PACKAGE_PATH, RESOURCE_PACKAGE_YAML)],
                },
            )
            .expect("resource package version should seed");
            let sample_dir = root.join(version).join("sample_voice").join("samples");
            fs::create_dir_all(&sample_dir).expect("sample directory should be created");
            crate::wav::write_wav_stereo_i16(
                fs::File::create(sample_dir.join("hit.wav")).unwrap(),
                TEST_SAMPLE_RATE_HZ,
                &[sample],
                &[sample],
            )
            .expect("tiny package sample should be written");
        }
        let context = preparation_context(&root);
        let package = load_referenced_kernel_package(RESOURCE_PACKAGE_REFERENCE, &context)
            .expect("pinned resource package should load");
        let settings = RenderSettings {
            sample_rate_hz: TEST_SAMPLE_RATE_HZ,
            block_size_frames: TEST_BLOCK_SIZE_FRAMES as u32,
            duration_frames: TEST_BLOCK_SIZE_FRAMES as u64,
        };
        let prepared = prepare_kernel_graph_with_buses_and_context(
            package.definition(),
            package.registry(),
            &settings,
            &HostBuses::new()
                .with_output("left", 1)
                .with_output("right", 1),
            &context,
        )
        .expect("pinned resource package should prepare through PreparationContext");
        let mut runtime = RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
            prepared.graph().clone(),
            prepared.compiled_patch().clone(),
            TEST_SAMPLE_RATE_HZ as f32,
            &PreparedSamplerAssets::empty(),
            &VoiceAllocation::default(),
            TEST_BLOCK_SIZE_FRAMES,
        );
        runtime.note_on(60, 100);
        let mut left = [0.0; TEST_BLOCK_SIZE_FRAMES];
        let mut right = [0.0; TEST_BLOCK_SIZE_FRAMES];

        assert_eq!(
            runtime.render(&mut left, &mut right),
            TEST_BLOCK_SIZE_FRAMES
        );
        assert!((left[0] - 0.25).abs() < 0.0001);
        assert_eq!(left, right);
    }

    fn preparation_context(root: &Path) -> PreparationContext {
        PreparationContext::new(root, TEST_SAMPLE_RATE_HZ)
            .with_macro_roots(MacroRoots::new().with_root(LIB_MACRO, root))
    }

    #[test]
    fn unchanged_crc_skips_extraction_and_preserves_existing_files() {
        let root = temp_root("unchanged");
        let seeded = library("1.0.0", b"first\n");

        let first = seed_standard_library(&root, &seeded).expect("initial seed should extract");
        let file_path = root
            .join("1.0.0")
            .join("drum_voice")
            .join("drum_voice.yaml");
        fs::write(&file_path, b"user-visible existing bytes\n")
            .expect("test should be able to mutate the seeded file");

        let second = seed_standard_library(&root, &seeded).expect("same seed should skip");

        assert!(
            matches!(first, SeedResult::Extracted { .. }),
            "the first seed should extract the version"
        );
        assert!(
            matches!(second, SeedResult::SkippedUnchanged { .. }),
            "an unchanged CRC should skip extraction"
        );
        assert_eq!(
            fs::read(&file_path).expect("file should remain readable"),
            b"user-visible existing bytes\n",
            "skipping extraction should leave the existing version directory untouched"
        );
    }

    #[test]
    fn changed_crc_replaces_only_that_version_directory() {
        let root = temp_root("changed");
        seed_standard_library(&root, &library("1.0.0", b"old\n"))
            .expect("initial seed should extract");
        fs::write(root.join("1.0.0").join("stale.txt"), b"stale")
            .expect("stale file should be written");

        let result = seed_standard_library(&root, &library("1.0.0", b"new\n"))
            .expect("changed seed should replace the version");

        assert!(
            matches!(result, SeedResult::Extracted { .. }),
            "a changed CRC should extract a replacement version"
        );
        assert_eq!(
            fs::read(
                root.join("1.0.0")
                    .join("drum_voice")
                    .join("drum_voice.yaml")
            )
            .expect("updated module file should exist"),
            b"new\n"
        );
        assert!(
            !root.join("1.0.0").join("stale.txt").exists(),
            "replacing one version should not retain stale files inside that version"
        );
    }

    #[test]
    fn reseeding_newer_versions_is_additive_and_latest_follows_newest_version() {
        let root = temp_root("additive");
        seed_standard_library(&root, &library("1.0.0", b"modules: []\n"))
            .expect("older version should seed");
        seed_standard_library(&root, &library("1.1.0", b"newer\n"))
            .expect("newer version should seed");

        assert!(
            root.join("1.0.0")
                .join("drum_voice")
                .join("drum_voice.yaml")
                .exists(),
            "old pinned versions should remain resolvable"
        );
        assert!(
            root.join("1.1.0")
                .join("drum_voice")
                .join("drum_voice.yaml")
                .exists(),
            "new versions should be added beside old versions"
        );

        let roots = MacroRoots::new().with_root(LIB_MACRO, &root);
        let latest = module_reference::resolve("$LIB/latest/drum_voice/drum_voice.yaml", &roots)
            .expect("latest should resolve after seeding versions");

        assert_eq!(
            latest,
            root.join("1.1.0")
                .join("drum_voice")
                .join("drum_voice.yaml"),
            "latest should follow the newest seeded version"
        );
    }

    #[test]
    fn seeded_file_path_escape_is_rejected() {
        let root = temp_root("escape");
        let seeded = SeededLibrary {
            version: "1.0.0".to_string(),
            files: vec![file("../outside.yaml", b"bad")],
        };

        let error = seed_standard_library(&root, &seeded)
            .expect_err("escaping paths must be rejected before writing");

        assert!(
            matches!(error, ModuleLibrarySeedError::PathEscape { .. }),
            "an escaping seeded path should report PathEscape, got {error:?}"
        );
        assert_eq!(
            error.to_diagnostic().error_code(),
            error_codes::LIBRARY_PATH_ESCAPE
        );
    }

    fn temp_root(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "dandrum-module-library-{tag}-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("stale test root should be removable");
        }
        root
    }
}
