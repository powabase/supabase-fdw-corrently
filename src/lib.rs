// Corrently GrünstromIndex WASM Foreign Data Wrapper
//
// This FDW wrapper provides access to the Corrently GrünstromIndex API,
// enabling green energy forecasting queries directly from PostgreSQL.
//
// Version: 0.2.0
// API: Corrently v2.0 (https://api.corrently.io/v2.0)
//
// BREAKING CHANGES in v0.2.0:
// - All column names standardized to follow database schema design standards
// - Temporal fields now use TIMESTAMP WITH TIME ZONE (not BIGINT)
// - See MIGRATION.md for upgrade guide

#[allow(warnings)]
mod bindings;

use serde_json::Value as JsonValue;

use bindings::{
    exports::supabase::wrappers::routines::Guest,
    supabase::wrappers::{
        http, stats,
        types::{
            Cell, Column, Context, FdwError, FdwResult, ImportForeignSchemaStmt, OptionsType, Qual,
            Row, Value,
        },
        utils,
    },
};

static FDW_NAME: &str = "CorrentlyFdw";

/// Main FDW struct holding all state for Corrently API queries
#[derive(Debug, Default)]
struct CorrentlyFdw {
    // Server options (from CREATE SERVER)
    base_url: String,
    api_key: String,
    headers: Vec<(String, String)>,

    // Query parameters (from WHERE clause)
    postal_code: String,
    hours: Option<i64>,

    // Cached forecast data (flattened from API response array)
    // Each Vec contains N elements (one per forecast hour, typically ~113)
    forecast_start_time: Vec<i64>, // Milliseconds (converted to TIMESTAMP WITH TIME ZONE)
    forecast_period_start: Vec<i64>, // Milliseconds (converted to TIMESTAMP WITH TIME ZONE)
    forecast_period_end: Vec<i64>, // Milliseconds (converted to TIMESTAMP WITH TIME ZONE)
    green_energy_index: Vec<f64>,
    renewable_energy_pct: Vec<i64>,
    wind_energy_pct: Vec<i64>,
    solar_energy_pct: Vec<i64>,
    net_wind_energy_pct: Vec<i64>,
    net_solar_energy_pct: Vec<i64>,
    smart_city_index: Vec<i64>,
    energy_price_eur_kwh: Vec<f64>,
    co2_baseline_g_kwh: Vec<f64>,
    standard_mix_co2_g_kwh: Vec<i64>,
    green_mix_co2_g_kwh: Vec<i64>,
    postal_code_values: Vec<String>,
    forecast_created_at: Vec<i64>, // Milliseconds (converted to TIMESTAMP WITH TIME ZONE)

    // Iteration state
    current_row: usize,
}

// Static instance pattern (required for Supabase WASM FDW)
static mut INSTANCE: *mut CorrentlyFdw = std::ptr::null_mut::<CorrentlyFdw>();

impl CorrentlyFdw {
    /// Initialize the static FDW instance
    fn init_instance() {
        let instance = Self::default();
        unsafe {
            INSTANCE = Box::leak(Box::new(instance));
        }
    }

