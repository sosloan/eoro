use eoro::config::HomeMixerConfig;
use eoro::contracts::{
    Candidate, CandidateFeatures, CandidateSource, ContentId, RecommendationRequest,
};
use eoro::home_mixer::HomeMixer;
use eoro::thunder::{InMemoryThunder, ThunderAdapter};
use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

const ITERATIONS: u32 = 100;

fn main() {
    let candidates = (1..=1_200)
        .map(|id| Candidate {
            id: ContentId(id),
            author_id: id % 200,
            created_at_secs: 1_000,
            features: CandidateFeatures {
                relevance: Some((id % 100) as f64 / 100.0),
                engagement: Some((id % 80) as f64 / 80.0),
                quality: Some((id % 60) as f64 / 60.0),
                freshness_secs: Some(id),
                category: Some(format!("category-{}", id % 20)),
            },
            ..Candidate::default()
        })
        .collect::<Vec<_>>();
    let config = HomeMixerConfig::default();
    let adapter = ThunderAdapter::initialize(
        Arc::new(InMemoryThunder::new("benchmark-thunder", candidates)),
        CandidateSource::Thunder,
        Duration::from_secs(1),
        config.max_candidates,
    )
    .expect("valid benchmark adapter");
    let request = RecommendationRequest {
        user_id: 1,
        session_id: "benchmark".to_string(),
        request_id: "benchmark".to_string(),
        limit: 100,
        now_secs: 2_000,
        ..RecommendationRequest::default()
    };

    let retrieval_started = Instant::now();
    for _ in 0..ITERATIONS {
        black_box(
            adapter
                .retrieve(black_box(&request), 1_200)
                .expect("benchmark retrieval"),
        );
    }
    let retrieval = retrieval_started.elapsed();

    let ids = (1..=1_200).map(ContentId).collect::<Vec<_>>();
    let hydration_started = Instant::now();
    for _ in 0..ITERATIONS {
        black_box(
            adapter
                .hydrate(black_box(&ids))
                .expect("benchmark hydration"),
        );
    }
    let hydration = hydration_started.elapsed();

    let mixer = HomeMixer::new(config, adapter, vec![]).expect("valid benchmark mixer");
    let full_started = Instant::now();
    let mut ranking_micros = 0_u128;
    for _ in 0..ITERATIONS {
        let response = mixer
            .mix(black_box(request.clone()))
            .expect("benchmark pipeline");
        ranking_micros += response
            .diagnostics
            .stages
            .iter()
            .filter(|stage| matches!(stage.stage.as_str(), "score" | "normalize_combine"))
            .map(|stage| stage.duration_micros)
            .sum::<u128>();
        black_box(response);
    }
    let full = full_started.elapsed();

    println!(
        "iterations={ITERATIONS} retrieval_avg_us={} hydration_avg_us={} ranking_avg_us={} full_pipeline_avg_us={}",
        retrieval.as_micros() / u128::from(ITERATIONS),
        hydration.as_micros() / u128::from(ITERATIONS),
        ranking_micros / u128::from(ITERATIONS),
        full.as_micros() / u128::from(ITERATIONS),
    );
}
