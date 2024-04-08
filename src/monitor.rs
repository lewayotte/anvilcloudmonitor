extern crate sysinfo;
extern crate hostname;
extern crate reqwest;
extern crate serde_json;

use std::process::Command;
use std::collections::HashMap;
use sysinfo::{ Disks, System };
use regex::Regex;
use serde_json::{Value, json};
use reqwest::Client;
use std::path::Path;

#[tokio::main]
async fn main() {
    let ansi_escape = Regex::new(r"\x1B\[([0-9]{1,2}(;[0-9]{1,2})?)?[m|K]").unwrap();
    let mut system = System::new_all();
    system.refresh_all();

    let mut payload: HashMap<String, Value> = HashMap::new();
    if let Ok( host ) = hostname::get() {
        payload.insert( "hostname".to_string(), json!( host.to_string_lossy().into_owned() ) );
    } else {
        payload.insert( "hostname".to_string(),json!( "UNKNOWN".to_string() ) );
    }

    let disks = Disks::new_with_refreshed_list();
    let root_path = Path::new("/");
    for disk in &disks {
        if disk.mount_point() == root_path {
            let total_space = disk.total_space();
            let available_space = disk.available_space();
            let used_space = total_space - available_space;

            payload.insert( "disk-used".to_string(),  json!(used_space.to_string() ) );
            payload.insert( "disk-avail".to_string(), json!(available_space.to_string() ) );
            payload.insert( "disk-total".to_string(), json!(total_space.to_string() ) );
        }
    }

    let cpus = system.cpus().len();
    payload.insert( "cpus".to_string(),  json!(cpus.to_string() ) );

    if let Ok(output) = Command::new("/bin/sh")
        .arg("/root/bin/Domain.sh")
        .output() {
        // Check if the command was successful
        if output.status.success() {
            payload.insert( "domain".to_string(), json!( String::from_utf8(output.stdout).expect("UNKNOWN").trim().to_string() ) );
        } else {
            payload.insert( "domain".to_string(), json!( "UNKNOWN".to_string() ) );
        }
    } else {
        eprintln!("Failed to execute command");
    }

    if let Ok(output) = Command::new("/bin/sh")
        .arg("/root/bin/InstanceType.sh")
        .output() {
        // Check if the command was successful
        if output.status.success() {
            payload.insert( "instance-type".to_string(), json!( String::from_utf8(output.stdout).expect("UNKNOWN").trim().to_string() ) );
        } else {
            payload.insert( "instance-type".to_string(), json!( "UNKNOWN".to_string() ) );
        }
    } else {
        eprintln!("Failed to execute command");
    }

    if let Ok(output) = Command::new("/bin/sh")
        .arg("/root/bin/NetworkOutBytes.sh")
        .output() {
            // Check if the command was successful
            if output.status.success() {
                payload.insert( "bandwidth".to_string(), json!( String::from_utf8(output.stdout).expect("UNKNOWN").trim().to_string() ) );
            } else {
                payload.insert( "bandwidth".to_string(), json!( "0".to_string() ) );
            }
    } else {
        eprintln!("Failed to execute command");
    }

    if let Ok(output) = Command::new("/bin/sh")
        .arg("/root/bin/MySQLDatabaseSize.sh")
        .output() {
            // Check if the command was successful
            if output.status.success() {
                payload.insert( "db-total-size".to_string(), json!( String::from_utf8(output.stdout).expect("UNKNOWN").trim().to_string() ) );
            } else {
                payload.insert( "db-total-size".to_string(), json!( "0".to_string() ) );
            }
    } else {
        eprintln!("Failed to execute command");
    }

    if let Ok(output) = Command::new("/usr/local/bin/wo")
        .arg("site")
        .arg("list")
        .output() {
        if output.status.success() {
            let sites_output = String::from_utf8(output.stdout).expect("UNKNOWN");
            let sites = sites_output.lines();
            let mut disk_usage = String::new();
            let mut site_db_size = String::new();

            let mut site_payload: HashMap<String, Value> = HashMap::new();
            for site in sites {
                let clean_site = ansi_escape.replace_all(site, "");

                let mut site_path = "/var/www/".to_string();
                site_path.push_str(&clean_site);
                
                if let Ok(du_output) = Command::new("du")
                    .arg("-s")
                    .arg(site_path)
                    .output() {

                    disk_usage = String::from_utf8(du_output.stdout).expect("0");
                    disk_usage = disk_usage.trim().split_whitespace().next().unwrap().parse().unwrap();
                }

                if let Ok(site_db_output) = Command::new("/bin/sh")
                    .arg("/root/bin/WordPressDatabaseSize.sh")
                    .arg(clean_site.to_string())
                    .output() {

                    site_db_size = String::from_utf8(site_db_output.stdout).expect("0");
                }

                let mut site_stats: HashMap<String, String> = HashMap::new();
                site_stats.insert( "disk-used".to_string(), disk_usage.to_string() );
                site_stats.insert( "db-size".to_string(), site_db_size.to_string() );

                site_payload.insert( clean_site.trim().to_string(), json!( site_stats ) );
            }
            //let json_site_payload: Value = json!( site_payload );
            payload.insert( "sites".to_string(), json!( site_payload ) );
        }
    } else {
        eprintln!("Failed to execute command");
    }

    let _ = send_ping( &payload ).await;
}

//async fn send_ping( payload: &HashMap<String, String> ) {
async fn send_ping(payload: &HashMap<String, Value>) -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("ACM_API_KEY")
        .expect("API_KEY environment variable not set");

    let client = Client::new();
    let response = client.post("https://monitor.anvilcloud.pub/api/server/update")
        .header("X-API-Key", format!("{}", api_key))
        .json(&payload)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Data sent successfully!");
        Ok(())
    } else {
        eprintln!("Failed to send data");
        Err("Failed to send data".into())
    }
}