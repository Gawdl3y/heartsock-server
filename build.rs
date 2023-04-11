#[cfg(windows)]
extern crate winres;

#[cfg(windows)]
fn main() {
	let mut res = winres::WindowsResource::new();
	res.set_icon("windows-icon.ico");
	res.compile().unwrap();
}

#[cfg(unix)]
fn main() {
	// Nothing special to do here for the time being
}
