use actix_files::Directory;
use actix_web::{dev::ServiceResponse, HttpRequest, HttpResponse};
use chrono::{offset::Utc, DateTime};
use percent_encoding::{utf8_percent_encode, CONTROLS};
use size_format::SizeFormatterBinary;
use std::{fmt::Write, io, path::Path};
use v_htmlescape::escape as escape_html_entity;

struct DirEntry {
	name: String,
	url: String,
	dt: DateTime<Utc>,
	len: u64,
	is_dir: bool,
}

impl DirEntry {
	fn new(entry: std::fs::DirEntry, dir: &Directory, base: &Path) -> Option<Self> {
		if let Ok(metadata) = entry.metadata() {
			if let Ok(p) = entry.path().strip_prefix(&dir.path) {
				return Some(DirEntry {
					name: escape_html_entity(&entry.file_name().to_string_lossy()).to_string(),
					url: utf8_percent_encode(&base.join(p).to_string_lossy(), CONTROLS).to_string(),
					dt: metadata.modified().unwrap().into(),
					len: metadata.len(),
					is_dir: metadata.is_dir(),
				})
			};
		}
		None
	}
}

pub fn directory_listing(dir: &Directory, req: &HttpRequest) -> Result<ServiceResponse, io::Error> {
	let index_of = format!("Index of {}", req.path());
	let mut body = String::new();
	let base = Path::new(req.path());

	let mut entries: Vec<DirEntry> = dir
		.path
		.read_dir()?
		.filter(|e| dir.is_visible(e))
		.filter_map(Result::ok)
		.filter_map(|e| DirEntry::new(e, dir, base))
		.collect();

	entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));

	if dir.path != dir.base {
		let p = base.parent().unwrap_or_else(|| Path::new(""));
		let _ = write!(
			body,
			"<li><a href=\"{}\">../</a></li>",
			utf8_percent_encode(&p.to_string_lossy(), CONTROLS)
		);
	}

	for entry in entries {
		if entry.is_dir {
			let _ = write!(
				body,
				"<li><span><a href=\"{}\">{}/</a></span></li>",
				entry.url, entry.name
			);
		} else {
			let _ = write!(
				body,
				"<li><span><a href=\"{}\">{}</a></span><span>{}</span><span>{}B</span></li>",
				entry.url,
				entry.name,
				entry.dt.format("%Y/%m/%d %T"),
				SizeFormatterBinary::new(entry.len)
			);
		}
	}

	let html = format!(
		"<html>\
         <head>
         <style>
         ul {{ list-style: none; }}
         li {{ display: flex; }}
         li:hover {{ background-color: lightgray; }}
         li>span {{ width: 33.333333%; }}
         </style>
         <title>{}</title></head>\
         <body><h1>{}</h1>\
         <hr>
         <ul>\
         {}\
         </ul><hr></body>\n</html>",
		index_of, index_of, body
	);
	Ok(ServiceResponse::new(
		req.clone(),
		HttpResponse::Ok()
			.content_type("text/html; charset=utf-8")
			.body(html),
	))
}
