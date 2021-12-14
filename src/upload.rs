use crate::config::Config;

use actix_multipart::Multipart;
use actix_web::{web, Error, HttpResponse};
use bytes::{BufMut, BytesMut};
use futures::{StreamExt, TryStreamExt};
use sanitize_filename::sanitize;
use std::{
	fs::{self, File},
	io::Write,
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
pub(crate) async fn upload(
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
	let cmd = Command::new("/sbin/apk")
		.current_dir(&root)
		.args(&apk_args)
		.output();
	match cmd {
		Ok(output) => log::info!("apk: {}", std::str::from_utf8(&output.stdout).unwrap_or("")),
		Err(e) => log::error!("Error when running apk: {:?}", e),
	}

	// call abuild-sign to sign generated index
	let cmd = Command::new("/usr/bin/abuild-sign")
		.current_dir(&root)
		.args(&["APKINDEX.tar.gz"])
		.output();
	match cmd {
		Ok(output) => log::info!(
			"abuild-sign: {}",
			std::str::from_utf8(&output.stdout).unwrap_or("")
		),
		Err(e) => log::error!("Error when running abuild-sign: {:?}", e),
	}
	Ok(HttpResponse::Ok().into())
}
