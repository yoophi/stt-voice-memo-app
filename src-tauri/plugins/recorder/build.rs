const COMMANDS: &[&str] = &[
    "permission_status",
    "request_permission",
    "recorder_status",
    "start",
    "pause",
    "resume",
    "stop",
    "cancel",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).ios_path("ios").build();
}
