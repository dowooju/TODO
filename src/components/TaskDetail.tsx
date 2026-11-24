import { useState } from "react";
import { fullDateLabel } from "../lib/dates";
import { paletteOf } from "../lib/tagColors";
import { useApp } from "../store";
import { Checkbox, TagLabel, btnOutline, btnPrimary } from "./common";

/** ③ 체크리스트 항목 선택 시 — 오른쪽 상세 화면 (F-6, F-9) */
export function TaskDetail({ taskId }: { taskId: string }) {
  const { tasks, tags, openDetail, toggleDone, openForm, deleteTask } = useApp();
  const [confirming, setConfirming] = useState(false);
  const task = tasks.find((t) => t.id === taskId);
  if (!task) return null;
  const tag = tags.find((t) => t.id === task.tagId);

  return (
    <aside aria-label="할 일 상세" className="flex h-full flex-col rounded-2xl border border-brand-100 bg-[#fdf8fa] p-4">
      <div className="flex justify-end">
        <button type="button" onClick={() => openDetail(null)} className="text-sm text-gray-400 hover:text-gray-700">
          ✕ 닫기
        </button>
      </div>

      <h2 className={`mt-1 flex items-start gap-1.5 text-[17px] font-bold ${task.done ? "text-gray-400 line-through" : ""}`}>
        <span className="mt-[5px]">
          <Checkbox checked={task.done} onToggle={() => toggleDone(task.id)} label={`${task.title} 완료`} />
        </span>
        <span className="min-w-0 break-words">
          <TagLabel tag={tag} done={task.done} /> {task.title}
        </span>
      </h2>

      <dl className="mt-5 grid grid-cols-[3rem_1fr] gap-y-1.5 text-sm">
        <dt className="text-gray-400">태그</dt>
        <dd>{tag ? <span style={{ color: paletteOf(tag).text }} className="font-bold">[{tag.name}]</span> : <span className="text-gray-400">없음</span>}</dd>
        <dt className="text-gray-400">날짜</dt>
        <dd className="text-gray-700">{task.date ? fullDateLabel(task.date) : "언젠가"}</dd>
        <dt className="text-gray-400">마감</dt>
        <dd className="text-gray-700">{task.dueTime ?? "-"}</dd>
      </dl>

      <h3 className="mt-5 mb-2 text-sm font-bold">상세 설명</h3>
      <div className="min-h-28 flex-1 overflow-y-auto rounded-xl border border-brand-100 bg-white p-3 text-sm leading-6 whitespace-pre-wrap text-gray-700 select-text">
        {task.description || <span className="text-gray-300">설명이 없어요.</span>}
      </div>

      <div className="mt-4 space-y-2">
        <button type="button" className={`${btnPrimary} w-full`} onClick={() => toggleDone(task.id)}>
          {task.done ? "완료 취소" : "완료로 표시"}
        </button>
        {confirming ? (
          <div className="rounded-lg border border-brand-200 bg-white p-2 text-center text-sm">
            <p className="mb-2 text-gray-600">이 할 일을 삭제할까요?</p>
            <div className="flex gap-2">
              <button type="button" className={`${btnOutline} flex-1 py-1.5`} onClick={() => setConfirming(false)}>
                취소
              </button>
              <button type="button" className={`${btnPrimary} flex-1 py-1.5`} onClick={() => deleteTask(task.id)}>
                삭제
              </button>
            </div>
          </div>
        ) : (
          <div className="flex gap-2">
            <button type="button" className={`${btnOutline} flex-1`} onClick={() => openForm({ mode: "edit", taskId: task.id })}>
              수정
            </button>
            <button type="button" className={`${btnOutline} flex-1`} onClick={() => setConfirming(true)}>
              삭제
            </button>
          </div>
        )}
      </div>
    </aside>
  );
}
