extern crate mysql;
extern crate dotenv;
extern crate chrono;
extern crate chrono_tz;
extern crate reqwest;

use chrono_tz::Tz;
use chrono::prelude::*;
use std::collections::HashMap;
use std::env;

fn main() {
    dotenv::dotenv().ok();

    let TOKEN = env::var("TOKEN").unwrap();
    const URL: &str = "https://discordapp.com/api/v6";

    let mut c = mysql::Conn::new("mysql://root:testpassword@localhost/timezone").unwrap();
    let client = reqwest::Client::new();

    let q = c.query("SELECT * FROM clocks").unwrap();

    for res in q {
        let (_id, channel_id, timezone, channel_name, _guild_id, message_id) = mysql::from_row::<(u32, u64, String, String, u64, Option<u64>)>(res.unwrap());

        let t: Tz = timezone.parse().unwrap();
        let dt = Utc::now().with_timezone(&t);

        if let Some(m_id) = message_id {
            let mut m = HashMap::new();
            m.insert("content", dt.format(&channel_name).to_string());

            client.patch(&format!("{}/channels/{}/messages/{}", URL, channel_id, m_id))
                .json(&m)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bot {}", TOKEN))
                .send().unwrap();
        }
        else {
            let mut m = HashMap::new();
            m.insert("name", dt.format(&channel_name).to_string());

            client.patch(&format!("{}/channels/{}", URL, channel_id))
                .json(&m)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bot {}", TOKEN))
                .send().unwrap();
        }
    }
}
