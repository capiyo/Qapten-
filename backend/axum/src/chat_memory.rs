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
        append_memory_event(&mut tx, workspace_id, chat_id, entry.id, "created", &entry, actor).await?;
        bump_memory_version(&mut tx, chat_id, now).await?;
        insert_audit(&mut tx, workspace_id, actor, "memory.created", "memory_entry", entry.id, json!({"chat_id": chat_id, "type": entry.entry_type})).await?;
        tx.commit().await?;
        Ok(Some(entry))
    }

    pub async fn mutate_memory_entry(&self, workspace_id: Uuid, chat_id: Uuid, entry_id: Uuid, actor: &str, operation: &str, edit: Option<EditMemoryEntry>) -> Result<Option<MemoryEntry>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let mut current = replay_memory(&mut tx, chat_id).await?.remove(&entry_id);
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
        let mut entries = replay_memory(&mut connection).await?.into_values().filter(|entry| entry.status != "deleted").collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.created_at);
        Ok(entries)
    }

    async fn regenerate_brief(&self, chat_id: Uuid, workspace_id: Uuid, memory_version: i64, last_message_id: Option<String>, memory: &[MemoryEntry], messages: &[ChatMessage]) -> Result<String, sqlx::Error> {
        let approved = memory.iter().filter(|entry| entry.status == "approved").map(|entry| format!("- {}: {}", entry.entry_type, entry.text)).collect::<Vec<_>>();
        let recent = messages.iter().rev().take(6).collect::<Vec<_>>();
        let mut lines = Vec::new();
        if !approved.is_empty() { lines.push(format!("Approved project memory: {}", approved.join(" | "))); }
        if !recent.is_empty() { lines.push(format!("Recent conversation: {}", recent.iter().rev().map(|m| format!("{}: {}", m.role, m.content)).collect::<Vec<_>>().join(" | "))); }
        let brief = truncate_words(&lines.join("\n"), 190);
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
