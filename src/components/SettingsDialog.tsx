import type { ReactNode } from "react";
import { useApp } from "../store";
import { Modal } from "./common";

/** 설정: 마감 알림 켜기/끄기 (F-20) */
export function SettingsDialog({ onClose }: { onClose: () => void }) {
  const { settings, saveSettings } = useApp();

  return (
    <Modal title="설정" onClose={onClose}>
      <div className="divide-y divide-gray-100">
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
