import urllib.request
import json
import time

url = "http://127.0.0.1:8990/v1/messages"
headers = {
    "x-api-key": "sk-kiro-rs-qazWSXedcRFV123456",
    "content-type": "application/json"
}

complex_prompt = (
    "There are three people: Alice, Bob, and Charlie. One of them is a knight (always tells the truth), "
    "one is a knave (always lies), and one is a spy (can lie or tell the truth).\n"
    "Alice says: 'Charlie is a knave.'\n"
    "Bob says: 'Alice is a knight.'\n"
    "Charlie says: 'I am the spy.'\n"
    "Who is who? Explain the step-by-step reasoning."
)

turn2_prompt = "Write a short Rust program to verify all possibilities and verify the answer."
turn3_prompt = "Explain in detail how a knave's statement is modeled in boolean logic in your program."

def send_request(payload):
    req = urllib.request.Request(url, data=json.dumps(payload).encode("utf-8"), headers=headers, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=120) as response:
            res_body = response.read().decode("utf-8")
            return json.loads(res_body)
    except Exception as e:
        print(f"Request failed: {e}")
        if hasattr(e, 'read'):
            print("Error body:", e.read().decode("utf-8"))
        return None

def run_test(effort):
    print(f"\n==========================================")
    print(f"Running 3-Turn Test with effort = {effort}")
    print(f"==========================================")
    
    # 1. Turn 1
    t1_payload = {
        "model": "claude-opus-4.7",
        "max_tokens": 4096,
        "messages": [
            {"role": "user", "content": complex_prompt}
        ],
        "thinking": {
            "type": "adaptive"
        },
        "output_config": {
            "effort": effort
        }
    }
    
    # Save Turn 1 Request on client side
    with open("cc_turn1_req.json", "w", encoding="utf-8") as f:
        json.dump(t1_payload, f, indent=2, ensure_ascii=False)
        
    print("Sending Turn 1 Request...")
    t1_res = send_request(t1_payload)
    if not t1_res:
        print("Turn 1 failed")
        return
        
    # Save Turn 1 Response on client side
    with open("cc_turn1_res.json", "w", encoding="utf-8") as f:
        json.dump(t1_res, f, indent=2, ensure_ascii=False)
    print("Turn 1 completed successfully.")
    
    # Extract assistant content blocks
    assistant_content_1 = t1_res.get("content", [])
    
    # 2. Turn 2
    t2_payload = {
        "model": "claude-opus-4.7",
        "max_tokens": 4096,
        "messages": [
            {"role": "user", "content": complex_prompt},
            {"role": "assistant", "content": assistant_content_1},
            {"role": "user", "content": turn2_prompt}
        ],
        "thinking": {
            "type": "adaptive"
        },
        "output_config": {
            "effort": effort
        }
    }
    
    # Save Turn 2 Request on client side
    with open("cc_turn2_req.json", "w", encoding="utf-8") as f:
        json.dump(t2_payload, f, indent=2, ensure_ascii=False)
        
    print("Sending Turn 2 Request...")
    t2_res = send_request(t2_payload)
    if not t2_res:
        print("Turn 2 failed")
        return
        
    # Save Turn 2 Response on client side
    with open("cc_turn2_res.json", "w", encoding="utf-8") as f:
        json.dump(t2_res, f, indent=2, ensure_ascii=False)
    print("Turn 2 completed successfully.")
    
    # Extract assistant content blocks
    assistant_content_2 = t2_res.get("content", [])
    
    # 3. Turn 3
    t3_payload = {
        "model": "claude-opus-4.7",
        "max_tokens": 4096,
        "messages": [
            {"role": "user", "content": complex_prompt},
            {"role": "assistant", "content": assistant_content_1},
            {"role": "user", "content": turn2_prompt},
            {"role": "assistant", "content": assistant_content_2},
            {"role": "user", "content": turn3_prompt}
        ],
        "thinking": {
            "type": "adaptive"
        },
        "output_config": {
            "effort": effort
        }
    }
    
    # Save Turn 3 Request on client side
    with open("cc_turn3_req.json", "w", encoding="utf-8") as f:
        json.dump(t3_payload, f, indent=2, ensure_ascii=False)
        
    print("Sending Turn 3 Request...")
    t3_res = send_request(t3_payload)
    if not t3_res:
        print("Turn 3 failed")
        return
        
    # Save Turn 3 Response on client side
    with open("cc_turn3_res.json", "w", encoding="utf-8") as f:
        json.dump(t3_res, f, indent=2, ensure_ascii=False)
    print("Turn 3 completed successfully.")

def main():
    run_test("xhigh")

if __name__ == "__main__":
    main()
