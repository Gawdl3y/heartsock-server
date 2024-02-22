use anyhow::{anyhow, Context, Result};
use clap::{arg, command, Parser};
use std::net::SocketAddr;
use tokio::fs;
use tracing::metadata::LevelFilter;

mod mdns;
mod websocket;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
	/// Socket address to listen on
	#[arg(short, long, default_value_t = SocketAddr::from(([0, 0, 0, 0], 9001)))]
	listen: SocketAddr,

	/// Disables mDNS advertisement
	#[cfg(any(feature = "simple-mdns", feature = "mdns-sd"))]
	#[arg(short, long)]
	disable_mdns: bool,

	/// IP to advertise (via mDNS) for connecting to
	#[cfg(feature = "simple-mdns")]
	#[arg(short, long)]
	advertise_ip: Option<std::net::IpAddr>,

	/// IP to advertise (via mDNS) for connecting to
	#[cfg(feature = "mdns-sd")]
	#[arg(short, long)]
	advertise_ip: Option<std::net::Ipv4Addr>,

	/// Directory to write plain text files in for each data type (HRM, battery)
	#[arg(short = 'D', long)]
	data_dir: Option<std::path::PathBuf>,

	/// Max log level to output
	#[arg(short = 'o', long, default_value_t = LevelFilter::INFO)]
	log_level: LevelFilter,
}

#[tokio::main]
async fn main() -> Result<()> {
	let args = Args::parse();

	// Set up tracing
	tracing_subscriber::fmt().with_max_level(args.log_level).init();

	// Create the data directory if it doesn't exist
	if let Some(data_dir) = &args.data_dir {
		fs::create_dir_all(data_dir)
			.await
			.context("Failed to create data directory")?;
	}

	// Advertise the server via MDNS
	cfg_if::cfg_if! {
		if #[cfg(any(feature = "simple-mdns", feature = "mdns-sd"))] {
			if !args.disable_mdns {
				mdns::advertise(args.listen.port(), args.advertise_ip)
					.await
					.unwrap_or_else(|err| tracing::error!("Unable to advertise via mDNS: {}", err));
			}
		}
	}

	// Run the server
	websocket::run(args.listen, args.data_dir)
		.await
		.map_err(|err| anyhow!(err))
		.with_context(|| format!("Failed to run WebSocket server on {}", args.listen))
}