    /// Get mutable reference to the static instance
    fn this_mut() -> &'static mut Self {
        unsafe { &mut (*INSTANCE) }
    }

    /// Get total number of rows in cached data
    fn row_count(&self) -> usize {
        self.forecast_start_time.len()
    }

    /// Clear all cached forecast data
    fn clear_data(&mut self) {
        self.forecast_start_time.clear();
        self.forecast_period_start.clear();
        self.forecast_period_end.clear();
        self.green_energy_index.clear();
        self.renewable_energy_pct.clear();
        self.wind_energy_pct.clear();
        self.solar_energy_pct.clear();
        self.net_wind_energy_pct.clear();
        self.net_solar_energy_pct.clear();
        self.smart_city_index.clear();
        self.energy_price_eur_kwh.clear();
        self.co2_baseline_g_kwh.clear();
        self.standard_mix_co2_g_kwh.clear();
        self.green_mix_co2_g_kwh.clear();
        self.postal_code_values.clear();
        self.forecast_created_at.clear();
        self.current_row = 0;
    }

    /// Extract string value from quals (WHERE clause)
    fn extract_qual_string(quals: &[Qual], field: &str) -> Option<String> {
        quals
            .iter()
            .find(|q| q.field() == field && q.operator() == "=")
            .and_then(|q| match q.value() {
                Value::Cell(Cell::String(s)) => Some(s.clone()),
                _ => None,
            })
    }

    /// Extract i64 value from quals (WHERE clause)
    fn extract_qual_i64(quals: &[Qual], field: &str) -> Option<i64> {
        quals
            .iter()
            .find(|q| q.field() == field && q.operator() == "=")
            .and_then(|q| match q.value() {
                Value::Cell(Cell::I64(i)) => Some(i),
                Value::Cell(Cell::Numeric(n)) => Some(n as i64),
                _ => None,
            })
    }

    /// Parse the forecast array from API response
    /// Pattern: Energy Charts array flattening (113 forecast objects → 113 rows)
    fn parse_forecast_response(&mut self, body: &str) -> FdwResult {
        let resp_json: JsonValue =
            serde_json::from_str(body).map_err(|e| format!("JSON parse error: {}", e))?;

        // Extract forecast array
        let forecast_array = resp_json
            .get("forecast")
            .and_then(|f| f.as_array())
            .ok_or("missing or invalid 'forecast' array in response")?;

        utils::report_info(&format!(
            "Parsing {} forecast objects from Corrently API",
            forecast_array.len()
        ));

        // Iterate through forecast array and flatten to vectors
        // CRITICAL: Use .get() for all JSON access (never use [])
        for (idx, forecast_obj) in forecast_array.iter().enumerate() {
            // forecast_start_time (timeStamp in milliseconds)
            let forecast_start_time_val = forecast_obj
                .get("timeStamp")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'timeStamp' at index {}", idx))?;
            self.forecast_start_time.push(forecast_start_time_val);

            // forecast_period_start (timeframe.start in milliseconds)
            let forecast_period_start_val = forecast_obj
                .get("timeframe")
                .and_then(|tf| tf.get("start"))
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'timeframe.start' at index {}", idx))?;
            self.forecast_period_start.push(forecast_period_start_val);

            // forecast_period_end (timeframe.end in milliseconds)
            let forecast_period_end_val = forecast_obj
                .get("timeframe")
                .and_then(|tf| tf.get("end"))
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'timeframe.end' at index {}", idx))?;
            self.forecast_period_end.push(forecast_period_end_val);

            // green_energy_index (GrünstromIndex value 0-100)
            let green_energy_index_val = forecast_obj
                .get("gsi")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| format!("missing or invalid 'gsi' at index {}", idx))?;
            self.green_energy_index.push(green_energy_index_val);

            // renewable_energy_pct (total renewable energy percentage)
            let renewable_energy_pct_val = forecast_obj
                .get("eevalue")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'eevalue' at index {}", idx))?;
            self.renewable_energy_pct.push(renewable_energy_pct_val);

            // wind_energy_pct (wind energy percentage)
            let wind_energy_pct_val = forecast_obj
                .get("ewind")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'ewind' at index {}", idx))?;
            self.wind_energy_pct.push(wind_energy_pct_val);

            // solar_energy_pct (solar energy percentage)
            let solar_energy_pct_val = forecast_obj
                .get("esolar")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'esolar' at index {}", idx))?;
            self.solar_energy_pct.push(solar_energy_pct_val);

            // net_wind_energy_pct (net wind energy percentage)
            let net_wind_energy_pct_val = forecast_obj
                .get("enwind")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'enwind' at index {}", idx))?;
            self.net_wind_energy_pct.push(net_wind_energy_pct_val);

            // net_solar_energy_pct (net solar energy percentage)
            let net_solar_energy_pct_val = forecast_obj
                .get("ensolar")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'ensolar' at index {}", idx))?;
            self.net_solar_energy_pct.push(net_solar_energy_pct_val);

            // smart_city_index (Smart City Index 0-100)
            let smart_city_index_val = forecast_obj
                .get("sci")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'sci' at index {}", idx))?;
            self.smart_city_index.push(smart_city_index_val);

            // energy_price_eur_kwh (CRITICAL: This is a STRING in API, needs parsing!)
            let energy_price_str = forecast_obj
                .get("energyprice")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("missing or invalid 'energyprice' at index {}", idx))?;
            let energy_price_val: f64 = energy_price_str.parse().unwrap_or(0.0); // Default to 0.0 if parsing fails
            self.energy_price_eur_kwh.push(energy_price_val);

            // co2_baseline_g_kwh (average CO2 baseline)
            let co2_baseline_val = forecast_obj
                .get("co2_avg")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| format!("missing or invalid 'co2_avg' at index {}", idx))?;
            self.co2_baseline_g_kwh.push(co2_baseline_val);

            // standard_mix_co2_g_kwh (CO2 for standard energy mix)
            let standard_mix_co2_val = forecast_obj
                .get("co2_g_standard")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'co2_g_standard' at index {}", idx))?;
            self.standard_mix_co2_g_kwh.push(standard_mix_co2_val);

            // green_mix_co2_g_kwh (CO2 for green energy mix)
            let green_mix_co2_val = forecast_obj
                .get("co2_g_oekostrom")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'co2_g_oekostrom' at index {}", idx))?;
            self.green_mix_co2_g_kwh.push(green_mix_co2_val);

            // postal_code (German postal code, 5 digits)
            let postal_code_val = forecast_obj
                .get("zip")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("missing or invalid 'zip' at index {}", idx))?
                .to_string();
            self.postal_code_values.push(postal_code_val);

            // forecast_created_at (issued-at timestamp in milliseconds)
            let forecast_created_at_val = forecast_obj
                .get("iat")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'iat' at index {}", idx))?;
            self.forecast_created_at.push(forecast_created_at_val);
        }

        utils::report_info(&format!(
            "Successfully parsed {} forecast rows",
            self.row_count()
        ));

        Ok(())
    }

    /// Map column name to cell value for current row
    fn get_cell_value(&self, tgt_col: &Column) -> Result<Option<Cell>, FdwError> {
        let row_idx = self.current_row;

        // Bounds check
        if row_idx >= self.row_count() {
            return Err("row index out of bounds".to_owned());
        }

        // Map column name to stored data using safe .get() pattern
        // CRITICAL: Temporal fields convert milliseconds → microseconds for TIMESTAMP WITH TIME ZONE
        let cell = match tgt_col.name().as_str() {
            // Temporal fields (TIMESTAMP WITH TIME ZONE) - convert ms to microseconds
            "forecast_start_time" => self
                .forecast_start_time
                .get(row_idx)
                .map(|&ms| Cell::Timestamptz(ms * 1000)),
            "forecast_period_start" => self
                .forecast_period_start
                .get(row_idx)
                .map(|&ms| Cell::Timestamptz(ms * 1000)),
            "forecast_period_end" => self
                .forecast_period_end
                .get(row_idx)
                .map(|&ms| Cell::Timestamptz(ms * 1000)),
            "forecast_created_at" => self
                .forecast_created_at
                .get(row_idx)
                .map(|&ms| Cell::Timestamptz(ms * 1000)),

            // Energy metrics
            "green_energy_index" => self
                .green_energy_index
                .get(row_idx)
                .map(|&v| Cell::Numeric(v)),
            "renewable_energy_pct" => self
                .renewable_energy_pct
                .get(row_idx)
                .map(|&v| Cell::I64(v)),
            "wind_energy_pct" => self.wind_energy_pct.get(row_idx).map(|&v| Cell::I64(v)),
            "solar_energy_pct" => self.solar_energy_pct.get(row_idx).map(|&v| Cell::I64(v)),
            "net_wind_energy_pct" => self.net_wind_energy_pct.get(row_idx).map(|&v| Cell::I64(v)),
            "net_solar_energy_pct" => self
                .net_solar_energy_pct
                .get(row_idx)
                .map(|&v| Cell::I64(v)),
            "smart_city_index" => self.smart_city_index.get(row_idx).map(|&v| Cell::I64(v)),

            // Pricing and CO2 metrics
            "energy_price_eur_kwh" => self
                .energy_price_eur_kwh
                .get(row_idx)
                .map(|&v| Cell::Numeric(v)),
            "co2_baseline_g_kwh" => self
                .co2_baseline_g_kwh
                .get(row_idx)
                .map(|&v| Cell::Numeric(v)),
            "standard_mix_co2_g_kwh" => self
                .standard_mix_co2_g_kwh
                .get(row_idx)
                .map(|&v| Cell::I64(v)),
            "green_mix_co2_g_kwh" => self.green_mix_co2_g_kwh.get(row_idx).map(|&v| Cell::I64(v)),

            // Geographic dimension
            "postal_code" => self
                .postal_code_values
                .get(row_idx)
                .map(|v| Cell::String(v.clone())),

            _ => return Err(format!("unknown column '{}'", tgt_col.name())),
        };

        Ok(cell)
    }
}

