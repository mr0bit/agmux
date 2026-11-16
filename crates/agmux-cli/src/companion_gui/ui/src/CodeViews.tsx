import { Fragment, useMemo } from "react";
import Prism from "prismjs";
import "prismjs/components/prism-typescript";
import "prismjs/components/prism-jsx";
import "prismjs/components/prism-tsx";
import "prismjs/components/prism-rust";
import { diffLines, files } from "./demo";
import { pairDiffLines, type DiffLine } from "./diff-model";
export function Syntax({
  value,
  language = "tsx",
}: {
  value: string;
  language?: string;
}) {
  const tokens = useMemo(
    () =>
      Prism.tokenize(
        value,
        Prism.languages[language] || Prism.languages.typescript,
      ),
    [value, language],
  );
  const render = (
    token: string | Prism.Token,
    index: number,
  ): React.ReactNode =>
    typeof token === "string" ? (
      <Fragment key={index}>{token}</Fragment>
    ) : (
      <span key={index} className={`token ${token.type}`}>
        {Array.isArray(token.content)
          ? token.content.map(render)
          : typeof token.content === "string"
            ? token.content
            : render(token.content, 0)}
      </span>
    );
  return <>{tokens.map(render)}</>;
}
export function Diff({
  file = 0,
  layout = "unified",
  lines: supplied,
  language = "tsx",
}: {
  file?: number;
  layout?: "unified" | "split";
  lines?: DiffLine[];
  language?: string;
}) {
  const lines: DiffLine[] =
    supplied ??
    (file === 0
      ? (diffLines as DiffLine[])
      : [
          { kind: "hunk", text: "@@ -1,3 +1,5 @@" },
          { kind: "context", old: 1, next: 1, text: `// ${files[file].path}` },
          { kind: "del", old: 2, text: 'const background = "#0b0f19";' },
          { kind: "add", next: 2, text: 'const background = "#171615";' },
          { kind: "add", next: 3, text: 'const surface = "#211e1b";' },
          { kind: "add", next: 4, text: 'const border = "#302c27";' },
          { kind: "context", old: 3, next: 5, text: "export { background };" },
        ]);
  return (
    <div
      className="diff-scroller"
      tabIndex={0}
      aria-label={`${layout === "split" ? "Side-by-side" : "Unified"} diff`}
    >
      {layout === "unified" ? (
        <div className="diff-code" aria-label="Code diff">
          {lines.map((line, index) => (
            <div className={`diff-line ${line.kind}`} key={index}>
              <span className="line-number">{line.old}</span>
              <span className="line-number">{line.next}</span>
              <span className="diff-sign">
                {line.kind === "add" ? "+" : line.kind === "del" ? "−" : ""}
              </span>
              <code>
                <Syntax value={line.text} language={language} />
              </code>
            </div>
          ))}
        </div>
      ) : (
        <div
          className="split-diff"
          aria-label="Code diff"
          style={{
            minWidth: Math.max(
              680,
              (Math.max(...lines.map((line) => line.text.length)) + 8) * 14,
            ),
          }}
        >
          <div className="split-head">
            <span>Before</span>
            <span>After</span>
          </div>
          {pairDiffLines(lines).map((row, index) =>
            row.hunk ? (
              <div className="split-hunk" key={index}>
                {row.hunk}
              </div>
            ) : (
              <div className="split-row" key={index}>
                {([row.left, row.right] as const).map((line, side) => (
                  <div
                    className={`split-cell ${line?.kind || "blank"}`}
                    key={side}
                  >
                    <span className="line-number">
                      {side === 0 ? line?.old : line?.next}
                    </span>
                    <span className="diff-sign">
                      {line?.kind === "add"
                        ? "+"
                        : line?.kind === "del"
                          ? "−"
                          : ""}
                    </span>
                    <code>
                      <Syntax value={line?.text || ""} language={language} />
                    </code>
                  </div>
                ))}
              </div>
            ),
          )}
        </div>
      )}
    </div>
  );
}
