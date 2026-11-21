use crate::capability::{CapabilityPoll, CapabilityVoteResult};
use chrono::{DateTime, Utc};
use futures_util::TryStreamExt;
use mongodb::{
    bson::doc,
    options::{FindOptions, IndexOptions},
    Client, Collection, IndexModel,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    Semantic,
    Episodic,
    Project,
    Decision,
    Preference,
    Procedural,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStatus {
    Proposed,
    Active,
    Rejected,
    Superseded,
}

impl MemoryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Active => "active",
            Self::Rejected => "rejected",
            Self::Superseded => "superseded",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySource {
    pub provider: String,
    pub conversation_id: String,
    pub import_id: Uuid,
    #[serde(default)]
    pub message_ids: Vec<String>,
    pub captured_at: DateTime<Utc>,
    pub source_content_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub kind: MemoryKind,
    pub title: String,
    pub content: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub source: MemorySource,
    pub confidence: f64,
    pub status: MemoryStatus,
    pub version: u64,
    pub content_sha256: String,
    pub superseded_by_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedMessage {
    pub id: Option<String>,
    pub role: String,
    pub content: String,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationImport {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub provider: String,
    pub conversation_id: String,
    pub title: String,
    pub messages: Vec<ImportedMessage>,
    pub content_sha256: String,
    pub message_count: usize,
    pub imported_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ImportConversationRequest {
    pub provider: String,
    pub conversation_id: String,
    pub title: String,
    pub messages: Vec<ImportedMessage>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMemoryRequest {
    pub kind: MemoryKind,
    pub title: String,
    pub content: String,
    pub summary: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub import_id: Uuid,
    #[serde(default)]
    pub message_ids: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryReviewDecision {
    Approve,
    Reject,
    Supersede,
}

#[derive(Debug, Deserialize)]
pub struct ReviewMemoryRequest {
    pub expected_version: u64,
    pub decision: MemoryReviewDecision,
    pub superseded_by_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct MemoryRepository {
    memories: Collection<MemoryRecord>,
    imports: Collection<ConversationImport>,
    capability_polls: Collection<CapabilityPoll>,
}

impl MemoryRepository {
    pub async fn connect(uri: &str, database_name: &str) -> mongodb::error::Result<Self> {
        let client = Client::with_uri_str(uri).await?;
        client
            .database("admin")
            .run_command(doc! { "ping": 1 }, None)
            .await?;

        let database = client.database(database_name);
        let memories = database.collection::<MemoryRecord>("memories");
        let imports = database.collection::<ConversationImport>("conversation_imports");
        let capability_polls = database.collection::<CapabilityPoll>("capability_polls");

        memories
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "workspace_id": 1, "status": 1, "updated_at": -1 })
                    .options(
                        IndexOptions::builder()
                            .name("memory_workspace_status_updated".into())
                            .build(),
                    )
                    .build(),
                None,
            )
            .await?;
        memories
            .create_index(
                IndexModel::builder()
                    .keys(doc! {
                        "title": "text",
                        "content": "text",
                        "summary": "text",
                        "tags": "text"
                    })
                    .options(IndexOptions::builder().name("memory_text_search".into()).build())
                    .build(),
                None,
            )
            .await?;
        imports
            .create_index(
                IndexModel::builder()
                    .keys(doc! {
                        "workspace_id": 1,
                        "provider": 1,
                        "conversation_id": 1,
                        "content_sha256": 1
                    })
                    .options(
                        IndexOptions::builder()
                            .name("conversation_import_identity".into())
                            .unique(true)
                            .build(),
                    )
                    .build(),
                None,
            )
            .await?;

        capability_polls
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "workspace_id": 1, "created_at": -1 })
                    .options(
                        IndexOptions::builder()
                            .name("capability_poll_workspace_time".into())
                            .build(),
                    )
                    .build(),
                None,
            )
            .await?;

        Ok(Self {
            memories,
            imports,
            capability_polls,
        })
    }

    pub async fn import_conversation(
        &self,
        workspace_id: Uuid,
        input: ImportConversationRequest,
    ) -> mongodb::error::Result<ConversationImport> {
        let canonical = input
            .messages
            .iter()
            .map(|message| format!("{}\n{}", message.role, message.content))
            .collect::<Vec<_>>()
            .join("\n\n");
        let content_sha256 = sha256(&canonical);
        if let Some(existing) = self
            .imports
            .find_one(
                doc! {
                    "workspace_id": workspace_id.to_string(),
                    "provider": input.provider.trim(),
                    "conversation_id": input.conversation_id.trim(),
                    "content_sha256": &content_sha256,
                },
                None,
            )
            .await?
        {
            return Ok(existing);
        }
        let record = ConversationImport {
            id: Uuid::new_v4(),
            workspace_id,
            provider: input.provider.trim().to_owned(),
            conversation_id: input.conversation_id.trim().to_owned(),
            title: input.title.trim().to_owned(),
            message_count: input.messages.len(),
            messages: input.messages,
            content_sha256,
            imported_at: Utc::now(),
        };
        self.imports.insert_one(record.clone(), None).await?;
        Ok(record)
    }

    pub async fn create_memory(
        &self,
        workspace_id: Uuid,
        input: CreateMemoryRequest,
    ) -> mongodb::error::Result<MemoryRecord> {
        let source = self
            .imports
            .find_one(
                doc! {
                    "id": input.import_id.to_string(),
                    "workspace_id": workspace_id.to_string()
                },
                None,
            )
            .await?
            .ok_or_else(|| {
                mongodb::error::Error::custom(
                    "Source conversation import was not found in this workspace.",
                )
            })?;

        let imported_message_ids: std::collections::HashSet<String> = source
            .messages
            .iter()
            .filter_map(|message| message.id.clone())
            .collect();
        if input.message_ids.iter().any(|message_id| !imported_message_ids.contains(message_id)) {
            return Err(mongodb::error::Error::custom(
                "A memory source message ID does not exist in the referenced conversation import.",
            ));
        }

        let now = Utc::now();
        let record = MemoryRecord {
            id: Uuid::new_v4(),
            workspace_id,
            kind: input.kind,
            title: input.title.trim().to_owned(),
            content: input.content.trim().to_owned(),
            summary: input.summary.trim().to_owned(),
            tags: input.tags.into_iter().map(|tag| tag.trim().to_owned()).collect(),
            source: MemorySource {
                provider: source.provider,
                conversation_id: source.conversation_id,
                import_id: source.id,
                message_ids: input.message_ids,
                captured_at: source.imported_at,
                source_content_sha256: source.content_sha256,
            },
            confidence: input.confidence,
            status: MemoryStatus::Proposed,
            version: 1,
            content_sha256: sha256(input.content.trim()),
            superseded_by_id: None,
            created_at: now,
            updated_at: now,
        };
        self.memories.insert_one(record.clone(), None).await?;
        Ok(record)
    }

    pub async fn list_memories(
        &self,
        workspace_id: Uuid,
        status: Option<MemoryStatus>,
        query: Option<&str>,
        limit: i64,
    ) -> mongodb::error::Result<Vec<MemoryRecord>> {
        let mut filter = doc! { "workspace_id": workspace_id.to_string() };
        if let Some(status) = status {
            filter.insert("status", status.as_str());
        }
        if let Some(query) = query.map(str::trim).filter(|value| !value.is_empty()) {
            filter.insert("$text", doc! { "$search": query });
        }
        let options = FindOptions::builder()
            .sort(doc! { "updated_at": -1 })
            .limit(limit.clamp(1, 200))
            .build();
        let cursor = self.memories.find(filter, options).await?;
        cursor.try_collect().await
    }

    pub async fn review_memory(
        &self,
        workspace_id: Uuid,
        memory_id: Uuid,
        input: ReviewMemoryRequest,
    ) -> mongodb::error::Result<Option<MemoryRecord>> {
        if !matches!(&input.decision, MemoryReviewDecision::Supersede)
            && input.superseded_by_id.is_some()
        {
            return Err(mongodb::error::Error::custom(
                "superseded_by_id is only valid for a supersede decision.",
            ));
        }
        if matches!(&input.decision, MemoryReviewDecision::Supersede) {
            let replacement_id = input.superseded_by_id.ok_or_else(|| {
                mongodb::error::Error::custom(
                    "superseded_by_id is required when superseding a memory.",
                )
            })?;
            let replacement = self.memories.find_one(doc! {
                "id": replacement_id.to_string(),
                "workspace_id": workspace_id.to_string(),
                "status": "active",
            }, None).await?;
            if replacement.is_none() {
                return Err(mongodb::error::Error::custom(
                    "Replacement memory must exist and be active in this workspace.",
                ));
            }
        }
        let expected_status = match &input.decision {
            MemoryReviewDecision::Approve | MemoryReviewDecision::Reject => "proposed",
            MemoryReviewDecision::Supersede => "active",
        };
        let next_status = match input.decision {
            MemoryReviewDecision::Approve => MemoryStatus::Active,
            MemoryReviewDecision::Reject => MemoryStatus::Rejected,
            MemoryReviewDecision::Supersede => MemoryStatus::Superseded,
        };
        let update = doc! {
            "$set": {
                "status": next_status.as_str(),
                "superseded_by_id": input.superseded_by_id.map(|id| id.to_string()),
                "updated_at": Utc::now().to_rfc3339(),
            },
            "$inc": { "version": 1_i64 },
        };
        let result = self
            .memories
            .update_one(
                doc! {
                    "id": memory_id.to_string(),
                    "workspace_id": workspace_id.to_string(),
                    "status": expected_status,
                    "version": i64::try_from(input.expected_version).unwrap_or(i64::MAX),
                },
                update,
                None,
            )
            .await?;
        if result.matched_count == 0 {
            return Ok(None);
        }
        self.memories
            .find_one(
                doc! { "id": memory_id.to_string(), "workspace_id": workspace_id.to_string() },
                None,
            )
            .await
    }

    pub async fn create_capability_poll(
        &self,
        poll: CapabilityPoll,
    ) -> mongodb::error::Result<CapabilityPoll> {
        self.capability_polls.insert_one(poll.clone(), None).await?;
        Ok(poll)
    }

    pub async fn get_capability_poll(
        &self,
        workspace_id: Uuid,
        poll_id: Uuid,
    ) -> mongodb::error::Result<Option<CapabilityPoll>> {
        self.capability_polls
            .find_one(
                doc! {
                    "id": poll_id.to_string(),
                    "workspace_id": workspace_id.to_string()
                },
                None,
            )
            .await
    }

    pub async fn submit_capability_nomination(
        &self,
        workspace_id: Uuid,
        poll_id: Uuid,
        voter_model_id: &str,
        nominated_model_id: &str,
        rationale: &str,
    ) -> mongodb::error::Result<CapabilityPoll> {
        let mut poll = self
            .get_capability_poll(workspace_id, poll_id)
            .await?
            .ok_or_else(|| mongodb::error::Error::custom("Capability poll was not found."))?;
        let expected_version = poll.version;
        poll.nominate(voter_model_id, nominated_model_id, rationale)
            .map_err(|error| mongodb::error::Error::custom(format!("{error:?}")))?;
        let result = self
            .capability_polls
            .replace_one(
                doc! {
                    "id": poll_id.to_string(),
                    "workspace_id": workspace_id.to_string(),
                    "version": i64::try_from(expected_version).unwrap_or(i64::MAX),
                    "status": "collecting",
                },
                poll.clone(),
                None,
            )
            .await?;
        if result.matched_count == 0 {
            return Err(mongodb::error::Error::custom(
                "Capability poll version conflict; reload before retrying.",
            ));
        }
        Ok(poll)
    }

    pub async fn finalize_capability_poll(
        &self,
        workspace_id: Uuid,
        poll_id: Uuid,
    ) -> mongodb::error::Result<(CapabilityPoll, CapabilityVoteResult)> {
        let mut poll = self
            .get_capability_poll(workspace_id, poll_id)
            .await?
            .ok_or_else(|| mongodb::error::Error::custom("Capability poll was not found."))?;
        let expected_version = poll.version;
        let result = poll
            .finalize()
            .map_err(|error| mongodb::error::Error::custom(format!("{error:?}")))?;
        let updated = self
            .capability_polls
            .replace_one(
                doc! {
                    "id": poll_id.to_string(),
                    "workspace_id": workspace_id.to_string(),
                    "version": i64::try_from(expected_version).unwrap_or(i64::MAX),
                    "status": "collecting",
                },
                poll.clone(),
                None,
            )
            .await?;
        if updated.matched_count == 0 {
            return Err(mongodb::error::Error::custom(
                "Capability poll version conflict; reload before retrying.",
            ));
        }
        Ok((poll, result))
    }

    pub async fn get_import(
        &self,
        workspace_id: Uuid,
        import_id: Uuid,
    ) -> mongodb::error::Result<Option<ConversationImport>> {
        self.imports
            .find_one(
                doc! { "id": import_id.to_string(), "workspace_id": workspace_id.to_string() },
                None,
            )
            .await
    }
}

pub fn sha256(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

pub fn validate_import(input: &ImportConversationRequest) -> Result<(), &'static str> {
    if input.provider.trim().is_empty() || input.provider.len() > 80 {
        return Err("Provider must contain 1–80 characters.");
    }
    if input.conversation_id.trim().is_empty() || input.conversation_id.len() > 256 {
        return Err("Conversation ID must contain 1–256 characters.");
    }
    if input.title.trim().is_empty() || input.title.len() > 300 {
        return Err("Conversation title must contain 1–300 characters.");
    }
    if input.messages.is_empty() || input.messages.len() > 5_000 {
        return Err("Conversation must contain between 1 and 5000 messages.");
    }
    let mut total_bytes = 0usize;
    for message in &input.messages {
        if message.role.trim().is_empty() || message.role.len() > 80 {
            return Err("Every message must have a valid role.");
        }
        if message.content.is_empty() {
            return Err("Messages cannot have empty content.");
        }
        if message.id.as_ref().is_some_and(|id| id.is_empty() || id.len() > 256) {
            return Err("Message IDs must contain 1–256 characters when provided.");
        }
        total_bytes = total_bytes.saturating_add(message.content.len());
    }
    if total_bytes > 900_000 {
        return Err("Conversation content exceeds the 900 KB import limit.");
    }
    Ok(())
}

pub fn validate_memory(input: &CreateMemoryRequest) -> Result<(), &'static str> {
    if input.title.trim().is_empty() || input.title.len() > 240 {
        return Err("Memory title must contain 1–240 characters.");
    }
    if input.content.trim().is_empty() || input.content.len() > 20_000 {
        return Err("Memory content must contain 1–20000 characters.");
    }
    if input.summary.trim().is_empty() || input.summary.len() > 2_000 {
        return Err("Memory summary must contain 1–2000 characters.");
    }
    if input.tags.len() > 30
        || input
            .tags
            .iter()
            .any(|tag| tag.trim().is_empty() || tag.len() > 60)
    {
        return Err(
            "Memory tags must be non-empty and limited to 30 tags of at most 60 characters.",
        );
    }
    if !input.confidence.is_finite() || !(0.0..=1.0).contains(&input.confidence) {
        return Err("Confidence must be a finite number between 0 and 1.");
    }
    if input.message_ids.len() > 200 {
        return Err("At most 200 source message IDs may be attached.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_is_stable_and_content_sensitive() {
        assert_eq!(sha256("axon"), sha256("axon"));
        assert_ne!(sha256("axon"), sha256("Axon"));
    }

    #[test]
    fn import_validation_bounds_untrusted_conversation_data() {
        let valid = ImportConversationRequest {
            provider: "claude".into(),
            conversation_id: "conversation-1".into(),
            title: "Architecture discussion".into(),
            messages: vec![ImportedMessage {
                id: Some("m1".into()),
                role: "user".into(),
                content: "Build persistent memory.".into(),
                created_at: None,
            }],
        };
        assert!(validate_import(&valid).is_ok());
        let mut invalid = valid;
        invalid.messages.clear();
        assert!(validate_import(&invalid).is_err());
    }

    #[test]
    fn memory_validation_rejects_invalid_confidence_and_empty_content() {
        let valid = CreateMemoryRequest {
            kind: MemoryKind::Project,
            title: "Axon memory".into(),
            content: "Axon owns canonical memory.".into(),
            summary: "Canonical provider-independent memory.".into(),
            tags: vec!["axon".into()],
            import_id: Uuid::new_v4(),
            message_ids: vec![],
            confidence: 0.9,
        };
        assert!(validate_memory(&valid).is_ok());
        let mut invalid = valid;
        invalid.confidence = f64::NAN;
        assert!(validate_memory(&invalid).is_err());
    }
}
