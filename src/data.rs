use serde::{Deserialize, Serialize};
use time::macros::format_description;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovedOutPerson {
    pub name: String,
    pub move_date_display: String,
    pub next_unit_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovedInPerson {
    pub name: String,
    pub move_date: String,
    pub prior_unit_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub address_lines: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberListPerson {
    pub address: Option<Address>,
    pub age: u8,
    pub convert: bool,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub sex: String,
    pub legacy_cmis_id: u64,

    pub name_given_preferred_local: String,
    pub name_family_preferred_local: String,
    pub name_list_preferred_local: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhotoInfo {
    pub spoken_name: String,
    pub image: Image,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    pub token_url: String,
}

#[derive(Debug)]
pub struct VisualPerson {
    pub name: String,
    pub photo_url: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberProfile {
    pub individual: MemberProfileIndividual,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberProfileIndividual {
    pub move_date: Option<String>,
    pub mrn: Option<String>,
    pub id: u64, // legacy_cmis_id elsewhere
    pub endowed: Option<bool>,
}

impl MemberProfileIndividual {
    pub fn move_date(&self) -> Option<time::Date> {
        let date_format = format_description!("[year][month][day]");
        self.move_date
            .as_ref()
            .and_then(|m| time::Date::parse(m, &date_format).ok())
    }
}
