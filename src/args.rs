use argh::{FromArgs, TopLevelCommand};
use std::path::Path;

#[derive(FromArgs)]
/// Simple Alpine Linux packages server
pub struct Opts {
	/// configuration file (/etc/reposerve.yaml)
	#[argh(option, short = 'c', default = "\"/etc/reposerve.yaml\".to_owned()")]
	pub config: String,

	/// dev mode: enable /webhook and /upload without jwt (false)
	#[argh(switch, short = 'd')]
	pub dev: bool,

	/// more detailed output (false)
	#[argh(switch, short = 'v')]
	pub verbose: bool,

	/// addr:port to bind to (0.0.0.0:8080) without tls
	#[argh(option, short = 'l', default = "\"0.0.0.0:8080\".to_owned()")]
	pub addr: String,

	/// addr:port to bind to (0.0.0.0:8443) when tls is used
	#[argh(option, short = 'L', default = "\"0.0.0.0:8443\".to_owned()")]
	pub addrs: String,

	/// only bind to tls (when tls config is present in configuration file)
	#[argh(switch, short = 'S')]
	pub secure: bool,
}

fn cmd<'a>(default: &'a str, path: &'a str) -> &'a str {
	Path::new(path)
		.file_name()
		.map(|s| s.to_str())
		.flatten()
		.unwrap_or(default)
}

/// copy of argh::from_env to insert command name and version
pub fn from_env<T: TopLevelCommand>() -> T {
	let strings: Vec<String> = std::env::args().collect();
	let cmd = cmd(&strings[0], &strings[0]);
	let strs: Vec<&str> = strings.iter().map(|s| s.as_str()).collect();
	T::from_args(&[cmd], &strs[1..]).unwrap_or_else(|early_exit| {
		println!("{} {}\n", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION"));
		println!("{}", early_exit.output);
		std::process::exit(match early_exit.status {
			Ok(()) => 0,
			Err(()) => 1,
		})
	})
}
