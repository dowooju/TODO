import type { Tag, TagColor } from "../types";

export interface Palette {
  /** 카드 배경 (파스텔) */
  bg: string;
  /** 카드 테두리, 달력 네모 */
  border: string;
  /** [태그] 글자 (같은 계열의 진한 색) */
  text: string;
  label: string;
}

export const PALETTE: Record<TagColor, Palette> = {
  purple: { bg: "#ede5fc", border: "#c8b4f1", text: "#7a4fc9", label: "보라" },
  yellow: { bg: "#fdf3c2", border: "#efd877", text: "#a77f05", label: "노랑" },
  sky: { bg: "#dcecfd", border: "#a8ccf3", text: "#2f74c0", label: "하늘" },
  orange: { bg: "#fddfc7", border: "#f3b281", text: "#c75b17", label: "주황" },
  green: { bg: "#e1f5d2", border: "#b3de95", text: "#4b9325", label: "연두" },
  pink: { bg: "#fbe0ea", border: "#f0a8c1", text: "#c2457a", label: "분홍" },
  teal: { bg: "#d5f3ee", border: "#92dad0", text: "#1d8b7c", label: "청록" },
  gray: { bg: "#efefef", border: "#d0d0d0", text: "#666666", label: "회색" },
};

export const TAG_COLORS = Object.keys(PALETTE) as TagColor[];

/** 태그 없는 할 일 */
export const NO_TAG: Palette = { bg: "#fdf2f6", border: "#f0c4d4", text: "#b84a73", label: "없음" };

/** 완료한 할 일 */
export const DONE: Palette = { bg: "#ececec", border: "#d6d6d6", text: "#9a9a9a", label: "완료" };

export function paletteOf(tag: Tag | undefined | null, done = false): Palette {
  if (done) return DONE;
  return tag ? PALETTE[tag.color] ?? NO_TAG : NO_TAG;
}
