use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelVisibility;

use crate::model_info;

/// Slug, display name, description, picker priority, and context window per gateway model.
///
/// Priorities follow the bundled catalog (which ends at 43) so gateway models list last
/// and never displace a backend model as the default.
const GATEWAY_MODELS: [(&str, &str, &str, i32, i64); 3] = [
    (
        "openrouter/mimo-v2.6-pro",
        "MiMo 2.6 Pro",
        "Xiaomi MiMo 2.6 Pro via OpenRouter.",
        44,
        1_050_000,
    ),
    (
        "openrouter/space-bunny-alpha",
        "Space Bunny Alpha",
        "Stealth Space Bunny Alpha via OpenRouter.",
        45,
        1_000_000,
    ),
    (
        "openpaths/openpaths-free",
        "OpenPaths Free",
        "OpenPaths free router via openpaths.io.",
        46,
        1_000_000,
    ),
];

/// Catalog entries for models this fork serves from third-party gateways.
///
/// The backend `/models` response stays authoritative for the picker, so these entries are
/// merged into the in-memory catalog after it is applied. Slugs keep their `openrouter/` or
/// `openpaths/` prefix so provider inference picks the matching built-in provider, and
/// `model_info::wire_model` maps each slug onto the id that provider serves.
pub(crate) fn gateway_models() -> Vec<ModelInfo> {
    GATEWAY_MODELS
        .iter()
        .copied()
        .map(
            |(slug, display_name, description, priority, context_window)| ModelInfo {
                slug: slug.to_string(),
                display_name: display_name.to_string(),
                description: Some(description.to_string()),
                visibility: ModelVisibility::List,
                priority,
                context_window: Some(context_window),
                max_context_window: Some(context_window),
                ..model_info::fallback_model_info(slug)
            },
        )
        .collect()
}

/// Append every gateway model that `models` does not already define.
///
/// Catalog entries win on a slug collision so a catalog that starts serving one of these
/// models keeps its own metadata.
pub(crate) fn merge_gateway_models(mut models: Vec<ModelInfo>) -> Vec<ModelInfo> {
    for model in gateway_models() {
        if !models.iter().any(|existing| existing.slug == model.slug) {
            models.push(model);
        }
    }
    models
}

#[cfg(test)]
#[path = "gateway_models_tests.rs"]
mod tests;
