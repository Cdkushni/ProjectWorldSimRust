use crate::Result;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use world_sim_event_bus::EventEnvelope;

/// Database connection manager
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database connection
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    /// Initialize database schema
    pub async fn initialize_schema(&self) -> Result<()> {
        // Create event history table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS event_history (
                id UUID PRIMARY KEY,
                timestamp TIMESTAMPTZ NOT NULL,
                event_type VARCHAR(255) NOT NULL,
                source VARCHAR(255) NOT NULL,
                payload JSONB NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create world snapshots table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS world_snapshots (
                id UUID PRIMARY KEY,
                timestamp TIMESTAMPTZ NOT NULL,
                name VARCHAR(255),
                data BYTEA NOT NULL,
                metadata JSONB
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indices
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_event_timestamp ON event_history(timestamp DESC)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_event_type ON event_history(event_type)",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Store an event in the history
    pub async fn store_event(&self, event: &EventEnvelope) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO event_history (id, timestamp, event_type, source, payload)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(event.id)
        .bind(event.timestamp)
        .bind(&event.event_type)
        .bind(&event.source)
        .bind(&event.payload)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Query events by type
    pub async fn query_events(
        &self,
        event_type: Option<&str>,
        limit: i64,
    ) -> Result<Vec<EventEnvelope>> {
        let query = if let Some(et) = event_type {
            sqlx::query(
                r#"
                SELECT id, timestamp, event_type, source, payload
                FROM event_history
                WHERE event_type = $1
                ORDER BY timestamp DESC
                LIMIT $2
                "#,
            )
            .bind(et)
            .bind(limit)
        } else {
            sqlx::query(
                r#"
                SELECT id, timestamp, event_type, source, payload
                FROM event_history
                ORDER BY timestamp DESC
                LIMIT $1
                "#,
            )
            .bind(limit)
        };

        let rows = query.fetch_all(&self.pool).await?;

        let events = rows
            .into_iter()
            .map(|row| EventEnvelope {
                id: row.get("id"),
                timestamp: row.get("timestamp"),
                event_type: row.get("event_type"),
                source: row.get("source"),
                payload: row.get("payload"),
            })
            .collect();

        Ok(events)
    }

    /// Save a world snapshot
    pub async fn save_snapshot(&self, name: &str, data: Vec<u8>) -> Result<uuid::Uuid> {
        let id = uuid::Uuid::new_v4();
        let timestamp = chrono::Utc::now();

        sqlx::query(
            r#"
            INSERT INTO world_snapshots (id, timestamp, name, data, metadata)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(id)
        .bind(timestamp)
        .bind(name)
        .bind(data)
        .bind(serde_json::json!({}))
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// Load a world snapshot by ID
    pub async fn load_snapshot(&self, id: uuid::Uuid) -> Result<Vec<u8>> {
        let row = sqlx::query(
            r#"
            SELECT data FROM world_snapshots WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(row.get("data")),
            None => Err(crate::PersistenceError::NotFound(format!(
                "Snapshot {}",
                id
            ))),
        }
    }

    /// List all snapshots
    pub async fn list_snapshots(&self) -> Result<Vec<(uuid::Uuid, String, chrono::DateTime<chrono::Utc>)>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, timestamp
            FROM world_snapshots
            ORDER BY timestamp DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let snapshots = rows
            .into_iter()
            .map(|row| {
                (
                    row.get("id"),
                    row.get::<String, _>("name"),
                    row.get("timestamp"),
                )
            })
            .collect();

        Ok(snapshots)
    }
}

