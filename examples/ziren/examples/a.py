import re

def parse_trace_file(file_path):
    # 正規表現でi, j, 値を抽出
    df_pattern = re.compile(r'define-fun trace_(\d+)_(\d+) \(\) \(_ BitVec 32\)')
    num_pattern = re.compile(r'#x([0-9a-fA-F]+)')
    
    # 辞書で一旦格納
    temp = {}
    max_i = 0
    max_j = 0
    with open(file_path, 'r') as f:
        lines = f.readlines() 
        for (idx, line) in enumerate(lines):
            if idx % 2 == 1:
                m = df_pattern.search(line)
                if m:
                    i = int(m.group(1))
                    j = int(m.group(2))
                    n = num_pattern.search(lines[idx + 1])
                    value = int(n.group(1), 16)  # 16進数から整数に変換
                    max_i = max(max_i, i)
                    max_j = max(max_j, j)
                    if i not in temp:
                        temp[i] = {}
                    temp[i][j] = value

    # 二次元配列に変換（行列の穴は0で埋める）
    array = []
    for i in range(max_i + 1):
        row = []
        for j in range(max_j + 1):
            row.append(temp.get(i, {}).get(j, 0))
        array.append(row)
    return array

# 使用例
trace_array = parse_trace_file("./examples/a.txt")
for row in trace_array:
    print(row)