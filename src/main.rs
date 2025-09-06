use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as TokioBufReader};

#[derive(Debug, Serialize, Deserialize)]
struct McpRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpError {
    code: i32,
    message: String,
    data: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolCall {
    name: String,
    arguments: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeepgramTtsRequest {
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeepgramTtsResponse {
    #[serde(rename = "content_type")]
    content_type: String,
    data: String,
}

struct DeepgramMcpServer {
    client: Client,
    api_key: String,
}

impl DeepgramMcpServer {
    fn new() -> Result<Self> {
        let api_key = env::var("DEEPGRAM_API_KEY")
            .map_err(|_| anyhow!("DEEPGRAM_API_KEY environment variable not set"))?;
        
        Ok(Self {
            client: Client::new(),
            api_key,
        })
    }

    async fn generate_audio(&self, text: &str) -> Result<Vec<u8>> {
        let url = "https://api.deepgram.com/v1/speak?model=aura-asteria-en";
        
        let request_body = json!({
            "text": text
        });

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Token {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Deepgram API error: {}", error_text));
        }

        let audio_data = response.bytes().await?;
        Ok(audio_data.to_vec())
    }

    async fn handle_list_tools(&self) -> Result<Value> {
        Ok(json!({
            "tools": [
                {
                    "name": "deepgram_text_to_speech",
                    "description": "Generate an audio file from text using Deepgram's text-to-speech API. The audio will be saved as an MP3 file.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "text": {
                                "type": "string",
                                "description": "The text to convert to speech"
                            },
                            "filename": {
                                "type": "string",
                                "description": "The filename for the output audio file (optional, defaults to 'output.mp3')"
                            }
                        },
                        "required": ["text"]
                    }
                }
            ]
        }))
    }

    async fn handle_call_tool(&self, name: &str, arguments: &HashMap<String, Value>) -> Result<Value> {
        match name {
            "deepgram_text_to_speech" => {
                let text = arguments
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'text' parameter"))?;

                let filename = arguments
                    .get("filename")
                    .and_then(|v| v.as_str())
                    .unwrap_or("output.mp3");

                let audio_data = self.generate_audio(text).await?;
                
                fs::write(filename, &audio_data)?;

                Ok(json!({
                    "content": [
                        {
                            "type": "text",
                            "text": format!("Successfully generated audio file '{}' from text: \"{}\"", filename, text)
                        }
                    ]
                }))
            }
            _ => Err(anyhow!("Unknown tool: {}", name)),
        }
    }

    async fn handle_request(&self, request: McpRequest) -> McpResponse {
        let result = match request.method.as_str() {
            "initialize" => {
                Ok(json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "deepgram-mcp",
                        "version": "0.1.0"
                    }
                }))
            }
            "tools/list" => self.handle_list_tools().await,
            "tools/call" => {
                match request.params.as_ref() {
                    Some(params) => {
                        match params.get("name").and_then(|v| v.as_str()) {
                            Some(name) => {
                                let arguments = params
                                    .get("arguments")
                                    .and_then(|v| v.as_object())
                                    .map(|obj| {
                                        obj.iter()
                                            .map(|(k, v)| (k.clone(), v.clone()))
                                            .collect::<HashMap<String, Value>>()
                                    })
                                    .unwrap_or_default();

                                self.handle_call_tool(name, &arguments).await
                            }
                            None => Err(anyhow!("Missing tool name"))
                        }
                    }
                    None => Err(anyhow!("Missing params"))
                }
            }
            _ => Err(anyhow!("Unknown method: {}", request.method)),
        };

        match result {
            Ok(result) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(result),
                error: None,
            },
            Err(e) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(McpError {
                    code: -32603,
                    message: e.to_string(),
                    data: None,
                }),
            },
        }
    }

    async fn run(&self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut reader = TokioBufReader::new(stdin);
        let mut stdout = tokio::io::stdout();
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<McpRequest>(trimmed) {
                        Ok(request) => {
                            let response = self.handle_request(request).await;
                            let response_json = serde_json::to_string(&response)?;
                            stdout.write_all(response_json.as_bytes()).await?;
                            stdout.write_all(b"\n").await?;
                            stdout.flush().await?;
                        }
                        Err(e) => {
                            eprintln!("Failed to parse request: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read line: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let server = DeepgramMcpServer::new()?;
    server.run().await
}