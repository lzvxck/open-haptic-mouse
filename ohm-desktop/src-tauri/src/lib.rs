mod sim;

use sim::{ProfileDto, RunResult};

#[tauri::command]
fn list_scenarios() -> Vec<String> {
    sim::list_scenarios()
}

#[tauri::command]
fn default_profile() -> ProfileDto {
    sim::default_profile()
}

#[tauri::command]
fn run_scenario(name: String, profile: ProfileDto) -> Result<RunResult, String> {
    sim::run_scenario(&name, profile)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![list_scenarios, default_profile, run_scenario])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
