use tauri::{AppHandle, Emitter, EventTarget};

/// GUI 打开单文件：返回 (JSON, warning)，前端 invoke 契约与重构前完全一致
#[tauri::command]
pub fn open_one_file(path: String) -> Result<(String, Option<String>), String> {
    match ledata_core::parse_file(std::path::Path::new(&path)) {
        Ok((chip_data, warning)) => Ok((serde_json::to_string(&chip_data).unwrap(), warning)),
        Err(e) => Err(e.to_string()),
    }
}

/// GUI 打开目录：解析在 core，事件 emit 属于 GUI 宿主层
#[tauri::command]
pub fn open_path(app: AppHandle, path: String) -> Result<String, String> {
    match ledata_core::parse_dir(std::path::Path::new(&path)) {
        Ok(outcome) => {
            if outcome.errors.len() != 0 {
                let error_text = outcome.errors.join("<br>");
                app.emit_to(EventTarget::any(), "fail-to-open", "以下文件的表格格式不受支持:<br>".to_string() + &error_text)
                    .unwrap();
            }
            if outcome.warnings.len() != 0 {
                let warn_text = outcome.warnings.join("<br>");
                app.emit_to(EventTarget::any(), "no-match-spc", warn_text)
                    .unwrap();
            }

            Ok(serde_json::to_string(&outcome.chips).unwrap())
        }
        Err(e) => Err(e.to_string()),
    }
}
