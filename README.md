# Corrently GrünstromIndex WASM FDW

> 🚧 **Status:** In Development (Phase 2 Complete - Repository Setup)

WebAssembly Foreign Data Wrapper for querying the Corrently GrünstromIndex API from PostgreSQL/Supabase.

## Overview

This FDW wrapper enables direct SQL queries against the Corrently GrünstromIndex API, providing hourly green energy forecasts for German locations.

**Key Features:**
- 🟢 Hourly GrünstromIndex forecasts (~113 hours ahead)
- ⚡ CO2 emissions data (standard and green energy mix)
- 💰 Dynamic energy pricing (EUR/kWh)
- 🌱 Renewable energy breakdown (solar, wind)
- 🔐 JWT token authentication
- 📍 German postal code (PLZ) based queries

## Project Status

**v0.1.0** - Repository setup complete, ready for implementation.

- ✅ Phase 1: Requirements Analysis - COMPLETE
- ✅ Phase 2: Repository Setup - COMPLETE
- 🚧 Phase 3: Implementation - NEXT
- ⏳ Phase 4: Testing & Validation
- ⏳ Phase 5: Documentation
- ⏳ Phase 6: CI/CD & Release
- ⏳ Phase 7: Backend Integration

## Quick Start

**Prerequisites:**
- Rust 1.90.0+
- cargo-component 0.21.1
- wasm32-unknown-unknown target
- Corrently API token

**Build:**
```bash
# Development
cargo component build --target wasm32-unknown-unknown

# Production
cargo component build --release --target wasm32-unknown-unknown
```

## Planned Endpoint (v0.1.0)

### gsi_prediction

Hourly green energy forecasts with comprehensive metrics.

**Example Query:**
```sql
SELECT * FROM fdw_corrently.gsi_prediction
WHERE zip = '69168'
LIMIT 10;
```

**Returns:** ~113 hourly forecast objects with GSI values, CO2 emissions, energy pricing, and renewable energy breakdown.

## Development

See `CLAUDE.md` for detailed development guidance and `phase1-research/` for complete API specifications.

## License

Apache-2.0

## Links

- **API Documentation:** https://console.corrently.io/gsi.html
- **Supabase Wrappers:** https://fdw.dev
- **Development Guide:** See `/Users/cf/Documents/GitHub/powabase/powabase-backend/docs/fdw-wrappers/`
