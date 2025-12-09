// Simple markup parser using RegExp
// Demonstrates: RegExp literals, exec(), capture groups

interface ParsedElement {
  type: string;
  content: string;
  attributes?: Record<string, string>;
}

// Parse bold text: **text**
export function parseBold(text: string): ParsedElement[] {
  const results: ParsedElement[] = [];
  const pattern = /\*\*([^*]+)\*\*/g;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text)) !== null) {
    results.push({
      type: "bold",
      content: match[1]
    });
  }

  return results;
}

// Parse italic text: *text* or _text_
export function parseItalic(text: string): ParsedElement[] {
  const results: ParsedElement[] = [];
  // Match single * or _ not followed by another
  const pattern = /(?<!\*)\*([^*]+)\*(?!\*)|_([^_]+)_/g;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text)) !== null) {
    results.push({
      type: "italic",
      content: match[1] || match[2]
    });
  }

  return results;
}

// Parse links: [text](url)
export function parseLinks(text: string): ParsedElement[] {
  const results: ParsedElement[] = [];
  const pattern = /\[([^\]]+)\]\(([^)]+)\)/g;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text)) !== null) {
    results.push({
      type: "link",
      content: match[1],
      attributes: { href: match[2] }
    });
  }

  return results;
}

// Parse headers: # Header, ## Header, etc.
export function parseHeaders(text: string): ParsedElement[] {
  const results: ParsedElement[] = [];
  const lines = text.split("\n");
  const pattern = /^(#{1,6})\s+(.+)$/;

  for (const line of lines) {
    const match = pattern.exec(line);
    if (match) {
      results.push({
        type: `h${match[1].length}`,
        content: match[2]
      });
    }
  }

  return results;
}

// Parse code blocks: `code`
export function parseInlineCode(text: string): ParsedElement[] {
  const results: ParsedElement[] = [];
  const pattern = /`([^`]+)`/g;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text)) !== null) {
    results.push({
      type: "code",
      content: match[1]
    });
  }

  return results;
}
