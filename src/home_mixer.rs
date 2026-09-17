use crate::config::{HomeMixerConfig, SourcePolicy};
use crate::contracts::{
    Candidate, CandidateFeatures, CandidateSource, ContentId, MixerFixture, MixerResponse,
    Pagination, PartialFailure, PipelineDiagnostics, PipelineWarning, RankedResult,
    RecommendationRequest, ScoreComponent, ScoreExplanation,
};
use crate::error::{ErrorCategory, MixerError};
use crate::telemetry::StageTimer;
use crate::thunder::{InMemoryThunder, ThunderAdapter};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct HomeMixer {
    config: HomeMixerConfig,
    thunder: ThunderAdapter,
    fallback_candidates: Vec<Candidate>,
}

struct ScoredCandidate {
    candidate: Candidate,
    score: f64,
    components: Vec<ScoreComponent>,
}

impl HomeMixer {
    pub fn new(
        config: HomeMixerConfig,
        thunder: ThunderAdapter,
        fallback_candidates: Vec<Candidate>,
    ) -> Result<Self, MixerError> {
        config.validate()?;
        Ok(Self {
            config,
            thunder,
            fallback_candidates,
        })
    }

    pub fn mix(&self, request: RecommendationRequest) -> Result<MixerResponse, MixerError> {
        if !self.config.enabled {
            return Err(MixerError::new(
                ErrorCategory::Unavailable,
                "Home Mixer is disabled",
            ));
        }

        let mut diagnostics = PipelineDiagnostics {
            request_id: request.request_id.clone(),
            ..PipelineDiagnostics::default()
        };
        let mut warnings = Vec::new();

        let timer = StageTimer::start("validate_normalize", 1);
        let request = self.normalize_request(request, &mut warnings)?;
        timer.finish(&mut diagnostics, 1, 0);

        let timer = StageTimer::start("hydrate_context", 1);
        let excluded: BTreeSet<ContentId> = request
            .excluded_ids
            .iter()
            .chain(&request.recently_served_ids)
            .copied()
            .take(self.config.max_candidates)
            .collect();
        timer.finish(&mut diagnostics, 1, 0);

        let timer = StageTimer::start("retrieve_candidates", 0);
        let mut candidates = self.retrieve(&request, &mut diagnostics, &mut warnings)?;
        timer.finish(&mut diagnostics, candidates.len(), 0);

        let timer = StageTimer::start("attach_provenance", candidates.len());
        for candidate in &mut candidates {
            if diagnostics.fallback_used {
                candidate.source = CandidateSource::Baseline;
            }
        }
        timer.finish(&mut diagnostics, candidates.len(), 0);

        let timer = StageTimer::start("deduplicate", candidates.len());
        let before_dedup = candidates.len();
        let mut seen = BTreeSet::new();
        candidates.retain(|candidate| seen.insert(candidate.id));
        diagnostics.duplicate_count = before_dedup - candidates.len();
        timer.finish(
            &mut diagnostics,
            candidates.len(),
            diagnostics.duplicate_count,
        );

        let timer = StageTimer::start("eligibility_safety", candidates.len());
        let before_filter = candidates.len();
        candidates.retain(|candidate| {
            candidate.id.0 != 0
                && candidate.eligible
                && candidate.safe
                && !excluded.contains(&candidate.id)
        });
        timer.finish(
            &mut diagnostics,
            candidates.len(),
            before_filter - candidates.len(),
        );

        let timer = StageTimer::start("hydrate_features", candidates.len());
        if !diagnostics.fallback_used && !candidates.is_empty() {
            let ids: Vec<_> = candidates.iter().map(|candidate| candidate.id).collect();
            match self.thunder.hydrate(&ids) {
                Ok(features) => merge_features(&mut candidates, features),
                Err(error) if self.config.thunder_policy == SourcePolicy::Required => {
                    return Err(error);
                }
                Err(error) => record_partial_failure(
                    &self.thunder,
                    error,
                    &mut diagnostics,
                    &mut warnings,
                ),
            }
        }
        timer.finish(&mut diagnostics, candidates.len(), 0);

        let timer = StageTimer::start("score", candidates.len());
        let mut scored = candidates
            .into_iter()
            .map(|candidate| self.score(candidate, &request, &mut warnings))
            .collect::<Vec<_>>();
        timer.finish(&mut diagnostics, scored.len(), 0);

        let timer = StageTimer::start("normalize_combine", scored.len());
        scored.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.candidate.id.cmp(&right.candidate.id))
        });
        timer.finish(&mut diagnostics, scored.len(), 0);

        let timer = StageTimer::start("diversity_repetition", scored.len());
        let before_diversity = scored.len();
        let scored = self.apply_diversity(scored, request.limit);
        timer.finish(
            &mut diagnostics,
            scored.len(),
            before_diversity - scored.len(),
        );

        let timer = StageTimer::start("select", scored.len());
        let result_count = scored.len().min(request.limit);
        let selected = scored.into_iter().take(result_count).collect::<Vec<_>>();
        timer.finish(&mut diagnostics, selected.len(), 0);

        let timer = StageTimer::start("explain_diagnostics", selected.len());
        let mut results = Vec::with_capacity(selected.len());
        for (index, scored) in selected.into_iter().enumerate() {
            *diagnostics
                .source_contributions
                .entry(source_name(scored.candidate.source).to_string())
                .or_insert(0) += 1;
            results.push(RankedResult {
                rank: index + 1,
                score: scored.score,
                explanation: ScoreExplanation {
                    summary: format!(
                        "weighted blend from {}",
                        source_name(scored.candidate.source)
                    ),
                    components: scored.components,
                },
                candidate: scored.candidate,
            });
        }
        diagnostics.empty_feed = results.is_empty();
        diagnostics.min_score = results.last().map(|result| result.score);
        diagnostics.max_score = results.first().map(|result| result.score);
        let pagination = Pagination {
            next_cursor: results
                .last()
                .filter(|_| results.len() == request.limit)
                .map(|result| format!("{}:{}", result.candidate.id.0, result.rank)),
            returned: results.len(),
        };
        timer.finish(&mut diagnostics, results.len(), 0);

        Ok(MixerResponse {
            results,
            pagination,
            warnings,
            diagnostics,
        })
    }

    fn normalize_request(
        &self,
        mut request: RecommendationRequest,
        warnings: &mut Vec<PipelineWarning>,
    ) -> Result<RecommendationRequest, MixerError> {
        if request.request_id.trim().is_empty() {
            return Err(MixerError::new(
                ErrorCategory::InvalidRequest,
                "request_id must not be empty",
            ));
        }
        if request.limit == 0 {
            return Err(MixerError::new(
                ErrorCategory::InvalidRequest,
                "request limit must be greater than zero",
            ));
        }
        if request.session_id.trim().is_empty() {
            request.session_id = "anonymous".to_string();
        }
        if request.now_secs == 0 {
            request.now_secs = unix_time_secs();
        }
        if request.limit > self.config.max_request_results {
            warnings.push(PipelineWarning {
                code: "limit_clamped".to_string(),
                source: None,
                message: format!(
                    "requested {} results; clamped to {}",
                    request.limit, self.config.max_request_results
                ),
            });
            request.limit = self.config.max_request_results;
        }
        request.excluded_ids.truncate(self.config.max_candidates);
        request.recently_served_ids.truncate(self.config.max_candidates);
        Ok(request)
    }

    fn retrieve(
        &self,
        request: &RecommendationRequest,
        diagnostics: &mut PipelineDiagnostics,
        warnings: &mut Vec<PipelineWarning>,
    ) -> Result<Vec<Candidate>, MixerError> {
        if self.config.thunder_policy != SourcePolicy::FallbackOnly {
            match self.thunder.retrieve(request, self.config.max_candidates) {
                Ok(candidates) if !candidates.is_empty() => return Ok(candidates),
                Ok(_) if !self.config.use_fallback_on_empty => return Ok(Vec::new()),
                Ok(_) => warnings.push(PipelineWarning {
                    code: "empty_primary_source".to_string(),
                    source: Some(self.thunder.name().to_string()),
                    message: "primary source returned no candidates".to_string(),
                }),
                Err(error) if self.config.thunder_policy == SourcePolicy::Required => {
                    return Err(error);
                }
                Err(error) => {
                    record_partial_failure(&self.thunder, error, diagnostics, warnings);
                }
            }
        }

        diagnostics.fallback_used = true;
        Ok(self
            .fallback_candidates
            .iter()
            .take(self.config.max_candidates)
            .cloned()
            .collect())
    }

    fn score(
        &self,
        mut candidate: Candidate,
        request: &RecommendationRequest,
        warnings: &mut Vec<PipelineWarning>,
    ) -> ScoredCandidate {
        let relevance = sanitize_feature(
            candidate.id,
            "relevance",
            candidate.features.relevance,
            warnings,
        );
        let engagement = sanitize_feature(
            candidate.id,
            "engagement",
            candidate.features.engagement,
            warnings,
        );
        let quality = sanitize_feature(
            candidate.id,
            "quality",
            candidate.features.quality,
            warnings,
        );
        let age = candidate
            .features
            .freshness_secs
            .unwrap_or_else(|| request.now_secs.saturating_sub(candidate.created_at_secs));
        candidate.features.freshness_secs = Some(age);
        let freshness =
            2.0_f64.powf(-(age as f64) / self.config.freshness_half_life_secs as f64);

        let raw_and_weights = [
            ("relevance", relevance, self.config.weights.relevance),
            ("engagement", engagement, self.config.weights.engagement),
            ("quality", quality, self.config.weights.quality),
            ("freshness", freshness, self.config.weights.freshness),
        ];
        let total_weight = raw_and_weights
            .iter()
            .map(|(_, _, weight)| weight)
            .sum::<f64>();
        let components = raw_and_weights
            .into_iter()
            .map(|(name, raw, weight)| ScoreComponent {
                name: name.to_string(),
                raw,
                weight,
                contribution: raw * weight / total_weight,
            })
            .collect::<Vec<_>>();
        let score = components
            .iter()
            .map(|component| component.contribution)
            .sum();

        ScoredCandidate {
            candidate,
            score,
            components,
        }
    }

    fn apply_diversity(
        &self,
        scored: Vec<ScoredCandidate>,
        limit: usize,
    ) -> Vec<ScoredCandidate> {
        let mut authors = BTreeMap::<u64, usize>::new();
        let mut categories = BTreeMap::<String, usize>::new();
        let mut selected = Vec::new();

        for candidate in scored {
            let author_count = authors.get(&candidate.candidate.author_id).copied().unwrap_or(0);
            if author_count >= self.config.max_per_author {
                continue;
            }
            let category = candidate
                .candidate
                .features
                .category
                .clone()
                .unwrap_or_else(|| "uncategorized".to_string());
            let category_count = categories.get(&category).copied().unwrap_or(0);
            if category_count >= self.config.max_per_category {
                continue;
            }

            *authors.entry(candidate.candidate.author_id).or_insert(0) += 1;
            *categories.entry(category).or_insert(0) += 1;
            selected.push(candidate);
            if selected.len() == limit {
                break;
            }
        }
        selected
    }
}

