use anyhow::{Context, Result};
use clap::Args;
use dialoguer::{Input, Select};

use super::workout_parser::WorkoutParser;
use crate::models::Workout;
use crate::storage::Storage;

#[derive(Args)]
pub struct WorkoutCommand {
    /// Natural language workout description (e.g., "Ran 5 miles in 40 minutes")
    description: Option<String>,

    /// Exercise type (running, cycling, swimming, walking, strength)
    #[arg(short = 't', long)]
    exercise_type: Option<String>,

    /// Duration in minutes
    #[arg(short = 'd', long)]
    duration: Option<u32>,

    /// Distance in kilometers
    #[arg(long)]
    distance: Option<f64>,

    /// Additional notes
    #[arg(short = 'n', long)]
    notes: Option<String>,
}

impl WorkoutCommand {
    pub async fn execute(self) -> Result<()> {
        println!("🏃 AI Coach - Log Workout");
        println!();

        let storage = Storage::init().context("Failed to initialize storage")?;

        // Try natural language parsing first if description provided
        if let Some(desc) = &self.description {
            if let Some(parsed) = self.try_parse_description(desc) {
                println!(
                    "✓ Parsed: {} for {} minutes",
                    parsed.exercise_type,
                    parsed
                        .duration_minutes
                        .map(|d| d.to_string())
                        .unwrap_or_else(|| "?".to_string())
                );

                let workout = Workout::new(
                    parsed.exercise_type,
                    parsed.duration_minutes,
                    parsed.distance_km,
                    self.notes.clone().or_else(|| Some(desc.clone())),
                );

                storage
                    .save_workout(&workout)
                    .context("Failed to save workout")?;

                storage
                    .queue_for_sync(&workout.id)
                    .context("Failed to queue for sync")?;

                self.print_success(&workout);
                return Ok(());
            } else {
                println!("⚠ Could not parse description, falling back to interactive mode");
                println!();
            }
        }

        // Use provided args or prompt interactively
        let exercise_type = if let Some(ref et) = self.exercise_type {
            et.clone()
        } else {
            self.prompt_exercise_type()?
        };

        let duration_minutes = if let Some(d) = self.duration {
            Some(d)
        } else {
            self.prompt_duration()?
        };

        let distance_km = if self.distance.is_some() {
            self.distance
        } else {
            self.prompt_distance(&exercise_type)?
        };

        let notes = if self.notes.is_some() {
            self.notes.clone()
        } else {
            self.prompt_notes()?
        };

        let workout = Workout::new(exercise_type, duration_minutes, distance_km, notes);

        storage
            .save_workout(&workout)
            .context("Failed to save workout")?;

        storage
            .queue_for_sync(&workout.id)
            .context("Failed to queue for sync")?;

        self.print_success(&workout);
        Ok(())
    }

    fn try_parse_description(
        &self,
        description: &str,
    ) -> Option<super::workout_parser::ParsedWorkout> {
        let parser = WorkoutParser::new();
        parser.parse(description).ok()
    }

    fn prompt_exercise_type(&self) -> Result<String> {
        let options = vec![
            "Running",
            "Cycling",
            "Swimming",
            "Walking",
            "Strength Training",
            "Other",
        ];

        let selection = Select::new()
            .with_prompt("Exercise Type")
            .items(&options)
            .default(0)
            .interact()
            .context("Failed to get exercise type")?;

        Ok(match selection {
            0 => "running",
            1 => "cycling",
            2 => "swimming",
            3 => "walking",
            4 => "strength",
            _ => {
                let custom: String = Input::new()
                    .with_prompt("Enter exercise type")
                    .interact_text()
                    .context("Failed to get custom exercise type")?;
                return Ok(custom.to_lowercase());
            }
        }
        .to_string())
    }

    fn prompt_duration(&self) -> Result<Option<u32>> {
        let input: String = Input::new()
            .with_prompt("Duration (minutes, press Enter to skip)")
            .allow_empty(true)
            .interact_text()
            .context("Failed to get duration")?;

        if input.trim().is_empty() {
            Ok(None)
        } else {
            let duration = input
                .trim()
                .parse::<u32>()
                .context("Invalid duration, must be a number")?;
            Ok(Some(duration))
        }
    }

