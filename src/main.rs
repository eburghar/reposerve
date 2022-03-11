mod args;
mod config;
mod directory;
mod upload;
mod webhook;

use crate::{
	args::Opts, config::Config, directory::directory_listing, upload::upload, webhook::webhook,
};

use actix_files::Files;
use actix_token_middleware::middleware::jwtauth::JwtAuth;
use actix_web::{middleware::Logger, web, App, HttpServer};
use anyhow::{anyhow, Context};
use rustls::{
	internal::pemfile::{certs, pkcs8_private_keys, rsa_private_keys},
	NoClientAuth, ServerConfig,
};
use std::{fs::File, io::BufReader};

async fn serve(mut config: Config, addr: String) -> anyhow::Result<()> {
	// set keys from jwks endpoint
	if let Some(ref mut jwt) = config.jwt {
		let _ = jwt
			.set_keys()
			.await
			.map_err(|e| anyhow!("failed to get jkws keys {}", e))?;
	} else {
		log::warn!("no JWT configuration found to protect /webhook and /upload. Use only for development");
	}
	// copy some values before config is moved
	let tls = config.tls.clone();

	// build the server
	let server = HttpServer::new(move || {
		let mut app = App::new().wrap(Logger::default()).data(config.clone());
		// wrap /webhook and /upload if jwt is set
		if let Some(ref jwt) = config.jwt {
			app = app
				.service(
					web::resource("/webhook/{webhook}")
						.wrap(JwtAuth::new(jwt.clone()))
						.route(web::post().to(webhook)),
				)
				.service(
					web::resource("/upload")
						.wrap(JwtAuth::new(jwt.clone()))
						.route(web::post().to(upload)),
				)
		// else dev mode !
		} else {
			app = app
				.service(
					web::resource("/webhook/{webhook}")
						.route(web::post().to(webhook)),
				)
				.service(
					web::resource("/upload")
						.route(web::post().to(upload)),
				)
		}
		app.service(
			Files::new("/", &config.dir)
				.show_files_listing()
				.files_listing_renderer(directory_listing),
		)
	});

	// bind to http or https
	let server = if let Some(ref tls) = tls {
		// Create tls config
		let mut tls_config = ServerConfig::new(NoClientAuth::new());

		// Parse the certificate and set it in the configuration
		let crt_chain = certs(&mut BufReader::new(
			File::open(&tls.crt).with_context(|| format!("unable to read {:?}", &tls.crt))?,
		))
		.map_err(|_| anyhow!("error reading certificate"))?;

		// Parse the key in RSA or PKCS8 format
		let invalid_key = |_| anyhow!("invalid key in {:?}", &tls.key);
		let no_key = || anyhow!("no key found in {:?}", &tls.key);
		let mut keys = rsa_private_keys(&mut BufReader::new(File::open(&tls.key)?))
			.map_err(invalid_key)
			.and_then(|x| (!x.is_empty()).then(|| x).ok_or_else(no_key))
			.or_else(|_| {
				pkcs8_private_keys(&mut BufReader::new(File::open(&tls.key)?))
					.map_err(invalid_key)
					.and_then(|x| (!x.is_empty()).then(|| x).ok_or_else(no_key))
			})?;
		tls_config
			.set_single_cert(crt_chain, keys.remove(0))
			.with_context(|| "error setting crt/key pair")?;
		server
			.bind_rustls(&addr, tls_config)
			.with_context(|| format!("unable to bind to https://{}", &addr))?
			.run()
	} else {
		log::warn!("TLS is not activated. Use only for development");
		server
			.bind(&addr)
			.with_context(|| format!("unable to bind to http://{}", &addr))?
			.run()
	};

	log::info!(
		"listening on http{}://{}",
		if tls.is_some() { "s" } else { "" },
		&addr
	);
	server.await?;
	Ok(())
}

fn main() -> anyhow::Result<()> {
	// read command line options
	let opts: Opts = args::from_env();

	// setup logging
	env_logger::init_from_env(
		env_logger::Env::new()
			.default_filter_or("reposerve=info,actix_web=info")
			.default_write_style_or("auto"),
	);
	log::info!("{} v{}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION"));

	// read yaml config
	let config = Config::read(&opts.config)?;

	// start actix main loop
	let mut system = actix_web::rt::System::new("main");
	system.block_on::<_, anyhow::Result<()>>(serve(config, opts.addr))?;
	Ok(())
}
