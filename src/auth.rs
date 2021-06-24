use crate::config::Config;

use actix_service::{Service, Transform};
use actix_web::{
	dev::{ServiceRequest, ServiceResponse},
	error::ErrorUnauthorized,
	web::Data,
	Error,
};
use futures::future::{err, ok, Either, Ready};
use std::task::{Context, Poll};

// There are two steps in middleware processing.
// 1. Middleware initialization, middleware factory gets called with
//    next service in chain as parameter.
// 2. Middleware's call method gets called with normal request.
pub struct TokenAuth;

// Middleware factory is `Transform` trait from actix-service crate
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S> for TokenAuth
where
	S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
	S::Future: 'static,
	B: 'static,
{
	type Request = ServiceRequest;
	type Response = ServiceResponse<B>;
	type Error = Error;
	type InitError = ();
	type Transform = TokenAuthMiddleware<S>;
	type Future = Ready<Result<Self::Transform, Self::InitError>>;

	fn new_transform(&self, service: S) -> Self::Future {
		ok(TokenAuthMiddleware { service })
	}
}

pub struct TokenAuthMiddleware<S> {
	service: S,
}

impl<S, B> Service for TokenAuthMiddleware<S>
where
	S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
	S::Future: 'static,
{
	type Request = ServiceRequest;
	type Response = ServiceResponse<B>;
	type Error = Error;
	type Future = Either<S::Future, Ready<Result<Self::Response, Self::Error>>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.service.poll_ready(cx)
	}

	fn call(&mut self, req: ServiceRequest) -> Self::Future {
		if let Some(token) = req.headers().get("token").and_then(|token| token.to_str().ok()) {
			if let Some(config) = req.app_data::<Data<Config>>() {
				if token == config.token {
					return Either::Left(self.service.call(req));
				}
			}
		}
		Either::Right(err(ErrorUnauthorized("not authorized")))
	}
}
