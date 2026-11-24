import { useMemo } from "react";
import { todayKey } from "../lib/dates";
import { paletteOf } from "../lib/tagColors";
import { filterByTag, somedayTasks, useApp } from "../store";
import type { Tag, Task } from "../types";
import { Checkbox, TagLabel, addButton } from "./common";
import { useSomedayDroppable, useTaskDraggable } from "./dnd";

/** 오른쪽 "언젠가 할 일" 패널 (F-4, F-8, F-12, F-17) */
export function SomedayPanel() {
  const { tasks, tags, tagFilter, openForm } = useApp();
  const items = useMemo(() => somedayTasks(filterByTag(tasks, tagFilter), tags), [tasks, tags, tagFilter]);
  const tagMap = useMemo(() => new Map(tags.map((t) => [t.id, t])), [tags]);
  // 달력의 할 일을 여기로 끌어 놓으면 날짜가 해제된다 (F-14)
  const { dropRef, isOver } = useSomedayDroppable();

  return (
    <aside
      ref={dropRef}
      aria-label="언젠가 할 일"
      className={`flex h-full flex-col rounded-2xl border p-4 transition-colors ${
        isOver ? "border-brand-400 bg-brand-50" : "border-brand-100 bg-[#fdf8fa]"
      }`}
    >
      <h2 className="mb-3 font-bold">언젠가 할 일</h2>
      <ul className="-mr-1 flex-1 space-y-2.5 overflow-y-auto pr-1">
        {items.map((t) => (
          <SomedayItem key={t.id} task={t} tag={t.tagId ? tagMap.get(t.tagId) : undefined} />
        ))}
        {items.length === 0 && (
          <li className="py-8 text-center text-sm leading-6 text-gray-400">
            날짜를 정하지 않은 할 일이 여기에 모여요.
            <br />
            언젠가 하고 싶은 일을 적어 보세요!
          </li>
        )}
      </ul>
      <button type="button" className={`${addButton} mt-3`} onClick={() => openForm({ mode: "create", date: null })}>
        + 할 일 추가
      </button>
    </aside>
  );
}

function SomedayItem({ task, tag }: { task: Task; tag: Tag | undefined }) {
  const { toggleDone, moveTask, openForm } = useApp();
  const { dragRef, dragProps, isDragging } = useTaskDraggable(task.id);
  const p = paletteOf(tag, task.done);

  return (
    <li
      ref={dragRef}
      {...dragProps}
      className={`group flex cursor-grab items-center gap-1.5 rounded-lg border px-3 py-2.5 text-sm ${
        isDragging ? "opacity-40" : ""
      }`}
      style={{ background: p.bg, borderColor: p.border }}
      onDoubleClick={() => openForm({ mode: "edit", taskId: task.id })}
    >
      <Checkbox checked={task.done} onToggle={() => toggleDone(task.id)} label={`${task.title} 완료`} />
      <TagLabel tag={tag} done={task.done} />
      {/* 한 줄을 넘으면 … 으로 줄인다 (F-11a) */}
      <span className={`min-w-0 flex-1 truncate ${task.done ? "text-gray-400 line-through" : ""}`} title={task.title}>
        {task.title}
      </span>
      {!task.done && (
        <button
          type="button"
          onClick={() => moveTask(task.id, todayKey())}
          onPointerDown={(e) => e.stopPropagation()}
          /* 제목이 최대한 보이도록 마우스를 올렸을 때만 나타난다 */
          className="hidden shrink-0 rounded-full bg-white px-2 py-0.5 text-xs font-semibold text-brand-500 shadow-sm group-focus-within:inline-block group-hover:inline-block hover:bg-brand-50"
        >
          오늘 하기
        </button>
      )}
    </li>
  );
}
