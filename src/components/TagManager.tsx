import { useState } from "react";
import { PALETTE, TAG_COLORS } from "../lib/tagColors";
import { useApp } from "../store";
import type { Tag, TagColor } from "../types";
import { Modal, btnPrimary } from "./common";

/** 태그 만들기 / 이름·색 바꾸기 / 지우기 (F-15) */
export function TagManager({ onClose }: { onClose: () => void }) {
  const { tags, tasks, createTag } = useApp();
  const [name, setName] = useState("");
  const [color, setColor] = useState<TagColor>("pink");

  const add = async () => {
    if (!name.trim()) return;
    await createTag({ name, color });
    setName("");
  };

  return (
    <Modal title="태그 관리" onClose={onClose}>
      <ul className="max-h-72 space-y-2 overflow-y-auto">
        {tags.map((t) => (
          <TagRow key={t.id} tag={t} count={tasks.filter((x) => x.tagId === t.id).length} />
        ))}
      </ul>

      <div className="mt-5 border-t border-gray-100 pt-4">
        <p className="mb-2 text-xs font-semibold text-gray-500">새 태그</p>
        <div className="flex gap-2">
          <input
            aria-label="새 태그 이름"
            className="min-w-0 flex-1 rounded-lg border border-gray-200 px-3 py-2 text-sm outline-none focus:border-brand-300"
            placeholder="예: 독서"
            value={name}
            maxLength={10}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && add()}
          />
          <button type="button" className={btnPrimary} onClick={add} disabled={!name.trim()}>
            추가
          </button>
        </div>
        <ColorPicker value={color} onChange={setColor} />
      </div>
    </Modal>
  );
}

function TagRow({ tag, count }: { tag: Tag; count: number }) {
  const { updateTag, deleteTag } = useApp();
  const [name, setName] = useState(tag.name);
  const [picking, setPicking] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const p = PALETTE[tag.color];

  const saveName = () => {
    if (name.trim() && name.trim() !== tag.name) updateTag(tag.id, { name, color: tag.color });
    else setName(tag.name);
  };

  return (
    <li className="rounded-lg border px-3 py-2" style={{ background: p.bg, borderColor: p.border }}>
      <div className="flex items-center gap-2">
        <button
          type="button"
          aria-label={`${tag.name} 색 바꾸기`}
          className="h-5 w-5 shrink-0 rounded-full border-2 border-white shadow"
          style={{ background: p.text }}
          onClick={() => setPicking((v) => !v)}
        />
        <input
          aria-label={`${tag.name} 이름`}
          className="min-w-0 flex-1 bg-transparent text-sm font-bold outline-none"
          style={{ color: p.text }}
          value={name}
          maxLength={10}
          onChange={(e) => setName(e.target.value)}
          onBlur={saveName}
          onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && (e.target as HTMLInputElement).blur()}
        />
        <span className="text-xs text-gray-500">{count}개</span>
        {confirming ? (
          <span className="flex gap-1 text-xs">
            <button type="button" className="font-semibold text-red-500" onClick={() => deleteTag(tag.id)}>
              삭제
            </button>
            <button type="button" className="text-gray-500" onClick={() => setConfirming(false)}>
              취소
            </button>
          </span>
        ) : (
          <button type="button" aria-label={`${tag.name} 삭제`} className="text-gray-400 hover:text-red-500" onClick={() => setConfirming(true)}>
            ✕
          </button>
        )}
      </div>
      {confirming && count > 0 && (
        <p className="mt-1 text-xs text-gray-500">할 일 {count}개는 지워지지 않고 태그만 없어져요.</p>
      )}
      {picking && (
        <ColorPicker
          value={tag.color}
          onChange={(c) => {
            updateTag(tag.id, { name: tag.name, color: c });
            setPicking(false);
          }}
        />
      )}
    </li>
  );
}

function ColorPicker({ value, onChange }: { value: TagColor; onChange: (c: TagColor) => void }) {
  return (
    <div className="mt-2 flex flex-wrap gap-1.5" role="radiogroup" aria-label="태그 색">
      {TAG_COLORS.map((c) => (
        <button
          key={c}
          type="button"
          role="radio"
          aria-checked={value === c}
          aria-label={PALETTE[c].label}
          title={PALETTE[c].label}
          onClick={() => onChange(c)}
          className="h-7 w-7 rounded-full border-2"
          style={{
            background: PALETTE[c].bg,
            borderColor: value === c ? PALETTE[c].text : PALETTE[c].border,
          }}
        />
      ))}
    </div>
  );
}
