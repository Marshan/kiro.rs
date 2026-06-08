use std::fs::File;
use std::io::Write;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthToken {
    access_token: String,
    refresh_token: String,
    profile_arn: String,
    expires_at: String,
    auth_method: String,
    provider: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 读取 token
    let token_path = "C:\\Users\\zhyha\\.aws\\sso\\cache\\kiro-auth-token.json";
    println!("Reading token from {}", token_path);
    let token_file = File::open(token_path)?;
    let mut auth: AuthToken = serde_json::from_reader(token_file)?;
    println!("Loaded Profile ARN: {}", auth.profile_arn);
    println!("Expires At: {}", auth.expires_at);

    // 配置代理
    let proxy_url = "http://192.168.0.110:31028";
    println!("Using proxy: {}", proxy_url);
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(proxy_url)?)
        .timeout(std::time::Duration::from_secs(90))
        .build()?;

    // 2. 检查是否过期或需要刷新
    let expires_time = chrono::DateTime::parse_from_rfc3339(&auth.expires_at)?;
    let now = chrono::Utc::now();
    let is_expired = expires_time.with_timezone(&chrono::Utc) < now + chrono::Duration::minutes(5);
    
    if is_expired {
        println!("Token is expired or expiring within 5 minutes. Refreshing token...");
        let refresh_url = "https://prod.us-east-1.auth.desktop.kiro.dev/refreshToken";
        let refresh_body = json!({
            "refreshToken": auth.refresh_token
        });
        
        let refresh_res = client.post(refresh_url)
            .header("Accept", "application/json, text/plain, */*")
            .header("Content-Type", "application/json")
            .header("User-Agent", "KiroIDE-1.0.0-test-machine-id")
            .header("host", "prod.us-east-1.auth.desktop.kiro.dev")
            .header("Connection", "close")
            .json(&refresh_body)
            .send()
            .await?;
            
        let status = refresh_res.status();
        println!("Refresh status: {}", status);
        if !status.is_success() {
            let err_body = refresh_res.text().await?;
            eprintln!("Failed to refresh token: {}", err_body);
            return Err("Token refresh failed".into());
        }
        
        // 解析响应并更新
        let data: serde_json::Value = refresh_res.json().await?;
        if let Some(access_token) = data.get("accessToken").and_then(|t| t.as_str()) {
            auth.access_token = access_token.to_string();
        }
        if let Some(new_refresh_token) = data.get("refreshToken").and_then(|t| t.as_str()) {
            auth.refresh_token = new_refresh_token.to_string();
        }
        if let Some(new_profile_arn) = data.get("profileArn").and_then(|t| t.as_str()) {
            auth.profile_arn = new_profile_arn.to_string();
        }
        if let Some(expires_in) = data.get("expiresIn").and_then(|t| t.as_i64()) {
            let new_expires_at = chrono::Utc::now() + chrono::Duration::seconds(expires_in);
            auth.expires_at = new_expires_at.to_rfc3339();
        }
        
        // 写回文件
        let mut out_file = File::create(token_path)?;
        serde_json::to_writer_pretty(&mut out_file, &auth)?;
        println!("Token refreshed successfully! Saved back to {}", token_path);
        println!("New Expires At: {}", auth.expires_at);
    } else {
        println!("Token is still valid.");
    }

    // 复杂的逻辑与编程问题
    let complex_prompt = "There are three people: Alice, Bob, and Charlie. One of them is a knight (always tells the truth), one is a knave (always lies), and one is a spy (can lie or tell the truth).\n\
                          Alice says: 'Charlie is a knave.'\n\
                          Bob says: 'Alice is a knight.'\n\
                          Charlie says: 'I am the spy.'\n\
                          Who is who? Explain the step-by-step reasoning and write a short Rust program to verify all possibilities.";

    // 3. 发送第一次请求：effort = max
    println!("\n--- Request 1: effort = max ---");
    let conv_id_1 = format!("conv-{}", Uuid::new_v4());
    let body_1 = json!({
        "conversationState": {
            "conversationId": conv_id_1,
            "currentMessage": {
                "userInputMessage": {
                    "content": complex_prompt,
                    "modelId": "claude-opus-4.7",
                    "userInputMessageContext": {
                        "tools": [],
                        "toolResults": []
                    },
                    "origin": "AI_EDITOR"
                }
            },
            "history": [],
            "agentTaskType": "vibe",
            "chatTriggerType": "MANUAL"
        },
        "profileArn": auth.profile_arn,
        "additionalModelRequestFields": {
            "output_config": {
                "effort": "max"
            }
        }
    });

    let res_1 = send_request(&client, &auth.access_token, &body_1).await?;
    let output_path_1 = "response_with_max_effort.txt";
    save_result(output_path_1, &res_1)?;
    println!("Saved Request 1 results to {}", output_path_1);

    // 4. 发送第二次请求：不带 effort
    println!("\n--- Request 2: WITHOUT effort ---");
    let conv_id_2 = format!("conv-{}", Uuid::new_v4());
    let body_2 = json!({
        "conversationState": {
            "conversationId": conv_id_2,
            "currentMessage": {
                "userInputMessage": {
                    "content": complex_prompt,
                    "modelId": "claude-opus-4.7",
                    "userInputMessageContext": {
                        "tools": [],
                        "toolResults": []
                    },
                    "origin": "AI_EDITOR"
                }
            },
            "history": [],
            "agentTaskType": "vibe",
            "chatTriggerType": "MANUAL"
        },
        "profileArn": auth.profile_arn
    });

    let res_2 = send_request(&client, &auth.access_token, &body_2).await?;
    let output_path_2 = "response_without_effort.txt";
    save_result(output_path_2, &res_2)?;
    println!("Saved Request 2 results to {}", output_path_2);

    // 5. 发送第三次请求：effort = high
    println!("\n--- Request 3: effort = high ---");
    let conv_id_3 = format!("conv-{}", Uuid::new_v4());
    let body_3 = json!({
        "conversationState": {
            "conversationId": conv_id_3,
            "currentMessage": {
                "userInputMessage": {
                    "content": complex_prompt,
                    "modelId": "claude-opus-4.7",
                    "userInputMessageContext": {
                        "tools": [],
                        "toolResults": []
                    },
                    "origin": "AI_EDITOR"
                }
            },
            "history": [],
            "agentTaskType": "vibe",
            "chatTriggerType": "MANUAL"
        },
        "profileArn": auth.profile_arn,
        "additionalModelRequestFields": {
            "output_config": {
                "effort": "high"
            }
        }
    });

    let res_3 = send_request(&client, &auth.access_token, &body_3).await?;
    let output_path_3 = "response_with_high_effort.txt";
    save_result(output_path_3, &res_3)?;
    println!("Saved Request 3 results to {}", output_path_3);

    println!("\nDone!");
    Ok(())
}

