use scylla::transport::query_result::QueryResult;
use scylla::frame::response::result::CqlValue;
use prettytable::{Table, Row, Cell, format};
use colored::*;
use crate::error::{CqlError, CqlResult};
use serde_json::{json, Value as JsonValue};
use terminal_size::{Width, terminal_size};

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Table,
    Json,
    Csv,
}

/// Get terminal width or default to 120
fn get_terminal_width() -> usize {
    terminal_size()
        .map(|(Width(w), _)| w as usize)
        .unwrap_or(120)
}

/// Truncate string to fit max width with ellipsis
fn truncate_str(s: &str, max_width: usize) -> String {
    if s.len() <= max_width {
        s.to_string()
    } else if max_width <= 3 {
        s.chars().take(max_width).collect()
    } else {
        let mut result: String = s.chars().take(max_width - 3).collect();
        result.push_str("...");
        result
    }
}

pub fn format_result(result: &QueryResult, format: OutputFormat) -> CqlResult<String> {
    match format {
        OutputFormat::Table => format_as_table(result),
        OutputFormat::Json => format_as_json(result),
        OutputFormat::Csv => format_as_csv(result),
    }
}

fn format_as_table(result: &QueryResult) -> CqlResult<String> {
    let rows = match result.rows {
        Some(ref rows) => rows,
        None => {
            return Ok(format!("{}", "Query OK (no results)".green()));
        }
    };

    if rows.is_empty() {
        return Ok(format!("{}", "Empty result set".yellow()));
    }

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_BOX_CHARS);

    // Get column specifications
    let col_specs = &result.col_specs;
    let num_cols = col_specs.len();
    
    if num_cols == 0 {
        return Ok(format!("{}", "No columns in result".yellow()));
    }

    // Get terminal width and calculate available space
    let terminal_width = get_terminal_width();
    // Account for table borders and padding: 3 chars per column (| x |) + 1 for final |
    let border_overhead = (num_cols * 3) + 1;
    let available_width = terminal_width.saturating_sub(border_overhead).max(num_cols);
    
    // Calculate max width per column
    let max_col_width = available_width / num_cols;
    let min_col_width = 3; // Minimum width for "..."
    let col_width = max_col_width.max(min_col_width).min(50); // Cap at 50 chars per column

    // Prepare all data first to determine actual column widths needed
    let mut data_rows: Vec<Vec<String>> = Vec::new();
    let mut col_max_widths: Vec<usize> = vec![0; num_cols];
    
    // Check header widths
    for (i, spec) in col_specs.iter().enumerate() {
        col_max_widths[i] = spec.name.len().min(col_width);
    }
    
    // Process all rows and track max widths
    for row in rows {
        let row_data: Vec<String> = row.columns.iter()
            .map(|col| format_cql_value(col))
            .collect();
        
        for (i, cell) in row_data.iter().enumerate() {
            if i < col_max_widths.len() {
                col_max_widths[i] = col_max_widths[i].max(cell.len().min(col_width));
            }
        }
        
        data_rows.push(row_data);
    }
    
    // Adjust column widths if total exceeds available space
    let total_width: usize = col_max_widths.iter().sum();
    if total_width > available_width {
        // Proportionally reduce all columns
        let scale = available_width as f64 / total_width as f64;
        for width in &mut col_max_widths {
            *width = ((*width as f64 * scale) as usize).max(min_col_width);
        }
    }

    // Add header row with truncated names
    let header_cells: Vec<Cell> = col_specs.iter()
        .enumerate()
        .map(|(i, spec)| {
            let truncated = truncate_str(&spec.name, col_max_widths[i]);
            Cell::new(&truncated).style_spec("Fb")
        })
        .collect();
    table.add_row(Row::new(header_cells));

    // Add data rows with truncated content
    for row_data in data_rows {
        let cells: Vec<Cell> = row_data.iter()
            .enumerate()
            .map(|(i, cell)| {
                let truncated = truncate_str(cell, col_max_widths[i]);
                Cell::new(&truncated)
            })
            .collect();
        table.add_row(Row::new(cells));
    }

    let mut output = table.to_string();
    output.push_str(&format!("\n{} row(s) returned\n", rows.len().to_string().cyan()));
    
    Ok(output)
}

