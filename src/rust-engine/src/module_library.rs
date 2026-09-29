//! Seeded standard-library extraction for reusable module packages.
//!
//! The render path must not create or mutate library files. Hosts call this
//! during preparation/startup to seed a versioned standard library under the
//! configured `$LIB` root. Extraction is CRC-gated and writes into a sibling
//! staging directory before publishing the completed version directory, so a
//! package version is never observed half-written.

use std::fs;
use std::io::{self, Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};

use crate::diagnostics::{Diagnostic, Severity, error_codes};
use crate::module_reference::{LIB_MACRO, MacroRoots, USER_LIB_MACRO};

/// Environment variable that overrides the default seeded `$LIB` storage root.
pub const STANDARD_LIBRARY_ROOT_ENV_VAR: &str = "DANDRUM_MODULE_LIBRARY_ROOT";
pub const USER_LIBRARY_ROOT_ENV_VAR: &str = "DANDRUM_USER_LIBRARY_ROOT";

/// Manifest file written inside each seeded version directory.
pub const STANDARD_LIBRARY_CRC_FILENAME: &str = ".dandrum-library.crc";

/// Current bundled standard-library version.
pub const BUNDLED_STANDARD_LIBRARY_VERSION: &str = "1.0.0";

const BUNDLED_SEED_ZIP: &[u8] = include_bytes!("../module-library/seed-1.0.0.zip");

#[cfg(test)]
const BUNDLED_DRUM_VOICE_PATH: &str = "drum_voice/drum_voice.yaml";
#[cfg(test)]
const BUNDLED_DRUM_VOICE_YAML: &[u8] =
    include_bytes!("../module-library/1.0.0/drum_voice/drum_voice.yaml");
#[cfg(test)]
const BUNDLED_DRUM_MACHINE_PATH: &str = "drum_machine/drum_machine.yaml";
#[cfg(test)]
const BUNDLED_SAMPLE_VOICE_YAML: &[u8] =
    include_bytes!("../module-library/1.0.0/sample_voice/sample_voice.yaml");

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
    Archive { message: String },
    Io { path: PathBuf, message: String },
}

