import { useEffect, useState, type ReactNode } from "react";
import { api } from "../api";
import { useApp } from "../store";
import { GoogleAccount } from "./GoogleAccount";
import { Modal } from "./common";

/** 설정: 마감 알림 켜기/끄기 (F-20), Windows 시작 시 자동 실행 */
export function SettingsDialog({ onClose }: { onClose: () => void }) {
  const { settings, saveSettings, showError } = useApp();
  // 자동 실행 여부는 DB 가 아니라 Windows 레지스트리에 있으므로 따로 읽는다
  const [autostart, setAutostart] = useState<boolean | null>(null);

  useEffect(() => {
    api.getAutostart().then(setAutostart, showError);
  }, [showError]);

  const toggleAutostart = async (enabled: boolean) => {
    setAutostart(enabled);
    try {
      setAutostart(await api.setAutostart(enabled));
    } catch (e) {
      showError(e);
      setAutostart(!enabled);
    }
  };

  return (
    <Modal title="설정" onClose={onClose}>
      <div className="divide-y divide-gray-100">
        <GoogleAccount />
        <Row
          title="마감 알림"
          desc="날짜와 마감 시간이 있는 할 일을 마감 10분 전에 Windows 알림으로 알려줘요. 완료한 할 일은 알리지 않아요."
        >
          <Toggle
            label="마감 알림"
            checked={settings.notificationsEnabled}
            onChange={(v) => saveSettings({ ...settings, notificationsEnabled: v })}
          />
        </Row>
        <Row
          title="Windows 시작 시 자동 실행"
          desc="컴퓨터를 켜면 트레이에서 조용히 시작해요. 앱이 꺼져 있으면 마감 알림을 받을 수 없어요."
        >
          <Toggle label="Windows 시작 시 자동 실행" checked={!!autostart} onChange={toggleAutostart} />
        </Row>
        <p className="pt-3 text-xs leading-5 text-gray-400">
          창을 닫아도 앱은 작업 표시줄 오른쪽 트레이에 남아 알림을 보내요. 완전히 끄려면 트레이 아이콘을 오른쪽
          클릭해 "종료" 를 누르세요.
        </p>
      </div>
    </Modal>
  );
}

export function Row({ title, desc, children }: { title: string; desc: string; children: ReactNode }) {
  return (
    <div className="flex items-center gap-4 py-3">
      <div className="flex-1">
        <p className="text-sm font-semibold">{title}</p>
        <p className="mt-0.5 text-xs leading-5 text-gray-500">{desc}</p>
      </div>
      {children}
    </div>
  );
}

export function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      onClick={() => onChange(!checked)}
      className={`relative h-6 w-11 shrink-0 rounded-full transition-colors ${checked ? "bg-brand-400" : "bg-gray-300"}`}
    >
      <span
        className={`absolute top-0.5 left-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform ${checked ? "translate-x-5" : ""}`}
      />
    </button>
  );
}
