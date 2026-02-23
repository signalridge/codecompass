// Package models defines the core domain types for the application.
package models

import (
	"fmt"
	"strings"
	"time"
)

// Role represents a user's permission level.
type Role string

// Predefined roles ordered by privilege level.
const (
	RoleGuest     Role = "guest"
	RoleUser      Role = "user"
	RoleModerator Role = "moderator"
	RoleAdmin     Role = "admin"
)

// roleOrder maps roles to their numeric privilege level.
var roleOrder = map[Role]int{
	RoleGuest:     0,
	RoleUser:      1,
	RoleModerator: 2,
	RoleAdmin:     3,
}

// IsValid reports whether the role is a recognized value.
func (r Role) IsValid() bool {
	_, ok := roleOrder[r]
	return ok
}

// HasPermission reports whether this role meets the minimum required level.
func (r Role) HasPermission(required Role) bool {
	return roleOrder[r] >= roleOrder[required]
}

// ParseRole converts a case-insensitive string to a Role.
// Returns an error if the string does not match any known role.
func ParseRole(s string) (Role, error) {
	normalized := Role(strings.ToLower(strings.TrimSpace(s)))
	if !normalized.IsValid() {
		return "", fmt.Errorf("unknown role: %q", s)
	}
	return normalized, nil
}

// User represents a user account in the system.
type User struct {
	ID        string    `json:"id"`
	Username  string    `json:"username"`
	Email     string    `json:"email"`
	Role      Role      `json:"role"`
	Active    bool      `json:"active"`
	CreatedAt time.Time `json:"created_at"`
}

// NewUser creates a new active user with the default User role.
func NewUser(id, username, email string) *User {
	return &User{
		ID:        id,
		Username:  username,
		Email:     email,
		Role:      RoleUser,
		Active:    true,
		CreatedAt: time.Now(),
	}
}

// Deactivate marks the user account as inactive.
func (u *User) Deactivate() {
	u.Active = false
}

// Promote changes the user's role if the new role is higher.
// Returns an error if the new role would be a demotion.
func (u *User) Promote(newRole Role) error {
	if !newRole.HasPermission(u.Role) {
		return fmt.Errorf("cannot demote from %s to %s", u.Role, newRole)
	}
	u.Role = newRole
	return nil
}

// String returns a human-readable representation of the user.
func (u *User) String() string {
	status := "active"
	if !u.Active {
		status = "inactive"
	}
	return fmt.Sprintf("User(%s, %s, %s, %s)", u.ID, u.Username, u.Role, status)
}

// UserSummary is a projection used in list responses.
type UserSummary struct {
	ID       string `json:"id"`
	Username string `json:"username"`
	Role     Role   `json:"role"`
}

// ToSummary converts a full User to a UserSummary projection.
func (u *User) ToSummary() UserSummary {
	return UserSummary{
		ID:       u.ID,
		Username: u.Username,
		Role:     u.Role,
	}
}
