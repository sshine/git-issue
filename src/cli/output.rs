use crate::common::{Issue, IssueStatus};
use console::{Color, Style, style};

pub fn format_issue_status(status: &IssueStatus) -> console::StyledObject<&str> {
    match status {
        IssueStatus::Todo => style("TODO").fg(Color::Yellow),
        IssueStatus::InProgress => style("IN PROGRESS").fg(Color::Blue),
        IssueStatus::Done => style("DONE").fg(Color::Green),
    }
}

pub fn format_issue_compact(issue: &Issue) -> String {
    format!(
        "#{} {} [{}]",
        style(issue.id).bold(),
        issue.title,
        format_issue_status(&issue.status)
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

    output.push_str(&format!(
        "Created by: {} ({}) on {}\n",
        style(&issue.created_by.name).green(),
        issue.created_by.email,
        issue.created_at.format("%Y-%m-%d %H:%M:%S")
    ));

    output.push_str(&format!(
        "Last updated: {}\n",
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
        output.push_str(&format!("{}\n", issue.description));
    }

    if !issue.comments.is_empty() {
        output.push_str("\nComments:\n");
        for comment in &issue.comments {
            output.push_str(&format!(
                "  {} by {} on {}:\n",
                style(&comment.id).dim(),
                style(&comment.author.name).green(),
                comment.created_at.format("%Y-%m-%d %H:%M")
            ));
            output.push_str(&format!("    {}\n", comment.content));
        }
    }

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
