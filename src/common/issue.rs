use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::common::{Comment, Identity, IssueEvent, Priority};

pub type IssueId = u64;
pub type CommentId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueStatus {
    Todo,
    InProgress,
    Done,
}

impl fmt::Display for IssueStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IssueStatus::Todo => write!(f, "todo"),
            IssueStatus::InProgress => write!(f, "in-progress"),
            IssueStatus::Done => write!(f, "done"),
        }
    }
}

impl std::str::FromStr for IssueStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "todo" => Ok(IssueStatus::Todo),
            "in-progress" | "inprogress" => Ok(IssueStatus::InProgress),
            "done" => Ok(IssueStatus::Done),
            _ => Err(anyhow::anyhow!("Invalid status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    pub id: IssueId,
    pub title: String,
    pub description: String,
    pub status: IssueStatus,
    pub priority: Priority,
    pub labels: Vec<String>,
    pub comments: Vec<Comment>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Identity,
    pub assignee: Option<Identity>,
}

impl Issue {
    pub fn new(id: IssueId, title: String, description: String, created_by: Identity) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            description,
            status: IssueStatus::Todo,
            priority: Priority::default(),
            labels: Vec::new(),
            comments: Vec::new(),
            created_at: now,
            updated_at: now,
            created_by,
            assignee: None,
        }
    }

    pub fn add_comment(&mut self, content: String, author: Identity) -> CommentId {
        let comment_id = format!("{}-{}", self.id, self.comments.len() + 1);
        let comment = Comment::new(comment_id.clone(), content, author);
        self.comments.push(comment);
        self.updated_at = Utc::now();
        comment_id
    }

    pub fn change_status(&mut self, new_status: IssueStatus) {
        if self.status != new_status {
            self.status = new_status;
            self.updated_at = Utc::now();
        }
    }

    pub fn add_label(&mut self, label: String) {
        if !self.labels.contains(&label) {
            self.labels.push(label);
            self.updated_at = Utc::now();
        }
    }

    pub fn remove_label(&mut self, label: &str) {
        if let Some(pos) = self.labels.iter().position(|l| l == label) {
            self.labels.remove(pos);
            self.updated_at = Utc::now();
        }
    }

    pub fn change_title(&mut self, new_title: String) {
        if self.title != new_title {
            self.title = new_title;
            self.updated_at = Utc::now();
        }
    }

    pub fn assign_to(&mut self, assignee: Option<Identity>) {
        if self.assignee != assignee {
            self.assignee = assignee;
            self.updated_at = Utc::now();
        }
    }

    pub fn change_description(&mut self, new_description: String) {
        if self.description != new_description {
            self.description = new_description;
            self.updated_at = Utc::now();
        }
    }

    pub fn change_priority(&mut self, new_priority: Priority) {
        if self.priority != new_priority {
            self.priority = new_priority;
            self.updated_at = Utc::now();
        }
    }
}

impl Issue {
    pub fn from_events(issue_id: IssueId, events: &[IssueEvent]) -> anyhow::Result<Self> {
        if events.is_empty() {
            return Err(anyhow::anyhow!("Cannot create issue from empty event list"));
        }

        let created_event = match &events[0] {
            IssueEvent::Created {
                title,
                description,
                author,
                timestamp,
            } => (
                title.clone(),
                description.clone(),
                author.clone(),
                *timestamp,
            ),
            _ => return Err(anyhow::anyhow!("First event must be Created")),
        };

        let mut issue = Issue {
            id: issue_id,
            title: created_event.0,
            description: created_event.1,
            status: IssueStatus::Todo,
            priority: Priority::default(),
            labels: Vec::new(),
            comments: Vec::new(),
            created_at: created_event.3,
            updated_at: created_event.3,
            created_by: created_event.2,
            assignee: None,
        };

        for event in events.iter().skip(1) {
            issue.apply_event(event)?;
        }

        Ok(issue)
    }

