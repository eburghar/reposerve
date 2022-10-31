use crate::config::Config;

use actix_web::{web, HttpResponse};
use std::process::Command;

pub(crate) async fn webhook(webhook: web::Path<String>, config: web::Data<Config>) -> HttpResponse {
	if let Some(ref webhooks) = config.webhooks {
		match webhooks.get(webhook.as_str()) {
			Some(script) => match Command::new(script).output() {
				Ok(output) => HttpResponse::Ok().body(format!(
					"{} executed with success: {}",
					script,
					std::str::from_utf8(&output.stdout).unwrap_or("")
				)),
				Err(error) => HttpResponse::NotFound()
					.body(format!("failed to execute {}: {}", script, error)),
			},
			_ => HttpResponse::NotFound().body("Not found"),
		}
	} else {
		panic!("webhooks not configured !");
	}
}
