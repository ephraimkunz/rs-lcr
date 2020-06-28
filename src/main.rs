mod lcr;
use anyhow::Result;
use lcr::{Client, Credentials};
use std::env;

fn main() -> Result<()> {
    let credentials = Credentials::new(&env::var("LCR_USERNAME")?, &env::var("LCR_PASSWORD")?);
    let mut client = Client::new(credentials);

    let moved_in = client.moved_in()?;
    println!("Moved in:\n{:#?}", moved_in);

    println!("---------------------------------------");

    let moved_out = &client.moved_out()?;
    println!("Moved out:\n{:#?}", moved_out);

    Ok(())
}