impl ModuleLibrarySeedError {
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            Self::MissingHomeDirectory => Diagnostic::new(
                error_codes::LIBRARY_SEED_FAILED,
                Severity::Error,
                "cannot determine default module library roots; set DANDRUM_MODULE_LIBRARY_ROOT and DANDRUM_USER_LIBRARY_ROOT",
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
            Self::Archive { message } => Diagnostic::new(
                error_codes::LIBRARY_SEED_FAILED,
                Severity::Error,
                format!("invalid module library seed zip: {message}"),
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

/// Returns the mutable user module root without creating it.
pub fn default_user_library_root() -> Result<PathBuf, ModuleLibrarySeedError> {
    if let Some(root) = std::env::var_os(USER_LIBRARY_ROOT_ENV_VAR) {
        return Ok(PathBuf::from(root));
    }
    home_directory()
        .map(|home| home.join(".dandrum").join("modules"))
        .ok_or(ModuleLibrarySeedError::MissingHomeDirectory)
}

/// Configures the two host macros and checks the bundled standard-library
/// seed. Hosts call this only while preparing a patch with external modules.
pub fn default_host_macro_roots() -> Result<MacroRoots, ModuleLibrarySeedError> {
    let standard = default_standard_library_root()?;
    let user = default_user_library_root()?;
    seed_bundled_standard_library(&standard)?;
    Ok(MacroRoots::new()
        .with_root(LIB_MACRO, standard)
        .with_root(USER_LIB_MACRO, user))
}

/// Returns the immutable standard module-library bundle shipped with the engine.
pub fn bundled_standard_library() -> SeededLibrary {
    SeededLibrary {
        version: BUNDLED_STANDARD_LIBRARY_VERSION.to_string(),
        files: read_seed_archive(BUNDLED_SEED_ZIP).expect("shipped seed zip must be valid"),
    }
}

/// Seeds the bundled standard module-library version under `root`.
pub fn seed_bundled_standard_library(
    root: impl AsRef<Path>,
) -> Result<SeedResult, ModuleLibrarySeedError> {
    seed_standard_library_zip(root, BUNDLED_STANDARD_LIBRARY_VERSION, BUNDLED_SEED_ZIP)
}

/// Seeds `library.version` under `root`, skipping extraction when the recorded
/// CRC already matches and replacing only that version directory when it differs.
pub fn seed_standard_library(
    root: impl AsRef<Path>,
    library: &SeededLibrary,
) -> Result<SeedResult, ModuleLibrarySeedError> {
    let archive = write_seed_archive(library)?;
    seed_standard_library_zip(root, &library.version, &archive)
}

fn seed_standard_library_zip(
    root: impl AsRef<Path>,
    version: &str,
    archive: &[u8],
) -> Result<SeedResult, ModuleLibrarySeedError> {
    validate_version(version)?;
    let files = read_seed_archive(archive)?;

    let root = root.as_ref();
    let crc = !crc32_update(0xffff_ffff, archive);
    let version_root = root.join(version);
    let manifest_path = version_root.join(STANDARD_LIBRARY_CRC_FILENAME);

    fs::create_dir_all(root).map_err(|error| io_error(root, error))?;
    // The lock also covers the CRC check so concurrent hosts cannot each
    // replace the same version directory after observing an old manifest.
    let lock_path = root.join(format!(".{version}.seed.lock"));
    let lock_file = fs::File::options()
        .create(true)
        .write(true)
        .open(&lock_path)
        .map_err(|error| io_error(&lock_path, error))?;
    lock_file
        .lock()
        .map_err(|error| io_error(&lock_path, error))?;

    // A matching manifest alone cannot prove the directory is complete after
    // an interrupted or older concurrent extraction.
    if recorded_crc(&manifest_path)? == Some(crc)
        && files
            .iter()
            .all(|file| version_root.join(&file.path).is_file())
    {
        return Ok(SeedResult::SkippedUnchanged {
            version: version.to_string(),
            crc,
        });
    }

    let staging_root = root.join(format!(".{version}.extracting.{}", std::process::id()));
    if staging_root.exists() {
        remove_seeded_directory(&staging_root)?;
    }
    fs::create_dir_all(&staging_root).map_err(|error| io_error(&staging_root, error))?;

    for file in &files {
        let target = staging_root.join(&file.path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| io_error(parent, error))?;
        }
        fs::write(&target, &file.contents).map_err(|error| io_error(&target, error))?;
        let mut permissions = fs::metadata(&target)
            .map_err(|error| io_error(&target, error))?
            .permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&target, permissions).map_err(|error| io_error(&target, error))?;
    }

    let staging_manifest = staging_root.join(STANDARD_LIBRARY_CRC_FILENAME);
    fs::write(&staging_manifest, format_crc(crc))
        .map_err(|error| io_error(&staging_manifest, error))?;

    publish_version_directory(&version_root, &staging_root)?;

    Ok(SeedResult::Extracted {
        version: version.to_string(),
        crc,
    })
}

fn write_seed_archive(library: &SeededLibrary) -> Result<Vec<u8>, ModuleLibrarySeedError> {
    validate_version(&library.version)?;
    let mut files = library.files.clone();
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .large_file(true);
    for file in files {
        reject_file_path_escape(&file.path)?;
        writer
            .start_file(&file.path, options)
            .map_err(archive_error)?;
        // A started ZIP64 file written to Cursor<Vec<u8>> has no fallible
        // backing store; allocation failure aborts rather than returning I/O.
        writer
            .write_all(&file.contents)
            .expect("in-memory ZIP writer cannot report an I/O error");
    }
    Ok(writer.finish().map_err(archive_error)?.into_inner())
}

fn read_seed_archive(archive: &[u8]) -> Result<Vec<SeededLibraryFile>, ModuleLibrarySeedError> {
    let mut reader = zip::ZipArchive::new(Cursor::new(archive)).map_err(archive_error)?;
    let mut files = Vec::new();
    for index in 0..reader.len() {
        let mut entry = reader.by_index(index).map_err(archive_error)?;
        if entry.is_dir() {
            continue;
        }
        let path = entry.name().to_string();
        reject_file_path_escape(&path)?;
        let mut contents = Vec::new();
        entry
            .read_to_end(&mut contents)
            .map_err(|error| ModuleLibrarySeedError::Archive {
                message: error.to_string(),
            })?;
        files.push(SeededLibraryFile { path, contents });
    }
    Ok(files)
}

fn archive_error(error: zip::result::ZipError) -> ModuleLibrarySeedError {
    ModuleLibrarySeedError::Archive {
        message: error.to_string(),
    }
}

fn publish_version_directory(
    version_root: &Path,
    staging_root: &Path,
) -> Result<(), ModuleLibrarySeedError> {
    if !version_root.exists() {
        return fs::rename(staging_root, version_root)
            .map_err(|error| io_error(version_root, error));
    }

    let mut backup_name = version_root
        .file_name()
        .expect("validated version root has a final component")
        .to_os_string();
    backup_name.push(format!(".replacing.{}", std::process::id()));
    let backup_root = version_root.with_file_name(backup_name);
    if backup_root.exists() {
        remove_seeded_directory(&backup_root)?;
    }

    fs::rename(version_root, &backup_root).map_err(|error| io_error(version_root, error))?;
    match fs::rename(staging_root, version_root) {
        Ok(()) => {
            remove_seeded_directory(&backup_root)?;
            Ok(())
        }
        Err(error) => {
            let _ = fs::rename(&backup_root, version_root);
            Err(io_error(version_root, error))
        }
    }
}

fn remove_seeded_directory(path: &Path) -> Result<(), ModuleLibrarySeedError> {
    // Windows cannot delete read-only files. Only walk real directories and
    // regular files; remove_dir_all itself unlinks symlinks without following them.
    if !fs::symlink_metadata(path)
        .map_err(|error| io_error(path, error))?
        .file_type()
        .is_symlink()
    {
        clear_seeded_file_readonly(path)?;
    }
    fs::remove_dir_all(path).map_err(|error| io_error(path, error))
}

fn clear_seeded_file_readonly(directory: &Path) -> Result<(), ModuleLibrarySeedError> {
    for entry in fs::read_dir(directory).map_err(|error| io_error(directory, error))? {
        let entry = entry.map_err(|error| io_error(directory, error))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| io_error(&path, error))?;
        if file_type.is_dir() {
            clear_seeded_file_readonly(&path)?;
        } else if file_type.is_file() {
            let mut permissions = entry
                .metadata()
                .map_err(|error| io_error(&path, error))?
                .permissions();
            if permissions.readonly() {
                permissions.set_readonly(false);
                fs::set_permissions(&path, permissions).map_err(|error| io_error(&path, error))?;
            }
        }
    }
    Ok(())
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

#[cfg(test)]
fn seeded_library_crc(library: &SeededLibrary) -> u32 {
    !crc32_update(
        0xffff_ffff,
        &write_seed_archive(library).expect("test seed should be valid"),
    )
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
    const NEWER_RESOURCE_PACKAGE_REFERENCE: &str = "$LIB/2.0.0/sample_voice/sample_voice.yaml";
    const RESOURCE_PACKAGE_YAML: &[u8] = BUNDLED_SAMPLE_VOICE_YAML;
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
    fn bundled_standard_library_contains_kernel_packages_and_sample_resource() {
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
        assert!(paths.contains("sample_voice/sample_voice.yaml"));
        assert!(paths.contains("sample_voice/samples/hit.wav"));
    }

    #[test]
    fn bundled_seed_extracts_one_zip_and_records_the_zip_crc() {
        use std::io::Read;

        let seed_zip = include_bytes!("../module-library/seed-1.0.0.zip");
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(seed_zip.as_slice()))
            .expect("the shipped seed is a zip archive");
        let bundled = bundled_standard_library();
        assert_eq!(archive.len(), bundled.files.len());
        for file in &bundled.files {
            let mut archived = archive
                .by_name(&file.path)
                .expect("each package file is in the single archive");
            let mut contents = Vec::new();
            archived.read_to_end(&mut contents).unwrap();
            assert_eq!(contents, file.contents, "{}", file.path);
            assert_eq!(
                contents,
                fs::read(
                    Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("module-library/1.0.0")
                        .join(&file.path)
                )
                .unwrap(),
                "the shipped archive must match the authored package file {}",
                file.path
            );
        }

        let root = tempfile::tempdir().unwrap();
        let result = seed_bundled_standard_library(root.path()).expect("zip seed extracts");
        let expected_crc = !crc32_update(0xffff_ffff, seed_zip);
        assert_eq!(
            result,
            SeedResult::Extracted {
                version: BUNDLED_STANDARD_LIBRARY_VERSION.to_string(),
                crc: expected_crc,
            }
        );
        assert_eq!(
            fs::read_to_string(
                root.path()
                    .join("1.0.0")
                    .join(STANDARD_LIBRARY_CRC_FILENAME)
            )
            .unwrap(),
            format_crc(expected_crc)
        );
    }

