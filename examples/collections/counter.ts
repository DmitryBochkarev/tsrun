// Word frequency counter using Map and Set

export interface WordCounter {
    frequencies: Map<string, number>;
    uniqueWords: Set<string>;
    totalWords: number;
}

export function createWordCounter(): WordCounter {
    return {
        frequencies: new Map(),
        uniqueWords: new Set(),
        totalWords: 0
    };
}

export function addWord(counter: WordCounter, word: string): void {
    const normalized: string = word.toLowerCase();

    counter.totalWords++;
    counter.uniqueWords.add(normalized);

    const count: number = counter.frequencies.get(normalized) || 0;
    counter.frequencies.set(normalized, count + 1);
}

export function getCount(counter: WordCounter, word: string): number {
    return counter.frequencies.get(word.toLowerCase()) || 0;
}

export function getMostFrequent(counter: WordCounter, n: number): [string, number][] {
    const entries: [string, number][] = Array.from(counter.frequencies.entries());

    // Sort by frequency descending
    entries.sort((a, b) => b[1] - a[1]);

    // Return top n
    return entries.slice(0, n);
}

export function getLeastFrequent(counter: WordCounter, n: number): [string, number][] {
    const entries: [string, number][] = Array.from(counter.frequencies.entries());

    // Sort by frequency ascending
    entries.sort((a, b) => a[1] - b[1]);

    // Return bottom n
    return entries.slice(0, n);
}

export function analyzeText(text: string): WordCounter {
    const counter: WordCounter = createWordCounter();

    // Split on whitespace and punctuation
    const words: string[] = text.split(/[\s,.!?;:'"()\[\]{}]+/);

    for (const word of words) {
        if (word.length > 0) {
            addWord(counter, word);
        }
    }

    return counter;
}

export function mergeCounters(a: WordCounter, b: WordCounter): WordCounter {
    const result: WordCounter = createWordCounter();

    // Add all words from counter a
    a.frequencies.forEach((count, word) => {
        result.frequencies.set(word, count);
        result.uniqueWords.add(word);
    });

    // Add words from counter b
    b.frequencies.forEach((count, word) => {
        const existing: number = result.frequencies.get(word) || 0;
        result.frequencies.set(word, existing + count);
        result.uniqueWords.add(word);
    });

    result.totalWords = a.totalWords + b.totalWords;

    return result;
}

export function getWordsByFrequency(counter: WordCounter, frequency: number): Set<string> {
    const result: Set<string> = new Set();

    counter.frequencies.forEach((count, word) => {
        if (count === frequency) {
            result.add(word);
        }
    });

    return result;
}

export function getFrequencyDistribution(counter: WordCounter): Map<number, number> {
    const distribution: Map<number, number> = new Map();

    counter.frequencies.forEach((count) => {
        const existing: number = distribution.get(count) || 0;
        distribution.set(count, existing + 1);
    });

    return distribution;
}
