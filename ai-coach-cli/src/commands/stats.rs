use anyhow::Result;
use clap::Args;

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
        println!("Training Statistics");
        println!();

        let period = if self.week {
            "Week"
        } else if self.month {
            "Month"
        } else if self.year {
            "Year"
        } else {
            "All Time"
        };

        println!("Period: {}", period);
        println!();
        println!("TODO: Display training statistics");

        Ok(())
    }
}
