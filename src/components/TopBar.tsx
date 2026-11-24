import { addDays } from "date-fns";
import { fromKey, toKey, weekLabel } from "../lib/dates";
import { useApp } from "../store";

/** 상단 핑크 바. 월간 화면은 "내 달력", 주간 화면은 "11월 4주차" + 이전/다음 주 */
export function TopBar() {
  const { view, selectedDate, openDay, openMonth } = useApp();
  const shiftWeek = (n: number) => openDay(toKey(addDays(fromKey(selectedDate), n * 7)));

  return (
    <header className="relative flex h-14 shrink-0 items-center justify-center bg-brand-400 text-white">
      <button type="button" aria-label="메뉴" className="absolute left-4 text-xl tracking-widest">
        •••
      </button>

      {view === "month" ? (
        <h1 className="text-lg font-bold">내 달력</h1>
      ) : (
        <div className="flex items-center gap-8">
          <button type="button" aria-label="이전 주" className="text-xl" onClick={() => shiftWeek(-1)}>
            ◀
          </button>
          <h1 className="min-w-28 text-center text-lg font-bold">{weekLabel(selectedDate)}</h1>
          <button type="button" aria-label="다음 주" className="text-xl" onClick={() => shiftWeek(1)}>
            ▶
          </button>
        </div>
      )}

      {view === "week" && (
        <button
          type="button"
          onClick={() => openMonth()}
          className="absolute right-4 rounded-full bg-white/20 px-3 py-1 text-sm font-semibold hover:bg-white/30"
        >
          월간 달력
        </button>
      )}
    </header>
  );
}
