use std::{collections::HashMap, sync::LazyLock};

use bytes::Bytes;
use foxglove::websocket::Client;

macro_rules! load_bytes {
    ($path:literal) => {
        ($path, include_bytes!($path))
    };
}

static ASSETS: &[(&str, &[u8])] = &[
    load_bytes!("assets/dae/apollo.dae"),
    load_bytes!("assets/dae/BOOSTER3.png"),
    load_bytes!("assets/dae/TEXTUREA.png"),
    load_bytes!("assets/dae/TEXTURE_.png"),
    load_bytes!("assets/stl/apollo.stl"),
];

static ASSET_MAP: LazyLock<HashMap<String, Bytes>> = LazyLock::new(|| {
    ASSETS
        .iter()
        .map(|(path, data)| {
            (
                path.replace("assets/", "package://").to_string(),
                Bytes::from_static(data),
            )
        })
        .collect()
});

pub fn fetch_asset(_client: Client, url: String) -> anyhow::Result<Bytes> {
    println!("fetch asset: {url}");
    ASSET_MAP.get(&url).cloned().ok_or_else(|| {
        eprintln!("asset not found: {url}");
        anyhow::anyhow!("not found")
    })
}
