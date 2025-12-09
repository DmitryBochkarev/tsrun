// RegExp Text Processing Demo
// Demonstrates: RegExp constructor and literals, test(), exec(),
// String match(), replace(), split(), capture groups, flags

import { parseBold, parseLinks, parseHeaders, parseInlineCode } from "./parser";
import { isValidEmail, isValidUrl, isValidPhone, isValidHexColor, isValidIPv4, extractNumbers } from "./validator";
import { toKebabCase, toCamelCase, titleCase, normalizeWhitespace, escapeHtml, template, formatNumber, slugify, extractWords } from "./formatter";

console.log("=== RegExp Text Processing Demo ===\n");

// === Parser Demo ===
console.log("--- Markup Parser ---");

const markupText = `# Welcome to TypeScript
This is **bold text** and more **emphasis here**.
Check out [TypeScript](https://typescriptlang.org) or [Rust](https://rust-lang.org).
Here is some \`inline code\` example.`;

console.log("Input text:");
console.log(markupText);
console.log("");

const headers = parseHeaders(markupText);
console.log("Headers found:", JSON.stringify(headers));

const boldElements = parseBold(markupText);
console.log("Bold elements:", JSON.stringify(boldElements));

const links = parseLinks(markupText);
console.log("Links found:", JSON.stringify(links));

const codeElements = parseInlineCode(markupText);
console.log("Code elements:", JSON.stringify(codeElements));

// === Validator Demo ===
console.log("\n--- Validators ---");

const emails = ["user@example.com", "invalid-email", "test.user@sub.domain.org", "@missing.local"];
console.log("Email validation:");
for (const email of emails) {
  console.log(`  ${email}: ${isValidEmail(email)}`);
}

const urls = ["https://example.com", "http://sub.domain.org/path", "not-a-url", "ftp://invalid.com"];
console.log("\nURL validation:");
for (const url of urls) {
  console.log(`  ${url}: ${isValidUrl(url)}`);
}

const phones = ["123-456-7890", "(123) 456-7890", "123.456.7890", "12345"];
console.log("\nPhone validation:");
for (const phone of phones) {
  console.log(`  ${phone}: ${isValidPhone(phone)}`);
}

const colors = ["#FFF", "#ffffff", "#AABBCC", "#GGG", "red"];
console.log("\nHex color validation:");
for (const color of colors) {
  console.log(`  ${color}: ${isValidHexColor(color)}`);
}

const ips = ["192.168.1.1", "10.0.0.1", "256.1.1.1", "1.2.3"];
console.log("\nIPv4 validation:");
for (const ip of ips) {
  console.log(`  ${ip}: ${isValidIPv4(ip)}`);
}

const textWithNumbers = "The answer is 42, but also -3.14 and 100.";
console.log(`\nExtract numbers from "${textWithNumbers}":`);
console.log(`  Result: ${JSON.stringify(extractNumbers(textWithNumbers))}`);

// === Formatter Demo ===
console.log("\n--- Formatters ---");

console.log("\nCase conversions:");
console.log(`  toKebabCase("myVariableName"): ${toKebabCase("myVariableName")}`);
console.log(`  toCamelCase("my-variable-name"): ${toCamelCase("my-variable-name")}`);
console.log(`  titleCase("hello world"): ${titleCase("hello world")}`);

console.log("\nText normalization:");
const messyText = "  too   many    spaces   ";
console.log(`  normalizeWhitespace("${messyText}"): "${normalizeWhitespace(messyText)}"`);

console.log("\nHTML escaping:");
const htmlText = "<script>alert('xss')</script>";
console.log(`  escapeHtml("${htmlText}"): "${escapeHtml(htmlText)}"`);

console.log("\nTemplate substitution:");
const templateStr = "Hello, {{name}}! Welcome to {{place}}.";
const vars = { name: "Alice", place: "Wonderland" };
console.log(`  template("${templateStr}", ${JSON.stringify(vars)}):`);
console.log(`  "${template(templateStr, vars)}"`);

console.log("\nNumber formatting:");
console.log(`  formatNumber(1234567): ${formatNumber(1234567)}`);
console.log(`  formatNumber(1000000.5): ${formatNumber(1000000.5)}`);

console.log("\nSlugify:");
console.log(`  slugify("Hello World! How are you?"): ${slugify("Hello World! How are you?")}`);
console.log(`  slugify("TypeScript & Rust"): ${slugify("TypeScript & Rust")}`);

console.log("\nWord extraction:");
const sentence = "The quick brown fox jumps over the lazy dog.";
console.log(`  extractWords("${sentence}"):`);
console.log(`  ${JSON.stringify(extractWords(sentence))}`);

// === Direct RegExp Demo ===
console.log("\n--- Direct RegExp Usage ---");

// RegExp literal
const wordPattern = /\w+/g;
const testString = "Hello, World!";
console.log(`Pattern /\\w+/g on "${testString}":`);
console.log(`  match(): ${JSON.stringify(testString.match(wordPattern))}`);

// RegExp constructor
const dynamicPattern = new RegExp("test", "gi");
console.log(`\nRegExp("test", "gi") on "Test TEST test":`);
console.log(`  test(): ${dynamicPattern.test("Test TEST test")}`);

// Using exec() in a loop
const execPattern = /(\d+)/g;
const execString = "a1b23c456";
console.log(`\nPattern /(\\d+)/g exec() loop on "${execString}":`);
let execMatch: RegExpExecArray | null;
while ((execMatch = execPattern.exec(execString)) !== null) {
  console.log(`  Found: ${execMatch[0]} at index ${execMatch.index}`);
}

// Split with RegExp
const splitString = "apple,banana;cherry orange";
console.log(`\nSplit "${splitString}" on /[,;\\s]+/:`);
console.log(`  ${JSON.stringify(splitString.split(/[,;\s]+/))}`);

// Replace with callback
const replaceString = "foo bar baz";
const replaced = replaceString.replace(/\b\w/g, (c) => c.toUpperCase());
console.log(`\nReplace /\\b\\w/g with uppercase on "${replaceString}":`);
console.log(`  ${replaced}`);

console.log("\n=== Demo Complete ===");
