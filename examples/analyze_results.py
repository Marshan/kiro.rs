import json
import os

efforts = ["high", "xhigh", "max"]

print(f"{'Effort':<10} | {'Turn':<5} | {'Thinking Len':<15} | {'Text Len':<10} | {'First 50 Chars of Thinking'}")
print("-" * 80)

for effort in efforts:
    for turn in [1, 2]:
        res_file = f"multiturn_{effort}_turn{turn}_res.json"
        if not os.path.exists(res_file):
            print(f"{effort:<10} | Turn {turn} | Result file not found")
            continue
            
        with open(res_file, "r", encoding="utf-8") as f:
            res_data = json.load(f)
            
        content = res_data.get("content", [])
        thinking_len = 0
        text_len = 0
        thinking_sample = "N/A"
        
        for block in content:
            b_type = block.get("type")
            if b_type == "thinking":
                thinking_text = block.get("thinking", "")
                thinking_len = len(thinking_text)
                thinking_sample = thinking_text[:50].replace('\n', ' ')
            elif b_type == "text":
                text_len = len(block.get("text", ""))
                
        print(f"{effort:<10} | Turn {turn:<2} | {thinking_len:<15} | {text_len:<10} | {thinking_sample}...")

print("\n--- History Checking for Turn 2 Requests ---")
for effort in efforts:
    req_file = f"multiturn_{effort}_turn2_req.json"
    if not os.path.exists(req_file):
        continue
    with open(req_file, "r", encoding="utf-8") as f:
        req_data = json.load(f)
    
    messages = req_data.get("messages", [])
    assistant_msg = next((m for m in messages if m.get("role") == "assistant"), None)
    if assistant_msg:
        content = assistant_msg.get("content", [])
        print(f"\nEffort: {effort} - Turn 2 Request Assistant Content Blocks:")
        print(json.dumps(content, indent=2, ensure_ascii=False))
