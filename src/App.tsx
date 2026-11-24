import { useEffect } from "react";
import { onDataChanged } from "./api";
import { DndProvider } from "./components/dnd";
import { MonthCalendar } from "./components/MonthCalendar";
import { SomedayPanel } from "./components/SomedayPanel";
import { TaskDetail } from "./components/TaskDetail";
import { TaskFormModal } from "./components/TaskFormModal";
import { TopBar } from "./components/TopBar";
import { WeekView } from "./components/WeekView";
import { useApp } from "./store";

export default function App() {
  const { load, error, clearError, view, detailId } = useApp();

  useEffect(() => {
    load();
    const unlisten = onDataChanged(() => load());
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [load]);

  useEffect(() => {
    if (!error) return;
    const t = setTimeout(clearError, 4000);
    return () => clearTimeout(t);
  }, [error, clearError]);

  return (
    <div className="flex h-full flex-col">
      <TopBar />

      <DndProvider>
        <main className="flex min-h-0 flex-1 gap-6 p-6">
          <div className="relative min-w-0 flex-1">{view === "month" ? <MonthCalendar /> : <WeekView />}</div>
          {/* 상세가 열려 있으면 언젠가 패널 자리를 대신한다 (F-4) */}
          <div className="w-[300px] shrink-0">{detailId ? <TaskDetail taskId={detailId} /> : <SomedayPanel />}</div>
        </main>
      </DndProvider>

      <TaskFormModal />

      {error && (
        <div
          role="alert"
          className="fixed bottom-5 left-1/2 z-50 -translate-x-1/2 rounded-lg bg-gray-800 px-4 py-2 text-sm text-white shadow-lg"
        >
          {error}
        </div>
      )}
    </div>
  );
}
