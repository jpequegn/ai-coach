use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use clap::Args;
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};

use crate::storage::Storage;

#[derive(Args)]
pub struct StatsCommand {
    /// Show weekly stats
    #[arg(long)]
    week: bool,

    /// Show monthly stats
    #[arg(long)]
    month: bool,

    /// Show yearly stats
    #[arg(long)]
    year: bool,
}

impl StatsCommand {
    pub async fn execute(self) -> Result<()> {
        let storage = Storage::init().context("Failed to initialize storage")?;
        let all_workouts = storage.list_workouts().context("Failed to list workouts")?;

        if all_workouts.is_empty() {
            println!("📊 No workout data available");
            println!("\n💡 Start logging workouts with 'ai-coach workout log'");
            return Ok(());
        }

        // Determine time period
        let (period_name, start_date) = if self.week {
            ("Week", Utc::now() - Duration::days(7))
        } else if self.month {
            ("Month", Utc::now() - Duration::days(30))
        } else if self.year {
            ("Year", Utc::now() - Duration::days(365))
        } else {
            ("All Time", DateTime::<Utc>::MIN_UTC)
        };

        // Filter workouts by date range
        let workouts: Vec<_> = all_workouts
            .iter()
            .filter(|w| w.date >= start_date)
            .collect();

        if workouts.is_empty() {
            println!("📊 No workouts found for {}", period_name);
            return Ok(());
        }

        println!("📊 Training Statistics - {}", period_name);
        println!();

        // Calculate overall statistics
        let total_workouts = workouts.len();
        let total_duration: u32 = workouts.iter().filter_map(|w| w.duration_minutes).sum();
        let total_distance: f64 = workouts.iter().filter_map(|w| w.distance_km).sum();

        // Calculate averages
        let workouts_with_duration = workouts
            .iter()
            .filter(|w| w.duration_minutes.is_some())
            .count();
        let avg_duration = if workouts_with_duration > 0 {
            total_duration as f64 / workouts_with_duration as f64
        } else {
            0.0
        };

        let workouts_with_distance = workouts
            .iter()
            .filter(|w| w.distance_km.is_some())
            .count();
        let avg_distance = if workouts_with_distance > 0 {
            total_distance / workouts_with_distance as f64
        } else {
            0.0
        };

        // Calculate average pace (min/km) for workouts with both duration and distance
        let workouts_with_pace: Vec<f64> = workouts
            .iter()
            .filter_map(|w| {
                if let (Some(duration), Some(distance)) = (w.duration_minutes, w.distance_km) {
                    if distance > 0.0 {
                        Some(duration as f64 / distance)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        let avg_pace = if !workouts_with_pace.is_empty() {
            workouts_with_pace.iter().sum::<f64>() / workouts_with_pace.len() as f64
        } else {
            0.0
        };

        // Count workouts by type
        let mut type_counts = std::collections::HashMap::new();
        for workout in &workouts {
            *type_counts.entry(workout.exercise_type.as_str()).or_insert(0) += 1;
        }

        // Calculate consistency (days with workouts / total days in period)
        let days_in_period = if self.week {
            7
        } else if self.month {
            30
        } else if self.year {
            365
        } else {
            // For all time, calculate actual days from first workout to now
            if let Some(first_workout) = workouts.iter().min_by_key(|w| w.date) {
                (Utc::now() - first_workout.date).num_days().max(1)
            } else {
                1
            }
        };

        // Count unique workout days
        let unique_days: std::collections::HashSet<_> = workouts
            .iter()
            .map(|w| w.date.date_naive())
            .collect();
        let workout_days = unique_days.len() as i64;

        let consistency_percentage = (workout_days as f64 / days_in_period as f64) * 100.0;

        // Display summary table
        let mut summary = Table::new();
        summary
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic);

        summary.set_header(vec![
            Cell::new("Metric").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ]);

        summary.add_row(vec![
            Cell::new("Total Workouts"),
            Cell::new(total_workouts.to_string()).fg(Color::Green),
        ]);

        if total_duration > 0 {
            summary.add_row(vec![
                Cell::new("Total Duration"),
                Cell::new(format!("{} minutes", total_duration)).fg(Color::Green),
            ]);
        }

        if total_distance > 0.0 {
            summary.add_row(vec![
                Cell::new("Total Distance"),
                Cell::new(format!("{:.2} km", total_distance)).fg(Color::Green),
            ]);
        }

        if avg_duration > 0.0 {
            summary.add_row(vec![
                Cell::new("Average Duration"),
                Cell::new(format!("{:.1} minutes", avg_duration)),
            ]);
        }

        if avg_distance > 0.0 {
            summary.add_row(vec![
                Cell::new("Average Distance"),
                Cell::new(format!("{:.2} km", avg_distance)),
            ]);
        }

        if avg_pace > 0.0 {
            let pace_min = avg_pace.floor() as u32;
            let pace_sec = ((avg_pace - pace_min as f64) * 60.0) as u32;
            summary.add_row(vec![
                Cell::new("Average Pace"),
                Cell::new(format!("{}:{:02} min/km", pace_min, pace_sec)),
            ]);
        }

        summary.add_row(vec![
            Cell::new("Training Days"),
            Cell::new(format!("{} / {}", workout_days, days_in_period)),
        ]);

        let consistency_color = if consistency_percentage >= 70.0 {
            Color::Green
        } else if consistency_percentage >= 50.0 {
            Color::Yellow
        } else {
            Color::Red
        };

        summary.add_row(vec![
            Cell::new("Consistency"),
            Cell::new(format!("{:.1}%", consistency_percentage)).fg(consistency_color),
        ]);

        println!("{summary}");
        println!();

        // Display breakdown by exercise type
        if !type_counts.is_empty() {
            println!("📈 Breakdown by Type:");
            println!();

            let mut type_table = Table::new();
            type_table
                .load_preset(UTF8_FULL)
                .set_content_arrangement(ContentArrangement::Dynamic);

            type_table.set_header(vec![
                Cell::new("Exercise Type").fg(Color::Cyan),
                Cell::new("Workouts").fg(Color::Cyan),
                Cell::new("Percentage").fg(Color::Cyan),
            ]);

            let mut types: Vec<_> = type_counts.iter().collect();
            types.sort_by(|a, b| b.1.cmp(a.1)); // Sort by count descending

            for (exercise_type, count) in types {
                let percentage = (*count as f64 / total_workouts as f64) * 100.0;
                type_table.add_row(vec![
                    Cell::new(exercise_type),
                    Cell::new(count.to_string()),
                    Cell::new(format!("{:.1}%", percentage)),
                ]);
            }

            println!("{type_table}");
        }

        Ok(())
    }
}
