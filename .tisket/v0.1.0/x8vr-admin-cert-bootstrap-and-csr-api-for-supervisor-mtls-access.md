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

The supervisor API uses mTLS. Workspace agents get certs at creation
time via SSH. But the human admin has no way to authenticate — `curl`
and `clc` commands from the host can't reach the API.

## Kubernetes model (reference)

Kubernetes solves this with a two-phase approach:

**Phase 1 — Bootstrap (kubeadm init):**
- CA generated and stored in `/etc/kubernetes/pki/`
- Admin client cert generated alongside CA at init time
- Written to `/etc/kubernetes/admin.conf` (kubeconfig format)
- Protected by filesystem permissions (root-only)
- Human copies to `~/.kube/config` to get kubectl access
- This is the initial trust anchor — no unauthenticated endpoint

**Phase 2 — Runtime cert issuance (CertificateSigningRequest API):**
- Clients generate a key pair locally, create a PKCS#10 CSR
- Submit CSR to the API as a CertificateSigningRequest resource
- Controller or admin approves/denies
- CA signs on approval, signed cert attached to CSR resource
- Client retrieves its cert
- Can be automated (kubelet rotation) or manual (`kubectl certificate approve`)
- All through the authenticated API — bootstrap cert required

Key: no unauthenticated endpoints. Chain of trust rooted in the
filesystem-protected initial admin cert.

Ref: https://kubernetes.io/docs/reference/access-authn-authz/certificate-signing-requests/

## Proposed design for clc

**Phase 1 — Bootstrap (`clc up` startup):**
- Supervisor generates ephemeral CA (already done)
- Supervisor also generates an admin client cert signed by the CA
- Writes admin cert + key + CA cert to `.clc/admin.json` with 0600 perms
- Format: `{ "api_url": "https://...", "ca_cert": "...", "client_cert": "...", "client_key": "..." }`
- `clc` commands on the host detect `.clc/admin.json` and use it for mTLS
- Admin cert has role=admin in the DN, bypasses workspace restrictions

**Phase 2 — Runtime CSR API (future):**
- `POST /certs/request` — submit a CSR, admin approves
- For: external workspaces joining the cluster, cert rotation
- Requires admin cert to approve — chain of trust

## Acceptance criteria

1. `clc up` writes `.clc/admin.json` at startup with 0600 permissions
2. `clc` commands on the host auto-detect and use the admin cert
3. `curl` with `--cert`/`--key`/`--cacert` flags can reach the API
4. Admin cert has short expiration (matches supervisor session lifetime)
5. Workspace certs and admin cert have different roles in the DN
6. API enforces role-based access (admin can do everything, workers restricted)
