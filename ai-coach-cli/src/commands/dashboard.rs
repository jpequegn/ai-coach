use crate::ui::Dashboard;
use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct DashboardCommand {}

impl DashboardCommand {
    pub async fn execute(self) -> Result<()> {
        let mut dashboard = Dashboard::new()?;
        dashboard.run()?;
        Ok(())
    }
}
