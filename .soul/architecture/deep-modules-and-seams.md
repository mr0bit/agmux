# Architecture: Deep Modules & Seam Discipline

## 1. Deep vs. Shallow Modules
This architecture adheres to the principle of **Deep Modules** (John Ousterhout, *A Philosophy of Software Design*):
* A module is **deep** if it provides powerful functionality through a simple, narrow interface.
* A module is **shallow** if its interface is complex relative to the small amount of logic it implements.

```
       DEEP MODULE                      SHALLOW MODULE
   +-----------------+               +-------------------+
   | Simple Interface|               | Complex Interface |
   +-----------------+               +-------------------+
   |                 |               |                   |
   |   Substantial   |               |   Small amount    |
   | Implementation  |               |  of shallow logic |
   |     Logic       |               +-------------------+
   |                 |
   +-----------------+
```

---

## 2. Seam Discipline
A **seam** is a place where you can alter or test behavior without editing source code (Michael Feathers, *Working Effectively with Legacy Code*).

1. **The Fewer Seams, the Better:** The ideal number of external testing seams across an entire subsystem is **one**.
2. **One Adapter Means Hypothetical; Two Adapters Means Real:** Do not introduce an abstract trait or port unless at least two real implementations exist (e.g. production ConPTY + virtual test PTY). Single-adapter traits are pure indirection and forbidden waste.
3. **Internal vs. External Seams:** Internal seams used by private unit tests must remain private. Never make internal struct fields or methods `pub` solely to facilitate a test.
