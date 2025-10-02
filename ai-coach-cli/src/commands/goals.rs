use anyhow::{Context, Result};
use chrono::Utc;
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};
use dialoguer::{Confirm, Input, Select};

use crate::models::{Goal, GoalType};
use crate::storage::Storage;

pub async fn list_goals(show_all: bool) -> Result<()> {
    let storage = Storage::init().context("Failed to initialize storage")?;
    let goals = storage
        .list_goals(show_all)
        .context("Failed to list goals")?;

    if goals.is_empty() {
        if show_all {
            println!("📋 No goals found");
        } else {
            println!("📋 No active goals found");
            println!("\n💡 Use 'ai-coach goals list --all' to see completed goals");
        }
        return Ok(());
    }

    println!(
        "📋 Goals {}",
        if show_all { "(All)" } else { "(Active)" }
    );
    println!();

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec![
        Cell::new("ID").fg(Color::Cyan),
        Cell::new("Title").fg(Color::Cyan),
        Cell::new("Type").fg(Color::Cyan),
        Cell::new("Target Date").fg(Color::Cyan),
        Cell::new("Progress").fg(Color::Cyan),
        Cell::new("Days Left").fg(Color::Cyan),
        Cell::new("Status").fg(Color::Cyan),
    ]);

    for goal in goals {
        let id_short = &goal.id[..8];
        let progress = format!("{:.1}%", goal.progress_percentage());
        let days_left = if goal.completed {
            "—".to_string()
        } else {
            let days = goal.days_remaining();
            if days < 0 {
                format!("{}d overdue", -days)
            } else {
                format!("{}d", days)
            }
        };

        let status = if goal.completed {
            Cell::new("✅ Done").fg(Color::Green)
        } else if goal.days_remaining() < 0 {
            Cell::new("⚠️ Overdue").fg(Color::Red)
        } else {
            Cell::new("⏳ Active").fg(Color::Yellow)
        };

        let progress_cell = if goal.completed {
            Cell::new("100.0%").fg(Color::Green)
        } else {
            Cell::new(&progress)
        };

        table.add_row(vec![
            Cell::new(id_short),
            Cell::new(&goal.title),
            Cell::new(goal.goal_type.to_string()),
            Cell::new(goal.target_date.format("%Y-%m-%d").to_string()),
            progress_cell,
            Cell::new(days_left),
            status,
        ]);
    }

    println!("{table}");
    Ok(())
}

pub async fn create_goal() -> Result<()> {
    println!("📝 Create New Goal");
    println!();

    // Goal title
    let title: String = Input::new()
        .with_prompt("Goal title")
        .interact_text()
        .context("Failed to read title")?;

    // Goal type
    let type_options = vec!["Distance", "Duration", "Event", "Frequency"];
    let type_selection = Select::new()
        .with_prompt("Goal type")
        .items(&type_options)
        .default(0)
        .interact()
        .context("Failed to select goal type")?;

    let goal_type = match type_selection {
        0 => GoalType::Distance,
        1 => GoalType::Duration,
        2 => GoalType::Event,
        3 => GoalType::Frequency,
        _ => unreachable!(),
    };

    // Target date
    let target_date_str: String = Input::new()
        .with_prompt("Target date (YYYY-MM-DD)")
        .interact_text()
        .context("Failed to read target date")?;

    let target_date =
        chrono::NaiveDate::parse_from_str(&target_date_str, "%Y-%m-%d")
            .context("Invalid date format. Use YYYY-MM-DD")?
            .and_hms_opt(0, 0, 0)
            .context("Failed to create datetime")?
            .and_local_timezone(Utc)
            .single()
            .context("Failed to convert to UTC")?;

    // Target value (optional for Event type)
    let target_value = if goal_type == GoalType::Event {
        None
    } else {
        let prompt = match goal_type {
            GoalType::Distance => "Target distance (km)",
            GoalType::Duration => "Target duration (minutes)",
            GoalType::Frequency => "Target number of workouts",
            _ => "Target value",
        };

        let value: f64 = Input::new()
            .with_prompt(prompt)
            .interact_text()
            .context("Failed to read target value")?;

        Some(value)
    };

    // Notes (optional)
    let notes_input: String = Input::new()
        .with_prompt("Notes (optional)")
        .allow_empty(true)
        .interact_text()
        .context("Failed to read notes")?;

    let notes = if notes_input.is_empty() {
        None
    } else {
        Some(notes_input)
    };

    // Create and save goal
    let goal = Goal::new(title, goal_type, target_date, target_value, notes);

    let storage = Storage::init().context("Failed to initialize storage")?;
    storage.save_goal(&goal).context("Failed to save goal")?;

    println!();
    println!("✅ Goal created successfully!");
    println!("   ID: {}", goal.id);
    println!("   Title: {}", goal.title);
    println!("   Target Date: {}", goal.target_date.format("%Y-%m-%d"));

    Ok(())
}

