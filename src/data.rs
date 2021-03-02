use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MovedOutPerson {
    name: String,
    move_date_display: String,
    next_unit_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MovedInPerson {
    name: String,
    move_date: String,
    prior_unit_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberListPerson {
    age: u8,
    convert: bool,
    email: Option<String>,
    phone_number: Option<String>,
    sex: String,
    name_list_preferred_local: String,
}
