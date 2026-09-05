use thiserror::Error;

/// 错误文案与原 GUI（tauri command 返回的 String）逐字一致
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("{file} 表格的格式不受支持")]
    UnsupportedFormat { file: String },
    #[error("该目录为空")]
    EmptyDir,
    #[error("该目录下没有找到符合支持格式的数据表格")]
    NoSupportedData,
}
