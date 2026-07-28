use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use camellia_remote_protocol::{
    anyhow::{anyhow, Context as _},
    bail, ResultType,
};
use dns_lookup::{lookup_addr, lookup_host};
use sodiumoxide::crypto::sign;
use std::{
    env,
    net::{IpAddr, TcpStream, ToSocketAddrs as _},
    process, str,
    time::{Duration, Instant},
};

const HEALTHCHECK_TOTAL_TIMEOUT: Duration = Duration::from_secs(3);
const HEALTHCHECK_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(1);
const HEALTHCHECK_MAX_ENDPOINTS: usize = 8;
const DOCTOR_CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

fn print_help() {
    println!(
        "Usage:
    camellia-remote-utils [command]\n
Available Commands:
    genkeypair                                   Generate a new keypair
    validatekeypair [public key] [secret key]    Validate an existing keypair
    doctor [remote-server]                       Check for server connection problems
    healthcheck [host:port]                      Exit non-zero unless TCP is reachable"
    );
}

fn error_then_help(msg: &str) -> ! {
    eprintln!("ERROR: {msg}\n");
    print_help();
    process::exit(2);
}

fn gen_keypair() {
    let (pk, sk) = sign::gen_keypair();
    let public_key = BASE64.encode(pk);
    let secret_key = BASE64.encode(sk);
    println!("Public Key:  {public_key}");
    println!("Secret Key:  {secret_key}");
}

fn validate_keypair(pk: &str, sk: &str) -> ResultType<()> {
    let secret_key_bytes = BASE64.decode(sk).context("Invalid secret key encoding")?;
    let secret_key = sign::SecretKey::from_slice(&secret_key_bytes)
        .ok_or_else(|| anyhow!("Invalid secret key length"))?;
    let public_key_bytes = BASE64.decode(pk).context("Invalid public key encoding")?;
    let public_key = sign::PublicKey::from_slice(&public_key_bytes)
        .ok_or_else(|| anyhow!("Invalid public key length"))?;

    let random_data_to_test = b"This is meh.";
    let signed_data = sign::sign(random_data_to_test, &secret_key);
    let verified_data =
        sign::verify(&signed_data, &public_key).map_err(|_| anyhow!("Key pair is INVALID"))?;

    if random_data_to_test != &verified_data[..] {
        bail!("Key pair is INVALID");
    }

    Ok(())
}

fn doctor_tcp(address: IpAddr, port: u16, description: &str) -> bool {
    let start = std::time::Instant::now();
    let endpoint = std::net::SocketAddr::new(address, port);
    match TcpStream::connect_timeout(&endpoint, DOCTOR_CONNECT_TIMEOUT) {
        Ok(_) => {
            println!(
                "TCP Port {} ({}): OK in {} ms",
                port,
                description,
                start.elapsed().as_millis()
            );
            true
        }
        Err(err) => {
            println!("TCP Port {port} ({description}): ERROR ({err})");
            false
        }
    }
}

fn doctor_ip(server_ip_address: IpAddr, server_address: Option<&str>) -> bool {
    println!("\nChecking IP address: {server_ip_address}");
    println!("Is IPV4: {}", server_ip_address.is_ipv4());
    println!("Is IPV6: {}", server_ip_address.is_ipv6());

    // reverse dns lookup
    // TODO: (check) doesn't seem to do reverse lookup on OSX...
    match lookup_addr(&server_ip_address) {
        Ok(reverse) => {
            if let Some(server_address) = server_address {
                if reverse == server_address {
                    println!("Reverse DNS lookup: '{reverse}' MATCHES server address");
                } else {
                    println!(
                        "Reverse DNS lookup: '{reverse}' DOESN'T MATCH server address '{server_address}'"
                    );
                }
            }
        }
        Err(err) => println!("Reverse DNS lookup: unavailable ({err})"),
    }

    let checks = [
        (21115, "identity NAT test"),
        (21116, "identity rendezvous"),
        (21117, "relay"),
        (21118, "identity WebSocket"),
        (21119, "relay WebSocket"),
    ];
    let mut healthy = true;
    for (port, description) in checks {
        healthy &= doctor_tcp(server_ip_address, port, description);
    }
    println!("UDP Port 21116 (identity rendezvous): not actively probed");
    healthy
}

fn doctor(server_address_unclean: &str) -> ResultType<()> {
    let server_address3 = server_address_unclean.trim();
    let server_address2 = server_address3.to_lowercase();
    let server_address = server_address2.as_str();
    println!("Checking server:  {server_address}\n");
    if let Ok(server_ipaddr) = server_address.parse::<IpAddr>() {
        // user requested an ip address
        if !doctor_ip(server_ipaddr, None) {
            bail!("One or more Camellia server TCP listeners are unreachable");
        }
    } else {
        // the passed string is not an ip address
        let ips: Vec<std::net::IpAddr> = lookup_host(server_address)?;
        if ips.is_empty() {
            bail!("No IP addresses resolved for {server_address}");
        }
        println!("Found {} IP addresses: ", ips.len());

        ips.iter().for_each(|ip| println!(" - {ip}"));

        let mut healthy = true;
        for ip in ips {
            healthy &= doctor_ip(ip, Some(server_address));
        }
        if !healthy {
            bail!("One or more Camellia server TCP listeners are unreachable");
        }
    }
    Ok(())
}

fn healthcheck(endpoint: &str) -> ResultType<()> {
    let endpoints = endpoint
        .to_socket_addrs()
        .with_context(|| format!("Invalid healthcheck endpoint: {endpoint}"))?;
    let started_at = Instant::now();
    for endpoint in endpoints.take(HEALTHCHECK_MAX_ENDPOINTS) {
        let Some(remaining) = HEALTHCHECK_TOTAL_TIMEOUT.checked_sub(started_at.elapsed()) else {
            break;
        };
        if TcpStream::connect_timeout(&endpoint, remaining.min(HEALTHCHECK_ATTEMPT_TIMEOUT)).is_ok()
        {
            return Ok(());
        }
    }
    bail!("TCP healthcheck failed: {endpoint}");
}

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() <= 1 {
        print_help();
        return;
    }

    let command = args[1].to_lowercase();
    match command.as_str() {
        "help" | "-h" | "--help" => print_help(),
        "-v" | "--version" => println!("camellia-remote-utils {}", env!("CARGO_PKG_VERSION")),
        "genkeypair" => gen_keypair(),
        "validatekeypair" => {
            if args.len() <= 3 {
                error_then_help("You must supply both the public and the secret key");
            }
            let res = validate_keypair(args[2].as_str(), args[3].as_str());
            if let Err(e) = res {
                println!("{e}");
                process::exit(0x0001);
            }
            println!("Key pair is VALID");
        }
        "doctor" => {
            if args.len() <= 2 {
                error_then_help("You must supply the Camellia Remote server address");
            }
            if let Err(err) = doctor(args[2].as_str()) {
                eprintln!("{err}");
                process::exit(0x0001);
            }
        }
        "healthcheck" => {
            if args.len() <= 2 {
                error_then_help("You must supply a host:port endpoint");
            }
            if let Err(err) = healthcheck(args[2].as_str()) {
                eprintln!("{err}");
                process::exit(0x0001);
            }
        }
        _ => error_then_help("Unknown command"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn healthcheck_accepts_reachable_tcp_endpoint() -> ResultType<()> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
        let endpoint = listener.local_addr()?.to_string();

        healthcheck(&endpoint)
    }

    #[test]
    fn healthcheck_rejects_invalid_endpoint() {
        assert!(healthcheck("127.0.0.1:not-a-port").is_err());
    }
}
