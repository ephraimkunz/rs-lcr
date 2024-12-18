use std::collections::{HashMap, HashSet};

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RSMinisteringAssignments {
    relief_society: Vec<QuorumOrClass>,
}

impl RSMinisteringAssignments {
    pub fn collect_unique_names(
        &self,
        set: &mut HashSet<String>,
        only_females: bool,
        females_by_id: &HashMap<u64, bool>,
    ) {
        for class in &self.relief_society {
            class.collect_unique_names(set, only_females, females_by_id);
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EQMinisteringAssignments {
    elders: Vec<QuorumOrClass>,
}

impl EQMinisteringAssignments {
    pub fn collect_unique_names(
        &self,
        set: &mut HashSet<String>,
        only_females: bool,
        females_by_id: &HashMap<u64, bool>,
    ) {
        for quorum in &self.elders {
            quorum.collect_unique_names(set, only_females, females_by_id);
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuorumOrClass {
    companionships: Vec<Companionship>,
}

impl QuorumOrClass {
    pub fn collect_unique_names(
        &self,
        set: &mut HashSet<String>,
        only_females: bool,
        females_by_id: &HashMap<u64, bool>,
    ) {
        for companionship in &self.companionships {
            companionship.collect_unique_names(set, only_females, females_by_id);
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Companionship {
    ministers: Vec<MinisteringCompanionship>,
    assignments: Option<Vec<MinisteringAssignment>>,
}

impl Companionship {
    pub fn collect_unique_names(
        &self,
        set: &mut HashSet<String>,
        only_females: bool,
        females_by_id: &HashMap<u64, bool>,
    ) {
        for minister in &self.ministers {
            let is_female = females_by_id.get(&minister.legacy_cmis_id).unwrap_or(&true);
            if !only_females || *is_female {
                set.insert(minister.name.to_string());
            }
        }

        if let Some(assignments) = &self.assignments {
            for assignment in assignments {
                let is_female = females_by_id
                    .get(&assignment.legacy_cmis_id)
                    .unwrap_or(&true);
                if !only_females || *is_female {
                    set.insert(assignment.name.to_string());
                }
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinisteringCompanionship {
    name: String,
    legacy_cmis_id: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinisteringAssignment {
    name: String,
    legacy_cmis_id: u64,
}
