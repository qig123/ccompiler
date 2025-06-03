use std::path::Path;
use std::process::Command;

use crate::error::CompilerError;

pub fn preprocess(input_file: &Path, output_file: &Path) -> Result<(), CompilerError> {
    let status = Command::new("gcc")
        .args(&[
            "-E",
            "-P",
            input_file.to_str().unwrap(),
            "-o",
            output_file.to_str().unwrap(),
        ])
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(CompilerError::Io(format!(
            "Preprocessing failed for {}",
            input_file.display()
        )))
    }
}
pub fn get_preprocessed_path(input_path: &Path) -> PathBuf {
    input_path.with_extension("i")
}
