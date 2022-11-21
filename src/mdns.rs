use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::net::{IpAddr, Ipv4Addr};

pub fn advertise(port: u16, local_ip: Option<Ipv4Addr>) -> Result<(), String> {
	// Get the local IP if it wasn't provided
	let ip = match local_ip {
		Some(ip) => Ok(ip),
		None => {
			let ip = local_ip_address::local_ip();
			match ip {
				Ok(ip) => {
					tracing::info!("Detected local IP: {}", ip);
					match ip {
						IpAddr::V4(ip4) => Ok(ip4),
						IpAddr::V6(_) => {
							Err("Detected IP is IPv6, which is unsupported for mDNS advertisement.".into())
						}
					}
				}
				Err(err) => Err(format!("Unable to detect local IP: {}", err)),
			}
		}
	}?;

	// Create a daemon
	let mdns = ServiceDaemon::new().expect("Failed to create daemon");

	// Create service info
	let service_type = "_hrmws._tcp.local.";
	let instance_name = "active";
	let hostname = format!("{}.local.", ip);
	let hostname_str = hostname.as_str();
	let service =
		ServiceInfo::new(service_type, instance_name, hostname_str, ip, port, None).map_err(|err| err.to_string())?;

	// Register the service
	mdns.register(service).map_err(|err| err.to_string())

	// let mut discovery = ServiceDiscovery::new("active", "_hrmws._tcp.local", 60).expect("Invalid service name");
	// discovery.add_service_info(SocketAddr::new(ip, port).into())
}
