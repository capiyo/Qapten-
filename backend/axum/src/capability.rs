use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PollStatus {
    Collecting,
    Decided,
    NoMajority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCandidate {
    pub model_id: String,
    pub provider: String,
    pub display_name: String,
    pub capabilities: Vec<String>,
    pub estimated_cost_per_1k_tokens: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelNomination {
    pub voter_model_id: String,
    pub nominated_model_id: String,
    pub rationale: String,
    pub submitted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityPoll {
    pub id: Uuid,
    pub capability: String,
    pub task_brief: String,
    pub electorate: Vec<String>,
    pub candidates: Vec<ModelCandidate>,
    pub nominations: Vec<ModelNomination>,
    pub status: PollStatus,
    pub result: Option<CapabilityVoteResult>,
    pub created_at: DateTime<Utc>,
    pub finalized_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CandidateTally {
    pub model_id: String,
    pub votes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityVoteResult {
    pub winner_model_id: Option<String>,
    pub votes_cast: usize,
    pub majority_required: usize,
    pub tallies: Vec<CandidateTally>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NominationError {
    InvalidPoll,
    UnknownVoter,
    UnknownCandidate,
    SelfNomination,
    DuplicateBallot,
    InvalidRationale,
    PollClosed,
}

impl CapabilityPoll {
    pub fn new(
        capability: String,
        task_brief: String,
        electorate: Vec<String>,
        candidates: Vec<ModelCandidate>,
    ) -> Result<Self, NominationError> {
        if capability.trim().is_empty()
            || task_brief.trim().is_empty()
            || electorate.len() < 3
            || candidates.len() < 2
            || electorate.iter().any(|id| id.trim().is_empty())
            || candidates.iter().any(|candidate| candidate.model_id.trim().is_empty())
        {
            return Err(NominationError::InvalidPoll);
        }

        let unique_electorate: std::collections::HashSet<_> = electorate.iter().collect();
        let unique_candidates: std::collections::HashSet<_> =
            candidates.iter().map(|candidate| &candidate.model_id).collect();
        if unique_electorate.len() != electorate.len()
            || unique_candidates.len() != candidates.len()
        {
            return Err(NominationError::InvalidPoll);
        }

        Ok(Self {
            id: Uuid::new_v4(),
            capability: capability.trim().to_owned(),
            task_brief: task_brief.trim().to_owned(),
            electorate,
            candidates,
            nominations: Vec::new(),
            status: PollStatus::Collecting,
            result: None,
            created_at: Utc::now(),
            finalized_at: None,
        })
    }

    /// A model may nominate another model, but cannot vote for itself.
    /// One model gets exactly one ballot in a poll.
    pub fn nominate(
        &mut self,
        voter_model_id: &str,
        nominated_model_id: &str,
        rationale: &str,
    ) -> Result<(), NominationError> {
        if self.status != PollStatus::Collecting {
            return Err(NominationError::PollClosed);
        }
        if !self.electorate.iter().any(|id| id == voter_model_id) {
            return Err(NominationError::UnknownVoter);
        }
        if voter_model_id == nominated_model_id {
            return Err(NominationError::SelfNomination);
        }
        if !self
            .candidates
            .iter()
            .any(|candidate| candidate.model_id == nominated_model_id)
        {
            return Err(NominationError::UnknownCandidate);
        }
        if self
            .nominations
            .iter()
            .any(|nomination| nomination.voter_model_id == voter_model_id)
        {
            return Err(NominationError::DuplicateBallot);
        }
        let rationale = rationale.trim();
        if rationale.is_empty() || rationale.len() > 2_000 {
            return Err(NominationError::InvalidRationale);
        }

        self.nominations.push(ModelNomination {
            voter_model_id: voter_model_id.to_owned(),
            nominated_model_id: nominated_model_id.to_owned(),
            rationale: rationale.to_owned(),
            submitted_at: Utc::now(),
        });
        Ok(())
    }

    /// Finalization requires every configured voter to submit a ballot. The
    /// winner must receive a strict majority of valid ballots; plurality and
    /// ties never silently select a model.
    pub fn finalize(&mut self) -> Result<CapabilityVoteResult, NominationError> {
        if self.status != PollStatus::Collecting {
            return Err(NominationError::PollClosed);
        }
        if self.nominations.len() != self.electorate.len() {
            return Err(NominationError::InvalidPoll);
        }

        let mut tallies: Vec<CandidateTally> = self
            .candidates
            .iter()
            .map(|candidate| CandidateTally {
                model_id: candidate.model_id.clone(),
                votes: 0,
            })
            .collect();
        for nomination in &self.nominations {
            if let Some(tally) = tallies
                .iter_mut()
                .find(|tally| tally.model_id == nomination.nominated_model_id)
            {
                tally.votes += 1;
            }
        }
        tallies.sort_by(|left, right| {
            right
                .votes
                .cmp(&left.votes)
                .then_with(|| left.model_id.cmp(&right.model_id))
        });

        let votes_cast = self.nominations.len();
        let majority_required = votes_cast / 2 + 1;
        let winner_model_id = tallies
            .first()
            .filter(|candidate| candidate.votes >= majority_required)
            .map(|candidate| candidate.model_id.clone());

        let result = CapabilityVoteResult {
            winner_model_id: winner_model_id.clone(),
            votes_cast,
            majority_required,
            tallies,
        };
        self.status = if winner_model_id.is_some() {
            PollStatus::Decided
        } else {
            PollStatus::NoMajority
        };
        self.finalized_at = Some(Utc::now());
        self.result = Some(result.clone());
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> CapabilityPoll {
        CapabilityPoll::new(
            "visual_design".into(),
            "Design a responsive dashboard for a finance application.".into(),
            vec![
                "voter-a".into(),
                "voter-b".into(),
                "voter-c".into(),
                "voter-d".into(),
                "voter-e".into(),
            ],
            vec![
                ModelCandidate {
                    model_id: "model-alpha".into(),
                    provider: "provider-a".into(),
                    display_name: "Alpha".into(),
                    capabilities: vec!["ui_design".into()],
                    estimated_cost_per_1k_tokens: Some(0.001),
                },
                ModelCandidate {
                    model_id: "model-beta".into(),
                    provider: "provider-b".into(),
                    display_name: "Beta".into(),
                    capabilities: vec!["ui_design".into()],
                    estimated_cost_per_1k_tokens: Some(0.002),
                },
            ],
        )
        .unwrap()
    }

    #[test]
    fn strict_majority_selects_the_model_for_the_task() {
        let mut poll = fixture();
        for voter in ["voter-a", "voter-b", "voter-c"] {
            poll.nominate(voter, "model-alpha", "Strong visual composition and layout control.")
                .unwrap();
        }
        poll.nominate("voter-d", "model-beta", "Better at generating complete UI code.")
            .unwrap();
        poll.nominate("voter-e", "model-beta", "Good visual reasoning.")
            .unwrap();

        let result = poll.finalize().unwrap();
        assert_eq!(result.winner_model_id.as_deref(), Some("model-alpha"));
        assert_eq!(result.majority_required, 3);
        assert_eq!(poll.status, PollStatus::Decided);
    }

    #[test]
    fn plurality_is_not_misreported_as_a_majority() {
        let mut poll = fixture();
        poll.electorate.push("voter-f".into());
        poll.nominate("voter-a", "model-alpha", "Reason one.").unwrap();
        poll.nominate("voter-b", "model-alpha", "Reason two.").unwrap();
        poll.nominate("voter-c", "model-beta", "Reason three.").unwrap();
        poll.nominate("voter-d", "model-beta", "Reason four.").unwrap();
        poll.nominate("voter-e", "model-alpha", "Reason five.").unwrap();
        poll.nominate("voter-f", "model-beta", "Reason six.").unwrap();

        let result = poll.finalize().unwrap();
        assert_eq!(result.winner_model_id, None);
        assert_eq!(poll.status, PollStatus::NoMajority);
    }

    #[test]
    fn rejects_self_votes_duplicate_ballots_and_unknown_voters() {
        let mut poll = fixture();
        assert_eq!(
            poll.nominate("voter-a", "voter-a", "Self vote."),
            Err(NominationError::SelfNomination)
        );
        poll.nominate("voter-a", "model-alpha", "First ballot.").unwrap();
        assert_eq!(
            poll.nominate("voter-a", "model-beta", "Second ballot."),
            Err(NominationError::DuplicateBallot)
        );
        assert_eq!(
            poll.nominate("not-in-electorate", "model-beta", "Unknown voter."),
            Err(NominationError::UnknownVoter)
        );
    }
}
