use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::{fs::File, io::AsyncReadExt};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Country {
    id: u8,
    #[serde(rename = "country_short_form_name")]
    country: String,
    capital_city: String,
    #[serde(rename = "country_code_2letter")]
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

#[derive(Debug)]
struct Dataset {
    by_id: HashMap<u8, Country>,
    all_items: Vec<Country>,
}

impl Dataset {
    fn get_by_id(&self, id: u8) -> Option<&Country> {
        self.by_id.get(&id)
    }

    fn get_items_with_predicate(&self, predicate: Option<Predicate>) -> Vec<&Country> {
        if let Some(p) = predicate {
            self.all_items
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
                .collect()
        } else {
            self.all_items.iter().collect()
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

pub async fn run() -> Result<()> {
    load_dataset().await?;
    Ok(())
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
        let result = d.get_items_with_predicate(None);
        assert!(
            result.len() == 6,
            "We should have 6 items when no filtering is applied"
        );
    }

    #[tokio::test]
    async fn test_filter_by_predicate_country_code() {
        let d = load_dataset().await.unwrap();
        let result = d.get_items_with_predicate(Some(Predicate::CountryCode(String::from("AO"))));
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
        let result = d.get_items_with_predicate(Some(Predicate::Name(String::from("an"))));
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
