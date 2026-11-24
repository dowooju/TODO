import { useEffect } from "react";
import { onDataChanged } from "./api";
import { MonthCalendar } from "./components/MonthCalendar";
import { SomedayPanel } from "./components/SomedayPanel";
import { TaskFormModal } from "./components/TaskFormModal";
import { useApp } from "./store";

export default function App() {
  const { load, error, clearError } = useApp();

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
      <header className="relative flex h-14 shrink-0 items-center justify-center bg-brand-400 text-white">
        <button type="button" aria-label="메뉴" className="absolute left-4 text-xl tracking-widest">
          •••
        </button>
        <h1 className="text-lg font-bold">내 달력</h1>
      </header>

      <main className="flex min-h-0 flex-1 gap-6 p-6">
        <div className="relative min-w-0 flex-1">
          <MonthCalendar />
        </div>
        <div className="w-[300px] shrink-0">
          <SomedayPanel />
        </div>
      </main>

      <TaskFormModal />

      {error && (
        <div role="alert" className="fixed bottom-5 left-1/2 z-50 -translate-x-1/2 rounded-lg bg-gray-800 px-4 py-2 text-sm text-white shadow-lg">
          {error}
        </div>
      )}
    </div>
  );
}
