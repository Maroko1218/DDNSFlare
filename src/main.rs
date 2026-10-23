const GET_IP_URL: &str = "https://www.cloudflare.com/cdn-cgi/trace";
const API_URL: &str = "https://api.cloudflare.com/client/v4/";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ZoneResponse {
    result: Vec<Zone>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Zone {
    id: String,
    name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct DnsResponse {
    result: Vec<DnsRecord>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct DnsRecord {
    id: String,
    name: String,
    #[serde(rename = "type")]
    record_type: String,
    content: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct DnsRecordPayload {
    #[serde(rename = "type")]
    record_type: String,
    name: String,
    content: String,
    proxied: bool,
    ttl: u64,
}

struct Args {
    api_key_file: String,
    ddns_name: String,
}

enum RuntimeError {
    NoIP(&'static str),
    FailedApiCall(String),
}

impl From<reqwest::Error> for RuntimeError {
    fn from(reqwest_error: reqwest::Error) -> Self {
        RuntimeError::FailedApiCall(reqwest_error.to_string())
    }
}

fn main() -> std::process::ExitCode {
    let Some(args) = get_args() else {
        eprintln!("Usage: <api_key_file> <ddns_name>");
        return std::process::ExitCode::FAILURE;
    };

    // We keep .expect here because the program literally cannot run without the key
    let api_key = std::fs::read_to_string(&args.api_key_file)
        .expect("Cloudflare API key file could not be read")
        .trim()
        .to_string();

    let client = reqwest::blocking::Client::new();

    println!("Starting DDNS update loop for {}...", args.ddns_name);

    loop {
        match update_cycle(&client, &api_key, &args) {
            Ok(_) => (),
            Err(RuntimeError::NoIP(e)) => eprintln!("Error during update cycle: {}", e),
            Err(RuntimeError::FailedApiCall(e)) => eprintln!("Error during update cycle: {}", e),
        }
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

fn update_cycle(
    client: &reqwest::blocking::Client,
    api_key: &str,
    args: &Args,
) -> Result<(), RuntimeError> {
    let ip = get_ip(client)?;
    let zones = get_zones(client, api_key)?;

    for zone in zones.result {
        if args.ddns_name.ends_with(&zone.name) {
            update_or_create_record(client, api_key, &zone.id, &args.ddns_name, &ip)?;
        }
    }
    Ok(())
}

fn get_ip(client: &reqwest::blocking::Client) -> Result<String, RuntimeError> {
    let text = client.get(GET_IP_URL).send()?.text()?;
    let ip_line = text
        .lines()
        .find(|line| line.starts_with("ip="))
        .ok_or(RuntimeError::NoIP(
            "Could not find IP line in Cloudflare trace",
        ))?;

    Ok(ip_line.replace("ip=", "").to_string())
}

fn get_zones(
    client: &reqwest::blocking::Client,
    api_key: &str,
) -> Result<ZoneResponse, RuntimeError> {
    client
        .get(format!("{API_URL}zones/"))
        .bearer_auth(api_key)
        .send()?
        .json()
        .map_err(|_| RuntimeError::FailedApiCall("Couldn't parse json?".to_string()))
}

fn get_dns_records_of_zone(
    client: &reqwest::blocking::Client,
    api_key: &str,
    zone_id: &str,
) -> Result<DnsResponse, RuntimeError> {
    client
        .get(format!("{API_URL}zones/{zone_id}/dns_records"))
        .bearer_auth(api_key)
        .send()?
        .json()
        .map_err(|_| RuntimeError::FailedApiCall("Couldn't parse json?".to_string()))
}

fn get_args() -> Option<Args> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        return None;
    }

    Some(Args {
        api_key_file: args[0].clone(),
        ddns_name: args[1].clone(),
    })
}

fn update_or_create_record(
    client: &reqwest::blocking::Client,
    api_key: &str,
    zone_id: &str,
    ddns_name: &str,
    ip: &str,
) -> Result<(), RuntimeError> {
    let dns_data = get_dns_records_of_zone(client, api_key, zone_id)?;

    let mut request_builder = client.post(format!("{API_URL}zones/{zone_id}/dns_records"));
    let mut is_update = false;

    for dns_record in dns_data.result {
        if dns_record.name == ddns_name && dns_record.record_type == "A" {
            if dns_record.content == ip {
                return Ok(()); // IP matches, no action needed
            }
            // Switch to PATCH if the record already exists
            request_builder = client.patch(format!(
                "{API_URL}zones/{zone_id}/dns_records/{}",
                dns_record.id
            ));
            is_update = true;
            break;
        }
    }

    let payload = DnsRecordPayload {
        name: ddns_name.to_string(),
        record_type: "A".to_string(),
        proxied: false,
        ttl: 1, // '1' is Auto in Cloudflare
        content: ip.to_string(),
    };

    let response = request_builder.bearer_auth(api_key).json(&payload).send()?;

    if response.status().is_success() {
        let action = if is_update { "Updated" } else { "Created" };
        println!("{} record for {} to {}", action, ddns_name, ip);
    } else {
        eprintln!(
            "Failed to update DNS: {}",
            response.text().map_err(|_| RuntimeError::FailedApiCall(
                "Unable to read response body".to_string()
            ))?
        );
    }

    Ok(())
}
