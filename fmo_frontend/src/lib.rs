pub mod components;
mod util;

const BASE_URL: &str = match option_env!("FMO_API_SERVER") {
    Some(url) => url,
    None => "http://127.0.0.1:3000",
};