    fn prompt_distance(&self, exercise_type: &str) -> Result<Option<f64>> {
        // Only prompt for distance for cardio exercises
        if !matches!(
            exercise_type,
            "running" | "cycling" | "swimming" | "walking"
        ) {
            return Ok(None);
        }

        let input: String = Input::new()
            .with_prompt("Distance (km, press Enter to skip)")
            .allow_empty(true)
            .interact_text()
            .context("Failed to get distance")?;

        if input.trim().is_empty() {
            Ok(None)
        } else {
            let distance = input
                .trim()
                .parse::<f64>()
                .context("Invalid distance, must be a number")?;
            Ok(Some(distance))
        }
    }

    fn prompt_notes(&self) -> Result<Option<String>> {
        let input: String = Input::new()
            .with_prompt("Notes (press Enter to skip)")
            .allow_empty(true)
            .interact_text()
            .context("Failed to get notes")?;

        if input.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(input))
        }
    }

    fn print_success(&self, workout: &Workout) {
        println!();
        println!("✅ Workout logged successfully!");
        println!();
        println!("  ID:       {}", workout.id);
        println!("  Type:     {}", workout.exercise_type);

        if let Some(duration) = workout.duration_minutes {
            println!("  Duration: {} min", duration);
        }

        if let Some(distance) = workout.distance_km {
            println!("  Distance: {:.2} km", distance);
        }

        if let Some(ref notes) = workout.notes {
            println!("  Notes:    {}", notes);
        }

        println!();
        println!("⏳ Queued for sync with server");
    }
}

pub async fn list_workouts(
    exercise_type: Option<String>,
    from: Option<String>,
    to: Option<String>,
    limit: usize,
) -> Result<()> {
    use chrono::{DateTime, Utc};

    println!("📋 Recent Workouts");
    println!();

    let storage = Storage::init().context("Failed to initialize storage")?;

    let mut workouts = storage.list_workouts().context("Failed to list workouts")?;

    // Apply filters
    if let Some(ref et) = exercise_type {
        workouts.retain(|w| w.exercise_type == *et);
    }

    if let Some(ref from_str) = from {
        let from_date = DateTime::parse_from_rfc3339(from_str)
            .or_else(|_| {
                // Try parsing as date only
                chrono::NaiveDate::parse_from_str(from_str, "%Y-%m-%d")
                    .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc().fixed_offset())
            })
            .context("Invalid from date format (use YYYY-MM-DD)")?;
        workouts.retain(|w| w.date >= from_date);
    }

    if let Some(ref to_str) = to {
        let to_date = DateTime::parse_from_rfc3339(to_str)
            .or_else(|_| {
                chrono::NaiveDate::parse_from_str(to_str, "%Y-%m-%d")
                    .map(|d| d.and_hms_opt(23, 59, 59).unwrap().and_utc().fixed_offset())
            })
            .context("Invalid to date format (use YYYY-MM-DD)")?;
        workouts.retain(|w| w.date <= to_date);
    }

    // Apply limit
    workouts.truncate(limit);

    if workouts.is_empty() {
        println!("No workouts found.");
        println!();
        println!("Log your first workout with: ai-coach workout log");
        return Ok(());
    }

    // Print table header
    println!(
        "{:<12} {:<10} {:<12} {:<10} {:<8} {:<30}",
        "DATE", "TYPE", "DURATION", "DISTANCE", "SYNC", "NOTES"
    );
    println!("{}", "-".repeat(92));

    // Print workouts
    for workout in &workouts {
        let date_str = workout.date.format("%Y-%m-%d").to_string();
        let duration_str = workout
            .duration_minutes
            .map(|d| format!("{} min", d))
            .unwrap_or_else(|| "-".to_string());
        let distance_str = workout
            .distance_km
            .map(|d| format!("{:.2} km", d))
            .unwrap_or_else(|| "-".to_string());
        let sync_icon = if workout.synced { "✓" } else { "⏳" };
        let notes_str = workout
            .notes
            .as_ref()
            .map(|n| {
                if n.len() > 30 {
                    format!("{}...", &n[..27])
                } else {
                    n.clone()
                }
            })
            .unwrap_or_else(|| "-".to_string());

        println!(
            "{:<12} {:<10} {:<12} {:<10} {:<8} {:<30}",
            date_str, workout.exercise_type, duration_str, distance_str, sync_icon, notes_str
        );
    }

    println!();
    println!("Showing {} workout(s)", workouts.len());

    Ok(())
}

