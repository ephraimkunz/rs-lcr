use anyhow::{Context, Result};
use lcr::client::Client;
use std::env;

fn main() -> Result<()> {
    let mut client = Client::new(&env::var("LCR_USERNAME")?, &env::var("LCR_PASSWORD")?);

    let moved_out = client
        .moved_out(254)
        .context("Unable to fetch moved out list")?;
    println!("Moved out:\n{:#?}", moved_out);

    println!("---------------------------------------");

    let moved_in = client
        .moved_in(2)
        .context("Unable to fetch moved in list")?;
    println!("Moved in:\n{:#?}", moved_in);

    let member_list = client.member_list().context("Unable to fetch member list");
    println!("Member list:\n{:#?}", member_list);

    Ok(())
}
