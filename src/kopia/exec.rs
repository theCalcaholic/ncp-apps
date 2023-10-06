use std::fs::{create_dir_all, read_to_string};
use std::path::Path;
use std::process::{Command, ExitStatus, Output};
use std::str::FromStr;
use clap::Parser;
use serde::Deserialize;
use regex::{Match, Regex, Captures};
use once_cell::sync::Lazy;
use crate::kopia::exec::RepoConfig::{B2, FileSystem, FromConfig, S3, Sftp};

const NCP_CONFIG_DIR: &str = "/usr/local/etc/ncp-config.d";
const KOPIA_CONFIG_DIR: &str = "/usr/local/etc/kopia";
const KOPIA_LOG_DIR: &str = "/var/log/kopia";
const REPOSITORY_DEFINITION_RE: &str = r"(?<type>.*)://((?<username>.*)@)?((?<host>.*):)?(?<path>.+)";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    repository: Option<String>,
    #[arg(short = 'p', long)]
    repository_password: Option<String>,
    #[arg(short, long)]
    storage_key: Option<String>,
    #[arg(short = 'b', long, default_value = "false")]
    run_backup: bool,
    #[arg(short, long, default_value = "true")]
    automatic_backups: bool,
    #[arg(short, long, default_value = "false")]
    web_ui: bool,
}

#[derive(Debug, Clone, Default)]
struct FileSystemRepoConfig {
    path: String,
}

impl FileSystemRepoConfig {
    fn from_definition(captures: Captures) -> Result<Self, &str> {
        let path = captures.name("path").ok_or("Failed to parse repo defintion")?.as_str();
        if path.is_empty() {
            Err("Repository definition musn't be empty")
        } else {
            Ok(FileSystemRepoConfig {
                path: path.to_string()
            })
        }
    }
}

#[derive(Debug, Clone, Default)]
struct SftpRepoConfig {
    username: String,
    host: String,
    path: String,
    private_key_data: Option<String>,
    known_hosts_data: Option<String>,
}

impl SftpRepoConfig {
    fn from_definition(captures: Captures, private_key_data: Option<String>, known_hosts_data: Option<String>) -> Result<Self, String> {
        let parse_error_msg = "Failed to parse repo defintion";
        let repo_type = captures.name("type").ok_or(parse_error_msg)?.as_str();
        if repo_type != "sftp" {
            return Err(format!("Can't create SftpRepoConfig from '{}'", repo_type));
        }
        Ok(SftpRepoConfig {
            username: captures.name("username").ok_or(parse_error_msg)?.as_str().to_string(),
            host: captures.name("host").ok_or(parse_error_msg)?.as_str().to_string(),
            path: captures.name("path").ok_or(parse_error_msg)?.as_str().to_string(),
            private_key_data,
            known_hosts_data,
        })
    }
}

#[derive(Debug, Clone, Default)]
struct B2RepoConfig {
    bucket: String,
    key_id: String,
    key_value: String,
}

impl B2RepoConfig {
    fn from_definition(captures: Captures, storage_key: Option<String>) -> Result<Self, String> {
        let storage_key_str = storage_key.ok_or("Storage key is required for S3 storage")?;
        let key_parts: Vec<&str> = storage_key_str
            .split(":").take(2).collect();
        Ok(B2RepoConfig {
            bucket: captures.name("host")
                .ok_or("Failed to parse repo defintion")?
                .as_str().to_string(),
            key_id: key_parts.get(0)
                .ok_or("Failed to parse storage key")?
                .to_string(),
            key_value: key_parts.get(1)
                .ok_or("Failed to parse storage key")?
                .to_string(),
        })
    }
}

#[derive(Debug, Clone, Default)]
struct S3RepoConfig {
    bucket: String,
    access_key_id: String,
    secret_access_key: String,
}

