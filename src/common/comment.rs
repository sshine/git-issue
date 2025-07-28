use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::{CommentId, Identity};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    pub id: CommentId,
    pub content: String,
    pub author: Identity,
    pub created_at: DateTime<Utc>,
}

impl Comment {
    pub fn new(id: CommentId, content: String, author: Identity) -> Self {
        Self {
            id,
            content,
            author,
            created_at: Utc::now(),
        }
    }
}
