use reqwest::{header::HeaderMap, Client};

const ROOT: &str = "https://www.pinterest.com";

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
    BadUrl
}


#[derive(Debug)]
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
            [.., user, name, ""] | [.., user, name]  => Some(Self {
                user: decode_str(user).unwrap(),
                name: decode_str(name).unwrap(),
            }),
            _ => None
        }
    }
}

#[derive(Debug)]
struct Board {
    client: Client,
    pub id: String,
}

impl Board {
    fn from_id(client: Client, id: String) -> Self {
        Self { client, id }
    }

    async fn from_description(client: Client, description: &BoardDescription) -> Result<Self, CreationError> {
        let into_api_error = |e| <reqwest::Error as Into<ApiError>>::into(e);
        let board_url = Self::request_board_url(description);

        let response = client
            .get(board_url)
            .headers(Self::headers())
            .send()
            .await.map_err(into_api_error)?
            .json::<serde_json::Value>()
            .await.map_err(into_api_error)?;

        let id = response
            .pointer("/resource_response/data/id")
            .ok_or(ApiError::MissingField).unwrap()
            .as_str()
            .ok_or(ApiError::MissingField)?;


        Ok(Self::from_id(client, id.to_string()))
    }

    async fn from_url(client: Client, url: &str) -> Result<Self, CreationError> {
        let description = BoardDescription::from_url(url).ok_or(CreationError::BadUrl)?;
        Self::from_description(client, &description).await
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
        ].into_iter() {
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
        }).to_string();

        Self::url_options("Board", &data)
    }

    fn request_pins_url(&self) -> reqwest::Url {
        let data = serde_json::json!({
            "options": {
                "board_id": self.id,
                "field_set_key": "react_grid_pin",
                "prepend": false,
                "bookmarks": (),
            }
        }).to_string();

        Self::url_options("BoardFeed", &data)
    }

    fn url_options(name: &str, options: &str) -> reqwest::Url {
        reqwest::Url::parse_with_params(
            &resource_format(name), vec![
                ("data", options),
                ("source_url", "")
            ]).unwrap()
    }

    async fn pins(&self) -> Result<Vec<(String, Option<String>)>, ApiError>
    {
        let into_api_error = |e| <reqwest::Error as Into<ApiError>>::into(e);
        let board = self.client
            // Request
            .get(self.request_pins_url())
            .headers(Self::headers())
            .send()
            .await.map_err(into_api_error)?

            // Decode
            .json::<serde_json::Value>()
            .await.map_err(into_api_error)?;

            // Pins
        let pins = board
            .pointer("/resource_response/data")
            .ok_or(ApiError::MissingField)?
            .as_array()
            .ok_or(ApiError::MissingField)?;

            // Images
        let r = pins
            .into_iter()
            .map(|value| (
                value.get("id").unwrap().as_str().unwrap().to_string(),
                value.pointer("/images/orig/url")
                .and_then(serde_json::Value::as_str)
                .map(|s| s.to_string())
            ))
            .take(pins.len() - 1)
            .collect();

        Ok(r)
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let url = "https://www.pinterest.com/DrunkenWarlock/%D1%8D%D1%81%D1%82%D0%B5%D1%82%D0%B8%D0%BA%D0%B0/";
    let client = Client::new();
    let board = Board::from_url(client, url).await.unwrap();
    let pins = board.pins().await.unwrap();
    println!("{pins:?}");
}
