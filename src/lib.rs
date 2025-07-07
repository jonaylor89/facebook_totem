use anyhow::Result;
use fake_useragent::UserAgents;
use reqwest::{Client, cookie::Jar};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::sync::Arc;
use csv::Writer;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct FacebookPage {
    #[serde(rename = "pageID")]
    pub page_id: Option<String>,
    #[serde(rename = "pageName")]
    pub page_name: Option<String>,
    #[serde(rename = "pageProfilePictureURI")]
    pub page_profile_picture_uri: Option<String>,
    #[serde(rename = "pageURI")]
    pub page_uri: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookPageSearchResponse {
    pub payload: FacebookPageSearchPayload,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookPageSearchPayload {
    #[serde(rename = "pageResults")]
    pub page_results: Vec<FacebookPage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookAdsResponse {
    pub payload: FacebookAdsPayload,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookAdsPayload {
    pub results: Vec<Vec<Value>>,
}

pub async fn get_id_from_url(url: &str) -> Result<String> {
    let client = Client::new();
    let response = client.get(url).send().await?;
    let text = response.text().await?;
    
    extract_page_id_from_html(&text)
}

pub fn extract_page_id_from_html(html: &str) -> Result<String> {
    let start = html.find("[{\"pageID\":\"").ok_or_else(|| anyhow::anyhow!("Could not find pageID in response"))?;
    let start = start + "[{\"pageID\":\"".len();
    let end = html[start..].find('"').ok_or_else(|| anyhow::anyhow!("Could not find end of pageID"))?;
    
    Ok(html[start..start + end].to_string())
}

pub async fn get_facebook_page_from_name(name: &str) -> Result<Vec<FacebookPage>> {
    get_facebook_page_from_name_with_client(name, &build_client().await?).await
}

pub async fn get_facebook_page_from_name_with_client(name: &str, client: &Client) -> Result<Vec<FacebookPage>> {
    let ua = UserAgents::new();
    let user_agent = ua.random();
    
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("User-Agent", user_agent.parse()?);
    headers.insert("Accept", "*/*".parse()?);
    headers.insert("Accept-Language", "en,en-US;q=0.5".parse()?);
    headers.insert("Referer", "https://www.facebook.com/ads/library/".parse()?);
    headers.insert("Content-Type", "application/x-www-form-urlencoded".parse()?);
    headers.insert("Origin", "https://www.facebook.com".parse()?);
    headers.insert("DNT", "1".parse()?);
    headers.insert("Connection", "keep-alive".parse()?);
    headers.insert("TE", "Trailers".parse()?);
    
    let mut params = HashMap::new();
    params.insert("ad_type", "all");
    params.insert("country", "");
    params.insert("is_mobile", "false");
    params.insert("q", name);
    params.insert("session_id", "\"\"");
    
    let data = build_facebook_form_data();
    
    let response = client
        .post("https://www.facebook.com/ads/library/async/search_typeahead/")
        .headers(headers)
        .query(&params)
        .form(&data)
        .send()
        .await?;
    
    let text = response.text().await?;
    parse_facebook_search_response(&text)
}

pub async fn get_ads_from_id(id: &str) -> Result<Vec<Value>> {
    get_ads_from_id_with_client(id, &build_client().await?).await
}

pub async fn get_ads_from_id_with_client(id: &str, client: &Client) -> Result<Vec<Value>> {
    let ua = UserAgents::new();
    let user_agent = ua.random();
    
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("User-Agent", user_agent.parse()?);
    headers.insert("Accept", "*/*".parse()?);
    headers.insert("Accept-Language", "en,en-US;q=0.5".parse()?);
    headers.insert("Referer", "https://www.facebook.com/ads/library/".parse()?);
    headers.insert("Content-Type", "application/x-www-form-urlencoded".parse()?);
    headers.insert("Origin", "https://www.facebook.com".parse()?);
    headers.insert("DNT", "1".parse()?);
    headers.insert("Connection", "keep-alive".parse()?);
    headers.insert("Cache-Control", "max-age=0".parse()?);
    
    let mut params = HashMap::new();
    params.insert("session_id", "\"\"");
    params.insert("count", "30");
    params.insert("active_status", "all");
    params.insert("ad_type", "all");
    params.insert("countries[0]", "ALL");
    params.insert("impression_search_field", "has_impressions_lifetime");
    params.insert("view_all_page_id", id);
    params.insert("sort_data[direction]", "desc");
    params.insert("sort_data[mode]", "relevancy_monthly_grouped");
    
    let data = build_facebook_form_data();
    
    let response = client
        .post("https://www.facebook.com/ads/library/async/search_ads/")
        .headers(headers)
        .query(&params)
        .form(&data)
        .send()
        .await?;
    
    let text = response.text().await?;
    parse_facebook_ads_response(&text)
}

pub fn parse_facebook_search_response(text: &str) -> Result<Vec<FacebookPage>> {
    let cleaned_text = text.replace("for (;;);", "");
    let parsed: FacebookPageSearchResponse = serde_json::from_str(&cleaned_text)?;
    Ok(parsed.payload.page_results)
}

pub fn parse_facebook_ads_response(text: &str) -> Result<Vec<Value>> {
    let cleaned_text = text.replace("for (;;);", "");
    let parsed: FacebookAdsResponse = serde_json::from_str(&cleaned_text)?;
    
    let mut result = Vec::new();
    for res in parsed.payload.results {
        for re in res {
            result.push(re);
        }
    }
    
    Ok(result)
}

pub fn write_json_to_csv(data: &[Value], filename: &str) -> Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    
    let file = File::create(filename)?;
    let mut wtr = Writer::from_writer(file);
    
    if let Some(first_item) = data.first() {
        if let Value::Object(obj) = first_item {
            let headers: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
            wtr.write_record(&headers)?;
            
            for item in data {
                if let Value::Object(obj) = item {
                    let mut record = Vec::new();
                    for header in &headers {
                        let value = obj.get(*header)
                            .map(|v| match v {
                                Value::String(s) => s.clone(),
                                Value::Number(n) => n.to_string(),
                                Value::Bool(b) => b.to_string(),
                                Value::Null => String::new(),
                                _ => serde_json::to_string(v).unwrap_or_default(),
                            })
                            .unwrap_or_default();
                        record.push(value);
                    }
                    wtr.write_record(&record)?;
                }
            }
        }
    }
    
    wtr.flush()?;
    Ok(())
}

pub fn write_facebook_pages_to_csv(pages: &[FacebookPage], filename: &str) -> Result<()> {
    let json_values: Vec<Value> = pages.iter()
        .map(|page| serde_json::to_value(page).unwrap_or(Value::Null))
        .collect();
    
    write_json_to_csv(&json_values, filename)
}

async fn build_client() -> Result<Client> {
    let jar = Arc::new(Jar::default());
    let client = Client::builder()
        .cookie_provider(jar)
        .build()?;
    Ok(client)
}

fn build_facebook_form_data() -> HashMap<&'static str, &'static str> {
    let mut data = HashMap::new();
    data.insert("__user", "0");
    data.insert("__a", "1");
    data.insert("__dyn", "\"\"");
    data.insert("__csr", "");
    data.insert("__req", "1");
    data.insert("__beoa", "0");
    data.insert("__pc", "PHASED:DEFAULT");
    data.insert("dpr", "1");
    data.insert("__ccg", "UNKNOWN");
    data.insert("__rev", "\"\"");
    data.insert("__s", "\"\"");
    data.insert("__hsi", "\"\"");
    data.insert("__comet_req", "0");
    data.insert("lsd", "\"\"");
    data.insert("jazoest", "\"\"");
    data.insert("__spin_r", "\"\"");
    data.insert("__spin_b", "trunk");
    data.insert("__spin_t", "\"\"");
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::NamedTempFile;
    use std::io::Read;

    #[test]
    fn test_extract_page_id_from_html() {
        let html = r#"<html>some content[{"pageID":"123456789","other":"data"}]more content</html>"#;
        let result = extract_page_id_from_html(html).unwrap();
        assert_eq!(result, "123456789");
    }

    #[test]
    fn test_extract_page_id_from_html_not_found() {
        let html = r#"<html>no page id here</html>"#;
        let result = extract_page_id_from_html(html);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_facebook_search_response() {
        let response_text = r#"for (;;);{"payload":{"pageResults":[{"pageID":"123","pageName":"Test Page","pageProfilePictureURI":"test.jpg","pageURI":"test"}]}}"#;
        let result = parse_facebook_search_response(response_text).unwrap();
        
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].page_id, Some("123".to_string()));
        assert_eq!(result[0].page_name, Some("Test Page".to_string()));
    }

    #[test]
    fn test_parse_facebook_ads_response() {
        let response_text = r#"for (;;);{"payload":{"results":[[{"ad_id":"123","content":"test ad"}]]}}"#;
        let result = parse_facebook_ads_response(response_text).unwrap();
        
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["ad_id"], "123");
        assert_eq!(result[0]["content"], "test ad");
    }

    #[test]
    fn test_write_json_to_csv() {
        let data = vec![
            json!({"name": "John", "age": 30}),
            json!({"name": "Jane", "age": 25}),
        ];
        
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_str().unwrap();
        
        write_json_to_csv(&data, temp_path).unwrap();
        
        let mut file_content = String::new();
        let mut file = File::open(temp_path).unwrap();
        file.read_to_string(&mut file_content).unwrap();
        
        assert!(file_content.contains("name,age") || file_content.contains("age,name"));
        assert!(file_content.contains("John,30") || file_content.contains("30,John"));
        assert!(file_content.contains("Jane,25") || file_content.contains("25,Jane"));
    }

    #[test]
    fn test_write_facebook_pages_to_csv() {
        let pages = vec![
            FacebookPage {
                page_id: Some("123".to_string()),
                page_name: Some("Test Page".to_string()),
                page_profile_picture_uri: Some("test.jpg".to_string()),
                page_uri: Some("test_uri".to_string()),
            },
        ];
        
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_str().unwrap();
        
        write_facebook_pages_to_csv(&pages, temp_path).unwrap();
        
        let mut file_content = String::new();
        let mut file = File::open(temp_path).unwrap();
        file.read_to_string(&mut file_content).unwrap();
        
        assert!(file_content.contains("123"));
        assert!(file_content.contains("Test Page"));
    }

    #[test]
    fn test_build_facebook_form_data() {
        let data = build_facebook_form_data();
        
        assert_eq!(data.get("__user"), Some(&"0"));
        assert_eq!(data.get("__a"), Some(&"1"));
        assert_eq!(data.get("__spin_b"), Some(&"trunk"));
        assert!(data.contains_key("__dyn"));
        assert!(data.contains_key("lsd"));
    }

    #[tokio::test]
    async fn test_parse_responses_with_mock_data() {
        // Test parsing without actual HTTP calls
        let mock_search_response = r#"for (;;);{"payload":{"pageResults":[{"pageID":"123","pageName":"Test Page","pageProfilePictureURI":"test.jpg","pageURI":"test"}]}}"#;
        let result = parse_facebook_search_response(mock_search_response);
        assert!(result.is_ok());
        let pages = result.unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].page_id, Some("123".to_string()));
        
        let mock_ads_response = r#"for (;;);{"payload":{"results":[[{"ad_id":"123","content":"test ad"}]]}}"#;
        let result = parse_facebook_ads_response(mock_ads_response);
        assert!(result.is_ok());
        let ads = result.unwrap();
        assert_eq!(ads.len(), 1);
        assert_eq!(ads[0]["ad_id"], "123");
    }

    #[test]
    fn test_facebook_page_serialization() {
        let page = FacebookPage {
            page_id: Some("123".to_string()),
            page_name: Some("Test Page".to_string()),
            page_profile_picture_uri: Some("test.jpg".to_string()),
            page_uri: Some("test_uri".to_string()),
        };
        
        let json = serde_json::to_string(&page).unwrap();
        let deserialized: FacebookPage = serde_json::from_str(&json).unwrap();
        
        assert_eq!(page, deserialized);
    }
}