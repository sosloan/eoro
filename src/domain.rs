use crate::GlobalSimulationState;
use crate::contracts::{Candidate, CandidateFeatures, CandidateSource, ContentId};

pub struct SimulationSnapshot {
    candidates: Vec<Candidate>,
}

impl SimulationSnapshot {
    pub fn capture(state: &GlobalSimulationState, now_secs: u64) -> Self {
        let mut candidates = Vec::new();

        candidates.extend(state.athletes.iter().map(|athlete| Candidate {
            id: ContentId(1_000_000_000 + u64::from(athlete.entity_id)),
            author_id: u64::from(athlete.entity_id),
            source: CandidateSource::Simulation,
            created_at_secs: now_secs,
            eligible: true,
            safe: true,
            features: CandidateFeatures {
                relevance: Some(1.0 - f64::from(athlete.cognitive_load.clamp(0.0, 1.0))),
                engagement: Some(f64::from(athlete.physical_exertion.clamp(0.0, 1.0))),
                quality: Some(0.8),
                freshness_secs: Some(0),
                category: Some("scholar-athlete".to_string()),
            },
        }));

        candidates.extend(state.biomes.zebras.members.iter().map(|zebra| Candidate {
            id: ContentId(2_000_000_000 + u64::from(zebra.id)),
            author_id: u64::from(zebra.id),
            source: CandidateSource::Simulation,
            created_at_secs: now_secs,
            eligible: zebra.stamina > 0.0,
            safe: true,
            features: CandidateFeatures {
                relevance: Some(f64::from(zebra.stamina.clamp(0.0, 1.0))),
                engagement: Some(f64::from((zebra.speed / 16.0).clamp(0.0, 1.0))),
                quality: Some(0.7),
                freshness_secs: Some(0),
                category: Some("zebra".to_string()),
            },
        }));

        for (index, orbit) in state.mission.orbits.iter().enumerate() {
            candidates.push(Candidate {
                id: ContentId(3_000_000_000 + index as u64),
                author_id: index as u64,
                source: CandidateSource::Simulation,
                created_at_secs: now_secs,
                eligible: orbit.mass.is_finite() && orbit.mass > 0.0,
                safe: true,
                features: CandidateFeatures {
                    relevance: Some(0.6),
                    engagement: Some(vector_magnitude(orbit.velocity).min(1.0)),
                    quality: Some(0.9),
                    freshness_secs: Some(0),
                    category: Some("mission".to_string()),
                },
            });
        }

        for (guild, names) in [
            ("reasoning", &state.collaboration.reasoning_agents),
            ("theorem-proving", &state.collaboration.theorem_provers),
            ("pattern-hunting", &state.collaboration.pattern_hunters),
        ] {
            candidates.extend(names.iter().map(|name| Candidate {
                id: ContentId(4_000_000_000 + stable_id(name)),
                author_id: stable_id(name),
                source: CandidateSource::Simulation,
                created_at_secs: now_secs,
                eligible: true,
                safe: true,
                features: CandidateFeatures {
                    relevance: Some(0.75),
                    engagement: Some(0.5),
                    quality: Some(0.85),
                    freshness_secs: Some(0),
                    category: Some(guild.to_string()),
                },
            }));
        }

        Self { candidates }
    }

    pub fn candidates(&self) -> &[Candidate] {
        &self.candidates
    }
}

fn vector_magnitude(vector: crate::Vector3) -> f64 {
    (vector.x * vector.x + vector.y * vector.y + vector.z * vector.z).sqrt()
}

fn stable_id(value: &str) -> u64 {
    value.bytes().fold(1_469_598_103_934_665_603, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(1_099_511_628_211)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BiomeEntities, CollaborationSystems, MissionArchitecture, ScholarAthlete,
        zebra::ZebraHerd,
    };

    #[test]
    fn captures_state_without_mutating_it() {
        let state = GlobalSimulationState {
            biomes: BiomeEntities {
                flora_health: vec![],
                fauna_stamina: vec![],
                deep_space_signals: vec![],
                zebras: ZebraHerd::new(),
            },
            athletes: vec![ScholarAthlete {
                entity_id: 7,
                cognitive_load: 0.2,
                physical_exertion: 0.4,
            }],
            collaboration: CollaborationSystems {
                reasoning_agents: vec!["Ada".to_string()],
                theorem_provers: vec![],
                pattern_hunters: vec![],
            },
            mission: MissionArchitecture {
                orbits: vec![],
                telemetry_data: vec![],
                trajectories: vec![],
            },
        };

        let snapshot = SimulationSnapshot::capture(&state, 100);
        assert_eq!(snapshot.candidates().len(), 2);
        assert_eq!(state.athletes.len(), 1);
    }
}
