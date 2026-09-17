use crate::error::{ErrorCategory, MixerError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourcePolicy {
    Required,
    Optional,
    FallbackOnly,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ScoreWeights {
    pub relevance: f64,
    pub engagement: f64,
    pub quality: f64,
    pub freshness: f64,
}

impl Default for ScoreWeights {
    fn default() -> Self {
        Self {
            relevance: 0.45,
            engagement: 0.25,
            quality: 0.20,
            freshness: 0.10,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct HomeMixerConfig {
    pub enabled: bool,
    pub max_request_results: usize,
    pub max_candidates: usize,
    pub max_per_author: usize,
    pub max_per_category: usize,
    pub source_timeout_ms: u64,
    pub freshness_half_life_secs: u64,
    pub thunder_policy: SourcePolicy,
    pub use_fallback_on_empty: bool,
    pub weights: ScoreWeights,
}

impl Default for HomeMixerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_request_results: 100,
            max_candidates: 1_200,
            max_per_author: 3,
            max_per_category: 10,
            source_timeout_ms: 250,
            freshness_half_life_secs: 86_400,
            thunder_policy: SourcePolicy::Optional,
            use_fallback_on_empty: true,
            weights: ScoreWeights::default(),
        }
    }
}

impl HomeMixerConfig {
    pub fn validate(&self) -> Result<(), MixerError> {
        if self.max_request_results == 0 || self.max_candidates == 0 {
            return Err(MixerError::new(
                ErrorCategory::Configuration,
                "result and candidate limits must be greater than zero",
            ));
        }
        if self.max_request_results > self.max_candidates {
            return Err(MixerError::new(
                ErrorCategory::Configuration,
                "max_request_results cannot exceed max_candidates",
            ));
        }
        if self.max_per_author == 0 || self.max_per_category == 0 {
            return Err(MixerError::new(
                ErrorCategory::Configuration,
                "diversity limits must be greater than zero",
            ));
        }
        if self.source_timeout_ms == 0 || self.freshness_half_life_secs == 0 {
            return Err(MixerError::new(
                ErrorCategory::Configuration,
                "timeouts and freshness half-life must be greater than zero",
            ));
        }
        let weights = [
            self.weights.relevance,
            self.weights.engagement,
            self.weights.quality,
            self.weights.freshness,
        ];
        if weights
            .iter()
            .any(|weight| !weight.is_finite() || *weight < 0.0)
            || weights.iter().sum::<f64>() <= 0.0
        {
            return Err(MixerError::new(
                ErrorCategory::Configuration,
                "score weights must be finite, non-negative, and not all zero",
            ));
        }
        Ok(())
    }
}
