use std::sync::Arc;

use bytes::Buf;
use foxglove::websocket::{Parameter, ParameterValue};
use parking_lot::RwLock;

use crate::landscape::{
    DEFAULT_LANDER_START_ALTITUDE, DEFAULT_LANDING_ZONE_MAX_DISTANCE, DEFAULT_LANDING_ZONE_RADIUS,
    DEFAULT_LANDSCAPE_WIDTH,
};

#[derive(Default, Clone)]
pub struct Parameters(Arc<RwLock<Inner>>);

impl Parameters {
    pub fn next_seed(&self) -> u64 {
        self.0.write().maybe_regenerate_seed()
    }

    pub fn landscape_width(&self) -> u32 {
        self.0.read().landscape_width
    }

    pub fn landing_zone_max_distance(&self) -> u32 {
        self.0.read().landing_zone_max_distance
    }

    pub fn landing_zone_radius(&self) -> u32 {
        self.0.read().landing_zone_radius
    }

    pub fn lander_start_altitude(&self) -> u32 {
        self.0.read().lander_start_altitude
    }

    pub fn get(&self, names: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<Parameter> {
        self.0.read().get(names)
    }

    pub fn set(&self, params: Vec<Parameter>) -> Vec<Parameter> {
        self.0.write().set(params)
    }
}

struct Inner {
    /// Random seed.
    seed: u64,
    /// Regenerate random seed on game reset.
    regenerate_seed: bool,
    /// Landscape width.
    landscape_width: u32,
    /// Max landing zone distance from landscape center.
    landing_zone_max_distance: u32,
    /// Landing zone radius.
    landing_zone_radius: u32,
    /// Lander altitude.
    lander_start_altitude: u32,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            seed: 0,
            regenerate_seed: true,
            landscape_width: DEFAULT_LANDSCAPE_WIDTH,
            landing_zone_max_distance: DEFAULT_LANDING_ZONE_MAX_DISTANCE,
            landing_zone_radius: DEFAULT_LANDING_ZONE_RADIUS,
            lander_start_altitude: DEFAULT_LANDER_START_ALTITUDE,
        }
    }
}

impl Inner {
    fn maybe_regenerate_seed(&mut self) -> u64 {
        if self.regenerate_seed {
            self.seed = rand::random();
        }
        self.seed
    }

    fn get(&self, names: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<Parameter> {
        let names: Vec<_> = names.into_iter().collect();
        if names.is_empty() {
            return self.get([
                "seed",
                "regenerate_seed",
                "landscape_width",
                "landing_zone_max_distance",
                "landing_zone_radius",
                "lander_start_altitude",
            ]);
        }
        names
            .into_iter()
            .filter_map(|n| match n.as_ref() {
                "seed" => Some(Parameter {
                    name: "seed".into(),
                    r#type: None,
                    value: Some(ParameterValue::String(self.seed.to_le_bytes().to_vec())),
                }),
                "regenerate_seed" => Some(Parameter {
                    name: "regenerate_seed".into(),
                    r#type: None,
                    value: Some(ParameterValue::Bool(self.regenerate_seed)),
                }),
                "landscape_width" => Some(Parameter {
                    name: "landscape_width".into(),
                    r#type: None,
                    value: Some(ParameterValue::Number(self.landscape_width.into())),
                }),
                "landing_zone_max_distance" => Some(Parameter {
                    name: "landing_zone_max_distance".into(),
                    r#type: None,
                    value: Some(ParameterValue::Number(
                        self.landing_zone_max_distance.into(),
                    )),
                }),
                "landing_zone_radius" => Some(Parameter {
                    name: "landing_zone_radius".into(),
                    r#type: None,
                    value: Some(ParameterValue::Number(self.landing_zone_radius.into())),
                }),
                "lander_start_altitude" => Some(Parameter {
                    name: "lander_start_altitude".into(),
                    r#type: None,
                    value: Some(ParameterValue::Number(self.lander_start_altitude.into())),
                }),
                _ => None,
            })
            .collect()
    }

    fn set(&mut self, params: Vec<Parameter>) -> Vec<Parameter> {
        params
            .into_iter()
            .filter_map(|n| match (n.name.as_str(), &n.value) {
                ("seed", Some(ParameterValue::String(v))) if v.len() == 8 => {
                    self.seed = v.as_slice().get_u64_le();
                    Some(n)
                }
                ("regenerate_seed", Some(ParameterValue::Bool(v))) => {
                    self.regenerate_seed = *v;
                    Some(n)
                }
                ("landscape_width", Some(ParameterValue::Number(v))) if *v > 0.0 && *v < 1000.0 => {
                    self.landscape_width = *v as u32;
                    Some(n)
                }
                ("landing_zone_max_distance", Some(ParameterValue::Number(v)))
                    if *v > 0.0 && *v < 1000.0 =>
                {
                    self.landing_zone_max_distance = *v as u32;
                    Some(n)
                }
                ("landing_zone_radius", Some(ParameterValue::Number(v)))
                    if *v > 0.0 && *v < 1000.0 =>
                {
                    self.landing_zone_radius = *v as u32;
                    Some(n)
                }
                ("lander_start_altitude", Some(ParameterValue::Number(v)))
                    if *v > 0.0 && *v < 1000.0 =>
                {
                    self.lander_start_altitude = *v as u32;
                    Some(n)
                }
                _ => None,
            })
            .collect()
    }
}
