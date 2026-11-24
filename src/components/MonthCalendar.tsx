import { addMonths, isSameMonth } from "date-fns";
import { useMemo } from "react";
import { WEEKDAY_NAMES, monthCells, monthLabel, toKey, todayKey, weekdayColor } from "../lib/dates";
import { paletteOf } from "../lib/tagColors";
import { filterByTag, googleEventsOn, tasksByDate, useApp } from "../store";
import type { GoogleEvent, Tag, Task } from "../types";
import { GoogleEventChip } from "./GoogleEvents";
import { useDateDroppable } from "./dnd";

/** ① 초기 화면 — 월간 달력 (F-1 ~ F-3) */
export function MonthCalendar() {
  const { month, tasks, tags, tagFilter, googleEvents, openMonth, openDay } = useApp();
  const cells = useMemo(() => monthCells(month), [month]);
  const byDate = useMemo(() => tasksByDate(filterByTag(tasks, tagFilter)), [tasks, tagFilter]);
  const tagMap = useMemo(() => new Map(tags.map((t) => [t.id, t])), [tags]);
  const today = todayKey();
  const isCurrentMonth = isSameMonth(month, new Date());

  return (
    <section className="flex h-full flex-col" aria-label="월간 달력">
      <div className="relative mb-3 flex items-center justify-center gap-8">
        <button type="button" aria-label="이전 달" onClick={() => openMonth(addMonths(month, -1))} className="px-2 text-xl">
          ◀
        </button>
        <h2 className="min-w-32 text-center text-lg font-bold">{monthLabel(month)}</h2>
        <button type="button" aria-label="다음 달" onClick={() => openMonth(addMonths(month, 1))} className="px-2 text-xl">
          ▶
        </button>
        {!isCurrentMonth && (
          <button
            type="button"
            onClick={() => openMonth(new Date(new Date().getFullYear(), new Date().getMonth(), 1))}
            className="absolute right-0 rounded-full border border-brand-300 px-3 py-0.5 text-xs text-brand-500 hover:bg-brand-50"
          >
            오늘
          </button>
        )}
      </div>

      <div className="grid grid-cols-7 pb-1 text-center text-xs">
        {WEEKDAY_NAMES.map((name, i) => (
          <div key={name} className={weekdayColor(i)}>
            {name}
          </div>
        ))}
      </div>

      <div className="grid flex-1 grid-cols-7 border-l border-t border-[#ece3e6]" style={{ gridAutoRows: "1fr" }}>
        {cells.map((date, i) =>
          date ? (
            <DayCell
              key={toKey(date)}
              dateKey={toKey(date)}
              day={date.getDate()}
              isToday={toKey(date) === today}
              tasks={byDate.get(toKey(date)) ?? []}
              // 태그로 거를 때는 태그가 없는 구글 일정은 숨긴다
              events={tagFilter ? [] : googleEventsOn(googleEvents, toKey(date))}
              tagMap={tagMap}
              onOpen={() => openDay(toKey(date))}
            />
          ) : (
            <div key={`empty-${i}`} className="border-b border-r border-[#ece3e6] bg-[#fcfafb]" />
          ),
        )}
      </div>
    </section>
  );
}

function DayCell({
  dateKey,
  day,
  isToday,
  tasks,
  events,
  tagMap,
  onOpen,
}: {
  dateKey: string;
  day: number;
  isToday: boolean;
  tasks: Task[];
  events: GoogleEvent[];
  tagMap: Map<string, Tag>;
  onOpen: () => void;
}) {
  const { dropRef, isOver } = useDateDroppable(dateKey);
  return (
    <button
      ref={dropRef}
      type="button"
      data-date={dateKey}
      aria-label={`${dateKey} 할 일 ${tasks.length}개${events.length ? `, 구글 일정 ${events.length}개` : ""}`}
      onClick={onOpen}
      className={`flex min-h-16 flex-col items-start border-b border-r border-[#ece3e6] p-1.5 text-left text-sm transition-colors ${
        isToday ? "bg-brand-400 font-bold text-white" : "bg-white hover:bg-brand-50"
      } ${isOver ? "relative z-10 ring-2 ring-brand-500 ring-inset" : ""}`}
    >
      <span className="pl-1">{day}</span>
      {/* 구글 캘린더 일정: 앱 할 일(색 네모)과 구분되게 글자 줄로 표시 (F-24) */}
      {events.length > 0 && (
        <div className="mt-0.5 w-full min-w-0 space-y-px">
          {events.slice(0, 2).map((e) => (
            <GoogleEventChip key={e.id} event={e} compact onDark={isToday} />
          ))}
          {events.length > 2 && (
            <span className={`block pl-1 text-[10px] ${isToday ? "text-white/80" : "text-gray-400"}`}>+{events.length - 2}</span>
          )}
        </div>
      )}
      {/* 할 일 1개당 작은 색 네모 1개. 완료한 할 일은 회색 */}
      <div className="mt-auto flex flex-wrap gap-[3px] pb-0.5 pl-1">
        {tasks.map((t) => {
          const p = paletteOf(t.tagId ? tagMap.get(t.tagId) : undefined, t.done);
          return (
            <span
              key={t.id}
              title={t.title}
              className="h-[11px] w-[11px] rounded-[2px] border"
              style={{ background: p.bg, borderColor: p.border }}
            />
          );
        })}
      </div>
    </button>
  );
}
