use bytes::Bytes;
use foxglove::websocket::Client;

static APOLLO_LUNAR_MODULE_URDF: &[u8] = include_bytes!("assets/urdf/apollo-lunar-module.urdf");
static APOLLO_LUNAR_MODULE_STL: &[u8] = include_bytes!("assets/meshes/apollo-lunar-module.stl");

pub fn fetch_asset(_client: Client, url: String) -> anyhow::Result<Bytes> {
    println!("fetch asset: {url}");
    match url.as_str() {
        "package://meshes/apollo-lunar-module.stl" => {
            Ok(Bytes::from_static(APOLLO_LUNAR_MODULE_STL))
        }
        "package://urdf/apollo-lunar-module.urdf" => {
            Ok(Bytes::from_static(APOLLO_LUNAR_MODULE_URDF))
        }
        _ => Err(anyhow::anyhow!("not found")),
    }
}
