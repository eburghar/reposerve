mod args;
mod auth;
mod config;
mod directory;

use crate::{args::Opts, auth::TokenAuth, config::Config, directory::directory_listing};

use actix_files::Files;
use actix_multipart::Multipart;
use actix_web::{middleware::Logger, post, web, App, Error, HttpResponse, HttpServer};
use anyhow::Context;
use bytes::{BufMut, BytesMut};
use futures::{StreamExt, TryStreamExt};
use rustls::{
	internal::pemfile::{certs, pkcs8_private_keys, rsa_private_keys},
	NoClientAuth, ServerConfig,
};
use sanitize_filename::sanitize;
use std::{
	env,
	fs::{self, File},
	io::{BufReader, Write},
	process::Command,
};
use tempdir::TempDir;

/// Represents an alpine repository
#[derive(Debug)]
struct ApkInfo {
	version: String,
	repo: String,
	arch: String,
}

impl ApkInfo {
	pub fn new() -> Self {
		ApkInfo {
			version: "edge".to_owned(),
			repo: "main".to_owned(),
			arch: "x86_64".to_owned(),
		}
	}

	pub fn set(&mut self, f: &str, v: String) {
		match f {
			"version" => self.version = v,
			"repo" => self.repo = v,
			"arch" => self.arch = v,
			_ => (),
		}
	}
}

/// upload new archives
#[post("/upload", wrap = "TokenAuth")]
async fn save_file(
	mut payload: Multipart,
	config: web::Data<Config>,
) -> Result<HttpResponse, Error> {
	let temp_dir = TempDir::new("reposerve")?;

	// iterate over multipart stream
	let mut info = ApkInfo::new();
	let mut files = Vec::new();
	while let Ok(Some(mut field)) = payload.try_next().await {
		if let Some(content_type) = field.content_disposition() {
			match content_type.get_name() {
				// save files to tmp dir
				Some("file") => {
					if let Some(filename) = content_type.get_filename() {
						let sane_file = sanitize(&filename);
						let filepath = temp_dir.path().join(&sane_file);
						log::info!("saving {}", filepath.display());
						files.push(sane_file);

						// File::create is blocking operation, use threadpool
						let mut f = web::block(|| File::create(filepath)).await.unwrap();

						// Field in turn is stream of *Bytes* objectr
						while let Some(chunk) = field.next().await {
							let data = chunk.unwrap();
							// filesystem operations are blocking, we have to use threadpool
							f = web::block(move || f.write_all(&data).map(|_| f)).await?;
						}
					}
				}
				// get other parameters for moving the files
				Some(f) if f == "version" || f == "repo" || f == "arch" => {
					let mut data = BytesMut::with_capacity(32);
					// Field in turn is stream of *Bytes* objectr
					while let Some(chunk) = field.next().await {
						data.put(chunk.unwrap());
					}
					info.set(f, std::str::from_utf8(&data).unwrap().to_string());
				}
				_ => (),
			}
		}
	}

	// create dest dir if necessary
	let mut root = config.dir.join(sanitize(&info.version));
	root.push(sanitize(&info.repo));
	root.push(sanitize(&info.arch));
	fs::create_dir_all(&root)?;

	// move files to correct destination when we have all the info
	for file in files {
		let src = temp_dir.path().join(&file);
		let dst = root.join(&file);
		fs::copy(&src, &dst)?;
	}

	// call apk to index all .apk files in root
	let mut apk_args: Vec<String> = [
		"index",
		"-o",
		"APKINDEX.tar.gz",
		"--rewrite-arch",
		&info.arch,
	]
	.iter()
	.map(|s| s.to_string())
	.collect();
	for entry in fs::read_dir(&root)? {
		let entry = entry?;
		let path = entry.path();
		if let Some(ext) = path.extension() {
			let metadata = fs::metadata(&path)?;
			if metadata.is_file() && ext == "apk" {
				apk_args.push(String::from(path.file_name().unwrap().to_str().unwrap()));
			}
		}
	}
	let cmd = Command::new("apk")
		.current_dir(&root)
		.args(&apk_args)
		.output();
	if let Ok(output) = cmd {
		log::info!("{}", std::str::from_utf8(&output.stdout).unwrap_or(""));
	}

	// call abuild-sign to sign generated index
	let cmd = Command::new("abuild-sign")
		.current_dir(&root)
		.args(&["APKINDEX.tar.gz"])
		.output();
	if let Ok(output) = cmd {
		log::info!("{}", std::str::from_utf8(&output.stdout).unwrap_or(""));
	}
	Ok(HttpResponse::Ok().into())
}

