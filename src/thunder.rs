use crate::contracts::{
    Candidate, CandidateFeatures, CandidateSource, ContentId, RecommendationRequest,
};
use crate::error::{ErrorCategory, MixerError};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

pub const UPSTREAM_REPOSITORY: &str = "https://github.com/xai-org/x-algorithm.git";
pub const UPSTREAM_REVISION: &str = "fad2f71edc780ab14e4cfaebbb8b221385782e43";
pub const UPSTREAM_PATH: &str = "thunder";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Readiness {
    Ready,
    Unavailable,
}

#[derive(Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

pub trait CandidateBackend: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn readiness(&self) -> Readiness;
    fn retrieve(
        &self,
        request: &RecommendationRequest,
        limit: usize,
        cancellation: &CancellationToken,
    ) -> Result<Vec<Candidate>, MixerError>;
    fn hydrate(
        &self,
        ids: &[ContentId],
        cancellation: &CancellationToken,
    ) -> Result<BTreeMap<ContentId, CandidateFeatures>, MixerError>;
}

#[derive(Clone)]
pub struct ThunderAdapter {
    backend: Arc<dyn CandidateBackend>,
    source: CandidateSource,
    timeout: Duration,
    max_candidates: usize,
}

impl ThunderAdapter {
    pub fn initialize(
        backend: Arc<dyn CandidateBackend>,
        source: CandidateSource,
        timeout: Duration,
        max_candidates: usize,
    ) -> Result<Self, MixerError> {
        if max_candidates == 0 {
            return Err(MixerError::new(
                ErrorCategory::Configuration,
                "Thunder adapter max_candidates must be greater than zero",
            ));
        }
        Ok(Self {
            backend,
            source,
            timeout,
            max_candidates,
        })
    }

    pub fn name(&self) -> &str {
        self.backend.name()
    }

    pub fn readiness(&self) -> Readiness {
        self.backend.readiness()
    }

    pub fn retrieve(
        &self,
        request: &RecommendationRequest,
        requested_limit: usize,
    ) -> Result<Vec<Candidate>, MixerError> {
        if self.readiness() != Readiness::Ready {
            return Err(MixerError::new(
                ErrorCategory::Unavailable,
                format!("{} index is not ready", self.name()),
            ));
        }

        let backend = Arc::clone(&self.backend);
        let request = request.clone();
        let limit = requested_limit.min(self.max_candidates);
        let source = self.source;
        self.call_bounded(move |cancellation| {
            let mut candidates = backend.retrieve(&request, limit, &cancellation)?;
            candidates.truncate(limit);
            for candidate in &mut candidates {
                candidate.source = source;
            }
            Ok(candidates)
        })
    }

    pub fn hydrate(
        &self,
        ids: &[ContentId],
    ) -> Result<BTreeMap<ContentId, CandidateFeatures>, MixerError> {
        let backend = Arc::clone(&self.backend);
        let ids = ids[..ids.len().min(self.max_candidates)].to_vec();
        self.call_bounded(move |cancellation| backend.hydrate(&ids, &cancellation))
    }

    fn call_bounded<T, F>(&self, operation: F) -> Result<T, MixerError>
    where
        T: Send + 'static,
        F: FnOnce(CancellationToken) -> Result<T, MixerError> + Send + 'static,
    {
        let (sender, receiver) = mpsc::sync_channel(1);
        let cancellation = CancellationToken::default();
        let worker_cancellation = cancellation.clone();
        thread::spawn(move || {
            let _ = sender.send(operation(worker_cancellation));
        });

        match receiver.recv_timeout(self.timeout) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                cancellation.cancel();
                Err(MixerError::new(
                    ErrorCategory::Timeout,
                    format!("{} query timed out", self.name()),
                ))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(MixerError::new(
                ErrorCategory::Backend,
                format!("{} query worker stopped unexpectedly", self.name()),
            )),
        }
    }
}

pub struct InMemoryThunder {
    name: String,
    candidates: Vec<Candidate>,
    features: BTreeMap<ContentId, CandidateFeatures>,
    available: AtomicBool,
    delay: Duration,
}

impl InMemoryThunder {
    pub fn new(name: impl Into<String>, candidates: Vec<Candidate>) -> Self {
        let features = candidates
            .iter()
            .map(|candidate| (candidate.id, candidate.features.clone()))
            .collect();
        Self {
            name: name.into(),
            candidates,
            features,
            available: AtomicBool::new(true),
            delay: Duration::ZERO,
        }
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    pub fn set_available(&self, available: bool) {
        self.available.store(available, Ordering::Release);
    }
}

impl CandidateBackend for InMemoryThunder {
    fn name(&self) -> &str {
        &self.name
    }

    fn readiness(&self) -> Readiness {
        if self.available.load(Ordering::Acquire) {
            Readiness::Ready
        } else {
            Readiness::Unavailable
        }
    }

    fn retrieve(
        &self,
        _request: &RecommendationRequest,
        limit: usize,
        cancellation: &CancellationToken,
    ) -> Result<Vec<Candidate>, MixerError> {
        if cancellation.is_cancelled() {
            return Err(MixerError::new(
                ErrorCategory::Cancelled,
                "query was cancelled",
            ));
        }
        if !self.delay.is_zero() {
            thread::sleep(self.delay);
        }
        if cancellation.is_cancelled() {
            return Err(MixerError::new(
                ErrorCategory::Cancelled,
                "query was cancelled",
            ));
        }
        Ok(self.candidates.iter().take(limit).cloned().collect())
    }

    fn hydrate(
        &self,
        ids: &[ContentId],
        cancellation: &CancellationToken,
    ) -> Result<BTreeMap<ContentId, CandidateFeatures>, MixerError> {
        if cancellation.is_cancelled() {
            return Err(MixerError::new(
                ErrorCategory::Cancelled,
                "hydration was cancelled",
            ));
        }
        Ok(ids
            .iter()
            .filter_map(|id| {
                self.features
                    .get(id)
                    .cloned()
                    .map(|features| (*id, features))
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_bounds_results_and_sets_provenance() {
        let candidates = (1..=4)
            .map(|id| Candidate {
                id: ContentId(id),
                ..Candidate::default()
            })
            .collect();
        let adapter = ThunderAdapter::initialize(
            Arc::new(InMemoryThunder::new("thunder", candidates)),
            CandidateSource::Thunder,
            Duration::from_secs(1),
            2,
        )
        .unwrap();

        let result = adapter
            .retrieve(&RecommendationRequest::default(), 10)
            .unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].source, CandidateSource::Thunder);
    }

    #[test]
    fn adapter_enforces_timeout() {
        let adapter = ThunderAdapter::initialize(
            Arc::new(InMemoryThunder::new("slow", vec![]).with_delay(Duration::from_millis(50))),
            CandidateSource::Thunder,
            Duration::from_millis(1),
            10,
        )
        .unwrap();

        let error = adapter
            .retrieve(&RecommendationRequest::default(), 10)
            .unwrap_err();
        assert_eq!(error.category, ErrorCategory::Timeout);
    }
}
