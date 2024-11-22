use std::{cmp::Ordering, fs::File, io::BufWriter};

use anyhow::{Context, Result};
use clap::Parser;
use reqwest::header::HeaderValue;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long, env = "GMH_HOST")]
    host: String,

    #[arg(long, env = "GMH_ACCESS_TOKEN")]
    access_token: String,

    #[arg(long, env = "GMH_ACCOUNT_ID")]
    account_id: Option<String>,

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
        .error_for_status()?;
    Ok(resp.json()?)
}

fn get_account_id(args: &Cli) -> Result<String> {
    let resp = api_get(args, "/api/v1/accounts/verify_credentials")?;
    Ok(resp["id"].as_str().unwrap().to_string())
}

fn parse_link(header: &HeaderValue, dir: &str) -> Option<String> {
    let rel = format!("rel=\"{}\"", dir);
    if let Ok(link_str) = header.to_str() {
        log::info!("Link str: {}", link_str);
        for link_part in link_str.split(',') {
            if link_part.contains(&rel) {
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

fn get_statuses(
    args: &Cli,
    account_id: &str,
    min_id: Option<&str>,
) -> Result<Vec<serde_json::Value>> {
    let mut has_more = true;

    let mut url = format!("/api/v1/accounts/{}/statuses", account_id);
    let mut params = vec![];

    // The direction of the links in linkes header to follow is a bit odd. If you get all the statuses
    // from the beginning, "next" goes towards newer statuses - but if you use `since_id`, "prev" goes
    // towards newer statuses?!
    let mut dir = "next";
    if let Some(min_id) = min_id {
        params.push(("since_id", min_id));
        dir = "prev";
    }

    let mut result: Vec<serde_json::Value> = vec![];

    while has_more {
        let client = reqwest::blocking::Client::new();
        let full_url = format!("{}{}", args.host, &url);
        log::info!("getting {}", full_url);
        let resp = client
            .get(&full_url)
            .query(&params)
            .bearer_auth(&args.access_token)
            .send()?
            .error_for_status()?;

        has_more = false;

        if let Some(next_url) = resp
            .headers()
            .get(reqwest::header::LINK)
            .and_then(|x| parse_link(x, dir))
        {
            has_more = true;
            url = next_url.replace(&args.host, "");
            params.clear();
        }

        let json: serde_json::Value = resp.json()?;
        result.extend_from_slice(json.as_array().context("expected JSON array")?);
    }

    Ok(result)
}

fn compare_key(key: &str, a: &serde_json::Value, b: &serde_json::Value) -> Ordering {
    a[key].as_str().unwrap().cmp(b[key].as_str().unwrap())
}

fn main() -> Result<()> {
    let _ = dotenvy::dotenv();

    env_logger::init();

    let args = Cli::parse();
    let account_id = if let Some(ref accound_id) = args.account_id {
        accound_id
    } else {
        &get_account_id(&args)?
    };

    let mut statuses: Vec<serde_json::Value> = vec![];

    let max_id = if let Some(ref filename) = args.file {
        // TODO(miikka) Give a good error message if args.file is not set.
        let f = File::open(filename)?;
        let v: serde_json::Value = serde_json::from_reader(f)?;
        statuses = v.as_array().unwrap().clone();
        statuses.sort_by(|a, b| compare_key("created_at", b, a));
        statuses
            .first()
            .map(|s| s["id"].as_str().unwrap().to_owned())
    } else {
        None
    };

    log::info!("max ID: {:?}", max_id);

    statuses.extend(get_statuses(&args, account_id, max_id.as_deref())?);
    statuses.sort_by(|a, b| compare_key("created_at", a, b));
    let output = serde_json::Value::Array(statuses);

    let writer: Box<dyn std::io::Write> = if args.update_in_place {
        Box::new(BufWriter::new(File::create(args.file.unwrap())?))
    } else {
        Box::new(std::io::stdout())
    };

    serde_json::to_writer_pretty(writer, &output)?;

    Ok(())
}
