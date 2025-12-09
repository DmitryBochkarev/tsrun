// Sample data array for transformation pipeline
// Demonstrates: interfaces, type annotations, arrays

export interface Product {
  id: number;
  name: string;
  category: string;
  price: number;
  stock: number;
  tags: string[];
}

export interface Order {
  id: number;
  customerId: number;
  products: { productId: number; quantity: number }[];
  date: string;
  status: "pending" | "shipped" | "delivered";
}

export const products: Product[] = [
  { id: 1, name: "Laptop", category: "Electronics", price: 999.99, stock: 50, tags: ["computer", "portable"] },
  { id: 2, name: "Mouse", category: "Electronics", price: 29.99, stock: 200, tags: ["peripheral", "input"] },
  { id: 3, name: "Keyboard", category: "Electronics", price: 79.99, stock: 150, tags: ["peripheral", "input"] },
  { id: 4, name: "Desk Chair", category: "Furniture", price: 299.99, stock: 30, tags: ["office", "seating"] },
  { id: 5, name: "Monitor", category: "Electronics", price: 449.99, stock: 75, tags: ["display", "computer"] },
  { id: 6, name: "Standing Desk", category: "Furniture", price: 599.99, stock: 20, tags: ["office", "ergonomic"] },
  { id: 7, name: "Webcam", category: "Electronics", price: 89.99, stock: 100, tags: ["peripheral", "video"] },
  { id: 8, name: "Headphones", category: "Electronics", price: 199.99, stock: 80, tags: ["audio", "wireless"] },
  { id: 9, name: "USB Hub", category: "Electronics", price: 39.99, stock: 250, tags: ["peripheral", "connectivity"] },
  { id: 10, name: "Desk Lamp", category: "Furniture", price: 49.99, stock: 60, tags: ["office", "lighting"] },
];

export const orders: Order[] = [
  { id: 101, customerId: 1, products: [{ productId: 1, quantity: 1 }, { productId: 2, quantity: 2 }], date: "2024-01-15", status: "delivered" },
  { id: 102, customerId: 2, products: [{ productId: 3, quantity: 1 }, { productId: 5, quantity: 1 }], date: "2024-01-16", status: "shipped" },
  { id: 103, customerId: 1, products: [{ productId: 8, quantity: 1 }], date: "2024-01-17", status: "pending" },
  { id: 104, customerId: 3, products: [{ productId: 4, quantity: 2 }, { productId: 6, quantity: 1 }], date: "2024-01-18", status: "delivered" },
  { id: 105, customerId: 2, products: [{ productId: 7, quantity: 1 }, { productId: 9, quantity: 3 }], date: "2024-01-19", status: "shipped" },
];
