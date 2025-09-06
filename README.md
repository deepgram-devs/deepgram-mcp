# Deepgram MCP Server

A Model Context Protocol (MCP) server that wraps Deepgram's text-to-speech API, allowing AI assistants to generate audio files from text.

## Features

- Text-to-speech conversion using Deepgram's Aura voices
- Generates MP3 audio files
- Simple MCP tool interface
- Configurable output filenames

## Prerequisites

- Rust (latest stable version)
- Deepgram API key (get one at [deepgram.com](https://deepgram.com))

## Installation

1. Clone this repository:

```bash
git clone <repository-url>
cd deepgram-mcp
```

2. Install the binary:

```bash
cargo install --path .
```

## Usage

### Available Tools

#### `deepgram_text_to_speech`

Generates an audio file from text using Deepgram's text-to-speech API.

**Parameters:**

- `text` (required): The text to convert to speech
- `filename` (optional): Output filename (defaults to "output.mp3")

**Example usage in an AI assistant:**

```text
Generate an audio file that says 'Hello Mael how long have you known Gustave?' using Deepgram.
```

This will create an MP3 file with the spoken text using Deepgram's Aura voice model.

### Integration with MCP Clients

To use this server with an MCP client like Claude Desktop, add it to your configuration:

```json
{
  "mcpServers": {
    "deepgram": {
      "command": "/Users/MYUSERNAME/.cargo/bin/deepgram-mcp",
      "env": {
        "DEEPGRAM_API_KEY": "your_api_key_here"
      }
    }
  }
}
```

## Configuration

The server requires a Deepgram API key set via the `DEEPGRAM_API_KEY` environment variable.

## Voice Model

The server uses Deepgram's `aura-asteria-en` voice model by default. This provides natural-sounding English speech synthesis.

## Error Handling

The server provides detailed error messages for:

- Missing API key
- Invalid API responses
- File system errors
- Malformed requests

## Development

### Running in Development

```bash
npx @modelcontextprotocol/inspector cargo run --quiet
```

## License

See `LICENSE.md`

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## Support

For issues related to:

- Deepgram API: Check [Deepgram's documentation](https://developers.deepgram.com/)
- MCP Protocol: See [Model Context Protocol specification](https://spec.modelcontextprotocol.io/)
- This MCP server: Open an issue in this GitHub repository
