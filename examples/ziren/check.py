import os
import re

# 対象フォルダ
DIR = "./voutput"   # 適宜変更

# 正規表現で op_b_value と op_c_value の先頭要素を抜き出す
pattern = re.compile(
    r"input0:\s*\[(\d+),\s*\d+,\s*\d+,\s*\d+\].*?"
    r"input1:\s*\[(\d+),\s*\d+,\s*\d+,\s*\d+\]"
)

observed = set()

for filename in os.listdir(DIR):
    path = os.path.join(DIR, filename)
    if not os.path.isfile(path):
        continue

    with open(path, "r") as f:
        line = f.readline().strip()

    m = pattern.search(line)
    if not m:
        print(f"[WARN] parse failed: {filename}")
        continue

    b = int(m.group(1))
    c = int(m.group(2))

    # 16×16 に収まるものだけ観測集合に入れる
    if 0 <= b < 16 and 0 <= c < 16:
        observed.add((b, c))

# 全 16×16
all_pairs = {(i, j) for i in range(16) for j in range(16)}

missing = sorted(all_pairs - observed)

print(f"Observed pairs: {len(observed)} / 256")
print(f"Missing pairs: {len(missing)}")

for b, c in missing:
    print(f"({b}, {c})")
