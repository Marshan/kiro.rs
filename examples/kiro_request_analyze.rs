use std::fs::File;
use std::io::Read;
use kiro_rs::kiro::parser::decoder::EventStreamDecoder;
use kiro_rs::kiro::model::events::Event;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    analyze_file("response_with_max_effort.txt")?;
    analyze_file("response_without_effort.txt")?;
    analyze_file("response_with_high_effort.txt")?;
    Ok(())
}

fn analyze_file(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n==================================================");
    println!("Analyzing File: {}", path);
    println!("==================================================");

    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // 找到 "=== Response Body ===\n" 并截取其后面的所有字节
    let marker = b"\n=== Response Body ===\n";
    let index = buffer.windows(marker.len())
        .position(|window| window == marker)
        .map(|idx| idx + marker.len())
        .unwrap_or(0);

    let raw_bytes = &buffer[index..];
    println!("Raw Response Body bytes: {}", raw_bytes.len());

    let mut decoder = EventStreamDecoder::new();
    if let Err(e) = decoder.feed(raw_bytes) {
        println!("Decoder feed error: {}", e);
    }

    use std::collections::HashMap;
    let mut event_counts = HashMap::new();
    let mut reasoning_text_len = 0;
    let mut assistant_text_len = 0;
    let mut sample_reasoning = String::new();
    let mut sample_assistant = String::new();

    for result in decoder.decode_iter() {
        match result {
            Ok(frame) => {
                let event_type = frame.event_type().unwrap_or("unknown").to_string();
                *event_counts.entry(event_type.clone()).or_insert(0) += 1;

                let payload_str = frame.payload_as_str();
                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&payload_str) {
                    if event_type == "reasoningContentEvent" {
                        if let Some(text) = json_val.get("text").and_then(|t| t.as_str()) {
                            reasoning_text_len += text.len();
                            if sample_reasoning.len() < 200 {
                                sample_reasoning.push_str(text);
                            }
                        }
                    } else if event_type == "assistantResponseEvent" {
                        if let Some(content) = json_val.get("content").and_then(|c| c.as_str()) {
                            assistant_text_len += content.len();
                            if sample_assistant.len() < 200 {
                                sample_assistant.push_str(content);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("DecoderError: {}", e);
            }
        }
    }

    println!("Event Counts: {:?}", event_counts);
    println!("Total reasoningContentEvent Text Length: {} chars", reasoning_text_len);
    println!("Total assistantResponseEvent Text Length: {} chars", assistant_text_len);
    println!("Reasoning Sample: {:?}", sample_reasoning);
    println!("Assistant Sample: {:?}", sample_assistant);

    Ok(())
}
