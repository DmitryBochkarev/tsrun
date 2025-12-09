// Transformation functions for data pipeline
// Demonstrates: arrow functions, destructuring, spread operator

import { Product, Order } from "./data";

// Filter products by category
export const filterByCategory = (products: Product[], category: string): Product[] =>
  products.filter((p) => p.category === category);

// Filter in-stock products
export const filterInStock = (products: Product[]): Product[] =>
  products.filter(({ stock }) => stock > 0);

// Filter low-stock products (below threshold)
export const filterLowStock = (products: Product[], threshold: number = 50): Product[] =>
  products.filter(({ stock }) => stock < threshold);

// Map to product summaries
export const toProductSummary = (products: Product[]): { name: string; price: number }[] =>
  products.map(({ name, price }) => ({ name, price }));

// Calculate total inventory value
export const calculateInventoryValue = (products: Product[]): number =>
  products.reduce((total, { price, stock }) => total + price * stock, 0);

// Get all unique tags (using a simpler approach without Set spread)
export const getAllTags = (products: Product[]): string[] => {
  const allTags = products.flatMap(({ tags }) => tags);
  const unique: string[] = [];
  for (const tag of allTags) {
    if (!unique.includes(tag)) {
      unique.push(tag);
    }
  }
  return unique;
};

// Sort products by price
export const sortByPrice = (products: Product[], ascending: boolean = true): Product[] =>
  [...products].sort((a, b) => ascending ? a.price - b.price : b.price - a.price);

// Group products by category
export const groupByCategory = (products: Product[]): { [category: string]: Product[] } =>
  products.reduce((groups, product) => {
    const { category } = product;
    if (!groups[category]) {
      groups[category] = [];
    }
    groups[category].push(product);
    return groups;
  }, {} as { [category: string]: Product[] });

// Get order totals
export const getOrderTotal = (order: Order, products: Product[]): number =>
  order.products.reduce((total, { productId, quantity }) => {
    const product = products.find((p) => p.id === productId);
    return total + (product ? product.price * quantity : 0);
  }, 0);

// Enrich orders with product details
export const enrichOrders = (orders: Order[], products: Product[]): any[] =>
  orders.map((order) => ({
    ...order,
    total: getOrderTotal(order, products),
    items: order.products.map(({ productId, quantity }) => {
      const product = products.find((p) => p.id === productId);
      return {
        product: product ? product.name : "Unknown",
        quantity,
        subtotal: product ? product.price * quantity : 0,
      };
    }),
  }));

// Get products by status
export const getOrdersByStatus = (orders: Order[], status: Order["status"]): Order[] =>
  orders.filter((o) => o.status === status);

// Calculate category statistics
export const getCategoryStats = (products: Product[]): { category: string; count: number; avgPrice: number; totalValue: number }[] => {
  const grouped = groupByCategory(products);
  return Object.entries(grouped).map(([category, items]) => ({
    category,
    count: items.length,
    avgPrice: items.reduce((sum, p) => sum + p.price, 0) / items.length,
    totalValue: items.reduce((sum, p) => sum + p.price * p.stock, 0),
  }));
};
