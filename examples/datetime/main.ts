// Date/Time Utilities Demo
// Demonstrates Date object functionality

import { generateCalendar, getMonthName, getDaysInMonth } from "./calendar";
import { Duration, formatDuration, addDuration, subtractDuration } from "./duration";

console.log("=== Date/Time Utilities Demo ===\n");

// --- Basic Date Operations ---
console.log("--- Basic Date Operations ---");

// Create dates in different ways
const now: Date = new Date();
const fromTimestamp: Date = new Date(1000000000000);  // Sep 9, 2001
const fromString: Date = new Date("2024-12-25T10:30:00");
const fromComponents: Date = new Date(2024, 11, 25, 10, 30, 0);  // Month is 0-indexed

console.log("Current date (timestamp):", now.getTime());
console.log("From timestamp (1000000000000):", fromTimestamp.toISOString());
console.log("From string '2024-12-25T10:30:00':", fromString.toISOString());
console.log("From components (2024, 11, 25, 10, 30, 0):", fromComponents.toISOString());

// --- Date Getters ---
console.log("\n--- Date Getters ---");
const testDate: Date = new Date("2024-07-15T14:30:45.123Z");
console.log("Test date:", testDate.toISOString());
console.log("  getFullYear():", testDate.getFullYear());
console.log("  getMonth():", testDate.getMonth(), "(July = 6, 0-indexed)");
console.log("  getDate():", testDate.getDate());
console.log("  getDay():", testDate.getDay(), "(Monday = 1)");
console.log("  getHours():", testDate.getUTCHours(), "(UTC)");
console.log("  getMinutes():", testDate.getUTCMinutes());
console.log("  getSeconds():", testDate.getUTCSeconds());
console.log("  getMilliseconds():", testDate.getUTCMilliseconds());
console.log("  getTime():", testDate.getTime());

// --- Date Arithmetic ---
console.log("\n--- Date Arithmetic ---");

function addDays(date: Date, days: number): Date {
    const result: Date = new Date(date.getTime());
    result.setDate(result.getDate() + days);
    return result;
}

function addMonths(date: Date, months: number): Date {
    const result: Date = new Date(date.getTime());
    result.setMonth(result.getMonth() + months);
    return result;
}

function addYears(date: Date, years: number): Date {
    const result: Date = new Date(date.getTime());
    result.setFullYear(result.getFullYear() + years);
    return result;
}

const baseDate: Date = new Date("2024-01-15T12:00:00Z");
console.log("Base date:", baseDate.toISOString());
console.log("  + 10 days:", addDays(baseDate, 10).toISOString());
console.log("  + 3 months:", addMonths(baseDate, 3).toISOString());
console.log("  + 1 year:", addYears(baseDate, 1).toISOString());
console.log("  - 5 days:", addDays(baseDate, -5).toISOString());

// --- Date Comparison ---
console.log("\n--- Date Comparison ---");

function compareDates(a: Date, b: Date): number {
    return a.getTime() - b.getTime();
}

function isSameDay(a: Date, b: Date): boolean {
    return a.getFullYear() === b.getFullYear() &&
           a.getMonth() === b.getMonth() &&
           a.getDate() === b.getDate();
}

function isLeapYear(year: number): boolean {
    return (year % 4 === 0 && year % 100 !== 0) || (year % 400 === 0);
}

const date1: Date = new Date("2024-03-15");
const date2: Date = new Date("2024-03-20");
const date3: Date = new Date("2024-03-15");

console.log("date1: 2024-03-15, date2: 2024-03-20, date3: 2024-03-15");
console.log("  date1 < date2:", compareDates(date1, date2) < 0);
console.log("  date1 === date3:", isSameDay(date1, date3));
console.log("  2024 is leap year:", isLeapYear(2024));
console.log("  2023 is leap year:", isLeapYear(2023));

// --- Days Between Dates ---
console.log("\n--- Days Between Dates ---");

function daysBetween(a: Date, b: Date): number {
    const msPerDay: number = 24 * 60 * 60 * 1000;
    const utc1: number = Date.UTC(a.getFullYear(), a.getMonth(), a.getDate());
    const utc2: number = Date.UTC(b.getFullYear(), b.getMonth(), b.getDate());
    return Math.floor((utc2 - utc1) / msPerDay);
}

const startDate: Date = new Date("2024-01-01");
const endDate: Date = new Date("2024-12-31");
console.log("Days from 2024-01-01 to 2024-12-31:", daysBetween(startDate, endDate));

const birthday: Date = new Date("2024-06-15");
const today: Date = new Date("2024-03-01");
console.log("Days from 2024-03-01 to 2024-06-15:", daysBetween(today, birthday));

// --- Calendar Generation ---
console.log("\n--- Calendar Generation ---");

