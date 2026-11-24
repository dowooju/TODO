import { useMemo } from "react";
import { WEEKDAY_NAMES, dayLabel, toKey, todayKey, weekDays } from "../lib/dates";
import { paletteOf } from "../lib/tagColors";
import { filterByTag, useApp } from "../store";
import type { Tag, Task } from "../types";
import { Checkbox, TagLabel, addButton } from "./common";

/** ② 날짜 선택 시 — 주간 바 + 해당 날짜 체크리스트 (F-5) */
export function WeekView() {
  return (
    <section className="flex h-full flex-col" aria-label="주간 체크리스트">
      <WeekBar />
      <DayChecklist />
    </section>
  );
}

function WeekBar() {
  const { selectedDate, openDay } = useApp();
  const days = useMemo(() => weekDays(selectedDate), [selectedDate]);
  const today = todayKey();

  return (
    <div className="grid grid-cols-7 border-l border-t border-[#ece3e6]">
      {days.map((d, i) => {
        const key = toKey(d);
        const selected = key === selectedDate;
        return (
          <button
            key={key}
            type="button"
            data-date={key}
            aria-pressed={selected}
            aria-label={`${key} 선택`}
            onClick={() => openDay(key)}
            className={`relative flex flex-col items-center border-b border-r border-[#ece3e6] py-2.5 leading-5 ${
              selected ? "bg-brand-400 font-bold text-white" : "bg-white hover:bg-brand-50"
            }`}
          >
            <span className={selected ? "" : i === 0 ? "text-sunday" : i === 6 ? "text-saturday" : ""}>
              {WEEKDAY_NAMES[i]}
            </span>
            <span>{d.getDate()}</span>
            {key === today && !selected && <span className="absolute top-1.5 right-2 h-1.5 w-1.5 rounded-full bg-brand-400" />}
          </button>
        );
      })}
    </div>
  );
}

function DayChecklist() {
  const { selectedDate, tasks, tags, tagFilter, detailId, openForm } = useApp();
  const items = useMemo(
    () => filterByTag(tasks, tagFilter).filter((t) => t.date === selectedDate),
    [tasks, tagFilter, selectedDate],
  );
  const tagMap = useMemo(() => new Map(tags.map((t) => [t.id, t])), [tags]);

  return (
    <div className="mt-5 flex min-h-0 flex-1 flex-col">
      <h2 className="mb-3 font-bold">{dayLabel(selectedDate)} 체크리스트</h2>
      <ul className="-mr-1 flex-1 space-y-2.5 overflow-y-auto pr-1 pb-1">
        {items.map((t) => (
          <ChecklistItem key={t.id} task={t} tag={t.tagId ? tagMap.get(t.tagId) : undefined} selected={t.id === detailId} />
        ))}
        {items.length === 0 && (
          <li className="py-10 text-center text-sm text-gray-400">
            이 날의 할 일이 없어요.
            <br />
            오른쪽 언젠가 할 일을 끌어와도 돼요.
          </li>
        )}
      </ul>
      <button type="button" className={`${addButton} mt-3`} onClick={() => openForm({ mode: "create", date: selectedDate })}>
        + 할 일 추가
      </button>
    </div>
  );
}

function ChecklistItem({ task, tag, selected }: { task: Task; tag: Tag | undefined; selected: boolean }) {
  const { toggleDone, openDetail } = useApp();
  const p = paletteOf(tag, task.done);

  return (
    <li>
      <div
        role="button"
        tabIndex={0}
        aria-label={`${task.title} 상세 보기`}
        onClick={() => openDetail(selected ? null : task.id)}
        onKeyDown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), openDetail(task.id))}
        className="flex w-full cursor-pointer items-start gap-1.5 rounded-lg px-4 py-3 text-left text-[15px] transition-shadow hover:shadow-sm"
        style={{
          background: p.bg,
          // 선택한 항목은 진한 테두리로 강조 (F-6)
          border: `${selected ? 2 : 1}px solid ${selected ? p.text : p.border}`,
          margin: selected ? 0 : 1,
        }}
      >
        <span className="mt-[3px]">
          <Checkbox checked={task.done} onToggle={() => toggleDone(task.id)} label={`${task.title} 완료`} />
        </span>
        {/* 제목이 길면 줄바꿈해서 전부 보여주고 카드 높이가 늘어난다 (F-11a) */}
        <span className={`min-w-0 flex-1 break-words ${task.done ? "text-gray-400 line-through" : ""} ${selected ? "font-semibold" : ""}`}>
          <TagLabel tag={tag} done={task.done} /> {task.title}
        </span>
        {task.dueTime && <span className="mt-0.5 shrink-0 text-xs text-gray-500">{task.dueTime}</span>}
        <span className="shrink-0 text-gray-400">›</span>
      </div>
    </li>
  );
}
