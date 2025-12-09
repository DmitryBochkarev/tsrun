// String formatting utilities using RegExp
// Demonstrates: String replace(), replaceAll(), match(), split()

// Convert camelCase to kebab-case
export function toKebabCase(str: string): string {
  return str
    .replace(/([a-z])([A-Z])/g, "$1-$2")
    .toLowerCase();
}

// Convert kebab-case to camelCase
export function toCamelCase(str: string): string {
  return str.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
}

// Convert snake_case to camelCase
export function snakeToCamel(str: string): string {
  return str.replace(/_([a-z])/g, (_, letter) => letter.toUpperCase());
}

// Capitalize first letter of each word
export function titleCase(str: string): string {
  return str.replace(/\b\w/g, (char) => char.toUpperCase());
}

// Remove extra whitespace
export function normalizeWhitespace(str: string): string {
  return str.replace(/\s+/g, " ").trim();
}

// Escape HTML special characters
export function escapeHtml(str: string): string {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

// Simple template substitution: {{variable}}
export function template(str: string, vars: Record<string, string>): string {
  let result = str;
  for (const key in vars) {
    const pattern = new RegExp(`\\{\\{${key}\\}\\}`, "g");
    result = result.replace(pattern, vars[key]);
  }
  return result;
}

// Mask sensitive data (e.g., credit card numbers)
// Note: Simplified to avoid lookahead regex
export function maskCreditCard(str: string): string {
  // Remove non-digits, then mask all but last 4
  const digits = str.replace(/\D/g, "");
  if (digits.length <= 4) return str;
  const masked = "*".repeat(digits.length - 4) + digits.slice(-4);
  // Format as groups of 4
  return masked.replace(/(.{4})/g, "$1-").slice(0, -1);
}

// Format number with thousands separator
// Note: Using a loop instead of lookahead regex for compatibility
export function formatNumber(num: number): string {
  const parts = num.toString().split(".");
  let intPart = parts[0];
  const decPart = parts[1];

  // Add commas to integer part
  let result = "";
  let count = 0;
  for (let i = intPart.length - 1; i >= 0; i--) {
    if (count > 0 && count % 3 === 0) {
      result = "," + result;
    }
    result = intPart.charAt(i) + result;
    count++;
  }

  return decPart ? result + "." + decPart : result;
}

// Extract words from text
export function extractWords(text: string): string[] {
  const matches = text.match(/\b\w+\b/g);
  return matches || [];
}

// Truncate text at word boundary
export function truncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;

  const truncated = text.slice(0, maxLength);
  const lastSpace = truncated.lastIndexOf(" ");

  if (lastSpace > 0) {
    return truncated.slice(0, lastSpace) + "...";
  }

  return truncated + "...";
}

// Slugify text (for URLs)
export function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^\w\s-]/g, "")
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
}
