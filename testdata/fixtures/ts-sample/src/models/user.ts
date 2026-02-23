/**
 * User domain models and data transfer objects.
 */

/** Roles available to users, ordered by privilege level. */
export enum UserRole {
  Guest = "guest",
  User = "user",
  Moderator = "moderator",
  Admin = "admin",
}

/** Core user entity returned from the database. */
export interface User {
  /** Unique identifier (UUID). */
  id: string;
  /** Display name, unique across the system. */
  username: string;
  /** Primary email address. */
  email: string;
  /** Assigned role. */
  role: UserRole;
  /** Whether the account is currently active. */
  active: boolean;
  /** Timestamp when the account was created. */
  createdAt: Date;
}

/** Data required to create a new user account. */
export type CreateUserDto = {
  username: string;
  email: string;
  password: string;
  role?: UserRole;
};

/** Data allowed when updating an existing user. */
export type UpdateUserDto = Partial<
  Pick<User, "username" | "email" | "role" | "active">
>;

/** Summary projection used in list endpoints. */
export interface UserSummary {
  id: string;
  username: string;
  role: UserRole;
}

/**
 * Convert a full User entity to a summary projection.
 */
export function toSummary(user: User): UserSummary {
  return {
    id: user.id,
    username: user.username,
    role: user.role,
  };
}

/**
 * Check whether the given role string is a valid UserRole.
 */
export function isValidRole(role: string): role is UserRole {
  return Object.values(UserRole).includes(role as UserRole);
}

/**
 * Compare two roles by privilege level.
 * Returns a negative number if a < b, 0 if equal, positive if a > b.
 */
export function compareRoles(a: UserRole, b: UserRole): number {
  const order: UserRole[] = [
    UserRole.Guest,
    UserRole.User,
    UserRole.Moderator,
    UserRole.Admin,
  ];
  return order.indexOf(a) - order.indexOf(b);
}
