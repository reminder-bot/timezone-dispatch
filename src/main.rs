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
use std::sync::Arc;
use std::sync::Mutex;


struct ResponseBox {
    id: u32,
    response: u16,
}

impl ResponseBox {

    fn new(id: u32) -> ResponseBox {
        ResponseBox { id: id, response: 200 }
    }

    fn set_response(&mut self, new: u16) {
        self.response = new;
    }
}


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
        let q = mysql_conn.prep_exec("SELECT id, channel, timezone, name, current_name FROM clocks", ()).unwrap();

        let end: Arc<Mutex<Vec<ResponseBox>>> = Arc::new(Mutex::new(vec![]));

        for res in q {
            let (id, channel_id, timezone, channel_name, current_name) = mysql::from_row::<(u32, u64, String, String, String)>(res.unwrap());

            let t: Tz = timezone.parse().unwrap();
            let dt = Utc::now().with_timezone(&t);
            let new_name = dt.format(&channel_name).to_string();

            if new_name != current_name {
                let mut m = HashMap::new();
                m.insert("name", new_name.clone());

                let req = send(format!("{}/channels/{}", URL, channel_id), &m, &token, &req_client);

                let e = end.clone();
                let m = mysql_conn.clone();

                pool.execute(move || {
                    match req.send() {
                        Err(_) => {},

                        Ok(r) => {
                            let mut new = ResponseBox::new(id);
                            new.set_response(r.status().as_u16());

                            if r.status().is_success() {
                                m.prep_exec("UPDATE clocks SET current_name = :n WHERE id = :id", params!{"n" => new_name, "id" => &id}).unwrap();
                            }

                            let mut l = e.lock().unwrap();
                            (*l).push(new);
                        }
                    }
                });
            }
        }

        pool.join();

        let out = end.lock().unwrap();
        // selects for __deletion__
        let selector = out
            .iter().filter(|r| {
                [400, 403, 404].contains(&r.response)
            }).map(|m| format!("{}", m.id) );

        let collected: Vec<String> = selector.collect();

        if !collected.is_empty() {
            mysql_conn.prep_exec(&format!("DELETE FROM clocks WHERE id IN ({})", collected.join(",")), ()).unwrap();
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
