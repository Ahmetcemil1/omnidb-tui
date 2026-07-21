use anyhow::Result;
use sqlx::{any::AnyPoolOptions, AnyPool, Column, Row};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbType {
    Postgres,
    MySql,
    Sqlite,
    Solana,
    Redis,
    Mongo,
}

pub fn detect_db_type(uri: &str) -> DbType {
    if uri.starts_with("postgres") || uri.starts_with("postgresql") {
        DbType::Postgres
    } else if uri.starts_with("mysql") {
        DbType::MySql
    } else if uri.starts_with("solana") {
        DbType::Solana
    } else if uri.starts_with("redis") {
        DbType::Redis
    } else if uri.starts_with("mongo") || uri.starts_with("mongodb") {
        DbType::Mongo
    } else {
        DbType::Sqlite
    }
}

pub fn escape_identifier(name: &str, db_type: DbType) -> String {
    match db_type {
        DbType::MySql => format!("`{}`", name.replace('`', "``")),
        _ => format!("\"{}\"", name.replace('"', "\"\"")),
    }
}

pub async fn connect(uri: &str) -> Result<AnyPool> {
    // Install default SQLx drivers (required for the dynamic 'any' driver)
    sqlx::any::install_default_drivers();
    
    let pool = AnyPoolOptions::new()
        .max_connections(5)
        .connect(uri)
        .await?;
    Ok(pool)
}

pub async fn get_tables(pool: &AnyPool, db_type: DbType) -> Result<Vec<String>> {
    if db_type == DbType::Solana {
        return Ok(vec![]);
    }
    let query_str = match db_type {
        DbType::Postgres => {
            "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name;"
        }
        DbType::MySql => {
            "SELECT table_name FROM information_schema.tables WHERE table_schema = DATABASE() ORDER BY table_name;"
        }
        DbType::Sqlite => {
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name;"
        }
        DbType::Solana | DbType::Redis | DbType::Mongo => unreachable!(),
    };

    let rows = sqlx::query(query_str).fetch_all(pool).await?;
    let mut tables = Vec::new();
    for row in rows {
        let table_name: String = row.try_get(0)?;
        tables.push(table_name);
    }
    Ok(tables)
}

pub async fn get_schema(pool: &AnyPool, db_type: DbType) -> Result<String> {
    if db_type == DbType::Solana {
        return Ok("Account Info, Recent Transactions, Borsh IDL Parser".to_string());
    }
    let mut schema_desc = String::new();

    match db_type {
        DbType::Postgres | DbType::MySql => {
            let query_str = if db_type == DbType::Postgres {
                "SELECT table_name, column_name, data_type FROM information_schema.columns WHERE table_schema = 'public' ORDER BY table_name, ordinal_position;"
            } else {
                "SELECT table_name, column_name, data_type FROM information_schema.columns WHERE table_schema = DATABASE() ORDER BY table_name, ordinal_position;"
            };

            let rows = sqlx::query(query_str).fetch_all(pool).await?;
            let mut current_table = String::new();
            for row in rows {
                let table: String = row.try_get(0)?;
                let column: String = row.try_get(1)?;
                let data_type: String = row.try_get(2)?;

                if table != current_table {
                    if !current_table.is_empty() {
                        schema_desc.push_str("), ");
                    }
                    current_table = table.clone();
                    schema_desc.push_str(&format!("{}({}", table, column));
                } else {
                    schema_desc.push_str(&format!(", {}:{}", column, data_type));
                }
            }
            if !current_table.is_empty() {
                schema_desc.push_str(")");
            }
        }
        DbType::Sqlite => {
            let tables = get_tables(pool, DbType::Sqlite).await?;
            for (idx, table) in tables.iter().enumerate() {
                let info_query = format!("PRAGMA table_info({});", table);
                let rows = sqlx::query(&info_query).fetch_all(pool).await?;
                
                schema_desc.push_str(&format!("{}(", table));
                let cols: Vec<String> = rows.iter().map(|row| {
                    let name: String = row.try_get("name").unwrap_or_default();
                    let data_type: String = row.try_get("type").unwrap_or_default();
                    format!("{}:{}", name, data_type)
                }).collect();
                
                schema_desc.push_str(&cols.join(", "));
                schema_desc.push_str(")");
                if idx < tables.len() - 1 {
                    schema_desc.push_str(", ");
                }
            }
        }
        DbType::Solana | DbType::Redis | DbType::Mongo => unreachable!(),
    }

    Ok(schema_desc)
}

