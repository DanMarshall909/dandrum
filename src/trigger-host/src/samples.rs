//! Bounded, instance-owned selected content. All methods run away from audio.
pub use dandrum_engine::sample_snapshot::MAX_ENCODED_SAMPLE_BYTES;
pub(crate) const MAX_RETAINED_ENCODED_BYTES: usize = 256 * 1024 * 1024;
pub(crate) const MAX_RESOURCE_COUNT: usize = 256;
use dandrum_engine::sample_snapshot::{SampleSnapshot, SnapshotRetention};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Seek},
    sync::{Arc, Weak},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SampleError {
    Busy,
    WorkerUnavailable,
    NativeUnavailable,
    FileCount,
    InvalidName,
    InvalidFile,
    Read,
    InvalidSample,
    EncodedLimit,
    DecodedLimit,
    ResourceLimit,
    UnknownResource,
}

pub struct NativeSampleFile {
    name: String,
    file: File,
}
impl NativeSampleFile {
    /// Native code supplies an already-opened regular file, never browser paths.
    pub fn new(name: String, file: File) -> Result<Self, SampleError> {
        if name.is_empty() || name.len() > 256 || name.contains('/') || name.contains('\0') {
            return Err(SampleError::InvalidName);
        }
        if !file.metadata().is_ok_and(|metadata| metadata.is_file()) {
            return Err(SampleError::InvalidFile);
        }
        Ok(Self { name, file })
    }
}

#[derive(Debug)]
pub struct SelectedFile {
    pub name: String,
    pub sample: Arc<SampleSnapshot>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CatalogUsage {
    pub encoded_bytes: usize,
    pub decoded_bytes: usize,
    pub resource_count: usize,
}
#[derive(Clone, Copy, Default)]
struct Limits {
    encoded: usize,
    decoded: usize,
    resources: usize,
}
struct Entry {
    retention: SnapshotRetention,
    encoded: usize,
    decoded: usize,
}
pub struct SampleCatalog {
    owner: Arc<()>,
    active: Weak<()>,
    entries: BTreeMap<String, Entry>,
    limits: Limits,
    reserved: Limits,
}
impl Default for SampleCatalog {
    fn default() -> Self {
        Self::new()
    }
}
impl SampleCatalog {
    pub fn new() -> Self {
        Self {
            owner: Arc::new(()),
            active: Weak::new(),
            entries: BTreeMap::new(),
            limits: Limits {
                encoded: MAX_RETAINED_ENCODED_BYTES,
                decoded: 512 * 1024 * 1024,
                resources: MAX_RESOURCE_COUNT,
            },
            reserved: Limits::default(),
        }
    }
    pub fn usage(&mut self) -> CatalogUsage {
        self.entries.retain(|_, entry| entry.retention.is_live());
        let mut usage = self
            .entries
            .values()
            .fold(CatalogUsage::default(), |mut usage, entry| {
                usage.encoded_bytes += entry.encoded;
                usage.decoded_bytes += entry.decoded;
                usage.resource_count += 1;
                usage
            });
        if self.active.strong_count() != 0 {
            usage.encoded_bytes += self.reserved.encoded;
            usage.decoded_bytes += self.reserved.decoded;
            usage.resource_count += self.reserved.resources;
        }
        usage
    }
    pub fn acquire(&self, id: &str) -> Result<Arc<SampleSnapshot>, SampleError> {
        self.entries
            .get(id)
            .and_then(|entry| entry.retention.upgrade())
            .ok_or(SampleError::UnknownResource)
    }
    pub fn begin(
        &mut self,
        files: Vec<NativeSampleFile>,
    ) -> Result<SampleSelectionJob, SampleError> {
        if self.active.strong_count() != 0 {
            return Err(SampleError::Busy);
        }
        if files.is_empty() || files.len() > 16 {
            return Err(SampleError::FileCount);
        }
        let reservation = self.reserve(files.len())?;
        Ok(SampleSelectionJob {
            owner: Arc::clone(&self.owner),
            token: reservation.token,
            files,
            existing: reservation.existing,
            remaining: reservation.remaining,
        })
    }
    fn reserve(&mut self, count: usize) -> Result<Reservation, SampleError> {
        if self.active.strong_count() != 0 {
            return Err(SampleError::Busy);
        }
        let used = self.usage();
        self.reserved = Limits {
            encoded: self
                .limits
                .encoded
                .saturating_sub(used.encoded_bytes)
                .min(count.saturating_mul(MAX_ENCODED_SAMPLE_BYTES)),
            decoded: self
                .limits
                .decoded
                .saturating_sub(used.decoded_bytes)
                .min(count.saturating_mul(2 * MAX_ENCODED_SAMPLE_BYTES)),
            resources: self
                .limits
                .resources
                .saturating_sub(used.resource_count)
                .min(count),
        };
        let token = Arc::new(());
        self.active = Arc::downgrade(&token);
        Ok(Reservation {
            token,
            existing: self
                .entries
                .iter()
                .map(|(id, entry)| (id.clone(), entry.retention.clone()))
                .collect(),
            remaining: self.reserved,
        })
    }

