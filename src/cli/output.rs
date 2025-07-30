use crate::common::{Issue, IssueStatus, Priority};
use chrono::Utc;
use console::{Color, style};
use std::time::Duration;

fn format_time_ago(duration: Duration) -> String {
    let total_seconds = duration.as_secs();

    if total_seconds < 60 {
        return format!(
            "{} second{}",
            total_seconds,
            if total_seconds == 1 { "" } else { "s" }
        );
    }

    let minutes = total_seconds / 60;
    if minutes < 60 {
        return format!("{} minute{}", minutes, if minutes == 1 { "" } else { "s" });
    }

    let hours = minutes / 60;
    if hours < 24 {
        return format!("{} hour{}", hours, if hours == 1 { "" } else { "s" });
    }

    let days = hours / 24;
    if days < 30 {
        return format!("{} day{}", days, if days == 1 { "" } else { "s" });
    }

    let months = days / 30;
    if months < 12 {
        return format!("{} month{}", months, if months == 1 { "" } else { "s" });
    }

    let years = months / 12;
    format!("{} year{}", years, if years == 1 { "" } else { "s" })
}

fn truncate_to_first_paragraph(text: &str) -> (String, Option<usize>) {
    if text.is_empty() {
        return (String::new(), None);
    }

    // Split by double newlines to find paragraphs
    let paragraphs: Vec<&str> = text.split("\n\n").collect();

    if paragraphs.len() <= 1 {
        // No paragraph break found, return original text
        return (text.to_string(), None);
    }

    let first_paragraph = paragraphs[0].trim();
    let remaining_text = paragraphs[1..].join("\n\n");
    let remaining_words = remaining_text.split_whitespace().count();

    if remaining_words > 0 {
        (first_paragraph.to_string(), Some(remaining_words))
    } else {
        (first_paragraph.to_string(), None)
    }
}

pub fn format_issue_status(status: &IssueStatus) -> console::StyledObject<&str> {
    match status {
        IssueStatus::Todo => style("TODO").fg(Color::Yellow),
        IssueStatus::InProgress => style("IN PROGRESS").fg(Color::Blue),
        IssueStatus::Done => style("DONE").fg(Color::Green),
    }
}

pub fn format_priority(priority: &Priority) -> console::StyledObject<String> {
    let priority_str = match priority {
        Priority::None => "—".to_string(), // Em dash for no priority
        Priority::Urgent => "URGENT".to_string(),
        Priority::High => "HIGH".to_string(),
        Priority::Medium => "MED".to_string(),
        Priority::Low => "LOW".to_string(),
    };

    match priority {
        Priority::None => style(priority_str).dim(),
        Priority::Urgent => style(priority_str).fg(Color::Red).bold(),
        Priority::High => style(priority_str).fg(Color::Red),
        Priority::Medium => style(priority_str).fg(Color::Yellow),
        Priority::Low => style(priority_str).fg(Color::Green),
    }
}

pub fn format_issue_compact(issue: &Issue) -> String {
    let priority_part = if issue.priority == Priority::None {
        String::new()
    } else {
        format!(" [{}]", format_priority(&issue.priority))
    };

    format!(
        "#{} [{}]{} {}",
        style(issue.id).bold(),
        format_issue_status(&issue.status),
        priority_part,
        issue.title
    )
}

