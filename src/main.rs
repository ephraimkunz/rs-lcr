use anyhow::{Context, Result};
use lcr::client::ClientOptions;
use lcr::{client::Client, data::MemberListPerson};
use std::collections::HashMap;
use std::env;
use time::OffsetDateTime;

fn main() -> Result<()> {
    let username = &env::var("LCR_USERNAME").expect("LCR_USERNAME env var required");
    let password = &env::var("LCR_PASSWORD").expect("LCR_PASSWORD env var required");
    let unit_number = &env::var("LCR_UNIT").expect("LCR_UNIT env var required");
    let mut client = Client::new_with_options(
        username,
        password,
        unit_number,
        ClientOptions { headless: false },
    );

    let moved_out = client
        .moved_out(254)
        .context("Unable to fetch moved out list")?;
    println!("Moved out:\n{:#?}", moved_out);

    println!("---------------------------------------");

    let moved_in = client
        .moved_in(2)
        .context("Unable to fetch moved in list")?;
    println!("Moved in:\n{:#?}", moved_in);

    let member_list = client
        .member_list()
        .context("Unable to fetch member list")?;
    println!("Member list:\n{:#?}", member_list);

    print_age_buckets(&member_list);
    print_gender_buckets(&member_list);

    let profiles: Result<Vec<_>> = member_list
        .iter()
        .map(|m| {
            client
                .member_profile(m.legacy_cmis_id)
                .context("Unable to fetch member profile")
        })
        .collect();

    // println!("{profiles:#?}");

    let profiles = profiles?;
    let now = OffsetDateTime::now_utc();
    let durations: Vec<_> = profiles
        .iter()
        .filter_map(|profile| {
            profile.individual.move_date().map(|m| {
                let difference = now.date() - m;
                difference.whole_weeks() / 4
            })
        })
        .collect();

    print_time_in_ward_buckets(&durations);

    Ok(())
}

fn print_gender_buckets(members: &[MemberListPerson]) {
    let mut male = 0;
    let mut female = 0;

    for member in members {
        if member.sex.eq_ignore_ascii_case("m") {
            male += 1;
        } else {
            female += 1;
        }
    }

    println!("\nGender buckets:\n{:^7}{:^7}", "Gender", "Count");
    let num = male;
    let mut s = String::new();
    for _ in 0..num {
        s.push('#');
    }

    println!("{:^7}{:^7} {}", "Male", num, s);

    let num = female;
    let mut s = String::new();
    for _ in 0..num {
        s.push('#');
    }

    println!("{:^7}{:^7} {}", "Female", num, s);
}

fn print_age_buckets(members: &[MemberListPerson]) {
    let mut map = HashMap::new();
    for member in members {
        let entry = map.entry(member.age).or_insert(0u8);
        *entry += 1;
    }

    let mut keys: Vec<_> = map.keys().collect();
    keys.sort();

    println!("\nAge buckets:\n{:^7}{:^7}", "Age", "Count");
    for key in keys {
        let num = map[key];
        let mut s = String::new();
        for _ in 0..num {
            s.push('#');
        }

        println!("{:^7}{:^7} {}", key, num, s);
    }
}

fn print_time_in_ward_buckets(month_vec: &[i64]) {
    let mut map = HashMap::new();
    for num_months in month_vec {
        let entry = map.entry(num_months).or_insert(0u8);
        *entry += 1;
    }

    let mut keys: Vec<_> = map.keys().collect();
    keys.sort();

    let mut running_count = 0;
    let total_count = month_vec.len();

    println!(
        "\nTime in ward buckets:\n{:^7}{:^7}{:^7}{:^7}",
        "Months", "Count", "Running", "Percent"
    );
    for key in keys {
        let num = map[key];
        running_count += num;
        let mut s = String::new();
        for _ in 0..num {
            s.push('#');
        }

        println!(
            "{:^7}{:^7}{:^7}{:<7.2} {}",
            key,
            num,
            running_count,
            (running_count as f32 / total_count as f32) * 100f32,
            s
        );
    }
}