    /// Restore bounded state bytes under the same live-union reservation as
    /// native file selection. The supplied IDs are verified before decoding.
    pub(crate) fn restore(
        &mut self,
        input: &mut impl Read,
        resources: &[(String, usize)],
    ) -> Result<Vec<Arc<SampleSnapshot>>, RestoreError> {
        if resources.is_empty() {
            return Ok(Vec::new());
        }
        if resources.len() > MAX_RESOURCE_COUNT {
            return Err(RestoreError::Sample(SampleError::ResourceLimit));
        }
        let mut total = 0usize;
        for (_, length) in resources {
            total = total
                .checked_add(*length)
                .ok_or(RestoreError::Sample(SampleError::EncodedLimit))?;
            if *length == 0
                || *length > MAX_ENCODED_SAMPLE_BYTES
                || total > MAX_RETAINED_ENCODED_BYTES
            {
                return Err(RestoreError::Sample(SampleError::EncodedLimit));
            }
        }
        let Reservation {
            token,
            existing,
            mut remaining,
        } = self
            .reserve(resources.len())
            .map_err(RestoreError::Sample)?;
        let mut input_error = None;
        let result = (|| {
            let mut prepared = BTreeMap::new();
            let mut files = Vec::with_capacity(resources.len());
            for (id, length) in resources {
                let mut bytes = vec![0; *length];
                input.read_exact(&mut bytes).map_err(|error| {
                    input_error = Some(error.kind());
                    SampleError::Read
                })?;
                let sample =
                    prepare_snapshot(bytes, Some(id), &mut prepared, &existing, &mut remaining)?;
                files.push(SelectedFile {
                    name: id.clone(),
                    sample,
                });
            }
            Ok(files)
        })();
        let result = self
            .accept(SampleSelectionCompletion {
                result,
                owner: Arc::clone(&self.owner),
                token,
            })
            .expect("completion belongs to this live catalog reservation");
        if let Some(kind) = input_error {
            return Err(RestoreError::Input(kind));
        }
        result
            .map(|files| files.into_iter().map(|file| file.sample).collect())
            .map_err(RestoreError::Sample)
    }
    pub fn accept(
        &mut self,
        completion: SampleSelectionCompletion,
    ) -> Result<Result<Vec<SelectedFile>, SampleError>, RejectedSampleCompletion> {
        if !Arc::ptr_eq(&self.owner, &completion.owner)
            || !self
                .active
                .upgrade()
                .is_some_and(|token| Arc::ptr_eq(&token, &completion.token))
        {
            return Err(RejectedSampleCompletion(completion));
        }
        if let Ok(files) = &completion.result {
            for file in files {
                self.entries.insert(
                    file.sample.resource_id().to_owned(),
                    Entry {
                        retention: file.sample.retention(),
                        encoded: file.sample.encoded_bytes().len(),
                        decoded: file.sample.decoded_capacity_bytes(),
                    },
                );
            }
        }
        Ok(completion.result)
    }
}

struct Reservation {
    token: Arc<()>,
    existing: BTreeMap<String, SnapshotRetention>,
    remaining: Limits,
}
pub(crate) enum RestoreError {
    Sample(SampleError),
    Input(std::io::ErrorKind),
}

pub struct SampleSelectionJob {
    files: Vec<NativeSampleFile>,
    existing: BTreeMap<String, SnapshotRetention>,
    remaining: Limits,
    owner: Arc<()>,
    token: Arc<()>,
}
pub struct SampleSelectionCompletion {
    // Field order releases result resources before the reservation on drop.
    result: Result<Vec<SelectedFile>, SampleError>,
    owner: Arc<()>,
    token: Arc<()>,
}
pub struct RejectedSampleCompletion(SampleSelectionCompletion);
impl RejectedSampleCompletion {
    pub fn into_completion(self) -> SampleSelectionCompletion {
        self.0
    }
}
impl std::fmt::Debug for RejectedSampleCompletion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RejectedSampleCompletion")
            .finish_non_exhaustive()
    }
}

