use crate::models::FilmRecord;

pub fn format_film(film: &FilmRecord) -> String {
    let rating = film
        .rating
        .map(format_rating)
        .unwrap_or_else(|| "未评分".to_string());
    format!(
        "**{}**\n- 主演: {}\n- 剧情: {}\n- 评价: {}\n- 评分: {}/10\n- 记录日期: {}",
        film.title,
        film.actors.as_deref().unwrap_or("未记录"),
        film.plot.as_deref().unwrap_or("未记录"),
        film.review.as_deref().unwrap_or("未记录"),
        rating,
        film.record_date.as_deref().unwrap_or("未记录"),
    )
}

fn format_rating(rating: f64) -> String {
    let rendered = rating.to_string();
    if rendered.contains('.') || rendered.contains('e') || rendered.contains('E') {
        rendered
    } else {
        format!("{rendered}.0")
    }
}
