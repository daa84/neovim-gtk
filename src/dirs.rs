use std;
use std::path::PathBuf;

pub fn get_app_config_dir_create() -> Result<PathBuf, String> {
    let config_dir = get_app_config_dir()?;

    std::fs::create_dir_all(&config_dir).map_err(
        |e| format!("{}", e),
    )?;

    Ok(config_dir)
}

pub fn get_app_config_dir() -> Result<PathBuf, String> {
    let mut config_dir = get_xdg_config_dir()?;

    config_dir.push("nvim-gtk");

    Ok(config_dir)
}

fn get_xdg_config_dir() -> Result<PathBuf, String> {
    if let Ok(config_path) = std::env::var("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(config_path));
    }

    let mut home_dir = std::env::home_dir().ok_or(
        "Impossible to get your home dir!",
    )?;
    home_dir.push(".config");
    Ok(home_dir)
}

