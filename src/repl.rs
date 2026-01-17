use rustyline::error::ReadlineError;
use rustyline::completion::{Completer, Pair};
use rustyline::hint::Hinter;
use rustyline::highlight::Highlighter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper, Editor};
use rustyline::history::DefaultHistory;
use rustyline::Result as RustylineResult;
use colored::*;
use std::path::PathBuf;
use std::collections::HashSet;
use crate::executor::QueryExecutor;
use crate::error::CqlResult;

/// CQL Auto-Completer with schema awareness
#[derive(Clone)]
struct CqlCompleter {
    keywords: Vec<String>,
    keyspaces: HashSet<String>,
    tables: HashSet<String>,
    current_keyspace: Option<String>,
}

impl CqlCompleter {
    fn new() -> Self {
        let keywords = vec![
            // DML
            "SELECT", "INSERT", "UPDATE", "DELETE", "TRUNCATE",
            "FROM", "WHERE", "SET", "VALUES", "INTO",
            "ORDER BY", "GROUP BY", "LIMIT", "ALLOW FILTERING",
            // DDL
            "CREATE", "ALTER", "DROP", "USE",
            "KEYSPACE", "TABLE", "INDEX", "TYPE", "MATERIALIZED VIEW",
            "WITH", "AND", "PRIMARY KEY", "CLUSTERING ORDER",
            // Data types
            "TEXT", "INT", "BIGINT", "FLOAT", "DOUBLE", "BOOLEAN",
            "UUID", "TIMEUUID", "TIMESTAMP", "DATE", "TIME",
            "BLOB", "COUNTER", "DECIMAL", "VARINT",
            "LIST", "SET", "MAP", "TUPLE", "FROZEN",
            // Keywords
            "IF", "EXISTS", "NOT EXISTS", "AS", "IN",
            "DISTINCT", "COUNT", "TOKEN", "TTL", "WRITETIME",
            // Describe
            "DESCRIBE", "DESC", "KEYSPACES", "TABLES", "TYPES",
            // Other
            "BEGIN", "BATCH", "APPLY", "UNLOGGED",
            "CONSISTENCY", "GRANT", "REVOKE", "PERMISSIONS",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        Self {
            keywords,
            keyspaces: HashSet::new(),
            tables: HashSet::new(),
            current_keyspace: None,
        }
    }

    fn update_keyspaces(&mut self, keyspaces: Vec<String>) {
        self.keyspaces = keyspaces.into_iter().collect();
    }

    fn update_tables(&mut self, tables: Vec<String>) {
        self.tables = tables.into_iter().collect();
    }

    fn set_keyspace(&mut self, keyspace: Option<String>) {
        self.current_keyspace = keyspace;
    }

    fn get_completions(&self, line: &str, pos: usize) -> Vec<Pair> {
        let line_up_to_cursor = &line[..pos];
        let last_word = line_up_to_cursor
            .split_whitespace()
            .last()
            .unwrap_or("");
        
        if last_word.is_empty() {
            return vec![];
        }

        let last_word_upper = last_word.to_uppercase();
        let line_upper = line_up_to_cursor.to_uppercase();
        let mut completions = Vec::new();

        for keyword in &self.keywords {
            if keyword.starts_with(&last_word_upper) {
                completions.push(Pair {
                    display: keyword.clone(),
                    replacement: keyword.clone(),
                });
            }
        }

        if line_upper.contains("USE ") || line_upper.contains("KEYSPACE ") {
            for keyspace in &self.keyspaces {
                if keyspace.to_uppercase().starts_with(&last_word_upper) {
                    completions.push(Pair {
                        display: keyspace.clone(),
                        replacement: keyspace.clone(),
                    });
                }
            }
        }

        if line_upper.contains("FROM ") || line_upper.contains("INTO ") || line_upper.contains("TABLE ") {
            for table in &self.tables {
                if table.to_uppercase().starts_with(&last_word_upper) {
                    completions.push(Pair {
                        display: table.clone(),
                        replacement: table.clone(),
                    });
                }
            }
        }

        completions
    }
}

impl Completer for CqlCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> RustylineResult<(usize, Vec<Pair>)> {
        let completions = self.get_completions(line, pos);
        
