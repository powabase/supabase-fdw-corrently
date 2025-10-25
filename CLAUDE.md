# CLAUDE.md

This file provides guidance to Claude Code when working with the Corrently GrünstromIndex WASM FDW wrapper.

## Project Overview

**corrently-fdw** is a WebAssembly (WASM) Foreign Data Wrapper for PostgreSQL that enables querying the Corrently GrünstromIndex API (https://api.corrently.io/v2.0) as if it were a native PostgreSQL table.

This wrapper follows the WASM FDW architecture required for hosted Supabase instances and can be used with any Supabase project.

## Project Status

**✅ v0.1.0 - Phase 4 Complete (Ready for CI/CD)**

- **Current Version:** v0.1.0
- **Phase:** Phase 4 - Testing & Validation ✅ COMPLETE
- **Next Phase:** Phase 6 - CI/CD & Release
- **Endpoints:** 1 endpoint (gsi_prediction) - **WORKING**
- **Columns:** 17 fields per forecast hour - **ALL TESTED**
- **WASM Binary:** 106 KB, validated, zero WASI CLI imports ✅
- **Query Performance:** ~300-400ms ✅

## Technology Stack

- **Language:** Rust 1.90.0+
- **Target:** wasm32-unknown-unknown (WebAssembly - NO wasip1!)
- **Framework:** Supabase Wrappers v2 API
- **Build Tool:** cargo-component 0.21.1
- **API:** Corrently GrünstromIndex API v2.0
- **Deployment:** GitHub releases with WASM binaries

## Available Endpoints (v0.1.0 Planned)

| Endpoint | Rows | Data Type | Use Case |
|----------|------|-----------|----------|
| **gsi_prediction** | ~113 | Forecast array | Hourly green energy forecasting |

## Quick Reference

### Build Commands

```bash
# Development build
cargo component build --target wasm32-unknown-unknown

# Production build (optimized for size)
# ⚠️ CRITICAL: Must use wasm32-unknown-unknown (NOT wasm32-wasip1)
cargo component build --release --target wasm32-unknown-unknown

# Verify output
ls -lh target/wasm32-unknown-unknown/release/*.wasm
# Expected: ~130-150 KB (similar to OpenWeather complexity)
```

### Validation Commands

```bash
# Validate WASM structure
wasm-tools validate target/wasm32-unknown-unknown/release/corrently_fdw.wasm

# Check for WASI CLI imports (should be ZERO)
wasm-tools component wit target/wasm32-unknown-unknown/release/corrently_fdw.wasm | grep wasi:cli
# Expected: (no output)

# Calculate checksum
shasum -a 256 target/wasm32-unknown-unknown/release/corrently_fdw.wasm
```

## Key Architecture Decisions

### Pattern: OpenWeather + Energy Charts Hybrid

This FDW uses a **combination of patterns** from reference implementations:

1. **OpenWeather Pattern** (Authentication + Nested JSON):
   - API key authentication via server options
   - Nested JSON objects (timeframe.start, timeframe.end)
   - Safe `.get()` access for all JSON fields

2. **Energy Charts Pattern** (Array Flattening):
   - Flatten forecast array (113 objects) to 113 SQL rows
   - Efficient iteration through forecast data
   - One row per forecast hour

### Critical Implementation Patterns

#### 1. Build Target (Most Common Error!)

**✅ ALWAYS use wasm32-unknown-unknown:**
```bash
cargo component build --release --target wasm32-unknown-unknown
```

**❌ NEVER use wasm32-wasip1:**
- Adds WASI CLI interfaces (stdin/stdout/env)
- Supabase doesn't provide these interfaces
- Causes: `component imports instance 'wasi:cli/environment@0.2.0'`

#### 2. Use .get() Instead of [] (Prevents Panics!)

**✅ Safe:**
```rust
let value = match json_obj.get("field") {
    Some(v) => v,
    None => return Ok(None),
};
```

**❌ Panics if key missing:**
```rust
let value = json_obj["field"];  // Don't do this!
```

#### 3. energyprice is STRING (requires parsing)

The Corrently API returns `energyprice` as a string, not a number:

```rust
// ✅ Correct parsing
let price_str = forecast_obj.get("energyprice")
    .and_then(|v| v.as_str())
    .unwrap_or("0");
let energyprice: f64 = price_str.parse().unwrap_or(0.0);
```

## Development Workflow

### Completed Phases ✅

**Phase 1: Requirements Analysis** ✅
- [x] API research complete
- [x] API_SPECIFICATION.md created
- [x] Sample responses captured

**Phase 2: Repository Setup** ✅
- [x] Git repository initialized
- [x] Directory structure created
- [x] Cargo.toml configured with optimization flags
- [x] WIT file created
- [x] Supabase Wrappers WIT dependencies downloaded
- [x] src/lib.rs stub created

**Phase 3: Implementation** ✅
- [x] 490 lines of Rust implementation
- [x] All 17 columns implemented
- [x] Nested JSON parsing (timeframe.start, timeframe.end)
- [x] String parsing (energyprice)
- [x] Array flattening (113 forecast objects → 113 SQL rows)
- [x] Error handling implemented
- [x] Host version set to ^0.1.0 (critical fix)

**Phase 4: Testing & Validation** ✅
- [x] WASM binary validated (106 KB, zero WASI CLI imports)
- [x] Local Supabase testing complete
- [x] All 17 columns returning data (no NULLs)
- [x] Query performance validated (~300-400ms)
- [x] Edge cases tested (negative prices, nested JSON)
- [x] Test suite created (test_fdw.sql - 12 queries)
- [x] setup_fdw.sql created

**Phase 5: Documentation** ✅
- [x] README.md comprehensive rewrite
- [x] QUICKSTART.md created (3-minute setup)
- [x] docs/endpoints/gsi-prediction.md created (detailed reference)
- [x] CLAUDE.md updated (this file)
- [x] PHASE5_HANDOFF.md (in progress)

### Next Phase: Phase 6 - CI/CD & Release

**Tasks:**
- [ ] Create .github/workflows/release.yml
- [ ] Set up automated builds on tag push
- [ ] Create GitHub release with WASM binary
- [ ] Test release deployment
- [ ] Update documentation with release URLs

## Testing Results (Phase 4)

**WASM Binary:**
- Size: 106 KB (under 150 KB target ✅)
- Checksum: `0747c2f6e9da61d27581b30716d9faa5204044419a4f796d5fb943e23143da02`
- Validation: Zero WASI CLI imports ✅
- Host version: ^0.1.0 (critical requirement)

**Query Performance:**
- Cold start: ~400ms
- Warm queries: ~300ms
- API latency: ~200-300ms
- Parsing overhead: ~50-100ms

**Data Quality:**
- All 17 columns returning data (no NULLs)
- Nested JSON parsing working (timeframe fields)
- String parsing working (energyprice)
- Negative prices handled correctly
- 113 forecast hours returned

**Critical Fixes Applied:**
- Host version changed from ^0.2.0 to ^0.1.0 (Phase 4 discovery)
- Supabase Wrappers local version: 0.1.5
- Requires Supabase restart after WASM changes (cache clearing)

## Known Limitations & Edge Cases

**Handled in v0.1.0:**
- ✅ energyprice string parsing (converts "-0.014000" → -0.014)
- ✅ Nested timeframe access (safe double .get() pattern)
- ✅ Missing zip parameter (clear error message)
- ✅ Negative energy prices (correctly handled, no abs() applied)
- ✅ Bounds checking (all Vec access uses safe .get())

**Not Yet Implemented:**
- ⚠️ import_foreign_schema() - Returns empty vec (manual table creation required)
- ⚠️ Signature field - Not exposed (cryptographic verification field)
- ⚠️ API metadata fields - support, info, documentation not exposed
- ⚠️ Rate limit handling - No retry logic for 429 errors
- ⚠️ Invalid ZIP validation - May return empty results or API error

**API Constraints:**
- Geographic scope: Germany only (German postal codes)
- Forecast window: ~113 hours (variable, API-dependent)
- Rate limiting: 2,000 requests/day (authenticated tier)
- Historical data: Not available via this endpoint
- Parameter validation: API-side (invalid ZIPs may error)

## Documentation

**User Documentation:**
- **README.md** - Comprehensive project overview
- **QUICKSTART.md** - 3-minute setup guide
- **docs/endpoints/gsi-prediction.md** - Complete endpoint reference

**Development Documentation:**
- **Phase 1 Research:** `phase1-research/`
  - `API_SPECIFICATION.md` - Complete API documentation
  - `PHASE1_HANDOFF.md` - Phase 1 → 2 handoff
  - `response_prediction_*.json` - Sample API responses
- **Phase Handoffs:** `PHASE2_HANDOFF.md`, `PHASE3_HANDOFF.md`, `PHASE4_HANDOFF.md`, `PHASE5_HANDOFF.md`
- **Testing:** `test_fdw.sql` (12 comprehensive queries), `setup_fdw.sql`
- **Development Guide:** `/Users/cf/Documents/GitHub/powabase/powabase-backend/docs/fdw-wrappers/DEVELOPMENT_GUIDE.md`

## Version Coordination

**Important:** Keep versions synchronized across:
- `Cargo.toml` - version = "0.1.0"
- `wit/world.wit` - package powabase:supabase-fdw-corrently@0.1.0
- `CLAUDE.md` - Current Version section (this file)

All three must match for successful builds and releases.

## Repository

- **GitHub:** https://github.com/powabase/supabase-fdw-corrently (to be created)
- **Package:** powabase:supabase-fdw-corrently
- **License:** Apache-2.0
