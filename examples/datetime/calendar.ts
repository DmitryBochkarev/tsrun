// Calendar generation utilities

interface CalendarDay {
    date: number;
    isCurrentMonth: boolean;
}

interface CalendarWeek {
    days: CalendarDay[];
}

interface Calendar {
    year: number;
    month: number;
    monthName: string;
    weeks: CalendarWeek[];
}

const MONTH_NAMES: string[] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December"
];

const DAY_NAMES: string[] = [
    "Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"
];

export function getMonthName(month: number): string {
    return MONTH_NAMES[month];
}

export function getDayName(day: number): string {
    return DAY_NAMES[day];
}

export function getDaysInMonth(year: number, month: number): number {
    // Create date for first day of next month, then go back one day
    return new Date(year, month + 1, 0).getDate();
}

export function getFirstDayOfMonth(year: number, month: number): number {
    return new Date(year, month, 1).getDay();
}

export function generateCalendar(year: number, month: number): Calendar {
    const daysInMonth: number = getDaysInMonth(year, month);
    const firstDay: number = getFirstDayOfMonth(year, month);

    // Get days in previous month for padding
    const prevMonth: number = month === 0 ? 11 : month - 1;
    const prevYear: number = month === 0 ? year - 1 : year;
    const daysInPrevMonth: number = getDaysInMonth(prevYear, prevMonth);

    const weeks: CalendarWeek[] = [];
    let currentWeek: CalendarDay[] = [];

    // Add days from previous month
    for (let i: number = 0; i < firstDay; i++) {
        const date: number = daysInPrevMonth - firstDay + i + 1;
        currentWeek.push({ date, isCurrentMonth: false });
    }

    // Add days from current month
    for (let date: number = 1; date <= daysInMonth; date++) {
        currentWeek.push({ date, isCurrentMonth: true });

        if (currentWeek.length === 7) {
            weeks.push({ days: currentWeek });
            currentWeek = [];
        }
    }

    // Add days from next month to complete the last week
    let nextDate: number = 1;
    while (currentWeek.length > 0 && currentWeek.length < 7) {
        currentWeek.push({ date: nextDate, isCurrentMonth: false });
        nextDate++;
    }

    if (currentWeek.length === 7) {
        weeks.push({ days: currentWeek });
    }

    return {
        year,
        month,
        monthName: getMonthName(month),
        weeks
    };
}

export function printCalendar(calendar: Calendar): void {
    console.log(`\n${calendar.monthName} ${calendar.year}`);
    console.log(DAY_NAMES.join(" "));

    for (const week of calendar.weeks) {
        const row: string = week.days
            .map(day => {
                const str: string = day.date < 10 ? " " + day.date : "" + day.date;
                return day.isCurrentMonth ? str : "  ";
            })
            .join("  ");
        console.log(row);
    }
}

export function getWeekNumber(date: Date): number {
    const firstDayOfYear: Date = new Date(date.getFullYear(), 0, 1);
    const pastDaysOfYear: number = (date.getTime() - firstDayOfYear.getTime()) / 86400000;
    return Math.ceil((pastDaysOfYear + firstDayOfYear.getDay() + 1) / 7);
}

export function isWeekend(date: Date): boolean {
    const day: number = date.getDay();
    return day === 0 || day === 6;
}

export function isWeekday(date: Date): boolean {
    return !isWeekend(date);
}

export function getQuarter(date: Date): number {
    return Math.floor(date.getMonth() / 3) + 1;
}
