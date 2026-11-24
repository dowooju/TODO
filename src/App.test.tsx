import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import App from "./App";
import { mockBackend } from "./api/mock";
import { useApp } from "./store";
import { todayKey } from "./lib/dates";
import type { Task } from "./types";

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

beforeEach(() => {
  useApp.setState({
    view: "month",
    detailId: null,
    tagFilter: null,
    form: null,
    selectedDate: todayKey(),
    googleSkipped: true,
  });
});

describe("App", () => {
  it("날짜를 누르면 체크리스트가 열리고, 완료해도 순서가 유지된다", async () => {
    const today = todayKey();
    mockBackend.reset({
      tasks: [
        task("과제 제출", 1, { date: today, tagId: "tag-hw", dueTime: "23:59", description: "테스트 케이스 확인" }),
        task("장보기", 2, { date: today }),
      ],
    });
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: new RegExp(`${today} 할 일 2개`) }));
    const list = await screen.findByRole("region", { name: "주간 체크리스트" });

    await user.click(within(list).getByRole("checkbox", { name: "과제 제출 완료" }));
    const titles = within(list)
      .getAllByRole("button", { name: /상세 보기$/ })
      .map((el) => el.getAttribute("aria-label"));
    expect(titles).toEqual(["과제 제출 상세 보기", "장보기 상세 보기"]);

    // 항목을 누르면 오른쪽 패널이 상세로 바뀐다
    await user.click(within(list).getByRole("button", { name: "장보기 상세 보기" }));
    expect(screen.getByRole("complementary", { name: "할 일 상세" })).toBeInTheDocument();
    expect(screen.queryByRole("complementary", { name: "언젠가 할 일" })).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "✕ 닫기" }));
    expect(screen.getByRole("complementary", { name: "언젠가 할 일" })).toBeInTheDocument();
  });

  it("언젠가 할 일의 '오늘 하기'를 누르면 오늘 날짜로 옮겨진다 (F-12)", async () => {
    mockBackend.reset({ tasks: [task("동생이랑 게임하기", 1)] });
    const user = userEvent.setup();
    render(<App />);

    const panel = await screen.findByRole("complementary", { name: "언젠가 할 일" });
    await within(panel).findByText("동생이랑 게임하기");
    await user.click(within(panel).getByRole("button", { name: "오늘 하기" }));

    expect(await screen.findByRole("button", { name: new RegExp(`${todayKey()} 할 일 1개`) })).toBeInTheDocument();
    expect(within(panel).queryByText("동생이랑 게임하기")).not.toBeInTheDocument();
  });
});

describe("구글 로그인 (F-23)", () => {
  it("클라이언트 ID 가 설정된 빌드에서 로그인하지 않았으면 로그인 화면을 보여준다", async () => {
    mockBackend.reset({ google: { configured: true, loggedIn: false, account: null } });
    useApp.setState({ googleSkipped: false });
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /Google 계정으로 로그인/ }));
    expect(await screen.findByRole("region", { name: "월간 달력" })).toBeInTheDocument();
  });

  it("클라이언트 ID 가 없는 개발 빌드는 건너뛸 수 있다", async () => {
    mockBackend.reset();
    useApp.setState({ googleSkipped: false });
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /구글 연동 없이 시작하기/ }));
    expect(await screen.findByRole("region", { name: "월간 달력" })).toBeInTheDocument();
  });
});
