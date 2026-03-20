use base64::{engine::general_purpose, Engine as _};
use chrono::TimeZone;
use mailparse::{parse_mail, MailHeaderMap};
use native_tls::TlsConnector;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub imap_host: String,
    pub imap_port: u16,
    pub username: String,
    pub password: String,
    pub sender_email: String,
    pub mailbox: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            imap_host: String::new(),
            imap_port: 993,
            username: String::new(),
            password: String::new(),
            sender_email: String::new(),
            mailbox: "INBOX".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmailRecord {
    pub id: i64,
    pub title: String,
    pub sent_at: String,
    pub image_filename: String,
}

fn get_app_dir(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().expect("Failed to get app data dir")
}

fn get_db_path(app: &AppHandle) -> PathBuf {
    get_app_dir(app).join("zarigani.db")
}

fn get_images_dir(app: &AppHandle) -> PathBuf {
    get_app_dir(app).join("images")
}

fn get_settings_path(app: &AppHandle) -> PathBuf {
    get_app_dir(app).join("settings.json")
}

fn ensure_dirs(app: &AppHandle) {
    fs::create_dir_all(get_app_dir(app)).ok();
    fs::create_dir_all(get_images_dir(app)).ok();
}

fn init_db(db_path: &PathBuf) -> Result<Connection, rusqlite::Error> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS emails (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            sent_at TEXT NOT NULL,
            image_filename TEXT NOT NULL,
            message_id TEXT UNIQUE
        );
        CREATE INDEX IF NOT EXISTS idx_sent_at ON emails(sent_at);",
    )?;
    Ok(conn)
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Result<Settings, String> {
    ensure_dirs(&app);
    let path = get_settings_path(&app);
    if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    } else {
        Ok(Settings::default())
    }
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    ensure_dirs(&app);
    let path = get_settings_path(&app);
    let content = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_month_data(app: AppHandle, year: i32, month: u32) -> Result<Vec<EmailRecord>, String> {
    ensure_dirs(&app);
    let db_path = get_db_path(&app);
    let conn = init_db(&db_path).map_err(|e| e.to_string())?;
    let prefix = format!("{:04}-{:02}-", year, month);
    let mut stmt = conn
        .prepare(
            "SELECT id, title, sent_at, image_filename FROM emails WHERE sent_at LIKE ? ORDER BY sent_at",
        )
        .map_err(|e| e.to_string())?;
    let records: Result<Vec<EmailRecord>, _> = stmt
        .query_map(params![format!("{}%", prefix)], |row| {
            Ok(EmailRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                sent_at: row.get(2)?,
                image_filename: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect();
    records.map_err(|e| e.to_string())
}

#[tauri::command]
fn get_image_data(app: AppHandle, filename: String) -> Result<String, String> {
    let images_dir = get_images_dir(&app);
    let image_path = images_dir.join(&filename);
    let data = fs::read(&image_path).map_err(|e| e.to_string())?;
    let ext = filename.rsplit('.').next().unwrap_or("jpg").to_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "image/jpeg",
    };
    Ok(format!(
        "data:{};base64,{}",
        mime,
        general_purpose::STANDARD.encode(&data)
    ))
}

#[tauri::command]
async fn fetch_emails(app: AppHandle) -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || fetch_emails_sync(app))
        .await
        .map_err(|e| e.to_string())?
}

