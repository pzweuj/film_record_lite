"""Database operations for FilmRecordLite."""

import sqlite3
from datetime import date
from pathlib import Path
from typing import Optional

from pydantic import BaseModel

# Default database path
DEFAULT_DB_PATH = Path(__file__).parent.parent.parent / "data" / "films.db"


class FilmRecord(BaseModel):
    """Film record data model."""

    id: int
    title: str
    actors: Optional[str] = None
    plot: Optional[str] = None
    review: Optional[str] = None
    rating: Optional[float] = None
    record_date: Optional[str] = None
    created_at: Optional[str] = None
    updated_at: Optional[str] = None


class FilmDatabase:
    """SQLite database manager for film records."""

    def __init__(self, db_path: Optional[str | Path] = None):
        self.db_path = Path(db_path) if db_path else DEFAULT_DB_PATH
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        self._init_db()

    def _get_connection(self) -> sqlite3.Connection:
        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        return conn

    def _init_db(self) -> None:
        """Initialize the database schema."""
        with self._get_connection() as conn:
            conn.execute("""
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
            """)

    def add_film(
        self,
        title: str,
        actors: Optional[str] = None,
        plot: Optional[str] = None,
        review: Optional[str] = None,
        rating: Optional[float] = None,
        record_date: Optional[str] = None,
    ) -> FilmRecord:
        """Add a new film record."""
        with self._get_connection() as conn:
            cursor = conn.execute(
                """
                INSERT INTO films (title, actors, plot, review, rating, record_date)
                VALUES (?, ?, ?, ?, ?, ?)
                """,
                (title, actors, plot, review, rating, record_date or date.today().isoformat()),
            )
            film_id = cursor.lastrowid
            conn.commit()

        return self.get_film_by_id(film_id)

    def get_film_by_id(self, film_id: int) -> Optional[FilmRecord]:
        """Get a film by its ID."""
        with self._get_connection() as conn:
            row = conn.execute(
                "SELECT * FROM films WHERE id = ?", (film_id,)
            ).fetchone()
            return FilmRecord.model_validate(dict(row)) if row else None

    def get_film_by_title(self, title: str) -> list[FilmRecord]:
        """Search for films by title (fuzzy match)."""
        with self._get_connection() as conn:
            rows = conn.execute(
                "SELECT * FROM films WHERE title LIKE ? ORDER BY created_at DESC",
                (f"%{title}%",),
            ).fetchall()
            return [FilmRecord.model_validate(dict(row)) for row in rows]

    def update_rating(self, title: str, rating: float) -> list[FilmRecord]:
        """Update the rating of films matching the title."""
        with self._get_connection() as conn:
            conn.execute(
                """
                UPDATE films
                SET rating = ?, updated_at = CURRENT_TIMESTAMP
                WHERE title LIKE ?
                """,
                (rating, f"%{title}%"),
            )
            conn.commit()
        return self.get_film_by_title(title)

    def delete_film(self, film_id: int) -> bool:
        """Delete a film by ID. Returns True if deleted."""
        with self._get_connection() as conn:
            cursor = conn.execute(
                "DELETE FROM films WHERE id = ?", (film_id,)
            )
            conn.commit()
            return cursor.rowcount > 0

    def delete_film_by_title(self, title: str) -> int:
        """Delete films matching title (fuzzy match). Returns count deleted."""
        with self._get_connection() as conn:
            cursor = conn.execute(
                "DELETE FROM films WHERE title LIKE ?", (f"%{title}%",)
            )
            conn.commit()
            return cursor.rowcount

    def list_films_by_actor(self, actor: str) -> list[FilmRecord]:
        """List all films featuring a specific actor."""
        with self._get_connection() as conn:
            rows = conn.execute(
                """
                SELECT * FROM films
                WHERE actors LIKE ?
                ORDER BY record_date DESC
                """,
                (f"%{actor}%",),
            ).fetchall()
            return [FilmRecord.model_validate(dict(row)) for row in rows]

    def list_all_films(self, limit: int = 20, offset: int = 0) -> list[FilmRecord]:
        """List film records with pagination."""
        with self._get_connection() as conn:
            rows = conn.execute(
                "SELECT * FROM films ORDER BY record_date DESC LIMIT ? OFFSET ?",
                (limit, offset),
            ).fetchall()
            return [FilmRecord.model_validate(dict(row)) for row in rows]

    def get_total_count(self) -> int:
        """Get total number of film records."""
        with self._get_connection() as conn:
            row = conn.execute("SELECT COUNT(*) FROM films").fetchone()
            return row[0] if row else 0