# CLAUDE.md

This file provides guidance to Claude Code when working with the Corrently GrünstromIndex WASM FDW wrapper.

## Project Overview

**corrently-fdw** is a WebAssembly (WASM) Foreign Data Wrapper for PostgreSQL that enables querying the Corrently GrünstromIndex API (https://api.corrently.io/v2.0) as if it were a native PostgreSQL table.

This wrapper follows the WASM FDW architecture required for hosted Supabase instances and can be used with any Supabase project.

## Project Status

**🚧 v0.1.0 - In Development (Phase 2 Complete)**

- **Current Version:** v0.1.0
- **Phase:** Phase 2 - Repository Setup ✅ COMPLETE
- **Next Phase:** Phase 3 - Implementation
- **Target Endpoints:** 1 endpoint (gsi_prediction)
- **Target Columns:** 17 fields per forecast hour

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

### Current Phase: Phase 2 Complete ✅

**Completed:**
- [x] Git repository initialized
- [x] Directory structure created
- [x] Cargo.toml configured with optimization flags
- [x] WIT file created
- [x] Supabase Wrappers WIT dependencies downloaded
- [x] src/lib.rs stub created
- [x] cargo check passes
- [x] .gitignore configured

**Next Phase: Phase 3 - Implementation**

See `phase1-research/PHASE1_HANDOFF.md` for complete API specification and implementation plan.

## Documentation

- **Phase 1 Research:** `phase1-research/`
  - `API_SPECIFICATION.md` - Complete API documentation
  - `PHASE1_HANDOFF.md` - Phase 1 → 2 handoff
  - `response_prediction_*.json` - Sample API responses

- **Development Guide:** `/Users/cf/Documents/GitHub/powabase/powabase-backend/docs/fdw-wrappers/DEVELOPMENT_GUIDE.md`

## Version Coordination

**Important:** Keep versions synchronized across:
- `Cargo.toml` - version = "0.1.0"
- `wit/world.wit` - package powabase:supabase-fdw-corrently@0.1.0
- `CLAUDE.md` - Current Version section (this file)

All three must match for successful builds and releases.

## Repository

- **GitHub:** https://github.com/powabase/powabase-fdw-corrently (to be created)
- **Package:** powabase:supabase-fdw-corrently
- **License:** Apache-2.0
