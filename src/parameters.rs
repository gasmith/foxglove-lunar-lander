use std::fmt::Debug;
use std::ops::Range;
use std::{collections::HashMap, sync::Arc};

use bytes::Buf;
use foxglove::websocket::{Parameter, ParameterType, ParameterValue};
use parking_lot::RwLock;

static SEED: &str = "seed";
static REGENERATE_SEED: &str = "regenerate_seed";
static LANDSCAPE_WIDTH: &str = "landscape_width";
static LANDING_ZONE_RADIUS: &str = "landing_zone_radius";
static LANDING_ZONE_MIN_DISTANCE: &str = "landing_zone_min_distance";
static LANDING_ZONE_MAX_DISTANCE: &str = "landing_zone_max_distance";
static INIT_ALTITUDE: &str = "init_altitude";
static INIT_VERTICAL_VELOCITY: &str = "init_vertical_velocity";
static INIT_VERTICAL_VELOCITY_TARGET: &str = "init_vertical_velocity_target";

fn default_values() -> HashMap<String, Value> {
    let params = [
        (SEED, "Random seed", Data::Seed(0), None),
        (
            REGENERATE_SEED,
            "Regenerate seed on game reset",
            Data::Bool(true),
            None,
        ),
        (
            LANDSCAPE_WIDTH,
            "Width of the landscape, which is always square",
            Data::F32(200.0),
            Some(ClampRange(100.0..1000.0).boxed()),
        ),
        (
            LANDING_ZONE_MIN_DISTANCE,
            "Minimum distance between landscape center and landing zone",
            Data::F32(30.0),
            Some(
                ClampFn(|reg, data| {
                    let max = reg.get_f32(LANDSCAPE_WIDTH).unwrap() / 2.0;
                    ClampRange(0.0..max).clamp(reg, data)
                })
                .boxed(),
            ),
        ),
        (
            LANDING_ZONE_MAX_DISTANCE,
            "Maximum distance between landscape center and landing zone",
            Data::F32(70.0),
            Some(
                ClampFn(|reg, data| {
                    let max = reg.get_f32(LANDSCAPE_WIDTH).unwrap() / 2.0;
                    ClampRange(0.0..max).clamp(reg, data)
                })
                .boxed(),
            ),
        ),
        (
            LANDING_ZONE_RADIUS,
            "Landing zone radius",
            Data::F32(20.0),
            Some(ClampRange(5.0..50.0).boxed()),
        ),
        (
            INIT_ALTITUDE,
            "Initial lander altitude",
            Data::F32(200.0),
            Some(ClampRange(100.0..1000.0).boxed()),
        ),
        (
            INIT_VERTICAL_VELOCITY,
            "Initial lander vertical velocity",
            Data::F32(0.0),
            Some(ClampRange(-20.0..0.0).boxed()),
        ),
        (
            INIT_VERTICAL_VELOCITY_TARGET,
            "Initial lander vertical velocity_target",
            Data::F32(-6.0),
            Some(ClampRange(-20.0..0.0).boxed()),
        ),
    ];
    params
        .into_iter()
        .map(|(name, descr, default, bounds)| {
            (name.to_string(), Value::new(name, descr, default, bounds))
        })
        .collect()
}

#[derive(Default, Clone)]
pub struct Parameters(Arc<RwLock<Registry>>);

impl Parameters {
    pub fn next_seed(&self) -> u64 {
        let mut registry = self.0.write();
        if registry.get_bool(REGENERATE_SEED).unwrap() {
            let seed = rand::random();
            registry.set_seed(SEED, seed);
            seed
        } else {
            registry.get_seed(SEED).unwrap()
        }
    }

    pub fn landscape_width(&self) -> u32 {
        self.0.read().get_f32(LANDSCAPE_WIDTH).unwrap() as u32
    }

    pub fn landing_zone_min_distance(&self) -> f32 {
        self.0.read().get_f32(LANDING_ZONE_MIN_DISTANCE).unwrap()
    }

    pub fn landing_zone_max_distance(&self) -> f32 {
        self.0.read().get_f32(LANDING_ZONE_MAX_DISTANCE).unwrap()
    }

    pub fn landing_zone_radius(&self) -> u32 {
        self.0.read().get_f32(LANDING_ZONE_RADIUS).unwrap() as u32
    }

    pub fn lander_init_altitude(&self) -> f32 {
        self.0.read().get_f32(INIT_ALTITUDE).unwrap()
    }

    pub fn lander_init_vertical_velocity(&self) -> f32 {
        self.0.read().get_f32(INIT_VERTICAL_VELOCITY).unwrap()
    }

    pub fn lander_init_vertical_velocity_target(&self) -> f32 {
        self.0
            .read()
            .get_f32(INIT_VERTICAL_VELOCITY_TARGET)
            .unwrap()
    }

    pub fn get(&self, names: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<Parameter> {
        self.0.read().get_parameters(names)
    }

    pub fn set(&self, params: Vec<Parameter>) -> Vec<Parameter> {
        self.0.write().set_parameters(params)
    }
}

#[derive(Debug, Clone, Copy)]
enum Data {
    Seed(u64),
    Bool(bool),
    F32(f32),
}
impl Data {
    fn parameter_type(&self) -> Option<ParameterType> {
        None
    }

    fn as_parameter_value(&self) -> Option<ParameterValue> {
        match self {
            Data::Seed(v) => Some(ParameterValue::String(v.to_le_bytes().to_vec())),
            Data::Bool(v) => Some(ParameterValue::Bool(*v)),
            Data::F32(v) => Some(ParameterValue::Number((*v).into())),
        }
    }
}

trait Clamp: Send + Sync + 'static {
    fn clamp(&self, registry: &Registry, data: &Data) -> Option<Data>;
}

