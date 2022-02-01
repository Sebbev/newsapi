use serde::Deserialize;
use url::Url;

const BASE_URL: &str = "https://newsapi.org/v2/";

#[derive(thiserror::Error, Debug)]
pub enum NewsApiError {
    #[error("Failed to fetch articles")]
    #[cfg(feature = "async")]
    AsyncRequestError(#[from] reqwest::Error),
    #[error("Failed to fetch articles")]
    TransportError(#[from] ureq::Error),
    #[error("Failed to convert the response to string")]
    ConversionError(#[from] std::io::Error),
    #[error("Failed to parse the response")]
    ParseError(#[from] serde_json::Error),
    #[error("Failed to parse the URL")]
    UrlParseError(#[from] url::ParseError),
    #[error("Error: {0}")]
    UnknownError(&'static str),
    #[error("Request failed: {0}")]
    BadRequest(&'static str),
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
pub struct NewsAPIResponse {
    status: String,
    totalResults: u32,
    code: Option<String>,
    articles: Vec<Article>,
}

impl NewsAPIResponse {
    pub fn articles(&self) -> &Vec<Article> {
        &self.articles
    }
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
pub struct Article {
    source: ArticleSource,
    title: String,
    author: Option<String>,
    description: Option<String>,
    url: String,
}

impl Article {
    pub fn source(&self) -> &ArticleSource {
        &self.source
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn author(&self) -> Option<&str> {
        self.author.as_deref()
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

#[derive(Deserialize, Debug)]
pub struct ArticleSource {
    id: Option<String>,
    name: String,
}

impl ArticleSource {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }
}

pub enum Endpoint {
    TopHeadlines,
}

impl ToString for Endpoint {
    fn to_string(&self) -> String {
        match self {
            Self::TopHeadlines => "top-headlines".to_string(),
        }
    }
}

pub enum Country {
    US,
    SE,
}

impl ToString for Country {
    fn to_string(&self) -> String {
        match self {
            Self::US => "us".to_string(),
            Self::SE => "se".to_string(),
        }
    }
}

pub struct NewsAPI {
    api_key: String,
    endpoint: Endpoint,
    country: Country,
}

impl NewsAPI {
    pub fn new(api_key: &str) -> NewsAPI {
        NewsAPI {
            api_key: api_key.to_string(),
            endpoint: Endpoint::TopHeadlines,
            country: Country::US,
        }
    }

    pub fn endpoint(&mut self, endpoint: Endpoint) -> &mut NewsAPI {
        self.endpoint = endpoint;
        self
    }

    pub fn country(&mut self, country: Country) -> &mut NewsAPI {
        self.country = country;
        self
    }

    pub fn prepare_url(&self) -> Result<String, NewsApiError> {
        let mut url = Url::parse(BASE_URL)?;
        url.path_segments_mut()
            .unwrap()
            .push(&self.endpoint.to_string());

        let country = format!("country={}", self.country.to_string());
        url.set_query(Some(&country));

        Ok(url.to_string())
    }

    pub fn fetch(&self) -> Result<NewsAPIResponse, NewsApiError> {
        let url = self.prepare_url()?;
        let req = ureq::get(&url).set("Authorization", &self.api_key);
        let json: NewsAPIResponse = req.call()?.into_json()?;
        match json.status.as_str() {
            "ok" => Ok(json),
            _ => Err(map_response_err(json.code)),
        }
    }

    #[cfg(feature = "async")]
    pub async fn fetch_async(&self) -> Result<NewsAPIResponse, NewsApiError> {
        let url = self.prepare_url()?;
        let client = reqwest::Client::new();
        let req = client
            .request(reqwest::Method::GET, &url)
            .header("Authorization", &self.api_key)
            .build()?;
        let json: NewsAPIResponse = client.execute(req).await?.json().await?;
        match json.status.as_str() {
            "ok" => Ok(json),
            _ => Err(map_response_err(json.code)),
        }
    }
}

fn map_response_err(code: Option<String>) -> NewsApiError {
    match code {
        Some(code) => match code.as_str() {
            "apiKeyDisabled" => NewsApiError::BadRequest("Your API key has been disabled"),
            _ => NewsApiError::UnknownError("Unknown error"),
        },
        None => NewsApiError::UnknownError("Unknown Error"),
    }
}