fn merge_features(
    candidates: &mut [Candidate],
    features: BTreeMap<ContentId, CandidateFeatures>,
) {
    for candidate in candidates {
        if let Some(hydrated) = features.get(&candidate.id) {
            candidate.features = hydrated.clone();
        }
    }
}

fn sanitize_feature(
    id: ContentId,
    name: &str,
    value: Option<f64>,
    warnings: &mut Vec<PipelineWarning>,
) -> f64 {
    match value {
        Some(value) if value.is_finite() => value.clamp(0.0, 1.0),
        Some(_) => {
            warnings.push(PipelineWarning {
                code: "invalid_feature".to_string(),
                source: None,
                message: format!("candidate {} has non-finite {name}", id.0),
            });
            0.0
        }
        None => 0.0,
    }
}

fn record_partial_failure(
    thunder: &ThunderAdapter,
    error: MixerError,
    diagnostics: &mut PipelineDiagnostics,
    warnings: &mut Vec<PipelineWarning>,
) {
    diagnostics.partial_failures.push(PartialFailure {
        source: thunder.name().to_string(),
        category: format!("{:?}", error.category),
        message: error.message.clone(),
    });
    warnings.push(PipelineWarning {
        code: "source_degraded".to_string(),
        source: Some(thunder.name().to_string()),
        message: error.message,
    });
}

