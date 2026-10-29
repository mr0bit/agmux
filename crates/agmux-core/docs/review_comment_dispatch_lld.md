# LLD-GIT-REVIEW-DISPATCH-004: Review comment dispatch

Draft comments are persisted with globally unique IDs and a hash of the annotated line. The daemon serializes dispatches under the storage lock, selects drafts in insertion order, and formats their persisted IDs and locations into a numbered Markdown prompt. It writes the prompt to the active PTY, then atomically records the round, marks its drafts sent, and persists WORKING before broadcasting the transition. A failed PTY write leaves drafts pending. PTY delivery and SQLite cannot be made atomic: a database failure after a successful write can cause a retry to deliver the prompt twice.

The CLI uses authenticated IPC for adding, listing and dispatching comments. The flattener is tested at its public function seam; the verification test exercises the daemon over IPC and checks the PTY output and persisted comment statuses.
