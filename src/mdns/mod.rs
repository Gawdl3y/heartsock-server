#[cfg(all(feature = "simple-mdns", feature = "mdns-sd"))]
compile_error!("feature \"simple-mdns\" and feature \"mdns-sd\" cannot be enabled at the same time");

#[cfg(feature = "simple-mdns")]
pub mod simple_mdns;
#[cfg(feature = "simple-mdns")]
pub use self::simple_mdns::{advertise, MdnsError};

#[cfg(feature = "mdns-sd")]
pub mod mdns_sd;
#[cfg(feature = "mdns-sd")]
pub use self::mdns_sd::{advertise, MdnsError};

#[derive(Debug)]
pub struct MdnsService<'a> {
	service_type: &'a str,
	instance_name: &'a str,
}

pub static SERVICE: MdnsService = MdnsService {
	service_type: "_heartsock._tcp.local.",
	instance_name: "❤️🧦",
};
