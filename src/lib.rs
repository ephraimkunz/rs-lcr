use anyhow::{anyhow, Result};
use headless_chrome::{
    browser::tab::RequestInterceptionDecision,
    protocol::network::events::RequestInterceptedEventParams,
    protocol::network::methods::RequestPattern, Browser, LaunchOptionsBuilder,
};
use reqwest::{
    blocking,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::Deserialize;

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::thread::sleep;
use std::time::Duration;

#[derive(Debug)]
pub struct Credentials {
    username: String,
    password: String,
}

impl Credentials {
    pub fn new(username: &str, password: &str) -> Self {
        Credentials {
            username: username.to_string(),
            password: password.to_string(),
        }
    }
}

#[derive(Debug)]
pub struct Client {
    wrapped_client: blocking::Client,
    credentials: Credentials,
    headers: Option<Headers>,
}

impl Client {
    pub fn new(credentials: Credentials) -> Self {
        Client {
            wrapped_client: blocking::Client::new(),
            credentials,
            headers: None,
        }
    }

    pub fn moved_in(&mut self) -> Result<Vec<MovedInPerson>> {
        let people: Vec<MovedInPerson> = self.wrapped_client.get("https://lcr.churchofjesuschrist.org/services/report/members-moved-in/unit/17515/1?lang=eng")
        .headers(self.header_map()?).send()?.json()?;
        Ok(people)
    }

    pub fn moved_out(&mut self) -> Result<Vec<MovedOutPerson>> {
        let people: Vec<MovedOutPerson> = self.wrapped_client.get("https://lcr.churchofjesuschrist.org/services/umlu/report/members-moved-out/unit/17515/1?lang=eng")
        .headers(self.header_map()?).send()?.json()?;
        Ok(people)
    }

    fn header_map(&mut self) -> Result<HeaderMap> {
        if self.headers.is_none() {
            self.headers = Some(self.login()?);
        }

        match &self.headers {
            None => unreachable!("Headers should have been set above or returned an error"),
            Some(h) => Ok(h.header_map()?),
        }
    }

    fn login(&self) -> Result<Headers> {
        let launch_options = LaunchOptionsBuilder::default()
            .headless(false)
            .build()
            .unwrap();
        let browser = Browser::new(launch_options).map_err(|e| e.compat())?;
        let tab = browser.wait_for_initial_tab().map_err(|e| e.compat())?;

        // Navigate to site.
        tab.navigate_to("https://lcr.churchofjesuschrist.org")
            .map_err(|e| e.compat())?;

        // Username
        tab.wait_for_element_with_custom_timeout(
            "input#okta-signin-username",
            Duration::from_secs(10),
        )
        .map_err(|e| e.compat())?
        .click()
        .map_err(|e| e.compat())?;
        tab.type_str(&self.credentials.username)
            .map_err(|e| e.compat())?;
        tab.wait_for_element("input#okta-signin-submit")
            .map_err(|e| e.compat())?
            .click()
            .map_err(|e| e.compat())?;

        // Password
        tab.wait_for_element("input[type=password]")
            .map_err(|e| e.compat())?
            .click()
            .map_err(|e| e.compat())?;
        tab.type_str(&self.credentials.password)
            .map_err(|e| e.compat())?;
        pause_for(1); // Not pausing here sometimes results in crashes.
        tab.wait_for_element("input[type=submit]")
            .map_err(|e| e.compat())?
            .click()
            .map_err(|e| e.compat())?;

        // Real page
        let member_lookup = tab
            .wait_for_element_with_custom_timeout("input#memberLookupMain", Duration::from_secs(5))
            .map_err(|e| e.compat())?;

        // Get the info we need to start requesting stuff ourselves.
        let pattern = RequestPattern {
            url_pattern: None,
            resource_type: Some("XHR"),
            interception_stage: Some("Request"),
        };
        let patterns = vec![pattern];

        const HEADER_FILE_NAME: &str = "headers.json"; // Kind of a hack, but I can't figure how to share data from within this closure to outside it another way.
        let interceptor = Box::new(|_, _, params: RequestInterceptedEventParams| {
            let request = params.request;
            if request
                .url
                .starts_with("https://lcr.churchofjesuschrist.org/services/member-lookup")
            {
                File::create(HEADER_FILE_NAME)
                    .and_then(|mut f| {
                        let s = serde_json::to_string(&request.headers).unwrap();
                        f.write_all(s.as_bytes())
                    })
                    .expect("Unable to write headers to file");
            }

            RequestInterceptionDecision::Continue
        });

        tab.enable_request_interception(&patterns, interceptor)
            .map_err(|e| e.compat())?;

        member_lookup.click().map_err(|e| e.compat())?;
        tab.type_str("ephraim").map_err(|e| e.compat())?;
        pause_for(1); // Wait for network request.

        let s = fs::read_to_string(HEADER_FILE_NAME)?;
        let headers: HashMap<String, String> = serde_json::from_str(&s)?;
        fs::remove_file(HEADER_FILE_NAME)?;

        match headers.is_empty() {
            true => Err(anyhow!("Couldn't retrieve header for making queries")),
            false => Ok(Headers::new(headers)),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MovedOutPerson {
    name: String,
    move_date_display: String,
    next_unit_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MovedInPerson {
    name: String,
    move_date: String,
    prior_unit_name: Option<String>,
}

#[derive(Debug)]
struct Headers(HashMap<String, String>);
impl Headers {
    fn new(map: HashMap<String, String>) -> Self {
        Headers(map)
    }

    fn header_map(&self) -> Result<HeaderMap> {
        let mut hm = HeaderMap::new();
        for (k, v) in &self.0 {
            let header_name = HeaderName::from_lowercase(k.to_lowercase().as_bytes())?;
            let header_value = HeaderValue::from_str(v)?;
            hm.insert(header_name, header_value);
        }

        Ok(hm)
    }
}

fn pause_for(d: u64) {
    sleep(Duration::from_secs(d));
}