impl S3RepoConfig {
    fn from_definition(captures: Captures, storage_key: Option<String>) -> Result<Self, String> {
        let storage_key_str = storage_key.ok_or("Storage key is required for S3 storage")?;
        let key_parts: Vec<&str> = storage_key_str
            .split(":").take(2).collect();
        Ok(S3RepoConfig {
            bucket: captures.name("host")
                .ok_or("Failed to parse repo defintion")?
                .as_str().to_string(),
            access_key_id: key_parts.get(0)
                .ok_or("Failed to parse storage key")?
                .to_string(),
            secret_access_key: key_parts.get(1)
                .ok_or("Failed to parse storage key")?
                .to_string(),
        })
    }
}

#[derive(Debug, Clone)]
struct FromConfigRepoConfig {
    config_type: Box<RepoConfig>,
}

impl FromConfigRepoConfig {
    fn from_config_file(config: &str) -> Result<Self, String> {
        let json: serde_json::Value = serde_json::from_str(config)
            .map_err(|e| e.to_string())?;
        match json["storage"]["type"].as_str() {
            Some("filesystem") => {
                let path = json["storage"]["config"]["path"].as_str()
                    .ok_or("Failed to parse repository config file")?
                    .replacen("/host/", "/", 1).to_string();
                Ok(Self { config_type: Box::new(FileSystem(FileSystemRepoConfig { path })) })
            }
            Some("sftp") => {
                Ok(Self { config_type: Box::new(Sftp(SftpRepoConfig::default())) })
            }
            Some("b2") => {
                Ok(Self { config_type: Box::new(B2(B2RepoConfig::default())) })
            }
            Some("s3") => {
                Ok(Self { config_type: Box::new(S3(S3RepoConfig::default())) })
            }
            Some(repo_type) => Err(format!("Unsupported repository type: {}", repo_type).to_string()),
            None => Err("Failed to parse repository config file".to_string())
        }
    }
}

#[derive(Debug, Clone)]
enum RepoConfig {
    FileSystem(FileSystemRepoConfig),
    Sftp(SftpRepoConfig),
    B2(B2RepoConfig),
    S3(S3RepoConfig),
    FromConfig(FromConfigRepoConfig),
}

impl RepoConfig {
    fn from_definition(repo_arg: Option<String>, storage_key: Option<String>, config_file_contents: Option<&str>) -> Result<Self, String> {
        static REPO_DEF_RE: Lazy<Regex> = Lazy::new(|| Regex::from_str(REPOSITORY_DEFINITION_RE).unwrap());
        fn get_from_config_cfg(config_file_contents: Option<&str>) -> Result<RepoConfig, String> {
            Ok(FromConfig(
                FromConfigRepoConfig::from_config_file(config_file_contents
                    .ok_or("Failed to parse either the repository parameter or an existing repository config file".to_string())?)?))
        }
        match repo_arg {
            None => get_from_config_cfg(config_file_contents),
            Some(s) if s.is_empty() => get_from_config_cfg(config_file_contents),
            Some(def) => {
                let captures = REPO_DEF_RE.captures(def.as_str())
                    .ok_or("Failed to parse repository definition")?;
                match captures.name("type").map(|m| m.as_str()) {
                    None | Some("file") => Ok(FileSystem(FileSystemRepoConfig::from_definition(captures)?)),
                    Some("sftp") => Ok(Sftp(SftpRepoConfig::from_definition(captures, storage_key, None)?)),
                    Some("s3") => Ok(S3(S3RepoConfig::from_definition(captures, storage_key)?)),
                    Some("b2") => Ok(B2(B2RepoConfig::from_definition(captures, storage_key)?)),
                    Some(repo_type) => Err(format!("Unsupported storage engine: {}", repo_type))
                }
            }
        }
    }
}


