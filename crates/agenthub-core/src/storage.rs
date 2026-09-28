use crate::models::{Plan, Target, Transaction};
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use std::{path::Path, sync::Mutex};

pub struct Store {
    connection: Mutex<Connection>,
}

pub type SecretRecord = (String, Vec<u8>, Vec<u8>);

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let connection = Connection::open(path)?;
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;
          CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS targets (target TEXT PRIMARY KEY, enabled INTEGER NOT NULL DEFAULT 0, last_sync TEXT);
          CREATE TABLE IF NOT EXISTS plans (id TEXT PRIMARY KEY, target TEXT NOT NULL, payload TEXT NOT NULL, created_at TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS transactions (id TEXT PRIMARY KEY, plan_id TEXT NOT NULL, target TEXT NOT NULL, status TEXT NOT NULL, payload TEXT NOT NULL, created_at TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS secrets (name TEXT PRIMARY KEY, algorithm TEXT NOT NULL, nonce BLOB NOT NULL, ciphertext BLOB NOT NULL, updated_at TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS scans (id TEXT PRIMARY KEY, payload TEXT NOT NULL, created_at TEXT NOT NULL);
          INSERT OR IGNORE INTO meta(key,value) VALUES ('schema_version','2'),('initialized','false');
          INSERT OR IGNORE INTO targets(target,enabled,last_sync)
            SELECT target,1,MAX(created_at) FROM transactions WHERE status='applied' GROUP BY target;
          UPDATE meta SET value='2' WHERE key='schema_version' AND CAST(value AS INTEGER) < 2;")?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.connection.lock().expect("database lock poisoned")
    }
    pub fn initialized(&self) -> Result<bool> {
        Ok(self.meta("initialized")?.as_deref() == Some("true"))
    }
    pub fn set_initialized(&self, value: bool) -> Result<()> {
        self.set_meta("initialized", if value { "true" } else { "false" })
    }
    pub fn meta(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn()
            .query_row("SELECT value FROM meta WHERE key=?1", [key], |row| {
                row.get(0)
            })
            .optional()?)
    }
    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn().execute("INSERT INTO meta(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key,value])?;
        Ok(())
    }
    pub fn set_target(&self, target: Target, enabled: bool) -> Result<()> {
        self.conn().execute("INSERT INTO targets(target,enabled) VALUES(?1,?2) ON CONFLICT(target) DO UPDATE SET enabled=excluded.enabled", params![target.as_str(), enabled])?;
        Ok(())
    }
    pub fn enabled_targets(&self) -> Result<Vec<Target>> {
        let conn = self.conn();
        let mut q = conn.prepare("SELECT target FROM targets WHERE enabled=1 ORDER BY target")?;
        let rows = q.query_map([], |r| r.get::<_, String>(0))?;
        let result = rows
            .filter_map(|v| v.ok().and_then(|s| s.parse().ok()))
            .collect();
        Ok(result)
    }
    pub fn save_plan(&self, plan: &Plan) -> Result<()> {
        self.conn().execute(
            "INSERT OR REPLACE INTO plans(id,target,payload,created_at) VALUES(?1,?2,?3,?4)",
            params![
                plan.id,
                plan.target.as_str(),
                serde_json::to_string(plan)?,
                plan.created_at
            ],
        )?;
        Ok(())
    }
    pub fn plan(&self, id: &str) -> Result<Option<Plan>> {
        let payload: Option<String> = self
            .conn()
            .query_row("SELECT payload FROM plans WHERE id=?1", [id], |r| r.get(0))
            .optional()?;
        payload
            .map(|p| serde_json::from_str(&p).map_err(Into::into))
            .transpose()
    }
    pub fn save_transaction(&self, tx: &Transaction) -> Result<()> {
        self.conn().execute("INSERT OR REPLACE INTO transactions(id,plan_id,target,status,payload,created_at) VALUES(?1,?2,?3,?4,?5,?6)",params![tx.id,tx.plan_id,tx.target.as_str(),tx.status,serde_json::to_string(tx)?,tx.created_at])?;
        Ok(())
    }
    pub fn transaction(&self, id: &str) -> Result<Option<Transaction>> {
        let payload: Option<String> = self
            .conn()
            .query_row(
                "SELECT payload FROM transactions WHERE id=?1",
                [id],
                |row| row.get(0),
            )
            .optional()?;
        payload
            .map(|payload| serde_json::from_str(&payload).map_err(Into::into))
            .transpose()
    }
    pub fn save_applied_transaction(&self, tx: &Transaction) -> Result<()> {
        anyhow::ensure!(tx.status == "applied", "transaction must be applied");
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        transaction.execute("INSERT OR REPLACE INTO transactions(id,plan_id,target,status,payload,created_at) VALUES(?1,?2,?3,?4,?5,?6)",params![tx.id,tx.plan_id,tx.target.as_str(),tx.status,serde_json::to_string(tx)?,tx.created_at])?;
        transaction.execute("INSERT INTO targets(target,enabled,last_sync) VALUES(?1,1,?2) ON CONFLICT(target) DO UPDATE SET enabled=1,last_sync=excluded.last_sync", params![tx.target.as_str(), tx.created_at])?;
        transaction.commit()?;
        Ok(())
    }
    pub fn recent_transactions(&self, limit: usize) -> Result<Vec<Transaction>> {
        let conn = self.conn();
        let mut q =
            conn.prepare("SELECT payload FROM transactions ORDER BY created_at DESC LIMIT ?1")?;
        let rows = q.query_map([limit as i64], |r| r.get::<_, String>(0))?;
        let result = rows
            .filter_map(|v| v.ok().and_then(|p| serde_json::from_str(&p).ok()))
            .collect();
        Ok(result)
    }
    pub fn put_secret(
        &self,
        name: &str,
        algorithm: &str,
        nonce: &[u8],
        ciphertext: &[u8],
    ) -> Result<()> {
        self.conn().execute("INSERT INTO secrets(name,algorithm,nonce,ciphertext,updated_at) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(name) DO UPDATE SET algorithm=excluded.algorithm,nonce=excluded.nonce,ciphertext=excluded.ciphertext,updated_at=excluded.updated_at",params![name,algorithm,nonce,ciphertext,chrono::Utc::now().to_rfc3339()])?;
        Ok(())
    }
    pub fn secret(&self, name: &str) -> Result<Option<SecretRecord>> {
        Ok(self
            .conn()
            .query_row(
                "SELECT algorithm,nonce,ciphertext FROM secrets WHERE name=?1",
                [name],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?)
    }
}
