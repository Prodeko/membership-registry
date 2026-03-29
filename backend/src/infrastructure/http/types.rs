use serde::Deserialize;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Deserialize, Debug, TS)]
#[ts(export)]
pub struct ApplicationPath {
    pub application_id: Uuid,
}

#[derive(Deserialize, Debug, TS)]
#[ts(export)]
pub struct RolePath {
    pub id: String,
}
