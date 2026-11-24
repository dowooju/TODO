import { describe, expect, it } from "vitest";
import { dayLabel, monthCells, toKey, weekDays, weekLabel } from "./dates";

describe("dates", () => {
  it("2026년 11월 달력은 일요일(1일)부터 시작해 5주로 그려진다", () => {
    const cells = monthCells(new Date(2026, 10, 1));
    expect(cells[0] && toKey(cells[0])).toBe("2026-11-01");
    expect(cells.length).toBe(35);
    expect(cells.filter(Boolean).length).toBe(30);
  });

  it("앞쪽 빈 칸을 채운다 (2026년 12월 1일은 화요일)", () => {
    const cells = monthCells(new Date(2026, 11, 1));
    expect(cells.slice(0, 2)).toEqual([null, null]);
    expect(cells[2] && toKey(cells[2])).toBe("2026-12-01");
  });

  it("주간 바는 일~토 7일", () => {
    expect(weekDays("2026-11-24").map(toKey)).toEqual([
      "2026-11-22",
      "2026-11-23",
      "2026-11-24",
      "2026-11-25",
      "2026-11-26",
      "2026-11-27",
      "2026-11-28",
    ]);
  });

  it("시안과 같은 라벨을 만든다", () => {
    expect(weekLabel("2026-11-24")).toBe("11월 4주차");
    expect(dayLabel("2026-11-24")).toBe("11월 24일 (화)");
  });
});
