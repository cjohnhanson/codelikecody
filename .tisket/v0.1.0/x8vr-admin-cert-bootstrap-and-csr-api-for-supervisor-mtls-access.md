---
title: "Admin cert bootstrap and CSR API for supervisor mTLS access"
status: discovery
priority: 2
assignee:
labels: [supervisor, security]
depends_on: []
created: "2026-03-25T01:11:50Z"
updated: "2026-03-25T01:11:50Z"
---

The supervisor API uses mTLS. Workspace agents get certs via SSH at
creation time. The human admin has no way to authenticate — `curl`
and `clc` commands from the host can't reach the API.

Kubernetes solves this with a bootstrap cert written to disk at init
time (kubeconfig), protected by filesystem permissions, plus a
CertificateSigningRequest API for runtime cert issuance.

Ref: https://kubernetes.io/docs/reference/access-authn-authz/certificate-signing-requests/
Ref: https://kubernetes.io/docs/setup/best-practices/certificates/
