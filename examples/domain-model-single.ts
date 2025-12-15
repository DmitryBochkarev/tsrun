// Domain Model Example - Single File Version
// Demonstrates: classes, inheritance, private fields, getters/setters, static methods

// Base Entity class demonstrating core OOP features
class Entity {
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

// User class demonstrating inheritance and encapsulation
class User extends Entity {
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

// OrderItem as a simple class (not extending Entity)
class OrderItem {
  #productName: string;
  #quantity: number;
  #unitPrice: number;

  constructor(productName: string, quantity: number, unitPrice: number) {
    this.#productName = productName;
    this.#quantity = quantity;
    this.#unitPrice = unitPrice;
  }

  get productName(): string {
    return this.#productName;
  }

  get quantity(): number {
    return this.#quantity;
  }

  get unitPrice(): number {
    return this.#unitPrice;
  }

  // Computed property
  get total(): number {
    return this.#quantity * this.#unitPrice;
  }

  toJSON(): object {
    return {
      productName: this.#productName,
      quantity: this.#quantity,
      unitPrice: this.#unitPrice,
      total: this.total,
    };
  }
}

// Order status enum-like values
const OrderStatus = {
  PENDING: "pending",
  CONFIRMED: "confirmed",
  SHIPPED: "shipped",
  DELIVERED: "delivered",
  CANCELLED: "cancelled",
};

class Order extends Entity {
  #user: User;
  #items: OrderItem[];
  #status: string;

  constructor(user: User) {
    super();
    this.#user = user;
    this.#items = [];
    this.#status = OrderStatus.PENDING;
  }

  get user(): User {
    return this.#user;
  }

  get items(): OrderItem[] {
    return [...this.#items]; // Return copy to prevent direct mutation
  }

  get status(): string {
    return this.#status;
  }

  // Computed property for total
  get total(): number {
    return this.#items.reduce((sum, item) => sum + item.total, 0);
  }

  // Computed property for item count
  get itemCount(): number {
    return this.#items.reduce((count, item) => count + item.quantity, 0);
  }

  // Method chaining - returns this
  addItem(productName: string, quantity: number, unitPrice: number): Order {
    this.#items.push(new OrderItem(productName, quantity, unitPrice));
    return this;
  }

  // Status transitions
  confirm(): boolean {
    if (this.#status === OrderStatus.PENDING && this.#items.length > 0) {
      this.#status = OrderStatus.CONFIRMED;
      return true;
    }
    return false;
  }

  ship(): boolean {
    if (this.#status === OrderStatus.CONFIRMED) {
      this.#status = OrderStatus.SHIPPED;
      return true;
    }
    return false;
  }

  deliver(): boolean {
    if (this.#status === OrderStatus.SHIPPED) {
      this.#status = OrderStatus.DELIVERED;
      return true;
    }
    return false;
  }

  cancel(): boolean {
    if (
      this.#status === OrderStatus.PENDING ||
      this.#status === OrderStatus.CONFIRMED
    ) {
      this.#status = OrderStatus.CANCELLED;
      return true;
    }
    return false;
  }

  // Override toJSON
  toJSON(): object {
    return {
      ...super.toJSON(),
      user: this.#user.toJSON(),
      items: this.#items.map((item) => item.toJSON()),
      status: this.#status,
      total: this.total,
      itemCount: this.itemCount,
    };
  }
}

// ============= Main Script =============

// Create users
const admin = new User("Alice Admin", "alice@example.com", "admin");
const customer = new User("Bob Customer", "bob@example.com");

// Create an order with method chaining
const order1 = new Order(customer)
  .addItem("Laptop", 1, 999.99)
  .addItem("Mouse", 2, 29.99)
  .addItem("Keyboard", 1, 79.99);

// Transition order through states
order1.confirm();
order1.ship();

// Create another order
const order2 = new Order(customer).addItem("Monitor", 1, 449.99);
order2.confirm();

// Create an order for admin
const adminOrder = new Order(admin)
  .addItem("Server", 1, 2999.99)
  .addItem("Network Cable", 10, 15.99);
adminOrder.confirm();
adminOrder.ship();
adminOrder.deliver();

// Demonstrate cancelled order
const cancelledOrder = new Order(customer).addItem("Cancelled Item", 1, 100.0);
cancelledOrder.cancel();

// Build results object
const results = {
  entityCount: Entity.getEntityCount(),
  users: {
    admin: admin.toJSON(),
    customer: customer.toJSON(),
  },
  orders: {
    order1: order1.toJSON(),
    order2: order2.toJSON(),
    adminOrder: adminOrder.toJSON(),
    cancelledOrder: cancelledOrder.toJSON(),
  },
  summary: {
    adminIsAdmin: admin.isAdmin(),
    customerIsAdmin: customer.isAdmin(),
    order1Status: order1.status,
    order1Total: order1.total,
    order1ItemCount: order1.itemCount,
    order2Status: order2.status,
    adminOrderStatus: adminOrder.status,
    cancelledOrderStatus: cancelledOrder.status,
  },
};

JSON.stringify(results, null, 2);
