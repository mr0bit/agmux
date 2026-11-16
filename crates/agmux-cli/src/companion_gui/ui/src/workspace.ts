import type { Session } from "./demo";

export type QueuedMessage = { id: string; text: string; attachments: string[] };
export type Layout = {
  sidebar: boolean;
  review: boolean;
  reviewWidth: number;
  wideReview?: boolean;
};
export const statuses: Record<
  Session["state"],
  { label: string; color: string }
> = {
  attention: { label: "Waiting", color: "amber" },
  working: { label: "Working", color: "green" },
  idle: { label: "Ready", color: "gray" },
  reconnecting: { label: "Reconnecting", color: "amber" },
  stopped: { label: "Stopped", color: "gray" },
};
export function clampReviewWidth(width: number, available: number) {
  return Math.round(
    Math.max(
      280,
      Math.min(Number.isFinite(width) ? width : 420, 900, available - 340),
    ),
  );
}
export function nextSessionId(
  sessions: Session[],
  current: string,
  delta: number,
) {
  if (!sessions.length) return current;
  const index = Math.max(
    0,
    sessions.findIndex((item) => item.id === current),
  );
  return sessions[(index + delta + sessions.length) % sessions.length].id;
}
export function moveQueuedMessage(
  queue: QueuedMessage[],
  id: string,
  delta: number,
) {
  const result = [...queue];
  const from = result.findIndex((item) => item.id === id);
  const to = from + delta;
  if (from < 0 || to < 0 || to >= result.length) return queue;
  [result[from], result[to]] = [result[to], result[from]];
  return result;
}
