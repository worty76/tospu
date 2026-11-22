use leptos::prelude::*;
use tospu_model::{AnimationName, PackView};

#[component]
pub fn Sprite(
    pack: ReadSignal<Option<PackView>>,
    animation: ReadSignal<AnimationName>,
    frame: ReadSignal<u32>,
) -> impl IntoView {
    let style = move || {
        let Some(p) = pack.get() else {
            return String::new();
        };
        let spec = p.manifest.resolve_animation(animation.get());
        let (w, h) = p.manifest.display_size();
        let url = p.sheet_urls.get(&spec.sheet).cloned().unwrap_or_default();
        let f = frame.get().min(spec.frames.saturating_sub(1)) as f32;
        let pixelated = if p.manifest.pixelated {
            "image-rendering: pixelated;"
        } else {
            ""
        };
        format!(
            "width: {w}px; height: {h}px; background-image: url(\"{url}\"); \
             background-repeat: no-repeat; background-size: {}px {h}px; \
             background-position: -{}px 0px; {pixelated}",
            w * spec.frames as f32,
            f * w,
        )
    };

    view! { <div class="sprite" style=style></div> }
}

/// Paused idle frame 0 preview of a sprite pack, scaled to `height` px.
#[component]
pub fn SpriteStill(pack: PackView, #[prop(default = 72.0)] height: f32) -> impl IntoView {
    let spec = &pack.manifest.animations.idle;
    let (dw, dh) = pack.manifest.display_size();
    let scale = if dh > 0.0 { height / dh } else { 1.0 };
    let w = (dw * scale).round();
    let h = height;
    let url = pack
        .sheet_urls
        .get(&spec.sheet)
        .cloned()
        .unwrap_or_default();
    let pixelated = if pack.manifest.pixelated {
        "image-rendering: pixelated;"
    } else {
        ""
    };
    let style = format!(
        "width: {w}px; height: {h}px; background-image: url(\"{url}\"); \
         background-repeat: no-repeat; background-size: {}px {h}px; \
         background-position: 0px 0px; {pixelated}",
        w * spec.frames as f32,
    );
    view! { <div class="sprite-still" style=style></div> }
}
