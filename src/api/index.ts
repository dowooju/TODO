/**
 * Rust 백엔드 호출 모음.
 *
 * Tauri 창 안에서는 invoke 로 Rust 명령을 부르고, 일반 브라우저(`npm run dev` 로 화면만 띄울 때)나
 * 테스트에서는 localStorage 기반 mock 으로 동작한다.
 */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Settings, Tag, TagInput, Task, TaskInput } from "../types";
import { mockBackend } from "./mock";

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return isTauri ? invoke<T>(cmd, args) : mockBackend.call<T>(cmd, args);
}

export const api = {
  listTasks: () => call<Task[]>("list_tasks"),
  createTask: (input: TaskInput) => call<Task>("create_task", { input }),
  updateTask: (id: string, input: TaskInput) => call<Task>("update_task", { id, input }),
  setTaskDone: (id: string, done: boolean) => call<Task>("set_task_done", { id, done }),
  setTaskDate: (id: string, date: string | null) => call<Task>("set_task_date", { id, date }),
  deleteTask: (id: string) => call<void>("delete_task", { id }),

  listTags: () => call<Tag[]>("list_tags"),
  createTag: (input: TagInput) => call<Tag>("create_tag", { input }),
  updateTag: (id: string, input: TagInput) => call<Tag>("update_tag", { id, input }),
  deleteTag: (id: string) => call<void>("delete_tag", { id }),

  getSettings: () => call<Settings>("get_settings"),
  saveSettings: (settings: Settings) => call<Settings>("save_settings", { settings }),

  getAutostart: () => call<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => call<boolean>("set_autostart", { enabled }),
};

/** 백엔드에서 데이터가 바뀌었을 때(다른 창, 동기화 등) 알림을 받는다 */
export async function onDataChanged(handler: () => void): Promise<() => void> {
  if (isTauri) return listen("data-changed", handler);
  return mockBackend.subscribe(handler);
}
