use crate::data::{MovedInPerson, MovedOutPerson};
use crate::error::{Error, HeadlessError};
use headless_chrome::{
    browser::tab::RequestInterceptionDecision,
    protocol::network::events::RequestInterceptedEventParams,
    protocol::network::methods::RequestPattern, Browser, LaunchOptionsBuilder,
};

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::thread::sleep;
use std::time::Duration;

type Headers = HashMap<String, String>;
type Result<R> = std::result::Result<R, Error>;

#[derive(Debug, Clone)]
pub struct Credentials {
    username: String,
    password: String,
}

impl Credentials {
    #[must_use]
    pub fn new(username: &str, password: &str) -> Self {
        Self {
            username: username.to_string(),
            password: password.to_string(),
        }
    }
}

#[derive(Debug)]
pub struct Client {
    credentials: Credentials,
    headers: Option<Headers>,
}

impl Client {
    #[must_use]
    pub fn new(credentials: Credentials) -> Self {
        Self {
            credentials,
            headers: None,
        }
    }

    fn get(&mut self, url: &str) -> Result<ureq::Response> {
        let mut req = ureq::get(url);
        let headers = self.header_map()?;
        for (k, v) in headers {
            req.set(k, v);
        }
        let resp = req.call();
        if resp.ok() {
            Ok(resp)
        } else if resp.synthetic() {
            Err(Error::Http(format!(
                "GET returned synthetic error: {}, {}",
                url,
                resp.synthetic_error().as_ref().unwrap()
            )))
        } else {
            Err(Error::Http(format!(
                "GET returned error status code: {}, {}",
                url,
                resp.status_line()
            )))
        }
    }

    /// # Errors
    /// HTTP fetching errors for this specific call or for logging in the user specified by the credentials when this client was created.
    pub fn moved_in(&mut self) -> Result<Vec<MovedInPerson>> {
        let resp = self.get("https://lcr.churchofjesuschrist.org/services/report/members-moved-in/unit/17515/1?lang=eng")?;
        let people: Vec<MovedInPerson> = resp.into_json_deserialize().map_err(|e| Error::IO(e))?;
        Ok(people)
    }

    /// # Errors
    /// HTTP fetching errors for this specific call or for logging in the user specified by the credentials when this client was created.
    pub fn moved_out(&mut self) -> Result<Vec<MovedOutPerson>> {
        let resp = self.get("https://lcr.churchofjesuschrist.org/services/umlu/report/members-moved-out/unit/17515/1?lang=eng")?;
        let people: Vec<MovedOutPerson> = resp.into_json_deserialize().map_err(|e| Error::IO(e))?;
        Ok(people)
    }

    fn header_map(&mut self) -> Result<&Headers> {
        if self.headers.is_none() {
            self.headers = Some(self.login()?);
        }

        match &self.headers {
            None => unreachable!("Headers should have been set above or returned an error"),
            Some(h) => Ok(h),
        }
    }

    fn login(&self) -> Result<Headers> {
        const HEADER_FILE_NAME: &str = "headers.json"; // Kind of a hack, but I can't figure how to share data from within this interceptor closure to outside it another way.

        let launch_options = LaunchOptionsBuilder::default()
            .headless(true)
            .build()
            .map_err(|s| Error::Headless(HeadlessError::String(s)))?;
        let browser = Browser::new(launch_options)
            .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?;
        let tab = browser
            .wait_for_initial_tab()
            .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?;
        tab.set_default_timeout(Duration::from_secs(30));

        // Navigate to site.
        tab.navigate_to("https://lcr.churchofjesuschrist.org")
            .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?;

        // Username. There's probably a better way to do this than clicking the element 3 times, but just doing it
        // once seems to fail on slow internet connections.
        for _ in 0..3 {
            tab.wait_for_element("input#okta-signin-username")
                .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?
                .click()
                .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?;
        }

        tab.type_str(&self.credentials.username)
            .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?;
        tab.wait_for_element("input#okta-signin-submit")
            .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?
            .click()
            .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?;

        // Password
        tab.wait_for_element("input[type=password]")
            .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?
            .click()
            .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?;
        tab.type_str(&self.credentials.password)
            .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?;
        sleep(Duration::from_secs(1)); // Not pausing here sometimes results in crashes.
        tab.wait_for_element("input[type=submit]")
            .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?
            .click()
            .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?;

        // Real page
        let member_lookup = tab
            .wait_for_element("input#memberLookupMain")
            .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?;

        // Get the info we need to start requesting stuff ourselves.
        let pattern = RequestPattern {
            url_pattern: None,
            resource_type: Some("XHR"),
            interception_stage: Some("Request"),
        };
        let patterns = vec![pattern];

        let interceptor = Box::new(|_, _, params: RequestInterceptedEventParams| {
            let request = params.request;
            if request
                .url
                .starts_with("https://lcr.churchofjesuschrist.org/services/member-lookup")
            {
                File::create(HEADER_FILE_NAME)
                    .and_then(|mut f| {
                        let s = serde_json::to_string(&request.headers)
                            .expect("Unable to serialze request headers to string");
                        f.write_all(s.as_bytes())
                    })
                    .expect("Unable to write headers to file");
            }

            RequestInterceptionDecision::Continue
        });

        tab.enable_request_interception(&patterns, interceptor)
            .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?;
        member_lookup
            .click()
            .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?;

        tab.type_str("ephraim")
            .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?;
        sleep(Duration::from_secs(1)); // Wait for network request.

        let s = fs::read_to_string(HEADER_FILE_NAME).map_err(|e| Error::IO(e))?;
        let headers: HashMap<String, String> = serde_json::from_str(&s)
            .map_err(|e| Error::IO(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;
        fs::remove_file(HEADER_FILE_NAME).map_err(|e| Error::IO(e))?;

        if headers.is_empty() {
            Err(Error::IO(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Header for making queries has no entries".to_string(),
            )))
        } else {
            Ok(headers)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_moved_out() {
        let credentials = Credentials::new(
            &env::var("LCR_USERNAME").expect("LCR_USERNAME env var required"),
            &env::var("LCR_PASSWORD").expect("LCR_USERNAME env var required"),
        );
        let mut client = Client::new(credentials);

        assert!(client.moved_out().expect("Client should have returned a list of moved out people").len() > 0);
    }

    #[test]
    fn test_moved_in() {
        let credentials = Credentials::new(
            &env::var("LCR_USERNAME").expect("LCR_USERNAME env var required"),
            &env::var("LCR_PASSWORD").expect("LCR_USERNAME env var required"),
        );
        let mut client = Client::new(credentials);

        assert!(client.moved_in().expect("Client should have returned a list of moved in people").len() > 0);
    }
}