        let start = line[..pos]
            .rfind(|c: char| c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);
        
        Ok((start, completions))
    }
}

impl Hinter for CqlCompleter {
    type Hint = String;

    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
        None
    }
}

impl Highlighter for CqlCompleter {}

impl Validator for CqlCompleter {}

impl Helper for CqlCompleter {}

pub struct Repl {
    executor: QueryExecutor,
    output_format: String,
    completer: CqlCompleter,
}

impl Repl {
    pub fn new(executor: QueryExecutor, output_format: String) -> Self {
        Self {
            executor,
            output_format,
            completer: CqlCompleter::new(),
        }
    }

    async fn refresh_schema(&mut self) -> CqlResult<()> {
        match self.executor.execute("SELECT keyspace_name FROM system_schema.keyspaces").await {
            Ok(result) => {
                if let Some(rows) = result.rows {
                    let keyspaces: Vec<String> = rows.iter()
                        .filter_map(|row| {
                            row.columns.first()
                                .and_then(|col| {
                                    if let Some(scylla::frame::response::result::CqlValue::Text(name)) = col {
                                        Some(name.clone())
                                    } else {
                                        None
                                    }
                                })
                        })
                        .collect();
                    self.completer.update_keyspaces(keyspaces);
                }
            }
            Err(_) => {} 
        }

        match self.executor.execute("SELECT keyspace_name, table_name FROM system_schema.tables").await {
            Ok(result) => {
                if let Some(rows) = result.rows {
                    let tables: Vec<String> = rows.iter()
                        .filter_map(|row| {
                            if row.columns.len() >= 2 {
                                if let Some(scylla::frame::response::result::CqlValue::Text(table)) = &row.columns[1] {
                                    Some(table.clone())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .collect();
                    self.completer.update_tables(tables);
                }
            }
            Err(_) => {} 
        }

        Ok(())
    }

    pub async fn run(&mut self) -> CqlResult<()> {
        println!("{}", "=== CQL Rust Client ===".bright_cyan().bold());
        println!("{}", "Type 'help' for available commands, 'quit' or 'exit' to exit.".bright_black());
        println!("{}", "Auto-completion enabled: Use TAB to complete CQL keywords, keyspaces, and tables.".bright_black());
        println!();

        let _ = self.refresh_schema().await;

        let mut rl = Editor::<CqlCompleter, DefaultHistory>::new()
            .map_err(|e| crate::error::CqlError::IoError(
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            ))?;
        
        rl.set_helper(Some(self.completer.clone()));

        let history_file = dirs::home_dir()
            .map(|mut p: PathBuf| {
                p.push(".cqlrs_history");
                p
            });

        if let Some(ref path) = history_file {
            let _ = rl.load_history(path);
        }

        let mut multi_line_query = String::new();

        loop {
            let prompt = if multi_line_query.is_empty() {
                format!("{} ", "cqlrs>".green().bold())
            } else {
                format!("{} ", "    ->".yellow())
            };

            let readline = rl.readline(&prompt);
            
            match readline {
                Ok(line) => {
                    let line = line.trim();
                    
                    let _ = rl.add_history_entry(line);

                    if multi_line_query.is_empty() {
                        match line.to_lowercase().as_str() {
                            "quit" | "exit" => {
                                println!("{}", "Goodbye!".bright_cyan());
                                break;
                            }
                            "help" => {
                                self.print_help();
                                continue;
                            }
                            "clear" => {
                                print!("\x1B[2J\x1B[1;1H");
                                continue;
                            }
                            "" => continue,
                            _ => {}
                        }
                    }

                    if line.starts_with("\\format ") {
                        let new_format = line[8..].trim();
                        self.output_format = new_format.to_string();
                        println!("Output format set to: {}", new_format.cyan());
                        continue;
                    }

                    if line == "\\refresh" {
                        println!("{}", "Refreshing schema...".cyan());
                        match self.refresh_schema().await {
                            Ok(_) => {
                                rl.set_helper(Some(self.completer.clone()));
                                println!("{}", "Schema refreshed successfully!".green());
                            }
                            Err(e) => {
                                eprintln!("{} {}", "Error refreshing schema:".red().bold(), e);
                            }
                        }
                        continue;
                    }

                    if line.starts_with("\\d") || line.to_lowercase().starts_with("describe ") {
                        self.handle_describe_command(line).await;
                        continue;
                    }

                    if !line.is_empty() {
                        if !multi_line_query.is_empty() {
                            multi_line_query.push(' ');
                        }
                        multi_line_query.push_str(line);
                    }

                    if multi_line_query.ends_with(';') {
                        match self.executor.execute_and_print(&multi_line_query, &self.output_format).await {
                            Ok(_) => {
                                let query_upper = multi_line_query.to_uppercase();
                                if query_upper.contains("CREATE ") || query_upper.contains("DROP ") || query_upper.contains("USE ") {
                                    let _ = self.refresh_schema().await;
                                    rl.set_helper(Some(self.completer.clone()));
                                }
                            }
                            Err(e) => {
                                eprintln!("{} {}", "Error:".red().bold(), e);
                            }
                        }
                        multi_line_query.clear();
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("{}", "^C".yellow());
                    multi_line_query.clear();
                }
                Err(ReadlineError::Eof) => {
                    println!("{}", "Goodbye!".bright_cyan());
                    break;
                }
                Err(err) => {
                    eprintln!("{} {:?}", "Error:".red().bold(), err);
                    break;
                }
            }
        }

        if let Some(ref path) = history_file {
            let _ = rl.save_history(path);
        }

        Ok(())
    }

    fn print_help(&self) {
        println!("{}", "=== Available Commands ===".bright_cyan().bold());
        println!("  {}  - Exit the REPL", "quit, exit".green());
        println!("  {}        - Show this help message", "help".green());
        println!("  {}       - Clear the screen", "clear".green());
        println!("  {}  - Change output format (table, json, csv)", "\\format <fmt>".green());
        println!("  {}   - List all keyspaces", "\\dk".green());
        println!("  {} - List tables in keyspace", "\\dt [keyspace]".green());
        println!("  {}   - Refresh schema cache", "\\refresh".green());
        println!();
        println!("{}", "=== Auto-Completion ===".bright_cyan().bold());
        println!("  Press {} to auto-complete:", "TAB".yellow().bold());
        println!("  - CQL keywords (SELECT, INSERT, CREATE, etc.)");
        println!("  - Keyspace names (after USE, CREATE KEYSPACE, etc.)");
        println!("  - Table names (after FROM, INTO, TABLE, etc.)");
        println!();
        println!("{}", "=== CQL Commands ===".bright_cyan().bold());
        println!("  Execute any CQL query ending with {}.", ";".yellow());
        println!("  Multi-line queries are supported.");
        println!();
        println!("{}", "Examples:".bright_black());
        println!("  SELECT * FROM system.local;");
        println!("  USE my_keyspace;");
        println!("  DESCRIBE KEYSPACES;");
        println!();
    }

    async fn handle_describe_command(&mut self, command: &str) {
        let query = if command == "\\dk" {
            "SELECT keyspace_name FROM system_schema.keyspaces;".to_string()
        } else if command.starts_with("\\dt") {
            let parts: Vec<&str> = command.split_whitespace().collect();
            if parts.len() > 1 {
                format!("SELECT table_name FROM system_schema.tables WHERE keyspace_name = '{}';", parts[1])
            } else {
                "SELECT keyspace_name, table_name FROM system_schema.tables;".to_string()
            }
        } else {
            command.to_string() + ";"
        };

        match self.executor.execute_and_print(&query, &self.output_format).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("{} {}", "Error:".red().bold(), e);
            }
        }
    }
}
