#[macro_use] extern crate mysql;

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

    let token = env::var("DISCORD_TOKEN").unwrap();
    let sql_url = env::var("SQL_URL").unwrap();
    let interval = env::var("INTERVAL").unwrap().parse::<u64>().unwrap();
    let threads = env::var("THREADS").unwrap().parse::<usize>().unwrap();

    const URL: &str = "https://discordapp.com/api/v6";

    let mysql_conn = mysql::Pool::new(sql_url).unwrap();
    let req_client = reqwest::Client::new();
    let pool = threadpool::ThreadPool::new(threads);

    loop {
        pool.join();

        let q = mysql_conn.prep_exec("SELECT * FROM clocks", ()).unwrap();

        for res in q {
            let (id, channel_id, timezone, channel_name, message_id) = mysql::from_row::<(u32, u64, String, String, Option<u64>)>(res.unwrap());

            let t: Tz = timezone.parse().unwrap();
            let dt = Utc::now().with_timezone(&t);

            let mut m = HashMap::new();
            m.insert("name", dt.format(&channel_name).to_string());

            let req = send(format!("{}/channels/{}", URL, channel_id), &m, &token, &req_client);

            let c = mysql_conn.clone();
            pool.execute(move || {
                match req.send() {
                    Err(_) => {},

                    Ok(r) => {
                        if r.status() == 404 {
                            let _ = c.prep_exec("DELETE FROM clocks WHERE id = :id", params!{"id" => id});
                        }
                    }
                }
            });
        }

        thread::sleep(Duration::from_secs(interval));
    }
}

fn send(url: String, m: &HashMap<&str, String>, token: &str, client: &reqwest::Client) -> reqwest::RequestBuilder {
    client.patch(&url)
        .json(m)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bot {}", token))
}
