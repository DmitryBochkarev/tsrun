// Order class demonstrating composition and complex objects
// Features: class composition, computed properties, method chaining

import { Entity } from "./entity";
import { User } from "./user";

// OrderItem as a simple class (not extending Entity)
export class OrderItem {
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

export class Order extends Entity {
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
