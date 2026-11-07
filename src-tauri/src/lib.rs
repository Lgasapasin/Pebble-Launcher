mod auth;
mod minecraft;
mod instances;
mod models;
mod net;
mod storage;
mod java;
mod loaders;
mod mods;

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
            loaders::list_loader_versions,
            loaders::configure_instance_loader,
            mods::find_mods,
            mods::install_modrinth_mod,
            mods::list_local_mods,
            mods::toggle_local_mod,
            mods::remove_local_mod,
        ])
        .run(tauri::generate_context!())
        .expect("Launcher startup failed");
}
