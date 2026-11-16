export type DiffLine = {
  kind: "context" | "add" | "del" | "hunk";
  text: string;
  old?: number;
  next?: number;
};
export type SplitRow = { hunk?: string; left?: DiffLine; right?: DiffLine };
/** Align replacement blocks while preserving every added and deleted line. */
export function pairDiffLines(lines: DiffLine[]): SplitRow[] {
  const rows: SplitRow[] = [];
  for (let index = 0; index < lines.length; ) {
    const line = lines[index];
    if (line.kind === "hunk") {
      rows.push({ hunk: line.text });
      index++;
    } else if (line.kind === "context") {
      rows.push({ left: line, right: line });
      index++;
    } else {
      const removed: DiffLine[] = [];
      const added: DiffLine[] = [];
      while (
        index < lines.length &&
        (lines[index].kind === "del" || lines[index].kind === "add")
      ) {
        (lines[index].kind === "del" ? removed : added).push(lines[index]);
        index++;
      }
      for (
        let offset = 0;
        offset < Math.max(removed.length, added.length);
        offset++
      )
        rows.push({ left: removed[offset], right: added[offset] });
    }
  }
  return rows;
}
