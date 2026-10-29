//! Format-neutral ownership boundary for the production Trigger instrument.
//!
//! Packaged session resources, runtime handoff and editor protocol live here.
//! CLAP and native WebView types stay in their adapter modules.

#![forbid(unsafe_code)]

pub mod parameters;
mod patch_budget;
pub mod protocol;
pub mod runtime;
pub mod samples;
pub mod state;

use dandrum_engine::graph::PortDirection;
use dandrum_engine::kernel::document::load_kernel_patch_str;
use dandrum_engine::preparation::{HostBuses, PreparationContext};
use dandrum_engine::render_settings::RealtimeRenderSettings;
pub use dandrum_engine::render_settings::SampleRate;
use dandrum_instrument_session::{Attachment, PreparedDocumentReplacement};
use std::fmt;
use std::path::{Path, PathBuf};

pub use dandrum_instrument_session::{InstrumentSession, SessionError};

pub const ADVANCED_DRUM_KIT_DOCUMENT: &str = "instrument/advanced-drum-kit.yaml";
pub const ADVANCED_DRUM_KIT_SAMPLE: &str = "instrument/assets/advanced-drums.wav";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerHostConfiguration {
    pub sample_rate_hz: SampleRate,
    pub max_block_frames: u32,
}

#[derive(Debug)]
pub enum PackagedResourceError {
    MissingRoot,
    InvalidRoot,
    Missing {
        relative: &'static str,
    },
    Invalid {
        relative: &'static str,
    },
    Read {
        relative: &'static str,
        message: String,
    },
}

impl fmt::Display for PackagedResourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRoot => write!(formatter, "Trigger resource root does not exist"),
            Self::InvalidRoot => write!(formatter, "Trigger resource root is not a directory"),
            Self::Missing { relative } => write!(formatter, "missing Trigger resource {relative}"),
            Self::Invalid { relative } => {
                write!(
                    formatter,
                    "Trigger resource is not a regular local file: {relative}"
                )
            }
            Self::Read { relative, message } => {
                write!(
                    formatter,
                    "cannot read Trigger resource {relative}: {message}"
                )
            }
        }
    }
}

impl std::error::Error for PackagedResourceError {}

#[derive(Debug)]
pub enum TriggerHostError {
    Resource(PackagedResourceError),
    Session(SessionError),
}

impl fmt::Display for TriggerHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resource(error) => error.fmt(formatter),
            Self::Session(error) => write!(formatter, "cannot prepare Trigger session: {error:?}"),
        }
    }
}

impl std::error::Error for TriggerHostError {}

impl From<PackagedResourceError> for TriggerHostError {
    fn from(error: PackagedResourceError) -> Self {
        Self::Resource(error)
    }
}

impl From<SessionError> for TriggerHostError {
    fn from(error: SessionError) -> Self {
        Self::Session(error)
    }
}

pub struct PackagedResources {
    root: PathBuf,
    document: String,
}

impl PackagedResources {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, PackagedResourceError> {
        let root = root.as_ref();
        let canonical = root.canonicalize().map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                PackagedResourceError::MissingRoot
            } else {
                PackagedResourceError::InvalidRoot
            }
        })?;
        if root.is_symlink() || !canonical.is_dir() {
            return Err(PackagedResourceError::InvalidRoot);
        }
        let document = Self::read_current_document_from(&canonical)?;
        Ok(Self {
            root: canonical,
            document,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn instrument_root(&self) -> PathBuf {
        self.root.join("instrument")
    }

    fn document(&self) -> &str {
        &self.document
    }

    fn read_current_document(&self) -> Result<String, PackagedResourceError> {
        Self::read_current_document_from(&self.root)
    }

    fn read_current_document_from(root: &Path) -> Result<String, PackagedResourceError> {
        Self::regular_file(root, ADVANCED_DRUM_KIT_SAMPLE)?;
        let document = Self::regular_file(root, ADVANCED_DRUM_KIT_DOCUMENT)?;
        std::fs::read_to_string(document).map_err(|error| PackagedResourceError::Read {
            relative: ADVANCED_DRUM_KIT_DOCUMENT,
            message: error.to_string(),
        })
    }

    fn regular_file(root: &Path, relative: &'static str) -> Result<PathBuf, PackagedResourceError> {
        let path = root.join(relative);
        let canonical = path.canonicalize().map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                PackagedResourceError::Missing { relative }
            } else {
                PackagedResourceError::Invalid { relative }
            }
        })?;
        if path.is_symlink() || !canonical.starts_with(root) || !canonical.is_file() {
            return Err(PackagedResourceError::Invalid { relative });
        }
        Ok(canonical)
    }
}

pub struct TriggerHost {
    resources: PackagedResources,
    session: InstrumentSession,
    sample_catalog: samples::SampleCatalog,
}

impl TriggerHost {
    pub fn open(
        resource_root: impl AsRef<Path>,
        configuration: TriggerHostConfiguration,
    ) -> Result<Self, TriggerHostError> {
        let resources = PackagedResources::open(resource_root)?;
        let settings = RealtimeRenderSettings {
            sample_rate_hz: configuration.sample_rate_hz,
            block_size_frames: configuration.max_block_frames,
            duration_frames: u64::from(configuration.max_block_frames),
        };
        let context = PreparationContext::new_realtime(
            resources.instrument_root(),
            configuration.sample_rate_hz,
        );
        let patch = load_kernel_patch_str(resources.document())
            .map_err(|error| SessionError::InvalidDocument(error.to_string()))?;
        // The Trigger host supplies mono controls and stereo output. Preparation
        // validates those channel constraints against each named root port.
        let buses = patch
            .root()
            .ports()
            .iter()
            .fold(HostBuses::new(), |buses, port| match port.direction() {
                PortDirection::Input => buses.with_input(port.name(), 1),
                PortDirection::Output => buses.with_output(port.name(), 2),
            });
        let session = InstrumentSession::new_realtime_for_planar_host(
            resources.document(),
            context,
            settings,
            buses,
        )?;
        Ok(Self {
            resources,
            session,
            sample_catalog: samples::SampleCatalog::default(),
        })
    }

    pub fn resource_root(&self) -> &Path {
        self.resources.root()
    }

    pub fn session(&self) -> &InstrumentSession {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut InstrumentSession {
        &mut self.session
    }

    pub fn prepare_resource_reload(
        &self,
        attachment: &Attachment,
    ) -> Result<PreparedDocumentReplacement, TriggerHostError> {
        let document = self.resources.read_current_document()?;
        self.session
            .prepare_document_replacement_from_files(attachment, &document)
            .map_err(Into::into)
    }
}
