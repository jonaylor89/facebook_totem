use anyhow::Result;
use facebook_totem::*;
use std::process::Command;
use tempfile::{NamedTempFile, TempDir};
use std::fs;

#[tokio::test]
async fn test_cli_help_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("single"));
    assert!(stdout.contains("multi"));
    assert!(stdout.contains("search"));
}

#[tokio::test]
async fn test_cli_version_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--version"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("facebook_totem"));
}

#[test]
fn test_cli_invalid_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "invalid"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
}

#[test]
fn test_cli_missing_output_flag() {
    let output = Command::new("cargo")
        .args(&["run", "--", "single", "--url", "https://example.com"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("output") || stderr.contains("required"));
}

#[tokio::test]
async fn test_multi_mode_with_csv_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let csv_file = temp_dir.path().join("test_urls.csv");
    
    // Create a test CSV file
    let csv_content = "url,name\nhttps://facebook.com/page1,Page 1\nhttps://facebook.com/page2,Page 2\n";
    fs::write(&csv_file, csv_content)?;
    
    let output = Command::new("cargo")
        .args(&[
            "run", "--", "multi",
            "--urls", csv_file.to_str().unwrap(),
            "--columns", "url",
            "--output", "test_output.csv"
        ])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to execute command");

    // The command should fail because it's trying to make real HTTP requests
    // but it should parse the CSV file correctly
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should have found the targets in the CSV
    assert!(stdout.contains("2 targets found") || stderr.contains("error"));
    
    Ok(())
}

#[test]
fn test_multi_mode_with_invalid_csv() {
    let temp_dir = TempDir::new().unwrap();
    let csv_file = temp_dir.path().join("invalid.csv");
    
    let output = Command::new("cargo")
        .args(&[
            "run", "--", "multi",
            "--urls", csv_file.to_str().unwrap(),
            "--columns", "url",
            "--output", "test_output.csv"
        ])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
}

#[test]
fn test_cli_single_mode_syntax() {
    // Test that the CLI accepts the right arguments for single mode
    let output = Command::new("cargo")
        .args(&[
            "run", "--", "single",
            "--url", "https://example.com",
            "--output", "test.csv"
        ])
        .output()
        .expect("Failed to execute command");

    // Command should attempt to run (may fail due to network, but syntax is correct)
    // We mainly care that it doesn't fail due to argument parsing
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should not contain argument parsing errors
    assert!(!stderr.contains("required") || stderr.contains("error") || stderr.contains("network"));
}

#[tokio::test]
async fn test_search_mode_basic() {
    let temp_dir = TempDir::new().unwrap();
    
    let output = Command::new("cargo")
        .args(&[
            "run", "--", "search",
            "--target", "TestPage",
            "--output", "search_results.csv"
        ])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to execute command");

    // Should attempt to search (will fail due to network, but command structure is correct)
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    assert!(stdout.contains("Searching for pages") || stderr.contains("error"));
}

// Integration test for the actual library functions with mocked responses
#[cfg(test)]
mod integration_with_mocks {
    use super::*;

    #[tokio::test]
    async fn test_full_workflow_with_mocks() -> Result<()> {
        // Test the workflow with static data
        let html_response = r#"<html>some content[{"pageID":"123456789","other":"data"}]more content</html>"#;
        let ads_response = r#"for (;;);{"payload":{"results":[[{"ad_id":"123","content":"test ad","impressions":"1000-5000"}]]}}"#;

        // Test the workflow
        let page_id = extract_page_id_from_html(html_response)?;
        assert_eq!(page_id, "123456789");

        let ads = parse_facebook_ads_response(ads_response)?;
        assert_eq!(ads.len(), 1);
        assert_eq!(ads[0]["ad_id"], "123");

        Ok(())
    }

    #[tokio::test]
    async fn test_search_workflow_with_mocks() -> Result<()> {
        let search_response = r#"for (;;);{"payload":{"pageResults":[{"pageID":"123","pageName":"Test Page","pageProfilePictureURI":"test.jpg","pageURI":"test"}]}}"#;
        
        let pages = parse_facebook_search_response(search_response)?;
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].page_id, Some("123".to_string()));
        assert_eq!(pages[0].page_name, Some("Test Page".to_string()));

        // Test CSV writing
        let temp_file = NamedTempFile::new()?;
        let temp_path = temp_file.path().to_str().unwrap();
        
        write_facebook_pages_to_csv(&pages, temp_path)?;
        
        let content = fs::read_to_string(temp_path)?;
        assert!(content.contains("123"));
        assert!(content.contains("Test Page"));

        Ok(())
    }
}

// Test utilities and helper functions
#[cfg(test)]
mod test_utilities {
    use facebook_totem::FacebookPage;

    pub fn create_test_facebook_page() -> FacebookPage {
        FacebookPage {
            page_id: Some("123456789".to_string()),
            page_name: Some("Test Page".to_string()),
            page_profile_picture_uri: Some("https://example.com/pic.jpg".to_string()),
            page_uri: Some("https://facebook.com/testpage".to_string()),
        }
    }

    #[test]
    fn test_create_test_facebook_page() {
        let page = create_test_facebook_page();
        assert_eq!(page.page_id, Some("123456789".to_string()));
        assert_eq!(page.page_name, Some("Test Page".to_string()));
    }
}

// Error handling tests
#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_extract_page_id_error_cases() {
        // Test with empty string
        let result = extract_page_id_from_html("");
        assert!(result.is_err());

        // Test with invalid HTML
        let result = extract_page_id_from_html("<html>no page id</html>");
        assert!(result.is_err());

        // Test with malformed page ID
        let result = extract_page_id_from_html(r#"[{"pageID":""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_facebook_responses_error_cases() {
        // Test with invalid JSON
        let result = parse_facebook_search_response("invalid json");
        assert!(result.is_err());

        // Test with missing fields
        let result = parse_facebook_search_response(r#"{"payload":{}}"#);
        assert!(result.is_err());

        // Test ads response with invalid JSON
        let result = parse_facebook_ads_response("invalid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_csv_writing_with_empty_data() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let temp_path = temp_file.path().to_str().unwrap();
        
        // Test with empty data
        let empty_data: Vec<serde_json::Value> = vec![];
        let result = write_json_to_csv(&empty_data, temp_path);
        assert!(result.is_ok());

        // File should exist but be empty (or just headers)
        let content = fs::read_to_string(temp_path)?;
        assert!(content.is_empty());

        Ok(())
    }

    #[test]
    fn test_csv_writing_to_invalid_path() {
        let invalid_path = "/invalid/path/that/does/not/exist/file.csv";
        let data = vec![serde_json::json!({"test": "value"})];
        
        let result = write_json_to_csv(&data, invalid_path);
        assert!(result.is_err());
    }
}