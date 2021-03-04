use anyhow::{Context, Result};
use lcr::{client::Client, data::MemberListPerson};
use std::collections::HashMap;
use std::env;

fn main() -> Result<()> {
    let mut client = Client::new(
        &env::var("LCR_USERNAME")?,
        &env::var("LCR_PASSWORD")?,
        &env::var("LCR_UNIT")?,
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

    println!("\nGender buckets:\n{:^7}{:^7}", "Gender", "Number");
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

    println!("\nAge buckets:\n{:^7}{:^7}", "Age", "Number");
    for key in keys {
        let num = map[key];
        let mut s = String::new();
        for _ in 0..num {
            s.push('#');
        }

        println!("{:^7}{:^7} {}", key, num, s);
    }
}
