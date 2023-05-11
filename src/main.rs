use serde::Deserialize;
use serde_json;
use std::{env, error::Error, fs, io::Write, path};

#[derive(Deserialize, Debug, Clone)]
struct AddrInfo {
    scope: String,
    #[serde(default)]
    dynamic: bool,
    local: String,
    prefixlen: u8,
}

#[derive(Deserialize, Debug, Clone)]
struct Interface {
    ifname: String,
    link_type: String,
    address: Option<String>,
    addr_info: Vec<AddrInfo>,
}

#[derive(Deserialize, Debug, Clone)]
struct Route {
    protocol: String,
    dev: String,
    dst: String,
    gateway: Option<String>,
}

fn filter_interfaces(interfaces: &Vec<Interface>) -> Vec<Interface> {
    let mut filtered = vec![];
    for interface in interfaces {
        if interface.link_type == "loopback" || interface.address.is_none() {
            // We need a mac address to match devices reliable
            continue;
        }
        let mut addr_infos = vec![];
        let mut has_dynamic_address = false;
        for addr_info in interface.addr_info.clone() {
            if addr_info.scope == "link" {
                // no link-local ipv4/ipv6
                continue;
            }
            if addr_info.dynamic {
                // do not explicitly configure addresses from dhcp or router advertisment
                has_dynamic_address = true;
                continue;
            }
            addr_infos.push(addr_info)
        }
        if !addr_infos.is_empty() || has_dynamic_address {
            let mut interface: Interface = interface.clone();
            interface.addr_info = addr_infos;
            filtered.push(interface)
        }
    }
    filtered
}

fn filter_routes(routes: &Vec<Route>) -> Vec<Route> {
    routes
        .iter()
        // Filter out routes set by addresses with subnets, dhcp and router advertisment
        .filter(|route| !["dhcp", "kernel", "ra"].contains(&route.protocol.as_str()))
        .map(|route| route.to_owned())
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if let [_, addresses_file, ipv4_routes_file, ipv6_routes_file, networkd_dir] = args.as_slice() {
        let interfaces: Vec<Interface> = serde_json::from_reader(fs::File::open(addresses_file)?)?;
        let ipv4_routes: Vec<Route> = serde_json::from_reader(fs::File::open(ipv4_routes_file)?)?;
        let ipv6_routes: Vec<Route> = serde_json::from_reader(fs::File::open(ipv6_routes_file)?)?;

        let interfaces = filter_interfaces(&interfaces);
        let routes = filter_routes(&[ipv4_routes, ipv6_routes].concat());

        let networkd_dir = path::Path::new(networkd_dir);
        fs::create_dir_all(networkd_dir)?;
        for interface in interfaces {
            let filename = networkd_dir
                .join(&interface.ifname)
                .with_extension("network");
            let mut file = fs::File::create(filename)?;
            writeln!(&mut file, "[Match]")?;
            writeln!(
                &mut file,
                "MACAddress = {}",
                interface
                    .address
                    .expect("MAC address in filtered interface")
            )?;
            writeln!(&mut file, "\n[Network]")?;
            writeln!(&mut file, "DHCP = yes")?;
            writeln!(&mut file, "IPv6AcceptRA = yes")?;
            for addr_info in interface.addr_info {
                writeln!(
                    &mut file,
                    "Address = {}/{}",
                    addr_info.local, addr_info.prefixlen
                )?;
            }
            for route in routes.iter() {
                if route.dev != interface.ifname {
                    // can be skipped for default routes
                    continue;
                }
                writeln!(&mut file, "\n[Route]")?;
                if route.dst != "default" {
                    writeln!(&mut file, "Destination = {0}", route.dst)?;
                }
                if let Some(gateway) = &route.gateway {
                    writeln!(&mut file, "Gateway = {0}", gateway)?;
                }
                writeln!(&mut file, "DHCP = yes")?;
            }
        }
    } else {
        eprintln!("USAGE: addresses routes-v4 routes-v6 networkd-directory");
        std::process::exit(1);
    }
    Ok(())
}
