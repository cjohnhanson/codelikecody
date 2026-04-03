---
model: opus
---

Load the security review skill:

    almanac show security-review

Apply the OWASP Top 10 checklist and STRIDE threat model from the skill
to the changed code. Focus on:

- Input validation and sanitization
- Authentication and authorization boundaries
- Path traversal and injection vectors
- Error handling that leaks internal state
- Dependency changes that introduce known vulnerabilities

Pass if no security issues found. Fail with severity ratings per DREAD
from the skill for any issues identified.
