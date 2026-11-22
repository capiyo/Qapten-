use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use uuid::Uuid;

const RESUME_PROMPT_VERSION: &str = "resume-v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chat {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: Uuid,
    pub chat_id: Uuid,
    pub role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub answered_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: Uuid,
    pub chat_id: Uuid,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub text: String,
    pub reason: Option<String>,
    pub source_message_id: Option<Uuid>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub approved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatDetail {
    pub chat: Chat,
    pub messages: Vec<ChatMessage>,
    pub memory: Vec<MemoryEntry>,
    pub resume_brief: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateChat {
    pub title: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateMessage {
    pub role: String,
    pub content: String,
    pub answered_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateMemoryEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    pub text: String,
    pub reason: Option<String>,
    pub source_message_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EditMemoryEntry {
    pub text: String,
    pub reason: Option<String>,
}

#[derive(Clone)]
pub struct ChatMemoryStore {
    pool: SqlitePool,
}

impl ChatMemoryStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_chats(&self, workspace_id: Uuid) -> Result<Vec<Chat>, sqlx::Error> {
        let rows = sqlx::query("SELECT id, workspace_id, title, created_at, updated_at, archived FROM chats WHERE workspace_id = ? AND archived = 0 ORDER BY updated_at DESC, id DESC")
            .bind(workspace_id.to_string()).fetch_all(&self.pool).await?;
        rows.iter().map(parse_chat).collect()
    }

    pub async fn create_chat(&self, workspace_id: Uuid, actor: &str, title: String) -> Result<Chat, sqlx::Error> {
        let now = Utc::now();
        let chat = Chat { id: Uuid::new_v4(), workspace_id, title, created_at: now, updated_at: now, archived: false };
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO chats (id, workspace_id, title, created_at, updated_at, archived, memory_version) VALUES (?, ?, ?, ?, ?, 0, 0)")
            .bind(chat.id.to_string()).bind(workspace_id.to_string()).bind(&chat.title)
            .bind(now.to_rfc3339()).bind(now.to_rfc3339()).execute(&mut *tx).await?;
        insert_audit(&mut tx, workspace_id, actor, "chat.created", "chat", chat.id, json!({"chat_id": chat.id})).await?;
        tx.commit().await?;
        Ok(chat)
    }

    pub async fn get_chat(&self, workspace_id: Uuid, chat_id: Uuid) -> Result<Option<ChatDetail>, sqlx::Error> {
        let row = sqlx::query("SELECT id, workspace_id, title, created_at, updated_at, archived, memory_version FROM chats WHERE id = ? AND workspace_id = ?")
            .bind(chat_id.to_string()).bind(workspace_id.to_string()).fetch_optional(&self.pool).await?;
        let Some(row) = row else { return Ok(None); };
        let chat = parse_chat(&row)?;
        let memory_version: i64 = row.try_get("memory_version")?;
        let messages = self.list_messages(chat_id).await?;
        let memory = self.rebuild_memory(chat_id).await?;
        let last_message_id = messages.last().map(|message| message.id.to_string());
        let cached = sqlx::query("SELECT memory_version, last_message_id, prompt_version, brief FROM resume_brief_cache WHERE chat_id = ?")
            .bind(chat_id.to_string()).fetch_optional(&self.pool).await?;
        let brief = if let Some(cached) = cached {
            let cached_version: i64 = cached.try_get("memory_version")?;
            let cached_last: Option<String> = cached.try_get("last_message_id")?;
            let cached_prompt: String = cached.try_get("prompt_version")?;
            if cached_version == memory_version && cached_last == last_message_id && cached_prompt == RESUME_PROMPT_VERSION {
                cached.try_get("brief")?
            } else {
                self.regenerate_brief(chat_id, workspace_id, memory_version, last_message_id, &memory, &messages).await?
            }
        } else {
            self.regenerate_brief(chat_id, workspace_id, memory_version, last_message_id, &memory, &messages).await?
        };
        Ok(Some(ChatDetail { chat, messages, memory, resume_brief: brief }))
    }

    async fn list_messages(&self, chat_id: Uuid) -> Result<Vec<ChatMessage>, sqlx::Error> {
        let rows = sqlx::query("SELECT id, chat_id, role, content, created_at, answered_by FROM chat_messages WHERE chat_id = ? ORDER BY created_at ASC, id ASC")
            .bind(chat_id.to_string()).fetch_all(&self.pool).await?;
        rows.iter().map(parse_message).collect()
    }

    pub async fn append_message(&self, workspace_id: Uuid, chat_id: Uuid, actor: &str, input: CreateMessage) -> Result<Option<ChatMessage>, sqlx::Error> {
        let now = Utc::now();
        let message = ChatMessage { id: Uuid::new_v4(), chat_id, role: input.role, content: input.content, created_at: now, answered_by: input.answered_by };
        let mut tx = self.pool.begin().await?;
        let exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM chats WHERE id = ? AND workspace_id = ? AND archived = 0")
            .bind(chat_id.to_string()).bind(workspace_id.to_string()).fetch_optional(&mut *tx).await?;
        if exists.is_none() { return Ok(None); }
        sqlx::query("INSERT INTO chat_messages (id, chat_id, role, content, created_at, answered_by) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(message.id.to_string()).bind(chat_id.to_string()).bind(&message.role).bind(&message.content)
            .bind(now.to_rfc3339()).bind(&message.answered_by).execute(&mut *tx).await?;
        sqlx::query("UPDATE chats SET updated_at = ? WHERE id = ?").bind(now.to_rfc3339()).bind(chat_id.to_string()).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM resume_brief_cache WHERE chat_id = ?").bind(chat_id.to_string()).execute(&mut *tx).await?;
        insert_audit(&mut tx, workspace_id, actor, "chat.message_created", "chat_message", message.id, json!({"chat_id": chat_id, "role": message.role})).await?;
        tx.commit().await?;
        Ok(Some(message))
    }

    pub async fn archive_chat(&self, workspace_id: Uuid, chat_id: Uuid, actor: &str) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query("UPDATE chats SET archived = 1, updated_at = ? WHERE id = ? AND workspace_id = ? AND archived = 0")
            .bind(Utc::now().to_rfc3339()).bind(chat_id.to_string()).bind(workspace_id.to_string()).execute(&mut *tx).await?;
        if result.rows_affected() == 1 {
            insert_audit(&mut tx, workspace_id, actor, "chat.archived", "chat", chat_id, json!({"chat_id": chat_id})).await?;
        }
        tx.commit().await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn create_memory_entry(&self, workspace_id: Uuid, chat_id: Uuid, actor: &str, input: CreateMemoryEntry) -> Result<Option<MemoryEntry>, sqlx::Error> {
        let now = Utc::now();
        let entry = MemoryEntry { id: Uuid::new_v4(), chat_id, entry_type: input.entry_type, text: input.text, reason: input.reason, source_message_id: input.source_message_id, status: "proposed".into(), created_at: now, approved_at: None };
        let mut tx = self.pool.begin().await?;
        let exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM chats WHERE id = ? AND workspace_id = ?")
            .bind(chat_id.to_string()).bind(workspace_id.to_string()).fetch_optional(&mut *tx).await?;
        if exists.is_none() { return Ok(None); }
        if let Some(source_id) = entry.source_message_id {
            let source_exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM chat_messages WHERE id = ? AND chat_id = ?")
                .bind(source_id.to_string()).bind(chat_id.to_string()).fetch_optional(&mut *tx).await?;
            if source_exists.is_none() { return Ok(None); }
        }
        append_memory_event(&mut tx, workspace_id, chat_id, entry.id, "created", &entry, actor).await?;
        bump_memory_version(&mut tx, chat_id, now).await?;
        insert_audit(&mut tx, workspace_id, actor, "memory.created", "memory_entry", entry.id, json!({"chat_id": chat_id, "type": entry.entry_type})).await?;
        tx.commit().await?;
        Ok(Some(entry))
    }

    pub async fn mutate_memory_entry(&self, workspace_id: Uuid, chat_id: Uuid, entry_id: Uuid, actor: &str, operation: &str, edit: Option<EditMemoryEntry>) -> Result<Option<MemoryEntry>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let mut current = replay_memory(&mut *tx, chat_id).await?.remove(&entry_id);
        let Some(mut entry) = current.take() else { return Ok(None); };
        if entry.status == "deleted" { return Ok(None); }
        let event_type = match operation {
            "approve" => { entry.status = "approved".into(); entry.approved_at = Some(Utc::now()); "approved" }
            "reject" => { entry.status = "rejected".into(); entry.approved_at = None; "rejected" }
            "edit" => {
                let Some(edit) = edit else { return Ok(None); };
                entry.text = edit.text;
                entry.reason = edit.reason;
                entry.status = "proposed".into();
                entry.approved_at = None;
                "edited"
            }
            "delete" => { entry.status = "deleted".into(); "deleted" }
            _ => return Ok(None),
        };
        let exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM chats WHERE id = ? AND workspace_id = ?")
            .bind(chat_id.to_string()).bind(workspace_id.to_string()).fetch_optional(&mut *tx).await?;
        if exists.is_none() { return Ok(None); }
        append_memory_event(&mut tx, workspace_id, chat_id, entry.id, event_type, &entry, actor).await?;
        bump_memory_version(&mut tx, chat_id, Utc::now()).await?;
        insert_audit(&mut tx, workspace_id, actor, &format!("memory.{event_type}"), "memory_entry", entry.id, json!({"chat_id": chat_id, "status": entry.status})).await?;
        tx.commit().await?;
        Ok(Some(entry))
    }

    async fn rebuild_memory(&self, chat_id: Uuid) -> Result<Vec<MemoryEntry>, sqlx::Error> {
        let mut connection = self.pool.acquire().await?;
        let mut entries = replay_memory(&mut *connection).await?.into_values().filter(|entry| entry.status != "deleted").collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.created_at);
        Ok(entries)
    }

    async fn regenerate_brief(&self, chat_id: Uuid, workspace_id: Uuid, memory_version: i64, last_message_id: Option<String>, memory: &[MemoryEntry], messages: &[ChatMessage]) -> Result<String, sqlx::Error> {
        let approved = memory.iter().filter(|entry| entry.status == "approved").collect::<Vec<_>>();
        let recent = messages.iter().rev().take(6).collect::<Vec<_>>();
        let section = |kind: &str| approved.iter().filter(|entry| entry.entry_type == kind).map(|entry| entry.text.as_str()).collect::<Vec<_>>();
        let mut lines = Vec::new();
        let where_left_off = recent.iter().rev().find(|message| message.role == "user" || message.role == "assistant")
            .map(|message| format!("{}: {}", message.role, message.content))
            .unwrap_or_else(|| "No conversation activity yet.".into());
        lines.push(format!("Where left off: {where_left_off}"));
        let decisions = section("decision");
        if !decisions.is_empty() { lines.push(format!("Decisions: {}", decisions.join("; "))); }
        let open_tasks = section("open_task");
        if !open_tasks.is_empty() { lines.push(format!("Open tasks: {}", open_tasks.join("; "))); }
        let next = section("note").into_iter().chain(section("constraint")).collect::<Vec<_>>();
        if !next.is_empty() { lines.push(format!("Next / constraints: {}", next.join("; "))); }
        let facts = section("fact");
        if !facts.is_empty() { lines.push(format!("Facts: {}", facts.join("; "))); }
        let brief = truncate_words(&lines.join("\\n"), 190);
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO resume_brief_cache (chat_id, memory_version, last_message_id, prompt_version, brief, generated_at) VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(chat_id) DO UPDATE SET memory_version = excluded.memory_version, last_message_id = excluded.last_message_id, prompt_version = excluded.prompt_version, brief = excluded.brief, generated_at = excluded.generated_at")
            .bind(chat_id.to_string()).bind(memory_version).bind(last_message_id).bind(RESUME_PROMPT_VERSION).bind(&brief).bind(Utc::now().to_rfc3339()).execute(&mut *tx).await?;
        let _ = workspace_id;
        tx.commit().await?;
        Ok(brief)
    }
}

fn truncate_words(value: &str, limit: usize) -> String {
    let words = value.split_whitespace().take(limit).collect::<Vec<_>>();
    words.join(" ")
}

fn parse_chat(row: &sqlx::sqlite::SqliteRow) -> Result<Chat, sqlx::Error> {
    Ok(Chat {
        id: Uuid::parse_str(&row.try_get::<String, _>("id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
        workspace_id: Uuid::parse_str(&row.try_get::<String, _>("workspace_id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
        title: row.try_get("title")?,
        created_at: DateTime::parse_from_rfc3339(&row.try_get::<String, _>("created_at")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?.with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&row.try_get::<String, _>("updated_at")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?.with_timezone(&Utc),
        archived: row.try_get::<i64, _>("archived")? != 0,
    })
}

fn parse_message(row: &sqlx::sqlite::SqliteRow) -> Result<ChatMessage, sqlx::Error> {
    Ok(ChatMessage {
        id: Uuid::parse_str(&row.try_get::<String, _>("id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
        chat_id: Uuid::parse_str(&row.try_get::<String, _>("chat_id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
        role: row.try_get("role")?,
        content: row.try_get("content")?,
        created_at: DateTime::parse_from_rfc3339(&row.try_get::<String, _>("created_at")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?.with_timezone(&Utc),
        answered_by: row.try_get("answered_by")?,
    })
}

async fn replay_memory<'a, E>(executor: E, chat_id: Uuid) -> Result<HashMap<Uuid, MemoryEntry>, sqlx::Error>
where E: sqlx::Executor<'a, Database = sqlx::Sqlite> {
    let rows = sqlx::query("SELECT payload_json FROM memory_events WHERE chat_id = ? ORDER BY sequence ASC")
        .bind(chat_id.to_string()).fetch_all(executor).await?;
    let mut entries = HashMap::new();
    for row in rows {
        let payload: String = row.try_get("payload_json")?;
        let entry: MemoryEntry = serde_json::from_str(&payload).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        entries.insert(entry.id, entry);
    }
    Ok(entries)
}

async fn append_memory_event(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, workspace_id: Uuid, chat_id: Uuid, entry_id: Uuid, event_type: &str, entry: &MemoryEntry, actor: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO memory_events (event_id, workspace_id, chat_id, memory_entry_id, event_type, payload_json, actor, occurred_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(Uuid::new_v4().to_string()).bind(workspace_id.to_string()).bind(chat_id.to_string()).bind(entry_id.to_string()).bind(event_type)
        .bind(serde_json::to_string(entry).map_err(|e| sqlx::Error::Encode(Box::new(e)))?).bind(actor).bind(Utc::now().to_rfc3339())
        .execute(&mut **tx).await?;
    Ok(())
}

async fn bump_memory_version(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, chat_id: Uuid, now: DateTime<Utc>) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE chats SET memory_version = memory_version + 1, updated_at = ? WHERE id = ?")
        .bind(now.to_rfc3339()).bind(chat_id.to_string()).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM resume_brief_cache WHERE chat_id = ?").bind(chat_id.to_string()).execute(&mut **tx).await?;
    Ok(())
}

async fn insert_audit(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, workspace_id: Uuid, actor: &str, event_type: &str, entity_type: &str, entity_id: Uuid, metadata: serde_json::Value) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO audit_events (event_id, workspace_id, actor, event_type, entity_type, entity_id, occurred_at, metadata_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(Uuid::new_v4().to_string()).bind(workspace_id.to_string()).bind(actor).bind(event_type).bind(entity_type).bind(entity_id.to_string()).bind(Utc::now().to_rfc3339()).bind(metadata.to_string())
        .execute(&mut **tx).await?;
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{SqliteStorage, Storage};

    #[tokio::test]
    async fn resume_brief_uses_only_approved_memory_and_invalidates_after_mutation() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        let store = storage.chat_memory_store();
        let workspace_id = Uuid::new_v4();
        let chat = store.create_chat(workspace_id, "human:test", "Persistence design".into()).await.unwrap();
        let message = store.append_message(workspace_id, chat.id, "human:test", CreateMessage {
            role: "user".into(),
            content: "We need a durable way to continue this project.".into(),
            answered_by: None,
        }).await.unwrap().unwrap();

        let proposed = store.create_memory_entry(workspace_id, chat.id, "human:test", CreateMemoryEntry {
            entry_type: "decision".into(),
            text: "Use append-only memory events as the source of truth.".into(),
            reason: Some("Can be rebuilt after cache loss.".into()),
            source_message_id: Some(message.id),
        }).await.unwrap().unwrap();
        let before_approval = store.get_chat(workspace_id, chat.id).await.unwrap().unwrap();
        assert!(!before_approval.resume_brief.contains("append-only memory events"));

        store.mutate_memory_entry(workspace_id, chat.id, proposed.id, "human:test", "approve", None).await.unwrap().unwrap();
        let after_approval = store.get_chat(workspace_id, chat.id).await.unwrap().unwrap();
        assert!(after_approval.resume_brief.contains("Decisions:"));
        assert!(after_approval.resume_brief.contains("append-only memory events"));

        store.mutate_memory_entry(workspace_id, chat.id, proposed.id, "human:test", "edit", Some(EditMemoryEntry {
            text: "Use SQLite as the durable event store.".into(),
            reason: None,
        })).await.unwrap().unwrap();
        let after_edit = store.get_chat(workspace_id, chat.id).await.unwrap().unwrap();
        assert!(!after_edit.resume_brief.contains("append-only memory events"));
        assert!(!after_edit.resume_brief.contains("Use SQLite as the durable event store"));
        assert_eq!(after_edit.memory[0].status, "proposed");

        let event_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memory_events WHERE chat_id = ?")
            .bind(chat.id.to_string()).fetch_one(&store.pool).await.unwrap();
        assert_eq!(event_count, 3);

        let audit_payloads: Vec<String> = sqlx::query_scalar("SELECT metadata_json FROM audit_events WHERE workspace_id = ?")
            .bind(workspace_id.to_string()).fetch_all(&store.pool).await.unwrap();
        assert!(audit_payloads.iter().all(|payload| !payload.contains("durable way to continue")));
    }

    #[tokio::test]
    async fn chat_reads_are_workspace_scoped_and_archived_chats_disappear_from_list() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        let store = storage.chat_memory_store();
        let workspace_id = Uuid::new_v4();
        let other_workspace = Uuid::new_v4();
        let chat = store.create_chat(workspace_id, "human:test", "Private chat".into()).await.unwrap();

        assert!(store.get_chat(other_workspace, chat.id).await.unwrap().is_none());
        assert_eq!(store.list_chats(workspace_id).await.unwrap().len(), 1);
        assert!(store.archive_chat(workspace_id, chat.id, "human:test").await.unwrap());
        assert!(store.list_chats(workspace_id).await.unwrap().is_empty());
        assert!(store.get_chat(workspace_id, chat.id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn memory_event_log_rejects_updates_and_deletes() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        let store = storage.chat_memory_store();
        let workspace_id = Uuid::new_v4();
        let chat = store.create_chat(workspace_id, "human:test", "Append only".into()).await.unwrap();
        let entry = store.create_memory_entry(workspace_id, chat.id, "human:test", CreateMemoryEntry {
            entry_type: "fact".into(),
            text: "Events are immutable.".into(),
            reason: None,
            source_message_id: None,
        }).await.unwrap().unwrap();
        let update = sqlx::query("UPDATE memory_events SET actor = 'tampered' WHERE memory_entry_id = ?")
            .bind(entry.id.to_string()).execute(&store.pool).await;
        assert!(update.is_err());
        let delete = sqlx::query("DELETE FROM memory_events WHERE memory_entry_id = ?")
            .bind(entry.id.to_string()).execute(&store.pool).await;
        assert!(delete.is_err());
    }
}
