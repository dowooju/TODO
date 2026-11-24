import { useState, type FormEvent } from "react";
import { paletteOf } from "../lib/tagColors";
import { useApp } from "../store";
import type { TaskInput } from "../types";
import { Modal, btnOutline, btnPrimary } from "./common";

/** 할 일 추가 / 수정 (F-7, F-9) */
export function TaskFormModal() {
  const { form, tasks, openForm } = useApp();
  if (!form) return null;
  const existing = form.mode === "edit" ? tasks.find((t) => t.id === form.taskId) : undefined;
  if (form.mode === "edit" && !existing) return null;

  const initial: TaskInput = existing
    ? {
        title: existing.title,
        description: existing.description,
        date: existing.date,
        dueTime: existing.dueTime,
        tagId: existing.tagId,
      }
    : { title: "", description: "", date: form.mode === "create" ? form.date : null, dueTime: null, tagId: null };

  return (
    <TaskForm
      key={existing?.id ?? "new"}
      title={existing ? "할 일 수정" : "할 일 추가"}
      initial={initial}
      editingId={existing?.id}
      onClose={() => openForm(null)}
    />
  );
}

function TaskForm({
  title,
  initial,
  editingId,
  onClose,
}: {
  title: string;
  initial: TaskInput;
  editingId?: string;
  onClose: () => void;
}) {
  const { tags, createTask, updateTask } = useApp();
  const [v, setV] = useState<TaskInput>(initial);
  const [saving, setSaving] = useState(false);
  const patch = (p: Partial<TaskInput>) => setV((prev) => ({ ...prev, ...p }));

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!v.title.trim()) return;
    setSaving(true);
    const input = { ...v, dueTime: v.date ? v.dueTime : null };
    if (editingId) await updateTask(editingId, input);
    else await createTask(input);
    setSaving(false);
    onClose();
  };

  const label = "mb-1 block text-xs font-semibold text-gray-500";
  const field =
    "w-full rounded-lg border border-gray-200 px-3 py-2 text-sm outline-none focus:border-brand-300 focus:ring-2 focus:ring-brand-100";

  return (
    <Modal title={title} onClose={onClose}>
      <form onSubmit={submit} className="space-y-4">
        <div>
          <label className={label} htmlFor="task-title">
            제목 <span className="text-brand-400">*</span>
          </label>
          <input
            id="task-title"
            autoFocus
            className={field}
            value={v.title}
            onChange={(e) => patch({ title: e.target.value })}
            placeholder="예: 언젠가 동생이랑 게임하기"
          />
        </div>

        <div className="flex gap-3">
          <div className="flex-1">
            <label className={label} htmlFor="task-date">
              날짜 <span className="font-normal text-gray-400">(비우면 언젠가 할 일)</span>
            </label>
            <div className="flex gap-1">
              <input
                id="task-date"
                type="date"
                className={field}
                value={v.date ?? ""}
                onChange={(e) => patch({ date: e.target.value || null })}
              />
              {v.date && (
                <button
                  type="button"
                  aria-label="날짜 지우기"
                  className="px-1 text-gray-400 hover:text-gray-700"
                  onClick={() => patch({ date: null, dueTime: null })}
                >
                  ✕
                </button>
              )}
            </div>
          </div>
          <div className="w-32">
            <label className={label} htmlFor="task-time">
              마감 시간
            </label>
            <input
              id="task-time"
              type="time"
              className={`${field} disabled:bg-gray-50 disabled:text-gray-300`}
              disabled={!v.date}
              value={v.dueTime ?? ""}
              onChange={(e) => patch({ dueTime: e.target.value || null })}
            />
          </div>
        </div>

        <div>
          <span className={label}>태그</span>
          <div className="flex flex-wrap gap-1.5" role="radiogroup" aria-label="태그">
            <TagChip selected={v.tagId === null} onClick={() => patch({ tagId: null })} bg="#fff" border="#ddd" text="#888">
              없음
            </TagChip>
            {tags.map((t) => {
              const p = paletteOf(t);
              return (
                <TagChip
                  key={t.id}
                  selected={v.tagId === t.id}
                  onClick={() => patch({ tagId: t.id })}
                  bg={p.bg}
                  border={p.border}
                  text={p.text}
                >
                  {t.name}
                </TagChip>
              );
            })}
          </div>
        </div>

        <div>
          <label className={label} htmlFor="task-desc">
            상세 설명
          </label>
          <textarea
            id="task-desc"
            rows={4}
            className={`${field} resize-none`}
            value={v.description}
            onChange={(e) => patch({ description: e.target.value })}
          />
        </div>

        <div className="flex justify-end gap-2 pt-1">
          <button type="button" className={btnOutline} onClick={onClose}>
            취소
          </button>
          <button type="submit" className={btnPrimary} disabled={!v.title.trim() || saving}>
            {editingId ? "저장" : "추가"}
          </button>
        </div>
      </form>
    </Modal>
  );
}

function TagChip({
  selected,
  onClick,
  bg,
  border,
  text,
  children,
}: {
  selected: boolean;
  onClick: () => void;
  bg: string;
  border: string;
  text: string;
  children: string;
}) {
  return (
    <button
      type="button"
      role="radio"
      aria-checked={selected}
      onClick={onClick}
      className="rounded-full border px-3 py-1 text-xs font-semibold"
      style={{
        background: bg,
        borderColor: selected ? text : border,
        color: text,
        boxShadow: selected ? `0 0 0 1px ${text}` : undefined,
      }}
    >
      {children}
    </button>
  );
}
