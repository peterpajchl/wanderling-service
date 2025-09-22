use anyhow::Result;
use axum::http::Method;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tokio::{fs::File, io::AsyncReadExt, net::TcpListener};
use tower_http::cors::{Any, CorsLayer};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Country {
    id: u8,
    #[serde(alias = "country_short_form_name")]
    country: String,
    capital: String,
    #[serde(alias = "country_code_2letter")]
    country_code: String,
    capital_latitude: f32,
    capital_longitude: f32,
    country_audio_filename: String,
    capital_audio_filename: Option<String>,
}

#[derive(Clone)]
enum Predicate {
    CountryCode(String),
    Name(String),
    Tag(String),
}

#[derive(Serialize)]
struct Pagination {
    page: u32,
    items_per_page: u32,
    total_items: u32,
}

#[derive(Serialize)]
struct CountryListResponse {
    data: Vec<Country>,
    pagination: Pagination,
}

#[derive(Debug, Clone)]
struct Dataset {
    by_id: HashMap<u8, Country>,
    all_items: Vec<Country>,
}

impl Dataset {
    fn get_by_id(&self, id: u8) -> Option<Country> {
        match self.by_id.get(&id) {
            None => None,
            Some(x) => Some(x.clone()),
        }
    }

    fn get_items_with_predicate(
        &self,
        predicate: Option<Predicate>,
        page: u32,
        limit: u32,
    ) -> CountryListResponse {
        if let Some(p) = predicate {
            let data = self
                .all_items
                .iter()
                .filter(|&x| match p.clone() {
                    Predicate::CountryCode(code) => {
                        x.country_code.to_lowercase().eq(&code.to_lowercase())
                    }
                    Predicate::Name(name) => {
                        x.country.to_lowercase().starts_with(&name.to_lowercase())
                    }
                    Predicate::Tag(tag) => {
                        x.country.to_lowercase().starts_with(&tag.to_lowercase())
                    }
                })
                .skip((page * limit) as usize)
                .take(limit as usize)
                .cloned()
                .collect();

            CountryListResponse {
                data,
                pagination: Pagination {
                    page,
                    items_per_page: limit,
                    total_items: self.all_items.iter().count() as u32,
                },
            }
        } else {
            let data = self
                .all_items
                .iter()
                .skip((page * limit) as usize)
                .take(limit as usize)
                .cloned()
                .collect();

            CountryListResponse {
                data,
                pagination: Pagination {
                    page,
                    items_per_page: limit,
                    total_items: self.all_items.iter().count() as u32,
                },
            }
        }
    }
}

impl From<Vec<Country>> for Dataset {
    fn from(value: Vec<Country>) -> Self {
        let mut map: HashMap<u8, Country> = HashMap::new();
        value.iter().for_each(|x| {
            map.insert(x.id, x.clone());
        });
        Dataset {
            by_id: map,
            all_items: value,
        }
    }
}

async fn load_dataset() -> Result<Dataset> {
    let mut file = File::open("input.json").await?;
    let mut file_content = String::new();
    file.read_to_string(&mut file_content).await?;
    let dataset: Dataset = serde_json::from_str::<Vec<Country>>(&file_content)?.into();
    Ok(dataset)
}

#[derive(Clone)]
struct AppState {
    db: Dataset,
}

pub async fn run() -> Result<()> {
    let dataset = load_dataset().await?;
    let state = AppState { db: dataset };

    let cors = CorsLayer::new()
        .allow_methods([Method::GET])
        .allow_origin(Any);

    let tcp_listener = TcpListener::bind("127.0.0.1:4123").await?;
    let router = Router::new()
        .route("/", get(api_handler_root))
        .route("/api/countries", get(api_handler_countries_list))
        .route("/api/countries/{id}", get(api_handler_countries_get))
        .with_state(state)
        .layer(cors);

    axum::serve(tcp_listener, router).await?;
    Ok(())
}

async fn api_handler_root() -> impl IntoResponse {
    (StatusCode::OK, "Hello world")
}

async fn api_handler_countries_get(
    State(app_state): State<AppState>,
    Path(id): Path<u8>,
) -> impl IntoResponse {
    match app_state.db.get_by_id(id) {
        Some(x) => (StatusCode::OK, Json(json!(x))),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({"msg": "Country not found"})),
        ),
    }
}

#[derive(Deserialize)]
struct QueryParams {
    filter_tag: Option<String>,
    filter_name: Option<String>,
    filter_country_code: Option<String>,
    page: Option<u32>,
    items_per_page: Option<u32>,
}

async fn api_handler_countries_list(
    State(app_state): State<AppState>,
    Query(query): Query<QueryParams>,
) -> impl IntoResponse {
    let max = query.items_per_page.unwrap_or(10);
    let page = query.page.unwrap_or(0);
    let predicate = if let Some(p) = query.filter_country_code {
        Some(Predicate::CountryCode(p))
    } else if let Some(p) = query.filter_name {
        Some(Predicate::Name(p))
    } else if let Some(p) = query.filter_tag {
        Some(Predicate::Tag(p))
    } else {
        None
    };
    let data = app_state.db.get_items_with_predicate(predicate, page, max);
    (StatusCode::OK, Json(data))
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_json_import() {
        let d = load_dataset().await;
        assert!(
            d.is_ok(),
            "Failed to import JSON into Dataset: {:?}",
            d.err()
        );
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let d = load_dataset().await.unwrap();
        let country = d.get_by_id(3);
        assert!(country.is_some(), "Failed to lookup by id");
        assert!(
            country.unwrap().id == 3,
            "Country ID does not match lookup ID"
        );
    }

    #[tokio::test]
    async fn test_filter_by_predicate_none() {
        let d = load_dataset().await.unwrap();
        let result = d.get_items_with_predicate(None, 0, 10);
        assert!(
            result.len() == 197,
            "We should have 197 items when no filtering is applied"
        );
    }

    #[tokio::test]
    async fn test_filter_by_predicate_country_code() {
        let d = load_dataset().await.unwrap();
        let result =
            d.get_items_with_predicate(Some(Predicate::CountryCode(String::from("AO"))), 0, 10);
        assert!(
            result.len() == 1,
            "We should have 1 item when filtered by country code"
        );

        assert!(
            result[0].country_code == "AO",
            "The filtered country should have same country code"
        );
    }

    #[tokio::test]
    async fn test_filter_by_predicate_name() {
        let d = load_dataset().await.unwrap();
        let result = d.get_items_with_predicate(Some(Predicate::Name(String::from("an"))), 0, 10);
        assert!(
            result.len() == 3,
            "We should have 3 item when filtered by name that matches several countries"
        );

        assert!(
            result[0].country.starts_with("An"),
            "The filtered country should have start with the filter string"
        );
    }
}
