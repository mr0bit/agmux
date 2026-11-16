import { useState } from "react";
import { ArrowRight, Check, Copy, Monitor, ShieldCheck } from "lucide-react";
export default function ConnectBackend() {
  const [address, setAddress] = useState("http://127.0.0.1:5174/");
  const [copied, setCopied] = useState(false);
  let valid = false;
  try {
    const url = new URL(address);
    valid =
      url.protocol === "http:" &&
      ["127.0.0.1", "localhost", "[::1]"].includes(url.hostname) &&
      !url.username &&
      !url.password &&
      !url.search &&
      !url.hash;
  } catch {
    /* Render the validation hint below. */
  }
  const command =
    "cd crates/agmux-cli/src/companion_gui/ui\nnpm ci\nnpm run live";
  return (
    <div className="connect-backend">
      <p className="modal-description">
        The hosted preview is a design sandbox. Your real agents run on the
        machine with your code, through the Rust companion app.
      </p>
      <div className="connect-step">
        <span>1</span>
        <div>
          <h3>Get the integration code</h3>
          <p>
            Check out the integration branch in your Agmux repository. Node.js
            and a Rust toolchain are required.
          </p>
          <a
            href="https://github.com/mr0bit/agmux/tree/feat/live-companion-workspace"
            target="_blank"
            rel="noreferrer"
          >
            Open integration branch
            <ArrowRight size={12} />
          </a>
        </div>
      </div>
      <div className="connect-step">
        <span>2</span>
        <div>
          <h3>Start the real app</h3>
          <p>
            Run this from the repository root. It builds the UI and starts the
            local companion with the daemon.
          </p>
          <pre>{command}</pre>
          <button
            className="live-action"
            onClick={() => {
              navigator.clipboard
                .writeText(command)
                .then(() => setCopied(true))
                .catch(() => setCopied(false));
            }}
          >
            {copied ? <Check size={13} /> : <Copy size={13} />}{" "}
            {copied ? "Copied" : "Copy commands"}
          </button>
        </div>
      </div>
      <div className="connect-step">
        <span>3</span>
        <div>
          <h3>Open the printed companion URL</h3>
          <p>
            Paste the URL printed by <code>agmux gui</code>. The companion
            supplies its own browser authentication; no API keys belong here.
          </p>
          <label>
            Companion URL
            <input
              aria-label="Companion URL"
              value={address}
              onChange={(event) => setAddress(event.target.value)}
            />
          </label>
          {valid ? (
            <a
              className="primary-button"
              href={address}
              target="_blank"
              rel="noreferrer"
            >
              <Monitor size={14} />
              Open live companion
              <ArrowRight size={14} />
            </a>
          ) : (
            <p className="live-hint">
              Enter the local HTTP URL printed by your companion app.
            </p>
          )}
        </div>
      </div>
      <div className="connect-note">
        <ShieldCheck size={15} />
        <span>
          This opens the app on your own machine. The hosted preview does not
          tunnel into localhost or run your agents.
        </span>
      </div>
    </div>
  );
}
