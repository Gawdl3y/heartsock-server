use crate::mdns::{MdnsService, SERVICE};
use simple_mdns::async_discovery::ServiceDiscovery;
use std::net::{IpAddr, SocketAddr};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MdnsError {
	#[error("mDNS service error: {0}")]
	MdnsDaemon(#[from] simple_mdns::SimpleMdnsError),
	#[error("Unable to detect local IP: {0}")]
	DetectionUnknown(#[from] local_ip_address::Error),
}

pub async fn advertise(port: u16, local_ip: Option<IpAddr>) -> Result<(), MdnsError> {
	// Get the local IP if it wasn't provided
	let ip = match local_ip {
		Some(ip) => Ok(ip),
		None => get_local_ip(),
	}?;

	let MdnsService {
		service_type,
		instance_name,
	} = SERVICE;

	tracing::info!(
		"Starting mDNS service advertisement of \"{}\".{} as {}:{}",
		instance_name,
		service_type,
		ip,
		port
	);

	let mut discovery = ServiceDiscovery::new(instance_name, service_type, 60)?;
	discovery
		.add_service_info(SocketAddr::new(ip, port).into())
		.await
		.map_err(|err| err.into())
}

fn get_local_ip() -> Result<IpAddr, MdnsError> {
	match local_ip_address::local_ip() {
		Ok(ip) => {
			tracing::info!("Detected local IP: {}", ip);
			Ok(ip)
		}
		Err(err) => Err(err.into()),
	}
}
