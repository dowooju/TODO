/**
 * 브라우저 전용 mock 백엔드. Rust 쪽 repo.rs 와 같은 규칙을 흉내 낸다.
 * 실제 앱(Tauri)에서는 쓰이지 않는다.
 */
import type { GoogleEvent, GoogleStatus, Settings, Tag, Task, TaskInput } from "../types";

interface MockData {
  tasks: Task[];
  tags: Tag[];
  settings: Settings;
  autostart?: boolean;
  google?: GoogleStatus;
  googleEvents?: GoogleEvent[];
}

const STORAGE_KEY = "todo-calendar-mock";

const defaultTags: Tag[] = [
  { id: "tag-study", name: "공부", color: "purple", sortOrder: 1 },
  { id: "tag-life", name: "생활", color: "yellow", sortOrder: 2 },
  { id: "tag-job", name: "취업", color: "sky", sortOrder: 3 },
  { id: "tag-hw", name: "과제", color: "orange", sortOrder: 4 },
  { id: "tag-health", name: "운동", color: "green", sortOrder: 5 },
];

function fresh(): MockData {
  return { tasks: [], tags: structuredClone(defaultTags), settings: { notificationsEnabled: true } };
}

function load(): MockData {
  try {
    const raw = globalThis.localStorage?.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw) as MockData;
  } catch {
    /* 손상된 데이터는 무시 */
  }
  return fresh();
}

let data = load();
const listeners = new Set<() => void>();

function save() {
  try {
    globalThis.localStorage?.setItem(STORAGE_KEY, JSON.stringify(data));
  } catch {
    /* 저장 실패는 무시 (테스트 환경 등) */
  }
  listeners.forEach((l) => l());
}

const now = () => new Date().toISOString();
const uuid = () => globalThis.crypto?.randomUUID?.() ?? Math.random().toString(36).slice(2);

function normalize(input: TaskInput) {
  const title = input.title.trim();
  if (!title) throw "제목을 입력해 주세요.";
  const date = input.date || null;
  return {
    title,
    description: input.description ?? "",
    date,
    dueTime: date ? input.dueTime || null : null,
    tagId: input.tagId || null,
  };
}

function getTask(id: string): Task {
  const t = data.tasks.find((x) => x.id === id);
  if (!t) throw `항목을 찾을 수 없습니다: 할 일 ${id}`;
  return t;
}

type Handler = (args: Record<string, any>) => unknown;

const handlers: Record<string, Handler> = {
  list_tasks: () => [...data.tasks].sort((a, b) => a.sortOrder - b.sortOrder),
  create_task: ({ input }) => {
    const ts = now();
    const task: Task = {
      id: uuid(),
      ...normalize(input),
      done: false,
      sortOrder: Math.max(0, ...data.tasks.map((t) => t.sortOrder)) + 1,
      createdAt: ts,
      updatedAt: ts,
    };
    data.tasks.push(task);
    return task;
  },
  update_task: ({ id, input }) => Object.assign(getTask(id), normalize(input), { updatedAt: now() }),
  set_task_done: ({ id, done }) => Object.assign(getTask(id), { done, updatedAt: now() }),
  set_task_date: ({ id, date }) => {
    const t = getTask(id);
    return Object.assign(t, { date: date || null, dueTime: date ? t.dueTime : null, updatedAt: now() });
  },
  delete_task: ({ id }) => {
    data.tasks = data.tasks.filter((t) => t.id !== id);
  },
  list_tags: () => [...data.tags].sort((a, b) => a.sortOrder - b.sortOrder),
  create_tag: ({ input }) => {
    if (!input.name.trim()) throw "태그 이름을 입력해 주세요.";
    const tag: Tag = {
      id: uuid(),
      name: input.name.trim(),
      color: input.color,
      sortOrder: Math.max(0, ...data.tags.map((t) => t.sortOrder)) + 1,
    };
    data.tags.push(tag);
    return tag;
  },
  update_tag: ({ id, input }) => {
    const tag = data.tags.find((t) => t.id === id);
    if (!tag) throw `항목을 찾을 수 없습니다: 태그 ${id}`;
    return Object.assign(tag, { name: input.name.trim(), color: input.color });
  },
  delete_tag: ({ id }) => {
    data.tags = data.tags.filter((t) => t.id !== id);
    data.tasks.forEach((t) => {
      if (t.tagId === id) t.tagId = null;
    });
  },
  get_settings: () => data.settings,
  save_settings: ({ settings }) => (data.settings = settings),
  get_autostart: () => data.autostart ?? true,
  set_autostart: ({ enabled }) => (data.autostart = enabled),
  // 브라우저 미리보기에서는 구글 연동이 설정되지 않은 상태로 본다
  google_status: () => data.google ?? { configured: false, loggedIn: false, account: null },
  google_login: () => (data.google = { configured: true, loggedIn: true, account: "me@example.com" }),
  google_logout: () => (data.google = { configured: true, loggedIn: false, account: null }),
  google_sync_now: () => undefined,
  list_google_events: () => data.googleEvents ?? [],
};

const READ_ONLY = new Set(["list_tasks", "list_tags", "get_settings", "get_autostart", "google_status", "list_google_events"]);

export const mockBackend = {
  async call<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
    const handler = handlers[cmd];
    if (!handler) throw `mock 에서 지원하지 않는 명령: ${cmd}`;
    const result = structuredClone(handler(args)) as T;
    if (!READ_ONLY.has(cmd)) save();
    return result;
  },
  async subscribe(handler: () => void) {
    listeners.add(handler);
    return () => listeners.delete(handler);
  },
  /** 테스트용 초기화 */
  reset(seed?: Partial<MockData>) {
    data = { ...fresh(), ...seed };
    save();
  },
};