impl SampleSelectionJob {
    pub fn prepare(self) -> SampleSelectionCompletion {
        let result = prepare_files(self.files, self.existing, self.remaining);
        SampleSelectionCompletion {
            owner: self.owner,
            token: self.token,
            result,
        }
    }
}
fn prepare_files(
    files: Vec<NativeSampleFile>,
    existing: BTreeMap<String, SnapshotRetention>,
    mut remaining: Limits,
) -> Result<Vec<SelectedFile>, SampleError> {
    let mut prepared = BTreeMap::<String, Arc<SampleSnapshot>>::new();
    let mut selected = Vec::with_capacity(files.len());
    for mut file in files {
        // One bounded read workspace also permits reuse at a full catalog. Hash
        // before decoding, so a duplicate never allocates a second decoded copy.
        let mut bytes = Vec::with_capacity(MAX_ENCODED_SAMPLE_BYTES + 1);
        file.file
            .rewind()
            .and_then(|()| {
                Read::by_ref(&mut file.file)
                    .take((MAX_ENCODED_SAMPLE_BYTES + 1) as u64)
                    .read_to_end(&mut bytes)
            })
            .map_err(|_| SampleError::Read)?;
        let sample = prepare_snapshot(bytes, None, &mut prepared, &existing, &mut remaining)?;
        selected.push(SelectedFile {
            name: file.name,
            sample,
        });
    }
    Ok(selected)
}

fn prepare_snapshot(
    bytes: Vec<u8>,
    expected: Option<&str>,
    prepared: &mut BTreeMap<String, Arc<SampleSnapshot>>,
    existing: &BTreeMap<String, SnapshotRetention>,
    remaining: &mut Limits,
) -> Result<Arc<SampleSnapshot>, SampleError> {
    let id =
        SampleSnapshot::resource_id_for_encoded(&bytes).map_err(|_| SampleError::EncodedLimit)?;
    if expected.is_some_and(|expected| expected != id) {
        return Err(SampleError::InvalidSample);
    }
    let reusable = prepared
        .get(&id)
        .cloned()
        .or_else(|| existing.get(&id).and_then(SnapshotRetention::upgrade));
    let sample = if let Some(sample) = reusable {
        if sample.encoded_bytes() != bytes {
            return Err(SampleError::InvalidSample);
        }
        sample
    } else {
        if remaining.resources == 0 {
            return Err(SampleError::ResourceLimit);
        }
        if bytes.len() > remaining.encoded {
            return Err(SampleError::EncodedLimit);
        }
        let sample =
            Arc::new(SampleSnapshot::decode(bytes).map_err(|_| SampleError::InvalidSample)?);
        if sample.decoded_capacity_bytes() > remaining.decoded {
            return Err(SampleError::DecodedLimit);
        }
        remaining.resources -= 1;
        remaining.encoded -= sample.encoded_bytes().len();
        remaining.decoded -= sample.decoded_capacity_bytes();
        sample
    };
    prepared.insert(id, Arc::clone(&sample));
    Ok(sample)
}

#[cfg(test)]
#[path = "../tests/internal/selected_samples.rs"]
mod tests;
