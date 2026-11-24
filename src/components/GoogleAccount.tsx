import { formatDistanceToNow } from "date-fns";
import { ko } from "date-fns/locale";
import { useState } from "react";
import { api } from "../api";
import { useApp } from "../store";
import { btnOutline } from "./common";

/** 설정 창의 구글 계정 항목: 연결된 계정, 동기화 상태, 지금 동기화, 로그아웃 */
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

  const small = `${btnOutline} px-3 py-1 text-xs`;

  return (
    <div className="py-3">
      <p className="text-sm font-semibold">구글 캘린더</p>
      {google.loggedIn ? (
        <>
          <p className="mt-1 truncate text-xs text-gray-500">
            <span className="font-semibold text-gray-700">{google.account ?? "연결됨"}</span> 의 기본 캘린더와 동기화 중
          </p>
          <p className="mt-1 text-xs text-gray-500">
            {google.lastSyncAt
              ? `마지막 동기화: ${formatDistanceToNow(new Date(google.lastSyncAt), { locale: ko, addSuffix: true })}`
              : "아직 동기화하지 않았어요"}
            {!!google.pending && ` · 보낼 변경 ${google.pending}개 대기 중`}
          </p>
          {google.lastError && (
            <p className="mt-1 text-xs text-red-500">
              {google.lastError}
              <span className="text-gray-400"> (인터넷이 연결되면 자동으로 이어서 동기화해요)</span>
            </p>
          )}
          <div className="mt-2 flex gap-2">
            <button type="button" className={small} disabled={busy} onClick={() => run(api.googleSyncNow)}>
              지금 동기화
            </button>
            <button type="button" className={small} disabled={busy} onClick={() => run(api.googleLogout)}>
              로그아웃
            </button>
          </div>
        </>
      ) : (
        <p className="mt-1 text-xs text-gray-500">
          {google.configured ? "로그인되어 있지 않아요." : "OAuth 클라이언트 ID 가 설정되지 않은 개발용 빌드예요."}
        </p>
      )}
    </div>
  );
}
