"""HTTP request handling with authentication and routing."""

from __future__ import annotations

import json
import logging
from dataclasses import dataclass
from typing import Any, Optional

from .auth import AuthError, validate_token
from .config import Config
from .database import DatabaseConnection

logger = logging.getLogger(__name__)


@dataclass
class Request:
    """Simplified HTTP request representation."""

    method: str
    path: str
    headers: dict[str, str]
    body: Optional[str] = None

    @property
    def content_type(self) -> Optional[str]:
        """Return the Content-Type header value, if present."""
        return self.headers.get("content-type")

    def json(self) -> Any:
        """Parse the request body as JSON.

        Raises:
            ValueError: If the body is empty or not valid JSON.
        """
        if not self.body:
            raise ValueError("Empty request body")
        return json.loads(self.body)


@dataclass
class Response:
    """Simplified HTTP response."""

    status: int
    body: str
    headers: dict[str, str]

    @classmethod
    def ok(cls, body: str) -> Response:
        """Create a 200 OK response."""
        return cls(status=200, body=body, headers={"content-type": "application/json"})

    @classmethod
    def not_found(cls) -> Response:
        """Create a 404 Not Found response."""
        return cls(status=404, body='{"error": "not found"}', headers={})

    @classmethod
    def unauthorized(cls, message: str) -> Response:
        """Create a 401 Unauthorized response."""
        return cls(status=401, body=json.dumps({"error": message}), headers={})

    @classmethod
    def internal_error(cls, message: str) -> Response:
        """Create a 500 Internal Server Error response."""
        return cls(status=500, body=json.dumps({"error": message}), headers={})


class RequestHandler:
    """Dispatches authenticated requests to the appropriate handler method."""

    def __init__(self, config: Config, db: DatabaseConnection) -> None:
        self._config = config
        self._db = db

    async def handle(self, request: Request) -> Response:
        """Route an incoming request after authentication."""
        try:
            claims = self._authenticate(request)
        except AuthError as exc:
            logger.warning("Auth failed: %s", exc)
            return Response.unauthorized(str(exc))

        route_key = (request.method.upper(), request.path)

        if route_key == ("GET", "/api/health"):
            return Response.ok('{"status": "healthy"}')
        elif route_key == ("GET", "/api/user"):
            return await self._get_user(claims.sub)
        elif route_key == ("POST", "/api/user"):
            return await self._create_user(request)
        else:
            return Response.not_found()

    def _authenticate(self, request: Request) -> Any:
        """Extract and validate the bearer token from request headers."""
        auth_header = request.headers.get("authorization", "")
        if not auth_header:
            raise AuthError("Missing Authorization header", "MALFORMED")
        return validate_token(auth_header, self._config.jwt_secret)

    async def _get_user(self, user_id: str) -> Response:
        """Fetch a user by ID from the database."""
        try:
            rows = await self._db.query("SELECT * FROM users WHERE id = $1", [user_id])
            if not rows:
                return Response.not_found()
            return Response.ok(json.dumps(rows[0]))
        except Exception as exc:
            logger.error("Database error: %s", exc)
            return Response.internal_error(f"Database error: {exc}")

    async def _create_user(self, request: Request) -> Response:
        """Create a new user from the request body."""
        try:
            data = request.json()
        except ValueError as exc:
            return Response.ok(json.dumps({"error": str(exc)}))

        affected = await self._db.execute(
            "INSERT INTO users (username, email) VALUES ($1, $2)",
            [data.get("username"), data.get("email")],
        )
        return Response.ok(json.dumps({"created": affected}))


async def handle_request(
    request: Request, config: Config, db: DatabaseConnection
) -> Response:
    """Convenience function that creates a handler and processes one request."""
    handler = RequestHandler(config, db)
    return await handler.handle(request)
