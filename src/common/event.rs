use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::{CommentId, Identity, IssueStatus, Priority};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueEvent {
    Created {
        title: String,
        description: String,
        author: Identity,
        timestamp: DateTime<Utc>,
    },
    StatusChanged {
        from: IssueStatus,
        to: IssueStatus,
        author: Identity,
        timestamp: DateTime<Utc>,
    },
    CommentAdded {
        comment_id: CommentId,
        content: String,
        author: Identity,
        timestamp: DateTime<Utc>,
    },
    LabelAdded {
        label: String,
        author: Identity,
        timestamp: DateTime<Utc>,
    },
    LabelRemoved {
        label: String,
        author: Identity,
        timestamp: DateTime<Utc>,
    },
    TitleChanged {
        old_title: String,
        new_title: String,
        author: Identity,
        timestamp: DateTime<Utc>,
    },
    AssigneeChanged {
        old_assignee: Option<Identity>,
        new_assignee: Option<Identity>,
        author: Identity,
        timestamp: DateTime<Utc>,
    },
    AssigneesChanged {
        old_assignees: Vec<Identity>,
        new_assignees: Vec<Identity>,
        author: Identity,
        timestamp: DateTime<Utc>,
    },
    DescriptionChanged {
        old_description: String,
        new_description: String,
        author: Identity,
        timestamp: DateTime<Utc>,
    },
    PriorityChanged {
        old_priority: Priority,
        new_priority: Priority,
        author: Identity,
        timestamp: DateTime<Utc>,
    },
    CreatedByChanged {
        old_created_by: Identity,
        new_created_by: Identity,
        author: Identity,
        timestamp: DateTime<Utc>,
    },
}

impl IssueEvent {
    pub fn created(title: String, description: String, author: Identity) -> Self {
        IssueEvent::Created {
            title,
            description,
            author,
            timestamp: Utc::now(),
        }
    }

    pub fn status_changed(from: IssueStatus, to: IssueStatus, author: Identity) -> Self {
        IssueEvent::StatusChanged {
            from,
            to,
            author,
            timestamp: Utc::now(),
        }
    }

    // FIXME(sshine): Resolve issue #3 to remove this #[allow(unused)].
    #[allow(unused)]
    pub fn comment_added(comment_id: CommentId, content: String, author: Identity) -> Self {
        IssueEvent::CommentAdded {
            comment_id,
            content,
            author,
            timestamp: Utc::now(),
        }
    }

    pub fn label_added(label: String, author: Identity) -> Self {
        IssueEvent::LabelAdded {
            label,
            author,
            timestamp: Utc::now(),
        }
    }

    pub fn label_removed(label: String, author: Identity) -> Self {
        IssueEvent::LabelRemoved {
            label,
            author,
            timestamp: Utc::now(),
        }
    }

    pub fn title_changed(old_title: String, new_title: String, author: Identity) -> Self {
        IssueEvent::TitleChanged {
            old_title,
            new_title,
            author,
            timestamp: Utc::now(),
        }
    }

    pub fn assignee_changed(
        old_assignee: Option<Identity>,
        new_assignee: Option<Identity>,
        author: Identity,
    ) -> Self {
        IssueEvent::AssigneeChanged {
            old_assignee,
            new_assignee,
            author,
            timestamp: Utc::now(),
        }
    }

    pub fn assignees_changed(
        old_assignees: Vec<Identity>,
        new_assignees: Vec<Identity>,
        author: Identity,
    ) -> Self {
        IssueEvent::AssigneesChanged {
            old_assignees,
            new_assignees,
            author,
            timestamp: Utc::now(),
        }
    }

    pub fn description_changed(
        old_description: String,
        new_description: String,
        author: Identity,
    ) -> Self {
        IssueEvent::DescriptionChanged {
            old_description,
            new_description,
            author,
            timestamp: Utc::now(),
        }
    }

    pub fn priority_changed(
        old_priority: Priority,
        new_priority: Priority,
        author: Identity,
    ) -> Self {
        IssueEvent::PriorityChanged {
            old_priority,
            new_priority,
            author,
            timestamp: Utc::now(),
        }
    }

    pub fn created_by_changed(
        old_created_by: Identity,
        new_created_by: Identity,
        author: Identity,
    ) -> Self {
        IssueEvent::CreatedByChanged {
            old_created_by,
            new_created_by,
            author,
            timestamp: Utc::now(),
        }
    }

    pub fn author(&self) -> &Identity {
        match self {
            IssueEvent::Created { author, .. } => author,
            IssueEvent::StatusChanged { author, .. } => author,
            IssueEvent::CommentAdded { author, .. } => author,
            IssueEvent::LabelAdded { author, .. } => author,
            IssueEvent::LabelRemoved { author, .. } => author,
            IssueEvent::TitleChanged { author, .. } => author,
            IssueEvent::AssigneeChanged { author, .. } => author,
            IssueEvent::AssigneesChanged { author, .. } => author,
            IssueEvent::DescriptionChanged { author, .. } => author,
            IssueEvent::PriorityChanged { author, .. } => author,
            IssueEvent::CreatedByChanged { author, .. } => author,
        }
    }
}
