// User class demonstrating inheritance and encapsulation
// Features: extends, super(), private fields, method override

import { Entity } from "./entity";

export class User extends Entity {
  #name: string;
  #email: string;
  #role: string;

  constructor(name: string, email: string, role: string = "user") {
    super(); // Call parent constructor
    this.#name = name;
    this.#email = email;
    this.#role = role;
  }

  // Getters for user properties
  get name(): string {
    return this.#name;
  }

  get email(): string {
    return this.#email;
  }

  get role(): string {
    return this.#role;
  }

  // Setter with validation
  set role(newRole: string) {
    const validRoles = ["user", "admin", "moderator"];
    if (validRoles.includes(newRole)) {
      this.#role = newRole;
    }
  }

  // Check if user is admin
  isAdmin(): boolean {
    return this.#role === "admin";
  }

  // Override parent's toJSON
  toJSON(): object {
    return {
      ...super.toJSON(),
      name: this.#name,
      email: this.#email,
      role: this.#role,
    };
  }
}
