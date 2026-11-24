/**
 * 끌어오기 (F-13, F-14)
 * - 언젠가 패널 항목 → 달력/주간 바의 날짜 칸: 그 날짜로 지정
 * - 체크리스트 항목 → 언젠가 패널: 날짜 해제
 * - 체크리스트 항목 → 다른 날짜 칸: 날짜 변경
 */
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  pointerWithin,
  useDraggable,
  useDroppable,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
} from "@dnd-kit/core";
import { useState, type ReactNode } from "react";
import { paletteOf } from "../lib/tagColors";
import { useApp } from "../store";

export const SOMEDAY_DROP_ID = "someday";
const dateDropId = (dateKey: string) => `date:${dateKey}`;

export function DndProvider({ children }: { children: ReactNode }) {
  const { tasks, tags, moveTask } = useApp();
  const [activeId, setActiveId] = useState<string | null>(null);
  // 5px 이상 움직여야 드래그로 본다. 그 전에는 일반 클릭으로 처리된다.
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 5 } }));

  const onDragStart = (e: DragStartEvent) => setActiveId(String(e.active.id));
  const onDragEnd = (e: DragEndEvent) => {
    setActiveId(null);
    const task = tasks.find((t) => t.id === e.active.id);
    const over = e.over?.id ? String(e.over.id) : null;
    if (!task || !over) return;
    const target = over === SOMEDAY_DROP_ID ? null : over.startsWith("date:") ? over.slice(5) : undefined;
    if (target === undefined || target === task.date) return;
    moveTask(task.id, target);
  };

  const active = activeId ? tasks.find((t) => t.id === activeId) : undefined;
  const tag = active?.tagId ? tags.find((t) => t.id === active.tagId) : undefined;
  const p = paletteOf(tag, active?.done);

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={pointerWithin}
      onDragStart={onDragStart}
      onDragEnd={onDragEnd}
      onDragCancel={() => setActiveId(null)}
    >
      {children}
      <DragOverlay dropAnimation={null}>
        {active && (
          <div
            className="max-w-64 cursor-grabbing truncate rounded-lg border px-3 py-2 text-sm shadow-lg"
            style={{ background: p.bg, borderColor: p.border }}
          >
            {tag && (
              <span className="mr-1 font-bold" style={{ color: p.text }}>
                [{tag.name}]
              </span>
            )}
            {active.title}
          </div>
        )}
      </DragOverlay>
    </DndContext>
  );
}

export function useTaskDraggable(taskId: string) {
  const { setNodeRef, listeners, attributes, isDragging } = useDraggable({ id: taskId });
  return {
    dragRef: setNodeRef,
    // 키보드 접근성 속성 중 role/tabIndex 는 각 컴포넌트가 직접 정한다
    dragProps: { ...listeners, "aria-roledescription": attributes["aria-roledescription"] },
    isDragging,
  };
}

export function useDateDroppable(dateKey: string) {
  const { setNodeRef, isOver } = useDroppable({ id: dateDropId(dateKey) });
  return { dropRef: setNodeRef, isOver };
}

export function useSomedayDroppable() {
  const { setNodeRef, isOver, active } = useDroppable({ id: SOMEDAY_DROP_ID });
  return { dropRef: setNodeRef, isOver, isDragging: active != null };
}
