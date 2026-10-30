# LLD-STATE-TIER3-TAILMATCH-004: Tier 3 VT100 Bottom-5 Row Tail Matcher

* **Unique ID:** `LLD-STATE-TIER3-TAILMATCH-004`
* **Upstream Requirement:** `REQ-STATE-TIER3-TAILMATCH-004`
* **Downstream Verification Test:** `TEST-VER-STATE-004` (`tests/e2e_multi_tier_classifier_test.rs`)
* **SW Component:** `crates/agmux-core::tail_matcher`

## 1. Overview
`TailMatcher` inspects only the bottom 5 visible rows of the VT100 screen buffer (`TAIL_ROWS = 5`) to classify:
1. **Rate-limit / Quota Errors (`FAILED` / `ERRORED`):** `"rate limit exceeded"`, `"429 too many requests"`, `"quota exceeded"`, `"insufficient_quota"`.
2. **Blocked Prompts (`WAITING_INPUT`):** `"(y/n)"`, `"[y/n]"`, `"[y/N]"`, `"[Y/n]"`, `"allow?"`, `"approve?"`, `"do you want to proceed"`.
3. **Active Spinners (`RUNNING` / `WORKING`):** Unicode Braille spinner characters (`⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`).
