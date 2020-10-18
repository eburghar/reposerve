mod args;
mod config;
mod auth;

use crate::{
	args::Opts,
	config::Config,
	auth::TokenAuth
};

use actix_files::Files;
use actix_multipart::Multipart;
use actix_web::{
	App,
	HttpServer,
	HttpResponse,
	Error,
	web,
	post,
	middleware::Logger
};
use futures::{
	StreamExt,
	TryStreamExt,
};
use std::io::Write;
use std::process::Command;
use log::info;

#[post("/upload", wrap="TokenAuth")]
async fn save_file(mut payload: Multipart) -> Result<HttpResponse, Error> {
	// iterate over multipart stream
	while let Ok(Some(mut field)) = payload.try_next().await {
		let content_type = field.content_disposition().unwrap();
		let filename = content_type.get_filename().unwrap();
		let filepath = format!("./tmp/{}", sanitize_filename::sanitize(&filename));

		// File::create is blocking operation, use threadpool
		let mut f = web::block(|| std::fs::File::create(filepath))
			.await
			.unwrap();

		// Field in turn is stream of *Bytes* object
		while let Some(chunk) = field.next().await {
			let data = chunk.unwrap();
			// filesystem operations are blocking, we have to use threadpool
			f = web::block(move || f.write_all(&data).map(|_| f)).await?;
		}
	}
	Ok(HttpResponse::Ok().into())
}

#[post("/webhook/{webhook}", wrap="TokenAuth")]
async fn webhooks(web::Path(webhook): web::Path<String>, config: web::Data<Config>) -> HttpResponse {
	match config.webhooks.get(webhook.as_str()) {
		Some(script) => {
			match Command::new(script).output() {
				Ok(output) => HttpResponse::Ok().body(format!("{} executed with success: {}", script, std::str::from_utf8(&output.stdout).unwrap_or(""))),
				Err(error) => HttpResponse::NotFound().body(format!("failed to execute {}: {}", script, error))
			}
		},
		_ => HttpResponse::NotFound().body("Not found")
	}
}

#[actix_web::main]
async fn serve(config: Config) -> std::io::Result<()> {
	let addr_port = "0.0.0.0:8080";
    std::env::set_var("RUST_LOG", "reposerve=info,actix_web=info");
    env_logger::init();
    info!("listening on {}", addr_port);
    HttpServer::new(move || {
		App::new()
			.wrap(Logger::default())
			.data(config.clone())
			.service(webhooks)
			.service(save_file)
			.service(Files::new("/", ".").show_files_listing())
	})
	.bind(addr_port)?
	.run()
	.await
}

fn main() -> anyhow::Result<()> {
	let opts: Opts = argh::from_env();
	// read yaml config
	let config = Config::read(&opts.config)?;
	serve(config).unwrap();
	Ok(())
}
