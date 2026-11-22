fn main() {
    println!("cargo:rerun-if-changed=../sprites");
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "set_hit_rect",
            "popup_finished",
            "get_settings_view",
            "add_message",
            "update_message",
            "delete_message",
            "save_settings",
            "set_launch_at_login",
            "show_test_popup",
            "open_data_folder",
            "open_sprites_folder",
            "open_pack_guide",
            "reload_files",
            "finish_onboarding",
            "check_for_updates",
            "open_release_page",
        ]),
    ))
    .expect("failed to run tauri-build");
}
