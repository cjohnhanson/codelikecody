---
title: "clc done should commit its changes"
status: done
priority:
assignee:
labels: [fix]
depends_on: []
created: 2026-02-26T00:00:00Z
updated: "2026-02-26T15:11:19Z"
---

clc done sets phase to done and closes the tisket, but those changes need to
be committed to the feature branch so clc merge can verify readiness. The
finalization commit serves as the durable signal that work is complete.
