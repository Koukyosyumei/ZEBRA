import os
import yaml

folder = "./report"  # YAMLが入っているフォルダ

values = []

for root, dirs, files in os.walk(folder):
    for file in files:
        if file.endswith(".yaml") or file.endswith(".yml"):
            path = os.path.join(root, file)
            with open(path) as f:
                data = yaml.safe_load(f)
                if "report" in data and "exe_time_mean" in data["report"]:
                    values.append(data["report"]["exe_time_mean"])

if values:
    avg = sum(values) / len(values)
    print("num files:", len(values))
    print("average exe_time_mean:", avg)
else:
    print("no exe_time_mean found")