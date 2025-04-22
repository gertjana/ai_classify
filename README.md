# Classify

Classify is a modular Rust application that exposes an API endpoint for content classification using AI. It can analyze text or follow links to extract and classify content, generating up to 5 relevant tags.

## Features

- REST API for content classification
- Modular architecture with pluggable components
- Multiple storage options for content and tags
- Support for different AI/LLM classification engines
- Configuration system with API keys and credentials management
- Automatic URL detection

## Architecture

The application consists of the following main components:

1. **API Module**: HTTP endpoint for classification requests/responses
2. **Content Storage**: Pluggable storage for classified content (starts with local filesystem)
3. **Tag Storage**: Pluggable storage for tags (starts with Redis)
4. **Classifier**: Pluggable AI/LLM for classification (starts with Claude)

## Requirements

- Rust 1.70+
- Redis server (for tag storage)
- Optional: Anthropic API key for Claude classifier

## Configuration

Configuration is handled via environment variables, which can be set in a `.env` file:

```
# API Configuration
API_HOST=127.0.0.1
API_PORT=3000
API_KEY=your_secret_api_key

# Storage Configuration
STORAGE_TYPE=filesystem
CONTENT_STORAGE_PATH=./data/content

# Tag Storage Configuration
TAG_STORAGE_TYPE=redis
REDIS_URL=redis://127.0.0.1:6379
REDIS_PASSWORD=

# Classifier Configuration
CLASSIFIER_TYPE=claude
ANTHROPIC_API_KEY=your_anthropic_api_key
MAX_PROMPT_LENGTH=200000

# Logging
LOG_LEVEL=info
```

## Getting Started

1. Clone the repository
2. Create a `.env` file with appropriate configuration
3. Make sure Redis is running (if using Redis tag storage)
4. Build and run the application:

```bash
cargo build --release
./target/release/classify
```

## API Usage

### Authentication

All API endpoints require authentication using an API key. The key must be provided in the `X-Api-Key` HTTP header with every request.

Example:
```
X-Api-Key: your_secret_api_key
```

If the API key is not set in the environment variables, a random key will be generated on startup and printed to the console. You can set your own API key using the `API_KEY` environment variable.

### Classify Content

**Endpoint**: `POST /classify`

**Request Body**:
```json
{
  "content": "This is some text to classify or a URL starting with http:// or https://"
}
```

The application automatically detects if the content is a URL by checking if it starts with `http://` or `https://`.

**Response**:
```json
{
  "content": {
    "id": "b7dfe826-c4ed-4d01-8c0b-a1804c2a2a0c",
    "content": "This is some text to classify or a URL",
    "tags": ["tag1", "tag2", "tag3"],
    "created_at": "2023-10-25T19:31:42.123456Z",
    "updated_at": "2023-10-25T19:31:42.123456Z"
  },
  "success": true,
  "error": null
}
```

### Query Content by Tags

**Endpoint**: `GET /query?tags=tag1,tag2`

Use this endpoint to find content with any of the specified tags. Multiple tags can be provided as a comma-separated list, and the endpoint will return all content that has at least one of those tags.

**Response**:
```json
{
  "items": [
    {
      "id": "b7dfe826-c4ed-4d01-8c0b-a1804c2a2a0c",
      "content": "This is some text to classify or a URL",
      "tags": ["tag1", "tag2", "tag3"],
      "created_at": "2023-10-25T19:31:42.123456Z",
      "updated_at": "2023-10-25T19:31:42.123456Z"
    }
  ],
  "tags": ["tag1", "tag2"],
  "count": 1,
  "success": true,
  "error": null
}
```

### List All Tags

**Endpoint**: `GET /tags`

Use this endpoint to retrieve a list of all tags currently in the system.

**Response**:
```json
{
  "tags": ["tag1", "tag2", "tag3", "programming", "rust", "web"],
  "count": 6,
  "success": true,
  "error": null
}
```

### Delete Content

**Endpoint**: `DELETE /content/:id`

Use this endpoint to delete content by its ID. The endpoint will also clean up any orphaned tags (tags that are no longer used by any content).

**Response**:
```json
{
  "success": true,
  "id": "b7dfe826-c4ed-4d01-8c0b-a1804c2a2a0c",
  "removed_tags": ["orphaned-tag1", "orphaned-tag2"],
  "error": null
}
```

### Health Check

**Endpoint**: `GET /`

**Response**: HTTP 200 OK

## Extending the Application

### Adding a New Storage Provider

1. Create a new module in `src/storage/content/` or `src/storage/tag/`
2. Implement the `ContentStorage` or `TagStorage` trait
3. Update the factory function in `src/storage/mod.rs`
4. Add the new storage type to the configuration enums

### Adding a New Classifier

1. Create a new module in `src/classifier/`
2. Implement the `Classifier` trait
3. Update the factory function in `src/classifier/mod.rs`
4. Add the new classifier type to the configuration enums

## License

MIT