    pub fn apply_event(&mut self, event: &IssueEvent) -> anyhow::Result<()> {
        match event {
            IssueEvent::Created { .. } => {
                return Err(anyhow::anyhow!(
                    "Cannot apply Created event to existing issue"
                ));
            }
            IssueEvent::StatusChanged { to, timestamp, .. } => {
                self.status = *to;
                self.updated_at = *timestamp;
            }
            IssueEvent::CommentAdded {
                comment_id,
                content,
                author,
                timestamp,
            } => {
                let comment = Comment {
                    id: comment_id.clone(),
                    content: content.clone(),
                    author: author.clone(),
                    created_at: *timestamp,
                };
                self.comments.push(comment);
                self.updated_at = *timestamp;
            }
            IssueEvent::LabelAdded {
                label, timestamp, ..
            } => {
                if !self.labels.contains(label) {
                    self.labels.push(label.clone());
                }
                self.updated_at = *timestamp;
            }
            IssueEvent::LabelRemoved {
                label, timestamp, ..
            } => {
                self.labels.retain(|l| l != label);
                self.updated_at = *timestamp;
            }
            IssueEvent::TitleChanged {
                new_title,
                timestamp,
                ..
            } => {
                self.title = new_title.clone();
                self.updated_at = *timestamp;
            }
            IssueEvent::AssigneeChanged {
                new_assignee,
                timestamp,
                ..
            } => {
                self.assignee = new_assignee.clone();
                self.updated_at = *timestamp;
            }
            IssueEvent::DescriptionChanged {
                new_description,
                timestamp,
                ..
            } => {
                self.description = new_description.clone();
                self.updated_at = *timestamp;
            }
            IssueEvent::PriorityChanged {
                new_priority,
                timestamp,
                ..
            } => {
                self.priority = *new_priority;
                self.updated_at = *timestamp;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_identity() -> Identity {
        Identity::new("Test User".to_string(), "test@example.com".to_string())
    }

    #[test]
    fn test_identity_creation() {
        let identity = test_identity();
        assert_eq!(identity.name, "Test User");
        assert_eq!(identity.email, "test@example.com");
        assert_eq!(identity.to_string(), "Test User <test@example.com>");
    }

    #[test]
    fn test_issue_status_parsing() {
        assert_eq!("todo".parse::<IssueStatus>().unwrap(), IssueStatus::Todo);
        assert_eq!(
            "in-progress".parse::<IssueStatus>().unwrap(),
            IssueStatus::InProgress
        );
        assert_eq!(
            "inprogress".parse::<IssueStatus>().unwrap(),
            IssueStatus::InProgress
        );
        assert_eq!("done".parse::<IssueStatus>().unwrap(), IssueStatus::Done);
        assert!("invalid".parse::<IssueStatus>().is_err());
    }

    #[test]
    fn test_issue_creation() {
        let author = test_identity();
        let issue = Issue::new(
            1,
            "Test Issue".to_string(),
            "Test Description".to_string(),
            author.clone(),
        );

        assert_eq!(issue.id, 1);
        assert_eq!(issue.title, "Test Issue");
        assert_eq!(issue.description, "Test Description");
        assert_eq!(issue.status, IssueStatus::Todo);
        assert_eq!(issue.created_by, author);
        assert!(issue.assignee.is_none());
        assert!(issue.labels.is_empty());
        assert!(issue.comments.is_empty());
    }

    #[test]
    fn test_issue_mutations() {
        let author = test_identity();
        let mut issue = Issue::new(
            1,
            "Test Issue".to_string(),
            "Test Description".to_string(),
            author.clone(),
        );

        let original_updated_at = issue.updated_at;

        // Add a small delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(1));

        // Test status change
        issue.change_status(IssueStatus::InProgress);
        assert_eq!(issue.status, IssueStatus::InProgress);
        assert!(issue.updated_at > original_updated_at);

        // Test adding comment
        let comment_id = issue.add_comment("Test comment".to_string(), author.clone());
        assert_eq!(issue.comments.len(), 1);
        assert_eq!(issue.comments[0].content, "Test comment");
        assert_eq!(comment_id, "1-1");

        // Test adding label
        issue.add_label("bug".to_string());
        assert!(issue.labels.contains(&"bug".to_string()));

        // Test removing label
        issue.remove_label("bug");
        assert!(!issue.labels.contains(&"bug".to_string()));

        // Test title change
        issue.change_title("New Title".to_string());
        assert_eq!(issue.title, "New Title");

        // Test assignee
        let assignee = Identity::new("Assignee".to_string(), "assignee@example.com".to_string());
        issue.assign_to(Some(assignee.clone()));
        assert_eq!(issue.assignee, Some(assignee));
    }

    #[test]
    fn test_event_creation() {
        let author = test_identity();

        let created_event = IssueEvent::created(
            "Test".to_string(),
            "Description".to_string(),
            author.clone(),
        );

        assert!(matches!(created_event, IssueEvent::Created { .. }));
        assert_eq!(created_event.author(), &author);

        let status_event =
            IssueEvent::status_changed(IssueStatus::Todo, IssueStatus::Done, author.clone());
        assert!(matches!(status_event, IssueEvent::StatusChanged { .. }));
    }

    #[test]
    fn test_issue_from_events() {
        let author = test_identity();
        let issue_id = 1;

        let events = vec![
            IssueEvent::created(
                "Test Issue".to_string(),
                "Description".to_string(),
                author.clone(),
            ),
            IssueEvent::status_changed(IssueStatus::Todo, IssueStatus::InProgress, author.clone()),
            IssueEvent::comment_added(
                "1-1".to_string(),
                "First comment".to_string(),
                author.clone(),
            ),
            IssueEvent::label_added("bug".to_string(), author.clone()),
        ];

        let issue = Issue::from_events(issue_id, &events).unwrap();

        assert_eq!(issue.id, issue_id);
        assert_eq!(issue.title, "Test Issue");
        assert_eq!(issue.status, IssueStatus::InProgress);
        assert_eq!(issue.comments.len(), 1);
        assert_eq!(issue.labels.len(), 1);
        assert!(issue.labels.contains(&"bug".to_string()));
    }

    #[test]
    fn test_issue_from_empty_events() {
        let result = Issue::from_events(1, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_issue_from_invalid_first_event() {
        let author = test_identity();
        let events = vec![IssueEvent::status_changed(
            IssueStatus::Todo,
            IssueStatus::Done,
            author,
        )];

        let result = Issue::from_events(1, &events);
        assert!(result.is_err());
    }

    #[test]
    fn test_serialization() {
        let author = test_identity();
        let issue = Issue::new(
            1,
            "Test Issue".to_string(),
            "Test Description".to_string(),
            author,
        );

        // Test that serialization and deserialization work
        let json = serde_json::to_string(&issue).unwrap();
        let deserialized: Issue = serde_json::from_str(&json).unwrap();

        assert_eq!(issue, deserialized);
    }
}