impl Guest for CorrentlyFdw {
    fn host_version_requirement() -> String {
        // Requires Supabase Wrappers framework version 0.1.x
        "^0.1.0".to_string()
    }

    fn init(ctx: &Context) -> FdwResult {
        Self::init_instance();
        let this = Self::this_mut();

        // Extract server options
        let opts = ctx.get_options(&OptionsType::Server);

        // Extract API key (required)
        this.api_key = opts.require("api_key")?;

        // Extract base URL (optional, with default)
        this.base_url = opts.require_or("api_url", "https://api.corrently.io");

        // Set up HTTP headers
        this.headers.push((
            "user-agent".to_owned(),
            "Supabase Wrappers Corrently FDW".to_string(),
        ));
        this.headers
            .push(("accept".to_owned(), "application/json".to_string()));

        utils::report_info(&format!(
            "Corrently FDW initialized with base URL: {}",
            this.base_url
        ));
        stats::inc_stats(FDW_NAME, stats::Metric::CreateTimes, 1);

        Ok(())
    }

    fn begin_scan(ctx: &Context) -> FdwResult {
        let this = Self::this_mut();

        // Clear any previous data
        this.clear_data();

        // Extract WHERE clause parameters
        let quals = ctx.get_quals();

        // Extract postal_code (required)
        this.postal_code = Self::extract_qual_string(&quals, "postal_code").ok_or(
            "postal_code parameter is required in WHERE clause (e.g., WHERE postal_code = '69168')",
        )?;

        // Extract hours (optional)
        this.hours = Self::extract_qual_i64(&quals, "hours");

        // Build API URL
        let mut url = format!(
            "{}/v2.0/gsi/prediction?zip={}&token={}",
            this.base_url, this.postal_code, this.api_key
        );

        if let Some(hours_val) = this.hours {
            url.push_str(&format!("&hours={}", hours_val));
        }

        utils::report_info(&format!(
            "Fetching Corrently forecast for postal code: {}, hours: {:?}",
            this.postal_code, this.hours
        ));

        // Make HTTP request
        let req = http::Request {
            method: http::Method::Get,
            url,
            headers: this.headers.clone(),
            body: String::default(),
        };

        let resp = http::get(&req)?;

        // Check for HTTP errors
        http::error_for_status(&resp)
            .map_err(|err| format!("Corrently API error: {} - {}", err, resp.body))?;

        utils::report_info(&format!(
            "Corrently API response: {} bytes, status {}",
            resp.body.len(),
            resp.status_code
        ));

        // Parse JSON response
        this.parse_forecast_response(&resp.body)?;

        // Track stats
        stats::inc_stats(FDW_NAME, stats::Metric::BytesIn, resp.body.len() as i64);
        stats::inc_stats(FDW_NAME, stats::Metric::RowsIn, this.row_count() as i64);

        // Reset row iterator
        this.current_row = 0;

        Ok(())
    }

