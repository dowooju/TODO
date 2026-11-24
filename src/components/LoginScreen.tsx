import { useState } from "react";
import { api } from "../api";
import { useApp } from "../store";
import { btnPrimary } from "./common";

/** 첫 실행 시 구글 로그인 화면 (F-23) */
export function LoginScreen() {
  const { google, loadGoogle, skipGoogle, showError } = useApp();
  const [waiting, setWaiting] = useState(false);
  const configured = google?.configured ?? false;

  const login = async () => {
    setWaiting(true);
    try {
      await api.googleLogin();
      await loadGoogle();
    } catch (e) {
      showError(e);
    } finally {
      setWaiting(false);
    }
  };

  return (
    <div className="flex h-full flex-col items-center justify-center bg-brand-50 px-6 text-center">
      <img src="/app-icon.svg" alt="" className="mb-6 h-20 w-20" />
      <h1 className="text-2xl font-bold text-gray-800">내 달력</h1>
      <p className="mt-3 max-w-sm text-sm leading-6 text-gray-600">
        구글 캘린더와 연결해서 시작해요.
        <br />
        날짜를 정한 할 일은 구글 캘린더에도 올라가고, 구글 캘린더 일정도 여기서 함께 보여요.
      </p>

      {configured ? (
        <>
          <button type="button" className={`${btnPrimary} mt-8 flex items-center gap-2 px-6 py-3`} onClick={login} disabled={waiting}>
            <GoogleMark />
            {waiting ? "브라우저에서 로그인을 마쳐 주세요…" : "Google 계정으로 로그인"}
          </button>
          {waiting && (
            <p className="mt-3 text-xs text-gray-500">
              기본 브라우저에 구글 로그인 창이 열렸어요. "확인되지 않은 앱" 경고가 보이면 고급 → 계속을 눌러 주세요.
            </p>
          )}
        </>
      ) : (
        <div className="mt-8 max-w-md rounded-xl border border-brand-200 bg-white p-5 text-left text-sm leading-6 text-gray-600">
          <p className="font-semibold text-gray-800">구글 OAuth 클라이언트 ID 가 아직 설정되지 않았어요.</p>
          <p className="mt-1">
            프로젝트 루트의 <code className="rounded bg-gray-100 px-1">.env</code> 에 클라이언트 ID 를 넣고 다시 빌드하면 구글
            로그인을 쓸 수 있어요. 방법은 README 의 "구글 캘린더 연동 설정" 을 참고하세요.
          </p>
          <button type="button" className={`${btnPrimary} mt-4 w-full`} onClick={skipGoogle}>
            구글 연동 없이 시작하기 (개발용)
          </button>
        </div>
      )}
    </div>
  );
}

function GoogleMark() {
  return (
    <span className="flex h-5 w-5 items-center justify-center rounded-full bg-white">
      <svg viewBox="0 0 48 48" className="h-3.5 w-3.5" aria-hidden>
        <path fill="#EA4335" d="M24 9.5c3.5 0 6.6 1.2 9 3.5l6.7-6.7C35.6 2.4 30.2 0 24 0 14.6 0 6.6 5.4 2.7 13.3l7.8 6C12.4 13.6 17.7 9.5 24 9.5z" />
        <path fill="#4285F4" d="M46.1 24.5c0-1.6-.1-3.1-.4-4.5H24v9h12.4c-.5 2.9-2.2 5.3-4.6 7l7.4 5.7c4.3-4 6.9-9.9 6.9-17.2z" />
        <path fill="#FBBC05" d="M10.5 28.7c-.5-1.4-.8-3-.8-4.7s.3-3.2.8-4.7l-7.8-6C1 16.6 0 20.2 0 24s1 7.4 2.7 10.7l7.8-6z" />
        <path fill="#34A853" d="M24 48c6.5 0 11.9-2.1 15.9-5.8l-7.4-5.7c-2.1 1.4-4.8 2.3-8.5 2.3-6.3 0-11.6-4.1-13.5-9.8l-7.8 6C6.6 42.6 14.6 48 24 48z" />
      </svg>
    </span>
  );
}
