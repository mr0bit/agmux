import { useEffect, useRef, useState } from "react";
import { Compartment, EditorState } from "@codemirror/state";
import {
  EditorView,
  lineNumbers,
  highlightActiveLine,
  drawSelection,
  keymap,
} from "@codemirror/view";
import {
  defaultKeymap,
  history,
  historyKeymap,
  indentWithTab,
} from "@codemirror/commands";
import {
  syntaxHighlighting,
  HighlightStyle,
  bracketMatching,
  indentOnInput,
} from "@codemirror/language";
import { tags } from "@lezer/highlight";

export default function CodeEditor({
  value,
  file,
  light,
  onChange,
  onSave,
}: {
  value: string;
  file: string;
  light: boolean;
  onChange: (value: string) => void;
  onSave: () => void;
}) {
  const [languageError, setLanguageError] = useState("");
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const change = useRef(onChange);
  change.current = onChange;
  const save = useRef(onSave);
  save.current = onSave;
  useEffect(() => {
    let disposed = false;
    const language = new Compartment();
    setLanguageError("");
    const highlighting = HighlightStyle.define([
      {
        tag: [tags.keyword, tags.operatorKeyword, tags.bool],
        color: "var(--syntax-keyword)",
      },
      {
        tag: [tags.string, tags.special(tags.string)],
        color: "var(--syntax-string)",
      },
      {
        tag: tags.comment,
        color: "var(--syntax-comment)",
        fontStyle: "italic",
      },
      { tag: [tags.number, tags.integer], color: "var(--syntax-number)" },
      {
        tag: [tags.function(tags.variableName), tags.className],
        color: "var(--syntax-function)",
      },
      {
        tag: [tags.tagName, tags.attributeName, tags.typeName],
        color: "var(--syntax-tag)",
      },
    ]);
    const editor = new EditorView({
      parent: host.current!,
      state: EditorState.create({
        doc: value,
        extensions: [
          lineNumbers(),
          history(),
          drawSelection(),
          highlightActiveLine(),
          bracketMatching(),
          indentOnInput(),
          language.of([]),
          syntaxHighlighting(highlighting),
          EditorView.contentAttributes.of({
            "aria-label": "Code editor",
            spellcheck: "false",
          }),
          keymap.of([
            {
              key: "Mod-s",
              run: () => {
                save.current();
                return true;
              },
            },
            indentWithTab,
            ...defaultKeymap,
            ...historyKeymap,
          ]),
          EditorView.updateListener.of((update) => {
            if (update.docChanged) change.current(update.state.doc.toString());
          }),
          EditorView.theme(
            {
              "&": {
                height: "100%",
                backgroundColor: "var(--code-bg)",
                color: "var(--text)",
              },
              ".cm-scroller": {
                overflow: "auto",
                fontFamily: "var(--font-mono)",
                lineHeight: "var(--code-leading)",
                fontSize: "12px",
              },
              ".cm-content": { padding: "14px 0" },
              ".cm-gutters": {
                backgroundColor: "var(--code-bg)",
                borderRight: "1px solid var(--line)",
                color: "var(--dim)",
              },
              ".cm-activeLine, .cm-activeLineGutter": {
                backgroundColor: "var(--surface-hover)",
              },
              ".cm-cursor": { borderLeftColor: "var(--text)" },
              "&.cm-focused .cm-selectionBackground, .cm-selectionBackground": {
                backgroundColor: "var(--selection)",
              },
              "&.cm-focused": { outline: "none" },
              ".cm-line": { padding: "0 16px" },
            },
            { dark: !light },
          ),
        ],
      }),
    });
    view.current = editor;
    const loadLanguage = async () =>
      file.endsWith(".rs")
        ? (await import("@codemirror/lang-rust")).rust()
        : file.endsWith(".css")
          ? (await import("@codemirror/lang-css")).css()
          : (await import("@codemirror/lang-javascript")).javascript({
              jsx: true,
              typescript: true,
            });
    loadLanguage()
      .then((extension) => {
        if (!disposed)
          editor.dispatch({ effects: language.reconfigure(extension) });
      })
      .catch(() => {
        if (!disposed)
          setLanguageError(
            "Syntax highlighting couldn't load. Your file is still editable.",
          );
      });
    return () => {
      disposed = true;
      view.current = null;
      editor.destroy();
    };
  }, [file, light]);
  useEffect(() => {
    const editor = view.current;
    if (editor && editor.state.doc.toString() !== value)
      editor.dispatch({
        changes: { from: 0, to: editor.state.doc.length, insert: value },
      });
  }, [value]);
  return (
    <>
      {languageError && (
        <p className="editor-language-error" role="status">
          {languageError}
        </p>
      )}
      <div className="code-mirror-host" ref={host} />
    </>
  );
}
