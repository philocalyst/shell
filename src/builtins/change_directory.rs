use std::path::{Path, PathBuf};

use std::env;
pub fn cd(target: &PathBuf) {
    env::set_current_dir(target).unwrap()
}
