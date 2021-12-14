use argh::{FromArgs, TopLevelCommand};
use std::path::Path;

#[derive(FromArgs)]
/// Extract latest projects archives from a gitlab server
pub struct Opts {
	/// configuration file containing projects and gitlab connection parameters
	#[argh(option, short = 'c', default="\"/etc/reposerve.yaml\".to_owned()")]
	pub config: String,

	/// more detailed output
	#[argh(switch, short = 'v')]
	pub verbose: bool,

	/// addr:port to bind to
	#[argh(option, short = 'a', default="\"0.0.0.0:8080\".to_owned()")]
	pub addr: String,
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
	const NAME: &str = env!("CARGO_BIN_NAME");
	const VERSION: &str = env!("CARGO_PKG_VERSION");
	let strings: Vec<String> = std::env::args().collect();
	let cmd = cmd(&strings[0], &strings[0]);
	let strs: Vec<&str> = strings.iter().map(|s| s.as_str()).collect();
	T::from_args(&[cmd], &strs[1..]).unwrap_or_else(|early_exit| {
		println!("{} {}\n", NAME, VERSION);
		println!("{}", early_exit.output);
		std::process::exit(match early_exit.status {
			Ok(()) => 0,
			Err(()) => 1,
		})
	})
}
