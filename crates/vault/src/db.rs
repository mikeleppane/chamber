use crate::{KdfParams, WrappedVaultKey};
use color_eyre::Result;
use color_eyre::eyre::eyre;
use rusqlite::{Connection, OptionalExtension, params};
use time::OffsetDateTime;

#[derive(Debug)]
pub struct Db {
    conn: Connection,
}

impl Db {
    /// Opens a database connection and performs necessary migrations.
    ///
    /// # Arguments
    /// * `path` - A reference to the path of the database file to open. This specifies the location of the `SQLite` database.
    ///
    /// # Returns
    /// * `Result<Self>` - Returns an instance of the struct if the connection is successfully established and migrations are applied.
    ///   If an error occurs during either the connection or migration process, it returns an error.
    ///
    /// # Behavior
    /// 1. Attempts to open a database connection using the given file path.
    /// 2. Initializes the caller struct with the established database connection.
    /// 3. Triggers any necessary database migrations to ensure the schema is up-to-date.
    /// 4. Returns the initialized struct if all operations are successful.
    ///
    /// # Errors
    /// This function will return an error if:
    /// * The database connection cannot be established (e.g., invalid path, file issues).
    /// * The migration process fails.
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            r"
        PRAGMA journal_mode=WAL;
        PRAGMA foreign_keys=ON;

        CREATE TABLE IF NOT EXISTS meta (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            kdf_params TEXT NOT NULL,
            wrapped_key BLOB NOT NULL,
            verifier BLOB NOT NULL
        );

