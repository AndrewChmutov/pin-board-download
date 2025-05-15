use std::path::PathBuf;

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{header::HeaderMap, Client};
use std::sync::Arc;
use tokio::{io::AsyncWriteExt, sync::Semaphore, task::JoinHandle};

const ROOT: &str = "https://www.pinterest.com";
const MAX_BOOKMARK_SIZE: usize = 26;

fn resource_format(resource: &str) -> String {
    format!("{ROOT}/resource/{resource}Resource/get/")
}

#[derive(Debug, thiserror::Error)]
enum ApiError {
    #[error("Network request failed")]
    RequestFailed(#[from] reqwest::Error),
    #[error("Missing expected data field")]
    MissingField,
}

#[derive(Debug, thiserror::Error)]
enum CreationError {
    #[error("Something went wrong during fetching data")]
    ApiError(#[from] ApiError),
    #[error("Url is incorrect")]
    BadUrl,
}

#[derive(Debug, Clone)]
struct BoardDescription {
    pub user: String,
    pub name: String,
}

fn decode_str(s: &str) -> Option<String> {
    percent_encoding::percent_decode_str(s)
        .decode_utf8()
        .map(|s| s.to_string())
        .ok()
}

impl BoardDescription {
    fn from_url(url: &str) -> Option<Self> {
        match url.split('/').collect::<Vec<_>>()[..] {
            [.., user, name, ""] | [.., user, name] => Some(Self {
                user: decode_str(user)?,
                name: decode_str(name)?,
            }),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct Board {
    client: Client,
    name: String,
    #[allow(dead_code)]
    user: String,
    id: String,
    len: usize,
}

#[derive(Debug)]
struct Pin {
    id: String,
    pic_url: String,
}

#[derive(Debug)]
struct PinResult {
    pins: Vec<Pin>,
    bookmarks: Vec<String>,
}

impl Board {
    async fn from_description(
        client: Client,
        description: BoardDescription,
    ) -> Result<Self, CreationError> {
        let into_api_error = |e| <reqwest::Error as Into<ApiError>>::into(e);
        let board_url = Self::request_board_url(&description);

        let response = client
            .get(board_url)
            .headers(Self::headers())
            .send()
            .await
            .map_err(into_api_error)?
            .json::<serde_json::Value>()
            .await
            .map_err(into_api_error)?;

        let id = response
            .pointer("/resource_response/data/id")
            .ok_or(ApiError::MissingField)?
            .as_str()
            .ok_or(ApiError::MissingField)?
            .to_string();

        let len = response
            .pointer("/resource_response/data/pin_count")
            .ok_or(ApiError::MissingField)?
            .as_u64()
            .ok_or(ApiError::MissingField)? as usize;

        let BoardDescription { name, user } = description;
        Ok(Self {
            client,
            name,
            user,
            id,
            len,
        })
    }

    async fn from_url(client: Client, url: &str) -> Result<Self, CreationError> {
        let description = BoardDescription::from_url(url).ok_or(CreationError::BadUrl)?;
        Self::from_description(client, description).await
    }

    fn headers() -> reqwest::header::HeaderMap {
        let mut headers = HeaderMap::new();
        for (key, value) in [
            ("Accept", "application/json, text/javascript, */*, q=0.01"),
            ("Content-Type", "application/json"),
            ("X-Requested-With", "XMLHttpRequest"),
            ("X-APP-VERSION", "a89153f"),
            ("X-Pinterest-AppState", "active"),
            ("X-Pinterest-Source-Url", ""),
            ("X-Pinterest-PWS-Handler", "www/[username].js"),
            ("Alt-Used", "www.pinterest.com"),
            ("Connection", "keep-alive"),
            ("Cookie", ""),
            ("Sec-Fetch-Dest", "empty"),
            ("Sec-Fetch-Mode", "cors"),
            ("Sec-Fetch-Site", "same-origin"),
        ]
        .into_iter()
        {
            headers.insert(key, reqwest::header::HeaderValue::from_static(value));
        }
        headers
    }

    fn request_board_url(description: &BoardDescription) -> reqwest::Url {
        let data = serde_json::json!({
            "options": {
                "slug": description.name,
                "username": description.user,
                "field_set_key": "detailed"
            }
        })
        .to_string();

        Self::url_options("Board", &data)
    }

    fn request_pins_url(&self, bookmarks: Vec<String>) -> reqwest::Url {
        let data = serde_json::json!({
            "options": {
                "board_id": self.id,
                "field_set_key": "react_grid_pin",
                "prepend": false,
                "bookmarks": bookmarks,
            }
        })
        .to_string();

        Self::url_options("BoardFeed", &data)
    }

    fn url_options(name: &str, options: &str) -> reqwest::Url {
        reqwest::Url::parse_with_params(
            &resource_format(name),
            vec![("data", options), ("source_url", "")],
        )
        .unwrap()
    }

    async fn pins(&self) -> Result<Vec<Pin>, ApiError> {
        let PinResult {
            mut pins,
            mut bookmarks,
        } = self.bookmark_pins(vec![]).await?;
        let pb = ProgressBar::new((self.len as f32 / MAX_BOOKMARK_SIZE as f32).ceil() as u64);
        pb.set_style(
            ProgressStyle::with_template("{prefix:>12.cyan.bold} [{bar:57}] {pos}/{len}")
                .unwrap()
                .progress_chars("=> "),
        );
        pb.set_prefix("Collecting bookmarks");

        loop {
            match &bookmarks[..] {
                [] => break,
                [s, ..] if s.starts_with("Y2JOb25lO") => break,
                [s, ..] if s == "-end-" => break,
                _ => {
                    let PinResult {
                        pins: new_pins,
                        bookmarks: new_bookmarks,
                    } = self.bookmark_pins(bookmarks).await?;
                    pb.inc(1);
                    pb.tick();
                    pins.extend(new_pins.into_iter());
                    bookmarks = new_bookmarks;
                }
            }
        }
        pb.finish();

        Ok(pins)
    }

    async fn bookmark_pins(&self, bookmarks: Vec<String>) -> Result<PinResult, ApiError> {
        let into_api_error = |e| <reqwest::Error as Into<ApiError>>::into(e);
        let board = self
            .client
            // Request
            .get(self.request_pins_url(bookmarks))
            .headers(Self::headers())
            .send()
            .await
            .map_err(into_api_error)?
            // Decode
            .json::<serde_json::Value>()
            .await
            .map_err(into_api_error)?;

        let pins_raw = board
            .pointer("/resource_response/data")
            .ok_or(ApiError::MissingField)?
            .as_array()
            .ok_or(ApiError::MissingField)?;

        // Images
        let pins = pins_raw
            .iter()
            .filter_map(|value| {
                value
                    .pointer("/images/orig/url")
                    .and_then(serde_json::Value::as_str)
                    .map(|s| Pin {
                        id: value.get("id").unwrap().as_str().unwrap().to_string(),
                        pic_url: s.to_string(),
                    })
            })
            .collect();

        let bookmarks = board
            .pointer("/resource/options/bookmarks")
            .ok_or(ApiError::MissingField)?
            .as_array()
            .ok_or(ApiError::MissingField)?
            .iter()
            .map(|value| value.as_str().unwrap().to_string())
            .collect();

        Ok(PinResult { pins, bookmarks })
    }
}

/// Program for Pinterest board downloading
#[derive(clap::Parser)]
struct Args {
    /// Board URL
    #[arg()]
    url: String,

    /// Directory where pictures should be stored
    #[arg(short, long)]
    dir: Option<PathBuf>,

    /// Whether to overwrite existing files with the same name
    #[arg(short, long, default_value_t = false)]
    force: bool,

    /// Maximum number of concurrent writes
    #[arg(long, default_value_t = 200usize)]
    max_concurrent_writes: usize,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let Args {
        url,
        dir,
        force,
        max_concurrent_writes,
    } = Args::parse();
    let (mut dir, join_name) = match (dir, std::env::current_dir()) {
        (Some(dir), _) => (dir, false),
        (None, Ok(dir)) => (dir, true),
        _ => {
            eprintln!("Couldn't access current directory, please provide the directopy path");
            return;
        }
    };

    let client = Client::new();
    let barrier = Arc::new(Semaphore::new(max_concurrent_writes));
    let board = Board::from_url(client.clone(), &url).await.unwrap();
    let pins = board.pins().await.unwrap();

    if join_name {
        dir = dir.join(&board.name);
    }
    if !dir.is_dir() {
        std::fs::create_dir(&dir).unwrap();
    }

    let pb = ProgressBar::new(pins.len() as u64);
    pb.set_style(
        ProgressStyle::with_template("{prefix:>12.cyan.bold} [{bar:57}] {pos}/{len}")
            .unwrap()
            .progress_chars("=> "),
    );
    pb.set_prefix("Downloading pictures");

    let handles = pins
        .into_iter()
        .map(|pin| {
            let Pin { id, pic_url } = pin;
            let client = client.clone();
            let pb = pb.clone();
            let dir = dir.clone();
            let barrier = barrier.clone();
            tokio::spawn(async move {
                let [.., extension] = pic_url.as_str().split('.').collect::<Vec<_>>()[..] else {
                    panic!("Couldn't find extension")
                };

                let mut path = dir.join(&id);
                path.set_extension(extension);
                if path.exists() && !force {
                    pb.inc(1);
                    return;
                }

                match client
                    .get(&pic_url)
                    .send()
                    .await
                    .and_then(|r| r.error_for_status())
                {
                    Ok(response) => {
                        _ = barrier.acquire().await.unwrap();
                        let content = response.bytes().await.unwrap();
                        let mut file = tokio::fs::File::create(path).await.unwrap();
                        file.write_all(&content).await.unwrap();
                    }
                    Err(_) => pb.println(format!("Could not download image from pin: {}", id)),
                }
                pb.inc(1);
            })
        })
        .collect::<Vec<JoinHandle<()>>>();

    futures::future::join_all(handles).await;
    pb.finish();
}
