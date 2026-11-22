use leptos::prelude::*;

#[component]
pub fn Bubble<F>(
    visible: ReadSignal<bool>,
    text: ReadSignal<String>,
    author: ReadSignal<Option<String>>,
    on_click: F,
    node_ref: NodeRef<leptos::html::Div>,
) -> impl IntoView
where
    F: Fn() + 'static,
{
    let aria_label = move || match author.get() {
        Some(a) if !a.is_empty() => format!("{} — {a}", text.get()),
        _ => text.get(),
    };

    view! {
        <div
            node_ref=node_ref
            class=move || if visible.get() { "bubble visible" } else { "bubble" }
            role="status"
            aria-label=aria_label
            on:click=move |_| on_click()
        >
            <p class="bubble-text">{move || text.get()}</p>
            <p class="bubble-author">
                {move || author.get().map(|a| format!("— {a}")).unwrap_or_default()}
            </p>
        </div>
    }
}
