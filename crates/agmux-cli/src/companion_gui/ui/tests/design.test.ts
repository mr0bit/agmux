import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { pairDiffLines, type DiffLine } from "../src/diff-model.ts";

test("split diff keeps replacement lines, line numbers, and hunk boundaries", () => {
  const lines: DiffLine[] = [
    { kind: "hunk", text: "@@ -1,2 +1,3 @@" },
    { kind: "context", old: 1, next: 1, text: "start" },
    { kind: "del", old: 2, text: "old" },
    { kind: "add", next: 2, text: "replacement" },
    { kind: "add", next: 3, text: "additional" },
    { kind: "hunk", text: "@@ -8,1 +9,0 @@" },
    { kind: "del", old: 8, text: "deleted" },
  ];
  const rows = pairDiffLines(lines);
  assert.deepEqual(
    rows.filter((row) => row.hunk).map((row) => row.hunk),
    ["@@ -1,2 +1,3 @@", "@@ -8,1 +9,0 @@"],
  );
  assert.deepEqual(
    rows.flatMap((row) => (row.left ? [row.left.text] : [])),
    ["start", "old", "deleted"],
  );
  assert.deepEqual(
    rows.flatMap((row) => (row.right ? [row.right.text] : [])),
    ["start", "replacement", "additional"],
  );
  assert.equal(rows[2].left?.old, 2);
  assert.equal(rows[2].right?.next, 2);
  assert.equal(rows[3].left, undefined);
  assert.equal(rows[5].right, undefined);
});
test("empty and all-added diffs remain representable in split view", () => {
  assert.deepEqual(pairDiffLines([]), []);
  const rows = pairDiffLines([{ kind: "add", text: "new", next: 1 }]);
  assert.equal(rows.length, 1);
  assert.equal(rows[0].left, undefined);
  assert.equal(rows[0].right?.text, "new");
});
const stylesheet = readFileSync(
  new URL("../src/design.css", import.meta.url),
  "utf8",
);
function parsePalette(block: string) {
  return Object.fromEntries(
    [...block.matchAll(/--([\w-]+):\s*(#[\da-f]{6})\s*;/gi)].map((match) => [
      match[1],
      match[2],
    ]),
  );
}
function luminance(color: string) {
  return [0.2126, 0.7152, 0.0722].reduce((sum, weight, index) => {
    const channel =
      parseInt(color.slice(1 + index * 2, 3 + index * 2), 16) / 255;
    return (
      sum +
      weight *
        (channel <= 0.04045
          ? channel / 12.92
          : ((channel + 0.055) / 1.055) ** 2.4)
    );
  }, 0);
}
for (const theme of ["dark", "light"]) {
  test(`${theme} palette has readable text, syntax, and action contrast`, () => {
    const base = parsePalette(
      stylesheet.match(/:root,\s*\.app\s*\{([\s\S]*?)\}/)![1],
    );
    const palette =
      theme === "light"
        ? {
            ...base,
            ...parsePalette(
              stylesheet.match(
                /\.app\[data-theme=["']light["']\]\s*\{([\s\S]*?)\}/,
              )![1],
            ),
          }
        : base;
    for (const [foreground, background] of [
      ["fg-primary", "surface-base"],
      ["fg-secondary", "surface-raised"],
      ["fg-muted", "surface-base"],
      ["syntax-keyword", "surface-inset"],
      ["syntax-string", "surface-inset"],
      ["syntax-comment", "surface-inset"],
      ["success", "success-soft"],
      ["danger", "danger-soft"],
      ["button-text", "button-bg"],
    ]) {
      assert.ok(palette[foreground], foreground);
      assert.ok(palette[background], background);
      const values = [
        luminance(palette[foreground]),
        luminance(palette[background]),
      ].sort((a, b) => a - b);
      const ratio = (values[1] + 0.05) / (values[0] + 0.05);
      assert.ok(
        ratio >= 4.5,
        `${foreground} / ${background}: ${ratio.toFixed(2)}`,
      );
    }
  });
}
