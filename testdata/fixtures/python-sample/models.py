"""Domain models for users, roles, and profiles."""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum
from typing import Optional


class Role(Enum):
    """User roles with ascending privilege levels."""

    GUEST = "guest"
    USER = "user"
    MODERATOR = "moderator"
    ADMIN = "admin"

    @classmethod
    def from_string(cls, value: str) -> Role:
        """Parse a role from a case-insensitive string.

        Raises:
            ValueError: If the string does not match any role.
        """
        normalized = value.lower().strip()
        for member in cls:
            if member.value == normalized:
                return member
        raise ValueError(f"Unknown role: {value!r}")

    def has_permission(self, required: Role) -> bool:
        """Check whether this role meets the minimum required level."""
        order = [Role.GUEST, Role.USER, Role.MODERATOR, Role.ADMIN]
        return order.index(self) >= order.index(required)


class User:
    """Core user entity stored in the database."""

    def __init__(
        self,
        user_id: str,
        username: str,
        email: str,
        role: Role = Role.USER,
        active: bool = True,
    ) -> None:
        self.user_id = user_id
        self.username = username
        self.email = email
        self.role = role
        self.active = active
        self._created_at = datetime.now(timezone.utc)

    @property
    def created_at(self) -> datetime:
        """Return the account creation timestamp."""
        return self._created_at

    def deactivate(self) -> None:
        """Mark the user account as inactive."""
        self.active = False

    def promote(self, new_role: Role) -> None:
        """Change the user's role if it is a promotion.

        Raises:
            ValueError: If the new role is not higher than the current one.
        """
        if not new_role.has_permission(self.role):
            raise ValueError(
                f"Cannot demote from {self.role.value} to {new_role.value}"
            )
        self.role = new_role

    def __repr__(self) -> str:
        return (
            f"User(id={self.user_id!r}, name={self.username!r}, "
            f"role={self.role.value!r}, active={self.active})"
        )

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, User):
            return NotImplemented
        return self.user_id == other.user_id


@dataclass
class UserProfile:
    """Extended user profile with optional metadata."""

    user: User
    display_name: Optional[str] = None
    bio: Optional[str] = None
    avatar_url: Optional[str] = None
    tags: list[str] = field(default_factory=list)

    @property
    def full_display_name(self) -> str:
        """Return the display name, falling back to the username."""
        return self.display_name or self.user.username

    def add_tag(self, tag: str) -> None:
        """Add a tag if not already present."""
        normalized = tag.lower().strip()
        if normalized and normalized not in self.tags:
            self.tags.append(normalized)

    def remove_tag(self, tag: str) -> bool:
        """Remove a tag, returning True if it was present."""
        normalized = tag.lower().strip()
        if normalized in self.tags:
            self.tags.remove(normalized)
            return True
        return False
