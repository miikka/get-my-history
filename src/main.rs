use std::cmp::Ordering;

use anyhow::Result;
use clap::Parser;
use reqwest::header::HeaderValue;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long, env = "GMH_HOST")]
    host: String,

    #[arg(long, env = "GMH_ACCESS_TOKEN")]
    access_token: String,

    #[arg(short = 'u', long)]
    update_in_place: bool,

    file: Option<String>,
}

fn api_get(args: &Cli, path: &str) -> Result<serde_json::Value> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(format!("{}{}", args.host, path))
        .bearer_auth(&args.access_token)
        .send()?
        .json()?;
    Ok(resp)
}

fn get_account_id(args: &Cli) -> Result<String> {
    let resp = api_get(args, "/api/v1/accounts/verify_credentials")?;
    Ok(resp["id"].as_str().unwrap().to_string())
}

fn parse_link(header: &HeaderValue) -> Option<String> {
    if let Ok(link_str) = header.to_str() {
        for link_part in link_str.split(',') {
            if link_part.contains("rel=\"next\"") {
                if let Some(next_url) = link_part
                    .split(';')
                    .next()
                    .and_then(|s| s.trim().strip_prefix('<'))
                    .and_then(|s| s.strip_suffix('>'))
                {
                    return Some(next_url.into());
                }
            }
        }
    }
    None
}

fn get_statuses(args: &Cli, account_id: &str) -> Result<Vec<serde_json::Value>> {
    let mut has_more = true;
    let mut url = format!("/api/v1/accounts/{}/statuses", account_id);

    let mut result: Vec<serde_json::Value> = vec![];

    while has_more {
        let client = reqwest::blocking::Client::new();
        let resp = client
            .get(format!("{}{}", args.host, &url))
            .bearer_auth(&args.access_token)
            .send()?;

        has_more = false;

        if let Some(next_url) = resp
            .headers()
            .get(reqwest::header::LINK)
            .and_then(parse_link)
        {
            has_more = true;
            url = next_url.replace(&args.host, "");
        }

        let json: serde_json::Value = resp.json()?;
        if let Some(statuses) = json.as_array() {
            result.extend_from_slice(&statuses);
        } else {
            println!("Expected array, got {}", json)
        }
    }

    Ok(result)
}

fn compare_key(key: &str, a: &serde_json::Value, b: &serde_json::Value) -> Ordering {
    a[key].as_str().unwrap().cmp(b[key].as_str().unwrap())
}

fn main() -> Result<()> {
    dotenvy::dotenv()?;

    let args = Cli::parse();
    let account_id = get_account_id(&args)?;

    let mut statuses = get_statuses(&args, &account_id)?;
    statuses.sort_by(|a, b| compare_key("created_at", a, b));
    let output = serde_json::Value::Array(statuses);

    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}
