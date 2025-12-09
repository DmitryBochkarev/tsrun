// Data Transformation Pipeline Example
// Demonstrates: array methods, arrow functions, destructuring, method chaining, spread operator

import { products, orders } from "./data";
import {
  filterByCategory,
  filterLowStock,
  toProductSummary,
  calculateInventoryValue,
  getAllTags,
  sortByPrice,
  groupByCategory,
  enrichOrders,
  getOrdersByStatus,
  getCategoryStats,
} from "./transforms";

// ═══════════════════════════════════════════════════════════════════════════
// Pipeline 1: Product Analysis
// ═══════════════════════════════════════════════════════════════════════════

const electronicsProducts = filterByCategory(products, "Electronics");
const electronicsSummary = toProductSummary(sortByPrice(electronicsProducts));

// ═══════════════════════════════════════════════════════════════════════════
// Pipeline 2: Inventory Analysis
// ═══════════════════════════════════════════════════════════════════════════

const lowStockItems = filterLowStock(products, 40);
const totalInventoryValue = calculateInventoryValue(products);
const allTags = getAllTags(products);

// ═══════════════════════════════════════════════════════════════════════════
// Pipeline 3: Category Statistics
// ═══════════════════════════════════════════════════════════════════════════

const categoryStats = getCategoryStats(products);
const groupedProducts = groupByCategory(products);

// ═══════════════════════════════════════════════════════════════════════════
// Pipeline 4: Order Processing
// ═══════════════════════════════════════════════════════════════════════════

const pendingOrders = getOrdersByStatus(orders, "pending");
const shippedOrders = getOrdersByStatus(orders, "shipped");
const enrichedOrders = enrichOrders(orders, products);

// ═══════════════════════════════════════════════════════════════════════════
// Pipeline 5: Complex Chained Transformation
// ═══════════════════════════════════════════════════════════════════════════

// Get top 3 most expensive in-stock electronics
const topExpensiveElectronics = products
  .filter((p) => p.category === "Electronics" && p.stock > 0)
  .sort((a, b) => b.price - a.price)
  .slice(0, 3)
  .map(({ id, name, price }) => ({ id, name, price }));

// Calculate average order value
const orderTotals = enrichedOrders.map((o) => o.total);
const avgOrderValue = orderTotals.reduce((a, b) => a + b, 0) / orderTotals.length;

// ═══════════════════════════════════════════════════════════════════════════
// Output Results
// ═══════════════════════════════════════════════════════════════════════════

const results = {
  electronicsAnalysis: {
    count: electronicsProducts.length,
    products: electronicsSummary,
  },
  inventoryAnalysis: {
    totalValue: Math.round(totalInventoryValue * 100) / 100,
    lowStockCount: lowStockItems.length,
    lowStockItems: lowStockItems.map((p) => p.name),
    uniqueTags: allTags,
  },
  categoryStats,
  orderSummary: {
    totalOrders: orders.length,
    pendingCount: pendingOrders.length,
    shippedCount: shippedOrders.length,
    averageOrderValue: Math.round(avgOrderValue * 100) / 100,
  },
  topExpensiveElectronics,
};

JSON.stringify(results, null, 2);
