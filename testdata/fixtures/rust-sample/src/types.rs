//! Shared types used across the application.

use std::fmt;

/// Unique identifier for a user in the system.
pub type UserId = u64;

/// Unique identifier for a session.
pub type SessionId = String;

/// User roles with ascending privilege levels.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Role {
    /// Read-only guest access.
    Guest = 0,
    /// Standard authenticated user.
    User = 1,
    /// Moderator with elevated permissions.
    Moderator = 2,
    /// Full administrative access.
    Admin = 3,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::Guest => write!(f, "guest"),
            Role::User => write!(f, "user"),
            Role::Moderator => write!(f, "moderator"),
            Role::Admin => write!(f, "admin"),
        }
    }
}

impl Role {
    /// Parse a role from a string name (case-insensitive).
    pub fn from_str_name(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "guest" => Some(Role::Guest),
            "user" => Some(Role::User),
            "moderator" | "mod" => Some(Role::Moderator),
            "admin" | "administrator" => Some(Role::Admin),
            _ => None,
        }
    }
}

/// Represents a user in the system.
#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub email: String,
    pub role: Role,
    pub active: bool,
    created_at: u64,
}

impl User {
    /// Create a new active user with the default `User` role.
    pub fn new(id: UserId, username: String, email: String) -> Self {
        Self {
            id,
            username,
            email,
            role: Role::User,
            active: true,
            created_at: 0,
        }
    }

    /// Check whether the user has at least the given role.
    pub fn has_role(&self, minimum: &Role) -> bool {
        (self.role.clone() as u8) >= (minimum.clone() as u8)
    }

    /// Return the account creation timestamp.
    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    /// Deactivate the user account.
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "User({}, {}, {})", self.id, self.username, self.role)
    }
}