// fn get_repo_config(repo_arg: &Option<String>) -> Result<RepoConfig, String> {
//     let repository_definition_re = Regex::new(REPOSITORY_DEFINITION_RE).unwrap();
//     let repo_def = match repo_arg {
//         Some(s) => s,
//         None => {
//             let repo_config = read_to_string(format!("{}/repository.config", KOPIA_CONFIG_DIR))
//                 .map_err(|e| e.to_string())?;
//             let json: serde_json::Value = serde_json::from_str(repo_config.as_str())
//                 .map_err(|e| e.to_string())?;
//             Ok(json["storage"]["type"].as_str().ok_or("Failed to parse config file")?.to_string())
//         }
//     };
//
//     let captures = repository_definition_re.captures(&*repo_def)
//         .expect(&*format!("Failed to parse repository: '{}'", repo_def));
//     match captures.name("type") {
//         None => Ok(FileSystem(FileSystemRepoConfig::default())),
//         Some(repo_type) => match repo_type.as_str() {
//             "file" => Ok(FileSystem(FileSystemRepoConfig::default())),
//             "sftp" => Ok(Sftp(SftpRepoConfig::default())),
//             "b2" => Ok(B2(B2RepoConfig::default())),
//             "s3" => Ok(S3(S3RepoConfig::default())),
//             s => Err(format!("Unsupported repository type: {}", s).to_string())
//         }
//     }
//
// }

// fn get_docker_args(cfg: RepoConfig) -> Vec<String> {
//     match cfg {
//         FileSystem(FileSystemRepoConfig { path }) => vec!["-v".to_string(), format!("{}:/repository", path).to_string()],
//         Sftp(_) => {}
//         B2(_) => {}
//         S3(_) => {}
//         FromConfig(_) => {}
//     }
// }

fn setup_repository(args: Args, config_file_contents: Option<&str>) -> Result<(), String> {
    let hostname = if let Ok(out) = Command::new("ncc")
        .arg("config:system:get")
        .arg("overwrite.cli.url")
        .output() {
        match out.status.success() {
            true => String::from_utf8_lossy(&*out.stdout).to_string(),
            false => String::from("ncp")
        }
    } else {
        String::from("ncp")
    };

    create_dir_all(KOPIA_CONFIG_DIR).map_err(|e| e.to_string())?;
    create_dir_all(KOPIA_LOG_DIR).map_err(|e| e.to_string())?;

    let repo_config = RepoConfig::from_definition(args.repository, args.storage_key, config_file_contents)?;


    Ok(())
}

fn main() {
    let args = Args::parse();
}

#[cfg(test)]
mod tests {
    use ncp::NcpAppConfig;

    #[test]
    fn deserialize_config() {
        let config = r#"{
  "id": "kopia",
  "name": "kopia-backup",
  "title": "Kopia Backup",
  "description": "Backup this NC instance using kopia (kopia.io)",
  "info": "Includes data and database. Supported targets are currently the local filesystem, sftp, S3 and B2.\nRepositories can have the following formats:\nlocal filesystem: file:///mnt/backup-path\nSFTP: sftp://user@host:/mnt/backup-path\nS3 Bucket: s3://bucket-id\nB2 Bucket: b2://bucket-id",
    "infotitle": "Incremental Backups with Kopia",
    "params": [
    {
        "id": "REPOSITORY",
        "name": "Kopia repository",
        "value": "",
        "suggest": "leave empty to use preconfigured repository"
    },
    {
        "id": "REPOSITORY_PASSWORD",
        "name": "Kopia Repository Password",
        "value": "",
        "suggest": "password",
        "type": "password"
    },
    {
        "id": "STORAGE_KEY",
        "name": "Storage key",
        "value": "",
        "suggest": "Either an ssh private key for sftp storage or KEY_ID:KEY_VALUE for b2/s3 storage (not required for local filesystem storage)",
        "type": "multiline",
        "unsafe": "true",
        "sensitive": "true"
    },
    {
        "id": "RUN_BACKUP",
        "name": "Run manual backup",
        "value": "no",
        "suggest": "no",
        "type": "bool"
    },
    {
        "id": "AUTOMATIC_BACKUPS",
        "name": "Enable auto backups",
        "value": "no",
        "suggest": "yes",
        "type": "bool"
    },
    {
        "id": "ENABLE_WEB_UI",
        "name": "Enable Kopia web UI",
        "value": "no",
        "suggest": "no",
        "type": "bool"
    }
    ]
}
        "#;
        let config: NcpAppConfig = serde_json::from_str(config).expect("Failed to parse JSON");
    }
}
