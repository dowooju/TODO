import { useEffect, useRef, useState } from "react";
import { PALETTE } from "../lib/tagColors";
import { useApp } from "../store";
import { TagManager } from "./TagManager";

type Dialog = "tags" | null;

/** 상단 바 왼쪽 ••• 메뉴: 태그 필터, 태그 관리 */
export function Menu() {
  const { tags, tagFilter, setTagFilter } = useApp();
  const [open, setOpen] = useState(false);
  const [dialog, setDialog] = useState<Dialog>(null);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => !ref.current?.contains(e.target as Node) && setOpen(false);
    const esc = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    document.addEventListener("mousedown", close);
    document.addEventListener("keydown", esc);
    return () => {
      document.removeEventListener("mousedown", close);
      document.removeEventListener("keydown", esc);
    };
  }, [open]);

  const show = (d: Dialog) => {
    setOpen(false);
    setDialog(d);
  };

  const item = "flex w-full items-center gap-2 rounded-md px-3 py-1.5 text-left text-sm hover:bg-brand-50";

  return (
    <div ref={ref} className="absolute left-4 flex items-center gap-2">
      <button type="button" aria-label="메뉴" aria-expanded={open} className="text-xl tracking-widest" onClick={() => setOpen((v) => !v)}>
        •••
      </button>

      {open && (
        <div role="menu" className="absolute top-10 left-0 z-40 w-52 rounded-xl border border-gray-100 bg-white p-1.5 text-gray-800 shadow-xl">
          <p className="px-3 pt-1 pb-1 text-xs font-semibold text-gray-400">태그로 보기</p>
          <button type="button" role="menuitemradio" aria-checked={!tagFilter} className={item} onClick={() => (setTagFilter(null), setOpen(false))}>
            <span className="w-3 text-brand-500">{!tagFilter && "✓"}</span>전체
          </button>
          {tags.map((t) => (
            <button
              key={t.id}
              type="button"
              role="menuitemradio"
              aria-checked={tagFilter === t.id}
              className={item}
              onClick={() => (setTagFilter(t.id), setOpen(false))}
            >
              <span className="w-3 text-brand-500">{tagFilter === t.id && "✓"}</span>
              <span className="font-bold" style={{ color: PALETTE[t.color].text }}>
                [{t.name}]
              </span>
            </button>
          ))}
          <div className="my-1 border-t border-gray-100" />
          <button type="button" role="menuitem" className={item} onClick={() => show("tags")}>
            태그 관리…
          </button>
        </div>
      )}

      <FilterChip />
      {dialog === "tags" && <TagManager onClose={() => setDialog(null)} />}
    </div>
  );
}

/** 필터가 켜져 있으면 메뉴 옆에 표시하고 바로 끌 수 있게 한다 (F-18) */
function FilterChip() {
  const { tags, tagFilter, setTagFilter } = useApp();
  const tag = tags.find((t) => t.id === tagFilter);
  if (!tag) return null;
  return (
    <span className="flex items-center gap-1 rounded-full bg-white px-2.5 py-0.5 text-xs font-bold" style={{ color: PALETTE[tag.color].text }}>
      [{tag.name}]만 보기
      <button type="button" aria-label="필터 해제" className="text-gray-400 hover:text-gray-700" onClick={() => setTagFilter(null)}>
        ✕
      </button>
    </span>
  );
}