fn format_as_json(result: &QueryResult) -> CqlResult<String> {
    let rows = match result.rows {
        Some(ref rows) => rows,
        None => {
            return Ok(json!({"status": "ok", "rows": []}).to_string());
        }
    };

    let col_specs = &result.col_specs;

    let mut json_rows = Vec::new();
    
    for row in rows {
        let mut json_row = serde_json::Map::new();
        for (i, col) in row.columns.iter().enumerate() {
            let col_name = &col_specs[i].name;
            let value = cql_value_to_json(col);
            json_row.insert(col_name.clone(), value);
        }
        json_rows.push(JsonValue::Object(json_row));
    }

    let result_json = json!({
        "rows": json_rows,
        "count": rows.len()
    });

    Ok(serde_json::to_string_pretty(&result_json)
        .map_err(|e| CqlError::QueryError(format!("JSON serialization error: {}", e)))?)
}

fn format_as_csv(result: &QueryResult) -> CqlResult<String> {
    let rows = match result.rows {
        Some(ref rows) => rows,
        None => {
            return Ok(String::new());
        }
    };

    let col_specs = &result.col_specs;

    let mut output = String::new();

    // Header
    let headers: Vec<String> = col_specs.iter()
        .map(|spec| spec.name.clone())
        .collect();
    output.push_str(&headers.join(","));
    output.push('\n');

    // Data rows
    for row in rows {
        let values: Vec<String> = row.columns.iter()
            .map(|col| escape_csv_value(&format_cql_value(col)))
            .collect();
        output.push_str(&values.join(","));
        output.push('\n');
    }

    Ok(output)
}

fn format_cql_value(value: &Option<CqlValue>) -> String {
    match value {
        None => "NULL".to_string(),
        Some(cql_val) => match cql_val {
            CqlValue::Ascii(s) | CqlValue::Text(s) => s.clone(),
            CqlValue::Boolean(b) => b.to_string(),
            CqlValue::Int(i) => i.to_string(),
            CqlValue::BigInt(i) => i.to_string(),
            CqlValue::Float(f) => f.to_string(),
            CqlValue::Double(f) => f.to_string(),
            CqlValue::Uuid(u) => u.to_string(),
            CqlValue::Timeuuid(u) => u.to_string(),
            CqlValue::Timestamp(ts) => format!("{:?}", ts),
            CqlValue::List(list) => format!("[{}]", list.iter()
                .map(|v| format_cql_value(&Some(v.clone())))
                .collect::<Vec<_>>()
                .join(", ")),
            CqlValue::Set(set) => format!("{{{}}}", set.iter()
                .map(|v| format_cql_value(&Some(v.clone())))
                .collect::<Vec<_>>()
                .join(", ")),
            CqlValue::Map(map) => format!("{{{}}}", map.iter()
                .map(|(k, v)| format!("{}: {}", 
                    format_cql_value(&Some(k.clone())), 
                    format_cql_value(&Some(v.clone()))))
                .collect::<Vec<_>>()
                .join(", ")),
            _ => format!("{:?}", cql_val),
        }
    }
}

fn cql_value_to_json(value: &Option<CqlValue>) -> JsonValue {
    match value {
        None => JsonValue::Null,
        Some(cql_val) => match cql_val {
            CqlValue::Ascii(s) | CqlValue::Text(s) => JsonValue::String(s.clone()),
            CqlValue::Boolean(b) => JsonValue::Bool(*b),
            CqlValue::Int(i) => json!(*i),
            CqlValue::BigInt(i) => json!(*i),
            CqlValue::Float(f) => json!(*f),
            CqlValue::Double(f) => json!(*f),
            CqlValue::Uuid(u) => JsonValue::String(u.to_string()),
            CqlValue::Timeuuid(u) => JsonValue::String(u.to_string()),
            CqlValue::Timestamp(ts) => json!(format!("{:?}", ts)),
            CqlValue::List(list) => JsonValue::Array(
                list.iter()
                    .map(|v| cql_value_to_json(&Some(v.clone())))
                    .collect()
            ),
            CqlValue::Set(set) => JsonValue::Array(
                set.iter()
                    .map(|v| cql_value_to_json(&Some(v.clone())))
                    .collect()
            ),
            _ => JsonValue::String(format!("{:?}", cql_val)),
        }
    }
}

fn escape_csv_value(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}
