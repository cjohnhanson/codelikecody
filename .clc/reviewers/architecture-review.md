---
model: opus
---

Load the architecture evaluation skill:

    almanac show architecture-eval

Apply ATAM quality attribute analysis, coupling/cohesion assessment, and
SOLID at the system level from the skill. Evaluate whether the changes:

- Respect existing module boundaries
- Maintain or improve separation of concerns
- Introduce inappropriate coupling between components
- Follow the established layering (clc-sdk → clc, workspace trait, etc.)

Pass if the architecture is maintained or improved. Fail if the changes
introduce structural problems that will be expensive to unwind later.
