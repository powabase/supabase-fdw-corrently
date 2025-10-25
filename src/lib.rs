// Corrently GrünstromIndex WASM Foreign Data Wrapper
//
// This FDW wrapper provides access to the Corrently GrünstromIndex API,
// enabling green energy forecasting queries directly from PostgreSQL.
//
// Version: 0.1.0
// API: Corrently v2.0 (https://api.corrently.io/v2.0)

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
    zip: String,
    hours: Option<i64>,

    // Cached forecast data (flattened from API response array)
    // Each Vec contains N elements (one per forecast hour, typically ~113)
    epochtime: Vec<i64>,
    timestamp: Vec<i64>,
    timeframe_start: Vec<i64>,
    timeframe_end: Vec<i64>,
    gsi: Vec<f64>,
    eevalue: Vec<i64>,
    ewind: Vec<i64>,
    esolar: Vec<i64>,
    enwind: Vec<i64>,
    ensolar: Vec<i64>,
    sci: Vec<i64>,
    energyprice: Vec<f64>,
    co2_avg: Vec<f64>,
    co2_g_standard: Vec<i64>,
    co2_g_oekostrom: Vec<i64>,
    zip_values: Vec<String>,
    iat: Vec<i64>,

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
        self.epochtime.len()
    }

    /// Clear all cached forecast data
    fn clear_data(&mut self) {
        self.epochtime.clear();
        self.timestamp.clear();
        self.timeframe_start.clear();
        self.timeframe_end.clear();
        self.gsi.clear();
        self.eevalue.clear();
        self.ewind.clear();
        self.esolar.clear();
        self.enwind.clear();
        self.ensolar.clear();
        self.sci.clear();
        self.energyprice.clear();
        self.co2_avg.clear();
        self.co2_g_standard.clear();
        self.co2_g_oekostrom.clear();
        self.zip_values.clear();
        self.iat.clear();
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
            // epochtime (seconds)
            let epochtime_val = forecast_obj
                .get("epochtime")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'epochtime' at index {}", idx))?;
            self.epochtime.push(epochtime_val);

            // timeStamp (milliseconds)
            let timestamp_val = forecast_obj
                .get("timeStamp")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'timeStamp' at index {}", idx))?;
            self.timestamp.push(timestamp_val);

            // timeframe.start (nested object)
            let timeframe_start_val = forecast_obj
                .get("timeframe")
                .and_then(|tf| tf.get("start"))
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'timeframe.start' at index {}", idx))?;
            self.timeframe_start.push(timeframe_start_val);

            // timeframe.end (nested object)
            let timeframe_end_val = forecast_obj
                .get("timeframe")
                .and_then(|tf| tf.get("end"))
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'timeframe.end' at index {}", idx))?;
            self.timeframe_end.push(timeframe_end_val);

            // gsi (GrünstromIndex value)
            let gsi_val = forecast_obj
                .get("gsi")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| format!("missing or invalid 'gsi' at index {}", idx))?;
            self.gsi.push(gsi_val);

            // eevalue (renewable energy percentage)
            let eevalue_val = forecast_obj
                .get("eevalue")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'eevalue' at index {}", idx))?;
            self.eevalue.push(eevalue_val);

            // ewind (wind energy percentage)
            let ewind_val = forecast_obj
                .get("ewind")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'ewind' at index {}", idx))?;
            self.ewind.push(ewind_val);

            // esolar (solar energy percentage)
            let esolar_val = forecast_obj
                .get("esolar")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'esolar' at index {}", idx))?;
            self.esolar.push(esolar_val);

            // enwind (net wind energy percentage)
            let enwind_val = forecast_obj
                .get("enwind")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'enwind' at index {}", idx))?;
            self.enwind.push(enwind_val);

            // ensolar (net solar energy percentage)
            let ensolar_val = forecast_obj
                .get("ensolar")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'ensolar' at index {}", idx))?;
            self.ensolar.push(ensolar_val);

            // sci (Smart City Index)
            let sci_val = forecast_obj
                .get("sci")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'sci' at index {}", idx))?;
            self.sci.push(sci_val);

            // energyprice (CRITICAL: This is a STRING in API, needs parsing!)
            let energyprice_str = forecast_obj
                .get("energyprice")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("missing or invalid 'energyprice' at index {}", idx))?;
            let energyprice_val: f64 = energyprice_str.parse().unwrap_or(0.0); // Default to 0.0 if parsing fails
            self.energyprice.push(energyprice_val);

            // co2_avg (average CO2 baseline)
            let co2_avg_val = forecast_obj
                .get("co2_avg")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| format!("missing or invalid 'co2_avg' at index {}", idx))?;
            self.co2_avg.push(co2_avg_val);

            // co2_g_standard (CO2 for standard mix)
            let co2_g_standard_val = forecast_obj
                .get("co2_g_standard")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'co2_g_standard' at index {}", idx))?;
            self.co2_g_standard.push(co2_g_standard_val);

            // co2_g_oekostrom (CO2 for green mix)
            let co2_g_oekostrom_val = forecast_obj
                .get("co2_g_oekostrom")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'co2_g_oekostrom' at index {}", idx))?;
            self.co2_g_oekostrom.push(co2_g_oekostrom_val);

            // zip (postal code)
            let zip_val = forecast_obj
                .get("zip")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("missing or invalid 'zip' at index {}", idx))?
                .to_string();
            self.zip_values.push(zip_val);

            // iat (issued-at timestamp)
            let iat_val = forecast_obj
                .get("iat")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| format!("missing or invalid 'iat' at index {}", idx))?;
            self.iat.push(iat_val);
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
        let cell = match tgt_col.name().as_str() {
            "epochtime" => self.epochtime.get(row_idx).map(|&v| Cell::I64(v)),
            "timestamp" => self.timestamp.get(row_idx).map(|&v| Cell::I64(v)),
            "timeframe_start" => self.timeframe_start.get(row_idx).map(|&v| Cell::I64(v)),
            "timeframe_end" => self.timeframe_end.get(row_idx).map(|&v| Cell::I64(v)),
            "gsi" => self.gsi.get(row_idx).map(|&v| Cell::Numeric(v)),
            "eevalue" => self.eevalue.get(row_idx).map(|&v| Cell::I64(v)),
            "ewind" => self.ewind.get(row_idx).map(|&v| Cell::I64(v)),
            "esolar" => self.esolar.get(row_idx).map(|&v| Cell::I64(v)),
            "enwind" => self.enwind.get(row_idx).map(|&v| Cell::I64(v)),
            "ensolar" => self.ensolar.get(row_idx).map(|&v| Cell::I64(v)),
            "sci" => self.sci.get(row_idx).map(|&v| Cell::I64(v)),
            "energyprice" => self.energyprice.get(row_idx).map(|&v| Cell::Numeric(v)),
            "co2_avg" => self.co2_avg.get(row_idx).map(|&v| Cell::Numeric(v)),
            "co2_g_standard" => self.co2_g_standard.get(row_idx).map(|&v| Cell::I64(v)),
            "co2_g_oekostrom" => self.co2_g_oekostrom.get(row_idx).map(|&v| Cell::I64(v)),
            "zip" => self
                .zip_values
                .get(row_idx)
                .map(|v| Cell::String(v.clone())),
            "iat" => self.iat.get(row_idx).map(|&v| Cell::I64(v)),
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

        // Extract zip (required)
        this.zip = Self::extract_qual_string(&quals, "zip")
            .ok_or("zip parameter is required in WHERE clause (e.g., WHERE zip = '69168')")?;

        // Extract hours (optional)
        this.hours = Self::extract_qual_i64(&quals, "hours");

        // Build API URL
        let mut url = format!(
            "{}/v2.0/gsi/prediction?zip={}&token={}",
            this.base_url, this.zip, this.api_key
        );

        if let Some(hours_val) = this.hours {
            url.push_str(&format!("&hours={}", hours_val));
        }

        utils::report_info(&format!(
            "Fetching Corrently forecast for ZIP: {}, hours: {:?}",
            this.zip, this.hours
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
