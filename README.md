# Facebook Totem

## Educational purposes only

Facebook Totem allows you to retrieve information about ads of a Facebook page. You can retrieve the number of people targeted, how much the ad cost and a lot of other information.

## Installtion

### Building from Source

```bash
git clone https://github.com/jonaylor89/facebook_totem.git
cd facebook_totem/
cargo build --release
```

The compiled binary will be available at `target/release/facebook_totem`

### Install with Cargo

```bash
cargo install --path .
```

# Usage

```bash
facebook_totem <COMMAND> [OPTIONS] --output <OUTPUT>

Commands:
  single  Get all ads on a single page
  multi   Get ads on multiple pages from a CSV file
  search  Search for a page by name
  help    Print this message or the help of the given subcommand(s)

Options:
  -o, --output <OUTPUT>  Name of the CSV output file
  -h, --help             Print help
  -V, --version          Print version
```

## Single Mode - Get ads from a single page

```bash
facebook_totem single --url <FACEBOOK_PAGE_URL> --output results.csv
```

## Multi Mode - Get ads from multiple pages

```bash
facebook_totem multi --urls pages.csv --columns url_column --output results.csv
```

## Search Mode - Search for pages by name

```bash
facebook_totem search --target "Page Name" --output search_results.csv
```

The output is saved in the `output/` folder. For multi mode, each page gets its own file named with the page name and ID.

## Testing

Run the comprehensive test suite:

```bash
cargo test
```

The test suite includes:
- **Unit tests** for core parsing and CSV writing functions
- **Integration tests** for CLI commands and workflows
- **Error handling tests** for edge cases and invalid inputs
- **Mock data tests** for testing with sample Facebook responses

Run tests with output:
```bash
cargo test -- --nocapture
```

Run specific test modules:
```bash
cargo test error_handling_tests
cargo test integration_with_mocks
```

