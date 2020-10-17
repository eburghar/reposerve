use crate::config::Config;

use actix_web::{
	HttpRequest,
	FromRequest,
	Result,
	Error,
	dev::Payload,
	error::ErrorUnauthorized,
	web::Data,
};
use futures::future::{
	Ready,
	ok,
	err
};

pub struct Authorized;

pub fn is_authorized(req: &HttpRequest) -> bool {
	if let Some(token) = req.headers().get("token") {
		if let Ok(token) = token.to_str() {
			let config = req.app_data::<Data<Config>>();
			if let Some(config) = config {
				return token == config.token;
			}
		}
	}
	false
}

impl FromRequest for Authorized {
	type Error = Error;
	type Future = Ready<Result<Self, Self::Error>>;
	type Config = ();
	
	fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
		if is_authorized(req) {
			ok(Authorized)
		}
		else {
			err(ErrorUnauthorized("not authorized"))
		}
	}
}
