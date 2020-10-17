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

pub fn is_authorized(req: &HttpRequest, payload: &mut Payload) -> bool {
	if let Some(token) = req.headers().get("token") {
		if let Ok(token) = token.to_str() {
			let config: Result<Data<Config>, Error> = Data::from_request(req, payload).into_inner();
			if let Ok(config) = config {
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
	
	fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
		if is_authorized(req, payload) {
			ok(Authorized)
		}
		else {
			err(ErrorUnauthorized("not authorized"))
		}
	}
}
