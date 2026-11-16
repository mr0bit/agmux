/** Shared timing for deliberate UI actions; streamed text never restarts entry motion. */
export const motionDurations = {
  control: 0.12,
  navigation: 0.2,
  overlay: 0.18,
  disclosure: 0.24,
  panel: 0.28,
  completion: 0.3,
} as const;
export function motionTiming(
  reduced: boolean,
  kind: keyof typeof motionDurations,
) {
  return {
    duration: reduced ? 0 : motionDurations[kind],
    ease: [0.22, 1, 0.36, 1] as [number, number, number, number],
  };
}
export function sessionDirection(
  ids: string[],
  current: string,
  next: string,
  intent?: -1 | 1,
) {
  if (intent) return intent;
  const previousIndex = ids.indexOf(current),
    nextIndex = ids.indexOf(next);
  return previousIndex >= 0 && nextIndex >= 0 && nextIndex < previousIndex
    ? -1
    : 1;
}
