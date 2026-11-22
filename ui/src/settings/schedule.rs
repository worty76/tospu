use leptos::prelude::*;
use serde::Serialize;
use tospu_model::{HhMm, MessageKind, QuietHours, ScheduleMode, Settings, Slot};
use wasm_bindgen_futures::spawn_local;

use crate::ipc;
use crate::settings::store::Store;

#[derive(Serialize)]
struct SaveSettingsArgs {
    settings: Settings,
}

fn kind_value(kind: MessageKind) -> &'static str {
    match kind {
        MessageKind::Goal => "goal",
        MessageKind::Quote => "quote",
        MessageKind::Gratitude => "gratitude",
        MessageKind::Proud => "proud",
    }
}

fn parse_kind(s: &str) -> MessageKind {
    match s {
        "goal" => MessageKind::Goal,
        "quote" => MessageKind::Quote,
        "gratitude" => MessageKind::Gratitude,
        "proud" => MessageKind::Proud,
        _ => MessageKind::Goal,
    }
}

fn optional_kind_value(kind: Option<MessageKind>) -> &'static str {
    match kind {
        None => "rotate",
        Some(k) => kind_value(k),
    }
}

fn parse_optional_kind(s: &str) -> Option<MessageKind> {
    match s {
        "rotate" => None,
        other => Some(parse_kind(other)),
    }
}

fn next_free_hour(slots: &[Slot]) -> HhMm {
    for h in (9..24).chain(0..9) {
        let candidate = HhMm::new(h, 0).expect("valid hour");
        if !slots.iter().any(|s| s.time == candidate) {
            return candidate;
        }
    }
    HhMm::new(12, 0).expect("valid hour")
}

