import {
  addDays,
  eachDayOfInterval,
  endOfMonth,
  format,
  getWeekOfMonth,
  parseISO,
  startOfMonth,
  startOfWeek,
} from "date-fns";
import { ko } from "date-fns/locale";

/** Date → "YYYY-MM-DD" (로컬 시간 기준) */
export const toKey = (d: Date) => format(d, "yyyy-MM-dd");
export const fromKey = (key: string) => parseISO(key);
export const todayKey = () => toKey(new Date());

/** 월간 달력에 그릴 날짜들. 일요일 시작, 앞뒤 빈 칸은 null */
export function monthCells(month: Date): (Date | null)[] {
  const first = startOfMonth(month);
  const days = eachDayOfInterval({ start: first, end: endOfMonth(month) });
  const cells: (Date | null)[] = Array(first.getDay()).fill(null);
  cells.push(...days);
  while (cells.length % 7 !== 0) cells.push(null);
  return cells;
}

/** 선택한 날짜가 속한 주(일~토) */
export function weekDays(dateKey: string): Date[] {
  const start = startOfWeek(fromKey(dateKey), { weekStartsOn: 0 });
  return Array.from({ length: 7 }, (_, i) => addDays(start, i));
}

/** "11월 4주차" */
export function weekLabel(dateKey: string): string {
  const d = fromKey(dateKey);
  return `${d.getMonth() + 1}월 ${getWeekOfMonth(d, { weekStartsOn: 0 })}주차`;
}

/** "11월 24일 (화)" */
export const dayLabel = (dateKey: string) => format(fromKey(dateKey), "M월 d일 (EEE)", { locale: ko });

/** "2026-11-24 (화)" */
export const fullDateLabel = (dateKey: string) => format(fromKey(dateKey), "yyyy-MM-dd (EEE)", { locale: ko });

export const monthLabel = (month: Date) => format(month, "yyyy년 M월");

export const WEEKDAY_NAMES = ["일", "월", "화", "수", "목", "금", "토"];

export function weekdayColor(dayIndex: number): string {
  if (dayIndex === 0) return "text-sunday";
  if (dayIndex === 6) return "text-saturday";
  return "text-gray-600";
}