pub async fn execute_query(pool: &AnyPool, sql: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let sql_trimmed = sql.trim().to_lowercase();
    let is_query = sql_trimmed.starts_with("select")
        || sql_trimmed.starts_with("show")
        || sql_trimmed.starts_with("explain")
        || sql_trimmed.starts_with("pragma")
        || sql_trimmed.starts_with("describe")
        || sql_trimmed.starts_with("with");

    if !is_query {
        // Execute DML/DDL command
        let result = sqlx::query(sql).execute(pool).await?;
        let headers = vec!["Execution Status".to_string(), "Rows Affected".to_string()];
        let rows = vec![vec!["Query OK".to_string(), result.rows_affected().to_string()]];
        return Ok((headers, rows));
    }

    let rows = sqlx::query(sql).fetch_all(pool).await?;
    if rows.is_empty() {
        return Ok((vec![], vec![]));
    }

    // Extract headers
    let headers: Vec<String> = rows[0]
        .columns()
        .iter()
        .map(|c| c.name().to_string())
        .collect();

    // Extract rows and convert values to String dynamically
    let mut result_rows = Vec::new();
    for row in rows {
        let mut row_values = Vec::new();
        for i in 0..row.columns().len() {
            let val_str = match row.try_get::<String, _>(i) {
                Ok(s) => s,
                Err(_) => match row.try_get::<i64, _>(i) {
                    Ok(n) => n.to_string(),
                    Err(_) => match row.try_get::<f64, _>(i) {
                        Ok(f) => f.to_string(),
                        Err(_) => match row.try_get::<bool, _>(i) {
                            Ok(b) => b.to_string(),
                            Err(_) => match row.try_get::<i32, _>(i) {
                                Ok(n) => n.to_string(),
                                Err(_) => "NULL".to_string(),
                            }
                        }
                    }
                }
            };
            row_values.push(val_str);
        }
        result_rows.push(row_values);
    }

    Ok((headers, result_rows))
}

pub async fn execute_update(pool: &AnyPool, sql: &str) -> Result<u64> {
    let result = sqlx::query(sql).execute(pool).await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sqlite_integration() {
        // 1. Test database connection with max_connections(1) for in-memory SQLite schema sharing
        sqlx::any::install_default_drivers();
        let pool = AnyPoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        
        // 2. Test DDL / command execution
        execute_update(&pool, "CREATE TABLE test_users (id INTEGER PRIMARY KEY, name TEXT);").await.unwrap();
        
        // 3. Test insert command
        execute_update(&pool, "INSERT INTO test_users VALUES (1, 'Alice');").await.unwrap();
        
        // 4. Test table metadata retrieval
        let tables = get_tables(&pool, DbType::Sqlite).await.unwrap();
        assert_eq!(tables, vec!["test_users".to_string()]);
        
        // 5. Test schema extraction for AI context
        let schema = get_schema(&pool, DbType::Sqlite).await.unwrap();
        assert!(schema.contains("test_users(id:INTEGER, name:TEXT)"));

        // 6. Test select query execution & data formatting
        let (headers, rows) = execute_query(&pool, "SELECT * FROM test_users;").await.unwrap();
        assert_eq!(headers, vec!["id".to_string(), "name".to_string()]);
        assert_eq!(rows, vec![vec!["1".to_string(), "Alice".to_string()]]);
        
        // 7. Test identifier escaping and safe inline update
        let esc_table = escape_identifier("test_users", DbType::Sqlite);
        let esc_col = escape_identifier("name", DbType::Sqlite);
        let esc_pk = escape_identifier("id", DbType::Sqlite);
        assert_eq!(esc_table, "\"test_users\"");
        assert_eq!(esc_col, "\"name\"");
        assert_eq!(esc_pk, "\"id\"");

        let update_sql = format!(
            "UPDATE {} SET {} = 'Bob' WHERE {} = '1';",
            esc_table, esc_col, esc_pk
        );
        let rows_affected = execute_update(&pool, &update_sql).await.unwrap();
        assert_eq!(rows_affected, 1);
        
        // 8. Verify data updated successfully
        let (_, new_rows) = execute_query(&pool, "SELECT name FROM test_users;").await.unwrap();
        assert_eq!(new_rows, vec![vec!["Bob".to_string()]]);
    }
}