    fn iter_scan(ctx: &Context, row: &Row) -> Result<Option<u32>, FdwError> {
        let this = Self::this_mut();

        // Check if we've exhausted all rows
        if this.current_row >= this.row_count() {
            stats::inc_stats(FDW_NAME, stats::Metric::RowsOut, this.current_row as i64);
            return Ok(None);
        }

        // Populate row with values for current row
        for tgt_col in ctx.get_columns() {
            let cell = this.get_cell_value(&tgt_col)?;
            row.push(cell.as_ref());
        }

        // Move to next row
        this.current_row += 1;
        Ok(Some(0))
    }

    fn end_scan(_ctx: &Context) -> FdwResult {
        let this = Self::this_mut();
        this.clear_data();
        Ok(())
    }

    fn begin_modify(_ctx: &Context) -> FdwResult {
        Err("modify operations on foreign table are not supported".to_owned())
    }

    fn insert(_ctx: &Context, _row: &Row) -> FdwResult {
        Ok(())
    }

    fn update(_ctx: &Context, _rowid: Cell, _row: &Row) -> FdwResult {
        Ok(())
    }

    fn delete(_ctx: &Context, _rowid: Cell) -> FdwResult {
        Ok(())
    }

    fn end_modify(_ctx: &Context) -> FdwResult {
        Ok(())
    }

    fn re_scan(_ctx: &Context) -> FdwResult {
        let this = Self::this_mut();
        this.current_row = 0;
        Ok(())
    }

    fn import_foreign_schema(
        _ctx: &Context,
        _stmt: ImportForeignSchemaStmt,
    ) -> Result<Vec<String>, FdwError> {
        // Phase 5: Generate CREATE FOREIGN TABLE statements
        Ok(vec![])
    }
}

bindings::export!(CorrentlyFdw with_types_in bindings);