pub async fn show_workout(id: &str) -> Result<()> {
    println!("📖 Workout Details");
    println!();

    let storage = Storage::init().context("Failed to initialize storage")?;

    let workout = storage
        .get_workout(id)
        .context("Failed to get workout")?
        .ok_or_else(|| anyhow::anyhow!("Workout not found: {}", id))?;

    println!("  ID:           {}", workout.id);
    println!(
        "  Date:         {}",
        workout.date.format("%Y-%m-%d %H:%M:%S")
    );
    println!("  Exercise:     {}", workout.exercise_type);

    if let Some(duration) = workout.duration_minutes {
        println!("  Duration:     {} minutes", duration);
    }

    if let Some(distance) = workout.distance_km {
        println!("  Distance:     {:.2} km", distance);
    }

    if let Some(ref notes) = workout.notes {
        println!("  Notes:        {}", notes);
    }

    println!(
        "  Synced:       {}",
        if workout.synced { "Yes ✓" } else { "No ⏳" }
    );
    println!(
        "  Created:      {}",
        workout.created_at.format("%Y-%m-%d %H:%M:%S")
    );
    println!(
        "  Updated:      {}",
        workout.updated_at.format("%Y-%m-%d %H:%M:%S")
    );

    Ok(())
}

pub async fn edit_workout(id: &str) -> Result<()> {
    println!("✏️  Edit Workout");
    println!();

    let storage = Storage::init().context("Failed to initialize storage")?;

    let mut workout = storage
        .get_workout(id)
        .context("Failed to get workout")?
        .ok_or_else(|| anyhow::anyhow!("Workout not found: {}", id))?;

    println!("Current values (press Enter to keep):");
    println!();

    // Exercise type
    let exercise_type: String = Input::new()
        .with_prompt("Exercise type")
        .default(workout.exercise_type.clone())
        .interact_text()
        .context("Failed to get exercise type")?;

    // Duration
    let duration_str: String = Input::new()
        .with_prompt("Duration (minutes)")
        .default(
            workout
                .duration_minutes
                .map(|d| d.to_string())
                .unwrap_or_default(),
        )
        .allow_empty(true)
        .interact_text()
        .context("Failed to get duration")?;

    let duration_minutes = if duration_str.trim().is_empty() {
        None
    } else {
        Some(
            duration_str
                .trim()
                .parse::<u32>()
                .context("Invalid duration")?,
        )
    };

    // Distance
    let distance_str: String = Input::new()
        .with_prompt("Distance (km)")
        .default(
            workout
                .distance_km
                .map(|d| d.to_string())
                .unwrap_or_default(),
        )
        .allow_empty(true)
        .interact_text()
        .context("Failed to get distance")?;

    let distance_km = if distance_str.trim().is_empty() {
        None
    } else {
        Some(
            distance_str
                .trim()
                .parse::<f64>()
                .context("Invalid distance")?,
        )
    };

    // Notes
    let notes: String = Input::new()
        .with_prompt("Notes")
        .default(workout.notes.clone().unwrap_or_default())
        .allow_empty(true)
        .interact_text()
        .context("Failed to get notes")?;

    let notes = if notes.trim().is_empty() {
        None
    } else {
        Some(notes)
    };

    // Update workout
    workout.update(Some(exercise_type), duration_minutes, distance_km, notes);

    storage
        .save_workout(&workout)
        .context("Failed to save workout")?;

    storage
        .queue_for_sync(&workout.id)
        .context("Failed to queue for sync")?;

    println!();
    println!("✅ Workout updated successfully!");
    println!("⏳ Queued for sync with server");

    Ok(())
}

pub async fn delete_workout(id: &str, force: bool) -> Result<()> {
    use dialoguer::Confirm;

    println!("🗑️  Delete Workout");
    println!();

    let storage = Storage::init().context("Failed to initialize storage")?;

    let workout = storage
        .get_workout(id)
        .context("Failed to get workout")?
        .ok_or_else(|| anyhow::anyhow!("Workout not found: {}", id))?;

    // Show workout details
    println!("  Type:     {}", workout.exercise_type);
    println!("  Date:     {}", workout.date.format("%Y-%m-%d"));
    if let Some(duration) = workout.duration_minutes {
        println!("  Duration: {} min", duration);
    }
    println!();

    // Confirm deletion
    if !force {
        let confirmed = Confirm::new()
            .with_prompt("Are you sure you want to delete this workout?")
            .default(false)
            .interact()
            .context("Failed to get confirmation")?;

        if !confirmed {
            println!("Cancelled.");
            return Ok(());
        }
    }

    storage
        .delete_workout(id)
        .context("Failed to delete workout")?;

    println!();
    println!("✅ Workout deleted successfully!");

    Ok(())
}