struct RequestResult {
    status: u16,
    headers: String,
    raw_response: Vec<u8>,
}

async fn send_request(
    client: &reqwest::Client,
    access_token: &str,
    body: &serde_json::Value,
) -> Result<RequestResult, Box<dyn std::error::Error>> {
    let url = "https://q.us-east-1.amazonaws.com/generateAssistantResponse";
    
    let invocation_id = Uuid::new_v4().to_string();
    let x_amz_ua = "aws-sdk-js/1.0.34 KiroIDE-1.0.0-test-machine-id";
    let ua = "aws-sdk-js/1.0.34 ua/2.1 os/windows lang/js md/nodejs#18.0.0 api/codewhispererstreaming#1.0.34 m/E KiroIDE-1.0.0-test-machine-id";
    
    let req = client
        .post(url)
        .header("content-type", "application/json")
        .header("Connection", "close")
        .header("x-amzn-codewhisperer-optout", "true")
        .header("x-amzn-kiro-agent-mode", "vibe")
        .header("x-amz-user-agent", x_amz_ua)
        .header("user-agent", ua)
        .header("host", "q.us-east-1.amazonaws.com")
        .header("amz-sdk-invocation-id", &invocation_id)
        .header("amz-sdk-request", "attempt=1; max=3")
        .header("Authorization", format!("Bearer {}", access_token))
        .json(body);

    println!("Sending request to {}...", url);
    let response = req.send().await?;
    let status = response.status();
    println!("Response Status: {}", status);

    let mut headers_str = String::new();
    for (k, v) in response.headers().iter() {
        headers_str.push_str(&format!("{}: {}\n", k.as_str(), v.to_str().unwrap_or("")));
    }

    let raw_bytes = response.bytes().await?;
    println!("Received {} bytes", raw_bytes.len());

    Ok(RequestResult {
        status: status.as_u16(),
        headers: headers_str,
        raw_response: raw_bytes.to_vec(),
    })
}

fn save_result(path: &str, res: &RequestResult) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    writeln!(file, "=== HTTP Status ===")?;
    writeln!(file, "{}", res.status)?;
    writeln!(file, "\n=== Headers ===")?;
    writeln!(file, "{}", res.headers)?;
    writeln!(file, "\n=== Response Body ===")?;
    file.write_all(&res.raw_response)?;
    Ok(())
}
