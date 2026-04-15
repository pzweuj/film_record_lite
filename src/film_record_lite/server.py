"""FastAPI server for FilmRecordLite with token authentication."""

import argparse
import os
import hmac
from typing import Optional, List
from datetime import date

from fastapi import FastAPI, HTTPException, Depends, Header, Query
from fastapi.security import HTTPBearer
from pydantic import BaseModel, Field
import uvicorn

from .database import FilmDatabase, FilmRecord

# Pydantic models
class FilmCreate(BaseModel):
    title: str = Field(..., description="Film title")
    actors: Optional[str] = Field(None, description="Main actors, comma-separated")
    plot: Optional[str] = Field(None, description="Plot summary")
    review: Optional[str] = Field(None, description="Personal review")
    rating: Optional[float] = Field(None, ge=0, le=10, description="Rating 0-10")
    record_date: Optional[str] = Field(None, description="Record date YYYY-MM-DD")


class FilmBatch(BaseModel):
    films: List[FilmCreate]


class RatingUpdate(BaseModel):
    title: str
    rating: float = Field(..., ge=0, le=10)


class FilmUpdate(BaseModel):
    title: Optional[str] = Field(None, description="New film title")
    actors: Optional[str] = Field(None, description="Main actors, comma-separated")
    plot: Optional[str] = Field(None, description="Plot summary")
    review: Optional[str] = Field(None, description="Personal review")
    rating: Optional[float] = Field(None, ge=0, le=10, description="Rating 0-10")
    record_date: Optional[str] = Field(None, description="Record date YYYY-MM-DD")


class DeleteResponse(BaseModel):
    message: str
    deleted: bool = False
    count: int = 0


# Global variables
db: Optional[FilmDatabase] = None
auth_token: Optional[str] = None
app: Optional[FastAPI] = None


def verify_token(authorization: Optional[str] = Header(None), token: Optional[str] = Query(None)):
    """Verify authentication token."""
    if not auth_token:
        raise HTTPException(status_code=500, detail="Server authentication not configured")

    client_token = None
    if authorization:
        client_token = authorization.replace("Bearer ", "")
    elif token:
        client_token = token

    if not client_token:
        raise HTTPException(status_code=401, detail="Authentication token required")

    if not hmac.compare_digest(client_token, auth_token):
        raise HTTPException(status_code=401, detail="Invalid authentication token")

    return client_token


def format_film(film: FilmRecord) -> str:
    """Format a film record for display."""
    lines = [
        f"**{film.title}**",
        f"- 主演: {film.actors or '未记录'}",
        f"- 剧情: {film.plot or '未记录'}",
        f"- 评价: {film.review or '未记录'}",
        f"- 评分: {film.rating if film.rating is not None else '未评分'}/10",
        f"- 记录日期: {film.record_date or '未记录'}",
    ]
    return "\n".join(lines)