fn fetch_emails_sync(app: AppHandle) -> Result<usize, String> {
    ensure_dirs(&app);
    let settings = get_settings(app.clone())?;

    if settings.imap_host.is_empty() || settings.username.is_empty() {
        return Err("設定が未完了です。設定画面からIMAPサーバー情報を入力してください。".to_string());
    }

    let images_dir = get_images_dir(&app);
    let db_path = get_db_path(&app);
    let conn = init_db(&db_path).map_err(|e| e.to_string())?;

    let tls = TlsConnector::builder()
        .build()
        .map_err(|e| e.to_string())?;

    let client = imap::connect(
        (settings.imap_host.as_str(), settings.imap_port),
        &settings.imap_host,
        &tls,
    )
    .map_err(|e| e.to_string())?;

    let mut session = client
        .login(&settings.username, &settings.password)
        .map_err(|(e, _)| e.to_string())?;

    session
        .select(&settings.mailbox)
        .map_err(|e| e.to_string())?;

    let search_query = format!("FROM \"{}\"", settings.sender_email);
    let message_ids = session
        .search(&search_query)
        .map_err(|e| e.to_string())?;

    let mut saved_count = 0;

    for &msg_seq in &message_ids {
        let messages = match session.fetch(msg_seq.to_string(), "RFC822") {
            Ok(m) => m,
            Err(_) => continue,
        };

        for message in messages.iter() {
            let body = match message.body() {
                Some(b) => b,
                None => continue,
            };

            let parsed = match parse_mail(body) {
                Ok(p) => p,
                Err(_) => continue,
            };

            let subject = parsed
                .headers
                .get_first_value("Subject")
                .unwrap_or_default();

            let message_id_header = parsed
                .headers
                .get_first_value("Message-ID")
                .unwrap_or_else(|| msg_seq.to_string());

            let date_str = parsed
                .headers
                .get_first_value("Date")
                .unwrap_or_default();

            let sent_at = parse_email_date(&date_str)
                .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string());

            let images = extract_images(&parsed);

            for (idx, (filename, data)) in images.into_iter().enumerate() {
                let safe_name = format!(
                    "{}_{}_{}_{}" ,
                    msg_seq,
                    idx,
                    &sent_at[..10],
                    sanitize_filename(&filename)
                );

                let unique_id = format!("{}_{}", message_id_header, idx);

                let exists: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM emails WHERE message_id = ?",
                        params![unique_id],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);

                if exists > 0 {
                    continue;
                }

                let image_path = images_dir.join(&safe_name);
                if fs::write(&image_path, &data).is_err() {
                    continue;
                }

                conn.execute(
                    "INSERT OR IGNORE INTO emails (title, sent_at, image_filename, message_id) VALUES (?1, ?2, ?3, ?4)",
                    params![subject, sent_at, safe_name, unique_id],
                )
                .ok();

                saved_count += 1;
            }
        }
    }

    session.logout().ok();
    Ok(saved_count)
}

fn parse_email_date(date_str: &str) -> Option<String> {
    // パース後にシステムのローカルタイム（JST等）に変換して保存する
    // メールの Date ヘッダーが UTC (+0000) で送信される場合でも正しい日付になる
    if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(date_str) {
        let local = dt.with_timezone(&chrono::Local);
        return Some(local.format("%Y-%m-%d %H:%M:%S").to_string());
    }
    // フォールバック: mailparse でパースしてローカルタイムに変換
    let ts = mailparse::dateparse(date_str).ok()?;
    let utc = chrono::Utc.timestamp_opt(ts, 0).single()?;
    let local = utc.with_timezone(&chrono::Local);
    Some(local.format("%Y-%m-%d %H:%M:%S").to_string())
}

fn extract_images(mail: &mailparse::ParsedMail) -> Vec<(String, Vec<u8>)> {
    let mut images = Vec::new();
    extract_images_recursive(mail, &mut images);
    images
}

fn extract_images_recursive(
    mail: &mailparse::ParsedMail,
    images: &mut Vec<(String, Vec<u8>)>,
) {
    let content_type = mail.ctype.mimetype.to_lowercase();

    if content_type.starts_with("image/") {
        let filename = mail
            .ctype
            .params
            .get("name")
            .cloned()
            .unwrap_or_else(|| {
                let ext = content_type.split('/').nth(1).unwrap_or("jpg");
                format!("image.{}", ext)
            });

        if let Ok(body) = mail.get_body_raw() {
            images.push((filename, body));
        }
    }

    for subpart in &mail.subparts {
        extract_images_recursive(subpart, images);
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            fetch_emails,
            get_month_data,
            get_image_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
