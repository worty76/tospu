use leptos::prelude::*;
use serde::Serialize;
use tospu_model::{OnboardingInput, MAX_TEXT_CHARS};
use wasm_bindgen_futures::spawn_local;

use crate::ipc;
use crate::settings::character::PackGrid;
use crate::settings::store::Store;

#[derive(Serialize)]
struct FinishOnboardingArgs {
    input: OnboardingInput,
}

#[component]
pub fn WelcomeView() -> impl IntoView {
    let store = use_context::<Store>().expect("Store provided");

    let initial_pack = store
        .view
        .get_untracked()
        .map(|v| v.settings.active_pack)
        .unwrap_or_else(|| "mochi".to_string());

    let goal = RwSignal::new(String::new());
    let gratitude = RwSignal::new(String::new());
    let proud = RwSignal::new(String::new());
    let selected_pack = RwSignal::new(initial_pack);
    let launch_at_login = RwSignal::new(true);
    let submitting = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let packs = Signal::derive(move || store.view.get().map(|v| v.packs).unwrap_or_default());
    let active_pack = Signal::derive(move || selected_pack.get());

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        if submitting.get_untracked() {
            return;
        }
        submitting.set(true);
        error.set(None);

        let opt = |s: String| {
            let trimmed = s.trim().to_string();
            (!trimmed.is_empty()).then_some(trimmed)
        };
        let input = OnboardingInput {
            goal: opt(goal.get_untracked()),
            gratitude: opt(gratitude.get_untracked()),
            proud: opt(proud.get_untracked()),
            active_pack: selected_pack.get_untracked(),
            launch_at_login: launch_at_login.get_untracked(),
        };

        spawn_local(async move {
            match ipc::invoke::<_, ()>("finish_onboarding", &FinishOnboardingArgs { input }).await {
                Ok(()) => {
                    store.refresh();
                }
                Err(e) => {
                    error.set(Some(e));
                    submitting.set(false);
                }
            }
        });
    };

    view! {
        <div class="welcome-shell">
            <form class="welcome-card" on:submit=on_submit>
                <header class="welcome-header">
                    <h1>"Hi, I'm Tospu."</h1>
                    <p class="welcome-sub">
                        "I'll pop up a few times a day with your big goals and a little encouragement, then get out of your way."
                    </p>
                </header>

                {move || {
                    error.get().map(|err| {
                        view! {
                            <div class="error-banner" role="alert">
                                <span>{err}</span>
                            </div>
                        }
                    })
                }}

                <div class="welcome-fields">
                    <label class="welcome-field">
                        <span>"One big goal"</span>
                        <input
                            type="text"
                            class="field-input"
                            maxlength=MAX_TEXT_CHARS
                            placeholder="Optional"
                            prop:value=move || goal.get()
                            on:input=move |ev| goal.set(event_target_value(&ev))
                        />
                    </label>

                    <label class="welcome-field">
                        <span>"Something you're grateful for"</span>
                        <input
                            type="text"
                            class="field-input"
                            maxlength=MAX_TEXT_CHARS
                            placeholder="Optional"
                            prop:value=move || gratitude.get()
                            on:input=move |ev| gratitude.set(event_target_value(&ev))
                        />
                    </label>

                    <label class="welcome-field">
                        <span>"Something you're proud of"</span>
                        <input
                            type="text"
                            class="field-input"
                            maxlength=MAX_TEXT_CHARS
                            placeholder="Optional"
                            prop:value=move || proud.get()
                            on:input=move |ev| proud.set(event_target_value(&ev))
                        />
                    </label>
                </div>

                <div class="welcome-section">
                    <h2>"Choose your character"</h2>
                    <PackGrid
                        packs=packs
                        active_pack=active_pack
                        on_select=move |id| selected_pack.set(id)
                    />
                </div>

                <label class="welcome-checkbox">
                    <input
                        type="checkbox"
                        prop:checked=move || launch_at_login.get()
                        on:change=move |ev| launch_at_login.set(event_target_checked(&ev))
                    />
                    <span>"Start Tospu when I log in"</span>
                </label>

                <div class="welcome-actions">
                    <button type="submit" class="btn primary" disabled=move || submitting.get()>
                        "Start"
                    </button>
                </div>
            </form>
        </div>
    }
}
