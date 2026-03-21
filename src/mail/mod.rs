use anyhow::Result;
use notmuch::Database;
use std::path::PathBuf;

pub mod preview;

#[derive(Debug, Clone)]
pub struct EmailMetadata {
    pub message_id: String,
    pub subject: String,
    pub from: String,
    pub to: String,
    pub date: i64,
    pub folder: String,
    pub path: PathBuf,
}

pub struct Searcher {
    db_path: Option<PathBuf>,
}

impl Searcher {
    pub fn new(db_path: Option<PathBuf>) -> Self {
        Self { db_path }
    }

    pub fn search(&self, query_string: &str, limit: usize) -> Result<Vec<EmailMetadata>> {
        let actual_query = if query_string.trim().is_empty() {
            String::from("*")
        } else {
            query_string.to_lowercase()
        };

        let db = Database::open_with_config(
            self.db_path.as_ref(),
            notmuch::DatabaseMode::ReadOnly,
            None::<&str>,
            None::<&str>,
        )?;
        let query = db.create_query(&actual_query)?;

        let messages = query.search_messages()?;

        let mut results = Vec::new();
        for message in messages.take(limit) {
            let message_id = message.id().to_string();
            let subject = message
                .header("subject")
                .unwrap_or_default()
                .unwrap_or_default()
                .to_string();
            let from = message
                .header("from")
                .unwrap_or_default()
                .unwrap_or_default()
                .to_string();
            let to = message
                .header("to")
                .unwrap_or_default()
                .unwrap_or_default()
                .to_string();
            let date = message.date();
            let path = message.filename().to_path_buf();

            // Extract folder from tags or from the path.
            let folder = path
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.file_name())
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();

            results.push(EmailMetadata {
                message_id,
                subject,
                from,
                to,
                date,
                folder,
                path,
            });
        }

        Ok(results)
    }
}
