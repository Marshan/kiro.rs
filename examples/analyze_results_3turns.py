import json
import os
import re

print("================================================================================")
print("3-TURN MULTI-TURN VERIFICATION LOG ANALYSIS")
print("================================================================================")

for turn in [1, 2, 3]:
    print(f"\n--- TURN {turn} ---")
    
    # 1. Client Side (cc)
    cc_req_path = f"cc_turn{turn}_req.json"
    cc_res_path = f"cc_turn{turn}_res.json"
    
    if os.path.exists(cc_req_path) and os.path.exists(cc_res_path):
        with open(cc_req_path, 'r', encoding='utf-8') as f:
            cc_req = json.load(f)
        with open(cc_res_path, 'r', encoding='utf-8') as f:
            cc_res = json.load(f)
            
        print("[Client Side (cc)]")
        # Request history check
        messages = cc_req.get("messages", [])
        print(f"  - Request messages count: {len(messages)}")
        # Check if previous assistant message has thinking block
        if turn > 1:
            prev_assistant = messages[-2] if len(messages) >= 2 else {}
            content = prev_assistant.get("content", [])
            has_thinking = any(b.get("type") == "thinking" for b in content)
            print(f"  - Prev Assistant message contains thinking block: {has_thinking}")
            
        # Response content check
        res_content = cc_res.get("content", [])
        thinking_block = next((b for b in res_content if b.get("type") == "thinking"), None)
        text_block = next((b for b in res_content if b.get("type") == "text"), None)
        print(f"  - Response thinking block length: {len(thinking_block.get('thinking', '')) if thinking_block else 0} chars")
        print(f"  - Response text block length: {len(text_block.get('text', '')) if text_block else 0} chars")
        
    # 2. kiro-rs to kiro.dev request conversion
    aws_req_path = f"kiro_rs_aws_turn{turn}_req.json"
    if os.path.exists(aws_req_path):
        with open(aws_req_path, 'r', encoding='utf-8') as f:
            aws_req = json.load(f)
        print("\n[kiro-rs -> kiro.dev (aws)]")
        effort = aws_req.get("additionalModelRequestFields", {}).get("output_config", {}).get("effort")
        print(f"  - Sent Bedrock effort level: {effort}")
        history = aws_req.get("conversationState", {}).get("history", [])
        print(f"  - Sent history message count: {len(history)}")
        if turn > 1:
            prev_hist = history[-1] if history else {}
            content_str = prev_hist.get("assistantResponseMessage", {}).get("content", "")
            has_xml = "<thinking>" in content_str and "</thinking>" in content_str
            print(f"  - History assistant content contains <thinking> tags: {has_xml}")
            if has_xml:
                # Extract first 50 chars of thinking inside tags
                thinking_match = re.search(r"<thinking>(.*?)</thinking>", content_str, re.DOTALL)
                if thinking_match:
                    sample = thinking_match.group(1)[:50].strip().replace('\n', ' ')
                    print(f"    - Sample: <thinking>{sample}...</thinking>")

    # 3. Kiro backend raw response
    aws_res_path = f"kiro_rs_aws_turn{turn}_res.txt"
    if os.path.exists(aws_res_path):
        print("\n[kiro.dev -> kiro-rs raw response]")
        with open(aws_res_path, 'r', encoding='utf-8', errors='ignore') as f:
            raw = f.read()
        reasoning_events = len(re.findall(r"reasoningContentEvent", raw))
        assistant_events = len(re.findall(r"assistantResponseEvent", raw))
        print(f"  - Number of reasoningContentEvent: {reasoning_events}")
        print(f"  - Number of assistantResponseEvent: {assistant_events}")

print("\n================================================================================")
print("ANALYSIS COMPLETED")
print("================================================================================")
