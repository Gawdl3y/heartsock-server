use std::net::{SocketAddr, IpAddr};
use simple_mdns::ServiceDiscovery;

pub fn advertise(port: u16, local_ip: Option<IpAddr>) -> Result<(), String> {
	// Get the local IP if it wasn't provided
	let ip = match local_ip {
		Some(ip) => Ok(ip),
		None => {
			let ip = local_ip_address::local_ip();
			match ip {
				Ok(ip) => {
					tracing::info!("Detected local IP: {}", ip);
					Ok(ip)
				},
				Err(err) => Err(format!("Unable to detect local IP: {}", err))
			}
		}
	}?;

	let mut discovery = ServiceDiscovery::new("active", "_hrmws._tcp.local", 60).expect("Invalid service name");
	discovery.add_service_info(SocketAddr::new(ip, port).into()).map_err(|err| err.to_string())
}