const calendar2024 = generateCalendar(2024, 2);  // March 2024 (0-indexed)
console.log("\nCalendar for March 2024:");
console.log("Month:", getMonthName(2));
console.log("Days in month:", getDaysInMonth(2024, 2));
console.log("Weeks:", JSON.stringify(calendar2024.weeks));

// --- Duration Calculations ---
console.log("\n--- Duration Calculations ---");

const duration1: Duration = { days: 5, hours: 3, minutes: 30, seconds: 0 };
const duration2: Duration = { days: 2, hours: 10, minutes: 45, seconds: 30 };

console.log("Duration 1:", formatDuration(duration1));
console.log("Duration 2:", formatDuration(duration2));
console.log("Sum:", formatDuration(addDuration(duration1, duration2)));
console.log("Difference:", formatDuration(subtractDuration(duration1, duration2)));

function durationBetween(a: Date, b: Date): Duration {
    let totalSeconds: number = Math.abs(b.getTime() - a.getTime()) / 1000;

    const days: number = Math.floor(totalSeconds / 86400);
    totalSeconds = totalSeconds % 86400;

    const hours: number = Math.floor(totalSeconds / 3600);
    totalSeconds = totalSeconds % 3600;

    const minutes: number = Math.floor(totalSeconds / 60);
    const seconds: number = Math.floor(totalSeconds % 60);

    return { days, hours, minutes, seconds };
}

const eventStart: Date = new Date("2024-03-15T09:00:00Z");
const eventEnd: Date = new Date("2024-03-17T18:30:45Z");
console.log("\nDuration between 2024-03-15T09:00:00 and 2024-03-17T18:30:45:");
console.log("  ", formatDuration(durationBetween(eventStart, eventEnd)));

// --- Date Formatting ---
console.log("\n--- Date Formatting ---");

function formatDate(date: Date, format: string): string {
    const year: number = date.getFullYear();
    const month: number = date.getMonth() + 1;
    const day: number = date.getDate();
    const hours: number = date.getHours();
    const minutes: number = date.getMinutes();
    const seconds: number = date.getSeconds();

    const pad = (n: number): string => n < 10 ? "0" + n : "" + n;

    let result: string = format;
    result = result.replace("YYYY", "" + year);
    result = result.replace("MM", pad(month));
    result = result.replace("DD", pad(day));
    result = result.replace("HH", pad(hours));
    result = result.replace("mm", pad(minutes));
    result = result.replace("ss", pad(seconds));

    return result;
}

const formatTestDate: Date = new Date("2024-07-04T15:30:45");
console.log("Date:", formatTestDate.toISOString());
console.log("  YYYY-MM-DD:", formatDate(formatTestDate, "YYYY-MM-DD"));
console.log("  DD/MM/YYYY:", formatDate(formatTestDate, "DD/MM/YYYY"));
console.log("  YYYY-MM-DD HH:mm:ss:", formatDate(formatTestDate, "YYYY-MM-DD HH:mm:ss"));

// --- UTC Methods ---
console.log("\n--- UTC Methods ---");

const utcDate: Date = new Date("2024-06-15T12:30:45Z");
console.log("Date:", utcDate.toISOString());
console.log("  Date.UTC(2024, 5, 15):", Date.UTC(2024, 5, 15));
console.log("  getUTCFullYear():", utcDate.getUTCFullYear());
console.log("  getUTCMonth():", utcDate.getUTCMonth());
console.log("  getUTCDate():", utcDate.getUTCDate());
console.log("  getUTCHours():", utcDate.getUTCHours());
console.log("  getUTCMinutes():", utcDate.getUTCMinutes());
console.log("  getUTCSeconds():", utcDate.getUTCSeconds());

// --- Date Parsing ---
console.log("\n--- Date Parsing ---");

const isoDate: string = "2024-08-20T14:30:00.000Z";
const parsed: Date = new Date(isoDate);
console.log("Parsed ISO string:", isoDate);
console.log("  toISOString():", parsed.toISOString());
console.log("  getTime():", parsed.getTime());
console.log("  valueOf():", parsed.valueOf());
console.log("  toJSON():", parsed.toJSON());

// --- Working with Timestamps ---
console.log("\n--- Working with Timestamps ---");

const timestamp: number = Date.now();
console.log("Current timestamp (Date.now()):", timestamp);

function timestampToDate(ts: number): Date {
    return new Date(ts);
}

function dateToTimestamp(date: Date): number {
    return date.getTime();
}

const someTimestamp: number = 1720000000000;  // July 3, 2024
const dateFromTs: Date = timestampToDate(someTimestamp);
console.log("Timestamp 1720000000000:", dateFromTs.toISOString());
console.log("Back to timestamp:", dateToTimestamp(dateFromTs));

console.log("\n=== Demo Complete ===");
