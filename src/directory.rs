use actix_web::{HttpRequest, HttpResponse, dev::ServiceResponse};
use actix_files::{Directory};
use std::{path::Path, fmt::Write, io};
use percent_encoding::{utf8_percent_encode, CONTROLS};
use v_htmlescape::escape as escape_html_entity;
use chrono::{offset::Utc, DateTime};
use size_format::SizeFormatterBinary;

// show file url as relative to static path
macro_rules! encode_file_url {
    ($path:ident) => {
        utf8_percent_encode(&$path, CONTROLS)
    };
}

// " -- &quot;  & -- &amp;  ' -- &#x27;  < -- &lt;  > -- &gt;  / -- &#x2f;
macro_rules! encode_file_name {
    ($entry:ident) => {
        escape_html_entity(&$entry.file_name().to_string_lossy())
    };
}

pub fn directory_listing(
    dir: &Directory,
    req: &HttpRequest,
) -> Result<ServiceResponse, io::Error> {
    let index_of = format!("Index of {}", req.path());
    let mut body = String::new();
    let base = Path::new(req.path());

	let mut entries: Vec<_> = dir.path.read_dir()?
		.filter(|e| dir.is_visible(&e))
		.map(|e| e.unwrap()).collect();
	entries.sort_by(|a, b| a.path().partial_cmp(&b.path()).unwrap());
    for entry in entries {
        let p = match entry.path().strip_prefix(&dir.path) {
            Ok(p) if cfg!(windows) => {
                base.join(p).to_string_lossy().replace("\\", "/")
            }
            Ok(p) => base.join(p).to_string_lossy().into_owned(),
            Err(_) => continue,
        };

        // if file is a directory, add '/' to the end of the name
        if let Ok(metadata) = entry.metadata() {
            let dt: DateTime<Utc> = metadata.modified()?.into();
            if metadata.is_dir() {
                let _ = write!(
                    body,
                    "<li><a href=\"{}\">{}/</a></li>",
                    encode_file_url!(p),
                    encode_file_name!(entry)
                );
            } else {
                let _ = write!(
                    body,
                    "<li><span><a href=\"{}\">{}</a></span><span>{}</span><span>{}B</span></li>",
                    encode_file_url!(p),
                    encode_file_name!(entry),
                    dt.format("%Y/%m/%d %T"),
                    SizeFormatterBinary::new(metadata.len())
                );
            }
        } else {
            continue;
        }
    }

    let html = format!(
        "<html>\
         <head>
         <style>
         ul {{ list-style: none; }}
         li {{ display: flex; }}
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