    #[test]
    fn seed_zip_rejects_entries_escaping_the_version_root() {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file("../outside.yaml", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"escaped").unwrap();
        let zip = writer.finish().unwrap().into_inner();
        let root = tempfile::tempdir().unwrap();

        let error = seed_standard_library_zip(root.path(), "1.0.0", &zip)
            .expect_err("archive traversal must fail before any version is published");
        assert_eq!(
            error,
            ModuleLibrarySeedError::PathEscape {
                path: "../outside.yaml".to_string()
            }
        );
        assert!(!root.path().join("1.0.0").exists());
    }

    #[test]
    fn malformed_seed_zip_has_a_stable_diagnostic() {
        let root = tempfile::tempdir().unwrap();
        let error = seed_standard_library_zip(root.path(), "1.0.0", b"not a zip")
            .expect_err("an invalid archive must fail before extraction");
        assert!(matches!(error, ModuleLibrarySeedError::Archive { .. }));
        assert_eq!(
            error.to_diagnostic().error_code(),
            error_codes::LIBRARY_SEED_FAILED
        );
        assert!(!root.path().join("1.0.0").exists());
    }

    #[test]
    fn seed_zip_ignores_directory_entries_and_extracts_files() {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        writer.add_directory("drum_voice/", options).unwrap();
        writer
            .start_file("drum_voice/drum_voice.yaml", options)
            .unwrap();
        writer.write_all(b"voice\n").unwrap();
        let archive = writer.finish().unwrap().into_inner();
        let root = tempfile::tempdir().unwrap();

        assert!(matches!(
            seed_standard_library_zip(root.path(), "1.0.0", &archive).unwrap(),
            SeedResult::Extracted { .. }
        ));
        assert_eq!(
            fs::read(root.path().join("1.0.0/drum_voice/drum_voice.yaml")).unwrap(),
            b"voice\n"
        );
    }

