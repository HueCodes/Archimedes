# Collab — known issues

Tracking-doc for CP21 (manual e2e). Picks up where the local smoke
test stopped.

## Open

(none)

## Resolved

### Blank page on first WASM load (`getrandom` panic) — fixed 2026-04-30

**Symptom:** `trunk serve` builds clean, server reports listening, but
`http://127.0.0.1:8080/Archimedes/?room=demo` (and the deployed
`huecodes.github.io/Archimedes/`) showed a fully black canvas with no
UI rendered.

**Cause:** `uuid::Uuid::new_v4()` in `PresenceTracker::new()` triggers
a `getrandom` call during `App::new`. uuid 1.23 pulls `getrandom 0.4`,
and the dep tree also has 0.3 in play (yrs etc.). On
`wasm32-unknown-unknown`, getrandom 0.3+ ships a panic stub unless an
explicit backend is selected — uuid's `js` feature only wires up
getrandom 0.2.

**Fix:**
1. Added explicit wasm32-target deps in `crates/archimedes/Cargo.toml`:
   `getrandom_03 = { package = "getrandom", version = "0.3", features = ["wasm_js"] }`
   and the same for `0.4`.
2. Added `.cargo/config.toml` at the workspace root setting
   `rustflags = ['--cfg', 'getrandom_backend="wasm_js"']` for the
   `wasm32-unknown-unknown` target — getrandom 0.3+ requires the cfg
   flag in addition to the feature.

**Verification:** rebuilt wasm now imports `__wbg_getRandomValues_*`
from JS (visible via `strings dist/*.wasm`); previously this import
was absent and the panic stub was linked instead.
