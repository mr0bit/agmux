# Cold-start onboarding

`LLD-TUI-COLD-START-006`: Following the initial daemon session list, an empty list and absent local agmux.toml open the wizard. It displays PATH discovery and the local Git root when available; arrow keys select the shell or an installed agent. Enter creates configuration and queues a spawn request in the repository root. File errors remain visible in the modal. A successful spawn subscribes to the new viewport before refreshing the session list.
