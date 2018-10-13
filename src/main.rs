extern crate mysql;
extern crate dotenv;
extern crate chrono;
extern crate chrono_tz;
extern crate reqwest;
extern crate threadpool;

use chrono_tz::Tz;
use chrono::prelude::*;
use std::collections::HashMap;
use std::env;
use std::thread;
use std::time::Duration;

fn main() {
    dotenv::dotenv().ok();

    let token = env::var("TOKEN").unwrap();
    let sql_url = env::var("SQL").unwrap();

    const URL: &str = "https://discordapp.com/api/v6";

    let mut c = mysql::Conn::new(sql_url).unwrap();
    let client = reqwest::Client::new();
    let pool = threadpool::ThreadPool::new(8);

    loop {
        let q = c.query("SELECT * FROM clocks").unwrap();
        let mut requests: Vec<reqwest::RequestBuilder> = vec![];

        for res in q {
            let (_id, channel_id, timezone, channel_name, _guild_id, message_id) = mysql::from_row::<(u32, u64, String, String, u64, Option<u64>)>(res.unwrap());

            let t: Tz = timezone.parse().unwrap();
            let dt = Utc::now().with_timezone(&t);

            if let Some(m_id) = message_id {
                let mut m = HashMap::new();
                m.insert("content", dt.format(&channel_name).to_string());

                requests.push(send(format!("{}/channels/{}/messages/{}", URL, channel_id, m_id), &m, &token, &client));
            }
            else {
                let mut m = HashMap::new();
                m.insert("name", dt.format(&channel_name).to_string());

                requests.push(send(format!("{}/channels/{}", URL, channel_id), &m, &token, &client));
            }
        }

        for req in requests {
            pool.execute(move || {
                match req.send() {
                    Err(r) => println!("{:?}", r.status()),

                    Ok(r) => println!("{:?}", r.status())
                }
            });
        }

        thread::sleep(Duration::from_secs(10));
    }
}

fn send(url: String, m: &HashMap<&str, String>, token: &str, client: &reqwest::Client) -> reqwest::RequestBuilder {
    client.patch(&url)
        .json(m)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bot {}", token))
}
