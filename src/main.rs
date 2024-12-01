use std::error::Error;

pub mod cmd;
pub mod config;
pub mod connect;
pub mod datasource;
pub mod rf;


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = cmd::parse();
    let mut config = config::read(&cli.config)?;
    let mut client = rf::new(&config)?;

    cli.run(&mut client, &mut config).await?;
    return Ok(config.write()?);
}
