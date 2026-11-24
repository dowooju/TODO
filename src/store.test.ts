import { describe, expect, it } from "vitest";
import { somedayTasks, tasksByDate } from "./store";
import type { Tag, Task } from "./types";

const tag = (id: string, sortOrder: number): Tag => ({ id, name: id, color: "purple", sortOrder });
const task = (id: string, sortOrder: number, extra: Partial<Task> = {}): Task => ({
  id,
  title: id,
  description: "",
  date: null,
  dueTime: null,
  tagId: null,
  done: false,
  sortOrder,
  createdAt: "",
  updatedAt: "",
  ...extra,
});

describe("somedayTasks", () => {
  it("날짜 없는 할 일만, 태그 순서로 묶고 태그 없는 항목은 마지막에 둔다 (F-17)", () => {
    const tags = [tag("study", 1), tag("life", 2)];
    const tasks = [
      task("a", 1, { tagId: "life" }),
      task("b", 2),
      task("c", 3, { tagId: "study" }),
      task("d", 4, { tagId: "life" }),
      task("e", 5, { date: "2026-11-24", tagId: "study" }),
    ];
    expect(somedayTasks(tasks, tags).map((t) => t.id)).toEqual(["c", "a", "d", "b"]);
  });

  it("완료해도 자리가 바뀌지 않는다 (F-10)", () => {
    const tasks = [task("a", 1, { done: true }), task("b", 2)];
    expect(somedayTasks(tasks, []).map((t) => t.id)).toEqual(["a", "b"]);
  });
});

describe("tasksByDate", () => {
  it("날짜별로 묶는다", () => {
    const map = tasksByDate([task("a", 1, { date: "2026-11-24" }), task("b", 2), task("c", 3, { date: "2026-11-24" })]);
    expect(map.get("2026-11-24")?.map((t) => t.id)).toEqual(["a", "c"]);
    expect(map.size).toBe(1);
  });
});
