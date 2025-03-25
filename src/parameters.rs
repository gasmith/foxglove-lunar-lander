use std::sync::Arc;

use bytes::Buf;
use foxglove::websocket::{Parameter, ParameterValue};
use parking_lot::RwLock;

#[derive(Default, Clone)]
pub struct Parameters(Arc<RwLock<Inner>>);

impl Parameters {
    pub fn next_seed(&self) -> u64 {
        self.0.write().maybe_regenerate_seed()
    }

    pub fn get(&self, names: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<Parameter> {
        self.0.read().get(names)
    }

    pub fn set(&self, params: Vec<Parameter>) -> Vec<Parameter> {
        self.0.write().set(params)
    }
}

struct Inner {
    seed: u64,
    regenerate_seed: bool,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            seed: 0,
            regenerate_seed: true,
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

                _ => None,
            })
            .collect()
    }
}