    #[test]
    fn corrupt_zip_entry_does_not_replace_an_existing_version() {
        let root = tempfile::tempdir().unwrap();
        seed_standard_library(root.path(), &library("1.0.0", b"old\n")).unwrap();
        let mut archive = write_seed_archive(&library("1.0.0", b"new\n")).unwrap();
        let position = archive
            .windows(4)
            .position(|bytes| bytes == b"new\n")
            .expect("stored ZIP contains the module bytes");
        archive[position] = b'X';

        let error = seed_standard_library_zip(root.path(), "1.0.0", &archive)
            .expect_err("a corrupt member must fail before replacing the version");
        assert!(matches!(error, ModuleLibrarySeedError::Archive { .. }));
        assert_eq!(
            error.to_diagnostic().error_code(),
            error_codes::LIBRARY_SEED_FAILED
        );
        assert_eq!(
            fs::read(root.path().join("1.0.0/drum_voice/drum_voice.yaml")).unwrap(),
            b"old\n"
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
        seed_bundled_standard_library(&root).expect("bundled resource package should seed");
        seed_standard_library(
            &root,
            &SeededLibrary {
                version: "2.0.0".to_string(),
                files: vec![file(RESOURCE_PACKAGE_PATH, RESOURCE_PACKAGE_YAML)],
            },
        )
        .expect("newer resource package version should seed");
        let sample_dir = root.join("2.0.0").join("sample_voice").join("samples");
        fs::create_dir_all(&sample_dir).expect("sample directory should be created");
        crate::wav::write_wav_stereo_i16(
            fs::File::create(sample_dir.join("hit.wav")).unwrap(),
            TEST_SAMPLE_RATE_HZ,
            &[0.75],
            &[0.75],
        )
        .expect("newer package sample should be written");
        let context = preparation_context(&root);
        let settings = RenderSettings {
            sample_rate_hz: TEST_SAMPLE_RATE_HZ,
            block_size_frames: TEST_BLOCK_SIZE_FRAMES as u32,
            duration_frames: TEST_BLOCK_SIZE_FRAMES as u64,
        };
        for (reference, expected_first_sample) in [
            (RESOURCE_PACKAGE_REFERENCE, 0.25),
            (NEWER_RESOURCE_PACKAGE_REFERENCE, 0.75),
        ] {
            let package = load_referenced_kernel_package(reference, &context)
                .expect("pinned resource package should load");
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
            let mut outputs = vec![vec![vec![0.0; TEST_BLOCK_SIZE_FRAMES]]; 2];

            assert_eq!(
                runtime.render_root_outputs(&mut outputs),
                TEST_BLOCK_SIZE_FRAMES
            );
            assert!(
                (outputs[0][0][0] - expected_first_sample).abs() < 0.0001,
                "{reference} should render its own package-relative sample",
            );
            assert_eq!(outputs[0], outputs[1]);
        }
    }

    #[test]
    fn resource_package_renders_identically_after_moving_between_library_roots() {
        let directory = tempfile::tempdir().expect("isolated library roots");
        let original_root = directory.path().join("original");
        let moved_root = directory.path().join("moved");
        seed_bundled_standard_library(&original_root).expect("resource package should seed");
        let settings = RenderSettings {
            sample_rate_hz: TEST_SAMPLE_RATE_HZ,
            block_size_frames: TEST_BLOCK_SIZE_FRAMES as u32,
            duration_frames: TEST_BLOCK_SIZE_FRAMES as u64,
        };
        let render = |root: &Path| {
            let context = preparation_context(root);
            let package = load_referenced_kernel_package(RESOURCE_PACKAGE_REFERENCE, &context)
                .expect("resource package should load from either library root");
            assert_eq!(
                package.root(),
                root.join("1.0.0/sample_voice").canonicalize().unwrap()
            );
            let prepared = prepare_kernel_graph_with_buses_and_context(
                package.definition(),
                package.registry(),
                &settings,
                &HostBuses::new()
                    .with_output("left", 1)
                    .with_output("right", 1),
                &context,
            )
            .expect("package-relative sample should prepare after relocation");
            let mut runtime = RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
                prepared.graph().clone(),
                prepared.compiled_patch().clone(),
                TEST_SAMPLE_RATE_HZ as f32,
                &PreparedSamplerAssets::empty(),
                &VoiceAllocation::default(),
                TEST_BLOCK_SIZE_FRAMES,
            );
            runtime.note_on(60, 100);
            let mut outputs = vec![vec![vec![0.0; TEST_BLOCK_SIZE_FRAMES]]; 2];
            assert_eq!(
                runtime.render_root_outputs(&mut outputs),
                TEST_BLOCK_SIZE_FRAMES
            );
            outputs
        };

        let original = render(&original_root);
        assert!((original[0][0][0] - 0.25).abs() < 0.0001);
        fs::create_dir_all(moved_root.join("1.0.0")).unwrap();
        fs::rename(
            original_root.join("1.0.0/sample_voice"),
            moved_root.join("1.0.0/sample_voice"),
        )
        .expect("move the same package with its co-located sample");
        assert_eq!(render(&moved_root), original);
    }

