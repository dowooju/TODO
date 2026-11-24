import type { GoogleEvent } from "../types";

/** 구글 캘린더 색(파랑) 왼쪽 막대로 앱 할 일과 구분한다 */
const GOOGLE_BLUE = "#4f7fd6";

function timeLabel(e: GoogleEvent) {
  return e.startTime ?? "종일";
}

/** 월간 달력 칸 안의 한 줄짜리 구글 일정 */
export function GoogleEventChip({ event, compact, onDark }: { event: GoogleEvent; compact?: boolean; onDark?: boolean }) {
  return (
    <span
      title={`${timeLabel(event)} ${event.summary} (구글 캘린더)`}
      className={`block truncate border-l-2 pl-1 leading-4 font-normal ${compact ? "text-[11px]" : "text-xs"} ${
        onDark ? "border-white text-white/90" : "text-gray-600"
      }`}
      style={onDark ? undefined : { borderColor: GOOGLE_BLUE }}
    >
      {event.startTime && <span className={onDark ? "" : "text-gray-400"}>{event.startTime} </span>}
      {event.summary}
    </span>
  );
}

/** 주간 화면의 "구글 캘린더 일정" 목록 (읽기 전용) */
export function GoogleEventList({ events }: { events: GoogleEvent[] }) {
  if (events.length === 0) return null;
  return (
    <section aria-label="구글 캘린더 일정" className="mb-3">
      <ul className="space-y-1.5">
        {events.map((e) => (
          <li
            key={e.id}
            className="flex items-center gap-2 rounded-lg border border-dashed border-[#c5d5f2] bg-white px-4 py-2 text-sm text-gray-700"
          >
            <span className="h-2.5 w-2.5 shrink-0 rounded-full" style={{ background: GOOGLE_BLUE }} aria-hidden />
            <span className="w-24 shrink-0 text-xs text-gray-500">
              {e.startTime ? `${e.startTime} – ${e.endTime ?? ""}` : "종일"}
            </span>
            <span className="min-w-0 flex-1 break-words">{e.summary}</span>
            <span className="shrink-0 text-[11px] text-gray-400">구글 캘린더</span>
          </li>
        ))}
      </ul>
    </section>
  );
}
