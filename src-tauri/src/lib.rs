pub mod ai_agent;
pub mod compiler;
pub mod mcp;
pub mod synctex;
pub mod workspace;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            workspace::init_default_workspace,
            workspace::open_workspace,
            workspace::create_new_project,
            workspace::list_workspace_files,
            workspace::read_text_file,
            workspace::save_text_file,
            workspace::create_file,
            workspace::rename_path,
            workspace::delete_path,
            workspace::export_pdf,
            compiler::detect_compilers,
            compiler::compile_latex,
            compiler::get_compile_cache_info,
            compiler::clear_compile_cache,
            synctex::synctex_forward,
            synctex::synctex_inverse,
            mcp::mcp_list_tools,
            mcp::mcp_jsonrpc,
            ai_agent::ai_get_env_status,
            ai_agent::ai_list_models,
            ai_agent::ai_send_message,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
