import { create } from "zustand";
import { startOfMonth } from "date-fns";
import { api } from "./api";
import { fromKey, todayKey } from "./lib/dates";
import type { GoogleEvent, GoogleStatus, Settings, Tag, TagInput, Task, TaskInput } from "./types";

/** 할 일 추가/수정 폼 상태 */
export type FormState = { mode: "create"; date: string | null } | { mode: "edit"; taskId: string } | null;

interface AppState {
  // ── 데이터 (Rust 가 원본, 여기는 화면용 사본) ──
  tasks: Task[];
  tags: Tag[];
  /** 구글 캘린더 일정 (표시용) */
  googleEvents: GoogleEvent[];
  settings: Settings;
  loaded: boolean;
  error: string | null;

  // ── 화면 상태 ──
  view: "month" | "week";
  /** 월간 달력에 보이는 달 (1일) */
  month: Date;
  /** 주간 화면에서 선택한 날짜 YYYY-MM-DD */
  selectedDate: string;
  /** 오른쪽 상세 패널에 열린 할 일 */
  detailId: string | null;
  /** 태그 필터 (F-18) */
  tagFilter: string | null;
  form: FormState;

  /** 구글 연동 상태. null 이면 아직 확인 전 */
  google: GoogleStatus | null;
  /** 클라이언트 ID 가 없는 개발 빌드에서 로그인을 건너뛰었는지 */
  googleSkipped: boolean;
  loadGoogle: () => Promise<void>;
  skipGoogle: () => void;

  load: () => Promise<void>;
  showError: (e: unknown) => void;
  clearError: () => void;

  openMonth: (month?: Date) => void;
  openDay: (dateKey: string) => void;
  openDetail: (id: string | null) => void;
  setTagFilter: (tagId: string | null) => void;
  openForm: (form: FormState) => void;

  createTask: (input: TaskInput) => Promise<void>;
  updateTask: (id: string, input: TaskInput) => Promise<void>;
  toggleDone: (id: string) => Promise<void>;
  moveTask: (id: string, date: string | null) => Promise<void>;
  deleteTask: (id: string) => Promise<void>;
  saveSettings: (s: Settings) => Promise<void>;

  createTag: (input: TagInput) => Promise<void>;
  updateTag: (id: string, input: TagInput) => Promise<void>;
  deleteTag: (id: string) => Promise<void>;
}

