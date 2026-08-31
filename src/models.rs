use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, FromRow, utoipa::ToSchema)]
pub struct FilmRecord {
    pub id: i64,
    pub title: String,
    pub actors: Option<String>,
    pub plot: Option<String>,
    pub review: Option<String>,
    pub rating: Option<f64>,
    pub record_date: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct FilmCreate {
    pub title: String,
    pub actors: Option<String>,
    pub plot: Option<String>,
    pub review: Option<String>,
    pub rating: Option<f64>,
    pub record_date: Option<String>,
}

impl FilmCreate {
    pub fn validate(&self) -> Result<(), String> {
        validate_rating(self.rating)
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct FilmBatch {
    pub films: Vec<FilmCreate>,
}

impl FilmBatch {
    pub fn validate(&self) -> Result<(), String> {
        for film in &self.films {
            film.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct RatingUpdate {
    pub title: String,
    pub rating: f64,
}

impl RatingUpdate {
    pub fn validate(&self) -> Result<(), String> {
        validate_rating(Some(self.rating))
    }
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
pub struct FilmUpdate {
    pub title: Option<String>,
    pub actors: Option<String>,
    pub plot: Option<String>,
    pub review: Option<String>,
    pub rating: Option<f64>,
    pub record_date: Option<String>,
}

impl FilmUpdate {
    pub fn validate(&self) -> Result<(), String> {
        validate_rating(self.rating)
    }

    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.actors.is_none()
            && self.plot.is_none()
            && self.review.is_none()
            && self.rating.is_none()
            && self.record_date.is_none()
    }
}

fn validate_rating(value: Option<f64>) -> Result<(), String> {
    if let Some(rating) = value {
        if !rating.is_finite() || !(0.0..=10.0).contains(&rating) {
            return Err("Rating must be between 0 and 10".to_string());
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pagination {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            limit: Some(20),
            offset: Some(0),
        }
    }
}

impl Pagination {
    pub fn values(&self) -> Result<(i64, i64), String> {
        let limit = self.limit.unwrap_or(20);
        let offset = self.offset.unwrap_or(0);
        if !(1..=100).contains(&limit) {
            return Err("limit must be between 1 and 100".to_string());
        }
        if offset < 0 {
            return Err("offset must be greater than or equal to 0".to_string());
        }
        Ok((limit, offset))
    }
}
