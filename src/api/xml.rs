use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Region {
    pub stations: Vec<Stations>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Stations {
    pub station: Vec<Station>,
    pub region_id: String,
    pub region_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Station {
    pub id: String,
    pub name: String,
    pub area_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaylistUrl {
    pub url: Vec<Urls>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Urls {
    #[serde(rename = "$value")]
    pub value: String,
}

/// current prog
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CurrentProg {
    pub stations: PStations,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PStations {
    pub station: PStation,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PStation {
    pub progs: Progs,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Progs {
    pub prog: Vec<Prog>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prog {
    pub ft: String,
    pub to: String,
    pub title: String,
    pub info: String,
}
