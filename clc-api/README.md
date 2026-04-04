# clc-api

Axum HTTP API wrapping tisket's file-based issue repository.

REST endpoints for listing, creating, editing, closing, and reopening
issues, plus project listing and search. CORS-permissive, stateless
(opens the tisket repo on each request). Can serve static files
alongside the API for single-page app hosting with fallback to
`index.html`.

Used by clc-web as its backend.
