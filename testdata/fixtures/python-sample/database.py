"""Database connection management with context manager support."""

from __future__ import annotations

import logging
from typing import Any, Optional, Sequence

logger = logging.getLogger(__name__)

# Default maximum number of connection retries.
MAX_RETRIES = 3


class DatabaseError(Exception):
    """Raised when a database operation fails."""

    def __init__(self, message: str, query: Optional[str] = None) -> None:
        super().__init__(message)
        self.query = query


class ConnectionPool:
    """A simple connection pool that tracks active connections."""

    def __init__(self, max_size: int = 5) -> None:
        self._max_size = max_size
        self._active = 0

    def acquire(self) -> bool:
        """Acquire a connection slot. Returns False if pool is exhausted."""
        if self._active >= self._max_size:
            return False
        self._active += 1
        return True

    def release(self) -> None:
        """Release a connection slot back to the pool."""
        if self._active > 0:
            self._active -= 1

    @property
    def available(self) -> int:
        """Number of available connection slots."""
        return self._max_size - self._active


class DatabaseConnection:
    """Manages a database connection with query execution and transactions.

    Supports use as a context manager for automatic cleanup::

        async with DatabaseConnection("postgres://localhost/db") as conn:
            rows = await conn.query("SELECT 1")
    """

    def __init__(self, url: str, pool_size: int = 5) -> None:
        self._url = url
        self._pool = ConnectionPool(max_size=pool_size)
        self._connected = False

    async def connect(self) -> None:
        """Establish the database connection."""
        if self._connected:
            return

        if not self._url:
            raise DatabaseError("Database URL must not be empty")

        if not self._pool.acquire():
            raise DatabaseError("Connection pool exhausted")

        self._connected = True
        logger.info("Connected to %s", self._redacted_url())

    async def disconnect(self) -> None:
        """Close the database connection and release pool resources."""
        if self._connected:
            self._pool.release()
            self._connected = False
            logger.info("Disconnected from database")

    @property
    def is_connected(self) -> bool:
        """Whether the connection is currently active."""
        return self._connected

    async def query(
        self,
        sql: str,
        params: Sequence[Any] = (),
    ) -> list[dict[str, Any]]:
        """Execute a SQL query and return result rows as dicts.

        Args:
            sql: SQL query with $1, $2, ... placeholders.
            params: Positional parameters to bind.

        Returns:
            List of row dictionaries.

        Raises:
            DatabaseError: If not connected or query fails.
        """
        if not self._connected:
            raise DatabaseError("Not connected", query=sql)

        if not sql.strip():
            raise DatabaseError("Empty query", query=sql)

        logger.debug("Executing: %s (params=%s)", sql, params)
        # Simulated result for fixture purposes.
        return [{"result": "ok"}]

    async def execute(self, sql: str, params: Sequence[Any] = ()) -> int:
        """Execute a SQL statement and return the number of affected rows."""
        if not self._connected:
            raise DatabaseError("Not connected", query=sql)
        _ = params
        return 1

    async def __aenter__(self) -> DatabaseConnection:
        await self.connect()
        return self

    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        await self.disconnect()

    def _redacted_url(self) -> str:
        """Return the URL with credentials redacted."""
        if "@" in self._url:
            return self._url.split("@", 1)[1]
        return self._url
