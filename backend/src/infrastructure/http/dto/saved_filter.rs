use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;
use uuid::Uuid;

use crate::application::ports::saved_filter_repository_port;

#[derive(Debug, Serialize, TS)]
#[ts(export, rename = "SavedFilter")]
pub struct SavedFilterDTO {
    pub name: String,
    pub filtered_model: String,
    pub owner_user_id: Uuid,
    pub visible_for_all: Option<bool>,
    pub search: Option<String>,
    pub sorting_col: Option<String>,
    pub sorting_desc: bool,
    #[ts(type = "Record<string, unknown> | null")]
    pub custom_filters: Option<Value>,
}

impl From<saved_filter_repository_port::SavedFilter> for SavedFilterDTO {
    fn from(f: saved_filter_repository_port::SavedFilter) -> Self {
        Self {
            name: f.name,
            filtered_model: f.filtered_model,
            owner_user_id: f.owner_user_id,
            visible_for_all: f.visible_for_all,
            search: f.search,
            sorting_col: f.sorting_col,
            sorting_desc: f.sorting_desc,
            custom_filters: f.custom_filters,
        }
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, rename = "NewSavedFilter")]
pub struct NewSavedFilterDTO {
    pub name: String,
    pub filtered_model: String,
    pub visible_for_all: bool,
    pub search: Option<String>,
    pub sorting_col: Option<String>,
    pub sorting_desc: bool,
    #[ts(type = "Record<string, unknown> | null")]
    pub custom_filters: Option<Value>,
}

impl From<NewSavedFilterDTO> for saved_filter_repository_port::NewSavedFilter {
    fn from(f: NewSavedFilterDTO) -> Self {
        Self {
            name: f.name,
            filtered_model: f.filtered_model,
            visible_for_all: f.visible_for_all,
            search: f.search,
            sorting_col: f.sorting_col,
            sorting_desc: f.sorting_desc,
            custom_filters: f.custom_filters,
        }
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, rename = "GetSavedFilterParams")]
pub struct GetSavedFilterParamsDTO {
    pub model: Option<String>,
}
