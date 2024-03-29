mod args;
mod config;
mod directory;
mod upload;
mod webhook;

use crate::{
	args::Opts, config::Config, directory::directory_listing, upload::upload, webhook::webhook,
};

use actix_files::Files;
use actix_schemeredirect_middleware::middleware::SchemeRedirect;
use actix_token_middleware::middleware::jwtauth::JwtAuth;
use actix_web::{
	middleware::Logger,
	web::{self, Data},
	App, HttpServer,
};
use anyhow::{anyhow, Context};
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use std::{fs::File, io::BufReader};

async fn serve(
	mut config: Config,
	secure: bool,
	addr: String,
	addrs: String,
	dev: bool,
) -> anyhow::Result<()> {
	// set keys from jwks endpoint
	if let Some(ref mut jwt) = config.jwt {
		jwt.set_keys()
			.await
			.map_err(|e| anyhow!("failed to get jkws keys {}", e))?;
	} else if dev {
		log::warn!(
			"no JWT configuration found to protect /webhook and /upload. Use only for development"
		);
	}
	// copy some values before config is moved
	let tls = config.tls.clone();

	// build the server
	let server = HttpServer::new(move || {
		let tls = config.tls.as_ref();
		// redirect only in dual protocoal and if a redirect config is provided
		let redirect_service =
			if let Some(redirect) = tls.filter(|_| !secure).and_then(|c| c.redirect.clone()) {
				log::info!(
					"redirect to https:{:?} for {:?} protocol(s)",
					redirect.port.unwrap_or(443),
					redirect.protocols
				);
				SchemeRedirect::new(
					redirect.protocols,
					tls.and_then(|o| o.hsts.clone()),
					redirect.port,
				)
			} else {
				// no redirection by default
				SchemeRedirect::default()
			};

		let mut app = App::new()
			.wrap(Logger::default())
			.wrap(redirect_service)
			.app_data(Data::new(config.clone()));

		// wrap /webhook and /upload inside JwtAuth if jwt is set
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
		// otherwise mount /webhook and /upload with no protection only if dev mode is activated
		} else if dev {
			app = app
				.service(web::resource("/webhook/{webhook}").route(web::post().to(webhook)))
				.service(web::resource("/upload").route(web::post().to(upload)))
		}
		app.service(
			Files::new("/", &config.dir)
				.show_files_listing()
				.files_listing_renderer(directory_listing),
		)
	});

	// bind to https if tls configuration is present
	let mut server = if let Some(tls) = &tls {
		// information about hsts
		if let Some(hsts) = &tls.hsts {
			log::info!("send HSTS header: {:?}", hsts.to_string());
		}
		// Create tls config
		let config = ServerConfig::builder()
			.with_safe_defaults()
			.with_no_client_auth();

		let crt_chain = certs(&mut BufReader::new(
			File::open(&tls.crt).with_context(|| format!("unable to read {:?}", &tls.crt))?,
		))
		.map_err(|_| anyhow!("error reading certificate"))?
		.into_iter()
		.map(Certificate)
		.collect();

		// Parse the key in RSA or PKCS8 format
		let invalid_key = |_| anyhow!("invalid key in {:?}", &tls.key);
		let no_key = || anyhow!("no key found in {:?}", &tls.key);
		let mut keys: Vec<PrivateKey> =
			rsa_private_keys(&mut BufReader::new(File::open(&tls.key)?))
				.map_err(invalid_key)
				// return an error if there is no key
				.and_then(|x| (!x.is_empty()).then_some(x).ok_or_else(no_key))
				.or_else(|_| {
					pkcs8_private_keys(&mut BufReader::new(File::open(&tls.key)?))
						.map_err(invalid_key)
						// return an error if there is no key
						.and_then(|x| (!x.is_empty()).then_some(x).ok_or_else(no_key))
				})?
				.into_iter()
				.map(PrivateKey)
				.collect();
		let tls_config = config
			.with_single_cert(crt_chain, keys.swap_remove(0))
			.with_context(|| "error setting crt/key pair")?;
		server
			.bind_rustls(&addrs, tls_config)
			.with_context(|| format!("unable to bind to https://{}", &addrs))?
	} else {
		log::warn!("TLS is not activated. Use only for development");
		server
	};

	// bind to http if secure is false (default)
	server = if !secure {
		server
			.bind(&addr)
			.with_context(|| format!("unable to bind to http://{}", &addr))?
	} else {
		server
	};

	if !secure {
		log::info!("listening on http://{}", &addr);
	}
	if tls.is_some() {
		log::info!("listening on https://{}", &addrs);
	}

	server.run().await?;
	Ok(())
}

fn main() -> anyhow::Result<()> {
	// read command line options
	let opts: Opts = args::from_env();

	// setup logging
	env_logger::init_from_env(
		env_logger::Env::new()
			.default_filter_or("reposerve=info,actix_web=info,actix_schemeredirect_middleware=info,actix-token-middleware=info")
			.default_write_style_or("auto"),
	);
	log::info!("{} v{}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION"));

	// read yaml config
	let config = Config::read(&opts.config)?;

	// start actix main loop
	let system = actix_web::rt::System::new();
	system.block_on(serve(config, opts.secure, opts.addr, opts.addrs, opts.dev))?;
	Ok(())
}
