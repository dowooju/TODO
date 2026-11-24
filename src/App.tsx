import { useEffect } from "react";
import { onDataChanged, onGoogleAuthChanged } from "./api";
import { DndProvider } from "./components/dnd";
import { LoginScreen } from "./components/LoginScreen";
import { MonthCalendar } from "./components/MonthCalendar";
import { SomedayPanel } from "./components/SomedayPanel";
import { TaskDetail } from "./components/TaskDetail";
import { TaskFormModal } from "./components/TaskFormModal";
import { TopBar } from "./components/TopBar";
import { WeekView } from "./components/WeekView";
import { useApp } from "./store";

export default function App() {
  const { load, loadGoogle, google, googleSkipped, error, clearError } = useApp();

  useEffect(() => {
    load();
    loadGoogle();
    const offData = onDataChanged(() => load());
    const offAuth = onGoogleAuthChanged(() => loadGoogle());
    return () => {
      offData.then((fn) => fn());
      offAuth.then((fn) => fn());
    };
  }, [load, loadGoogle]);

  useEffect(() => {
    if (!error) return;
    const t = setTimeout(clearError, 4000);
    return () => clearTimeout(t);
  }, [error, clearError]);

  // 구글 로그인은 필수 (F-23). 클라이언트 ID 없이 빌드한 개발용 앱에서만 건너뛸 수 있다.
  const needsLogin = google !== null && !google.loggedIn && !(googleSkipped && !google.configured);

  return (
    <>
      {google === null ? null : needsLogin ? <LoginScreen /> : <MainScreen />}
      {error && (
        <div
          role="alert"
          className="fixed bottom-5 left-1/2 z-50 -translate-x-1/2 rounded-lg bg-gray-800 px-4 py-2 text-sm text-white shadow-lg"
        >
          {error}
        </div>
      )}
    </>
  );
}

function MainScreen() {
  const { view, detailId } = useApp();
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
    </div>
  );
}