        CREATE TABLE IF NOT EXISTS items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            kind TEXT NOT NULL,
            nonce BLOB NOT NULL,
            ciphertext BLOB NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_items_name ON items(name);
        ",
        )?;
        Ok(())
    }

    /// Checks if the `meta` table in the database is empty.
    ///
    /// This function executes a SQL query to count the number of rows
    /// in the `meta` table. If the count is zero, it indicates that
    /// the table is empty, and the function returns `Ok(true)`. Otherwise,
    /// it returns `Ok(false)`.
    ///
    /// # Returns
    /// - `Ok(true)` if the `meta` table is empty.
    /// - `Ok(false)` if the `meta` table has one or more rows.
    /// - `Err` if an error occurs while querying the database.
    ///
    /// # Errors
    /// This function returns an error if there is an issue executing the SQL query,
    /// such as a database connection problem or an issue with the `meta` table.
    pub fn is_meta_empty(&self) -> Result<bool> {
        let count: i64 = self.conn.query_row("SELECT COUNT(*) FROM meta", [], |r| r.get(0))?;
        Ok(count == 0)
    }

    /// Writes metadata to the database for a secure vault system.
    ///
    /// This function performs the following operations:
    /// 1. Serializes the given KDF (Key Derivation Function) parameters to a JSON string.
    /// 2. Serializes the provided wrapped vault key to a byte vector (JSON format).
    /// 3. Deletes any existing data from the `meta` table in the database.
    /// 4. Inserts a new record into the `meta` table with the following fields:
    ///    - `id`: The primary key, hardcoded as `1`.
    ///    - `kdf_params`: The serialized KDF parameters.
    ///    - `wrapped_key`: The serialized wrapped vault key.
    ///    - `verifier`: A cryptographic verifier (byte slice).
    ///
    /// # Arguments
    /// * `kdf` - A reference to the `KdfParams` struct containing key derivation function parameters.
    /// * `wrapped` - A reference to a `WrappedVaultKey` struct representing the securely wrapped vault key.
    /// * `verifier` - A slice of bytes used as a cryptographic verifier to ensure integrity or authenticity.
    ///
    /// # Returns
    /// * `Result<()>` - Returns `Ok(())` if the metadata is successfully written to the database,
    ///   or an error (via `Result::Err`) if any operation fails (e.g., serialization or database access).
    ///
    /// # Errors
    /// Returns an error if:
    /// * Serialization of `kdf` or `wrapped` fails.
    /// * Any SQL operation (DELETE or INSERT) fails.
    ///
    /// # Notes
    /// * This function assumes that the `meta` table already exists and has the following schema:
    ///   - `meta (id INTEGER PRIMARY KEY, kdf_params TEXT, wrapped_key BLOB, verifier BLOB)`.
    /// * The `id` is fixed as `1` to allow for a single metadata record, effectively overriding any
    ///   existing metadata in the `meta` table.
    pub fn write_meta(&self, kdf: &KdfParams, wrapped: &WrappedVaultKey, verifier: &[u8]) -> Result<()> {
        let kdf_json = serde_json::to_string(kdf)?;
        let wrapped_json = serde_json::to_vec(wrapped)?;
        self.conn.execute("DELETE FROM meta", [])?;
        self.conn.execute(
            "INSERT INTO meta (id, kdf_params, wrapped_key, verifier) VALUES (1, ?, ?, ?)",
            params![kdf_json, wrapped_json, verifier],
        )?;
        Ok(())
    }

    /// Reads metadata from the database.
    ///
    /// This method retrieves metadata from the `meta` table in the database with the `id` of 1.
    /// Specifically, it fetches the Key Derivation Function (KDF) parameters, the wrapped key, and the verifier.
    /// It performs the following operations:
    /// 1. Executes a SQL query to fetch the `kdf_params`, `wrapped_key`, and `verifier` columns.
    /// 2. Maps the query result to raw JSON and binary blobs using a row-mapping closure.
    /// 3. Parses the JSON and binary data into strongly-typed Rust structures outside the closure.
    ///
    /// # Returns
    /// - An `Ok(Some((KdfParams, WrappedVaultKey, Vec<u8>)))` tuple if the record is found and successfully parsed:
    ///   - `KdfParams`: Deserialized KDF parameters from JSON.
    ///   - `WrappedVaultKey`: The deserialized wrapped key.
    ///   - `Vec<u8>`: A verifier used for integrity checks.
    /// - `Ok(None)` if no metadata record exists with `id = 1`.
    /// - An `Err` value if any error occurs during the operation, such as:
    ///   - Query errors from the `SQLite` database.
    ///   - JSON parsing errors for `KdfParams` and `WrappedVaultKey`.
    ///
    /// # Errors
    /// This method propagates errors in the following cases:
    /// - `SQLite` errors via `rusqlite::Error`, especially when querying the database.
    /// - Deserialization errors when converting `kdf_params` or `wrapped_key` into their respective types using `serde_json`.
    /// - Any other runtime errors are wrapped in `anyhow::Error`.
    ///
    /// # Dependencies
    /// - `rusqlite` for database interaction.
    /// - `serde_json` for JSON deserialization.
    /// - `anyhow` for error handling.
    pub fn read_meta(&self) -> Result<Option<(KdfParams, WrappedVaultKey, Vec<u8>)>> {
        // Fetch raw columns without JSON parsing inside the row-mapper
        let row = self
            .conn
            .query_row(
                "SELECT kdf_params, wrapped_key, verifier FROM meta WHERE id = 1",
                [],
                |r| {
                    let kdf_json: String = r.get(0)?;
                    let wrapped_blob: Vec<u8> = r.get(1)?;
                    let verifier: Vec<u8> = r.get(2)?;
                    Ok((kdf_json, wrapped_blob, verifier))
                },
            )
            .optional()?; // This is rusqlite::OptionalExtension; ok here

        // Parse JSON outside the closure so `?` produces anyhow::Error, not rusqlite::Error
        if let Some((kdf_json, wrapped_blob, verifier)) = row {
            let kdf: KdfParams = serde_json::from_str(&kdf_json)?;
            let wrapped: WrappedVaultKey = serde_json::from_slice(&wrapped_blob)?;
            Ok(Some((kdf, wrapped, verifier)))
        } else {
            Ok(None)
        }
    }

    /// Inserts a new item into the database with the provided attributes.
    ///
    /// # Parameters
    /// - `name`: The name of the item (unique).
    /// - `kind`: The type or category of the item.
    /// - `nonce`: A nonce (number used once) associated with the item, typically for cryptographic purposes.
    /// - `ciphertext`: The encrypted data associated with the item.
    ///
    /// # Returns
    /// - `Ok(())` if the item was successfully inserted into the database.
    /// - `Err`: If an error occurred during the operation. Specifically:
    ///   - If the `name` already exists in the database, a friendly message is returned indicating
    ///     a uniqueness violation.
    ///   - Any other underlying database or system errors are propagated.
    ///
    /// # Notes
    /// - The `created_at` and `updated_at` timestamps are automatically generated using the current UTC time
    ///   in RFC 3339 format.
    /// - The function uses `SQLite`'s `INSERT` statement, and if a constraint violation occurs (such as a duplicate `name`),
    ///   it maps the error to a more user-friendly `anyhow` error.
    ///
    /// # Errors
    /// - Returns an error if:
    ///   - The current timestamp could not be formatted as RFC 3339.
    ///   - There is a database insertion failure for any reason (e.g., constraint violation, I/O error).
    pub fn insert_item(&self, name: &str, kind: &str, nonce: &[u8], ciphertext: &[u8]) -> Result<()> {
        let now = OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?;
        match self.conn.execute(
            "INSERT INTO items (name, kind, nonce, ciphertext, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
            params![name, kind, nonce, ciphertext, now, now],
        ) {
            Ok(_) => Ok(()),
            Err(e) => {
                // Map uniqueness violation to a friendly error message
                if let rusqlite::Error::SqliteFailure(ref err, _) = e {
                    // Error code 2067 = SQLITE_CONSTRAINT_UNIQUE
                    if err.extended_code == 2067 || err.code == rusqlite::ErrorCode::ConstraintViolation {
                        return Err(eyre!("An item named '{}' already exists", name));
                    }
                }
                Err(e.into())
            }
        }
    }

    /// Retrieves a list of items from the database, ordered by name in ascending order.
    ///
    /// This method queries the database for all rows in the `items` table and maps
    /// each row into an `ItemRow` struct. The query retrieves the following fields:
    /// `id`, `name`, `kind`, `nonce`, `ciphertext`, `created_at`, and `updated_at`.
    /// The `created_at` and `updated_at` fields are parsed from RFC 3339 formatted
    /// strings into `OffsetDateTime` instances.
    ///
    /// # Returns
    ///
    /// A `Result` containing either:
    /// - A `Vec<ItemRow>` representing all the items in the table, or
    /// - An error if the query or data conversion fails.
    ///
    /// # Errors
    ///
    /// This method will return an error in the following cases:
    /// - If preparing the SQL statement fails.
    /// - If executing the query or mapping rows fails.
    /// - If the `created_at` or `updated_at` fields fail to parse from an RFC 3339 formatted string.
    ///
    /// # Dependencies
    ///
    /// This function assumes the following:
    /// - The `time` crate is used for date/time parsing.
    /// - The `OffsetDateTime::parse` method is employed with the RFC 3339 format description.
    /// - The `rusqlite` crate is used for `SQLite` database operations.
    ///
    /// # Note
    ///
    /// The method iterates over the rows returned by the query and pushes them into
    /// a vector before returning it. Ensure that the `items` table structure in the database
    /// matches the fields being queried (`id`, `name`, `kind`, `nonce`, `ciphertext`, `created_at`, `updated_at`).
    pub fn list_items(&self) -> Result<Vec<ItemRow>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, kind, nonce, ciphertext, created_at, updated_at FROM items ORDER BY name ASC")?;
        let rows = stmt.query_map([], |r| {
            Ok(ItemRow {
                id: r.get(0)?,
                name: r.get(1)?,
                kind: r.get(2)?,
                nonce: r.get(3)?,
                ciphertext: r.get(4)?,
                created_at: OffsetDateTime::parse(
                    &r.get::<_, String>(5)?,
                    &time::format_description::well_known::Rfc3339,
                )
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?, // Add comma and error conversion
                updated_at: OffsetDateTime::parse(
                    &r.get::<_, String>(6)?,
                    &time::format_description::well_known::Rfc3339,
                )
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Deletes an item from the database with the specified `id`.
    ///
    /// # Arguments
    ///
    /// * `id` - A 64-bit integer representing the unique identifier of the item to be deleted.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the operation is successful and the item is deleted.
    /// * `Err` - If there is an error during the database operation.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// * The connection to the database fails.
    /// * The SQL execution fails.
    pub fn delete_item(&self, id: u64) -> Result<()> {
        self.conn.execute("DELETE FROM items WHERE id = ?", params![id])?;
        Ok(())
    }

    /// Updates an item in the database with new encrypted data.
    ///
    /// # Parameters
    /// - `id`: The unique identifier of the item to be updated.
    /// - `nonce`: A reference to the byte slice representing the nonce used in encryption.
    /// - `ciphertext`: A reference to the byte slice containing the encrypted data.
    ///
    /// # Returns
    /// - `Result<()>`: Returns an `Ok(())` on successful update or an error if the operation fails.
    ///
    /// # Details
    /// This method constructs the current timestamp in RFC 3339 format and updates the specified
    /// item's `nonce`, `ciphertext`, and `updated_at` values in the `items` table. It identifies
    /// the target item using the provided `id`. The database interaction is performed using a
    /// connection (`self.conn`) and an SQL `UPDATE` query.
    ///
    /// # Errors
    /// - Returns an error if the timestamp formatting fails.
    /// - Returns an error if the SQL execution fails.
    pub fn update_item(&self, id: u64, nonce: &[u8], ciphertext: &[u8]) -> Result<()> {
        let now = OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?;
        self.conn.execute(
            "UPDATE items SET nonce = ?, ciphertext = ?, updated_at = ? WHERE id = ?",
            params![nonce, ciphertext, now, id],
        )?;
        Ok(())
    }
}

pub struct ItemRow {
    pub id: u64,
    pub name: String,
    pub kind: String,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl ItemRow {
    #[must_use]
    pub fn ad(&self) -> Vec<u8> {
        Self::ad_for_name_kind(&self.name, &self.kind)
    }
    #[must_use]
    pub fn ad_for_name_kind(name: &str, kind: &str) -> Vec<u8> {
        // Associated data binds the ciphertext to immutable fields.
        let mut v = Vec::with_capacity(name.len() + kind.len() + 2);
        v.extend_from_slice(name.as_bytes());
        v.push(0x1f);
        v.extend_from_slice(kind.as_bytes());
        v
    }
}

// Rust
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::unwrap_in_result)]
    #![allow(clippy::panic)]
    #![allow(clippy::panic_in_result_fn)]
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::{KdfParams, WrappedVaultKey};
    use std::{fs, thread, time::Duration};

    fn tmp_path(name: &str) -> std::path::PathBuf {
        let now = time::OffsetDateTime::now_utc().unix_timestamp_nanos();
        let pid = std::process::id();
        std::env::temp_dir().join(format!("chamber_db_{name}_{pid}_{now}.sqlite3"))
    }

    fn mk_wrapped(bytes: u8) -> WrappedVaultKey {
        WrappedVaultKey {
            nonce: vec![bytes; 24],
            ciphertext: vec![bytes ^ 0xAA; 32],
        }
    }

    fn small_kdf() -> KdfParams {
        KdfParams {
            salt: vec![1, 2, 3, 4, 5, 6, 7, 8],
            m_cost_kib: 8,
            t_cost: 1,
            p_cost: 1,
        }
    }

    #[test]
    fn test_open_and_migrate_and_is_meta_empty() -> Result<()> {
        let path = tmp_path("open_migrate");
        let db = Db::open(&path)?;
        assert!(db.is_meta_empty()?);

        // Reopen should succeed (idempotent migration)
        let db2 = Db::open(&path)?;
        assert!(db2.is_meta_empty()?);

        // Cleanup
        fs::remove_file(path).ok();
        Ok(())
    }

    #[test]
    fn test_read_meta_none_when_empty() -> Result<()> {
        let path = tmp_path("meta_none");
        let db = Db::open(&path)?;
        assert!(db.is_meta_empty()?);

        let meta = db.read_meta()?;
        assert!(meta.is_none());

        fs::remove_file(path).ok();
        Ok(())
    }

    #[test]
    fn test_write_and_read_meta_roundtrip() -> Result<()> {
        let path = tmp_path("meta_roundtrip");
        let db = Db::open(&path)?;

        let kdf = small_kdf();
        let wrapped = mk_wrapped(0x11);
        let verifier = vec![0xDE, 0xAD, 0xBE, 0xEF];

        db.write_meta(&kdf, &wrapped, &verifier)?;
        assert!(!db.is_meta_empty()?);

        let (kdf2, wrapped2, verifier2) = db.read_meta()?.expect("meta present");
        assert_eq!(kdf2.salt, kdf.salt);
        assert_eq!(kdf2.m_cost_kib, kdf.m_cost_kib);
        assert_eq!(kdf2.t_cost, kdf.t_cost);
        assert_eq!(kdf2.p_cost, kdf.p_cost);

        assert_eq!(wrapped2.nonce, wrapped.nonce);
        assert_eq!(wrapped2.ciphertext, wrapped.ciphertext);
        assert_eq!(verifier2, verifier);

        // Overwrite meta (DELETE + INSERT) should still work
        let kdf_new = KdfParams {
            salt: vec![9, 9, 9, 9, 9, 9, 9, 9],
            ..kdf
        };
        let wrapped_new = mk_wrapped(0x22);
        let verifier_new = vec![0xAA, 0xBB, 0xCC];
        db.write_meta(&kdf_new, &wrapped_new, &verifier_new)?;

        let (kdf3, wrapped3, verifier3) = db.read_meta()?.expect("meta present");
        assert_eq!(kdf3.salt, kdf_new.salt);
        assert_eq!(wrapped3.ciphertext, wrapped_new.ciphertext);
        assert_eq!(verifier3, verifier_new);

        fs::remove_file(path).ok();
        Ok(())
    }

    #[test]
    fn test_insert_and_list_items_and_ordering() -> Result<()> {
        let path = tmp_path("items_basic");
        let db = Db::open(&path)?;

        db.insert_item("b-name", "password", b"nonce1", b"ct1")?;
        db.insert_item("a-name", "env", b"nonce2", b"ct2")?;
        db.insert_item("c-name", "note", b"nonce3", b"ct3")?;

        let rows = db.list_items()?;
        // list_items orders by name ASC
        let names: Vec<_> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["a-name", "b-name", "c-name"]);

        // Check fields present and timestamps parseable (already parsed in list_items)
        let row = &rows[0];
        assert_eq!(row.kind, "env");
        assert_eq!(row.nonce, b"nonce2");
        assert_eq!(row.ciphertext, b"ct2");
        // created_at and updated_at should be close to now, but we just ensure they exist
        assert!(row.created_at <= time::OffsetDateTime::now_utc());
        assert!(row.updated_at <= time::OffsetDateTime::now_utc());

        fs::remove_file(path).ok();
        Ok(())
    }

    #[test]
    fn test_unique_name_constraint() -> Result<()> {
        let path = tmp_path("unique_name");
        let db = Db::open(&path)?;

        db.insert_item("unique", "password", b"n", b"c")?;
        let dup = db.insert_item("unique", "password", b"n2", b"c2");

        assert!(dup.is_err(), "duplicate item with same name should fail");

        fs::remove_file(path).ok();
        Ok(())
    }

    #[test]
    fn test_update_item_changes_ciphertext_and_updated_at() -> Result<()> {
        let path = tmp_path("update_item");
        let db = Db::open(&path)?;

        db.insert_item("item", "note", b"n0", b"c0")?;
        let before = db.list_items()?;
        assert_eq!(before.len(), 1);
        let id = before[0].id;
        let before_updated = before[0].updated_at;

        // Ensure a measurable time difference for updated_at
        thread::sleep(Duration::from_millis(10));

        db.update_item(id, b"n1", b"c1")?;

        let after = db.list_items()?;
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].id, id);
        assert_eq!(after[0].nonce, b"n1");
        assert_eq!(after[0].ciphertext, b"c1");
        assert!(after[0].updated_at > before_updated);

        fs::remove_file(path).ok();
        Ok(())
    }

    #[test]
    fn test_delete_item_removes_row() -> Result<()> {
        let path = tmp_path("delete_item");
        let db = Db::open(&path)?;

        db.insert_item("to-del-1", "note", b"n", b"c")?;
        db.insert_item("to-del-2", "note", b"n2", b"c2")?;
        let rows = db.list_items()?;
        assert_eq!(rows.len(), 2);
        let id = rows[0].id;

        db.delete_item(id)?;
        let rows2 = db.list_items()?;
        assert_eq!(rows2.len(), 1);
        assert_ne!(rows2[0].id, id);

        fs::remove_file(path).ok();
        Ok(())
    }

    #[test]
    fn test_itemrow_ad_helpers() {
        let name = "example";
        let kind = "password";
        let ad = ItemRow::ad_for_name_kind(name, kind);
        // Ensure it contains both fields with a separator 0x1f
        let expected = {
            let mut v = Vec::new();
            v.extend_from_slice(name.as_bytes());
            v.push(0x1f);
            v.extend_from_slice(kind.as_bytes());
            v
        };
        assert_eq!(ad, expected);

        // Construct a dummy row to test ad()
        let row = ItemRow {
            id: 1,
            name: name.to_string(),
            kind: kind.to_string(),
            nonce: vec![],
            ciphertext: vec![],
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        };
        assert_eq!(row.ad(), expected);
    }
}
