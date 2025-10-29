# CLAUDE.md

This file provides guidance to Claude Code when working with the Corrently GrünstromIndex WASM FDW wrapper.

## Project Overview

**corrently-fdw** is a WebAssembly (WASM) Foreign Data Wrapper for PostgreSQL that enables querying the Corrently GrünstromIndex API (https://api.corrently.io/v2.0) as if it were a native PostgreSQL table.

This wrapper follows the WASM FDW architecture required for hosted Supabase instances and can be used with any Supabase project.

## Project Status

**✅ v0.2.1 - Released**

- **Current Version:** v0.2.1
- **Status:** Released and production-ready
- **Endpoints:** 1 endpoint (gsi_prediction)
- **Columns:** 16 fields per forecast hour (removed `epochtime` from v0.1.0)
- **WASM Binary:** ~106 KB, validated, zero WASI CLI imports ✅
- **Query Performance:** ~300-400ms ✅
- **Security:** Vault support for API keys (Recommended), plain text deprecated
- **Compatibility:** 100% backward compatible with v0.2.0

## Technology Stack

- **Language:** Rust 1.90.0+
- **Target:** wasm32-unknown-unknown (WebAssembly - NO wasip1!)
- **Framework:** Supabase Wrappers v2 API
- **Build Tool:** cargo-component 0.21.1
- **API:** Corrently GrünstromIndex API v2.0
- **Deployment:** GitHub releases with WASM binaries

## Available Endpoints

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
# Expected: ~106 KB
```

### Validation Commands

```bash
# Validate WASM structure
wasm-tools validate target/wasm32-unknown-unknown/release/corrently_fdw.wasm

# Check for WASI CLI imports (should be ZERO)
wasm-tools component wit target/wasm32-unknown-unknown/release/corrently_fdw.wasm | grep wasi:cli
# Expected: (no output)

# Calculate checksum for v0.2.0
shasum -a 256 target/wasm32-unknown-unknown/release/corrently_fdw.wasm
# Expected: 6f182a640568669afa6294641aa074bb13a332b146516ae199505ff470d94b18
```

## Key Architecture Decisions

### v0.2.0 Standards Compliance

**Breaking Changes from v0.1.0:**
- All 16 columns renamed for clarity and standards compliance
- 4 temporal columns changed from BIGINT (Unix milliseconds) to TIMESTAMP WITH TIME ZONE
- Removed redundant `epochtime` field (was duplicate of `timestamp`)
- Parameter renamed: `zip` → `postal_code`

**Standards Alignment:**
- Follows PostgreSQL best practices for column naming and type selection
- Native PostgreSQL types for better query optimization
- AI-friendly column names for automated query generation
- Explicit units in column names (e.g., `_eur_kwh`, `_g_kwh`, `_pct`)

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

#### 3. Milliseconds to Microseconds Conversion (v0.2.0)

For native TIMESTAMP WITH TIME ZONE support:

```rust
// Convert API milliseconds to PostgreSQL microseconds
let timestamp_us = (ms_value as i64) * 1000;
let timestamp_str = format_timestamp_with_tz(timestamp_us);
```

#### 4. energyprice is STRING (requires parsing)

The Corrently API returns `energyprice` as a string, not a number:

```rust
// ✅ Correct parsing
let price_str = forecast_obj.get("energyprice")
    .and_then(|v| v.as_str())
    .unwrap_or("0");
let energyprice: f64 = price_str.parse().unwrap_or(0.0);
```

## Production Metrics (v0.2.0)

**WASM Binary:**
- Size: 106 KB (under 150 KB target ✅)
- Checksum: `6f182a640568669afa6294641aa074bb13a332b146516ae199505ff470d94b18`
- Validation: Zero WASI CLI imports ✅
- Host version: ^0.1.0 (critical requirement)

**Query Performance:**
- Cold start: ~400ms
- Warm queries: ~300ms
- API latency: ~200-300ms
- Parsing overhead: ~50-100ms

**Data Quality:**
- All 16 columns returning data (no NULLs)
- Nested JSON parsing working (timeframe fields)
- String parsing working (energyprice)
- Negative prices handled correctly
- 113 forecast hours returned
- Native TIMESTAMP WITH TIME ZONE (no conversion needed!)

## Known Limitations & Edge Cases

**Handled in v0.2.0:**
- ✅ energyprice string parsing (converts "-0.014000" → -0.014)
- ✅ Nested timeframe access (safe double .get() pattern)
- ✅ Missing postal_code parameter (clear error message)
- ✅ Negative energy prices (correctly handled, no abs() applied)
- ✅ Bounds checking (all Vec access uses safe .get())
- ✅ Milliseconds to microseconds conversion for PostgreSQL timestamps

**Not Yet Implemented:**
- ⚠️ import_foreign_schema() - Returns empty vec (manual table creation required)
- ⚠️ Signature field - Not exposed (cryptographic verification field)
- ⚠️ API metadata fields - support, info, documentation not exposed
- ⚠️ Rate limit handling - No retry logic for 429 errors
- ⚠️ Invalid postal code validation - May return empty results or API error

**API Constraints:**
- Geographic scope: Germany only (German postal codes)
- Forecast window: ~113 hours (variable, API-dependent)
- Rate limiting: 2,000 requests/day (authenticated tier)
- Historical data: Not available via this endpoint
- Parameter validation: API-side (invalid postal codes may error)

## Documentation

**User Documentation:**
- **README.md** - Comprehensive project overview (primary reference)
- **QUICKSTART.md** - 3-minute setup guide (minimal, links to README)
- **MIGRATION.md** - v0.1.0 → v0.2.0 upgrade guide (authoritative for changes)
- **docs/endpoints/gsi-prediction.md** - Complete endpoint reference (technical authority)

**For Users vs Developers:**
- Users: Start with [QUICKSTART.md](QUICKSTART.md), then [README.md](README.md)
- Developers: Read this file, then see [README.md](README.md#contributing) for contribution guidelines
- Migration: See [MIGRATION.md](MIGRATION.md) for v0.1.0 → v0.2.0 upgrade

## Version Coordination

**Important:** Keep versions synchronized across:
- `Cargo.toml` - version = "0.2.0"
- `wit/world.wit` - package powabase:supabase-fdw-corrently@0.2.0
- `CLAUDE.md` - Current Version section (this file)
- `README.md` - fdw_package_version '0.2.0'
- `MIGRATION.md` - FDW Version reference

All must match for successful builds and releases.

## Repository

- **GitHub:** https://github.com/powabase/supabase-fdw-corrently
- **Package:** powabase:supabase-fdw-corrently
- **License:** Apache-2.0
