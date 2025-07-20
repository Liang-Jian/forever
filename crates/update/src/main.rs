

use anyhow_ext::{Ok, Result};
use update::web::upgrade_ap;



#[tokio::main]
async fn main() -> Result<()> {
    upgrade_ap().await?;
    Ok(())
}
