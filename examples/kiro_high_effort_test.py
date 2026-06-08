import urllib.request
import json
import time
import os

# Prompts - Lock-free MPMC queue in Rust
prompts = [
    # Turn 1
    "Design a lock-free concurrent MPMC (Multi-Producer Multi-Consumer) queue in Rust. Use atomic operations instead of locks, implement a basic epoch-based or hazard-pointer memory reclamation to prevent the ABA problem, and provide the complete code implementation.",
    # Turn 2
    "Now, add a detailed explanation and code implementation for a batching mechanism (push/pop in batches) to amortize the cost of atomic CAS operations under high thread contention, and show the exact modifications.",
    # Turn 3
    "Write a robust correctness and stress test suite in Rust that spawns 16 threads (8 producers, 8 consumers) sending 1,000,000 items in total, verifying that no items are lost, duplicated, or torn, and no memory is leaked."
]

configs = [
    {
        "name": "env1_high",
        "url": "http://127.0.0.1:8990/v1/messages",
        "key": "sk-kiro-rs-qazWSXedcRFV123456"
    },
    {
        "name": "env2_high",
        "url": "http://127.0.0.1:8992/v1/messages",
        "key": "sk-kiro-rs-qazWSXedcRFV123456789"
    }
]

def send_request(url, key, payload):
    headers = {
        "x-api-key": key,
        "content-type": "application/json"
    }
    # Increased timeout to 400 seconds (6.6 minutes) for deep reasoning and long output tokens
    req = urllib.request.Request(url, data=json.dumps(payload).encode("utf-8"), headers=headers, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=400) as response:
            res_body = response.read().decode("utf-8")
            return json.loads(res_body)
    except Exception as e:
        print(f"Request failed: {e}")
        if hasattr(e, 'read'):
            print("Error body:", e.read().decode("utf-8"))
        return None

def main():
    print("Starting high effort comparison tests...")
    
    for conf in configs:
        name = conf["name"]
        url = conf["url"]
        key = conf["key"]
        
        print(f"\n==========================================")
        print(f"Testing environment: {name}")
        print(f"==========================================")
        
        history = []
        
        for turn_idx, prompt in enumerate(prompts):
            turn = turn_idx + 1
            print(f"--- Turn {turn} ({name}) ---")
            
            history.append({"role": "user", "content": prompt})
            
            payload = {
                "model": "claude-opus-4.7",
                "max_tokens": 32000,
                "messages": history,
                "thinking": {
                    "type": "adaptive"
                },
                "output_config": {
                    "effort": "high"
                }
            }
            
            # Save request JSON
            req_filename = f"{name}_turn{turn}_req.json"
            with open(req_filename, "w", encoding="utf-8") as f:
                json.dump(payload, f, indent=2, ensure_ascii=False)
                
            print(f"Sending Turn {turn} Request to {name}...")
            start_time = time.time()
            res = send_request(url, key, payload)
            elapsed = time.time() - start_time
            
            if not res:
                print(f"Turn {turn} failed for {name}!")
                break
                
            print(f"Turn {turn} completed in {elapsed:.2f} seconds.")
            
            # Save response JSON
            res_filename = f"{name}_turn{turn}_res.json"
            res["test_metadata"] = {"elapsed_seconds": elapsed}
            with open(res_filename, "w", encoding="utf-8") as f:
                json.dump(res, f, indent=2, ensure_ascii=False)
                
            # Append assistant's response to history for next turn
            assistant_content = res.get("content", [])
            history.append({"role": "assistant", "content": assistant_content})
            
            # Brief delay to prevent hitting rate limits
            time.sleep(2)

    print("\nAll tests completed.")

if __name__ == "__main__":
    main()
