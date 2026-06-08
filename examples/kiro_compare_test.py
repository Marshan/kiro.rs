import urllib.request
import json
import time
import os

# Prompts - complex but more focused to avoid token exhaustion and long timeouts
prompts = [
    # Turn 1
    "Design a thread-safe concurrent map in Rust using Mutex or RwLock sharding. Provide a complete and clean implementation code.",
    # Turn 2
    "Now, explain how you would optimize read-safety and memory overhead of your implementation under high read contention, and show the code changes.",
    # Turn 3
    "Write a basic unit test in Rust that verifies the concurrent map correctness under multiple parallel reading and writing threads."
]

configs = [
    {
        "name": "env1_xhigh",
        "url": "http://127.0.0.1:8990/v1/messages",
        "key": "sk-kiro-rs-qazWSXedcRFV123456",
        "effort": "xhigh"
    },
    {
        "name": "env1_max",
        "url": "http://127.0.0.1:8990/v1/messages",
        "key": "sk-kiro-rs-qazWSXedcRFV123456",
        "effort": "max"
    },
    {
        "name": "env2_xhigh",
        "url": "http://127.0.0.1:8992/v1/messages",
        "key": "sk-kiro-rs-qazWSXedcRFV123456789",
        "effort": "xhigh"
    },
    {
        "name": "env2_max",
        "url": "http://127.0.0.1:8992/v1/messages",
        "key": "sk-kiro-rs-qazWSXedcRFV123456789",
        "effort": "max"
    }
]

def send_request(url, key, payload):
    headers = {
        "x-api-key": key,
        "content-type": "application/json"
    }
    # Increased timeout to 300 seconds (5 minutes) for deep thinking models
    req = urllib.request.Request(url, data=json.dumps(payload).encode("utf-8"), headers=headers, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=300) as response:
            res_body = response.read().decode("utf-8")
            return json.loads(res_body)
    except Exception as e:
        print(f"Request failed: {e}")
        if hasattr(e, 'read'):
            print("Error body:", e.read().decode("utf-8"))
        return None

def main():
    print("Starting comparison tests...")
    
    for conf in configs:
        name = conf["name"]
        url = conf["url"]
        key = conf["key"]
        effort = conf["effort"]
        
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
                "max_tokens": 4096,
                "messages": history,
                "thinking": {
                    "type": "adaptive"
                },
                "output_config": {
                    "effort": effort
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
            # Inject elapsed time info for metadata
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
