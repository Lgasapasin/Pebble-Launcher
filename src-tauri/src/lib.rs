mod auth;
mod minecraft;
mod instances;
mod models;
mod net;
mod storage;

pub fn run() {
    tauri::Builder::default()
        .manage(auth::AuthState::default())
        .manage(minecraft::MinecraftState::default())
        .invoke_handler(tauri::generate_handler![
            minecraft::list_versions,
            minecraft::list_installed,
            minecraft::is_installed,
            minecraft::install_version,
            minecraft::launch_game,
            auth::list_accounts,
            auth::select_account,
            auth::remove_account,
            auth::add_offline_account,
            auth::import_session,
            auth::validate_account,
            auth::start_login,
            auth::complete_login,
            instances::list_instances,
            instances::create_instance,
            instances::remove_instance,
            instances::list_instance_mods,
            instances::list_instance_worlds,
            instances::open_instance_folder,
        ])
        .run(tauri::generate_context!())
        .expect("Launcher startup failed");
}
