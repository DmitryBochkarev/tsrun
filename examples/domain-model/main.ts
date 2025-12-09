// Domain Model Example
// Demonstrates: classes, inheritance, private fields, getters/setters, static methods

import { Entity } from "./entity";
import { User } from "./user";
import { Order, OrderItem } from "./order";

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
