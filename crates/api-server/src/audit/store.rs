use std::path::{Path, PathBuf};

use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::RwLock;
use tracing::warn;

use super::{AuditEvent, AuditListQuery};

pub struct AuditStore {
    events_path: PathBuf,
    events: RwLock<Vec<AuditEvent>>,
}

fn trim_to_none(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn matches_action_filter(action: &str, filter: Option<&str>) -> bool {
    let Some(filter) = filter else {
        return true;
    };
    action.to_lowercase().contains(filter)
}

impl AuditStore {
    pub async fn new(root_dir: PathBuf) -> std::io::Result<Self> {
        fs::create_dir_all(&root_dir).await?;
        let events_path = root_dir.join("events.jsonl");

        if fs::metadata(&events_path).await.is_err() {
            fs::File::create(&events_path).await?;
        }

        let events = Self::load_events(&events_path).await?;
        Ok(Self {
            events_path,
            events: RwLock::new(events),
        })
    }

    async fn load_events(path: &Path) -> std::io::Result<Vec<AuditEvent>> {
        let file = fs::File::open(path).await?;
        let mut reader = BufReader::new(file).lines();
        let mut events = Vec::new();

        while let Some(line) = reader.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<AuditEvent>(&line) {
                Ok(event) => events.push(event),
                Err(err) => warn!(
                    "Ignoring malformed audit event in {}: {}",
                    path.display(),
                    err
                ),
            }
        }

        Ok(events)
    }

    pub async fn append(&self, event: AuditEvent) -> Result<(), String> {
        let encoded = serde_json::to_string(&event)
            .map_err(|err| format!("Failed to encode audit event: {}", err))?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.events_path)
            .await
            .map_err(|err| format!("Failed to open audit log: {}", err))?;

        file.write_all(encoded.as_bytes())
            .await
            .map_err(|err| format!("Failed to write audit log: {}", err))?;
        file.write_all(b"\n")
            .await
            .map_err(|err| format!("Failed to finalize audit log line: {}", err))?;
        file.flush()
            .await
            .map_err(|err| format!("Failed to flush audit log: {}", err))?;

        let mut state = self.events.write().await;
        state.push(event);
        Ok(())
    }

    pub async fn list_paginated(&self, query: &AuditListQuery) -> (Vec<AuditEvent>, bool) {
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(100).clamp(1, 1000);
        let org_filter = trim_to_none(query.org_id.clone());
        let action_filter = trim_to_none(query.action.clone()).map(|value| value.to_lowercase());

        let state = self.events.read().await;
        let mut matched = 0usize;
        let mut events = Vec::with_capacity(limit);

        for event in state.iter().rev() {
            if let Some(org_id) = org_filter.as_deref() {
                if event.org_id != org_id {
                    continue;
                }
            }

            if !matches_action_filter(&event.action, action_filter.as_deref()) {
                continue;
            }

            if let Some(execution_id) = query.execution_id {
                if event.execution_id != Some(execution_id) {
                    continue;
                }
            }

            if let Some(task_id) = query.task_id {
                if event.task_id != Some(task_id) {
                    continue;
                }
            }

            if matched < offset {
                matched += 1;
                continue;
            }

            if events.len() < limit {
                events.push(event.clone());
            }
            matched += 1;
        }

        let has_more = matched > offset + events.len();
        (events, has_more)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn append_and_list_returns_latest_first() {
        let temp_dir = TempDir::new().unwrap();
        let store = AuditStore::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        let first = AuditEvent::new(
            "org-1".to_string(),
            "system",
            "execution.start",
            None,
            None,
            None,
            None,
            Some("accepted".to_string()),
            serde_json::Value::Null,
        );
        let second = AuditEvent::new(
            "org-1".to_string(),
            "system",
            "execution.stop",
            None,
            None,
            None,
            None,
            Some("cancelled".to_string()),
            serde_json::Value::Null,
        );

        store.append(first.clone()).await.unwrap();
        store.append(second.clone()).await.unwrap();

        let (events, has_more) = store.list_paginated(&AuditListQuery::default()).await;
        assert!(!has_more);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].action, second.action);
        assert_eq!(events[1].action, first.action);
    }
}
