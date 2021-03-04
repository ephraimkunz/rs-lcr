use crate::data::{MemberListPerson, MemberProfile, MovedInPerson, MovedOutPerson};
use crate::error::{Error, HeadlessError};
use headless_chrome::{
    browser::tab::RequestInterceptionDecision,
    protocol::network::events::RequestInterceptedEventParams,
    protocol::network::methods::RequestPattern, Browser, LaunchOptionsBuilder,
};

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;

type Headers = HashMap<String, String>;
type Result<R> = std::result::Result<R, Error>;

// Lots of shenanigans since we can't directly set the headers inside the Fn interceptor because it's not FnMut.
use std::sync::mpsc::{channel, Receiver, Sender};
type MutexedHeaderSender = Mutex<Sender<Headers>>;
type MutexedHeaderReceiver = Mutex<Receiver<Headers>>;
static HEADER_CHANNEL: Lazy<(MutexedHeaderSender, MutexedHeaderReceiver)> = Lazy::new(|| {
    let (tx, rx) = channel();
    (Mutex::new(tx), Mutex::new(rx))
});

#[derive(Debug, Clone)]
pub struct Client {
    username: String,
    password: String,
    unit_number: String,
    headers: Option<Headers>,
}

impl Client {
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
        unit_number: impl Into<String>,
    ) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            unit_number: unit_number.into(),
            headers: None,
        }
    }

    fn get(&mut self, url: &str) -> Result<ureq::Response> {
        let mut req = ureq::get(url);
        let headers = self.header_map()?;
        for (k, v) in headers {
            req = req.set(k, v);
        }

        Ok(req.call()?)
    }

    /// # Errors
    /// HTTP fetching errors for this specific call or for logging in the user specified by the credentials when this client was created.
    pub fn moved_in(&mut self, num_months: u8) -> Result<Vec<MovedInPerson>> {
        let url = format!("https://lcr.churchofjesuschrist.org/services/report/members-moved-in/unit/{}/{}?lang=eng", self.unit_number, num_months);
        let resp = self.get(&url)?;
        let people: Vec<MovedInPerson> = resp.into_json().map_err(Error::Io)?;
        Ok(people)
    }

    /// # Errors
    /// HTTP fetching errors for this specific call or for logging in the user specified by the credentials when this client was created.
    pub fn moved_out(&mut self, num_months: u8) -> Result<Vec<MovedOutPerson>> {
        let url = format!("https://lcr.churchofjesuschrist.org/services/umlu/report/members-moved-out/unit/{}/{}?lang=eng", self.unit_number, num_months);
        let resp = self.get(&url)?;
        let people: Vec<MovedOutPerson> = resp.into_json().map_err(Error::Io)?;
        Ok(people)
    }

    pub fn member_list(&mut self) -> Result<Vec<MemberListPerson>> {
        let url = format!("https://lcr.churchofjesuschrist.org/services/umlu/report/member-list?lang=eng&unitNumber={}", self.unit_number);
        let resp = self.get(&url)?;
        let people: Vec<MemberListPerson> = resp.into_json().map_err(Error::Io)?;
        Ok(people)
    }

    pub fn member_profile(&mut self, legacy_cmis_id: u64) -> Result<MemberProfile> {
        let url = format!(
            "https://lcr.churchofjesuschrist.org/records/member-profile/service/{}?lang=eng",
            legacy_cmis_id
        );
        let resp = self.get(&url)?;
        let profile: MemberProfile = resp.into_json().map_err(Error::Io)?;
        Ok(profile)
    }

    fn header_map(&mut self) -> Result<&Headers> {
        if self.headers.is_none() {
            let headers = self.login()?;
            self.headers = Some(headers);
        }

        match &self.headers {
            None => unreachable!("Headers should have been set above or returned an error"),
            Some(h) => Ok(h),
        }
    }

    fn login(&self) -> Result<Headers> {
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

        tab.type_str(&self.username)
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
        tab.type_str(&self.password)
            .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?;
        sleep(Duration::from_secs(1)); // Not pausing here sometimes results in crashes.

        let submit_element = tab
            .wait_for_element("input[type=submit]")
            .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?;

        // Get the info we need to start requesting stuff ourselves.
        let pattern = RequestPattern {
            url_pattern: None,
            resource_type: Some("Document"),
            interception_stage: Some("Request"),
        };

        let interceptor = Box::new(|_, _, params: RequestInterceptedEventParams| {
            let request = params.request;
            if request.url == "https://lcr.churchofjesuschrist.org/?lang=eng"
                && request.method == "GET"
            {
                HEADER_CHANNEL
                    .0
                    .lock()
                    .unwrap()
                    .send(request.headers)
                    .unwrap();
            }
            RequestInterceptionDecision::Continue
        });

        tab.enable_request_interception(&[pattern], interceptor)
            .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?;

        submit_element
            .click()
            .map_err(|e| Error::Headless(HeadlessError::Wrapped(Box::new(e.compat()))))?;

        let headers = HEADER_CHANNEL.1.lock().unwrap().recv().unwrap();
        if headers.is_empty() {
            Err(Error::Io(std::io::Error::new(
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
        let username = &env::var("LCR_USERNAME").expect("LCR_USERNAME env var required");
        let password = &env::var("LCR_PASSWORD").expect("LCR_PASSWORD env var required");
        let unit_number = &env::var("LCR_UNIT").expect("LCR_UNITenv var required");
        let mut client = Client::new(username, password, unit_number);

        assert!(
            client
                .moved_out(1)
                .expect("Client should have returned a list of moved out people")
                .len()
                > 0
        );
    }

    #[test]
    fn test_moved_in() {
        let username = &env::var("LCR_USERNAME").expect("LCR_USERNAME env var required");
        let password = &env::var("LCR_PASSWORD").expect("LCR_PASSWORD env var required");
        let unit_number = &env::var("LCR_UNIT").expect("LCR_UNITenv var required");
        let mut client = Client::new(username, password, unit_number);

        assert!(
            client
                .moved_in(1)
                .expect("Client should have returned a list of moved in people")
                .len()
                > 0
        );
    }

    #[test]
    fn test_member_list() {
        let username = &env::var("LCR_USERNAME").expect("LCR_USERNAME env var required");
        let password = &env::var("LCR_PASSWORD").expect("LCR_PASSWORD env var required");
        let unit_number = &env::var("LCR_UNIT").expect("LCR_UNITenv var required");
        let mut client = Client::new(username, password, unit_number);

        assert!(
            client
                .member_list()
                .expect("Client should have returned a list of moved in people")
                .len()
                > 0
        );
    }
}
