use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{BarChart, Block, Borders, List, ListItem, Paragraph, Sparkline, Widget},
};

use super::app::{Panel, WeeklySummary};
use crate::models::Workout;

/// Render weekly summary widget
pub fn render_weekly_summary(
    area: Rect,
    buf: &mut Buffer,
    summary: &WeeklySummary,
    is_selected: bool,
) {
    let border_style = if is_selected {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 📊 Weekly Summary ")
        .border_style(border_style);

    let inner = block.inner(area);
    block.render(area, buf);

    // Content lines
    let lines = vec![
        Line::from(vec![
            Span::styled("Total Workouts: ", Style::default().fg(Color::Gray)),
            Span::styled(
                summary.total_workouts.to_string(),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Total Distance: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:.1} km", summary.total_distance_km),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Total Duration: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{} min", summary.total_duration_min),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Weekly Activity:",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
    ];

    let paragraph = Paragraph::new(lines);
    paragraph.render(inner, buf);
}

/// Render weekly bar chart
pub fn render_weekly_chart(area: Rect, buf: &mut Buffer, summary: &WeeklySummary) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 📈 Weekly Workouts ")
        .border_style(Style::default().fg(Color::Gray));

    let inner = block.inner(area);
    block.render(area, buf);

    // Create bar chart data
    let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let data: Vec<(&str, u64)> = days
        .iter()
        .zip(summary.workouts_by_day.iter())
        .map(|(day, count)| (*day, *count as u64))
        .collect();

    let barchart = BarChart::default()
        .data(&data)
        .bar_width(5)
        .bar_gap(1)
        .bar_style(Style::default().fg(Color::Green))
        .value_style(Style::default().fg(Color::White).bg(Color::Green));

    barchart.render(inner, buf);
}

/// Render recent workouts list
pub fn render_recent_workouts(
    area: Rect,
    buf: &mut Buffer,
    workouts: &[Workout],
    selected_index: usize,
    is_selected: bool,
) {
    let border_style = if is_selected {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 🏃 Recent Workouts ")
        .border_style(border_style);

    let inner = block.inner(area);
    block.render(area, buf);

    if workouts.is_empty() {
        let empty_text = Paragraph::new("No workouts yet.\nPress 'L' to log your first workout!")
            .style(Style::default().fg(Color::Gray));
        empty_text.render(inner, buf);
        return;
    }

    let items: Vec<ListItem> = workouts
        .iter()
        .enumerate()
        .map(|(idx, workout)| {
            let sync_icon = if workout.synced { "✓" } else { "⏳" };

            let distance_str = workout
                .distance_km
                .map(|d| format!("{:.1}km", d))
                .unwrap_or_else(|| "-".to_string());

            let duration_str = workout
                .duration_minutes
                .map(|d| format!("{}min", d))
                .unwrap_or_else(|| "-".to_string());

            let date_str = workout.date.format("%m/%d").to_string();

            let line_style = if is_selected && idx == selected_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let content = format!(
                "{} {} {:>8} {:>8} {}",
                sync_icon,
                date_str,
                &workout.exercise_type[..workout.exercise_type.len().min(8)],
                distance_str,
                duration_str
            );

            ListItem::new(Line::from(Span::styled(content, line_style)))
        })
        .collect();

    let list = List::new(items);
    list.render(inner, buf);
}

/// Render goals panel
pub fn render_goals(area: Rect, buf: &mut Buffer, is_selected: bool) {
    let border_style = if is_selected {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 🎯 Goals ")
        .border_style(border_style);

    let inner = block.inner(area);
    block.render(area, buf);

    // Placeholder content
    let lines = vec![
        Line::from(Span::styled(
            "No active goals",
            Style::default().fg(Color::Gray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press 'G' to create goals",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(lines);
    paragraph.render(inner, buf);
}

/// Render quick actions panel
pub fn render_quick_actions(
    area: Rect,
    buf: &mut Buffer,
    selected_index: usize,
    is_selected: bool,
    sync_pending: usize,
) {
    let border_style = if is_selected {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ⚡ Quick Actions ")
        .border_style(border_style);

    let inner = block.inner(area);
    block.render(area, buf);

    let actions = vec![
        ("L", "Log Workout", Color::Green),
        ("G", "View Goals", Color::Yellow),
        ("S", "View Stats", Color::Cyan),
        (
            "Y",
            "Sync",
            if sync_pending > 0 {
                Color::Red
            } else {
                Color::Blue
            },
        ),
    ];

    let items: Vec<ListItem> = actions
        .iter()
        .enumerate()
        .map(|(idx, (key, desc, color))| {
            let line_style = if is_selected && idx == selected_index {
                Style::default()
                    .fg(*color)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(*color)
            };

            let content = if *key == "Y" && sync_pending > 0 {
                format!("  [{}] {} ({} pending)", key, desc, sync_pending)
            } else {
                format!("  [{}] {}", key, desc)
            };

            ListItem::new(Line::from(Span::styled(content, line_style)))
        })
        .collect();

    let list = List::new(items);
    list.render(inner, buf);
}

/// Render help overlay
pub fn render_help_overlay(area: Rect, buf: &mut Buffer) {
    // Create semi-transparent background
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ❓ Help ")
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    block.render(area, buf);

    let help_text = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Navigation:",
            Style::default().fg(Color::Cyan),
        )),
        Line::from("  ↑/k      - Move up"),
        Line::from("  ↓/j      - Move down"),
        Line::from("  ←/h      - Previous panel"),
        Line::from("  →/l      - Next panel"),
        Line::from("  Tab      - Next panel"),
        Line::from("  Shift+Tab - Previous panel"),
        Line::from(""),
        Line::from(Span::styled(
            "Quick Actions:",
            Style::default().fg(Color::Cyan),
        )),
        Line::from("  L        - Log workout"),
        Line::from("  G        - View goals"),
        Line::from("  S        - View stats"),
        Line::from("  Y        - Sync with server"),
        Line::from("  R        - Refresh data"),
        Line::from(""),
        Line::from(Span::styled("Other:", Style::default().fg(Color::Cyan))),
        Line::from("  ?        - Toggle this help"),
        Line::from("  q        - Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "Press ? or ESC to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(help_text);
    paragraph.render(inner, buf);
}

/// Render status bar at bottom
pub fn render_status_bar(area: Rect, buf: &mut Buffer, sync_pending: usize) {
    let sync_status = if sync_pending > 0 {
        Span::styled(
            format!(" ⏳ {} pending ", sync_pending),
            Style::default().fg(Color::Yellow).bg(Color::DarkGray),
        )
    } else {
        Span::styled(
            " ✓ Synced ",
            Style::default().fg(Color::Green).bg(Color::DarkGray),
        )
    };

    let help_hint = Span::styled(
        " Press ? for help ",
        Style::default().fg(Color::Gray).bg(Color::DarkGray),
    );

    let line = Line::from(vec![sync_status, help_hint]);
    let paragraph = Paragraph::new(line);
    paragraph.render(area, buf);
}
