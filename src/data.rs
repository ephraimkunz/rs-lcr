use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MovedOutPerson {
    pub name: String,
    pub move_date_display: String,
    pub next_unit_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MovedInPerson {
    pub name: String,
    pub move_date: String,
    pub prior_unit_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub address_lines: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberListPerson {
    pub address: Address,
    pub age: u8,
    pub convert: bool,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub sex: String,

    pub name_given_preferred_local: String,
    pub name_family_preferred_local: String,
    pub name_list_preferred_local: String,
}