pub async fn update_goal(id: &str) -> Result<()> {
    let storage = Storage::init().context("Failed to initialize storage")?;

    let mut goal = storage
        .get_goal(id)
        .context("Failed to get goal")?
        .ok_or_else(|| anyhow::anyhow!("Goal not found: {}", id))?;

    println!("📝 Update Goal: {}", goal.title);
    println!();

    // Update title
    let new_title: String = Input::new()
        .with_prompt("Title")
        .default(goal.title.clone())
        .interact_text()
        .context("Failed to read title")?;

    // Update target date
    let current_date = goal.target_date.format("%Y-%m-%d").to_string();
    let new_target_date_str: String = Input::new()
        .with_prompt("Target date (YYYY-MM-DD)")
        .default(current_date)
        .interact_text()
        .context("Failed to read target date")?;

    let new_target_date =
        chrono::NaiveDate::parse_from_str(&new_target_date_str, "%Y-%m-%d")
            .context("Invalid date format. Use YYYY-MM-DD")?
            .and_hms_opt(0, 0, 0)
            .context("Failed to create datetime")?
            .and_local_timezone(Utc)
            .single()
            .context("Failed to convert to UTC")?;

    // Update target value (if exists)
    let new_target_value = if let Some(current_value) = goal.target_value {
        let prompt = match goal.goal_type {
            GoalType::Distance => "Target distance (km)",
            GoalType::Duration => "Target duration (minutes)",
            GoalType::Frequency => "Target number of workouts",
            GoalType::Event => "Target value",
        };

        let value: f64 = Input::new()
            .with_prompt(prompt)
            .default(current_value)
            .interact_text()
            .context("Failed to read target value")?;

        Some(value)
    } else {
        None
    };

    // Update notes
    let current_notes = goal.notes.clone().unwrap_or_default();
    let new_notes_input: String = Input::new()
        .with_prompt("Notes (optional)")
        .default(current_notes)
        .allow_empty(true)
        .interact_text()
        .context("Failed to read notes")?;

    let new_notes = if new_notes_input.is_empty() {
        None
    } else {
        Some(new_notes_input)
    };

    // Apply updates
    goal.update(
        Some(new_title),
        Some(new_target_date),
        new_target_value,
        new_notes,
    );

    storage.update_goal(&goal).context("Failed to update goal")?;

    println!();
    println!("✅ Goal updated successfully!");

    Ok(())
}

pub async fn complete_goal(id: &str) -> Result<()> {
    let storage = Storage::init().context("Failed to initialize storage")?;

    let goal = storage
        .get_goal(id)
        .context("Failed to get goal")?
        .ok_or_else(|| anyhow::anyhow!("Goal not found: {}", id))?;

    if goal.completed {
        println!("⚠️  Goal '{}' is already completed", goal.title);
        return Ok(());
    }

    println!("🎯 Complete Goal: {}", goal.title);
    println!();

    let confirm = Confirm::new()
        .with_prompt("Mark this goal as complete?")
        .default(true)
        .interact()
        .context("Failed to confirm")?;

    if !confirm {
        println!("Cancelled");
        return Ok(());
    }

    storage
        .complete_goal(id)
        .context("Failed to complete goal")?;

    println!();
    println!("✅ Goal completed! Great work! 🎉");

    Ok(())
}

pub async fn delete_goal(id: &str, force: bool) -> Result<()> {
    let storage = Storage::init().context("Failed to initialize storage")?;

    let goal = storage
        .get_goal(id)
        .context("Failed to get goal")?
        .ok_or_else(|| anyhow::anyhow!("Goal not found: {}", id))?;

    println!("🗑️  Delete Goal: {}", goal.title);
    println!();
    println!("ID: {}", goal.id);
    println!("Type: {}", goal.goal_type);
    println!("Target Date: {}", goal.target_date.format("%Y-%m-%d"));
    println!();

    if !force {
        let confirm = Confirm::new()
            .with_prompt("Are you sure you want to delete this goal?")
            .default(false)
            .interact()
            .context("Failed to confirm deletion")?;

        if !confirm {
            println!("Cancelled");
            return Ok(());
        }
    }

    let deleted = storage.delete_goal(id).context("Failed to delete goal")?;

    if deleted {
        println!("✅ Goal deleted");
    } else {
        println!("⚠️  Goal not found");
    }

    Ok(())
}

#[derive(clap::Args)]
pub struct GoalsCommand {}
