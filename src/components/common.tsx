import type { ReactNode } from "react";
import { paletteOf } from "../lib/tagColors";
import type { Tag } from "../types";

/** 시안의 작은 네모 체크박스 */
export function Checkbox({
  checked,
  onToggle,
  label,
}: {
  checked: boolean;
  onToggle: () => void;
  label: string;
}) {
  return (
    <button
      type="button"
      role="checkbox"
      aria-checked={checked}
      aria-label={label}
      onClick={(e) => {
        e.stopPropagation();
        onToggle();
      }}
      onPointerDown={(e) => e.stopPropagation()}
      className="flex h-4 w-4 shrink-0 items-center justify-center rounded-[3px] border border-gray-500 bg-white/70 hover:border-gray-800"
    >
      {checked && (
        <svg viewBox="0 0 12 12" className="h-3 w-3 text-gray-500" aria-hidden>
          <path d="M2.5 6.5l2.2 2.2L9.5 3.8" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
        </svg>
      )}
    </button>
  );
}

/** [공부] 처럼 태그 색 글자로 표시 */
export function TagLabel({ tag, done = false }: { tag: Tag | undefined; done?: boolean }) {
  if (!tag) return null;
  return (
    <span className="shrink-0 font-bold" style={{ color: paletteOf(tag, done).text }}>
      [{tag.name}]
    </span>
  );
}

export function Modal({ title, onClose, children }: { title: string; onClose: () => void; children: ReactNode }) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/25"
      onMouseDown={(e) => e.target === e.currentTarget && onClose()}
    >
      <div role="dialog" aria-label={title} className="w-[440px] max-w-[92vw] rounded-2xl bg-white p-6 shadow-xl">
        <div className="mb-4 flex items-center justify-between">
          <h2 className="text-lg font-bold">{title}</h2>
          <button type="button" onClick={onClose} className="text-gray-400 hover:text-gray-700" aria-label="닫기">
            ✕
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}

export const btnPrimary =
  "rounded-lg bg-brand-400 px-4 py-2 font-semibold text-white hover:bg-brand-500 disabled:opacity-50";
export const btnOutline =
  "rounded-lg border border-brand-400 bg-white px-4 py-2 font-semibold text-brand-400 hover:bg-brand-50";
export const addButton =
  "w-full rounded-lg border border-dashed border-brand-300 py-2.5 text-sm text-brand-400 hover:bg-brand-50";
