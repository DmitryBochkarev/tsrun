// Email and URL validation using RegExp
// Demonstrates: RegExp test(), flags, complex patterns

// Simple email validation
export function isValidEmail(email: string): boolean {
  // Basic email pattern: something@something.something
  const pattern = /^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/;
  return pattern.test(email);
}

// Simple URL validation
export function isValidUrl(url: string): boolean {
  // Basic URL pattern
  const pattern = /^https?:\/\/[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}(\/[^\s]*)?$/;
  return pattern.test(url);
}

// Phone number validation (various formats)
export function isValidPhone(phone: string): boolean {
  // Matches: 123-456-7890, (123) 456-7890, 123.456.7890, +1 123 456 7890
  const pattern = /^(\+\d{1,2}\s?)?\(?\d{3}\)?[\s.-]?\d{3}[\s.-]?\d{4}$/;
  return pattern.test(phone);
}

// Validate hex color
export function isValidHexColor(color: string): boolean {
  // Matches: #FFF, #FFFFFF, #fff, #ffffff
  const pattern = /^#([0-9A-Fa-f]{3}|[0-9A-Fa-f]{6})$/;
  return pattern.test(color);
}

// Validate IP address (simple IPv4)
export function isValidIPv4(ip: string): boolean {
  const pattern = /^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/;
  const match = pattern.exec(ip);

  if (!match) return false;

  // Check each octet is 0-255
  for (let i = 1; i <= 4; i++) {
    const octet = parseInt(match[i], 10);
    if (octet < 0 || octet > 255) return false;
  }

  return true;
}

// Validate date format (YYYY-MM-DD)
export function isValidDateFormat(date: string): boolean {
  const pattern = /^\d{4}-\d{2}-\d{2}$/;
  return pattern.test(date);
}

// Extract all numbers from string
export function extractNumbers(text: string): number[] {
  const pattern = /-?\d+(\.\d+)?/g;
  const results: number[] = [];
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text)) !== null) {
    results.push(parseFloat(match[0]));
  }

  return results;
}

// Check if string contains only alphanumeric
export function isAlphanumeric(text: string): boolean {
  const pattern = /^[a-zA-Z0-9]+$/;
  return pattern.test(text);
}
