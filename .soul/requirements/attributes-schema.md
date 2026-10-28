# Requirements: Mandatory Attributes Schema

## 1. Overview
In this engineering methodology, every software requirement must be **atomic** (implementable within a single module boundary) and must strictly declare the 9 mandatory attributes described below.

---

## 2. The 9 Mandatory Attributes

| Attribute Name | Format / Syntax | Description & Permitted Values |
| :--- | :--- | :--- |
| **Unique ID** | `REQ-<DOMAIN>-<NAME>-<NUM>` | Globally unique, meaningful requirement identifier (e.g., `REQ-IPC-FRAMING-001`). |
| **Text** | Markdown text | Written in user-story style: *"As a <role>, the system shall <behavior>, so that <benefit>"*. |
| **Criticality** | `High`, `Medium`, `Low` | Delivery and safety prioritization. |
| **SW Component** | `crates/<crate>::<module>` | Target module or crate that holds the implementation. Must match an entry in `docs/archi/module-list.md`. |
| **Implementation Status** | `Not implemented`, `Implemented`, `Verified` | Current lifecycle state of the implementation in the codebase. |
| **Verification Criteria** | Holistic test scenario | Comprehensive end-to-end / user-level acceptance test scenario defining success. |
| **Downstream Req** | `LLD-<DOMAIN>-<NAME>-<NUM>` | Linkage to Low-Level Design requirement documented in the module's local docs. |
| **Downstream Test** | `TEST-VER-<DOMAIN>-<NUM>` | Linkage to the Software Verification Test (SVT) located in `tests/`. |
| **Test Status** | `Passed`, `Failed` | Current execution status of the downstream verification test. |

---

## 3. Standard Requirement Template

```markdown
### REQ-DOMAIN-NAME-001
* **Unique ID:** `REQ-DOMAIN-NAME-001`
* **Text:** As a <role>, the system shall <behavior>, so that <outcome>.
* **Criticality:** High | Medium | Low
* **SW Component:** `crates/<crate>::<module>`
* **Implementation Status:** Not implemented | Implemented | Verified
* **Verification Criteria:** <Detailed step-by-step observable verification criteria>
* **Downstream Req:** `LLD-DOMAIN-NAME-001`
* **Downstream Test:** `TEST-VER-DOMAIN-001`
* **Test Status:** Passed | Failed
```
