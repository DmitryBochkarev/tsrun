// Duration calculation utilities

export interface Duration {
    days: number;
    hours: number;
    minutes: number;
    seconds: number;
}

const SECONDS_PER_MINUTE: number = 60;
const SECONDS_PER_HOUR: number = 3600;
const SECONDS_PER_DAY: number = 86400;

export function durationToSeconds(duration: Duration): number {
    return duration.days * SECONDS_PER_DAY +
           duration.hours * SECONDS_PER_HOUR +
           duration.minutes * SECONDS_PER_MINUTE +
           duration.seconds;
}

export function secondsToDuration(totalSeconds: number): Duration {
    const negative: boolean = totalSeconds < 0;
    let remaining: number = Math.abs(totalSeconds);

    const days: number = Math.floor(remaining / SECONDS_PER_DAY);
    remaining = remaining % SECONDS_PER_DAY;

    const hours: number = Math.floor(remaining / SECONDS_PER_HOUR);
    remaining = remaining % SECONDS_PER_HOUR;

    const minutes: number = Math.floor(remaining / SECONDS_PER_MINUTE);
    const seconds: number = Math.floor(remaining % SECONDS_PER_MINUTE);

    if (negative) {
        return { days: -days, hours: -hours, minutes: -minutes, seconds: -seconds };
    }

    return { days, hours, minutes, seconds };
}

export function addDuration(a: Duration, b: Duration): Duration {
    const totalSeconds: number = durationToSeconds(a) + durationToSeconds(b);
    return secondsToDuration(totalSeconds);
}

export function subtractDuration(a: Duration, b: Duration): Duration {
    const totalSeconds: number = durationToSeconds(a) - durationToSeconds(b);
    return secondsToDuration(totalSeconds);
}

export function formatDuration(duration: Duration): string {
    const parts: string[] = [];

    if (duration.days !== 0) {
        parts.push(duration.days + "d");
    }
    if (duration.hours !== 0) {
        parts.push(duration.hours + "h");
    }
    if (duration.minutes !== 0) {
        parts.push(duration.minutes + "m");
    }
    if (duration.seconds !== 0 || parts.length === 0) {
        parts.push(duration.seconds + "s");
    }

    return parts.join(" ");
}

export function parseDuration(str: string): Duration {
    const duration: Duration = { days: 0, hours: 0, minutes: 0, seconds: 0 };

    // Match patterns like "5d", "3h", "30m", "45s"
    const dayMatch: RegExpMatchArray | null = str.match(/(\d+)d/);
    const hourMatch: RegExpMatchArray | null = str.match(/(\d+)h/);
    const minuteMatch: RegExpMatchArray | null = str.match(/(\d+)m/);
    const secondMatch: RegExpMatchArray | null = str.match(/(\d+)s/);

    if (dayMatch) {
        duration.days = parseInt(dayMatch[1], 10);
    }
    if (hourMatch) {
        duration.hours = parseInt(hourMatch[1], 10);
    }
    if (minuteMatch) {
        duration.minutes = parseInt(minuteMatch[1], 10);
    }
    if (secondMatch) {
        duration.seconds = parseInt(secondMatch[1], 10);
    }

    return duration;
}

export function multiplyDuration(duration: Duration, factor: number): Duration {
    const totalSeconds: number = durationToSeconds(duration) * factor;
    return secondsToDuration(Math.floor(totalSeconds));
}

export function compareDuration(a: Duration, b: Duration): number {
    return durationToSeconds(a) - durationToSeconds(b);
}

export function isZeroDuration(duration: Duration): boolean {
    return duration.days === 0 &&
           duration.hours === 0 &&
           duration.minutes === 0 &&
           duration.seconds === 0;
}

export function normalizeDuration(duration: Duration): Duration {
    // Normalize by converting to seconds and back
    return secondsToDuration(durationToSeconds(duration));
}
