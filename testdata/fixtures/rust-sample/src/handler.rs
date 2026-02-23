//! HTTP request handler that dispatches authenticated requests.

use std::collections::HashMap;

use crate::auth::{self, AuthError, Claims};
use crate::config::Config;
use crate::db::Connection;
use crate::types::UserId;

/// HTTP methods supported by the handler.
#[derive(Debug, Clone, PartialEq)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
}

/// A simplified HTTP request.
#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

/// A simplified HTTP response.
#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub body: String,
}

impl Response {
    fn ok(body: impl Into<String>) -> Self {
        Self { status: 200, body: body.into() }
    }

    fn unauthorized(msg: impl Into<String>) -> Self {
        Self { status: 401, body: msg.into() }
    }

    fn not_found() -> Self {
        Self { status: 404, body: "not found".into() }
    }

    fn internal_error(msg: impl Into<String>) -> Self {
        Self { status: 500, body: msg.into() }
    }
}

/// Handles incoming requests with authentication and routing.
pub struct AuthHandler {
    config: Config,
    db: Connection,
}

impl AuthHandler {
    /// Create a new handler with the given configuration and database.
    pub fn new(config: Config, db: Connection) -> Self {
        Self { config, db }
    }

    /// Process an incoming request.
    ///
    /// Validates the `Authorization` header, extracts claims, and routes
    /// the request to the appropriate internal handler.
    pub fn handle_request(&self, req: &Request) -> Response {
        let claims = match self.authenticate(req) {
            Ok(c) => c,
            Err(e) => return Response::unauthorized(e.to_string()),
        };

        match (&req.method, req.path.as_str()) {
            (Method::Get, "/api/user") => self.get_user(claims.sub),
            (Method::Get, "/api/health") => Response::ok("healthy"),
            (Method::Post, "/api/user") => self.create_user(req, &claims),
            _ => Response::not_found(),
        }
    }

    /// Extract and validate the auth token from request headers.
    fn authenticate(&self, req: &Request) -> Result<Claims, AuthError> {
        let header = req
            .headers
            .get("authorization")
            .ok_or_else(|| AuthError::MalformedToken("missing Authorization header".into()))?;

        auth::validate_token(header, self.config.jwt_secret.as_bytes())
    }

    /// Fetch a user by ID from the database.
    fn get_user(&self, user_id: UserId) -> Response {
        match self.db.query(&format!("SELECT * FROM users WHERE id = {}", user_id)) {
            Ok(rows) if !rows.is_empty() => Response::ok(rows.join(",")),
            Ok(_) => Response::not_found(),
            Err(e) => Response::internal_error(format!("db error: {}", e)),
        }
    }

    /// Create a new user from the request body.
    fn create_user(&self, req: &Request, _claims: &Claims) -> Response {
        match &req.body {
            Some(body) if !body.is_empty() => Response::ok(format!("created: {}", body)),
            _ => Response::ok("missing body"),
        }
    }
}
