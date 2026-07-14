use rmcp::model::{Icon, IconTheme, Meta};
use serde_json::{json, Map, Value};

pub(super) const META_NAMESPACE: &str = "ai.dinglebear/tailscale-rmcp";
const ICON_SRC: &str = "https://avatars.githubusercontent.com/u/38927646?v=4";

pub(super) fn icons() -> Vec<Icon> {
    vec![Icon::new(ICON_SRC)
        .with_mime_type("image/png")
        .with_sizes(vec!["460x460".to_string()])
        .with_theme(IconTheme::Dark)]
}

pub(super) fn icons_json() -> Vec<Value> {
    vec![json!({
        "src": ICON_SRC,
        "mimeType": "image/png",
        "sizes": ["460x460"],
        "theme": "dark"
    })]
}

pub(super) fn meta(surface: &str, detail: Value) -> Meta {
    Meta(meta_object(surface, detail))
}

pub(super) fn meta_json(surface: &str, detail: Value) -> Value {
    Value::Object(meta_object(surface, detail))
}

fn meta_object(surface: &str, detail: Value) -> Map<String, Value> {
    let mut object = Map::new();
    object.insert(
        META_NAMESPACE.to_string(),
        json!({
            "name": META_NAMESPACE,
            "surface": surface,
            "version": env!("CARGO_PKG_VERSION"),
            "repository": env!("CARGO_PKG_REPOSITORY"),
            "detail": detail
        }),
    );
    object
}
