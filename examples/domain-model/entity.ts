// Base Entity class demonstrating core OOP features
// Features: private fields, getters/setters, static methods/fields

export class Entity {
  // Private field for unique ID
  #id: number;

  // Static field to track entity count
  static #nextId: number = 1;

  // Creation timestamp
  #createdAt: Date;

  constructor() {
    this.#id = Entity.#nextId++;
    this.#createdAt = new Date();
  }

  // Getter for id (read-only from outside)
  get id(): number {
    return this.#id;
  }

  // Getter for creation timestamp
  get createdAt(): Date {
    return this.#createdAt;
  }

  // Static method to get total entities created
  static getEntityCount(): number {
    return Entity.#nextId - 1;
  }

  // Virtual method for serialization (can be overridden)
  toJSON(): object {
    return {
      id: this.#id,
      createdAt: this.#createdAt.toISOString(),
    };
  }
}