def create_app() -> FastAPI:
    """Create FastAPI application."""
    application = FastAPI(
        title="FilmRecordLite",
        description="A lightweight film record management API",
        version="0.1.0",
    )

    @application.post("/films", response_model=FilmRecord, summary="Add a film record")
    async def add_film(film: FilmCreate, _: str = Depends(verify_token)):
        """Add a single film record."""
        record = db.add_film(
            title=film.title,
            actors=film.actors,
            plot=film.plot,
            review=film.review,
            rating=film.rating,
            record_date=film.record_date or date.today().isoformat(),
        )
        return record.model_dump()

    @application.post("/films/batch", summary="Add multiple films")
    async def add_films_batch(batch: FilmBatch, _: str = Depends(verify_token)):
        """Add multiple film records at once."""
        added_films: List[FilmRecord] = []
        for film in batch.films:
            record = db.add_film(
                title=film.title,
                actors=film.actors,
                plot=film.plot,
                review=film.review,
                rating=film.rating,
                record_date=film.record_date or date.today().isoformat(),
            )
            added_films.append(record)

        return {
            "message": f"已成功添加 {len(added_films)} 部电影记录",
            "films": [format_film(f) for f in added_films],
            "data": [f.model_dump() for f in added_films],
            "count": len(added_films)
        }

    @application.get("/films", summary="Search films by title")
    async def get_film(title: str, _: str = Depends(verify_token)):
        """Search films by title (fuzzy match)."""
        films = db.get_film_by_title(title)
        if not films:
            return {"message": f"未找到标题包含「{title}」的电影", "films": []}

        return {
            "message": f"找到 {len(films)} 部电影",
            "films": [format_film(f) for f in films],
            "data": [f.model_dump() for f in films]
        }

    @application.put("/films/rating", summary="Update film rating")
    async def update_rating(update: RatingUpdate, _: str = Depends(verify_token)):
        """Update the rating of a film."""
        films = db.update_rating(update.title, update.rating)
        if not films:
            return {"message": f"未找到标题包含「{update.title}」的电影"}

        return {
            "message": "已更新评分",
            "films": [format_film(f) for f in films],
            "data": [f.model_dump() for f in films]
        }

    @application.put("/films/{film_id}", summary="Update film by ID")
    async def update_film(film_id: int, update: FilmUpdate, _: str = Depends(verify_token)):
        """Update a film record by ID. Only provided fields will be updated."""
        existing = db.get_film_by_id(film_id)
        if not existing:
            raise HTTPException(status_code=404, detail=f"未找到 ID 为 {film_id} 的电影")

        updated = db.update_film(
            film_id=film_id,
            title=update.title,
            actors=update.actors,
            plot=update.plot,
            review=update.review,
            rating=update.rating,
            record_date=update.record_date,
        )

        return {
            "message": "已更新电影信息",
            "film": format_film(updated),
            "data": updated.model_dump()
        }

    @application.delete("/films/{film_id}", summary="Delete film by ID")
    async def delete_film(film_id: int, _: str = Depends(verify_token)):
        """Delete a film record by its ID."""
        deleted = db.delete_film(film_id)
        if not deleted:
            raise HTTPException(status_code=404, detail=f"未找到 ID 为 {film_id} 的电影")

        return {"message": f"已删除电影记录 (ID: {film_id})", "deleted": True, "count": 1}

    @application.delete("/films", summary="Delete films by title")
    async def delete_film_by_title(title: str, _: str = Depends(verify_token)):
        """Delete films matching title (fuzzy match)."""
        count = db.delete_film_by_title(title)
        if count == 0:
            return {"message": f"未找到标题包含「{title}」的电影", "deleted": False, "count": 0}

        return {"message": f"已删除 {count} 部标题包含「{title}」的电影", "deleted": True, "count": count}

    @application.get("/films/by-actor", summary="List films by actor")
    async def list_films_by_actor(actor: str, _: str = Depends(verify_token)):
        """List all films featuring a specific actor."""
        films = db.list_films_by_actor(actor)
        if not films:
            return {"message": f"未找到「{actor}」主演的电影", "films": []}

        return {
            "message": f"「{actor}」主演的电影 ({len(films)} 部)",
            "films": [format_film(f) for f in films],
            "data": [f.model_dump() for f in films]
        }

    @application.get("/films/all", summary="List all films with pagination")
    async def list_all_films(
        limit: int = Query(20, ge=1, le=100, description="Number of results per page"),
        offset: int = Query(0, ge=0, description="Offset for pagination"),
        _: str = Depends(verify_token)
    ):
        """List film records with pagination."""
        films = db.list_all_films(limit=limit, offset=offset)
        total = db.get_total_count()
        if not films:
            return {"message": "暂无电影记录", "films": [], "data": [], "total": 0, "limit": limit, "offset": offset}

        return {
            "message": f"所有电影记录 (共 {total} 部，显示 {offset+1}-{offset+len(films)} 部)",
            "films": [format_film(f) for f in films],
            "data": [f.model_dump() for f in films],
            "total": total,
            "limit": limit,
            "offset": offset
        }

    @application.get("/health", summary="Health check")
    async def health_check():
        """Health check endpoint (no auth required)."""
        return {"status": "ok"}

    return application


def main():
    """Run the FastAPI server."""
    parser = argparse.ArgumentParser(
        description="FilmRecordLite FastAPI Server",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python -m film_record_lite.server --token mysecret123 --port 8080
  FILM_RECORD_TOKEN=mysecret123 python -m film_record_lite.server --port 8080
        """
    )
    parser.add_argument(
        "--host",
        default="0.0.0.0",
        help="Host address (default: 0.0.0.0)",
    )
    parser.add_argument(
        "--port",
        type=int,
        default=8000,
        help="Port (default: 8000)",
    )
    parser.add_argument(
        "--token",
        default=None,
        help="Authentication token (or set FILM_RECORD_TOKEN env var)",
    )
    parser.add_argument(
        "--db",
        default=None,
        help="Database file path (default: data/films.db)",
    )

    args = parser.parse_args()

    # Initialize database
    global db
    db = FilmDatabase(args.db)

    # Get token
    token = args.token or os.environ.get("FILM_RECORD_TOKEN")
    if not token:
        print("Error: Authentication token required.")
        print("Set --token argument or FILM_RECORD_TOKEN environment variable.")
        return

    # Check token strength
    if len(token) < 8:
        print("Error: Token too weak. Minimum 8 characters required.")
        return

    global auth_token
    auth_token = token

    global app
    app = create_app()

    print(f"Starting server on {args.host}:{args.port}")
    print(f"API docs: http://{args.host}:{args.port}/docs")
    print(f"Token: {token[:4]}...{token[-4:]}")

    uvicorn.run(app, host=args.host, port=args.port)


if __name__ == "__main__":
    main()