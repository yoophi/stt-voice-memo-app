mod inbound;
mod infrastructure;
mod transcription_state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_recorder::init())
        .setup(|app| {
            let state = transcription_state::TranscriptionState::build(app.handle())
                .map_err(|error| error.to_string())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            inbound::transcription_commands::transcription_submit,
            inbound::transcription_commands::transcription_status,
            inbound::transcription_commands::transcription_retry,
            inbound::transcription_commands::transcription_cancel,
            inbound::transcription_commands::transcription_recover,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
