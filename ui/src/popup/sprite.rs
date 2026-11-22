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
