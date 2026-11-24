import { useState } from "react";
import { api } from "../api";
import { useApp } from "../store";
import { btnOutline } from "./common";

/** 설정 창의 구글 계정 항목: 연결된 계정, 로그아웃 */
export function GoogleAccount() {
  const { google, loadGoogle, showError } = useApp();
  const [busy, setBusy] = useState(false);
  if (!google) return null;

  const run = async (fn: () => Promise<unknown>) => {
    setBusy(true);
    try {
      await fn();
      await loadGoogle();
    } catch (e) {
      showError(e);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="py-3">
      <p className="text-sm font-semibold">구글 캘린더</p>
      {google.loggedIn ? (
        <div className="mt-1 flex items-center gap-3">
          <p className="flex-1 truncate text-xs text-gray-500">
            <span className="font-semibold text-gray-700">{google.account ?? "연결됨"}</span> 의 기본 캘린더와 동기화 중
          </p>
          <button type="button" className={`${btnOutline} px-3 py-1 text-xs`} disabled={busy} onClick={() => run(api.googleLogout)}>
            로그아웃
          </button>
        </div>
      ) : (
        <p className="mt-1 text-xs text-gray-500">
          {google.configured ? "로그인되어 있지 않아요." : "OAuth 클라이언트 ID 가 설정되지 않은 개발용 빌드예요."}
        </p>
      )}
    </div>
  );
}