struct ClampFn<F>(F)
where
    F: Fn(&Registry, &Data) -> Option<Data>;

impl<F> ClampFn<F>
where
    F: Fn(&Registry, &Data) -> Option<Data> + Send + Sync + 'static,
{
    fn boxed(self) -> Box<dyn Clamp> {
        Box::new(self)
    }
}

impl<F> Clamp for ClampFn<F>
where
    F: Fn(&Registry, &Data) -> Option<Data> + Send + Sync + 'static,
{
    fn clamp(&self, registry: &Registry, data: &Data) -> Option<Data> {
        self.0(registry, data)
    }
}

struct ClampRange<T>(Range<T>);
impl<T> ClampRange<T>
where
    ClampRange<T>: Clamp,
{
    fn boxed(self) -> Box<dyn Clamp> {
        Box::new(self)
    }
}
impl Clamp for ClampRange<f32> {
    fn clamp(&self, _: &Registry, data: &Data) -> Option<Data> {
        match data {
            Data::F32(val) => Some(Data::F32(val.clamp(self.0.start, self.0.end))),
            _ => None,
        }
    }
}

struct Value {
    name: &'static str,
    descr: &'static str,
    current: Data,
    default: Data,
    clamp: Option<Box<dyn Clamp>>,
}
impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Value")
            .field("name", &self.name)
            .field("descr", &self.descr)
            .field("current", &self.current)
            .field("default", &self.default)
            .finish_non_exhaustive()
    }
}
impl From<&Value> for Parameter {
    fn from(value: &Value) -> Self {
        Self {
            name: value.name.to_string(),
            r#type: value.current.parameter_type(),
            value: value.current.as_parameter_value(),
        }
    }
}
impl Value {
    fn new(
        name: &'static str,
        descr: &'static str,
        default: Data,
        clamp: Option<Box<dyn Clamp>>,
    ) -> Self {
        Self {
            name,
            descr,
            current: default,
            default,
            clamp,
        }
    }

    fn as_parameter(&self) -> Parameter {
        Parameter {
            name: self.name.to_string(),
            r#type: self.current.parameter_type(),
            value: self.current.as_parameter_value(),
        }
    }

    fn get_update_from_parameter(
        &self,
        registry: &Registry,
        parameter: &Parameter,
    ) -> Option<Data> {
        let updated = match (&self.current, &parameter.value) {
            (_, None) => self.default,
            (Data::Seed(_), Some(ParameterValue::String(v))) if v.len() == 8 => {
                Data::Seed(v.as_slice().get_u64_le())
            }
            (Data::Bool(_), Some(ParameterValue::Bool(v))) => Data::Bool(*v),
            (Data::F32(_), Some(ParameterValue::Number(v))) => Data::F32(*v as f32),
            _ => return None,
        };
        let clamped = self
            .clamp
            .as_ref()
            .and_then(|c| c.clamp(registry, &updated))
            .unwrap_or(updated);
        Some(clamped)
    }

    fn update(&mut self, data: Data) {
        match (&mut self.current, data) {
            (Data::Seed(p), Data::Seed(v)) => *p = v,
            (Data::Bool(p), Data::Bool(v)) => *p = v,
            (Data::F32(p), Data::F32(v)) => *p = v,
            _ => panic!("cannot change data type"),
        }
    }
}

struct Registry(HashMap<String, Value>);
impl Default for Registry {
    fn default() -> Self {
        Self(default_values())
    }
}

impl Registry {
    fn get_data(&self, key: &str) -> Option<Data> {
        self.0.get(key).map(|v| v.current)
    }

    fn get_seed(&self, key: &str) -> Option<u64> {
        self.get_data(key).and_then(|d| match d {
            Data::Seed(v) => Some(v),
            _ => None,
        })
    }

    fn get_bool(&self, key: &str) -> Option<bool> {
        self.get_data(key).and_then(|d| match d {
            Data::Bool(v) => Some(v),
            _ => None,
        })
    }

    fn get_f32(&self, key: &str) -> Option<f32> {
        self.get_data(key).and_then(|d| match d {
            Data::F32(v) => Some(v),
            _ => None,
        })
    }

    fn set_seed(&mut self, key: &str, seed: u64) {
        if let Some(value) = self.0.get_mut(key) {
            if let Data::Seed(v) = &mut value.current {
                *v = seed;
            }
        }
    }

    /// Retruns a vec of websocket parameters.
    fn get_parameters(&self, keys: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<Parameter> {
        let keys: Vec<_> = keys.into_iter().collect();
        if keys.is_empty() {
            assert!(!self.0.is_empty());
            return self.get_parameters(self.0.keys());
        }
        keys.into_iter()
            .filter_map(|key| self.0.get(key.as_ref()).map(Parameter::from))
            .collect()
    }

    fn set_parameters(&mut self, params: Vec<Parameter>) -> Vec<Parameter> {
        let mut updates = Vec::with_capacity(params.len());
        for param in params {
            if let Some(value) = self.0.get(&param.name) {
                if let Some(data) = value.get_update_from_parameter(self, &param) {
                    updates.push((param.name, data));
                }
            }
        }
        updates
            .into_iter()
            .map(|(name, data)| {
                let value = self.0.get_mut(&name).unwrap();
                value.update(data);
                value.as_parameter()
            })
            .collect()
    }
}