fn source_name(source: CandidateSource) -> &'static str {
    match source {
        CandidateSource::Thunder => "thunder",
        CandidateSource::Simulation => "simulation",
        CandidateSource::Fixture => "fixture",
        CandidateSource::Baseline => "baseline",
    }
}

fn unix_time_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn run_cli(arguments: impl Iterator<Item = String>) -> Result<(), MixerError> {
    let mut arguments = arguments;
    match arguments.next().as_deref() {
        Some("home-mixer") => {}
        _ => {
            return Err(MixerError::new(
                ErrorCategory::InvalidRequest,
                "usage: eoro home-mixer [fixture.json]",
            ));
        }
    }

    let fixture = match arguments.next() {
        Some(path) => load_fixture(Path::new(&path))?,
        None => example_fixture(),
    };
    if arguments.next().is_some() {
        return Err(MixerError::new(
            ErrorCategory::InvalidRequest,
            "home-mixer accepts at most one fixture path",
        ));
    }

    let mut config = HomeMixerConfig::default();
    if matches!(
        std::env::var("EORO_HOME_MIXER_ENABLED").as_deref(),
        Ok("0" | "false" | "off")
    ) {
        config.enabled = false;
    }
    let thunder = ThunderAdapter::initialize(
        Arc::new(InMemoryThunder::new("fixture-thunder", fixture.candidates)),
        CandidateSource::Thunder,
        Duration::from_millis(config.source_timeout_ms),
        config.max_candidates,
    )?;
    let mixer = HomeMixer::new(config, thunder, fixture.fallback_candidates)?;
    let response = mixer.mix(fixture.request)?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

fn load_fixture(path: &Path) -> Result<MixerFixture, MixerError> {
    let bytes = std::fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn example_fixture() -> MixerFixture {
    MixerFixture {
        request: RecommendationRequest {
            user_id: 1,
            session_id: "local".to_string(),
            request_id: "example-request".to_string(),
            limit: 3,
            now_secs: unix_time_secs(),
            ..RecommendationRequest::default()
        },
        candidates: vec![
            example_candidate(101, 11, 0.9, 0.8, "science"),
            example_candidate(102, 12, 0.8, 0.9, "games"),
            example_candidate(103, 13, 0.7, 0.7, "science"),
        ],
        fallback_candidates: vec![example_candidate(201, 21, 0.5, 0.5, "baseline")],
    }
}

fn example_candidate(
    id: u64,
    author_id: u64,
    relevance: f64,
    quality: f64,
    category: &str,
) -> Candidate {
    Candidate {
        id: ContentId(id),
        author_id,
        created_at_secs: unix_time_secs(),
        features: CandidateFeatures {
            relevance: Some(relevance),
            engagement: Some(0.6),
            quality: Some(quality),
            freshness_secs: Some(0),
            category: Some(category.to_string()),
        },
        ..Candidate::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thunder::InMemoryThunder;

    fn mixer_with(
        candidates: Vec<Candidate>,
        fallback: Vec<Candidate>,
        configure: impl FnOnce(&mut HomeMixerConfig),
    ) -> HomeMixer {
        let mut config = HomeMixerConfig::default();
        configure(&mut config);
        let adapter = ThunderAdapter::initialize(
            Arc::new(InMemoryThunder::new("test-thunder", candidates)),
            CandidateSource::Thunder,
            Duration::from_millis(config.source_timeout_ms),
            config.max_candidates,
        )
        .unwrap();
        HomeMixer::new(config, adapter, fallback).unwrap()
    }

    fn request(limit: usize) -> RecommendationRequest {
        RecommendationRequest {
            user_id: 1,
            session_id: "session".to_string(),
            request_id: "request".to_string(),
            limit,
            now_secs: 100,
            ..RecommendationRequest::default()
        }
    }

    fn candidate(id: u64, author: u64, score: f64, category: &str) -> Candidate {
        Candidate {
            id: ContentId(id),
            author_id: author,
            created_at_secs: 100,
            features: CandidateFeatures {
                relevance: Some(score),
                engagement: Some(score),
                quality: Some(score),
                freshness_secs: Some(0),
                category: Some(category.to_string()),
            },
            ..Candidate::default()
        }
    }

    #[test]
    fn ranks_deduplicates_filters_and_breaks_ties_by_id() {
        let mut unsafe_candidate = candidate(9, 9, 1.0, "unsafe");
        unsafe_candidate.safe = false;
        let mixer = mixer_with(
            vec![
                candidate(2, 2, 0.8, "a"),
                candidate(1, 1, 0.8, "a"),
                candidate(1, 1, 0.8, "a"),
                unsafe_candidate,
            ],
            vec![],
            |_| {},
        );

        let response = mixer.mix(request(10)).unwrap();
        assert_eq!(
            response
                .results
                .iter()
                .map(|result| result.candidate.id.0)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(response.diagnostics.duplicate_count, 1);
        assert_eq!(response.diagnostics.stages.len(), 12);
    }

    #[test]
    fn falls_back_when_primary_is_empty() {
        let mixer = mixer_with(vec![], vec![candidate(7, 1, 0.5, "base")], |_| {});
        let response = mixer.mix(request(2)).unwrap();
        assert!(response.diagnostics.fallback_used);
        assert_eq!(response.results[0].candidate.source, CandidateSource::Baseline);
    }

    #[test]
    fn handles_missing_and_non_finite_features() {
        let mut invalid = candidate(1, 1, 0.5, "a");
        invalid.features.relevance = Some(f64::NAN);
        invalid.features.quality = None;
        let mixer = mixer_with(vec![invalid], vec![], |_| {});

        let response = mixer.mix(request(1)).unwrap();
        assert!(response.results[0].score.is_finite());
        assert!(
            response
                .warnings
                .iter()
                .any(|warning| warning.code == "invalid_feature")
        );
    }

    #[test]
    fn enforces_author_and_category_diversity() {
        let mixer = mixer_with(
            vec![
                candidate(1, 1, 1.0, "a"),
                candidate(2, 1, 0.9, "b"),
                candidate(3, 2, 0.8, "a"),
                candidate(4, 3, 0.7, "c"),
            ],
            vec![],
            |config| {
                config.max_per_author = 1;
                config.max_per_category = 1;
            },
        );

        let response = mixer.mix(request(4)).unwrap();
        assert_eq!(
            response
                .results
                .iter()
                .map(|result| result.candidate.id.0)
                .collect::<Vec<_>>(),
            vec![1, 4]
        );
    }

    #[test]
    fn records_timeout_as_partial_failure_and_uses_fallback() {
        let mut config = HomeMixerConfig::default();
        config.source_timeout_ms = 1;
        let adapter = ThunderAdapter::initialize(
            Arc::new(
                InMemoryThunder::new("slow-thunder", vec![candidate(1, 1, 1.0, "a")])
                    .with_delay(Duration::from_millis(50)),
            ),
            CandidateSource::Thunder,
            Duration::from_millis(config.source_timeout_ms),
            config.max_candidates,
        )
        .unwrap();
        let mixer =
            HomeMixer::new(config, adapter, vec![candidate(2, 2, 0.5, "base")]).unwrap();

        let response = mixer.mix(request(1)).unwrap();
        assert!(response.diagnostics.fallback_used);
        assert_eq!(response.diagnostics.partial_failures.len(), 1);
    }

    #[test]
    fn required_source_failure_is_fatal() {
        let config = HomeMixerConfig {
            thunder_policy: SourcePolicy::Required,
            source_timeout_ms: 1,
            ..HomeMixerConfig::default()
        };
        let adapter = ThunderAdapter::initialize(
            Arc::new(
                InMemoryThunder::new("slow-thunder", vec![])
                    .with_delay(Duration::from_millis(50)),
            ),
            CandidateSource::Thunder,
            Duration::from_millis(1),
            config.max_candidates,
        )
        .unwrap();
        let mixer = HomeMixer::new(config, adapter, vec![]).unwrap();

        let error = mixer.mix(request(1)).unwrap_err();
        assert_eq!(error.category, ErrorCategory::Timeout);
    }

    #[test]
    fn rejects_malformed_request_and_clamps_large_limit() {
        let mixer = mixer_with(vec![candidate(1, 1, 1.0, "a")], vec![], |config| {
            config.max_request_results = 1;
        });
        let mut malformed = request(1);
        malformed.request_id.clear();
        assert_eq!(
            mixer.mix(malformed).unwrap_err().category,
            ErrorCategory::InvalidRequest
        );

        let response = mixer.mix(request(20)).unwrap();
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.warnings[0].code, "limit_clamped");
    }

    #[test]
    fn emits_empty_feed_diagnostic() {
        let mixer = mixer_with(vec![], vec![], |_| {});
        let response = mixer.mix(request(1)).unwrap();
        assert!(response.results.is_empty());
        assert!(response.diagnostics.empty_feed);
    }
}
