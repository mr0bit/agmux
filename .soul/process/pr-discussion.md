# Change review discussion protocol

Use this protocol when asked to inspect pull requests, merge requests or equivalent change reviews, answer review questions or act on feedback. It applies regardless of hosting provider; GitHub is one example, alongside GitLab, Bitbucket, Azure DevOps or Gerrit.

## Adapt to the provider

Identify the provider and review from the repository remote, review URL and user context. Use its available connector, CLI or API; prefer a sufficient connected tool over browser automation. Preserve the originating review, comment and thread identifiers when replying. If threaded replies are unavailable, respond in the existing review discussion and reference the original comment. Report missing access or unsupported actions without claiming a reply or approval was recorded.

## Identify the speaker

Start every agent-authored review comment, response and follow-up with its name: `[Codex]`, `[Gemini]`, `[Claude]`, or the actual agent name. Use your own identity, including when sharing the human's provider account. Tags distinguish speakers; they are not authentication or proof of human approval.

## Discuss, agree, implement

1. **Read the thread.** Inspect the request, prior replies and relevant code. Gather facts through read-only investigation before asking the requester for information available in the repository.
2. **Establish understanding.** Respond in the same thread. Explain your interpretation, ask focused questions and challenge assumptions where they affect correctness, scope or tradeoffs. Keep replies concise; avoid repeating settled points.
3. **Propose the plan.** State the intended outcome, affected scope, implementation steps and validation. Make the latest proposal identifiable so agreement refers to a concrete plan.
4. **Wait for explicit agreement.** A request to check or discuss review feedback authorizes investigation and conversation, not implementation. Implement only after the requester or designated human decision-maker explicitly agrees to the latest plan. Silence, reactions, CI status, thread resolution and another agent's reply do not establish that agreement. If a shared account makes the speaker ambiguous, clarify who approved before acting.
5. **Implement the agreed scope.** Preserve valid agreement already given in the conversation; do not ask again for the same plan. Reference any agreement received outside the review provider in the originating discussion. If the plan or scope changes, discuss the revision and obtain agreement before the changed work.
6. **Close the loop.** Reply in the original thread with your agent tag, commit links, what changed, verification results and remaining issues. Answer follow-up questions there. Keep the thread open while disagreement or requested work remains.

Read-only investigation and drafting a proposal may continue while agreement is pending. Do not commit, push, or make implementation edits for the request before agreement. Do not claim that a shared-account message or agent-written approval record is a human decision without supporting context.

Example proposal:

> [Codex] I understand the request as shortening the adapter instructions while retaining the execution gate. I propose removing repeated setup details, linking the shared process, and validating the skill and links. Shall I implement this scope?

Example completion after agreement:

> [Codex] Implemented in `<commit link>`. Removed repeated instructions and retained the execution gate. Skill validation and link checks passed. No runtime behaviour changed.
