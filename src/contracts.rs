use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ContentId(pub u64);

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateSource {
    #[default]
    Thunder,
    Simulation,
    Fixture,
    Baseline,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct CandidateFeatures {
    pub relevance: Option<f64>,
    pub engagement: Option<f64>,
    pub quality: Option<f64>,
    pub freshness_secs: Option<u64>,
    pub category: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Candidate {
    pub id: ContentId,
    pub author_id: u64,
    pub source: CandidateSource,
    pub created_at_secs: u64,
    pub eligible: bool,
    pub safe: bool,
    pub features: CandidateFeatures,
}

impl Default for Candidate {
    fn default() -> Self {
        Self {
            id: ContentId(0),
            author_id: 0,
            source: CandidateSource::Thunder,
            created_at_secs: 0,
            eligible: true,
            safe: true,
            features: CandidateFeatures::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct RecommendationRequest {
    pub user_id: u64,
    pub session_id: String,
    pub request_id: String,
    pub excluded_ids: Vec<ContentId>,
    pub recently_served_ids: Vec<ContentId>,
    pub limit: usize,
    pub now_secs: u64,
}

impl Default for RecommendationRequest {
    fn default() -> Self {
        Self {
            user_id: 0,
            session_id: String::new(),
            request_id: String::new(),
            excluded_ids: Vec::new(),
            recently_served_ids: Vec::new(),
            limit: 20,
            now_secs: 0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ScoreComponent {
    pub name: String,
    pub raw: f64,
    pub weight: f64,
    pub contribution: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ScoreExplanation {
    pub components: Vec<ScoreComponent>,
    pub summary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RankedResult {
    pub rank: usize,
    pub candidate: Candidate,
    pub score: f64,
    pub explanation: ScoreExplanation,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Pagination {
    pub next_cursor: Option<String>,
    pub returned: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PipelineWarning {
    pub code: String,
    pub source: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartialFailure {
    pub source: String,
    pub category: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct StageDiagnostic {
    pub stage: String,
    pub duration_micros: u128,
    pub input_count: usize,
    pub output_count: usize,
    pub rejected_count: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PipelineDiagnostics {
    pub request_id: String,
    pub stages: Vec<StageDiagnostic>,
    pub source_contributions: BTreeMap<String, usize>,
    pub partial_failures: Vec<PartialFailure>,
    pub duplicate_count: usize,
    pub fallback_used: bool,
    pub empty_feed: bool,
    pub min_score: Option<f64>,
    pub max_score: Option<f64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MixerResponse {
    pub results: Vec<RankedResult>,
    pub pagination: Pagination,
    pub warnings: Vec<PipelineWarning>,
    pub diagnostics: PipelineDiagnostics,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct MixerFixture {
    pub request: RecommendationRequest,
    pub candidates: Vec<Candidate>,
    pub fallback_candidates: Vec<Candidate>,
}
