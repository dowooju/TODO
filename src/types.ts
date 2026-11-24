export interface Task {
  id: string;
  title: string;
  description: string;
  /** YYYY-MM-DD. null 이면 "언젠가 할 일" */
  date: string | null;
  /** HH:MM */
  dueTime: string | null;
  tagId: string | null;
  done: boolean;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export interface TaskInput {
  title: string;
  description: string;
  date: string | null;
  dueTime: string | null;
  tagId: string | null;
}

export type TagColor = "purple" | "yellow" | "sky" | "orange" | "green" | "pink" | "teal" | "gray";

export interface Tag {
  id: string;
  name: string;
  color: TagColor;
  sortOrder: number;
}

export interface TagInput {
  name: string;
  color: TagColor;
}

export interface GoogleStatus {
  /** OAuth 클라이언트 ID 가 들어간 채로 빌드됐는지 */
  configured: boolean;
  loggedIn: boolean;
  account: string | null;
  /** 마지막으로 동기화에 성공한 시각 (ISO) */
  lastSyncAt?: string | null;
  /** 마지막 동기화 오류. 성공하면 비워진다 */
  lastError?: string | null;
  /** 아직 구글로 보내지 못한 변경 수 */
  pending?: number;
}

export interface Settings {
  notificationsEnabled: boolean;
}
