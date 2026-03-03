use std::time::Instant;

use rocket::http::Status;
use rocket::serde::json::{json, Json, Value};
use rocket::State;

use aw_models::Query;

use crate::endpoints::{HttpErrorJson, ServerState};

#[post("/", data = "<query_req>", format = "application/json")]
pub fn query(query_req: Json<Query>, state: &State<ServerState>) -> Result<Value, HttpErrorJson> {
    let query_code = query_req.0.query.join("\n");
    let intervals = &query_req.0.timeperiods;

    // Debug: log incoming query (truncated) and timeperiods
    let query_preview: String = query_code.chars().take(500).collect();
    debug!(
        "Query received — {} timeperiod(s), query preview: {}{}",
        intervals.len(),
        query_preview,
        if query_code.len() > 500 { "…" } else { "" }
    );
    for (i, interval) in intervals.iter().enumerate() {
        debug!("  timeperiod[{}]: {}", i, interval);
    }

    let mut results = Vec::new();
    let datastore = endpoints_get_lock!(state.datastore);
    let start_time = Instant::now();
    for (i, interval) in intervals.iter().enumerate() {
        let result = match aw_query::query(&query_code, interval, &datastore) {
            Ok(data) => {
                debug!(
                    "  timeperiod[{}] succeeded — result type: {:?}, json size: ~{} bytes",
                    i,
                    std::mem::discriminant(&data),
                    serde_json::to_string(&data).map(|s| s.len()).unwrap_or(0)
                );
                data
            }
            Err(e) => {
                warn!("Query failed: {:?}", e);
                return Err(HttpErrorJson::new(
                    Status::InternalServerError,
                    e.to_string(),
                ));
            }
        };
        results.push(result);
    }
    let elapsed = start_time.elapsed();
    debug!(
        "Query completed — {} result(s) in {:.1}ms",
        results.len(),
        elapsed.as_secs_f64() * 1000.0
    );
    Ok(json!(results))
}
