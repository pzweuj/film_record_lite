use axum::extract::State;
use axum::middleware;
use axum::response::IntoResponse;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme};
use utoipa::{IntoParams, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

use crate::auth::require_token;
use crate::db::FilmDatabase;
use crate::error::{AppError, ValidatedJson, ValidatedPath, ValidatedQuery};
use crate::format::format_film;
use crate::models::{FilmBatch, FilmCreate, FilmUpdate, Pagination, RatingUpdate};
use crate::AppState;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct TitleQuery {
    /// Partial film title.
    pub title: String,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ActorQuery {
    /// Partial actor name.
    pub actor: String,
}

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi
            .components
            .get_or_insert_with(utoipa::openapi::Components::new);
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );
        components.add_security_scheme(
            "query_token",
            SecurityScheme::ApiKey(ApiKey::Query(ApiKeyValue::new("token"))),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        add_film,
        add_films_batch,
        get_film,
        update_rating,
        update_film,
        delete_film,
        delete_film_by_title,
        list_films_by_actor,
        list_all_films,
        health_check
    ),
    components(schemas(
        crate::models::FilmRecord,
        FilmCreate,
        FilmBatch,
        RatingUpdate,
        FilmUpdate
    )),
    modifiers(&SecurityAddon),
    info(
        title = "FilmRecordLite",
        version = "0.1.0",
        description = "A lightweight film record management API"
    )
)]
pub struct ApiDoc;

pub fn build_router(state: AppState) -> Router {
    // Every data route is registered in this router and receives the auth
    // middleware as a single layer. New data endpoints must not be added to
    // the public router below.
    let protected = Router::<AppState>::new()
        .route(
            "/films",
            post(add_film).get(get_film).delete(delete_film_by_title),
        )
        .route("/films/batch", post(add_films_batch))
        .route("/films/rating", put(update_rating))
        .route("/films/by-actor", get(list_films_by_actor))
        .route("/films/all", get(list_all_films))
        .route("/films/{film_id}", put(update_film).delete(delete_film))
        .layer(middleware::from_fn_with_state(state.clone(), require_token));

    let docs: Router<AppState> = SwaggerUi::new("/docs")
        .url("/openapi.json", ApiDoc::openapi())
        .into();

    Router::<AppState>::new()
        .route("/health", get(health_check))
        .merge(docs)
        .merge(protected)
        .with_state(state)
}

#[utoipa::path(
    get,
    path = "/health",
    responses((status = 200, description = "Health status"))
)]
async fn health_check() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

#[utoipa::path(
    post,
    path = "/films",
    request_body = FilmCreate,
    responses((status = 200, description = "Film added", body = crate::models::FilmRecord)),
    security(("bearer_auth" = []), ("query_token" = []))
)]
async fn add_film(
    State(state): State<AppState>,
    ValidatedJson(film): ValidatedJson<FilmCreate>,
) -> Result<Json<crate::models::FilmRecord>, AppError> {
    film.validate().map_err(AppError::Validation)?;
    let record_date = film
        .record_date
        .clone()
        .unwrap_or_else(FilmDatabase::today_local);
    let record = state.db.add_film(&film, record_date).await?;
    Ok(Json(record))
}

#[utoipa::path(
    post,
    path = "/films/batch",
    request_body = FilmBatch,
    responses((status = 200, description = "Films added")),
    security(("bearer_auth" = []), ("query_token" = []))
)]
async fn add_films_batch(
    State(state): State<AppState>,
    ValidatedJson(batch): ValidatedJson<FilmBatch>,
) -> Result<Json<serde_json::Value>, AppError> {
    batch.validate().map_err(AppError::Validation)?;
    let mut added = Vec::with_capacity(batch.films.len());
    for film in &batch.films {
        let record_date = film
            .record_date
            .clone()
            .unwrap_or_else(FilmDatabase::today_local);
        added.push(state.db.add_film(film, record_date).await?);
    }

    let count = added.len();
    Ok(Json(json!({
        "message": format!("已成功添加 {count} 部电影记录"),
        "films": added.iter().map(format_film).collect::<Vec<_>>(),
        "data": added,
        "count": count
    })))
}

#[utoipa::path(
    get,
    path = "/films",
    params(TitleQuery),
    responses((status = 200, description = "Films matching title")),
    security(("bearer_auth" = []), ("query_token" = []))
)]
async fn get_film(
    State(state): State<AppState>,
    ValidatedQuery(query): ValidatedQuery<TitleQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let films = state.db.get_film_by_title(&query.title).await?;
    if films.is_empty() {
        return Ok(Json(json!({
            "message": format!("未找到标题包含「{}」的电影", query.title),
            "films": []
        })));
    }

    let count = films.len();
    Ok(Json(json!({
        "message": format!("找到 {count} 部电影"),
        "films": films.iter().map(format_film).collect::<Vec<_>>(),
        "data": films
    })))
}

