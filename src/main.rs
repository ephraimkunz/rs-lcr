use anyhow::{Context, Result};
use lcr::client::{Client, Credentials};
use std::env;

fn main() -> Result<()> {
    let credentials = Credentials::new(&env::var("LCR_USERNAME")?, &env::var("LCR_PASSWORD")?);
    let mut client = Client::new(credentials);

    let moved_out = client
        .moved_out()
        .context("Unable to fetch moved out list")?;
    println!("Moved out:\n{:#?}", moved_out);

    println!("---------------------------------------");

    let moved_in = client.moved_in().context("Unable to fetch moved in list")?;
    println!("Moved in:\n{:#?}", moved_in);

    Ok(())
}
