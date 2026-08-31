use std::path::Path;
use std::time::Duration;

use chrono::Local;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};

use crate::models::{FilmCreate, FilmRecord, FilmUpdate};

const CREATE_FILMS_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS films (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        title TEXT NOT NULL,
        actors TEXT,
        plot TEXT,
        review TEXT,
        rating REAL,
        record_date DATE,
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
        updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
    )
"#;

#[derive(Clone)]
pub struct FilmDatabase {
    pool: SqlitePool,
}

impl FilmDatabase {
    pub async fn connect(path: &Path) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        sqlx::query(CREATE_FILMS_TABLE).execute(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn add_film(
        &self,
        film: &FilmCreate,
        record_date: String,
    ) -> Result<FilmRecord, sqlx::Error> {
        let mut conn = self.pool.acquire().await?;
        let result = sqlx::query(
            "INSERT INTO films (title, actors, plot, review, rating, record_date) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&film.title)
        .bind(&film.actors)
        .bind(&film.plot)
        .bind(&film.review)
        .bind(film.rating)
        .bind(record_date)
        .execute(&mut *conn)
        .await?;

        let film_id = result.last_insert_rowid();
        sqlx::query_as::<_, FilmRecord>("SELECT * FROM films WHERE id = ?")
            .bind(film_id)
            .fetch_one(&mut *conn)
            .await
    }

    pub async fn get_film_by_id(&self, film_id: i64) -> Result<Option<FilmRecord>, sqlx::Error> {
        sqlx::query_as::<_, FilmRecord>("SELECT * FROM films WHERE id = ?")
            .bind(film_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn get_film_by_title(&self, title: &str) -> Result<Vec<FilmRecord>, sqlx::Error> {
        sqlx::query_as::<_, FilmRecord>(
            "SELECT * FROM films WHERE title LIKE ? ORDER BY created_at DESC",
        )
        .bind(format!("%{title}%"))
        .fetch_all(&self.pool)
        .await
    }

    pub async fn update_rating(
        &self,
        title: &str,
        rating: f64,
    ) -> Result<Vec<FilmRecord>, sqlx::Error> {
        sqlx::query(
            "UPDATE films SET rating = ?, updated_at = CURRENT_TIMESTAMP WHERE title LIKE ?",
        )
        .bind(rating)
        .bind(format!("%{title}%"))
        .execute(&self.pool)
        .await?;
        self.get_film_by_title(title).await
    }

    pub async fn update_film(
        &self,
        film_id: i64,
        update: &FilmUpdate,
    ) -> Result<Option<FilmRecord>, sqlx::Error> {
        if update.is_empty() {
            return self.get_film_by_id(film_id).await;
        }

        let mut builder = QueryBuilder::<Sqlite>::new("UPDATE films SET ");
        let mut first_assignment = true;
        if let Some(value) = &update.title {
            if !first_assignment {
                builder.push(", ");
            }
            builder.push("title = ").push_bind(value);
            first_assignment = false;
        }
        if let Some(value) = &update.actors {
            if !first_assignment {
                builder.push(", ");
            }
            builder.push("actors = ").push_bind(value);
            first_assignment = false;
        }
        if let Some(value) = &update.plot {
            if !first_assignment {
                builder.push(", ");
            }
            builder.push("plot = ").push_bind(value);
            first_assignment = false;
        }
        if let Some(value) = &update.review {
            if !first_assignment {
                builder.push(", ");
            }
            builder.push("review = ").push_bind(value);
            first_assignment = false;
        }
        if let Some(value) = update.rating {
            if !first_assignment {
                builder.push(", ");
            }
            builder.push("rating = ").push_bind(value);
            first_assignment = false;
        }
        if let Some(value) = &update.record_date {
            if !first_assignment {
                builder.push(", ");
            }
            builder.push("record_date = ").push_bind(value);
            first_assignment = false;
        }
        if !first_assignment {
            builder.push(", ");
        }
        builder.push("updated_at = CURRENT_TIMESTAMP");
        builder.push(" WHERE id = ").push_bind(film_id);
        builder.build().execute(&self.pool).await?;
        self.get_film_by_id(film_id).await
    }

    pub async fn delete_film(&self, film_id: i64) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM films WHERE id = ?")
            .bind(film_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_film_by_title(&self, title: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM films WHERE title LIKE ?")
            .bind(format!("%{title}%"))
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn list_films_by_actor(&self, actor: &str) -> Result<Vec<FilmRecord>, sqlx::Error> {
        sqlx::query_as::<_, FilmRecord>(
            "SELECT * FROM films WHERE actors LIKE ? ORDER BY record_date DESC",
        )
        .bind(format!("%{actor}%"))
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_all_films(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<FilmRecord>, sqlx::Error> {
        sqlx::query_as::<_, FilmRecord>(
            "SELECT * FROM films ORDER BY record_date DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_total_count(&self) -> Result<i64, sqlx::Error> {
        let row = sqlx::query("SELECT COUNT(*) AS count FROM films")
            .fetch_one(&self.pool)
            .await?;
        row.try_get("count")
    }

    pub fn today_local() -> String {
        Local::now().date_naive().to_string()
    }
}