#[utoipa::path(
    put,
    path = "/films/rating",
    request_body = RatingUpdate,
    responses((status = 200, description = "Ratings updated")),
    security(("bearer_auth" = []), ("query_token" = []))
)]
async fn update_rating(
    State(state): State<AppState>,
    ValidatedJson(update): ValidatedJson<RatingUpdate>,
) -> Result<Json<serde_json::Value>, AppError> {
    update.validate().map_err(AppError::Validation)?;
    let films = state.db.update_rating(&update.title, update.rating).await?;
    if films.is_empty() {
        return Ok(Json(json!({
            "message": format!("未找到标题包含「{}」的电影", update.title)
        })));
    }

    Ok(Json(json!({
        "message": "已更新评分",
        "films": films.iter().map(format_film).collect::<Vec<_>>(),
        "data": films
    })))
}

#[utoipa::path(
    put,
    path = "/films/{film_id}",
    params(("film_id" = i64, Path, description = "Film ID")),
    request_body = FilmUpdate,
    responses((status = 200, description = "Film updated")),
    security(("bearer_auth" = []), ("query_token" = []))
)]
async fn update_film(
    State(state): State<AppState>,
    ValidatedPath(film_id): ValidatedPath<i64>,
    ValidatedJson(update): ValidatedJson<FilmUpdate>,
) -> Result<Json<serde_json::Value>, AppError> {
    update.validate().map_err(AppError::Validation)?;
    if state.db.get_film_by_id(film_id).await?.is_none() {
        return Err(AppError::NotFound(format!("未找到 ID 为 {film_id} 的电影")));
    }

    let updated = state
        .db
        .update_film(film_id, &update)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("未找到 ID 为 {film_id} 的电影")))?;
    Ok(Json(json!({
        "message": "已更新电影信息",
        "film": format_film(&updated),
        "data": updated
    })))
}

#[utoipa::path(
    delete,
    path = "/films/{film_id}",
    params(("film_id" = i64, Path, description = "Film ID")),
    responses((status = 200, description = "Film deleted")),
    security(("bearer_auth" = []), ("query_token" = []))
)]
async fn delete_film(
    State(state): State<AppState>,
    ValidatedPath(film_id): ValidatedPath<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !state.db.delete_film(film_id).await? {
        return Err(AppError::NotFound(format!("未找到 ID 为 {film_id} 的电影")));
    }
    Ok(Json(json!({
        "message": format!("已删除电影记录 (ID: {film_id})"),
        "deleted": true,
        "count": 1
    })))
}

#[utoipa::path(
    delete,
    path = "/films",
    params(TitleQuery),
    responses((status = 200, description = "Films deleted")),
    security(("bearer_auth" = []), ("query_token" = []))
)]
async fn delete_film_by_title(
    State(state): State<AppState>,
    ValidatedQuery(query): ValidatedQuery<TitleQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let count = state.db.delete_film_by_title(&query.title).await?;
    if count == 0 {
        return Ok(Json(json!({
            "message": format!("未找到标题包含「{}」的电影", query.title),
            "deleted": false,
            "count": 0
        })));
    }
    Ok(Json(json!({
        "message": format!("已删除 {count} 部标题包含「{}」的电影", query.title),
        "deleted": true,
        "count": count
    })))
}

#[utoipa::path(
    get,
    path = "/films/by-actor",
    params(ActorQuery),
    responses((status = 200, description = "Films matching actor")),
    security(("bearer_auth" = []), ("query_token" = []))
)]
async fn list_films_by_actor(
    State(state): State<AppState>,
    ValidatedQuery(query): ValidatedQuery<ActorQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let films = state.db.list_films_by_actor(&query.actor).await?;
    if films.is_empty() {
        return Ok(Json(json!({
            "message": format!("未找到「{}」主演的电影", query.actor),
            "films": []
        })));
    }

    let count = films.len();
    Ok(Json(json!({
        "message": format!("「{}」主演的电影 ({count} 部)", query.actor),
        "films": films.iter().map(format_film).collect::<Vec<_>>(),
        "data": films
    })))
}

#[utoipa::path(
    get,
    path = "/films/all",
    params(
        ("limit" = Option<i64>, Query, description = "Number of results per page"),
        ("offset" = Option<i64>, Query, description = "Number of results to skip")
    ),
    responses((status = 200, description = "Paginated films")),
    security(("bearer_auth" = []), ("query_token" = []))
)]
async fn list_all_films(
    State(state): State<AppState>,
    ValidatedQuery(query): ValidatedQuery<Pagination>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (limit, offset) = query.values().map_err(AppError::Validation)?;
    let films = state.db.list_all_films(limit, offset).await?;
    let total = state.db.get_total_count().await?;
    if films.is_empty() {
        return Ok(Json(json!({
            "message": "暂无电影记录",
            "films": [],
            "data": [],
            "total": 0,
            "limit": limit,
            "offset": offset
        })));
    }

    Ok(Json(json!({
        "message": format!(
            "所有电影记录 (共 {total} 部，显示 {}-{} 部)",
            offset + 1,
            offset + films.len() as i64
        ),
        "films": films.iter().map(format_film).collect::<Vec<_>>(),
        "data": films,
        "total": total,
        "limit": limit,
        "offset": offset
    })))
}