    fn preparation_context(root: &Path) -> PreparationContext {
        PreparationContext::new(root, TEST_SAMPLE_RATE_HZ)
            .with_macro_roots(MacroRoots::new().with_root(LIB_MACRO, root))
    }

    #[test]
    fn unchanged_crc_skips_extraction_of_readonly_seeded_files() {
        let root = temp_root("unchanged");
        let seeded = library("1.0.0", b"first\n");

        let first = seed_standard_library(&root, &seeded).expect("initial seed should extract");
        let file_path = root
            .join("1.0.0")
            .join("drum_voice")
            .join("drum_voice.yaml");
        assert!(
            fs::metadata(&file_path).unwrap().permissions().readonly(),
            "standard library package files should be read-only"
        );

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
            b"first\n",
            "skipping extraction should leave the seeded contents untouched"
        );
    }

    #[test]
    fn matching_crc_repairs_a_missing_bundled_package_file() {
        let root = tempfile::tempdir().expect("isolated library root");
        let bundled = bundled_standard_library();
        seed_standard_library(root.path(), &bundled).expect("bundle seeds");
        let missing = root
            .path()
            .join(BUNDLED_STANDARD_LIBRARY_VERSION)
            .join(BUNDLED_DRUM_VOICE_PATH);
        fs::remove_file(&missing).expect("bundled file can be removed");

        let result = seed_standard_library(root.path(), &bundled)
            .expect("seeding should repair a missing file despite matching CRC");

        assert!(matches!(result, SeedResult::Extracted { .. }));
        assert_eq!(fs::read(missing).unwrap(), BUNDLED_DRUM_VOICE_YAML);
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
    fn replacing_another_patch_version_preserves_a_pending_backup() {
        let root = tempfile::tempdir().unwrap();
        seed_standard_library(root.path(), &library("1.0.0", b"old first\n")).unwrap();
        seed_standard_library(root.path(), &library("1.0.1", b"old second\n")).unwrap();
        // An interrupted first-version replacement may leave this backup.
        // The second version must never delete it while publishing its own update.
        let pending_first_backup = root
            .path()
            .join(format!("1.0.replacing.{}", std::process::id()));
        fs::create_dir(&pending_first_backup).unwrap();
        fs::write(pending_first_backup.join("preserved.yaml"), b"first backup").unwrap();

        seed_standard_library(root.path(), &library("1.0.1", b"new second\n")).unwrap();
        assert_eq!(
            fs::read(pending_first_backup.join("preserved.yaml")).unwrap(),
            b"first backup"
        );
        assert_eq!(
            fs::read(root.path().join("1.0.0/drum_voice/drum_voice.yaml")).unwrap(),
            b"old first\n"
        );
        assert_eq!(
            fs::read(root.path().join("1.0.1/drum_voice/drum_voice.yaml")).unwrap(),
            b"new second\n"
        );
    }

    #[test]
    fn stale_readonly_backup_is_removed_before_replacing_its_version() {
        let root = tempfile::tempdir().unwrap();
        seed_standard_library(root.path(), &library("1.0.0", b"old\n")).unwrap();
        let backup = root
            .path()
            .join(format!("1.0.0.replacing.{}", std::process::id()));
        fs::create_dir_all(&backup).unwrap();
        let stale = backup.join("stale.yaml");
        fs::write(&stale, b"stale").unwrap();
        let mut permissions = fs::metadata(&stale).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&stale, permissions).unwrap();

        seed_standard_library(root.path(), &library("1.0.0", b"new\n")).unwrap();
        assert!(!backup.exists());
        assert_eq!(
            fs::read(root.path().join("1.0.0/drum_voice/drum_voice.yaml")).unwrap(),
            b"new\n"
        );
    }

    #[test]
    fn stale_readonly_staging_files_are_removed_before_reseeding() {
        let root = tempfile::tempdir().unwrap();
        let staging = root
            .path()
            .join(format!(".1.0.0.extracting.{}", std::process::id()));
        fs::create_dir_all(&staging).unwrap();
        let stale = staging.join("stale.yaml");
        fs::write(&stale, b"old staging contents").unwrap();
        let mut permissions = fs::metadata(&stale).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&stale, permissions).unwrap();

        assert!(matches!(
            seed_standard_library(root.path(), &library("1.0.0", b"new\n")).unwrap(),
            SeedResult::Extracted { .. }
        ));
        assert!(!staging.exists());
        assert_eq!(
            fs::read(root.path().join("1.0.0/drum_voice/drum_voice.yaml")).unwrap(),
            b"new\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn staging_cleanup_does_not_follow_resource_symlinks() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = root.path().join("outside.yaml");
        fs::write(&outside, b"outside must survive").unwrap();
        let mut permissions = fs::metadata(&outside).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&outside, permissions).unwrap();
        let staging = root
            .path()
            .join(format!(".1.0.0.extracting.{}", std::process::id()));
        fs::create_dir_all(&staging).unwrap();
        symlink(&outside, staging.join("linked.yaml")).unwrap();

        seed_standard_library(root.path(), &library("1.0.0", b"new\n")).unwrap();
        assert_eq!(fs::read(&outside).unwrap(), b"outside must survive");
        assert!(fs::metadata(&outside).unwrap().permissions().readonly());
    }

    #[cfg(unix)]
    #[test]
    fn seeded_cleanup_unlinks_a_directory_symlink_without_touching_its_target() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = root.path().join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("preserved.yaml"), b"outside").unwrap();
        let link = root.path().join("stale-backup");
        symlink(&outside, &link).unwrap();

        remove_seeded_directory(&link).unwrap();
        assert!(!link.exists());
        assert_eq!(
            fs::read(outside.join("preserved.yaml")).unwrap(),
            b"outside"
        );
    }

    #[test]
    fn concurrent_seeders_publish_one_complete_updated_version() {
        let root = tempfile::tempdir().expect("isolated library root");
        let old = library("1.0.0", b"old\n");
        let updated = library("1.0.0", b"new\n");
        seed_standard_library(root.path(), &old).expect("old version seeds");
        let start = std::sync::Arc::new(std::sync::Barrier::new(8));
        let workers = (0..8)
            .map(|_| {
                let root = root.path().to_path_buf();
                let updated = updated.clone();
                let start = std::sync::Arc::clone(&start);
                std::thread::spawn(move || {
                    start.wait();
                    seed_standard_library(&root, &updated)
                })
            })
            .collect::<Vec<_>>();
        for worker in workers {
            worker
                .join()
                .expect("seeder thread should not panic")
                .expect("concurrent seeder should succeed");
        }
        assert_eq!(
            fs::read(root.path().join("1.0.0/drum_voice/drum_voice.yaml")).unwrap(),
            b"new\n"
        );
        assert_eq!(
            recorded_crc(
                &root
                    .path()
                    .join("1.0.0")
                    .join(STANDARD_LIBRARY_CRC_FILENAME)
            )
            .unwrap(),
            Some(seeded_library_crc(&updated))
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
