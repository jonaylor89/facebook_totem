// Example usage of facebook_totem library
// This is not meant to be run as it requires network access and valid Facebook URLs
// But it demonstrates the API usage

use facebook_totem::*;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Example 1: Extract page ID from HTML content
    let html_content = r#"<html>some content[{"pageID":"123456789","other":"data"}]more content</html>"#;
    let page_id = extract_page_id_from_html(html_content)?;
    println!("Extracted page ID: {}", page_id);

    // Example 2: Parse Facebook search response
    let search_response = r#"for (;;);{"payload":{"pageResults":[{"pageID":"123","pageName":"Test Page","pageProfilePictureURI":"test.jpg","pageURI":"test"}]}}"#;
    let pages = parse_facebook_search_response(search_response)?;
    println!("Found {} pages", pages.len());
    for page in &pages {
        println!("Page: {:?}", page.page_name);
    }

    // Example 3: Parse Facebook ads response
    let ads_response = r#"for (;;);{"payload":{"results":[[{"ad_id":"123","content":"test ad","impressions":"1000-5000"}]]}}"#;
    let ads = parse_facebook_ads_response(ads_response)?;
    println!("Found {} ads", ads.len());

    // Example 4: Write results to CSV
    write_facebook_pages_to_csv(&pages, "example_pages.csv")?;
    println!("Pages written to example_pages.csv");

    Ok(())
}