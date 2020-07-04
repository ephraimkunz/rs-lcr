// use anyhow::{Context, Result};
// use lcr::{Client, Credentials};
// use std::env;

// fn main() -> Result<()> {
//     let credentials = Credentials::new(&env::var("LCR_USERNAME")?, &env::var("LCR_PASSWORD")?);
//     let mut client = Client::new(credentials);

//     let moved_out = client
//         .moved_out()
//         .context("Unable to fetch moved out list")?;
//     println!("Moved out:\n{:#?}", moved_out);

//     println!("---------------------------------------");

//     let moved_in = client.moved_in().context("Unable to fetch moved in list")?;
//     println!("Moved in:\n{:#?}", moved_in);

//     Ok(())
// }

use fantoccini::{Client, Locator};

// let's set up the sequence of steps we want the browser to take
#[tokio::main]
async fn main() -> Result<(), fantoccini::error::CmdError> {
    let mut c = Client::new("http://localhost:4444").await.expect("failed to connect to WebDriver");

    // first, go to the Wikipedia page for Foobar
    c.goto("https://en.wikipedia.org/wiki/Foobar").await?;
    let url = c.current_url().await?;
    assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");

    // click "Foo (disambiguation)"
    c.find(Locator::Css(".mw-disambig")).await?.click().await?;

    // click "Foo Lake"
    c.find(Locator::LinkText("Foo Lake")).await?.click().await?;

    let url = c.current_url().await?;
    assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foo_Lake");

    c.close().await
}