#[post("/webhook/{webhook}", wrap = "TokenAuth")]
async fn webhooks(
	web::Path(webhook): web::Path<String>,
	config: web::Data<Config>,
) -> HttpResponse {
	match config.webhooks.get(webhook.as_str()) {
		Some(script) => match Command::new(script).output() {
			Ok(output) => HttpResponse::Ok().body(format!(
				"{} executed with success: {}",
				script,
				std::str::from_utf8(&output.stdout).unwrap_or("")
			)),
			Err(error) => {
				HttpResponse::NotFound().body(format!("failed to execute {}: {}", script, error))
			}
		},
		_ => HttpResponse::NotFound().body("Not found"),
	}
}

async fn serve(config: Config, addr: String) -> anyhow::Result<()> {
	// copy some values before config is moved
	let tls = config.tls;
	let crt = config.crt.clone();
	let key = config.key.clone();

	// build the server
	let server = HttpServer::new(move || {
		App::new()
			.wrap(Logger::default())
			.data(config.clone())
			.service(webhooks)
			.service(save_file)
			.service(
				Files::new("/", &config.dir)
					.show_files_listing()
					.files_listing_renderer(directory_listing),
			)
	});

	// bind to http or https
	let server = if tls {
		// Create tls config
		let mut tls_config = ServerConfig::new(NoClientAuth::new());
		// Parse the certificate and set it in the configuration
		let crt_chain = certs(&mut BufReader::new(
			File::open(&crt).with_context(|| format!("unable to read {:?}", &crt))?,
		))
		.map_err(|_| anyhow::anyhow!("error reading certificate"))?;
		// Parse the key in RSA or PKCS8 format
		let invalid_key = |()| anyhow::anyhow!("invalid key in {:?}", &key);
		let no_key = || anyhow::anyhow!("no key found in {:?}", &key);
		let mut keys = rsa_private_keys(&mut BufReader::new(File::open(&key)?))
			.map_err(invalid_key)
			.and_then(|x| (!x.is_empty()).then(|| x).ok_or(no_key()))
			.or_else(|_| {
				pkcs8_private_keys(&mut BufReader::new(File::open(&key)?))
					.map_err(invalid_key)
					.and_then(|x| (!x.is_empty()).then(|| x).ok_or(no_key()))
			})?;
		tls_config
			.set_single_cert(crt_chain, keys.remove(0))
			.with_context(|| "error setting crt/key pair")?;
		server
			.bind_rustls(&addr, tls_config)
			.with_context(|| format!("unable to bind to https://{}", &addr))?
			.run()
	} else {
		server
			.bind(&addr)
			.with_context(|| format!("unable to bind to http://{}", &addr))?
			.run()
	};

	log::info!(
		"listening on http{}://{}",
		if tls { "s" } else { "" },
		&addr
	);
	server.await?;
	Ok(())
}

fn main() -> anyhow::Result<()> {
	// setup logging
	env_logger::Builder::new()
		.parse_filters(
			&env::var("RUST_LOG".to_owned()).unwrap_or("reposerve=info,actix_web=info".to_owned()),
		)
		.init();

	// read command line options
	let opts: Opts = argh::from_env();
	// read yaml config
	let config = Config::read(&opts.config)?;

	// start actix main loop
	let mut system = actix_web::rt::System::new("main");
	system.block_on::<_, anyhow::Result<()>>(serve(config.clone(), opts.addr.clone()))?;
	Ok(())
}