pub fn format_issue_detailed(issue: &Issue) -> String {
    let mut output = String::new();

    output.push_str(&format!(
        "Issue {}: {}\n",
        style(format!("#{}", issue.id)).bold().cyan(),
        style(&issue.title).bold()
    ));

    output.push_str(&format!("Status: {}\n", format_issue_status(&issue.status)));
    output.push_str(&format!("Priority: {}\n", format_priority(&issue.priority)));

    let created_time_since = Utc::now() - issue.created_at;
    output.push_str(&format!(
        "Created by: {} ({}), {} ago ({})\n",
        style(&issue.created_by.name).green(),
        issue.created_by.email,
        format_time_ago(created_time_since.to_std().unwrap_or_default()),
        issue.created_at.format("%Y-%m-%d %H:%M:%S")
    ));

    let updated_time_since = Utc::now() - issue.updated_at;
    output.push_str(&format!(
        "Last updated: {} ago ({})\n",
        format_time_ago(updated_time_since.to_std().unwrap_or_default()),
        issue.updated_at.format("%Y-%m-%d %H:%M:%S")
    ));

    if let Some(ref assignee) = issue.assignee {
        output.push_str(&format!(
            "Assigned to: {} ({})\n",
            style(&assignee.name).green(),
            assignee.email
        ));
    }

    if !issue.labels.is_empty() {
        output.push_str(&format!(
            "Labels: {}\n",
            issue
                .labels
                .iter()
                .map(|l| style(l).magenta().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    if !issue.description.is_empty() {
        output.push_str("\nDescription:\n");
        let (truncated_desc, remaining_words) = truncate_to_first_paragraph(&issue.description);
        output.push_str(&format!("{}\n", truncated_desc));

        if let Some(word_count) = remaining_words {
            output.push_str(&format!(
                "{}\n",
                style(format!(
                    "[{} more words; run `git issue show #{}`]",
                    word_count, issue.id
                ))
                .dim()
            ));
        }
    }

    if !issue.comments.is_empty() {
        output.push_str("\nComments:\n");
        for comment in &issue.comments {
            let time_since = Utc::now() - comment.created_at;
            output.push_str(&format!(
                "  {} by {}, {} ago ({}):\n",
                style(&comment.id).dim(),
                style(&comment.author.name).green(),
                format_time_ago(time_since.to_std().unwrap_or_default()),
                comment.created_at.format("%Y-%m-%d %H:%M")
            ));
            output.push_str(&format!("    {}\n", comment.content));
        }
    }

    output.push_str("\n");

    output
}

pub fn success_message(message: &str) -> String {
    format!("{} {}", style("✓").green().bold(), message)
}

pub fn error_message(message: &str) -> String {
    format!("{} {}", style("✗").red().bold(), message)
}

pub fn warning_message(message: &str) -> String {
    format!("{} {}", style("⚠").yellow().bold(), message)
}

pub fn info_message(message: &str) -> String {
    format!("{} {}", style("ℹ").blue().bold(), message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{Identity, Issue, IssueStatus, Priority};
    use chrono::Utc;

    fn create_test_issue() -> Issue {
        let author = Identity::new("Test Author".to_string(), "test@example.com".to_string());
        Issue {
            id: 42,
            title: "Test Issue Title".to_string(),
            description: "Single paragraph description".to_string(),
            status: IssueStatus::Todo,
            priority: Priority::None,
            created_by: author.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            assignee: None,
            labels: vec!["test".to_string(), "formatting".to_string()],
            comments: vec![],
        }
    }

    #[test]
    fn test_format_issue_compact() {
        let issue = create_test_issue();
        let formatted = format_issue_compact(&issue);

        // Should contain the ID, status in brackets, and title
        assert!(formatted.contains("#42"));
        assert!(formatted.contains("[TODO]"));
        assert!(formatted.contains("Test Issue Title"));

        // Status should come before title
        let status_pos = formatted.find("[TODO]").unwrap();
        let title_pos = formatted.find("Test Issue Title").unwrap();
        assert!(
            status_pos < title_pos,
            "Status should come before title in compact format"
        );
    }

    #[test]
    fn test_truncate_to_first_paragraph_single() {
        let text = "This is a single paragraph with no breaks.";
        let (truncated, remaining) = truncate_to_first_paragraph(text);

        assert_eq!(truncated, text);
        assert_eq!(remaining, None);
    }

    #[test]
    fn test_truncate_to_first_paragraph_multiple() {
        let text =
            "First paragraph here.\n\nSecond paragraph with more content.\n\nThird paragraph too.";
        let (truncated, remaining) = truncate_to_first_paragraph(text);

        assert_eq!(truncated, "First paragraph here.");
        assert!(remaining.is_some());
        let word_count = remaining.unwrap();
        assert!(word_count > 0);
    }

    #[test]
    fn test_truncate_to_first_paragraph_empty() {
        let text = "";
        let (truncated, remaining) = truncate_to_first_paragraph(text);

        assert_eq!(truncated, "");
        assert_eq!(remaining, None);
    }

    #[test]
    fn test_format_issue_detailed_with_multi_paragraph() {
        let mut issue = create_test_issue();
        issue.description =
            "First paragraph here.\n\nSecond paragraph with additional information.".to_string();

        let formatted = format_issue_detailed(&issue);

        // Should contain the first paragraph
        assert!(formatted.contains("First paragraph here."));

        // Should not contain the second paragraph in full
        assert!(!formatted.contains("Second paragraph with additional information."));

        // Should contain the "more words" hint
        assert!(formatted.contains("more words"));
        assert!(formatted.contains("git issue show #42"));
    }

    #[test]
    fn test_format_issue_detailed_single_paragraph() {
        let issue = create_test_issue();
        let formatted = format_issue_detailed(&issue);

        // Should contain the full description
        assert!(formatted.contains("Single paragraph description"));

        // Should not contain the "more words" hint
        assert!(!formatted.contains("more words"));
    }

    #[test]
    fn test_format_priority_none() {
        let priority = Priority::None;
        let formatted = format_priority(&priority);

        // Should display em dash for no priority
        assert_eq!(formatted.to_string(), "—");
    }

    #[test]
    fn test_format_priority_urgent() {
        let priority = Priority::Urgent;
        let formatted = format_priority(&priority);

        // Should display URGENT
        assert_eq!(formatted.to_string(), "URGENT");
    }

    #[test]
    fn test_format_issue_compact_with_priority() {
        let mut issue = create_test_issue();
        issue.priority = Priority::High;
        let formatted = format_issue_compact(&issue);

        // Should contain the ID, status, priority, and title
        assert!(formatted.contains("#42"));
        assert!(formatted.contains("[TODO]"));
        assert!(formatted.contains("[HIGH]"));
        assert!(formatted.contains("Test Issue Title"));

        // Status should come before priority, priority before title
        let status_pos = formatted.find("[TODO]").unwrap();
        let priority_pos = formatted.find("[HIGH]").unwrap();
        let title_pos = formatted.find("Test Issue Title").unwrap();
        assert!(status_pos < priority_pos);
        assert!(priority_pos < title_pos);
    }

    #[test]
    fn test_format_issue_compact_no_priority() {
        let issue = create_test_issue(); // Priority::None by default
        let formatted = format_issue_compact(&issue);

        // Should contain the ID, status, and title
        assert!(formatted.contains("#42"));
        assert!(formatted.contains("[TODO]"));
        assert!(formatted.contains("Test Issue Title"));

        // Should not contain priority brackets when priority is None
        assert!(!formatted.contains("[—]"));
        assert!(!formatted.contains("[NONE]"));

        // Should not have double spaces between status and title
        assert!(!formatted.contains("]  "));
    }

    #[test]
    fn test_format_issue_detailed_with_priority() {
        let mut issue = create_test_issue();
        issue.priority = Priority::Medium;
        let formatted = format_issue_detailed(&issue);

        // Should contain priority information
        assert!(formatted.contains("Priority: MED"));
        assert!(formatted.contains("Status: TODO"));
    }

    #[test]
    fn test_all_priority_levels_format() {
        let priorities = [
            (Priority::None, "—"),
            (Priority::Urgent, "URGENT"),
            (Priority::High, "HIGH"),
            (Priority::Medium, "MED"),
            (Priority::Low, "LOW"),
        ];

        for (priority, expected) in priorities {
            let formatted = format_priority(&priority);
            assert_eq!(
                formatted.to_string(),
                expected,
                "Priority {:?} should format as {}",
                priority,
                expected
            );
        }
    }
}
