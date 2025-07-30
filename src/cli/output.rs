use crate::common::{Issue, IssueStatus};
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
        output.push_str(&format!("{}\n", issue.description));
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
