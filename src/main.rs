const GET_IP_URL: &str = "https://www.cloudflare.com/cdn-cgi/trace";
const API_URL: &str = "https://api.cloudflare.com/client/v4/";

struct Args {
    api_key_file: String,
    ddns_name: String,
}

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

fn main() {
    let args = get_args();
    let api_key = std::fs::read_to_string(&args.api_key_file)
        .expect(&format!("{} not found!", &args.api_key_file));

    let client = reqwest::blocking::Client::new();
    loop {
        let ip = get_ip(&client).expect("Unable to get own IP");
        let zones = get_zones(&client, &api_key).expect("Failed fetching zones");

        for zone in zones.result {
            if args.ddns_name.ends_with(&zone.name) {
                update_or_create_record(&client, &api_key, &zone.id, &args.ddns_name, &ip)
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

fn get_ip(client: &reqwest::blocking::Client) -> Result<String, reqwest::Error> {
    let ip = client.get(GET_IP_URL).send()?.text()?;
    Ok(ip
        .lines()
        .filter(|x| x.starts_with("ip="))
        .next()
        .expect("")
        .replace("ip=", ""))
}

fn get_zones(
    client: &reqwest::blocking::Client,
    api_key: &str,
) -> Result<ZoneResponse, reqwest::Error> {
    Ok(client
        .get(format!("{API_URL}zones/"))
        .bearer_auth(api_key)
        .send()?
        .json()?)
}

fn get_dns_records_of_zone(
    client: &reqwest::blocking::Client,
    api_key: &str,
    zone_id: &str,
) -> Result<DnsResponse, reqwest::Error> {
    Ok(client
        .get(format!("{API_URL}zones/{zone_id}/dns_records"))
        .bearer_auth(api_key)
        .send()?
        .json()?)
}

fn get_args() -> Args {
    let args: Vec<String> = std::env::args().skip(1).collect();

    Args {
        api_key_file: args[0].to_string(),
        ddns_name: args[1].to_string(),
    }
}

fn update_or_create_record(
    client: &reqwest::blocking::Client,
    api_key: &str,
    zone_id: &str,
    ddns_name: &str,
    ip: &str,
) {
    let zone = get_dns_records_of_zone(client, api_key, zone_id).expect("Somehow ID didn't match?");
    let mut request = client.post(format!("{API_URL}zones/{zone_id}/dns_records"));
    let new_record = DnsRecordPayload {
        name: ddns_name.to_string(),
        record_type: "A".to_string(),
        proxied: false,
        ttl: 1,
        content: ip.to_string(),
    };
    for dns_record in zone.result {
        if dns_record.name == ddns_name {
            if dns_record.content == ip {
                return; // There was nothing to update
            }
            request = client.patch(format!(
                "{API_URL}zones/{zone_id}/dns_records/{}",
                dns_record.id
            ));
            break;
        }
    }
    println!(
        "{:?}",
        request
            .bearer_auth(api_key)
            .json(&new_record)
            .send()
            .unwrap()
            .text()
            .unwrap()
    );
}