#[component]
pub fn ScheduleTab() -> impl IntoView {
    let store = use_context::<Store>().expect("Store provided");
    let field_errors = RwSignal::new(Vec::<String>::new());

    let save = move |mutate: Box<dyn FnOnce(&mut Settings)>| {
        let Some(mut next) = store.view.get_untracked().map(|v| v.settings) else {
            return;
        };
        mutate(&mut next);
        spawn_local(async move {
            match ipc::invoke_vec_err::<_, Settings>(
                "save_settings",
                &SaveSettingsArgs { settings: next },
            )
            .await
            {
                Ok(saved) => {
                    field_errors.set(Vec::new());
                    store.error.set(None);
                    store.view.update(|opt| {
                        if let Some(v) = opt {
                            v.settings = saved;
                        }
                    });
                    store.refresh();
                }
                Err(errs) => {
                    field_errors.set(errs);
                    store.refresh(); // roll back controls to saved values
                }
            }
        });
    };

    let err_for =
        move |field: &'static str| field_errors.get().into_iter().find(|e| e.contains(field));

    let next_popup_line = move || match store.view.get().and_then(|v| v.next_popup) {
        Some(when) => format!("Next popup: {when}"),
        None => "No popups scheduled".to_string(),
    };

    let mode = move || {
        store
            .view
            .get()
            .map(|v| v.settings.schedule_mode)
            .unwrap_or(ScheduleMode::Daily)
    };

    let slots = move || {
        store
            .view
            .get()
            .map(|v| v.settings.slots)
            .unwrap_or_default()
    };
    let indexed_slots = move || slots().into_iter().enumerate().collect::<Vec<_>>();
    let at_max_slots = move || slots().len() >= tospu_model::settings::MAX_SLOTS;

    let interval_minutes = move || {
        store
            .view
            .get()
            .map(|v| v.settings.interval.every_minutes)
            .unwrap_or(30)
    };

    let interval_kind = move || store.view.get().and_then(|v| v.settings.interval.kind);

    let jitter = move || {
        store
            .view
            .get()
            .map(|v| v.settings.jitter_minutes)
            .unwrap_or(10)
    };

    let quiet_hours = move || store.view.get().and_then(|v| v.settings.quiet_hours);

    let display_seconds = move || {
        store
            .view
            .get()
            .map(|v| v.settings.display_seconds)
            .unwrap_or(8)
    };

    view! {
        <section class="tab-panel" aria-label="Schedule">
            <header class="panel-header">
                <h2>"Schedule"</h2>
                <span class="next-popup">{next_popup_line}</span>
            </header>

            <div class="chips" role="group" aria-label="Schedule mode">
                <button
                    type="button"
                    class=move || if mode() == ScheduleMode::Daily { "chip active" } else { "chip" }
                    on:click=move |_| {
                        save(Box::new(|s| s.schedule_mode = ScheduleMode::Daily));
                    }
                >
                    "Daily times"
                </button>
                <button
                    type="button"
                    class=move || if mode() == ScheduleMode::Interval { "chip active" } else { "chip" }
                    on:click=move |_| {
                        save(Box::new(|s| s.schedule_mode = ScheduleMode::Interval));
                    }
                >
                    "Every N minutes"
                </button>
            </div>

            {move || match mode() {
                ScheduleMode::Daily => view! {
                    <div class="card">
                        <div class="card-head">
                            <div>
                                <h3>"Daily times"</h3>
                                <p class="hint">"Up to 12 popups a day, each with its own message kind."</p>
                            </div>
                            <button
                                type="button"
                                class="btn secondary small"
                                disabled=at_max_slots
                                on:click=move |_| {
                                    save(Box::new(|s| {
                                        let time = next_free_hour(&s.slots);
                                        s.slots.push(Slot { time, kind: MessageKind::Goal });
                                    }));
                                }
                            >
                                "Add time"
                            </button>
                        </div>

                        <div class="slot-list">
                            <For
                                each=indexed_slots
                                key=|(idx, slot)| (*idx, slot.time, slot.kind)
                                children=move |(idx, slot)| {
                                    let time_str = slot.time.to_string();
                                    let kind_str = kind_value(slot.kind);
                                    view! {
                                        <div class="slot-row">
                                            <input
                                                type="time"
                                                class="field-input"
                                                aria-label="Popup time"
                                                prop:value=time_str
                                                on:change=move |ev| {
                                                    let val = event_target_value(&ev);
                                                    if let Ok(parsed) = val.parse::<HhMm>() {
                                                        save(Box::new(move |s| {
                                                            if let Some(target) = s.slots.get_mut(idx) {
                                                                target.time = parsed;
                                                            }
                                                        }));
                                                    } else {
                                                        field_errors.set(vec!["slots: invalid time".into()]);
                                                        store.refresh();
                                                    }
                                                }
                                            />
                                            <select
                                                class="field-input"
                                                aria-label="Message kind"
                                                prop:value=kind_str
                                                on:change=move |ev| {
                                                    let k = parse_kind(&event_target_value(&ev));
                                                    save(Box::new(move |s| {
                                                        if let Some(target) = s.slots.get_mut(idx) {
                                                            target.kind = k;
                                                        }
                                                    }));
                                                }
                                            >
                                                <option value="goal">"Goal"</option>
                                                <option value="quote">"Quote"</option>
                                                <option value="gratitude">"Gratitude"</option>
                                                <option value="proud">"Pride"</option>
                                            </select>
                                            <button
                                                type="button"
                                                class="btn ghost danger small"
                                                on:click=move |_| {
                                                    save(Box::new(move |s| {
                                                        if idx < s.slots.len() {
                                                            s.slots.remove(idx);
                                                        }
                                                    }));
                                                }
                                            >
                                                "Remove"
                                            </button>
                                        </div>
                                    }
                                }
                            />
                        </div>
                        {move || err_for("slots").map(|e| view! { <p class="inline-error">{e}</p> })}
                    </div>

                    <div class="card">
                        <div class="control-row">
                            <div>
                                <h3>"Random shift"</h3>
                                <p class="hint">"Shifts each popup slightly so it feels natural."</p>
                            </div>
                            <div class="range-group">
                                <input
                                    type="range"
                                    min="0"
                                    max="30"
                                    aria-label="Random shift in minutes"
                                    prop:value=move || jitter().to_string()
                                    on:change=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse::<u32>() {
                                            save(Box::new(move |s| s.jitter_minutes = v));
                                        }
                                    }
                                />
                                <span class="range-value">{move || format!("±{} min", jitter())}</span>
                            </div>
                        </div>
                        {move || err_for("jitterMinutes").map(|e| view! { <p class="inline-error">{e}</p> })}
                    </div>
                }
                .into_any(),
                ScheduleMode::Interval => view! {
                    <div class="card">
                        <div class="control-row">
                            <div>
                                <h3>"Repeat every"</h3>
                                <p class="hint">"Respects quiet hours. Random shift is off in interval mode."</p>
                            </div>
                            <div class="number-group">
                                <input
                                    type="number"
                                    min="1"
                                    max="1440"
                                    class="field-input small-num"
                                    aria-label="Interval minutes"
                                    prop:value=move || interval_minutes().to_string()
                                    on:change=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse::<u32>() {
                                            save(Box::new(move |s| s.interval.every_minutes = v));
                                        } else {
                                            field_errors.set(vec![
                                                "interval: enter a number between 1 and 1440".into(),
                                            ]);
                                            store.refresh();
                                        }
                                    }
                                />
                                <span>"min"</span>
                            </div>
                        </div>

                        <div class="preset-chips">
                            {[1u32, 5, 15, 30, 60]
                                .into_iter()
                                .map(|m| {
                                    view! {
                                        <button
                                            type="button"
                                            class=move || {
                                                if interval_minutes() == m {
                                                    "chip active"
                                                } else {
                                                    "chip"
                                                }
                                            }
                                            on:click=move |_| {
                                                save(Box::new(move |s| s.interval.every_minutes = m));
                                            }
                                        >
                                            {format!("{m}m")}
                                        </button>
                                    }
                                })
                                .collect_view()}
                        </div>

                        <hr class="divider" />

                        <div class="control-row">
                            <div>
                                <h3>"Message kind"</h3>
                                <p class="hint">"Rotate through all four kinds or pick just one."</p>
                            </div>
                            <select
                                class="field-input"
                                aria-label="Interval message kind"
                                prop:value=move || optional_kind_value(interval_kind())
                                on:change=move |ev| {
                                    let k = parse_optional_kind(&event_target_value(&ev));
                                    save(Box::new(move |s| s.interval.kind = k));
                                }
                            >
                                <option value="rotate">"Rotate all"</option>
                                <option value="goal">"Goal"</option>
                                <option value="quote">"Quote"</option>
                                <option value="gratitude">"Gratitude"</option>
                                <option value="proud">"Pride"</option>
                            </select>
                        </div>

                        {move || err_for("interval").map(|e| view! { <p class="inline-error">{e}</p> })}
                    </div>
                }
                .into_any(),
            }}

            <div class="card">
                <div class="control-row">
                    <div>
                        <h3>"Quiet hours"</h3>
                        <p class="hint">"Skip popups during this window."</p>
                    </div>
                    <label class="switch">
                        <input
                            type="checkbox"
                            prop:checked=move || quiet_hours().is_some()
                            on:change=move |ev| {
                                let checked = event_target_checked(&ev);
                                save(Box::new(move |s| {
                                    s.quiet_hours = if checked {
                                        Some(QuietHours {
                                            start: HhMm::new(23, 0).unwrap(),
                                            end: HhMm::new(7, 30).unwrap(),
                                        })
                                    } else {
                                        None
                                    };
                                }));
                            }
                        />
                        <span class="slider"></span>
                    </label>
                </div>

                {move || {
                    quiet_hours().map(|q| {
                        let start_val = q.start.to_string();
                        let end_val = q.end.to_string();
                        view! {
                            <div class="quiet-times">
                                <label>
                                    <span>"From"</span>
                                    <input
                                        type="time"
                                        class="field-input"
                                        prop:value=start_val
                                        on:change=move |ev| {
                                            if let Ok(t) = event_target_value(&ev).parse::<HhMm>() {
                                                save(Box::new(move |s| {
                                                    if let Some(qh) = &mut s.quiet_hours {
                                                        qh.start = t;
                                                    }
                                                }));
                                            }
                                        }
                                    />
                                </label>
                                <label>
                                    <span>"To"</span>
                                    <input
                                        type="time"
                                        class="field-input"
                                        prop:value=end_val
                                        on:change=move |ev| {
                                            if let Ok(t) = event_target_value(&ev).parse::<HhMm>() {
                                                save(Box::new(move |s| {
                                                    if let Some(qh) = &mut s.quiet_hours {
                                                        qh.end = t;
                                                    }
                                                }));
                                            }
                                        }
                                    />
                                </label>
                            </div>
                        }
                    })
                }}
                {move || err_for("quietHours").map(|e| view! { <p class="inline-error">{e}</p> })}
            </div>

            <div class="card">
                <div class="control-row">
                    <div>
                        <h3>"Display time"</h3>
                        <p class="hint">"How long each popup stays on screen (3–60 seconds)."</p>
                    </div>
                    <div class="number-group">
                        <input
                            type="number"
                            min="3"
                            max="60"
                            class="field-input small-num"
                            aria-label="Display seconds"
                            prop:value=move || display_seconds().to_string()
                            on:change=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<u32>() {
                                    save(Box::new(move |s| s.display_seconds = v));
                                } else {
                                    field_errors.set(vec![
                                        "displaySeconds: enter a number between 3 and 60".into(),
                                    ]);
                                    store.refresh();
                                }
                            }
                        />
                        <span>"s"</span>
                    </div>
                </div>
                {move || err_for("displaySeconds").map(|e| view! { <p class="inline-error">{e}</p> })}
            </div>
        </section>
    }
}