export const useApp = create<AppState>((set, get) => {
  /** 백엔드 호출 후 목록을 다시 불러온다. 실패하면 오류 토스트를 띄운다. */
  const mutate = async (fn: () => Promise<unknown>) => {
    try {
      await fn();
    } catch (e) {
      get().showError(e);
    }
    await get().load();
  };

  return {
    tasks: [],
    tags: [],
    googleEvents: [],
    settings: { notificationsEnabled: true },
    loaded: false,
    error: null,

    view: "month",
    month: startOfMonth(new Date()),
    selectedDate: todayKey(),
    detailId: null,
    tagFilter: null,
    form: null,

    google: null,
    googleSkipped: globalThis.localStorage?.getItem("google-skip") === "1",
    loadGoogle: async () => {
      try {
        set({ google: await api.googleStatus() });
      } catch (e) {
        get().showError(e);
        set({ google: { configured: false, loggedIn: false, account: null } });
      }
    },
    skipGoogle: () => {
      globalThis.localStorage?.setItem("google-skip", "1");
      set({ googleSkipped: true });
    },

    load: async () => {
      try {
        const [tasks, tags, settings, googleEvents] = await Promise.all([
          api.listTasks(),
          api.listTags(),
          api.getSettings(),
          api.listGoogleEvents(),
        ]);
        const { detailId, tagFilter } = get();
        set({
          tasks,
          tags,
          settings,
          googleEvents,
          loaded: true,
          // 지워진 항목/태그를 가리키고 있으면 닫는다 (예: 구글 캘린더에서 삭제됨)
          detailId: detailId && tasks.some((t) => t.id === detailId) ? detailId : null,
          tagFilter: tagFilter && tags.some((t) => t.id === tagFilter) ? tagFilter : null,
        });
      } catch (e) {
        get().showError(e);
      }
    },
    showError: (e) => set({ error: typeof e === "string" ? e : e instanceof Error ? e.message : String(e) }),
    clearError: () => set({ error: null }),

    openMonth: (month) =>
      set((s) => ({ view: "month", month: month ?? startOfMonth(fromKey(s.selectedDate)), detailId: null })),
    openDay: (dateKey) =>
      set((s) => ({
        view: "week",
        selectedDate: dateKey,
        // 다른 날짜로 가면 상세는 닫는다
        detailId: s.selectedDate === dateKey ? s.detailId : null,
      })),
    openDetail: (id) => set({ detailId: id }),
    setTagFilter: (tagId) => set({ tagFilter: tagId }),
    openForm: (form) => set({ form }),

    createTask: (input) => mutate(() => api.createTask(input)),
    updateTask: (id, input) => mutate(() => api.updateTask(id, input)),
    toggleDone: (id) => {
      const task = get().tasks.find((t) => t.id === id);
      if (!task) return Promise.resolve();
      // 체크박스가 바로 반응하도록 먼저 화면을 바꾼다
      set((s) => ({ tasks: s.tasks.map((t) => (t.id === id ? { ...t, done: !t.done } : t)) }));
      return mutate(() => api.setTaskDone(id, !task.done));
    },
    moveTask: (id, date) => {
      set((s) => ({
        tasks: s.tasks.map((t) => (t.id === id ? { ...t, date, dueTime: date ? t.dueTime : null } : t)),
      }));
      return mutate(() => api.setTaskDate(id, date));
    },
    deleteTask: (id) => {
      set((s) => ({ detailId: s.detailId === id ? null : s.detailId }));
      return mutate(() => api.deleteTask(id));
    },
    saveSettings: (settings) => mutate(() => api.saveSettings(settings)),

    createTag: (input) => mutate(() => api.createTag(input)),
    updateTag: (id, input) => mutate(() => api.updateTag(id, input)),
    deleteTag: (id) => mutate(() => api.deleteTag(id)),
  };
});

// ── 파생 데이터 헬퍼 ──

export function filterByTag(tasks: Task[], tagFilter: string | null) {
  return tagFilter ? tasks.filter((t) => t.tagId === tagFilter) : tasks;
}

/** 날짜별 할 일 (생성 순서 유지) */
export function tasksByDate(tasks: Task[]): Map<string, Task[]> {
  const map = new Map<string, Task[]>();
  for (const t of tasks) {
    if (!t.date) continue;
    const list = map.get(t.date) ?? [];
    list.push(t);
    map.set(t.date, list);
  }
  return map;
}

/** 그 날짜에 걸친 구글 일정 (여러 날 일정 포함). 종일 일정이 먼저, 그다음 시작 시각 순 */
export function googleEventsOn(events: GoogleEvent[], dateKey: string): GoogleEvent[] {
  return events
    .filter((e) => e.startDate <= dateKey && dateKey <= e.endDate)
    .sort((a, b) => (a.startTime ?? "").localeCompare(b.startTime ?? ""));
}

/** "언젠가 할 일": 날짜 없는 할 일을 태그 순서대로 묶고, 태그 없는 항목은 마지막 (F-17) */
export function somedayTasks(tasks: Task[], tags: Tag[]): Task[] {
  const order = new Map(tags.map((t, i) => [t.id, i]));
  const rank = (t: Task) => (t.tagId != null && order.has(t.tagId) ? order.get(t.tagId)! : tags.length);
  return tasks.filter((t) => !t.date).sort((a, b) => rank(a) - rank(b) || a.sortOrder - b.sortOrder);
}
