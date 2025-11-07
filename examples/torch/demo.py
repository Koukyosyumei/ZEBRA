import torch
import math

# パラメータ設定
p = 7.0  # mod p
lr = 0.001
steps = 5000

# 学習パラメータ (x1, x2)
x = torch.randn(2, requires_grad=True)  # 初期値ランダム

# 損失関数 L(x) = sin^2(pi * f(x) / p)
def loss_fn(x):
    f = x[0] + x[1]
    return torch.sin(math.pi * f / p) ** 2

optimizer = torch.optim.Adam([x], lr=lr)

for step in range(steps):
    optimizer.zero_grad()
    loss = loss_fn(x)
    loss.backward()
    optimizer.step()

    # 1000ステップごとにログ出力
    if step % 100 == 0 or step == steps - 1:
        print(f"step={step:5d}, loss={loss.item():.6f}, x={x.detach().numpy()}")

# 結果確認
f_val = (x[0] + x[1]).item()
print(f"\nFinal f(x) = {f_val:.6f}")
print(f"f(x) mod p = {f_val % p:.6f}")